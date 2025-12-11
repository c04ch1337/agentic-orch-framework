# Config Update Service

Service for downloading and deploying LLM adapters (LoRA) and configuration updates with cryptographic signature verification.

## Features

- **Adapter Download**: Download LoRA adapters from remote URL
- **Config Push**: Deploy configuration updates with integrity checks
- **Signature Verification**: Ed25519 signature verification for all updates
- **Checksum Validation**: SHA-256 checksum verification
- **Automatic Backup**: Backs up existing config before updates

## Configuration

Environment variables:
- `CONFIG_UPDATE_ENABLED`: Enable/disable updates (default: true)
- `CONFIG_UPDATE_ADAPTER_URL`: Base URL for adapter downloads
- `CONFIG_UPDATE_VERIFY_SIGNATURES`: Enable signature verification (default: true)
- `CONFIG_UPDATE_PUBLIC_KEY`: Path to Ed25519 public key
- `CONFIG_UPDATE_CHECK_INTERVAL_SECS`: Update check interval (default: 86400)

## Usage

```rust
use config_update::{ConfigUpdateService, ConfigUpdateConfig, AdapterMetadata};

let service = ConfigUpdateService::new_default()?;

// Download adapter
let metadata = AdapterMetadata {
    adapter_id: "adapter-1".to_string(),
    version: "1.0.0".to_string(),
    model_name: "gpt-4".to_string(),
    download_url: "https://example.com/adapters/adapter-1.bin".to_string(),
    signature: "base64-signature".to_string(),
    file_size: 1024 * 1024,
    checksum: "sha256-checksum".to_string(),
    description: None,
};

service.download_adapter(&metadata, Path::new("./adapters/adapter-1.bin")).await?;
```

## Security

- All downloads verified with Ed25519 signatures
- SHA-256 checksums ensure file integrity
- Public key must be provided for signature verification
- Failed verifications prevent deployment

