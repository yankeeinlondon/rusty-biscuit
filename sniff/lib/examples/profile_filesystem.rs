//! Filesystem-heavy profiling entry point.
//!
//! Runs the staged filesystem request against the same deterministic
//! synthetic monorepo used by the Criterion benches so flamegraphs
//! and Criterion numbers describe the same workload. The fixture is
//! materialized once into a stable on-disk location and reused across
//! runs — pass `--rebuild` (or set `SNIFF_PROFILE_REBUILD=1`) to force
//! a clean rebuild, or pass a path argument to profile a specific
//! directory instead of the synthetic fixture.

use sniff::filesystem::detect_filesystem_with_request;
use sniff::request::{FilesystemRequest, GitRequest, RepoRequest};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "../benches/support/builder.rs"]
mod builder;

const FIXTURE_DIR_NAME: &str = "sniff-profile-filesystem-fixture";
const FIXTURE_SENTINEL: &str = ".sniff-fixture-ready";

fn main() {
    let iterations: u32 = env::var("SNIFF_PROFILE_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let rebuild = env::var("SNIFF_PROFILE_REBUILD")
        .ok()
        .is_some_and(|value| !value.is_empty() && value != "0");

    let args: Vec<String> = env::args().skip(1).collect();
    let mut explicit_path: Option<PathBuf> = None;
    let mut force_rebuild = rebuild;
    for arg in &args {
        match arg.as_str() {
            "--rebuild" => force_rebuild = true,
            other => {
                if explicit_path.is_none() {
                    explicit_path = Some(PathBuf::from(other));
                }
            }
        }
    }

    let root = match explicit_path {
        Some(path) => path,
        None => prepare_fixture(force_rebuild),
    };

    let request = FilesystemRequest::new()
        .git(GitRequest::full())
        .repo(RepoRequest::full());

    println!(
        "profiling detect_filesystem_with_request({}) x{iterations}",
        root.display()
    );

    for _ in 0..iterations {
        let info =
            detect_filesystem_with_request(&root, &request).expect("filesystem detect failed");
        std::hint::black_box(info);
    }
}

fn prepare_fixture(force_rebuild: bool) -> PathBuf {
    let root = env::temp_dir().join(FIXTURE_DIR_NAME);
    let sentinel = root.join(FIXTURE_SENTINEL);

    if force_rebuild && root.exists() {
        fs::remove_dir_all(&root).expect("remove stale profile fixture");
    }

    if !sentinel.exists() {
        if root.exists() {
            // The sentinel is missing but the directory exists, meaning
            // a previous build was interrupted. Wipe and rebuild so the
            // fixture shape is deterministic.
            fs::remove_dir_all(&root).expect("remove incomplete profile fixture");
        }
        fs::create_dir_all(&root).expect("create profile fixture root");
        builder::build_large_monorepo(&root);
        fs::write(&sentinel, b"ready\n").expect("write fixture sentinel");
        println!("materialized profile fixture at {}", root.display());
    } else {
        println!("reusing profile fixture at {}", root.display());
    }

    assert!(
        root.as_path().is_dir(),
        "fixture root {} is not a directory",
        root.display()
    );
    assert_fixture_shape(&root);
    root
}

fn assert_fixture_shape(root: &Path) {
    assert!(
        root.join("Cargo.toml").is_file(),
        "workspace manifest missing"
    );
    assert!(
        root.join("crates/pkg00/Cargo.toml").is_file(),
        "expected rust package pkg00 manifest"
    );
    assert!(
        root.join("apps/app00/package.json").is_file(),
        "expected js package app00 manifest"
    );
}
