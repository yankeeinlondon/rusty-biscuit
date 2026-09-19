mod common;

use common::{CliProcessFixture, md_file};
use predicates::prelude::*;

#[test]
fn test_hash_default_outputs_two_hashes() {
    let fixture = CliProcessFixture::named("hash-test-hash-default-outputs-two-hashes");
    // Default mode: frontmatter_hash-body_hash (each 16 hex chars)
    fixture
        .command()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_body_only() {
    let fixture = CliProcessFixture::named("hash-test-hash-body-only");
    fixture
        .command()
        .args(["hash", "--body", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_frontmatter_only() {
    let fixture = CliProcessFixture::named("hash-test-hash-frontmatter-only");
    fixture
        .command()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_no_frontmatter() {
    let fixture = CliProcessFixture::named("hash-test-hash-no-frontmatter");
    // Document with no frontmatter should still produce a valid hash pair
    fixture
        .command()
        .args(["hash", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_deterministic() {
    let fixture = CliProcessFixture::named("hash-test-hash-deterministic");
    // Same input should produce the same hash
    let result1 = fixture
        .command()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = fixture
        .command()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_frontmatter_reordering() {
    let fixture = CliProcessFixture::named("hash-test-hash-frontmatter-reordering");
    // Frontmatter with different key ordering should produce the same hash
    let result1 = fixture
        .command()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: Hello\nauthor: Alice\n---\n# Content")
        .output()
        .unwrap();
    let result2 = fixture
        .command()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\nauthor: Alice\ntitle: Hello\n---\n# Content")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_body_whitespace_insensitive() {
    let fixture = CliProcessFixture::named("hash-test-hash-body-whitespace-insensitive");
    // Body with different whitespace should produce the same hash (non-strict)
    let result1 = fixture
        .command()
        .args(["hash", "--body", "-"])
        .write_stdin("# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = fixture
        .command()
        .args(["hash", "--body", "-"])
        .write_stdin("# Hello\n\n\nWorld")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_strict_whitespace_sensitive() {
    let fixture = CliProcessFixture::named("hash-test-hash-strict-whitespace-sensitive");
    // With --strict, different whitespace should produce different hashes
    let result1 = fixture
        .command()
        .args(["hash", "--body", "--strict", "-"])
        .write_stdin("# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = fixture
        .command()
        .args(["hash", "--body", "--strict", "-"])
        .write_stdin("# Hello\n\n\nWorld")
        .output()
        .unwrap();

    assert_ne!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_strict_frontmatter_differs_from_normalized() {
    let fixture =
        CliProcessFixture::named("hash-test-hash-strict-frontmatter-differs-from-normalized");
    // Strict and non-strict use different serialization strategies, so their
    // hashes should differ (strict uses serde_yaml, non-strict uses sorted canonical JSON)
    let input = "---\ntitle: Hello\nauthor: Alice\n---\n# Content";
    let strict = fixture
        .command()
        .args(["hash", "--frontmatter", "--strict", "-"])
        .write_stdin(input)
        .output()
        .unwrap();
    let normal = fixture
        .command()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin(input)
        .output()
        .unwrap();

    assert_ne!(strict.stdout, normal.stdout);
}

#[test]
fn test_hash_from_file() {
    let fixture = CliProcessFixture::named("hash-test-hash-from-file");
    let tmp = md_file("---\ntitle: File Test\n---\n# Hello\n\nWorld\n");

    fixture
        .command()
        .arg("hash")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}
