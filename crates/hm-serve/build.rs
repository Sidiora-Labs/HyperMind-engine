fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = tonic_prost_build::Config::new();
    config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    tonic_prost_build::configure()
        .build_transport(false)
        .compile_with_config(
            config,
            &["../../schemas/hypermind.proto"],
            &["../../schemas"],
        )?;
    println!("cargo:rerun-if-changed=../../schemas/hypermind.proto");
    Ok(())
}
