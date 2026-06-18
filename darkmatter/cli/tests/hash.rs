mod common;

use common::{md_cmd, md_file};
use predicates::prelude::*;

#[test]
fn test_hash_default_outputs_two_hashes() {
    // Default mode: frontmatter_hash-body_hash (each 16 hex chars)
    md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_body_only() {
    md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_frontmatter_only() {
    md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_no_frontmatter() {
    // Document with no frontmatter should still produce a valid hash pair
    md_cmd()
        .args(["hash", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_deterministic() {
    // Same input should produce the same hash
    let result1 = md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_frontmatter_reordering() {
    // Frontmatter with different key ordering should produce the same hash
    let result1 = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: Hello\nauthor: Alice\n---\n# Content")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\nauthor: Alice\ntitle: Hello\n---\n# Content")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_body_whitespace_insensitive() {
    // Body with different whitespace should produce the same hash (non-strict)
    let result1 = md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("# Hello\n\n\nWorld")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_strict_whitespace_sensitive() {
    // With --strict, different whitespace should produce different hashes
    let result1 = md_cmd()
        .args(["hash", "--body", "--strict", "-"])
        .write_stdin("# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--body", "--strict", "-"])
        .write_stdin("# Hello\n\n\nWorld")
        .output()
        .unwrap();

    assert_ne!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_strict_frontmatter_differs_from_normalized() {
    // Strict and non-strict use different serialization strategies, so their
    // hashes should differ (strict uses serde_yaml, non-strict uses sorted canonical JSON)
    let input = "---\ntitle: Hello\nauthor: Alice\n---\n# Content";
    let strict = md_cmd()
        .args(["hash", "--frontmatter", "--strict", "-"])
        .write_stdin(input)
        .output()
        .unwrap();
    let normal = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin(input)
        .output()
        .unwrap();

    assert_ne!(strict.stdout, normal.stdout);
}

#[test]
fn test_hash_from_file() {
    let tmp = md_file("---\ntitle: File Test\n---\n# Hello\n\nWorld\n");

    md_cmd()
        .arg("hash")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

