#[cfg(not(windows))]
fn main() -> std::io::Result<()> {
    #[allow(unused_imports)]
    use std::path::Path;

    let protoc = prost_build::protoc_from_env();
    if std::process::Command::new(protoc)
        .arg("--version")
        .output()
        .is_err()
    {
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var("PROTOC", protobuf_src::protoc());
        }
    }

    // prost_build::Config::new()
    //     .out_dir(&Path::new("./src"))
    //     .compile_protos(&["protos/perfetto_trace.proto"], &["protos"])?;

    // TODO: comment in
    // https://github.com/google/perfetto/blob/main/protos/perfetto/trace/perfetto_trace.proto
    // prost_build::compile_protos(&["protos/perfetto_trace.proto"], &["protos"])?;
    Ok(())
}

#[cfg(windows)]
fn main() -> std::io::Result<()> {
    Ok(())
}
