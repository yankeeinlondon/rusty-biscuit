use std::fs;

use tempfile::TempDir;
use tree_hugger::{ProgrammingLanguage, TreePackage, find_git_root, find_package_root};

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

#[test]
fn find_package_root_returns_subcrate_in_workspace() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path();

    // Simulate workspace root with only [workspace]
    fs::create_dir(root.join(".git"))?;
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/foo\"]\n",
    )?;

    // Simulate a subcrate with [package]
    fs::create_dir_all(root.join("crates/foo/src"))?;
    fs::write(
        root.join("crates/foo/Cargo.toml"),
        "[package]\nname = \"foo\"\n",
    )?;
    fs::write(root.join("crates/foo/src/lib.rs"), "pub fn f() {}\n")?;

    let git_root = find_git_root(root)?;
    let pkg_root = find_package_root(&root.join("crates/foo"), &git_root);
    assert_eq!(pkg_root, root.join("crates/foo"));

    Ok(())
}

#[test]
fn find_package_root_skips_workspace_only_cargo_toml() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path();

    // Workspace root — no [package]
    fs::create_dir(root.join(".git"))?;
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/foo\"]\n",
    )?;

    // A package area directory with no manifest
    fs::create_dir_all(root.join("area/src"))?;
    fs::write(root.join("area/src/lib.rs"), "pub fn f() {}\n")?;

    let git_root = find_git_root(root)?;
    let pkg_root = find_package_root(&root.join("area"), &git_root);

    // Should fall back to start dir, not the workspace root
    assert_eq!(pkg_root, root.join("area"));

    Ok(())
}

#[test]
fn find_package_root_uses_git_root_when_it_has_package() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path();

    // A repo root that is both a workspace and a package
    fs::create_dir(root.join(".git"))?;
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"root\"\n\n[workspace]\nmembers = [\"sub\"]\n",
    )?;

    fs::create_dir_all(root.join("sub/src"))?;

    let git_root = find_git_root(root)?;
    let pkg_root = find_package_root(&root.join("sub"), &git_root);

    // sub/ has no manifest, but git root has [package] so it qualifies
    assert_eq!(pkg_root, git_root);

    Ok(())
}

#[test]
fn find_package_root_skips_node_workspaces_root() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path();

    fs::create_dir(root.join(".git"))?;
    fs::write(
        root.join("package.json"),
        r#"{"name":"root","workspaces":["packages/*"]}"#,
    )?;

    // A sub-package with a real package.json
    fs::create_dir_all(root.join("packages/app"))?;
    fs::write(
        root.join("packages/app/package.json"),
        r#"{"name":"app","version":"1.0.0"}"#,
    )?;

    let git_root = find_git_root(root)?;
    let pkg_root = find_package_root(&root.join("packages/app"), &git_root);
    assert_eq!(pkg_root, root.join("packages/app"));

    // From a dir with no manifest, should fall back to start dir
    fs::create_dir_all(root.join("packages/utils"))?;
    let pkg_root = find_package_root(&root.join("packages/utils"), &git_root);
    assert_eq!(pkg_root, root.join("packages/utils"));

    Ok(())
}
