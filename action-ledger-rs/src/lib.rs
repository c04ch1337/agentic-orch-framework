// action-ledger-rs/src/lib.rs
// Deterministic append-only Action Ledger for Phoenix ORCH AGI.
//
// This crate provides a minimal, in-process ledger API with strong
// invariants suitable for orchestrator integration:
//
// - Append-only on disk
// - Entries are encrypted at rest (AES-256-GCM)
// - Each entry participates in a SHA-256 hash chain for tamper detection
// - Public API:
//     * ActionLedger::commit_pre_execution
//     * ActionLedger::commit_post_execution

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Opaque identifier for a logical ledger entry (action).
pub type LedgerEntryId = Uuid;

/// A single step in an action plan that is about to be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlanStep {
    pub request_id: Option<String>,
    pub actor: String,
    pub tool_or_action_name: String,
    /// Canonical JSON representation of parameters (bounded in size by callers).
    pub parameters_json: String,
    /// Optional redacted/hashed snapshot of the user query.
    pub user_query_snapshot: Option<String>,
    /// Whether this step is considered critical (used by orchestrator policy).
    pub critical: bool,
    /// Free-form metadata, typically short strings / ids.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Outcome of an executed action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutcome {
    pub status: ActionOutcomeStatus,
    pub result_summary: Option<String>,
    pub error_summary: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

/// Status for an action outcome.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionOutcomeStatus {
    Pending,
    Success,
    Failed,
}

/// Internal kind of ledger event.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum LedgerEventKind {
    PreCommit,
    PostCommit,
}

/// Internal, plaintext structure that is encrypted per entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LedgerEvent {
    id: LedgerEntryId,
    kind: LedgerEventKind,
    step: Option<ActionPlanStep>,
    outcome: Option<ActionOutcome>,
    created_at: DateTime<Utc>,
}

/// On-disk representation of a single encrypted entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LedgerFileEntry {
    /// SHA-256(prev_hash || ciphertext) to form a verifiable chain.
    hash_chain: [u8; 32],
    /// Random 96-bit nonce for AES-GCM.
    nonce: [u8; 12],
    /// Encrypted bytes of LedgerEvent.
    ciphertext: Vec<u8>,
}

/// Ledger configuration derived from environment.
#[derive(Debug, Clone)]
pub struct ActionLedgerConfig {
    /// Path to the underlying append-only file.
    pub path: PathBuf,
    /// Raw 32-byte encryption key.
    pub key: [u8; 32],
}

impl ActionLedgerConfig {
    pub fn from_env() -> Self {
        let path = std::env::var("ACTION_LEDGER_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/action-ledger/ledger.bin"));

        // Encryption key: 32 bytes.
        // In production this must be provided; here we fall back to a
        // deterministic dev key with a loud log warning.
        let key_bytes = if let Ok(hex) = std::env::var("ACTION_LEDGER_KEY") {
            match decode_hex_32(&hex) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("WARNING: invalid ACTION_LEDGER_KEY: {e}; using insecure dev key");
                    insecure_dev_key()
                }
            }
        } else {
            eprintln!("WARNING: ACTION_LEDGER_KEY not set; using insecure dev key");
            insecure_dev_key()
        };

        Self {
            path,
            key: key_bytes,
        }
    }
}

/// Deterministic append-only action ledger.
pub struct ActionLedger {
    cfg: ActionLedgerConfig,
    cipher: Aes256Gcm,
    /// Last hash in the chain (or all zeros for an empty file).
    last_hash: Mutex<[u8; 32]>,
}

impl ActionLedger {
    /// Construct a new ledger using configuration from environment.
    ///
    /// This will scan the existing file (if present) to rebuild the
    /// hash chain head and validate basic integrity.
    pub fn new_default() -> Result<Self, LedgerError> {
        let cfg = ActionLedgerConfig::from_env();
        if let Some(parent) = cfg.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let key = Key::<Aes256Gcm>::from_slice(&cfg.key);
        let cipher = Aes256Gcm::new(key);

        let last_hash = rebuild_chain_head(&cfg.path, &cipher)?;

        Ok(Self {
            cfg,
            cipher,
            last_hash: Mutex::new(last_hash),
        })
    }

    fn path(&self) -> &Path {
        &self.cfg.path
    }

