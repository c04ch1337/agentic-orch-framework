fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../.proto/log_analyzer.proto");
    println!("cargo:rerun-if-env-changed=PROTOC");

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["../.proto/log_analyzer.proto"], &["../.proto"])?;
    Ok(())
}
