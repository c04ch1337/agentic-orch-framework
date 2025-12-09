// auth-service-rs/build.rs
//
// Build script to compile Protocol Buffer definitions

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Rebuild proto files if they change
    println!("cargo:rerun-if-changed=../.proto/auth_service.proto");
    println!("cargo:rerun-if-changed=../.proto/agi_core.proto");
    println!("cargo:rerun-if-changed=../.proto/secrets_service.proto");

    // Build auth_service.proto
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto") // Generate code in src/proto
        .compile(
            &[
                "../.proto/auth_service.proto",
                "../.proto/agi_core.proto",
                "../.proto/secrets_service.proto"
            ],
            &["../.proto"], // The directory containing the proto files
        )?;

    Ok(())
}