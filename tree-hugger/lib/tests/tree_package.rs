use std::fs;

use tempfile::TempDir;
use tree_hugger_lib::{ProgrammingLanguage, TreePackage};

#[test]
fn discovers_rust_package() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    fs::create_dir(root.join(".git"))?;
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"sample\"\n")?;
    fs::write(root.join("src/lib.rs"), "pub fn greet() {}\n")?;

    let package = TreePackage::new(root)?;
    assert_eq!(package.language, ProgrammingLanguage::Rust);
    assert_eq!(package.source_files.len(), 1);

    Ok(())
}
