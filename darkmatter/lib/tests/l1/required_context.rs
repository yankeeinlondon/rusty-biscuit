//! Surfaces with no document path resolve through the request's context, never
//! the process's directory, home, or environment.
//!
//! Each test runs from the test process's own directory (the crate root), which
//! holds none of the fixture files, so a result that depends on the process
//! directory fails. Every fixture directory is a fresh temporary directory, and
//! no test mutates the process environment or directory.

use std::collections::HashMap;
use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::conditions::evaluate_condition_against;
use darkmatter::markdown::compose::shell_expansion::store::resolve_policy_paths;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
};
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOptions, ComposeRequest, ComposeSource, RequestSnapshot, ShellExpansionError,
    build_resolution_context,
};
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, PropertyDef, SimplifiedType, TypeExpr, ValidatorCache, detect_from_document,
};
use serde_json::json;
use tempfile::TempDir;

use crate::request_support::{context_at, request_at};

/// A fixture name no real environment defines.
const PROBE: &str = "DM_REQUIRED_CONTEXT_PROBE";

/// A string document's eager `file` value resolves from the request
/// directory, and a miss names that directory as the one it resolved from.
#[test]
fn a_string_document_resolves_file_values_from_the_request_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("target.md"), "# Target\n").unwrap();
    let api = DarkmatterSchemas::new(context_at(dir.path()));

    let found: Markdown = "---\n$schema:\n  spec: file(eager)\nspec: ./target.md\n---\nbody\n".into();
    let report = api.validate(&found).unwrap();
    assert!(report.valid, "the request directory holds the file: {:?}", report.problems);

    let missing: Markdown = "---\n$schema:\n  spec: file(eager)\nspec: ./missing.md\n---\nbody\n".into();
    let report = api.validate(&missing).unwrap();
    assert!(!report.valid, "a missing file is a problem");
    let message = &report.problems[0].message;
    assert!(
        message.contains(&dir.path().display().to_string()),
        "the miss names the request directory, not the process's: {message}",
    );
}

/// `env.*` in the standalone condition API reads the context's environment.
#[test]
fn a_condition_reads_env_from_the_context_not_the_process() {
    let dir = TempDir::new().unwrap();
    assert!(std::env::var_os(PROBE).is_none(), "the process must not define {PROBE}");
    let with_probe = RequestSnapshot::new(dir.path())
        .with_env(HashMap::from([(PROBE.to_string(), "yes".to_string())]));
    let context = build_resolution_context(&with_probe).unwrap();

    let expression = format!("env.{PROBE} == 'yes'");
    assert!(evaluate_condition_against(&expression, &json!({}), &context).unwrap());

    let without = build_resolution_context(&RequestSnapshot::new(dir.path())).unwrap();
    assert!(!evaluate_condition_against(&expression, &json!({}), &without).unwrap());
}

/// Read-side functions in the condition API resolve from the context's
/// directory, quoted and bare paths alike.
#[test]
fn a_condition_resolves_files_from_the_context_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("present.md"), "x").unwrap();
    let context = build_resolution_context(&RequestSnapshot::new(dir.path())).unwrap();

    for expression in ["file_exists('./present.md')", "file_exists('present.md')"] {
        assert!(
            evaluate_condition_against(expression, &json!({}), &context).unwrap(),
            "{expression} resolves from the context directory",
        );
    }
    assert!(!evaluate_condition_against("file_exists('./absent.md')", &json!({}), &context).unwrap());
}

/// Schema detection infers `file` for a string document's value that names a
/// file in the context's directory, and `string` once the file is gone.
#[test]
fn detection_infers_file_values_from_the_context_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("present.md"), "x").unwrap();
    let context = context_at(dir.path());
    let md: Markdown = "---\nspec: ./present.md\nnote: ./absent.md\n---\n".into();

    let shape = detect_from_document(&md, &context);

    let ty = |key: &str| match &shape.properties[key] {
        PropertyDef::Single(atom) => atom.ty.clone(),
        other => panic!("{key} detected as {other:?}"),
    };
    assert_eq!(ty("spec"), TypeExpr::Primitive(SimplifiedType::File));
    assert_eq!(ty("note"), TypeExpr::Primitive(SimplifiedType::String));
}

