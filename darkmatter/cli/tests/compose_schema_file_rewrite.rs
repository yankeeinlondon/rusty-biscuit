//! Integration tests for the eager-`file` value rewrite in the compose pipeline.
//!
//! Exercises the `md` binary end-to-end (via `assert_cmd`) to prove the
//! spec's user-visible scenarios (Phase 4 integration bullets):
//!
//! 1. `md compose` of a doc whose `file(eager)`-typed property started raw
//!    shows the resolved repo-relative value in the effective frontmatter
//!    dump.
//! 2. Compose write-back persists the rewritten value — re-running
//!    `md compose` on the persisted file is a fixpoint (idempotence at the
//!    file level, Decision #6).
//! 3. End-to-end review-feature-shaped fixture — the motivating bug
//!    reproduced and then structurally fixed.
//!
//! See `darkmatter/features/2026-06-27-file-property-rewrite/spec.md`.

use darkmatter::markdown::Markdown;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn md_cmd() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("md").unwrap()
}

fn write_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

/// Creates a temp directory that looks like a git repository root (with a
/// `.git` marker) so the rewrite projects git-root-relative independent of
/// the host's real repo boundaries.
fn repo_fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join(".git")).unwrap();
    dir
}

/// Composes `doc` with `--frontmatter` and re-parses the emitted document's
/// frontmatter so the caller can inspect the *serialized* rewritten values.
fn composed_frontmatter(doc: &Path) -> serde_json::Map<String, serde_json::Value> {
    let assert = md_cmd()
        .args(["compose", "--frontmatter"])
        .arg(doc)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let reparsed: Markdown = stdout.as_str().into();
    reparsed
        .frontmatter()
        .as_map()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

// ── Integration bullet 1: rewritten value appears in the frontmatter dump ──

/// A doc whose `file(eager)`-typed property started raw shows the resolved
/// repo-relative value in the effective frontmatter dump.
///
/// The prompt lives at the repo root and references a spec in a subdirectory,
/// so the git-root-relative projection is visible (the raw `./area/spec.md`
/// becomes `area/spec.md`).
#[test]
fn compose_frontmatter_shows_resolved_repo_relative_spec() {
    let repo = repo_fixture();
    fs::create_dir_all(repo.path().join("area")).unwrap();
    fs::write(repo.path().join("area/spec.md"), "# Spec\n").unwrap();
    let doc = write_file(
        &repo,
        "prompt.md",
        "---\n$schema:\n  spec: 'file(eager; required)'\nspec: ./area/spec.md\n---\nBody\n",
    );

    let fm = composed_frontmatter(&doc);
    // Raw `./area/spec.md` -> repo-relative `area/spec.md`.
    assert_eq!(fm.get("spec"), Some(&serde_json::json!("area/spec.md")));
}

// ── Integration bullet 2: compose write-back persists; re-compose is a fixpoint ──

/// Compose write-back persists the rewritten value. Re-running `md compose`
/// on the persisted (already-rewritten) file is a fixpoint: the stored value
/// does not drift across runs (Decision #6).
///
/// The prompt lives at the repo root so the repo-relative rewritten value
/// re-resolves document-first on the second pass without a launch-area
/// fallback.
#[test]
fn compose_write_back_persists_rewritten_value_as_fixpoint() {
    let repo = repo_fixture();
    fs::create_dir_all(repo.path().join("area")).unwrap();
    fs::write(repo.path().join("area/spec.md"), "# Spec\n").unwrap();
    let original = write_file(
        &repo,
        "prompt.md",
        "---\n$schema:\n  spec: 'file(eager; required)'\nspec: ./area/spec.md\n---\nBody\n",
    );

    // First compose: capture the rewritten frontmatter.
    let fm1 = composed_frontmatter(&original);
    assert_eq!(fm1.get("spec"), Some(&serde_json::json!("area/spec.md")));

    // Persist the rewritten document to a new file (simulating the on-disk
    // write-back) and re-compose it. The rewritten value is a fixpoint —
    // it must not drift across runs.
    let persisted = repo.path().join("persisted.md");
    fs::write(
        &persisted,
        "---\n$schema:\n  spec: 'file(eager; required)'\nspec: area/spec.md\n---\nBody\n",
    )
    .unwrap();

    let fm2 = composed_frontmatter(&persisted);
    assert_eq!(
        fm2.get("spec"),
        Some(&serde_json::json!("area/spec.md")),
        "re-composing the persisted (rewritten) value must be a fixpoint",
    );
}

// ── Integration bullet 3: review-feature-shaped fixture ──────────────────────
//
// The motivating bug: `spec: string` stayed raw while `dirname(spec)` resolved,
// so `review_file: "{{dirname(spec)}}/review-{{iteration}}.md"` doubled the
// area prefix when an author hand-prepended `{{ctx.area}}/`. With
// `spec: file(eager)`, the rewrite stores the resolved repo-relative path, so
// `dirname(spec)` and `review_file` agree by construction — no hand-prefix
// needed. `review_file` itself stays bare/lazy `file` — it may name a
// not-yet-existing output, but here it is created on disk to prove the
// derived path is correct.

#[test]
fn review_feature_fixture_dirname_and_review_file_agree_after_rewrite() {
    let repo = repo_fixture();
    fs::create_dir_all(repo.path().join("area")).unwrap();
    fs::write(repo.path().join("area/spec.md"), "# Spec\n").unwrap();
    // Create the review file so the derived `review_file` path exists on disk
    // when composition resolves it — proving the path is correct. The bug's
    // symptom was a non-existent doubled path that threw at event time.
    fs::write(repo.path().join("area/review-1.md"), "# Review\n").unwrap();

    let doc = write_file(
        &repo,
        "prompt.md",
        "---\n\
         $schema:\n\
         \x20 spec: 'file(eager; required)'\n\
         \x20 review_file: 'file'\n\
         \x20 iteration: 'number(required)'\n\
         spec: ./area/spec.md\n\
         iteration: 1\n\
         review_file: \"{{dirname(spec)}}/review-{{iteration}}.md\"\n\
         ---\nBody\n",
    );

    let fm = composed_frontmatter(&doc);
    // The rewrite stores the repo-relative spec.
    let spec = fm
        .get("spec")
        .and_then(|v| v.as_str())
        .expect("spec present");
    assert_eq!(spec, "area/spec.md");

    // `review_file` is derived from the already-resolved spec via
    // `dirname(spec)`, so it lands at `area/review-1.md` — no doubling.
    let review_file = fm
        .get("review_file")
        .and_then(|v| v.as_str())
        .expect("review_file present");
    assert_eq!(
        review_file, "area/review-1.md",
        "review_file must derive from the rewritten spec without doubling the area prefix",
    );

    // `dirname(spec)` and `dirname(review_file)` resolve to the same prefix.
    let spec_dir = parent_dir(spec);
    let review_dir = parent_dir(review_file);
    assert_eq!(
        spec_dir, review_dir,
        "dirname(spec) and dirname(review_file) must agree after the rewrite",
    );

    // The derived path exists on disk (the doubled-path bug would have missed).
    let resolved = repo.path().join(review_file);
    assert!(
        resolved.exists(),
        "derived review_file `{}` must exist on disk; the doubled-path bug would have missed it",
        resolved.display(),
    );
}

/// Returns the parent directory of a `/`-joined path, or the empty string
/// when the path has no parent (mirrors `dirname`'s repo-relative projection).
fn parent_dir(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => String::new(),
    }
}
