fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo that if the .proto file changes, to rerun this build script.
    println!("cargo:rerun-if-changed=../.proto/agi_core.proto");

    // Configure and compile proto files
    unsafe {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());
    }
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["../.proto/agi_core.proto"], &["../.proto"])?;
    Ok(())
}

