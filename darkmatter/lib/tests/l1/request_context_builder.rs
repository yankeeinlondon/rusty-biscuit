//! The context builder and `ComposeRequest`: an invalid request is an error
//! at the builder, magic roots enter only through the snapshot, and an
//! expression and a file reference read one environment.
//!
//! Every fixture `HOME` and environment reaches the code through a
//! `RequestSnapshot`; no test mutates the process.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{ContextAnchor, FileReference, FileReferenceError, PathPosition, ResolutionFailure};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{
    ComposeOptions, ComposeRequest, ContextBuildError, RequestSnapshot, build_resolution_context,
};

fn canonical_temp() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    // Simplified, not `canonicalize`: Windows would return a verbatim `\\?\`
    // spelling that discovery and resolution do not use.
    let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
    (temp, root)
}

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn repository(root: &Path) -> PathBuf {
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    gix::init(&repo).expect("initialize repository");
    repo
}

/// Builder-failure input (a): a relative request directory.
#[test]
fn a_relative_request_directory_is_rejected() {
    let snapshot = RequestSnapshot::new("relative/dir");

    let error = build_resolution_context(&snapshot).unwrap_err();

    let ContextBuildError::Invalid { source, .. } = &error else {
        panic!("expected a validation failure, got {error:?}");
    };
    assert!(
        matches!(source, FileReferenceError::RelativeContextDirectory { anchor: ContextAnchor::RequestDirectory, .. }),
        "{source:?}"
    );
    assert_eq!(error.request_dir(), Path::new("relative/dir"));
    assert_eq!(error.resolution_failure(), ResolutionFailure::MissingContext);
    // The prepared request fails the same way rather than composing.
    let prepared = ComposeRequest::prepare(ComposeOptions::new(), &snapshot).unwrap_err();
    assert!(matches!(prepared, ContextBuildError::Invalid { .. }), "{prepared:?}");
}

/// Builder-failure input (b): the request directory is in a repository and
/// the opening reference, through `~` or `{{VAR}}`, resolves outside it.
#[test]
fn an_opening_reference_outside_the_request_repository_is_rejected() {
    let (_temp, root) = canonical_temp();
    let repo = repository(&root);
    let notes = root.join("home").join("notes");
    let opened = notes.join("doc.md");
    write(&opened, "# Notes\n");
    let base = RequestSnapshot::new(&repo)
        .with_home(Some(root.join("home")))
        // Exported spelling: a verbatim `\\?\` value cannot anchor `{{VAR}}/…`.
        .with_env(HashMap::from([("NOTES".to_string(), biscuit_file::to_portable_string(&notes))]));

    // Control: the same snapshot without an opening reference builds.
    let control = build_resolution_context(&base).unwrap();
    assert_eq!(control.repository_root(), Some(repo.as_path()));

    for reference in ["~/notes/doc.md", "{{NOTES}}/doc.md"] {
        let snapshot = base
            .clone()
            .with_opening_reference(FileReference::new(reference).unwrap(), opened.clone());

        let error = build_resolution_context(&snapshot).unwrap_err();

        let ContextBuildError::Invalid { source, dir } = &error else {
            panic!("{reference}: expected a validation failure, got {error:?}");
        };
        assert!(
            matches!(source, FileReferenceError::RepositoryRootNotContainingSource { .. }),
            "{reference}: {source:?}"
        );
        assert_eq!(dir, &repo, "{reference}");
        assert_eq!(error.resolution_failure(), ResolutionFailure::MissingContext, "{reference}");
    }
}

/// With no repository there is no tree to leave: the opening reference's
/// anchor becomes the tree, so the same opening builds.
#[test]
fn an_opening_reference_builds_when_the_request_has_no_repository() {
    let (_temp, root) = canonical_temp();
    let request_dir = root.join("work");
    std::fs::create_dir_all(&request_dir).unwrap();
    let opened = root.join("home").join("notes").join("doc.md");
    write(&opened, "# Notes\n");
    let snapshot = RequestSnapshot::new(&request_dir)
        .with_home(Some(root.join("home")))
        .with_opening_reference(FileReference::new("~/notes/doc.md").unwrap(), opened.clone());

    let context = build_resolution_context(&snapshot).unwrap();

    assert_eq!(context.repository_root(), None);
    assert_eq!(context.cwd(), opened.parent().unwrap());
}

/// Builder-failure input (c): a syntactically corrupt `.git/config` is a
/// discovery error, not "no repository", and repairing it builds again.
#[test]
fn a_corrupt_git_config_is_a_discovery_error_until_repaired() {
    let (_temp, root) = canonical_temp();
    let repo = repository(&root);
    let config = repo.join(".git").join("config");
    let snapshot = RequestSnapshot::new(&repo);
    std::fs::write(&config, "[core\nthis is not = = valid\n").unwrap();

    let error = build_resolution_context(&snapshot).unwrap_err();

    assert!(matches!(error, ContextBuildError::Discovery { .. }), "{error:?}");
    assert_eq!(error.request_dir(), repo.as_path());
    assert_eq!(error.resolution_failure(), ResolutionFailure::MissingContext);
    let prepared = ComposeRequest::prepare(ComposeOptions::new(), &snapshot).unwrap_err();
    assert!(matches!(prepared, ContextBuildError::Discovery { .. }), "{prepared:?}");

    std::fs::write(&config, "[core]\n\trepositoryformatversion = 0\n\tbare = false\n").unwrap();

    let repaired = build_resolution_context(&snapshot).unwrap();
    assert_eq!(repaired.repository_root(), Some(repo.as_path()));
}

