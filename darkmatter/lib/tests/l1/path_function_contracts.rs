//! Path-function contracts through the public expression evaluator.
//!
//! Each row names one reference shape and states, per function, its own
//! return type and missing-target contract: `absolute` builds a native
//! absolute path (and no longer proves existence), `dirname` and the
//! file-index family keep an absolute input's anchor, and `file_exists`
//! answers `false` only for a genuinely absent file.
//!
//! Every context is anchored in a canonical temporary directory with a
//! private home and an empty environment; nothing is read from the process
//! directory. References reach the evaluator as variables so Windows-native
//! spellings never pass through expression string escaping.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReferenceError, FileResolutionContext, to_portable_string};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ExpressionError, ResolutionContext, evaluate, parse,
};
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions, ComposeRequest};
use serde_json::{Value, json};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    repo: PathBuf,
    doc: PathBuf,
    home: PathBuf,
}

/// `<root>/repo` (with `.git`), document directory `<root>/repo/prompts`, and
/// a private home `<root>/home`.
fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
    let repo = root.join("repo");
    let doc = repo.join("prompts");
    let home = root.join("home");
    for dir in [repo.join(".git"), doc.clone(), home.clone()] {
        std::fs::create_dir_all(dir).unwrap();
    }
    Fixture {
        _temp: temp,
        root,
        repo,
        doc,
        home,
    }
}

fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn under(base: &Path, segments: &[&str]) -> PathBuf {
    segments.iter().fold(base.to_path_buf(), |path, segment| path.join(segment))
}

fn repo_context(f: &Fixture) -> FileResolutionContext {
    FileResolutionContext::from_snapshot(&f.doc, Some(f.home.clone()), HashMap::new())
        .with_repository_root(&f.repo)
}

struct Lookup {
    context: ResolutionContext,
    reference: String,
}

impl EvaluationLookup for Lookup {
    fn get(&self, path: &str) -> Option<Value> {
        (path == "p").then(|| json!(self.reference))
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        Some(self.context.clone())
    }
}

fn eval(context: &FileResolutionContext, function: &str, reference: &str) -> Result<Value, ExpressionError> {
    let lookup = Lookup {
        context: ResolutionContext::new(context.clone()),
        reference: reference.to_string(),
    };
    evaluate(&parse(&format!("{function}(p)")).unwrap(), &lookup)
}

