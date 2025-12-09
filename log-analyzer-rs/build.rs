fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src")
        .compile(
            &["../.proto/log_analyzer.proto"],
            &["../.proto"],
        )?;
    Ok(())
}