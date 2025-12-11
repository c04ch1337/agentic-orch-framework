fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../.proto/agi_core.proto");
    println!("cargo:rerun-if-env-changed=PROTOC");

    // Compile agi_core.proto since curiosity_engine.proto doesn't exist
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["../.proto/agi_core.proto"], &["../.proto"])?;
    Ok(())
}
