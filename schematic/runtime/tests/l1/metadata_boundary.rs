//! The definition's rate-limit and retry metadata is descriptive only: the
//! runtime must never read, enforce, or act on it (no retries, no sleeping).

/// Lower-cased spellings that would mean the runtime consumes the metadata.
const METADATA_TOKENS: &[&str] = &[
    "ratelimit",
    "rate_limit",
    "retryguidance",
    "retry_guidance",
    "retry-after",
    "retry_after",
    "thread::sleep",
    "time::sleep",
];

#[test]
fn the_runtime_neither_reads_nor_acts_on_rate_limit_or_retry_metadata() {
    let source = biscuit_test_harness::manifest_dir!().join("src");
    let manifest = std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join("Cargo.toml")).unwrap();
    assert!(!manifest.contains("schematic-define"), "the runtime must not depend on the definition types");

    let mut stack = vec![source];
    let mut scanned = 0;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                scanned += 1;
                let text = std::fs::read_to_string(&path).unwrap().to_lowercase();
                for token in METADATA_TOKENS {
                    assert!(!text.contains(token), "{} mentions `{token}`", path.display());
                }
            }
        }
    }
    assert!(scanned > 10, "scanned only {scanned} runtime source files");
}