    /// Record a pre-execution event for a critical action.
    ///
    /// This is append-only (no previous records are modified).
    pub fn commit_pre_execution(&self, step: ActionPlanStep) -> Result<LedgerEntryId, LedgerError> {
        let id = LedgerEntryId::new_v4();
        let event = LedgerEvent {
            id,
            kind: LedgerEventKind::PreCommit,
            step: Some(step),
            outcome: None,
            created_at: Utc::now(),
        };

        self.append_event(&event)?;
        Ok(id)
    }

    /// Record a post-execution event for the given logical action id.
    ///
    /// This does not mutate the pre-commit entry; instead it appends a
    /// new event linked by the logical id, preserving append-only
    /// semantics.
    pub fn commit_post_execution(
        &self,
        id: LedgerEntryId,
        outcome: ActionOutcome,
    ) -> Result<(), LedgerError> {
        let event = LedgerEvent {
            id,
            kind: LedgerEventKind::PostCommit,
            step: None,
            outcome: Some(outcome),
            created_at: Utc::now(),
        };

        self.append_event(&event)?;
        Ok(())
    }

    fn append_event(&self, event: &LedgerEvent) -> Result<(), LedgerError> {
        let plaintext = bincode::serialize(event)?;

        // Encrypt
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| LedgerError::Crypto(format!("encrypt failed: {e}")))?;

        // Compute new hash in chain.
        let mut last = self.last_hash.lock().unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&*last);
        hasher.update(&ciphertext);
        let new_hash = hasher.finalize();
        let mut hash_arr = [0u8; 32];
        hash_arr.copy_from_slice(&new_hash);

        let file_entry = LedgerFileEntry {
            hash_chain: hash_arr,
            nonce: nonce_bytes,
            ciphertext,
        };

        // Append to file.
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(self.path())?;

        let encoded = bincode::serialize(&file_entry)?;
        // Prepend length for easy parsing: u32 length + bytes.
        let len = encoded.len() as u32;
        file.write_all(&len.to_le_bytes())?;
        file.write_all(&encoded)?;
        file.flush()?;

        // Update chain head in memory.
        *last = hash_arr;

        Ok(())
    }
}

/// Errors produced by the ledger.
#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] bincode::Error),

    #[error("crypto error: {0}")]
    Crypto(String),
}

// --- helpers ---------------------------------------------------------------

fn insecure_dev_key() -> [u8; 32] {
    // Fixed, obviously insecure key for development; MUST NOT be used
    // in regulated or production deployments.
    [0x42; 32]
}

fn decode_hex_32(s: &str) -> Result<[u8; 32], LedgerError> {
    let s = s.trim();
    let bytes = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2.min(s.len() - i)], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    if bytes.len() != 32 {
        return Err(LedgerError::Crypto(format!(
            "expected 32-byte key, got {} bytes",
            bytes.len()
        )));
    }

    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Rebuild the hash-chain head from an existing file, validating that
/// entries decrypt and the chain is internally consistent.
///
/// If the file does not exist, returns the zero-hash.
fn rebuild_chain_head(path: &Path, cipher: &Aes256Gcm) -> Result<[u8; 32], LedgerError> {
    if !path.exists() {
        return Ok([0u8; 32]);
    }

    let mut file = OpenOptions::new().read(true).open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let mut cursor = 0usize;
    let mut last_hash = [0u8; 32];

    while cursor + 4 <= buf.len() {
        let len_bytes: [u8; 4] = buf[cursor..cursor + 4].try_into().unwrap();
        cursor += 4;
        let len = u32::from_le_bytes(len_bytes) as usize;

        if cursor + len > buf.len() {
            return Err(LedgerError::Crypto(
                "ledger file truncated or corrupted".to_string(),
            ));
        }

        let slice = &buf[cursor..cursor + len];
        cursor += len;

        let entry: LedgerFileEntry = bincode::deserialize(slice)?;

        // Verify hash chain continuity.
        let mut hasher = Sha256::new();
        hasher.update(&last_hash);
        hasher.update(&entry.ciphertext);
        let computed = hasher.finalize();

        if entry.hash_chain != computed.as_slice() {
            return Err(LedgerError::Crypto(
                "ledger hash chain mismatch; possible tampering".to_string(),
            ));
        }

        // Attempt decryption to ensure key correctness; discard result.
        let nonce = Nonce::from_slice(&entry.nonce);
        let _ = cipher
            .decrypt(nonce, entry.ciphertext.as_ref())
            .map_err(|e| LedgerError::Crypto(format!("decrypt failed: {e}")))?;

        last_hash = entry.hash_chain;
    }

    Ok(last_hash)
}
