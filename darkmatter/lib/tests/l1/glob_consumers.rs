//! Darkmatter's glob-reference consumers, read through their public results:
//! `find_files()` and `::file-links <glob>` list every root of a prefix,
//! merged and most local first; `file(match(...))` judges a value by its
//! nearest root, with the file-name view; and the schema grammar rejects a
//! `match()` pattern that is not a glob reference.

use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_file::{GlobReferenceError, ResolutionFailure};
use darkmatter::markdown::compose::expression::ExpressionError;
use darkmatter::markdown::compose::{ComposeOptions, ComposeReport, ComposeWarning, FileLinksError};
use darkmatter::markdown::schemas::file_match::file_match_admits;
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::{Value, json};

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, contents).expect("write fixture");
}

/// A Git repository holding Cargo package `area/pkg` (so `^` has three
/// roots) and `*.md` files at every level a consumer can reach.
fn monorepo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .expect("run git");
    assert!(status.success());
    write(&root.join("Cargo.toml"), "[workspace]\nmembers = [\"area/pkg\"]\n");
    write(
        &root.join("area/pkg/Cargo.toml"),
        "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&root.join("area/pkg/src/lib.rs"), "");
    for path in [
        "root-top.md",
        "deep/root-deep.md",
        "docs/repo-doc.md",
        "docs/_x.md",
        "area/docs/area-doc.md",
        "area/pkg/docs/pkg-doc.md",
        "area/pkg/guide/guide-top.md",
        "area/pkg/guide/sub/guide-deep.md",
        "area/pkg/guide/x/spec.md",
        "area/pkg/fixes/y/spec.md",
        "fixes/2026-09-29-ts-review-improvements/spec.md",
        "fixes/_completed/2026-01-01-old/spec.md",
        "other/z/spec.md",
    ] {
        write(&root.join(path), "# Fixture\n");
    }
    dir
}

/// Composes `document` (written with `body`) under a request prepared at its
/// own folder.
fn compose(document: &Path, body: &str, options: ComposeOptions) -> Result<(Markdown, ComposeReport), MarkdownError> {
    write(document, body);
    Markdown::try_from(document)
        .expect("document parses")
        .compose_with(&crate::request_support::request(options.with_source_file(document)))
}

/// The frontmatter value `v: "{{ <expression> }}"` composes to, in a
/// document in `dir`, and the compose report.
fn expression_value(dir: &Path, expression: &str) -> Result<(Value, ComposeReport), MarkdownError> {
    let yaml = format!("{{{{ {expression} }}}}").replace('\'', "''");
    let document = dir.join("probe.md");
    let (composed, report) = compose(&document, &format!("---\nv: '{yaml}'\n---\nBody\n"), ComposeOptions::new())?;
    let value = composed.frontmatter().as_map().get("v").cloned().unwrap_or(Value::Null);
    Ok((value, report))
}

/// `find_files()` paths, canonicalized so a symlinked temp directory
/// (macOS `/var` → `/private/var`) compares equal.
fn listed(value: &Value) -> Vec<PathBuf> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("expected an array, got {value}"))
        .iter()
        .map(|path| std::fs::canonicalize(path.as_str().expect("path string")).expect("match exists"))
        .collect()
}

fn canonical(root: &Path, relative: &str) -> PathBuf {
    std::fs::canonicalize(root.join(relative)).expect("fixture exists")
}

