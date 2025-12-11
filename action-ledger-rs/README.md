# Action Ledger Service

## Overview

Deterministic append-only Action Ledger for Phoenix ORCH AGI. Provides a minimal, in-process ledger API with strong invariants suitable for orchestrator integration.

## Features

- **Append-only on disk**: Immutable ledger entries
- **Encryption at rest**: AES-256-GCM encryption for all entries
- **Tamper detection**: SHA-256 hash chain for integrity verification
- **Pre/Post execution tracking**: Separate commit points for action planning and execution

## Port Information

This is a library crate with no standalone service. It is integrated into orchestrator and other services that need action tracking.

## Key Functionalities

- **Pre-execution commits**: Record action plans before execution
- **Post-execution commits**: Record action outcomes after execution
- **Hash chain validation**: Automatic integrity checking on startup
- **Encrypted storage**: All entries encrypted with configurable keys

## Dependencies and Requirements

### Core Dependencies
- Rust 2024 edition
- Tokio 1.48.0 (async runtime)
- AES-GCM 0.10 (encryption)
- SHA-2 0.10 (hash chain)
- Bincode 1.3 (serialization)
- UUID 1.11 (entry identification)

## Configuration

### Environment Variables
- `ACTION_LEDGER_PATH`: Path to ledger file (default: `data/action-ledger/ledger.bin`)
- `ACTION_LEDGER_KEY`: 32-byte hex-encoded encryption key (required in production)

### Security Notes

- **Production requirement**: `ACTION_LEDGER_KEY` must be set to a secure 32-byte key
- **Development warning**: Falls back to insecure dev key if not set (logged as warning)
- **Key format**: 64-character hex string (32 bytes)

## Usage

```rust
use action_ledger::ActionLedger;

let ledger = ActionLedger::new_default()?;

// Pre-execution commit
let entry_id = ledger.commit_pre_execution(action_plan_step)?;

// Post-execution commit
ledger.commit_post_execution(entry_id, action_outcome)?;
```

## Architecture

- **LedgerEvent**: Internal plaintext structure (encrypted before storage)
- **LedgerFileEntry**: On-disk encrypted representation with hash chain
- **Hash chain**: Each entry includes SHA-256(prev_hash || ciphertext) for tamper detection
- **File format**: Length-prefixed bincode-encoded entries

## Error Handling

All operations return `Result<T, LedgerError>` with variants for:
- I/O errors
- Serialization errors
- Cryptographic errors (encryption/decryption failures)
- Hash chain validation failures