/// A structural validator, used where there is no request, judges a `file`
/// value that needs a context by its syntax alone; an absolute path needs
/// none, so it is judged as a context-bound validator judges it.
#[test]
fn structural_validators_resolve_only_context_free_file_values() {
    let dir = TempDir::new().unwrap();
    for tree in ["fixes", "features"] {
        std::fs::create_dir_all(dir.path().join(tree).join("x")).unwrap();
        std::fs::write(dir.path().join(tree).join("x/spec.md"), "x").unwrap();
    }
    let fix = biscuit_file::to_portable_string(&dir.path().join("fixes/x/spec.md"));
    let feature = biscuit_file::to_portable_string(&dir.path().join("features/x/spec.md"));
    let missing = biscuit_file::to_portable_string(&dir.path().join("fixes/x/absent.md"));
    let schema = json!({
        "type": "object",
        "properties": {
            "spec": { "type": "string", "format": "darkmatter-file" },
            "fix": { "type": "string", "x-darkmatter-match": ["**/fixes/**/spec.md"] }
        }
    });
    let cache = ValidatorCache::with_capacity(4);
    let structural = cache.structural_validator_for(&schema).unwrap();

    let relative_missing = json!({ "spec": "./missing.md" });
    assert!(structural.is_valid(&relative_missing), "a relative value is judged by syntax");
    assert!(!structural.is_valid(&json!({ "spec": "" })), "an empty reference is malformed");
    assert!(structural.is_valid(&json!({ "spec": fix })), "an existing absolute path");
    assert!(!structural.is_valid(&json!({ "spec": missing })), "a missing absolute path");
    assert!(structural.is_valid(&json!({ "fix": fix })), "the glob accepts its tree");
    assert!(!structural.is_valid(&json!({ "fix": feature })), "the glob rejects another tree");
    assert!(structural.is_valid(&json!({ "fix": "features/x/spec.md" })), "a relative value is not judged");

    let resolved = cache.validator_for(&schema, Some(dir.path()), &context_at(dir.path())).unwrap();
    assert!(!resolved.is_valid(&relative_missing), "a context-bound validator checks existence");
}

/// A source with no directory keeps its shell policy files in the request
/// directory when the context names no repository or home.
#[test]
fn a_pathless_source_keeps_shell_policy_in_the_request_directory() {
    let dir = TempDir::new().unwrap();
    let context = build_resolution_context(&RequestSnapshot::new(dir.path())).unwrap();
    assert_eq!(context.repository_root(), None);
    assert_eq!(context.home_dir(), None);

    for source in [ComposeSource::Unknown, ComposeSource::File("doc.md".into())] {
        let paths =
            resolve_policy_paths(&ShellExpansionOptions::default(), &source, &context).unwrap();
        assert_eq!(paths.whitelist, dir.path().join(".darkmatter-shell-whitelist"), "{source:?}");
        assert_eq!(paths.blacklist, dir.path().join(".darkmatter-shell-blacklist"), "{source:?}");
    }
}

/// `ComposeOptions::new` captures no process directory; a prepared request
/// anchors its context on the request directory instead.
#[test]
fn new_options_are_anchored_only_by_their_request() {
    let dir = TempDir::new().unwrap();
    let options = ComposeOptions::new();
    assert_eq!(options.context().anchor(), Path::new(""));

    let request = request_at(dir.path(), options);
    assert_eq!(request.context().anchor(), dir.path());
    assert_eq!(request.resolution_context().request_cwd(), dir.path());
}