/// Criterion 10: a bare glob lists the document folder's matches, then the
/// repository root's; `^` lists the package's, the area's, then the
/// repository's. Criterion 11: no file-name widening outside `match()`.
#[test]
fn find_files_merges_every_root_most_local_first() {
    let fixture = monorepo();
    let root = fixture.path();
    let guide = root.join("area/pkg/guide");
    let find = |expression: &str| {
        let (value, _) = expression_value(&guide, expression).unwrap_or_else(|error| panic!("{expression}: {error}"));
        listed(&value)
    };

    assert_eq!(
        find("find_files('*.md')"),
        [
            canonical(root, "area/pkg/guide/guide-top.md"),
            canonical(root, "area/pkg/guide/probe.md"),
            canonical(root, "root-top.md"),
        ],
        "the document folder's top level, then the repository root's; no other depth",
    );
    assert_eq!(
        find("find_files('^docs/*.md')"),
        [
            canonical(root, "area/pkg/docs/pkg-doc.md"),
            canonical(root, "area/docs/area-doc.md"),
            canonical(root, "docs/_x.md"),
            canonical(root, "docs/repo-doc.md"),
        ],
    );
    assert!(find("find_files('spec.md')").is_empty(), "a bare name does not reach `x/spec.md`");
    assert!(
        find("find_files('&fixes/**/spec.md')").contains(&canonical(root, "fixes/_completed/2026-01-01-old/spec.md")),
        "no `_` filter",
    );
}

/// Criterion 10 for `::file-links`: a bare glob in a nested document lists
/// its folder's and the repository root's top-level `*.md`, and `^docs/*.md`
/// lists all three `docs/` folders. The containing document stays excluded.
#[test]
fn file_links_merges_every_root_most_local_first() {
    let fixture = monorepo();
    let root = fixture.path();
    let document = root.join("area/pkg/guide/index.md");

    let (composed, _) = compose(&document, "# Index\n\n::file-links *.md\n", ComposeOptions::new()).unwrap();
    let text = composed.content();
    for listed in ["guide-top.md", "root-top.md"] {
        assert!(text.contains(listed), "missing {listed}: {text}");
    }
    for unlisted in ["guide-deep.md", "root-deep.md", "index.md"] {
        assert!(!text.contains(unlisted), "unexpected {unlisted}: {text}");
    }

    let (composed, _) = compose(&document, "# Index\n\n::file-links ^docs/*.md\n", ComposeOptions::new()).unwrap();
    let text = composed.content();
    for listed in ["pkg-doc.md", "area-doc.md", "repo-doc.md"] {
        assert!(text.contains(listed), "missing {listed}: {text}");
    }
}

/// Criteria 1 (validation), 4, 5, 11, and 12: `match()` judges a value by
/// the reference prefix of each pattern, from the value's own `cwd`.
#[test]
fn match_validation_reads_reference_prefixes() {
    let fixture = monorepo();
    let root = fixture.path();
    let admits = |cwd: &Path, value: &str, patterns: &[&str]| {
        let patterns: Vec<String> = patterns.iter().map(|pattern| (*pattern).to_string()).collect();
        file_match_admits(value, &patterns, &crate::request_support::context_at(cwd))
    };
    let incident = "fixes/2026-09-29-ts-review-improvements/spec.md";

    // Incident 2: `^**/…` is a prefix, not a literal `^`.
    assert!(admits(root, incident, &["^**/*spec*.md"]));
    assert!(!admits(root, incident, &["^**/*plan*.md"]), "control: the glob still decides");
    // Criterion 4: a repository exclusion composes with a scoped pattern.
    let completed = "fixes/_completed/2026-01-01-old/spec.md";
    assert!(admits(root, completed, &["^**/*spec*.md"]));
    assert!(!admits(root, completed, &["^**/*spec*.md", "!&**/_completed/**"]));
    // Criterion 5: each pattern judges a file from its nearest root.
    let pkg = root.join("area/pkg");
    let nearest = ["**/*spec*.md", "!fixes/**"];
    assert!(!admits(&pkg, "fixes/y/spec.md", &nearest), "`fixes/` under the launch directory");
    assert!(!admits(&pkg, &format!("&{incident}"), &nearest), "`fixes/` under the repository root");
    assert!(admits(&pkg, "&other/z/spec.md", &nearest));
    // Criterion 11: the file-name view reaches `x/spec.md`, also through `^`.
    let guide = root.join("area/pkg/guide");
    assert!(admits(&guide, "x/spec.md", &["spec.md"]));
    assert!(admits(&guide, "x/spec.md", &["^spec.md"]));
    assert!(!admits(&guide, "x/spec.md", &["^other.md"]));
    // Criterion 12: a file-name negation rejects at any depth.
    assert!(admits(root, "docs/repo-doc.md", &["*.md", "!_*.md"]));
    assert!(!admits(root, "docs/_x.md", &["*.md", "!_*.md"]));
}

