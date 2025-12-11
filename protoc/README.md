# Protocol Buffer Compiler

## Overview

Precompiled Protocol Buffer compiler (protoc) for Phoenix ORCH AGI system. Used for generating Rust code from `.proto` definition files.

## Contents

- **bin/protoc.exe**: Windows executable for Protocol Buffer compiler
- **include/**: Google Protocol Buffer well-known types
  - Standard message types (Any, Timestamp, Duration, etc.)
  - API definitions
  - Compiler plugin definitions

## Usage

### Windows
```powershell
.\protoc\bin\protoc.exe --version
```

### Generate Rust Code
```bash
protoc --rust_out=src --proto_path=protoc/include --proto_path=. your_file.proto
```

## Installation

### Automatic Installation
Use the installation scripts:
- **Linux/macOS**: `scripts/install_protoc.sh`
- **Windows**: `scripts/install_protoc.ps1`

### Manual Installation
1. Add `protoc/bin` to your PATH
2. Copy `include/` to system include directory (optional)
3. Verify installation: `protoc --version`

## Protocol Buffer Files

The system uses Protocol Buffers for:
- gRPC service definitions
- Message serialization
- Cross-language communication
- API contracts

## Well-Known Types

Included standard types:
- `google.protobuf.Any`
- `google.protobuf.Timestamp`
- `google.protobuf.Duration`
- `google.protobuf.Empty`
- `google.protobuf.Struct`
- And more...

## Integration

### Build Process
Services use `build.rs` scripts to:
1. Compile `.proto` files
2. Generate Rust code via `tonic-build`
3. Include generated code in service binaries

### Service Definitions
Protocol definitions located in:
- `phoenix_orch_proto/`: Main proto definitions
- Service-specific proto files

## Version Information

This package contains a precompiled binary version of the Protocol Buffer compiler. For source code and latest versions, see:
- https://github.com/protocolbuffers/protobuf

## Requirements

- Windows: No additional dependencies
- Linux/macOS: Use system package manager or build from source
- All platforms: Compatible with Protocol Buffers 3.x

