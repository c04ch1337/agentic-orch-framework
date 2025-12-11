// secrets-service-rs/build.rs
// Build script to compile Protocol Buffers definitions

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the proto file into Rust code
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["../.proto/agi_core.proto", "../.proto/secrets_service.proto"],
            &["../.proto"],
        )?;
    Ok(())
}
