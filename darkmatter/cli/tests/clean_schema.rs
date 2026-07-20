//! Level-1 CLI coverage for `md clean`'s schema resolution (acceptance rows
//! D-10 and D-8).
//!
//! Every test here is a *differential*: the same document is cleaned twice,
//! differing only in schema state, and the assertion is that schema-proven
//! quoting fires in one run and not the other. `release: 1.20` is the probe —
//! it is only ever quoted when an authoritative schema proves a string was
//! required, so its treatment reports directly on which schema layers were in
//! effect.
//!
//! See `darkmatter/features/2026-07-14-invalid-frontmatter/spec.md`.

mod common;

use common::md_cmd;
use std::fs;
use std::path::{Path, PathBuf};

/// A repo-shaped fixture: a Git root, a `schemas/` trigger root, and `docs/`.
struct Repo {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        Self { _dir: dir, root }
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    /// Installs a trigger schema that requires `release` to be a string, armed
    /// on `kind: thing`.
    fn with_release_trigger(&self) -> &Self {
        self.write(
            "schemas/thing.trigger.yaml",
            "kind: trigger-schema\nmatch:\n  kind: enum(thing; required)\n$schema: thing.yaml\n",
        );
        self.write("schemas/thing.yaml", RELEASE_IS_STRING);
        self
    }
}

/// A schema requiring `release` to be a string — the quoting probe's trigger.
const RELEASE_IS_STRING: &str = "$schema:\n  release: string(required)\n";

/// A schema that says nothing about `release`.
const RELEASE_UNCONSTRAINED: &str = "$schema:\n  other: string\n";