/// A string document's absolute link normalizes against the request's
/// context: the request directory's repository makes it portable, where the
/// process directory (outside that repository) could only keep it absolute.
#[test]
fn a_string_documents_links_normalize_against_the_request_context() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(repo.join("docs")).unwrap();
    gix::init(&repo).unwrap();
    let target = repo.join("docs/guide.md");
    std::fs::write(&target, "# Guide\n").unwrap();
    let absolute = biscuit_file::to_portable_string(&target);
    let md: Markdown = format!("See [the guide]({absolute}).\n").into();

    let request = request_at(&repo.join("docs"), ComposeOptions::new());
    let (composed, report) = md.compose_with(&request).unwrap();

    assert_eq!(report.link_normalizations_applied, 1, "{}", composed.content());
    assert!(
        !composed.content().contains(&absolute),
        "the link is portable, not absolute: {}",
        composed.content(),
    );
    assert!(composed.content().contains("guide.md"), "{}", composed.content());
}

/// A string document's relative link resolves from the request directory.
/// The process directory (the crate root) holds no `guide.md`, so a resolve
/// through the process would leave the link untouched.
#[test]
fn a_string_documents_relative_link_resolves_from_the_request_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("guide.md"), "# Guide\n").unwrap();
    assert!(!Path::new("guide.md").exists(), "the process directory must not hold the target");
    let md: Markdown = "See [the guide](guide.md).\n".into();

    let (composed, report) = md.compose_with(&request_at(dir.path(), ComposeOptions::new())).unwrap();
    assert_eq!(report.link_resolves_applied, 1, "{}", composed.content());

    let missing: Markdown = "See [nothing](absent.md).\n".into();
    let (composed, report) = missing.compose_with(&request_at(dir.path(), ComposeOptions::new())).unwrap();
    assert_eq!(report.link_resolves_applied, 0, "a miss has no anchor to absolutize: {}", composed.content());
    assert!(composed.content().contains("(absent.md)"), "{}", composed.content());
}

/// Approves every shell command once.
struct ApproveAll;

impl ShellApprovalHandler for ApproveAll {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// A string document's shell command runs in the request directory, never
/// the process's: `cat` reads a file only the request directory holds.
#[cfg(unix)]
#[test]
fn a_string_documents_shell_command_runs_in_the_request_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("marker.txt"), "request-directory-marker").unwrap();
    assert!(!Path::new("marker.txt").exists(), "the process directory must not hold the marker");
    let md: Markdown = "::shell cat marker.txt\n".into();

    let options = ComposeOptions::new().with_shell_approval_handler(std::sync::Arc::new(ApproveAll));
    let (composed, _) = md.compose_with(&request_at(dir.path(), options)).unwrap();
    assert!(composed.content().contains("request-directory-marker"), "{}", composed.content());
}

/// A standalone `ComposeContext` capture reads no process environment, and a
/// request installs its snapshot's: `ctx.agent` comes from the snapshot alone.
#[test]
fn ctx_environment_comes_from_the_request_snapshot_only() {
    let dir = TempDir::new().unwrap();
    let source = "Agent: {{ ctx.agent }}\n";
    let capture = ComposeContext::capture_for_content(dir.path(), source);
    assert!(capture.env().is_empty(), "a capture reads no process environment: {:?}", capture.env().keys());

    let md: Markdown = source.into();
    let snapshot = RequestSnapshot::new(dir.path())
        .with_env(HashMap::from([("AGENT".to_string(), "snapshot-agent".to_string())]));
    let request = ComposeRequest::prepare(ComposeOptions::new_with_context(capture.clone()), &snapshot).unwrap();
    let (composed, _) = md.compose_with(&request).unwrap();
    assert!(composed.content().contains("Agent: snapshot-agent"), "{}", composed.content());

    let empty = ComposeRequest::prepare(ComposeOptions::new_with_context(capture), &RequestSnapshot::new(dir.path())).unwrap();
    let (composed, _) = md.compose_with(&empty).unwrap();
    assert!(composed.content().contains("Agent: unknown"), "{}", composed.content());
}
