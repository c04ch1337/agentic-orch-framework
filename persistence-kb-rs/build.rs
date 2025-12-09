// persistence-kb-rs/build.rs
// Build script for persistence-kb-rs

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(
            &["../.proto/agi_core.proto"],
            &["../.proto"],
        )?;
    Ok(())
}