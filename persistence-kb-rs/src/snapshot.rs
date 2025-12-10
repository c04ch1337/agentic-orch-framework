use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use std::fs;
use std::io;
use thiserror::Error;
use log::{info, warn, error};
use tokio::sync::broadcast;

// Constants for resource monitoring
const CRITICAL_RESOURCE_BREACH: &str = "CRITICAL_RESOURCE_BREACH";
const MAX_SNAPSHOTS: usize = 5;

#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("IO error during snapshot operation: {0}")]
    IoError(#[from] io::Error),
    
    #[error("Snapshot not found: {0}")]
    NotFound(Uuid),
    
    #[error("Checksum verification failed for snapshot {0}")]
    ChecksumMismatch(Uuid),
    
    #[error("Transaction log error: {0}")]
    TransactionError(String),
}

#[derive(Debug, Clone)]
pub struct KBSnapshot {
    snapshot_id: Uuid,
    timestamp: DateTime<Utc>,
    data_path: PathBuf,
    metadata: HashMap<String, String>,
    checksum: [u8; 32],
}

#[derive(Debug)]
pub struct TransactionLog {
    log_id: Uuid,
    operations: Vec<Operation>,
    timestamp: DateTime<Utc>,
    snapshot_id: Uuid,
}

#[derive(Debug)]
pub enum Operation {
    Create { path: PathBuf, content: Vec<u8> },
    Modify { path: PathBuf, patches: Vec<Patch> },
    Delete { path: PathBuf, backup: Vec<u8> },
}

#[derive(Debug)]
pub struct Patch {
    offset: usize,
    old_data: Vec<u8>,
    new_data: Vec<u8>,
}

pub struct SnapshotManager {
    storage_path: PathBuf,
    max_snapshots: usize,
    current_snapshot: Arc<RwLock<Option<KBSnapshot>>>,
    transaction_log: Arc<RwLock<TransactionLog>>,
    watchdog_tx: broadcast::Sender<String>,
}

impl SnapshotManager {
    pub fn new(storage_path: PathBuf, max_snapshots: usize) -> io::Result<Self> {
        info!("Initializing SnapshotManager with storage path: {}", storage_path.display());
        fs::create_dir_all(&storage_path)?;
        
        // Create broadcast channel for watchdog notifications
        let (watchdog_tx, _) = broadcast::channel(100);
        
        let manager = Self {
            storage_path,
            max_snapshots,
            current_snapshot: Arc::new(RwLock::new(None)),
            transaction_log: Arc::new(RwLock::new(TransactionLog {
                log_id: Uuid::new_v4(),
                operations: Vec::new(),
                timestamp: Utc::now(),
                snapshot_id: Uuid::nil(),
            })),
            watchdog_tx,
        };

        info!("SnapshotManager initialized successfully");
        Ok(manager)
    }

    pub async fn create_snapshot(&self, kb_path: &Path) -> Result<KBSnapshot, SnapshotError> {
        info!("Creating new snapshot for KB at {}", kb_path.display());
        let snapshot_id = Uuid::new_v4();
        let timestamp = Utc::now();
        let snapshot_dir = self.storage_path.join(snapshot_id.to_string());
        
        // Notify watchdog of snapshot creation
        let _ = self.watchdog_tx.send(format!(
            "Creating snapshot {} for KB at {}",
            snapshot_id,
            kb_path.display()
        ));
        
        // Create snapshot directory atomically
        fs::create_dir_all(&snapshot_dir).map_err(|e| {
            error!("Failed to create snapshot directory: {}", e);
            SnapshotError::IoError(e)
        })?;
        
        // Copy KB files atomically using temporary directory
        let temp_dir = snapshot_dir.join("temp");
        fs::create_dir_all(&temp_dir)?;
        
        // Copy files to temporary location
        self.copy_directory_contents(kb_path, &temp_dir)?;
        
        // Calculate checksum
        let checksum = self.calculate_directory_checksum(&temp_dir)?;
        
        // Atomic rename from temp to final location
        let final_dir = snapshot_dir.join("data");
        fs::rename(&temp_dir, &final_dir)?;
        
        let snapshot = KBSnapshot {
            snapshot_id,
            timestamp,
            data_path: final_dir,
            metadata: HashMap::new(),
            checksum,
        };
        
        // Update current snapshot
        *self.current_snapshot.write().unwrap() = Some(snapshot.clone());
        
        // Reset transaction log
        *self.transaction_log.write().unwrap() = TransactionLog {
            log_id: Uuid::new_v4(),
            operations: Vec::new(),
            timestamp,
            snapshot_id,
        };
        
        Ok(snapshot)
    }

    fn copy_directory_contents(&self, src: &Path, dst: &Path) -> io::Result<()> {
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            if file_type.is_dir() {
                fs::create_dir_all(&dst_path)?;
                self.copy_directory_contents(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
        Ok(())
    }

    fn calculate_directory_checksum(&self, dir: &Path) -> io::Result<[u8; 32]> {
        let mut hasher = Sha256::new();
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let contents = fs::read(&path)?;
                hasher.update(&contents);
            }
        }
        
        Ok(hasher.finalize().into())
    }

