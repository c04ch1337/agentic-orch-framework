use std::env;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo that if the .proto file changes, to rerun this build script.
    println!("cargo:rerun-if-changed=../.proto/agi_core.proto");
    println!("cargo:rerun-if-changed=../.proto/log_analyzer.proto");
    println!("cargo:rerun-if-env-changed=PROTOC");

    // Get PROTOC path from environment or use default
    let protoc = env::var("PROTOC").unwrap_or_else(|_| "protoc".to_string());
    println!("cargo:warning=Using protoc from: {}", protoc);

    // Configure proto compilation
    let mut config = tonic_prost_build::configure();

    // Add type attributes for serde
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    
    // Add field attributes for optional fields
    config.field_attribute(".", "#[serde(skip_serializing_if = \"Option::is_none\")]");

    // Set protoc path and compile
    config
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["../.proto/agi_core.proto", "../.proto/log_analyzer.proto"],
            &["../.proto"],
        )?;

    Ok(())
}
