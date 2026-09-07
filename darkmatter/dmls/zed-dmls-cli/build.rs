use std::{fs, io, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest_path.display());

    let manifest_text = fs::read_to_string(&manifest_path)?;
    let manifest: toml::Value = toml::from_str(&manifest_text)?;
    let version = manifest
        .get("package")
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} has no string package.version", manifest_path.display()),
            )
        })?;
    println!("cargo:rustc-env=DMLS_PACKAGE_VERSION={version}");

    Ok(())
}