/// Criterion 7 at the compose boundary: in a root union, a caller-supplied
/// file is judged from the launch directory (`area/`), so `match(docs/*.md)`
/// admits `docs/area-doc.md` although the document lives in `prompts/`, where
/// the same value is `area/docs/area-doc.md` from the repository root. The
/// other arm requires `severity`, so only the glob can make the value valid.
#[test]
fn a_caller_value_is_judged_from_the_launch_directory() {
    let fixture = monorepo();
    let root = fixture.path();
    let document = root.join("prompts/union.md");
    write(
        &document,
        "---\n$schema:\n  - spec: file(eager; match(docs/*.md))\n  - spec: file(eager; match(**/*spec*.md))\n    severity: string(required)\n---\nBody\n",
    );
    let compose_from = |launch: &Path, spec: &str| {
        let options = ComposeOptions::new()
            .with_source_file(&document)
            .with_set_overrides(json!({ "spec": spec }));
        let snapshot = darkmatter::markdown::compose::RequestSnapshot::new(launch)
            .with_home(biscuit_file::home_dir());
        let request = darkmatter::markdown::compose::ComposeRequest::prepare(options, &snapshot).expect("request");
        Markdown::try_from(document.as_path()).unwrap().compose_with(&request)
    };
    let area = root.join("area");
    compose_from(&area, "docs/area-doc.md")
        .unwrap_or_else(|error| panic!("judged from the launch directory: {error:?}"));
    // Control: a value neither arm's glob admits needs the `severity` arm.
    let error = compose_from(&area, "../other/z/spec.md").expect_err("the second arm needs `severity`");
    assert!(format!("{error:?}").contains("severity"), "{error:?}");
}

/// Criterion 8: a `match()` pattern that is not a glob reference is a schema
/// definition error naming the property and the pattern.
#[test]
fn match_rejects_patterns_that_are_not_glob_references() {
    let parse = |definition: &str| {
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&format!("spec: '{definition}'")).unwrap();
        darkmatter::markdown::schemas::simplified::parse_yaml_schema(&yaml)
    };
    parse("file(match(vault:notes/*.md))").expect("a vault prefix is valid");
    parse("file(match(^**/*spec*.md, !&**/_completed/**))").expect("control");
    for (pattern, fragment) in [
        ("%**/*.md", "`%` recursive modifier"),
        ("https://example.com/*.md", "remote URL"),
        ("docs/[", "not a valid glob"),
        ("!*.md", "not a `!` exclusion"),
    ] {
        let error = parse(&format!("file(match({pattern}))"))
            .expect_err(pattern)
            .to_string();
        assert!(error.contains("spec"), "{pattern}: names the property: {error}");
        assert!(error.contains(fragment), "{pattern}: expected `{fragment}` in: {error}");
        if !pattern.starts_with('!') {
            assert!(error.contains(pattern), "{pattern}: names the pattern: {error}");
        }
    }
}