/// What one function returns for one row.
#[derive(Debug)]
enum Expect {
    /// A native absolute path, compared by component identity.
    Native(PathBuf),
    /// Exact string output (portable spelling for an absolute result).
    Text(String),
    Bool(bool),
    /// `<function>() does not accept HTTP(S) URLs`, without a fetch.
    UrlRejected,
    /// A file-reference diagnostic of this kind whose typed cause matches.
    FileRef(&'static str, fn(&FileReferenceError) -> bool),
}

fn text(value: impl Into<String>) -> Expect {
    Expect::Text(value.into())
}

fn portable(path: &Path) -> Expect {
    Expect::Text(to_portable_string(path))
}

fn check(context: &FileResolutionContext, row: &str, function: &str, reference: &str, expect: &Expect) {
    let actual = eval(context, function, reference);
    let label = format!("row {row}: {function}({reference:?})");
    match (expect, &actual) {
        (Expect::Native(expected), Ok(Value::String(path))) => {
            assert!(Path::new(path).is_absolute(), "{label}: {path} is not absolute");
            assert_eq!(Path::new(path), expected.as_path(), "{label}");
            assert!(
                !path.contains("/./") && !path.contains("/../"),
                "{label}: {path} keeps dot segments"
            );
        }
        (Expect::Text(expected), Ok(Value::String(value))) => assert_eq!(value, expected, "{label}"),
        (Expect::Bool(expected), Ok(Value::Bool(value))) => assert_eq!(value, expected, "{label}"),
        (Expect::UrlRejected, Err(error)) => assert!(
            error.to_string().contains(&format!("{function}() does not accept HTTP(S) URLs")),
            "{label}: {error}"
        ),
        (Expect::FileRef(kind, cause), Err(ExpressionError::FileReference(diagnostic))) => {
            assert_eq!(diagnostic.kind.as_str(), *kind, "{label}");
            let source = diagnostic.source.as_deref();
            assert!(source.is_some_and(cause), "{label}: unexpected cause {source:?}");
        }
        _ => panic!("{label}: expected {expect:?}, got {actual:?}"),
    }
}

struct Row {
    name: &'static str,
    reference: String,
    absolute: Expect,
    dirname: Expect,
    increment: Expect,
    file_exists: Expect,
}

fn run(context: &FileResolutionContext, rows: &[Row]) {
    for row in rows {
        check(context, row.name, "absolute", &row.reference, &row.absolute);
        check(context, row.name, "dirname", &row.reference, &row.dirname);
        check(context, row.name, "increment_file_index", &row.reference, &row.increment);
        check(context, row.name, "file_exists", &row.reference, &row.file_exists);
    }
}

fn tree_escape(error: &FileReferenceError) -> bool {
    matches!(error, FileReferenceError::RelativeTreeEscape { .. })
}

fn vault_unconfigured(error: &FileReferenceError) -> bool {
    matches!(error, FileReferenceError::VaultNotConfigured)
}

fn missing_home(error: &FileReferenceError) -> bool {
    matches!(error, FileReferenceError::MissingHomeContext)
}

fn outside_repository(error: &FileReferenceError) -> bool {
    matches!(error, FileReferenceError::OutsideRepository { .. })
}

#[test]
fn path_functions_follow_document_first_resolution_inside_a_repository() {
    let f = fixture();
    write(&f.doc.join("notes.md"), "---\nsrc: document\n---\n");
    write(&f.repo.join("notes.md"), "---\nsrc: repository\n---\n");
    write(&under(&f.repo, &["sub", "x.md"]), "---\nsrc: repository\n---\n");
    write(&f.repo.join("up.md"), "");
    let context = repo_context(&f);

    run(
        &context,
        &[
            Row {
                name: "1 collision",
                reference: "notes.md".into(),
                absolute: Expect::Native(f.doc.join("notes.md")),
                dirname: text("prompts"),
                increment: text("prompts/notes-2.md"),
                file_exists: Expect::Bool(true),
            },
            Row {
                name: "1b repository fallback",
                reference: "sub/x.md".into(),
                absolute: Expect::Native(under(&f.repo, &["sub", "x.md"])),
                dirname: text("sub"),
                increment: text("sub/x-2.md"),
                file_exists: Expect::Bool(true),
            },
            Row {
                name: "2 explicit ./ missing",
                reference: "./sub/x.md".into(),
                absolute: Expect::Native(under(&f.doc, &["sub", "x.md"])),
                dirname: text("prompts/./sub"),
                increment: text("prompts/./sub/x-2.md"),
                file_exists: Expect::Bool(false),
            },
            Row {
                name: "2b explicit ../ existing",
                reference: "../up.md".into(),
                absolute: Expect::Native(f.repo.join("up.md")),
                dirname: text(""),
                increment: text("up-2.md"),
                file_exists: Expect::Bool(true),
            },
            Row {
                name: "2b explicit ../ missing",
                reference: "../gone.md".into(),
                absolute: Expect::Native(f.repo.join("gone.md")),
                dirname: text("prompts/.."),
                increment: text("prompts/../gone-2.md"),
                file_exists: Expect::Bool(false),
            },
            Row {
                name: "2c tree escape",
                reference: "../../escape.md".into(),
                absolute: Expect::FileRef("not_found", tree_escape),
                dirname: Expect::FileRef("not_found", tree_escape),
                increment: Expect::FileRef("not_found", tree_escape),
                file_exists: Expect::FileRef("not_found", tree_escape),
            },
            Row {
                name: "6 implicit missing",
                reference: "sub/new.md".into(),
                absolute: Expect::Native(under(&f.doc, &["sub", "new.md"])),
                dirname: text("prompts/sub"),
                increment: text("prompts/sub/new-2.md"),
                file_exists: Expect::Bool(false),
            },
            Row {
                name: "8 spaces",
                reference: "my dir/a b.md".into(),
                absolute: Expect::Native(under(&f.doc, &["my dir", "a b.md"])),
                dirname: text("prompts/my dir"),
                increment: text("prompts/my dir/a b-2.md"),
                file_exists: Expect::Bool(false),
            },
        ],
    );

    // Reads keep their documented targets: the document copy wins a
    // collision, and a missing target is an error rather than a value.
    let lookup = |reference: &str| Lookup {
        context: ResolutionContext::new(context.clone()),
        reference: reference.to_string(),
    };
    let read = |reference: &str| evaluate(&parse("frontmatter(p, \"src\")").unwrap(), &lookup(reference));
    assert_eq!(read("notes.md").unwrap(), json!("document"));
    assert_eq!(read("sub/x.md").unwrap(), json!("repository"));
    assert!(matches!(
        read("sub/new.md"),
        Err(ExpressionError::FileReference(diagnostic)) if diagnostic.kind.as_str() == "not_found"
    ));
}

#[test]
fn prefixed_references_keep_their_own_anchors() {
    let f = fixture();
    write(&under(&f.home, &["notes", "x.md"]), "");
    let magic = f.repo.join("magic");
    write(&magic.join("found.md"), "");
    let context = repo_context(&f).add_magic_path(&magic, biscuit_file::PathPosition::Start);

    run(
        &context,
        &[
            Row {
                name: "4a magic, existing",
                reference: "@found.md".into(),
                absolute: Expect::Native(magic.join("found.md")),
                dirname: text("magic"),
                increment: text("magic/found-2.md"),
                file_exists: Expect::Bool(true),
            },
            Row {
                name: "4a magic, missing",
                reference: "@x.md".into(),
                absolute: Expect::Native(magic.join("x.md")),
                dirname: text("magic"),
                increment: text("magic/x-2.md"),
                file_exists: Expect::Bool(false),
            },
            Row {
                name: "4b home",
                reference: "~/notes/x.md".into(),
                absolute: Expect::Native(under(&f.home, &["notes", "x.md"])),
                dirname: text("~/notes"),
                increment: text("~/notes/x-2.md"),
                file_exists: Expect::Bool(true),
            },
            Row {
                name: "4c repository root, missing",
                reference: "&docs/x.md".into(),
                absolute: Expect::Native(under(&f.repo, &["docs", "x.md"])),
                dirname: text("docs"),
                increment: text("docs/x-2.md"),
                file_exists: Expect::Bool(false),
            },
            Row {
                name: "4e vault unconfigured",
                reference: "vault:x.md".into(),
                absolute: Expect::FileRef("not_found", vault_unconfigured),
                dirname: Expect::FileRef("not_found", vault_unconfigured),
                increment: Expect::FileRef("not_found", vault_unconfigured),
                file_exists: Expect::FileRef("not_found", vault_unconfigured),
            },
        ],
    );

    let homeless = FileResolutionContext::from_snapshot(&f.doc, None, HashMap::new())
        .with_repository_root(&f.repo);
    run(
        &homeless,
        &[Row {
            name: "4b no home",
            reference: "~/x.md".into(),
            absolute: Expect::FileRef("not_found", missing_home),
            dirname: Expect::FileRef("not_found", missing_home),
            increment: Expect::FileRef("not_found", missing_home),
            file_exists: Expect::FileRef("not_found", missing_home),
        }],
    );
}

#[test]
fn recursive_reference_without_a_match_is_not_a_path() {
    let f = fixture();
    let context = repo_context(&f);
    let error = eval(&context, "absolute", "%x.md").unwrap_err();
    let ExpressionError::FileReference(diagnostic) = error else {
        panic!("expected a file-reference diagnostic, got {error:?}");
    };
    assert_eq!(diagnostic.kind.as_str(), "not_found");
    assert!(diagnostic.source.is_none(), "a recursive miss is a clean not_found");
}

#[test]
fn absolute_references_keep_their_anchor_inside_and_outside_every_root() {
    let f = fixture();
    let spec = under(&f.repo, &["claudine", "features", "x", "spec.md"]);
    write(&spec, "");
    let review = spec.with_file_name("review-1.md");
    let outside = under(&f.root, &["other", "a", "b.md"]);
    let context = repo_context(&f);
    let reference = |path: &Path| path.to_string_lossy().into_owned();

    run(
        &context,
        &[
            Row {
                name: "11 absolute inside the repository",
                reference: reference(&spec),
                absolute: Expect::Native(spec.clone()),
                dirname: portable(spec.parent().unwrap()),
                increment: portable(&spec.with_file_name("spec-2.md")),
                file_exists: Expect::Bool(true),
            },
            Row {
                name: "11 absolute sibling, missing",
                reference: reference(&review),
                absolute: Expect::Native(review.clone()),
                dirname: portable(spec.parent().unwrap()),
                increment: portable(&spec.with_file_name("review-2.md")),
                file_exists: Expect::Bool(false),
            },
            Row {
                name: "5 absolute outside every root",
                reference: reference(&outside),
                absolute: Expect::Native(outside.clone()),
                dirname: portable(outside.parent().unwrap()),
                increment: portable(&outside.with_file_name("b-2.md")),
                file_exists: Expect::Bool(false),
            },
        ],
    );

    // `find_*` scan the resolved parent and keep the absolute anchor too.
    write(&review, "");
    write(&spec.with_file_name("review-3.md"), "");
    check(&context, "11", "find_last_index", &reference(&review), &portable(&spec.with_file_name("review-3.md")));
    check(&context, "11", "find_first_index", &reference(&spec.with_file_name("review-3.md")), &portable(&review));
    check(&context, "11", "decrement_file_index", &reference(&review), &portable(&spec.with_file_name("review-0.md")));
}

#[test]
fn outside_a_repository_no_root_is_invented() {
    let f = fixture();
    let docs = under(&f.root, &["loose", "docs"]);
    std::fs::create_dir_all(&docs).unwrap();
    let context = FileResolutionContext::from_snapshot(&docs, Some(f.home.clone()), HashMap::new());

    run(
        &context,
        &[
            Row {
                name: "7 implicit",
                reference: "x.md".into(),
                absolute: Expect::Native(docs.join("x.md")),
                dirname: text(""),
                increment: text("x-2.md"),
                file_exists: Expect::Bool(false),
            },
            Row {
                name: "7 repository sigil",
                reference: "&x.md".into(),
                absolute: Expect::FileRef("not_found", outside_repository),
                dirname: Expect::FileRef("not_found", outside_repository),
                increment: Expect::FileRef("not_found", outside_repository),
                file_exists: Expect::FileRef("not_found", outside_repository),
            },
            // With no repository the tree root is only a fallback, not a
            // boundary, so `../` may climb out of the document directory.
            Row {
                name: "7 explicit ../",
                reference: "../x.md".into(),
                absolute: Expect::Native(under(&f.root, &["loose", "x.md"])),
                dirname: text(".."),
                increment: text("../x-2.md"),
                file_exists: Expect::Bool(false),
            },
        ],
    );
}

#[test]
fn path_building_functions_reject_urls_without_fetching() {
    let f = fixture();
    let context = repo_context(&f);
    for url in ["https://example.invalid/x.md", "HTTP://example.invalid/x.md"] {
        for function in [
            "absolute",
            "dirname",
            "increment_file_index",
            "decrement_file_index",
            "find_first_index",
            "find_last_index",
        ] {
            check(&context, "9", function, url, &Expect::UrlRejected);
        }
        // With no remote runtime, existence of a URL cannot be answered.
        let error = eval(&context, "file_exists", url).unwrap_err();
        assert!(error.to_string().contains("local-only"), "{error}");
    }
}

#[cfg(unix)]
#[test]
fn probe_errors_are_never_missing_files() {
    use std::os::unix::fs::PermissionsExt;

    let f = fixture();
    let locked = f.doc.join("locked");
    write(&locked.join("x.md"), "---\nsrc: locked\n---\n");
    let context = repo_context(&f);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    // Running as root bypasses the permission bits; there is no probe error
    // to observe then.
    let bypassed = std::fs::metadata(locked.join("x.md")).is_ok();
    let probe_error = |error: &FileReferenceError| matches!(error, FileReferenceError::Io { .. });
    let outcome = std::panic::catch_unwind(|| {
        if bypassed {
            return;
        }
        for function in ["absolute", "dirname", "increment_file_index", "file_exists", "frontmatter"] {
            check(&context, "10", function, "locked/x.md", &Expect::FileRef("not_found", probe_error));
        }
    });
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

/// A prompt whose caller supplies `spec` from another directory derives the
/// review sibling beside the spec, whether the prompt lives in the repository
/// or outside it.
#[test]
fn caller_file_siblings_stay_beside_the_caller_file() {
    let f = fixture();
    let launch = f.repo.join("pkg");
    let case_dir = under(&launch, &["fixes", "c"]);
    let spec = case_dir.join("spec.md");
    write(&spec, "# Spec\n");
    let in_repo = f.doc.join("review-loop.md");
    let external = under(&f.root, &["user-prompts", "review-loop.md"]);
    let body = "---\n\
                $schema:\n\
                \x20 spec: file(required)\n\
                ---\n\
                ABS={{ absolute(spec) }}\n\
                DIR={{ dirname(absolute(spec)) }}\n\
                LATEST={{ dirname(absolute(spec)) + '/review-1.md' }}\n\
                EXISTS={{ file_exists(dirname(absolute(spec)) + '/review-1.md') }}\n\
                NEXT={{ increment_file_index(dirname(absolute(spec)) + '/review-1.md') }}\n";
    write(&in_repo, body);
    write(&external, body);

    let compose = |prompt: &Path| -> String {
        let context = FileResolutionContext::from_snapshot(&launch, Some(f.home.clone()), HashMap::new())
            .with_repository_root(&f.repo)
            .with_source_path(prompt);
        let options = ComposeOptions::new()
            .with_source_file(prompt)
            .with_file_ref_fallback_dir(&launch)
            .with_set_overrides(json!({ "spec": "fixes/c/spec.md" }))
            .only(&[ComposeOperation::FrontmatterInterpolation, ComposeOperation::Interpolation]);
        let request = ComposeRequest::with_context(options, context).unwrap();
        Markdown::try_from(prompt).unwrap().compose_with(&request).unwrap().0.content().to_string()
    };

    let dir = to_portable_string(&case_dir);
    for (row, prompt) in [("11", &in_repo), ("11b", &external)] {
        let line = |content: &str, key: &str| -> String {
            content
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .unwrap_or_else(|| panic!("row {row}: no {key} in {content}"))
                .to_string()
        };
        let missing = compose(prompt);
        assert_eq!(Path::new(&line(&missing, "ABS")), spec.as_path(), "row {row}");
        assert_eq!(line(&missing, "DIR"), dir, "row {row}");
        assert_eq!(line(&missing, "LATEST"), format!("{dir}/review-1.md"), "row {row}");
        assert_eq!(line(&missing, "EXISTS"), "false", "row {row}");
        assert_eq!(line(&missing, "NEXT"), format!("{dir}/review-2.md"), "row {row}");

        write(&case_dir.join("review-1.md"), "");
        assert_eq!(line(&compose(prompt), "EXISTS"), "true", "row {row}");
        std::fs::remove_file(case_dir.join("review-1.md")).unwrap();
    }
}

#[cfg(windows)]
#[test]
fn windows_absolute_inputs_keep_a_portable_drive_anchor() {
    let f = fixture();
    let spec = under(&f.repo, &["area", "spec.md"]);
    write(&spec, "");
    let context = repo_context(&f);
    let native = spec.to_string_lossy().into_owned();
    assert!(native.contains('\\'), "fixture must use a native spelling: {native}");
    check(&context, "win", "absolute", &native, &Expect::Native(spec.clone()));
    check(&context, "win", "dirname", &native, &portable(spec.parent().unwrap()));
    let dirname = to_portable_string(spec.parent().unwrap());
    assert!(!dirname.contains('\\') && dirname.contains(':'), "{dirname}");
}
