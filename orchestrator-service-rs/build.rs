use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo that if the .proto file changes, to rerun this build script.
    println!("cargo:rerun-if-changed=../.proto/agi_core.proto");
    println!("cargo:rerun-if-changed=../.proto/log_analyzer.proto");
    println!("cargo:rerun-if-env-changed=PROTOC");

    // Get PROTOC path from environment or use default
    let protoc = env::var("PROTOC").unwrap_or_else(|_| "protoc".to_string());
    println!("cargo:warning=Using protoc from: {}", protoc);

    // Configure proto compilation with chained method calls
    tonic_prost_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .field_attribute(".", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["../.proto/agi_core.proto", "../.proto/log_analyzer.proto"],
            &["../.proto"],
        )?;

    Ok(())
}