    pub async fn rollback(&self, reason: Option<&str>) -> Result<(), SnapshotError> {
        info!("Initiating rollback procedure. Reason: {}", reason.unwrap_or("Not specified"));
        
        let snapshot = self.current_snapshot.read()
            .unwrap()
            .clone()
            .ok_or_else(|| {
                error!("No snapshot available for rollback");
                SnapshotError::NotFound(Uuid::nil())
            })?;

        // Notify watchdog of rollback initiation
        let _ = self.watchdog_tx.send(format!(
            "Initiating rollback to snapshot {} - Reason: {}",
            snapshot.snapshot_id,
            reason.unwrap_or("Not specified")
        ));

        // If reason is CRITICAL_RESOURCE_BREACH, handle as emergency
        if reason == Some(CRITICAL_RESOURCE_BREACH) {
            warn!("CRITICAL_RESOURCE_BREACH detected - performing emergency rollback");
            self.emergency_rollback(&snapshot).await?;
        } else {
            self.normal_rollback(&snapshot).await?;
        }
        
        info!("Rollback completed successfully");
        Ok(())
    }

    async fn emergency_rollback(&self, snapshot: &KBSnapshot) -> Result<(), SnapshotError> {
        warn!("Emergency rollback in progress - restoring from snapshot {}", snapshot.snapshot_id);
        
        // Notify watchdog of emergency rollback
        let _ = self.watchdog_tx.send(format!(
            "EMERGENCY_ROLLBACK: Restoring snapshot {}",
            snapshot.snapshot_id
        ));
        
        // Verify snapshot integrity with strict checking
        let current_checksum = self.calculate_directory_checksum(&snapshot.data_path)?;
        if current_checksum != snapshot.checksum {
            error!("Checksum verification failed during emergency rollback");
            return Err(SnapshotError::ChecksumMismatch(snapshot.snapshot_id));
        }

        // Apply compensating operations immediately
        let log = self.transaction_log.read().unwrap();
        for op in log.operations.iter().rev() {
            match op {
                Operation::Create { path, .. } => {
                    if let Err(e) = fs::remove_file(path) {
                        error!("Failed to remove file during emergency rollback: {}", e);
                        return Err(SnapshotError::IoError(e));
                    }
                },
                Operation::Modify { path, patches } => {
                    for patch in patches.iter().rev() {
                        let mut contents = fs::read(path).map_err(|e| {
                            error!("Failed to read file during patch reversal: {}", e);
                            SnapshotError::IoError(e)
                        })?;
                        
                        contents.splice(
                            patch.offset..patch.offset + patch.new_data.len(),
                            patch.old_data.clone()
                        );
                        
                        fs::write(path, contents).map_err(|e| {
                            error!("Failed to write file during patch reversal: {}", e);
                            SnapshotError::IoError(e)
                        })?;
                    }
                },
                Operation::Delete { path, backup } => {
                    if let Err(e) = fs::write(path, backup) {
                        error!("Failed to restore deleted file: {}", e);
                        return Err(SnapshotError::IoError(e));
                    }
                }
            }
        }

        warn!("Emergency rollback completed successfully");
        Ok(())
    }

    async fn normal_rollback(&self, snapshot: &KBSnapshot) -> Result<(), SnapshotError> {
        info!("Performing normal rollback to snapshot {}", snapshot.snapshot_id);
        
        // Verify snapshot integrity
        let current_checksum = self.calculate_directory_checksum(&snapshot.data_path)?;
        if current_checksum != snapshot.checksum {
            error!("Checksum verification failed during normal rollback");
            return Err(SnapshotError::ChecksumMismatch(snapshot.snapshot_id));
        }
        
        // Apply compensating operations from transaction log in reverse
        let log = self.transaction_log.read().unwrap();
        for op in log.operations.iter().rev() {
            match op {
                Operation::Create { path, .. } => {
                    fs::remove_file(path).map_err(|e| {
                        error!("Failed to remove file during rollback: {}", e);
                        SnapshotError::IoError(e)
                    })?;
                },
                Operation::Modify { path, patches } => {
                    for patch in patches.iter().rev() {
                        let mut contents = fs::read(path).map_err(|e| {
                            error!("Failed to read file during rollback: {}", e);
                            SnapshotError::IoError(e)
                        })?;
                        
                        contents.splice(
                            patch.offset..patch.offset + patch.new_data.len(),
                            patch.old_data.clone()
                        );
                        
                        fs::write(path, contents).map_err(|e| {
                            error!("Failed to write file during rollback: {}", e);
                            SnapshotError::IoError(e)
                        })?;
                    }
                },
                Operation::Delete { path, backup } => {
                    fs::write(path, backup).map_err(|e| {
                        error!("Failed to restore deleted file during rollback: {}", e);
                        SnapshotError::IoError(e)
                    })?;
                }
            }
        }
        
        info!("Normal rollback completed successfully");
        Ok(())
    }

