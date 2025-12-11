// auth-service-rs/build.rs
//
// Build script to compile Protocol Buffer definitions

use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Rebuild if proto files change
    println!("cargo:rerun-if-changed=../.proto/auth_service.proto");
    println!("cargo:rerun-if-changed=../.proto/agi_core.proto");
    println!("cargo:rerun-if-changed=../.proto/secrets_service.proto");
    println!("cargo:rerun-if-env-changed=PROTOC");

    // Configure proto compilation
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "../.proto/auth_service.proto",
                "../.proto/agi_core.proto",
                "../.proto/secrets_service.proto",
            ],
            &["../.proto"],
        )
        .map_err(|e| {
            println!("cargo:warning=Proto compilation failed: {}", e);
            e
        })?;

    Ok(())
}
