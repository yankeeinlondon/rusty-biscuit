fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Point prost/tonic at a vendored protoc so the build needs no
    // system-installed protobuf compiler. This keeps CI and fresh checkouts
    // building on macOS, Windows, and Linux without an out-of-band install.
    unsafe {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }

    let proto = "proto/rendezvous.proto";
    println!("cargo:rerun-if-changed={proto}");
    tonic_prost_build::compile_protos(proto)?;
    Ok(())
}
