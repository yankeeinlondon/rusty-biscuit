//! Production hashing in this package goes through `biscuit-hash`. The
//! standard library's `DefaultHasher` is SipHash whose algorithm the standard
//! library does not promise to keep, so a fingerprint taken with it is
//! neither the repository's hash nor stable between toolchains; streaming
//! content goes through `biscuit_hash::XxHasher` instead.

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn production_source_hashes_through_biscuit_hash() {
    let src = biscuit_test_harness::manifest_dir!().join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(files.len() > 5, "scanned {}: {files:?}", src.display());

    let mut found = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        for (number, line) in text.lines().enumerate() {
            if line.contains("DefaultHasher") {
                found.push(format!("{}:{}: {}", file.strip_prefix(&src).unwrap().display(), number + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "hash through `biscuit_hash`, not `DefaultHasher`:\n{}", found.join("\n"));
}