/// A snapshot's extra `@` root is the one way a magic root enters a context,
/// and it reaches both the built context and a composed transclusion.
#[test]
fn a_snapshot_magic_root_resolves_an_at_reference() {
    let (_temp, root) = canonical_temp();
    let request_dir = root.join("work");
    let magic = root.join("magic");
    let target = magic.join("special.md");
    write(&target, "Special body\n");
    let document = request_dir.join("doc.md");
    write(&document, "::file @special.md\n");
    let snapshot = RequestSnapshot::new(&request_dir).with_magic_root(&magic, PathPosition::Start);

    let context = build_resolution_context(&snapshot).unwrap();
    let resolved = FileReference::new("@special.md").unwrap().resolve_in_context(&context).unwrap();
    assert_eq!(resolved.as_deref(), Some(target.as_path()));

    let request =
        ComposeRequest::prepare(ComposeOptions::new().with_source_file(&document), &snapshot).unwrap();
    let (composed, _) = Markdown::try_from(document.as_path()).unwrap().compose_with(&request).unwrap();
    assert!(composed.content().contains("Special body"), "{}", composed.content());

    // Without the root the same reference finds nothing.
    let bare = ComposeRequest::prepare(
        ComposeOptions::new().with_source_file(&document),
        &RequestSnapshot::new(&request_dir),
    )
    .unwrap();
    let missing = Markdown::try_from(document.as_path()).unwrap().compose_with(&bare);
    assert!(
        missing.as_ref().map_or(true, |(composed, _)| !composed.content().contains("Special body")),
        "{missing:?}"
    );
}

/// An expression and a `{{VAR}}` file reference (read by an expression
/// function) in one request read the snapshot's environment, not the
/// process's (which has no such variable).
#[test]
fn an_expression_and_a_file_reference_read_the_same_snapshot_environment() {
    const NAME: &str = "DARKMATTER_REQUEST_SNAPSHOT_PROBE";
    assert!(std::env::var_os(NAME).is_none(), "the process must not carry {NAME}");
    let (_temp, root) = canonical_temp();
    let fixtures = root.join("fixtures");
    write(&fixtures.join("target.md"), "# Target title\n\nTarget body\n");
    let request_dir = root.join("work");
    let document = request_dir.join("doc.md");
    write(
        &document,
        &format!(
            "value=[{{{{ env.{NAME} }}}}] agent=[{{{{ ctx.agent }}}}] title=[{{{{ markdown_title(\"{{{{{NAME}}}}}/target.md\") }}}}]\n"
        ),
    );
    let fixtures_text = biscuit_file::to_portable_string(&fixtures);
    let snapshot = RequestSnapshot::new(&request_dir).with_env(HashMap::from([
        (NAME.to_string(), fixtures_text.clone()),
        ("AGENT".to_string(), "snapshot-agent".to_string()),
    ]));

    let request =
        ComposeRequest::prepare(ComposeOptions::new().with_source_file(&document), &snapshot).unwrap();
    let (composed, _) = Markdown::try_from(document.as_path()).unwrap().compose_with(&request).unwrap();

    let content = composed.content();
    assert!(content.contains(&format!("value=[{fixtures_text}]")), "{content}");
    assert!(content.contains("agent=[snapshot-agent]"), "{content}");
    assert!(content.contains("title=[Target title]"), "{content}");
    assert_eq!(request.resolution_context().env().get(NAME), Some(&fixtures_text));
    assert_eq!(request.options().context().env().get(NAME), Some(&fixtures_text));
}

/// A transclusion's `{{VAR}}` target reads the snapshot's environment.
/// Interpolation is off so the directive target is not an expression.
#[test]
fn a_transcluded_variable_reference_reads_the_snapshot_environment() {
    const NAME: &str = "DARKMATTER_REQUEST_SNAPSHOT_TRANSCLUSION";
    assert!(std::env::var_os(NAME).is_none(), "the process must not carry {NAME}");
    let (_temp, root) = canonical_temp();
    let fixtures = root.join("fixtures");
    write(&fixtures.join("target.md"), "Target body\n");
    let document = root.join("work").join("doc.md");
    write(&document, &format!("::file \"{{{{{NAME}}}}}/target.md\"\n"));
    let snapshot = RequestSnapshot::new(root.join("work"))
        .with_env(HashMap::from([(NAME.to_string(), biscuit_file::to_portable_string(&fixtures))]));
    let options = ComposeOptions::new()
        .with_source_file(&document)
        .with_fail_fast(true)
        .only(&[darkmatter::markdown::compose::ComposeOperation::BlockTransclusion]);

    let request = ComposeRequest::prepare(options, &snapshot).unwrap();
    let (composed, _) = Markdown::try_from(document.as_path()).unwrap().compose_with(&request).unwrap();

    assert!(composed.content().contains("Target body"), "{}", composed.content());
}

/// A caller's own context is validated before it is accepted.
#[test]
fn with_context_rejects_an_invalid_context() {
    let context = biscuit_file::FileResolutionContext::from_snapshot("relative", None, HashMap::new());

    let error = ComposeRequest::with_context(ComposeOptions::new(), context).unwrap_err();

    assert!(
        matches!(
            &error,
            ContextBuildError::Invalid { source: FileReferenceError::RelativeContextDirectory { .. }, .. }
        ),
        "{error:?}"
    );
}
