// self-improve-rs/src/repository.rs
// Persistence layer for self-improvement ErrorRecord storage.
//
// Implementation notes:
// - Append-only NDJSON file on disk (one ErrorRecord per line).
// - Simple filtering by request_id and failure_type.
// - Retention and compaction strategies can be added later if needed.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error_record::ErrorRecord;

/// Repository error type.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[async_trait]
pub trait ErrorRecordRepository {
    async fn insert(&self, record: &ErrorRecord) -> Result<(), RepositoryError>;

    async fn get_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<Vec<ErrorRecord>, RepositoryError>;

    async fn get_by_failure_type(
        &self,
        failure_type: &str,
    ) -> Result<Vec<ErrorRecord>, RepositoryError>;
}

/// Simple file-backed repository that stores records as NDJSON:
/// one serialized ErrorRecord per line.
///
/// This is intentionally append-only and suitable for local development
/// or single-node deployments. A more advanced backend (e.g. SQLite,
/// Postgres, or a column store) can be wired behind the same trait later.
pub struct FileBackedRepository {
    path: PathBuf,
}

impl FileBackedRepository {
    /// Create a repository instance using a path derived from
    /// SELF_IMPROVE_STORE_PATH or a safe default.
    ///
    /// This constructor eagerly validates that the parent directory is
    /// writable by attempting to create it. This allows callers to fail
    /// fast (e.g., during service startup) when the configured storage
    /// path is invalid or unwritable.
    pub fn new_default() -> Result<Self, RepositoryError> {
        let path = std::env::var("SELF_IMPROVE_STORE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/self-improve/error_records.ndjson"));

        if let Some(parent) = path.parent() {
            // Use blocking std::fs here since this is a one-time startup check.
            std::fs::create_dir_all(parent)?;
        }

        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    async fn ensure_parent_dir(&self) -> Result<(), RepositoryError> {
        if let Some(parent) = self.path().parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    async fn read_all(&self) -> Result<Vec<ErrorRecord>, RepositoryError> {
        if !self.path().exists() {
            return Ok(Vec::new());
        }

        let mut file = fs::File::open(self.path()).await?;
        let mut buf = String::new();
        file.read_to_string(&mut buf).await?;

        let mut out = Vec::new();
        for line in buf.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<ErrorRecord>(line) {
                Ok(rec) => out.push(rec),
                Err(err) => {
                    // Log and continue on parse failures to avoid breaking ingestion.
                    tracing::warn!(error = %err, "failed to parse ErrorRecord line; skipping");
                }
            }
        }

        Ok(out)
    }
}

#[async_trait]
impl ErrorRecordRepository for FileBackedRepository {
    async fn insert(&self, record: &ErrorRecord) -> Result<(), RepositoryError> {
        self.ensure_parent_dir().await?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(self.path())
            .await?;

        let line = serde_json::to_string(record)?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    async fn get_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<Vec<ErrorRecord>, RepositoryError> {
        let all = self.read_all().await?;
        Ok(all
            .into_iter()
            .filter(|r| r.request_id == request_id)
            .collect())
    }

    async fn get_by_failure_type(
        &self,
        failure_type: &str,
    ) -> Result<Vec<ErrorRecord>, RepositoryError> {
        let all = self.read_all().await?;
        Ok(all
            .into_iter()
            .filter(|r| r.failure_type == failure_type)
            .collect())
    }
}
