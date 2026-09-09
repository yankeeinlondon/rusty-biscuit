mod common;

use common::CliProcessFixture;
use predicates::prelude::*;

// =============================================================================
//                      HASH DIRECTORY MODE TESTS
// =============================================================================

/// Builds a representative Markdown tree inside the process fixture.
fn create_hash_dir(fixture: &CliProcessFixture, name: &str) -> std::path::PathBuf {
    let dir = fixture.workspace_path().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.as_path().join("a.md"),
        "---\ntitle: Alpha\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();
    std::fs::write(
        dir.as_path().join("b.md"),
        "---\ntitle: Beta\n---\n# Beta\n\nSecond file.",
    )
    .unwrap();
    // Nested subdirectory
    std::fs::create_dir(dir.as_path().join("sub")).unwrap();
    std::fs::write(
        dir.as_path().join("sub/c.md"),
        "---\ntitle: Gamma\n---\n# Gamma\n\nThird file.",
    )
    .unwrap();
    // Non-markdown file (should be ignored)
    std::fs::write(dir.as_path().join("notes.txt"), "not markdown").unwrap();
    dir
}

#[test]
fn test_hash_directory_default() {
    let fixture = CliProcessFixture::named("hash-directory-test-hash-directory-default");
    let dir = create_hash_dir(&fixture, "tree");

    fixture
        .command()
        .arg("hash")
        .arg(dir.as_path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_body_only() {
    let fixture = CliProcessFixture::named("hash-directory-test-hash-directory-body-only");
    let dir = create_hash_dir(&fixture, "tree");

    fixture
        .command()
        .args(["hash", "--body"])
        .arg(dir.as_path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_frontmatter_only() {
    let fixture = CliProcessFixture::named("hash-directory-test-hash-directory-frontmatter-only");
    let dir = create_hash_dir(&fixture, "tree");

    fixture
        .command()
        .args(["hash", "--frontmatter"])
        .arg(dir.as_path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_deterministic() {
    let fixture = CliProcessFixture::named("hash-directory-test-hash-directory-deterministic");
    let dir = create_hash_dir(&fixture, "tree");

    let result1 = fixture
        .command()
        .arg("hash")
        .arg(dir.as_path())
        .output()
        .unwrap();
    let result2 = fixture
        .command()
        .arg("hash")
        .arg(dir.as_path())
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_directory_differs_from_single_file() {
    let fixture =
        CliProcessFixture::named("hash-directory-test-hash-directory-differs-from-single-file");
    let dir = create_hash_dir(&fixture, "tree");

    let dir_result = fixture
        .command()
        .arg("hash")
        .arg(dir.as_path())
        .output()
        .unwrap();
    let file_result = fixture
        .command()
        .arg("hash")
        .arg(dir.as_path().join("a.md"))
        .output()
        .unwrap();

    // Directory aggregate should differ from any single file hash
    assert_ne!(dir_result.stdout, file_result.stdout);
}

#[test]
fn test_hash_directory_skips_hidden_dirs() {
    let fixture = CliProcessFixture::named("hash-directory-test-hash-directory-skips-hidden-dirs");
    let dir = fixture.workspace_path().join("dir");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.as_path().join("a.md"), "# Visible").unwrap();
    std::fs::create_dir(dir.as_path().join(".hidden")).unwrap();
    std::fs::write(dir.as_path().join(".hidden/secret.md"), "# Secret").unwrap();

    // Hash with only visible file
    let with_hidden = fixture
        .command()
        .arg("hash")
        .arg(dir.as_path())
        .output()
        .unwrap();

    // Hash a dir that only has the visible file (no hidden dir)
    let dir2 = fixture.workspace_path().join("dir2");
    std::fs::create_dir_all(&dir2).unwrap();
    std::fs::write(dir2.as_path().join("a.md"), "# Visible").unwrap();

    let without_hidden = fixture
        .command()
        .arg("hash")
        .arg(dir2.as_path())
        .output()
        .unwrap();

    assert_eq!(with_hidden.stdout, without_hidden.stdout);
}

#[test]
fn test_hash_directory_strict() {
    let fixture = CliProcessFixture::named("hash-directory-test-hash-directory-strict");
    let dir = create_hash_dir(&fixture, "tree");

    let normal = fixture
        .command()
        .arg("hash")
        .arg(dir.as_path())
        .output()
        .unwrap();
    let strict = fixture
        .command()
        .args(["hash", "--strict"])
        .arg(dir.as_path())
        .output()
        .unwrap();

    // Strict and normal should produce different hashes (different normalization)
    assert_ne!(normal.stdout, strict.stdout);
}

#[test]
fn test_hash_directory_ignores_non_markdown() {
    let fixture =
        CliProcessFixture::named("hash-directory-test-hash-directory-ignores-non-markdown");
    // A directory with only non-markdown files should still produce a valid hash
    let dir = fixture.workspace_path().join("dir");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.as_path().join("notes.txt"), "not markdown").unwrap();
    std::fs::write(dir.as_path().join("data.json"), "{}").unwrap();

    fixture
        .command()
        .arg("hash")
        .arg(dir.as_path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_ignores_managed_keys() {
    let fixture =
        CliProcessFixture::named("hash-directory-test-hash-directory-ignores-managed-keys");
    // Adding the managed `hash` / `last_updated` baseline fields must not move
    // the directory aggregate — the hash never hashes itself.
    let dir = fixture.workspace_path().join("dir");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.as_path().join("a.md"),
        "---\ntitle: Alpha\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();
    std::fs::write(
        dir.as_path().join("b.md"),
        "---\ntitle: Beta\n---\n# Beta\n\nSecond file.",
    )
    .unwrap();

    let before = fixture
        .command()
        .arg("hash")
        .arg(dir.as_path())
        .output()
        .unwrap();
    assert!(before.status.success());

    std::fs::write(
        dir.as_path().join("a.md"),
        "---\ntitle: Alpha\nhash: 1111111111111111-2222222222222222\nlast_updated: 2020-01-01\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();
    std::fs::write(
        dir.as_path().join("b.md"),
        "---\ntitle: Beta\nhash: 3333333333333333-4444444444444444\nlast_updated: 2020-01-01\n---\n# Beta\n\nSecond file.",
    )
    .unwrap();

    let after = fixture
        .command()
        .arg("hash")
        .arg(dir.as_path())
        .output()
        .unwrap();
    assert_eq!(
        before.stdout, after.stdout,
        "managed keys must not change the directory aggregate",
    );
}

#[test]
fn test_hash_directory_honors_ignore_properties() {
    let fixture =
        CliProcessFixture::named("hash-directory-test-hash-directory-honors-ignore-properties");
    // A file differing only in an ignored property must aggregate identically.
    let with_draft = fixture.workspace_path().join("with-draft");
    std::fs::create_dir_all(&with_draft).unwrap();
    std::fs::write(
        with_draft.as_path().join("a.md"),
        "---\ntitle: Alpha\ndraft: true\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();
    let without_draft = fixture.workspace_path().join("without-draft");
    std::fs::create_dir_all(&without_draft).unwrap();
    std::fs::write(
        without_draft.as_path().join("a.md"),
        "---\ntitle: Alpha\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();

    let a = fixture
        .command()
        .env("HASH_IGNORE_PROPERTIES", "draft")
        .arg("hash")
        .arg(with_draft.as_path())
        .output()
        .unwrap();
    let b = fixture
        .command()
        .env("HASH_IGNORE_PROPERTIES", "draft")
        .arg("hash")
        .arg(without_draft.as_path())
        .output()
        .unwrap();
    assert_eq!(
        a.stdout, b.stdout,
        "HASH_IGNORE_PROPERTIES must apply in directory mode",
    );
}

#[test]
fn test_hash_directory_rejects_save() {
    let fixture = CliProcessFixture::named("hash-directory-test-hash-directory-rejects-save");
    let dir = create_hash_dir(&fixture, "tree");
    fixture
        .command()
        .args(["hash", "--save"])
        .arg(dir.as_path())
        .assert()
        .failure();
}

#[test]
fn test_hash_directory_rejects_structured_kind() {
    let fixture =
        CliProcessFixture::named("hash-directory-test-hash-directory-rejects-structured-kind");
    let dir = create_hash_dir(&fixture, "tree");
    fixture
        .command()
        .args(["hash", "--kind", "structured"])
        .arg(dir.as_path())
        .assert()
        .failure();
}

/// Finding 22 revert (end-to-end): Markdown under directories named
/// `node_modules`, `target`, and `vendor` is part of the aggregate again. A
/// tree containing them must not hash the same as a tree without them, and the
/// invocation succeeds with no diagnostics on stderr.
#[test]
fn test_hash_directory_includes_vendored_dirs() {
    let fixture =
        CliProcessFixture::named("hash-directory-test-hash-directory-includes-vendored-dirs");
    let with_vendored = fixture.workspace_path().join("with-vendored");
    std::fs::create_dir_all(&with_vendored).unwrap();
    std::fs::write(
        with_vendored.as_path().join("top.md"),
        "---\ntitle: Top\n---\n# Top\n",
    )
    .unwrap();
    for vendored in ["node_modules", "target", "vendor"] {
        let sub = with_vendored.as_path().join(vendored);
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("nested.md"), format!("# Nested in {vendored}\n")).unwrap();
    }

    let without_vendored = fixture.workspace_path().join("without-vendored");
    std::fs::create_dir_all(&without_vendored).unwrap();
    std::fs::write(
        without_vendored.as_path().join("top.md"),
        "---\ntitle: Top\n---\n# Top\n",
    )
    .unwrap();

    let with_out = fixture
        .command()
        .arg("hash")
        .arg(with_vendored.as_path())
        .output()
        .unwrap();
    let without_out = fixture
        .command()
        .arg("hash")
        .arg(without_vendored.as_path())
        .output()
        .unwrap();

    assert!(with_out.status.success(), "hash must exit 0");
    assert!(
        with_out.stderr.is_empty(),
        "no diagnostics expected on stderr: {}",
        String::from_utf8_lossy(&with_out.stderr),
    );
    // Freeze the aggregate shape: `%016x-%016x` (two 16-hex parts) plus newline.
    let with_stdout = String::from_utf8(with_out.stdout.clone()).unwrap();
    assert!(
        predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$")
            .unwrap()
            .eval(&with_stdout),
        "aggregate must be a two-part hash: {with_stdout}",
    );
    assert_ne!(
        with_out.stdout, without_out.stdout,
        "Markdown under node_modules/target/vendor must contribute to the aggregate",
    );
}

/// Content under a vendored directory name hashes identically to the same
/// content under an ordinary directory name (path affects only sort order, and
/// both `vendor/x.md` and `zzz/x.md` sort after `a.md`), proving the vendored
/// file is genuinely hashed rather than skipped.
#[test]
fn test_hash_directory_vendored_membership_matches_plain_dir() {
    let fixture = CliProcessFixture::named(
        "hash-directory-test-hash-directory-vendored-membership-matches-plain-dir",
    );
    let vendored = fixture.workspace_path().join("vendored");
    std::fs::create_dir_all(&vendored).unwrap();
    std::fs::write(vendored.as_path().join("a.md"), "# A\n").unwrap();
    std::fs::create_dir(vendored.as_path().join("vendor")).unwrap();
    std::fs::write(vendored.as_path().join("vendor/x.md"), "# X\n").unwrap();

    let plain = fixture.workspace_path().join("plain");
    std::fs::create_dir_all(&plain).unwrap();
    std::fs::write(plain.as_path().join("a.md"), "# A\n").unwrap();
    std::fs::create_dir(plain.as_path().join("zzz")).unwrap();
    std::fs::write(plain.as_path().join("zzz/x.md"), "# X\n").unwrap();

    let v = fixture
        .command()
        .arg("hash")
        .arg(vendored.as_path())
        .output()
        .unwrap();
    let p = fixture
        .command()
        .arg("hash")
        .arg(plain.as_path())
        .output()
        .unwrap();

    assert!(v.status.success());
    assert!(p.status.success());
    assert_eq!(
        v.stdout, p.stdout,
        "vendored-dir content must hash the same as ordinary-dir content",
    );
}

/// A single malformed Markdown file must fail the whole directory aggregate
/// rather than being silently hashed as an empty document. Otherwise a CI /
/// release check using `md hash <dir>` could pass on a broken file and record
/// a false baseline.
#[test]
fn test_hash_directory_malformed_frontmatter_exits_one() {
    let fixture = CliProcessFixture::named(
        "hash-directory-test-hash-directory-malformed-frontmatter-exits-one",
    );
    let dir = fixture.workspace_path().join("dir");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.as_path().join("good.md"),
        "---\ntitle: Alpha\n---\n# Alpha\n",
    )
    .unwrap();
    // Quoted scalar followed by trailing unquoted text: a frontmatter parse error.
    std::fs::write(
        dir.as_path().join("bad.md"),
        "---\nphases: 5\nfindings:\n  - id: '@' magic lookup emits results\n---\n# Doc\n",
    )
    .unwrap();

    fixture
        .command()
        .arg("hash")
        .arg(dir.as_path())
        .assert()
        .failure();
}
