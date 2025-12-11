fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo that if the proto files change, to rerun this build script.
    println!("cargo:rerun-if-changed=../.proto/agi_core.proto");
    println!("cargo:rerun-if-env-changed=PROTOC");

    // Configure and compile proto files
    tonic_prost_build::configure()
        .proto_path("../.proto")
        .build_server(true)
        .build_client(true)
        .compile_protos(&["../.proto/agi_core.proto"], &["../.proto"])
        .map_err(|e| {
            println!("cargo:warning=Proto compilation failed: {}", e);
            e
        })?;

    Ok(())
}
