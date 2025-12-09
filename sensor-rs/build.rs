fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo that if the .proto file changes, to rerun this build script.
    println!("cargo:rerun-if-changed=../.proto/agi_core.proto");

    // Configure and compile proto files
    // We only need client code since sensor-rs is a gRPC client, not a server
    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["../.proto/agi_core.proto"], &["../.proto"])?;
    Ok(())
}
