use git2::Repository;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary git repo for testing
pub fn create_test_git_repo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let _repo = Repository::init(dir.path()).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a Cargo workspace structure
pub fn create_cargo_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"pkg1\", \"pkg2\"]\n",
    )
    .unwrap();
    fs::create_dir(dir.path().join("pkg1")).unwrap();
    fs::create_dir(dir.path().join("pkg2")).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a directory with mixed language files
pub fn create_mixed_language_dir() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn foo() {}").unwrap();
    fs::write(dir.path().join("index.js"), "console.log('hello')").unwrap();
    fs::write(dir.path().join("app.py"), "print('hello')").unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Create a pnpm workspace
pub fn create_pnpm_workspace() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::create_dir_all(dir.path().join("packages/app")).unwrap();
    fs::create_dir_all(dir.path().join("packages/lib")).unwrap();
    let path = dir.path().to_path_buf();
    (dir, path)
}