    pub fn cleanup_old_snapshots(&self) -> io::Result<()> {
        info!("Starting cleanup of old snapshots");
        let mut snapshots: Vec<_> = fs::read_dir(&self.storage_path)?
            .filter_map(|entry| entry.ok())
            .collect();

        // Notify watchdog of cleanup operation
        let _ = self.watchdog_tx.send("Starting snapshot cleanup operation".to_string());
            
        if snapshots.len() <= self.max_snapshots {
            return Ok(());
        }
        
        // Sort by creation time, oldest first
        snapshots.sort_by_key(|entry| entry.metadata().unwrap().created().unwrap());
        
        // Remove oldest snapshots
        for entry in snapshots.iter().take(snapshots.len() - self.max_snapshots) {
            fs::remove_dir_all(entry.path())?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_snapshot_creation_and_rollback() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let kb_dir = temp.path().join("kb");
        let snapshot_dir = temp.path().join("snapshots");
        
        fs::create_dir(&kb_dir)?;
        
        // Create test file
        let test_file = kb_dir.join("test.txt");
        let mut file = File::create(&test_file)?;
        file.write_all(b"original content")?;
        
        let manager = SnapshotManager::new(snapshot_dir, 5)?;
        
        // Create snapshot
        let snapshot = manager.create_snapshot(&kb_dir).await?;
        assert!(snapshot.data_path.exists());
        
        // Modify file
        fs::write(&test_file, b"modified content")?;
        
        // Rollback
        manager.rollback().await?;
        
        // Verify content
        let content = fs::read_to_string(&test_file)?;
        assert_eq!(content, "original content");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_access() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let kb_dir = temp.path().join("kb");
        let snapshot_dir = temp.path().join("snapshots");
        
        fs::create_dir(&kb_dir)?;
        
        let manager = Arc::new(SnapshotManager::new(snapshot_dir, 5)?);
        
        let mut handles = vec![];
        
        for i in 0..5 {
            let manager = manager.clone();
            let kb_dir = kb_dir.clone();
            
            handles.push(tokio::spawn(async move {
                let file_path = kb_dir.join(format!("test_{}.txt", i));
                fs::write(&file_path, format!("content {}", i))?;
                
                manager.create_snapshot(&kb_dir).await?;
                
                Ok::<_, SnapshotError>(())
            }));
        }
        
        for handle in handles {
            handle.await??;
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_emergency_rollback() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let kb_dir = temp.path().join("kb");
        let snapshot_dir = temp.path().join("snapshots");
        
        fs::create_dir(&kb_dir)?;
        
        // Create test files
        let test_file1 = kb_dir.join("critical_data.txt");
        let test_file2 = kb_dir.join("config.txt");
        
        fs::write(&test_file1, "important data")?;
        fs::write(&test_file2, "configuration")?;
        
        let manager = SnapshotManager::new(snapshot_dir, 5)?;
        
        // Create initial snapshot
        let snapshot = manager.create_snapshot(&kb_dir).await?;
        assert!(snapshot.data_path.exists());
        
        // Simulate critical resource breach
        fs::write(&test_file1, "corrupted data")?;
        fs::write(&test_file2, "invalid config")?;
        
        // Perform emergency rollback
        manager.rollback(Some(CRITICAL_RESOURCE_BREACH)).await?;
        
        // Verify emergency rollback restored original content
        let content1 = fs::read_to_string(&test_file1)?;
        let content2 = fs::read_to_string(&test_file2)?;
        
        assert_eq!(content1, "important data", "Emergency rollback failed to restore critical data");
        assert_eq!(content2, "configuration", "Emergency rollback failed to restore configuration");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_watchdog_notifications() -> Result<(), Box<dyn std::error::Error>> {
        let temp = TempDir::new()?;
        let kb_dir = temp.path().join("kb");
        let snapshot_dir = temp.path().join("snapshots");
        
        fs::create_dir(&kb_dir)?;
        
        let manager = SnapshotManager::new(snapshot_dir.clone(), 5)?;
        let mut rx = manager.watchdog_tx.subscribe();
        
        // Create a file and snapshot
        let test_file = kb_dir.join("test.txt");
        fs::write(&test_file, "test data")?;
        
        // Create snapshot and verify watchdog notification
        manager.create_snapshot(&kb_dir).await?;
        
        if let Ok(msg) = rx.try_recv() {
            assert!(msg.contains("Creating snapshot"), "Expected snapshot creation notification");
        } else {
            panic!("No watchdog notification received for snapshot creation");
        }
        
        // Trigger emergency rollback and verify notification
        manager.rollback(Some(CRITICAL_RESOURCE_BREACH)).await?;
        
        if let Ok(msg) = rx.try_recv() {
            assert!(msg.contains("EMERGENCY_ROLLBACK"), "Expected emergency rollback notification");
        } else {
            panic!("No watchdog notification received for emergency rollback");
        }
        
        Ok(())
    }
}