/// Runs `md clean` and returns STDOUT.
fn clean(path: &Path, args: &[&str]) -> String {
    let assert = md_cmd()
        .arg("clean")
        .arg(path)
        .args(args)
        .assert()
        .success();
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

/// Whether schema-proven quoting fired on the `release` probe.
fn release_quoted(stdout: &str) -> bool {
    assert!(
        stdout.contains("release:"),
        "probe key missing from output:\n{stdout}"
    );
    stdout.contains("release: \"1.20\"")
}

// --- Baseline schema ------------------------------------------------------

/// `--baseline-schema PATH` replaces the built-in baseline and its constraints
/// reach the quoting tier.
#[test]
fn test_baseline_schema_flag_drives_schema_proven_quoting() {
    let repo = Repo::new();
    let schema = repo.write("strict.yaml", RELEASE_IS_STRING);
    let doc = repo.write("docs/doc.md", "---\nrelease: 1.20\n---\n\n# Body\n");

    let with_flag = clean(&doc, &["--baseline-schema", schema.to_str().unwrap()]);
    assert!(release_quoted(&with_flag), "expected quoting:\n{with_flag}");

    let without = clean(&doc, &[]);
    assert!(
        !release_quoted(&without),
        "the built-in baseline does not constrain `release`:\n{without}"
    );
}

/// The default baseline is on, and `--no-baseline-schema` turns it off. The
/// probe is a Darkmatter-owned key (`ctx`), which only the baseline declares.
#[test]
fn test_default_baseline_is_active_and_no_baseline_schema_disables_it() {
    let repo = Repo::new();
    let doc = repo.write("docs/doc.md", "---\nctx:\n  undeclared_key: 1\n---\n\n# Body\n");

    let default = md_cmd().arg("clean").arg(&doc).assert().success();
    let default_stderr = String::from_utf8(default.get_output().stderr.clone()).unwrap();
    assert!(
        default_stderr.contains("schema.key-correction"),
        "the default baseline must declare `ctx`, got:\n{default_stderr}"
    );

    let disabled = md_cmd()
        .arg("clean")
        .arg(&doc)
        .arg("--no-baseline-schema")
        .assert()
        .success();
    let disabled_stderr = String::from_utf8(disabled.get_output().stderr.clone()).unwrap();
    assert!(
        !disabled_stderr.contains("schema.key-correction"),
        "--no-baseline-schema must remove the baseline layer, got:\n{disabled_stderr}"
    );
}

/// `--no-baseline-schema` also removes a baseline's quoting proof.
#[test]
fn test_no_baseline_schema_suppresses_quoting() {
    let repo = Repo::new();
    let schema = repo.write("strict.yaml", RELEASE_IS_STRING);
    let doc = repo.write("docs/doc.md", "---\nrelease: 1.20\n---\n\n# Body\n");

    assert!(release_quoted(&clean(
        &doc,
        &["--baseline-schema", schema.to_str().unwrap()]
    )));
    assert!(!release_quoted(&clean(&doc, &["--no-baseline-schema"])));
}

/// The two baseline flags are mutually exclusive, matching `md compose`.
#[test]
fn test_baseline_schema_flags_conflict() {
    md_cmd()
        .args([
            "clean",
            "doc.md",
            "--baseline-schema",
            "s.yaml",
            "--no-baseline-schema",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

// --- Document `$schema` ---------------------------------------------------

/// A file-reference `$schema` in the document's own frontmatter is honored.
#[test]
fn test_document_schema_file_reference_drives_quoting() {
    let repo = Repo::new();
    repo.write("docs/own.yaml", RELEASE_IS_STRING);
    let doc = repo.write(
        "docs/doc.md",
        "---\n$schema: ./own.yaml\nrelease: 1.20\n---\n\n# Body\n",
    );

    assert!(release_quoted(&clean(&doc, &[])));
}

/// An inline mapping `$schema` is honored too.
#[test]
fn test_document_inline_schema_drives_quoting() {
    let repo = Repo::new();
    let doc = repo.write(
        "docs/doc.md",
        "---\n$schema:\n  release: string(required)\nrelease: 1.20\n---\n\n# Body\n",
    );

    assert!(release_quoted(&clean(&doc, &[])));
}

/// A root-union `$schema` sequence resolves; the arm constraining `release`
/// still proves the quoting.
#[test]
fn test_document_root_union_schema_drives_quoting() {
    let repo = Repo::new();
    let doc = repo.write(
        "docs/doc.md",
        "---\n$schema:\n  - release: string(required)\nrelease: 1.20\n---\n\n# Body\n",
    );

    assert!(release_quoted(&clean(&doc, &[])));
}

// --- `--schema` override --------------------------------------------------

/// `--schema` supplies a schema to a document that declares none.
#[test]
fn test_schema_flag_supplies_schema_to_document_without_one() {
    let repo = Repo::new();
    let schema = repo.write("strict.yaml", RELEASE_IS_STRING);
    let doc = repo.write("docs/doc.md", "---\nrelease: 1.20\n---\n\n# Body\n");

    assert!(!release_quoted(&clean(&doc, &[])));
    assert!(release_quoted(&clean(
        &doc,
        &["--schema", schema.to_str().unwrap()]
    )));
}

/// Precedence: `--schema` *replaces* the document's own `$schema` layer rather
/// than merging with it. The document's schema would prove the quoting; the
/// override does not, so quoting must stop.
#[test]
fn test_schema_flag_replaces_document_schema_layer() {
    let repo = Repo::new();
    repo.write("docs/own.yaml", RELEASE_IS_STRING);
    let loose = repo.write("loose.yaml", RELEASE_UNCONSTRAINED);
    let doc = repo.write(
        "docs/doc.md",
        "---\n$schema: ./own.yaml\nrelease: 1.20\n---\n\n# Body\n",
    );

    assert!(
        release_quoted(&clean(&doc, &[])),
        "the document's own schema proves the quoting"
    );
    assert!(
        !release_quoted(&clean(&doc, &["--schema", loose.to_str().unwrap()])),
        "--schema must replace, not merge with, the document layer"
    );
}

// --- Trigger schemas ------------------------------------------------------

/// A matching trigger schema reaches the quoting tier.
#[test]
fn test_matching_trigger_schema_drives_quoting() {
    let repo = Repo::new();
    repo.with_release_trigger();
    let doc = repo.write(
        "docs/doc.md",
        "---\nkind: thing\nrelease: 1.20\n---\n\n# Body\n",
    );

    assert!(release_quoted(&clean(&doc, &[])));
}

/// A trigger whose arm does not match contributes nothing.
#[test]
fn test_nonmatching_trigger_schema_is_inert() {
    let repo = Repo::new();
    repo.with_release_trigger();
    let doc = repo.write(
        "docs/doc.md",
        "---\nkind: other\nrelease: 1.20\n---\n\n# Body\n",
    );

    assert!(!release_quoted(&clean(&doc, &[])));
}

/// `--no-trigger-schemas` disables discovery even when an arm would match.
#[test]
fn test_no_trigger_schemas_disables_discovery() {
    let repo = Repo::new();
    repo.with_release_trigger();
    let doc = repo.write(
        "docs/doc.md",
        "---\nkind: thing\nrelease: 1.20\n---\n\n# Body\n",
    );

    assert!(release_quoted(&clean(&doc, &[])));
    assert!(!release_quoted(&clean(&doc, &["--no-trigger-schemas"])));
}

// --- Stdin and the top-level shorthand ------------------------------------

/// Schema flags apply to stdin documents, which have no path to anchor
/// discovery to.
#[test]
fn test_stdin_honors_schema_flag() {
    let repo = Repo::new();
    let schema = repo.write("strict.yaml", RELEASE_IS_STRING);

    let assert = md_cmd()
        .args(["clean", "-"])
        .arg("--schema")
        .arg(&schema)
        .write_stdin("---\nrelease: 1.20\n---\n\n# Body\n")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(release_quoted(&stdout), "got:\n{stdout}");
}

/// The top-level `md <file> --save` shorthand runs the same pipeline with the
/// same schema defaults.
#[test]
fn test_save_shorthand_uses_default_schema_state_and_repairs() {
    let repo = Repo::new();
    repo.with_release_trigger();
    let doc = repo.write(
        "docs/doc.md",
        "---\nkind: thing\ntitle: @daily-report\nrelease: 1.20\n---\n\n# Body\n",
    );

    md_cmd().arg(&doc).arg("--save").assert().success();

    let saved = fs::read_to_string(&doc).unwrap();
    assert!(
        saved.contains("title: \"@daily-report\""),
        "shorthand must run the syntax tier:\n{saved}"
    );
    assert!(
        saved.contains("release: \"1.20\""),
        "shorthand must run trigger discovery:\n{saved}"
    );
}

// --- D-8: schema work is bypassed without frontmatter ---------------------

/// D-8 counter proof. `--baseline-schema` pointing at a missing file makes
/// schema resolution *fail loudly*, so it is an observable probe for whether
/// that work ran at all.
///
/// The with-frontmatter case is the positive control: it proves the probe is
/// live. The absent- and empty-frontmatter cases then succeed only because the
/// bypass fired before any schema resolution — including the trigger-schema
/// ancestor walk — was attempted.
#[test]
fn test_absent_and_empty_frontmatter_perform_no_schema_work() {
    let repo = Repo::new();
    repo.with_release_trigger();
    let missing = repo.root.join("does-not-exist.yaml");

    // Positive control: the probe fires when frontmatter is present.
    let with_frontmatter = repo.write("docs/has-fm.md", "---\ntitle: Fine\n---\n\n# Body\n");
    md_cmd()
        .arg("clean")
        .arg(&with_frontmatter)
        .arg("--baseline-schema")
        .arg(&missing)
        .assert()
        .failure()
        .stderr(predicates::str::contains("does-not-exist.yaml"));

    // Counter proof: no frontmatter, and no empty frontmatter, do no schema
    // work at all — so the unloadable baseline is never reached.
    for (label, source) in [
        ("absent", "# Just A Body\n\nNo frontmatter here.\n"),
        ("empty", "---\n---\n\n# Body\n"),
    ] {
        let doc = repo.write(&format!("docs/{label}.md"), source);
        md_cmd()
            .arg("clean")
            .arg(&doc)
            .arg("--baseline-schema")
            .arg(&missing)
            .assert()
            .success();
    }
}

/// The same counter proof for the `--schema` layer.
#[test]
fn test_absent_frontmatter_never_resolves_the_schema_override() {
    let repo = Repo::new();
    let missing = repo.root.join("nope.yaml");

    let with_frontmatter = repo.write("docs/has-fm.md", "---\ntitle: Fine\n---\n\n# Body\n");
    md_cmd()
        .arg("clean")
        .arg(&with_frontmatter)
        .arg("--schema")
        .arg(&missing)
        .assert()
        .failure();

    let without = repo.write("docs/no-fm.md", "# Body Only\n");
    md_cmd()
        .arg("clean")
        .arg(&without)
        .arg("--schema")
        .arg(&missing)
        .assert()
        .success();
}