/// Criterion 14: a bare or `./` glob that climbs above the repository is a
/// typed `RelativeTreeEscape`; `&` outside a repository is `OutsideRepository`.
#[test]
fn relative_globs_stay_inside_the_repository() {
    let workspace = tempfile::tempdir().expect("temp dir");
    let repo = workspace.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    assert!(Command::new("git").args(["init", "-q"]).current_dir(&repo).status().unwrap().success());
    write(&workspace.path().join("outside/a.md"), "# Outside\n");

    for pattern in ["../outside/*.md", "./../outside/*.md"] {
        let error = expression_value(&repo, &format!("find_files('{pattern}')")).expect_err(pattern);
        assert!(
            matches!(
                &error,
                MarkdownError::Interpolation { cause, .. } if matches!(
                    cause.as_ref(),
                    ExpressionError::GlobReference { source, .. }
                        if matches!(source.as_ref(), GlobReferenceError::RelativeTreeEscape { .. })
                )
            ),
            "find_files('{pattern}'): {error:?}"
        );
        assert_eq!(error.resolution_failure(), Some(ResolutionFailure::InvalidReference));

        let error = compose(
            &repo.join("index.md"),
            &format!("# Index\n\n::file-links {pattern}\n"),
            ComposeOptions::new().with_fail_fast(true),
        )
        .expect_err(pattern);
        assert!(
            matches!(
                &error,
                MarkdownError::FileLinks(FileLinksError::GlobReference {
                    source: GlobReferenceError::RelativeTreeEscape { .. },
                    ..
                })
            ),
            "::file-links {pattern}: {error:?}"
        );
    }
    // An absolute root is not relative, so it may lie outside the repository.
    let outside = biscuit_file::to_portable_string(&workspace.path().join("outside"));
    let (value, _) = expression_value(&repo, &format!("find_files('{outside}/*.md')")).unwrap();
    assert_eq!(listed(&value), [canonical(workspace.path(), "outside/a.md")]);

    // `&` needs a repository.
    let plain = tempfile::tempdir().expect("temp dir");
    let error = expression_value(plain.path(), "find_files('&*.md')").expect_err("no repository");
    assert_eq!(error.resolution_failure(), Some(ResolutionFailure::MissingContext), "{error:?}");
}

/// Criterion 25: a bound glob leaves out a file symlink whose target is
/// outside the repository and reports it once, naming the link and target;
/// an in-tree file symlink is listed as before.
#[test]
fn an_out_of_tree_file_symlink_is_skipped_with_one_warning() {
    let workspace = tempfile::tempdir().expect("temp dir");
    let repo = workspace.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    assert!(Command::new("git").args(["init", "-q"]).current_dir(&repo).status().unwrap().success());
    let secret = workspace.path().join("secret.md");
    write(&secret, "# Secret\n");
    write(&repo.join("docs/inside.md"), "# Inside\n");
    write(&repo.join("notes/target.md"), "# Target\n");
    link_file(&secret, &repo.join("docs/leak.md"));
    link_file(&repo.join("notes/target.md"), &repo.join("docs/alias.md"));

    let skipped = |report: &ComposeReport| -> Vec<ComposeWarning> {
        report
            .warnings
            .iter()
            .filter(|warning| warning.code.as_deref() == Some(ComposeWarning::GLOB_SKIPPED_SYMLINK_CODE))
            .cloned()
            .collect()
    };
    let names_link_and_target = |warning: &ComposeWarning| {
        let target = biscuit_file::to_portable_string(&std::fs::canonicalize(&secret).unwrap());
        warning.message.contains("docs/leak.md") && warning.message.contains(&target)
    };

    // `find_files()`, evaluated twice in one document: still one warning.
    let document = repo.join("probe.md");
    let (composed, report) = compose(
        &document,
        "---\nv: \"{{ find_files('docs/*.md') }}\"\nw: \"{{ find_files('docs/*.md') }}\"\n---\nBody\n",
        ComposeOptions::new(),
    )
    .unwrap();
    let names: Vec<_> = listed(composed.frontmatter().as_map().get("v").unwrap())
        .into_iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"inside.md".to_string()), "{names:?}");
    assert!(!names.contains(&"leak.md".to_string()), "{names:?}");
    let warnings = skipped(&report);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(names_link_and_target(&warnings[0]), "{:?}", warnings[0]);

    // `::file-links`.
    let (composed, report) = compose(&repo.join("index.md"), "# Index\n\n::file-links docs/*.md\n", ComposeOptions::new()).unwrap();
    assert!(!composed.content().contains("leak.md"), "{}", composed.content());
    assert!(composed.content().contains("alias.md"), "an in-tree link is listed: {}", composed.content());
    let warnings = skipped(&report);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(names_link_and_target(&warnings[0]), "{:?}", warnings[0]);
}

fn link_file(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).expect("symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, link)
        .expect("creating a file symlink needs Developer Mode or SeCreateSymbolicLinkPrivilege");
}
