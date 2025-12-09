fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Mark build script to rerun if proto files change
    println!("cargo:rerun-if-changed=../.proto/agi_core.proto");
    println!("cargo:rerun-if-changed=../.proto/secrets_service.proto");

    // Compile all proto files we need
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&[
            "../.proto/agi_core.proto",
            "../.proto/secrets_service.proto"
        ], &["../.proto"])?;
    
    Ok(())
}
