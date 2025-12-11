// secrets-service-rs/build.rs
// Build script to compile Protocol Buffers definitions

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the proto file into Rust code
    tonic_build::compile_protos("../.proto/agi_core.proto")?;
    tonic_build::compile_protos("../.proto/secrets_service.proto")?;
    Ok(())
}
