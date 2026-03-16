fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use vendored protoc so the project builds without requiring `brew install protobuf`.
    std::env::set_var(
        "PROTOC",
        protoc_bin_vendored::protoc_bin_path().expect("vendored protoc"),
    );
    tonic_build::compile_protos("proto/map.proto")?;
    Ok(())
}
