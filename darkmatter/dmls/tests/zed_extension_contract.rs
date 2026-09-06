//! Passive contract checks for the workspace-excluded Zed extension.
//!
//! Zed loads the manifest before compiling the extension, while Cargo builds
//! this crate outside the main workspace. These file-level invariants therefore
//! need an explicit cross-platform L1 gate in the `dmls` package.

use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

fn extension_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("zed-dmls")
}

fn parse_toml(path: &Path) -> Value {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    toml::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn required_string<'a>(document: &'a Value, key: &str, path: &Path) -> &'a str {
    document
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("{} must contain a non-empty `{key}`", path.display()))
}

#[test]
fn zed_extension_manifest_and_crate_shape_are_loadable() {
    let extension = extension_dir();
    let manifest_path = extension.join("extension.toml");
    let crate_manifest_path = extension.join("Cargo.toml");
    let lockfile_path = extension.join("Cargo.lock");

    let manifest = parse_toml(&manifest_path);
    assert_eq!(required_string(&manifest, "id", &manifest_path), "dmls");
    assert_eq!(manifest.get("schema_version").and_then(Value::as_integer), Some(1));
    required_string(&manifest, "name", &manifest_path);
    let manifest_version = required_string(&manifest, "version", &manifest_path);

    let language_server = manifest
        .get("language_servers")
        .and_then(|servers| servers.get("dmls"))
        .unwrap_or_else(|| panic!("{} must declare `[language_servers.dmls]`", manifest_path.display()));
    let languages = language_server
        .get("languages")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("`language_servers.dmls.languages` must be an array"));
    assert_eq!(languages, &[Value::String("Markdown".to_owned())]);

    let crate_manifest = parse_toml(&crate_manifest_path);
    let crate_version = crate_manifest
        .get("package")
        .and_then(|package| package.get("version"))
        .and_then(Value::as_str)
        .expect("zed-dmls package must declare a version");
    assert_eq!(manifest_version, crate_version, "extension and crate versions must match");

    let crate_types = crate_manifest
        .get("lib")
        .and_then(|lib| lib.get("crate-type"))
        .and_then(Value::as_array)
        .expect("zed-dmls must declare `lib.crate-type`");
    assert_eq!(crate_types, &[Value::String("cdylib".to_owned())]);
    assert!(
        crate_manifest
            .get("dependencies")
            .and_then(|dependencies| dependencies.get("zed_extension_api"))
            .is_some(),
        "zed-dmls must depend on `zed_extension_api`"
    );

    let lockfile = parse_toml(&lockfile_path);
    let api_packages = lockfile
        .get("package")
        .and_then(Value::as_array)
        .expect("zed-dmls Cargo.lock must contain packages")
        .iter()
        .filter(|package| package.get("name").and_then(Value::as_str) == Some("zed_extension_api"))
        .count();
    assert_eq!(api_packages, 1, "Cargo.lock must resolve exactly one `zed_extension_api`");
    assert!(extension.join("src/lib.rs").is_file(), "zed-dmls/src/lib.rs must exist");
}
