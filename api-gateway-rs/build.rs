fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../.proto/agi_core.proto");

    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["../.proto/agi_core.proto"], &["../.proto"])?;
    Ok(())
}
