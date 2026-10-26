//! The editor responses to the repository's largest lifecycle document,
//! pinned exactly: hover text and ranges, completion order and edits,
//! diagnostic codes, ranges, messages, and severities, definition targets,
//! and document links. The document and Claudine's schema bundle are the
//! frozen A3 benchmark inputs, driven in-process through the provider
//! registry as the benchmark drives them, so a change to how the frontmatter
//! walk owns or caches schema shapes cannot change what an editor receives.
//!
//! Regenerate a golden with `DMLS_UPDATE_GOLDEN=1` only for an intended
//! response change.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::RequestSnapshot;
use dmls::capabilities::ClientProfile;
use dmls::config::DmlsConfig;
use dmls::context::RepositoryContexts;
use dmls::graph::{NoContexts, WorkspaceGraph};
use dmls::overlay::OverlayState;
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use lsp_types::{CompletionItem, InitializeParams, Uri};
use serde_json::{Value, json};

const CLAUDINE_SCHEMAS: [(&str, &str); 9] = [
    ("action.yaml", include_str!("../../../lib/benches/fixtures/claudine-schemas/action.yaml")),
    ("claudine.yaml", include_str!("../../../lib/benches/fixtures/claudine-schemas/claudine.yaml")),
    ("globals.yaml", include_str!("../../../lib/benches/fixtures/claudine-schemas/globals.yaml")),
    ("inline-compose.yaml", include_str!("../../../lib/benches/fixtures/claudine-schemas/inline-compose.yaml")),
    ("lifecycle.yaml", include_str!("../../../lib/benches/fixtures/claudine-schemas/lifecycle.yaml")),
    ("linking.yaml", include_str!("../../../lib/benches/fixtures/claudine-schemas/linking.yaml")),
    ("review.yaml", include_str!("../../../lib/benches/fixtures/claudine-schemas/review.yaml")),
    ("sequence-types.yaml", include_str!("../../../lib/benches/fixtures/claudine-schemas/sequence-types.yaml")),
    ("sequence.yaml", include_str!("../../../lib/benches/fixtures/claudine-schemas/sequence.yaml")),
];

const CLAUDINE_LIFECYCLE_ARTIFACT: &str =
    include_str!("../../../lib/benches/fixtures/pipeline-work/globals.lifecycle.json");

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("create directory");
    std::fs::write(path, text).expect("write file");
}

/// The benchmark's repository: `claudine/schemas/` holds the frozen bundle
/// and the document sits in Claudine's `tasks/`, with an empty home.
struct Session {
    _dir: tempfile::TempDir,
    root: PathBuf,
    path: PathBuf,
    uri: Uri,
    contexts: RepositoryContexts,
    config: DmlsConfig,
    registry: ProviderRegistry,
    graph: WorkspaceGraph,
    profile: ClientProfile,
    overlays: OverlayState,
}

impl Session {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let repo = root.join("repo");
        std::fs::create_dir_all(repo.join(".git/objects")).expect("objects");
        std::fs::create_dir_all(repo.join(".git/refs/heads")).expect("refs");
        write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
        write(&repo.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
        write(&repo.join("Cargo.toml"), "[workspace]\nmembers = [\"claudine/lib\"]\n");
        write(&repo.join("claudine/lib/Cargo.toml"), "[package]\nname = \"claudine\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
        write(&repo.join("claudine/lib/src/lib.rs"), "");
        for (name, text) in CLAUDINE_SCHEMAS {
            write(&repo.join("claudine/schemas").join(name), text);
        }
        write(&repo.join("claudine/schemas/globals.lifecycle.json"), CLAUDINE_LIFECYCLE_ARTIFACT);
        std::fs::create_dir_all(root.join("home")).expect("home");
        let path = repo.join("claudine/lib/tasks/implement-plan.md");
        let uri: Uri = url::Url::from_file_path(&path).expect("file URL").to_string().parse().expect("URI");
        let snapshot = RequestSnapshot::new(&root).with_home(Some(root.join("home")));
        Self {
            _dir: dir,
            path,
            uri,
            contexts: RepositoryContexts::new(snapshot),
            config: DmlsConfig::default(),
            registry: ProviderRegistry::with_substrate(),
            graph: WorkspaceGraph::build(&BTreeMap::new(), 1, &NoContexts),
            profile: ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16),
            overlays: OverlayState::default(),
            root,
        }
    }

    /// Writes `text` as the document and runs `f` against one request's
    /// context for it.
    fn with_document<R>(&self, text: &str, f: impl FnOnce(&DocumentContext<'_>, &ProviderRegistry) -> R) -> R {
        write(&self.path, text);
        let repo = [self.root.join("repo")];
        let resolution = self.contexts.for_document(&self.path);
        let overlay = self.overlays.for_document(&self.uri, text, &self.path, &self.config, &repo, &resolution);
        assert!(overlay.is_some(), "the document has a frontmatter overlay");
        let source_map = SourceMap::new(self.uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let ctx = DocumentContext {
            uri: &self.uri,
            path: &self.path,
            text,
            source_map: &source_map,
            graph: &self.graph,
            doc_id: None,
            config: &self.config,
            profile: &self.profile,
            overlay: overlay.as_ref(),
            resolution: &resolution,
        };
        f(&ctx, &self.registry)
    }

    /// `value` with the session's temporary root replaced by `<root>` in
    /// every string, path separators normalized, so a golden is portable.
    fn portable(&self, value: Value) -> Value {
        let url = url::Url::from_directory_path(&self.root).expect("root URL").to_string();
        let url = url.trim_end_matches('/').to_string();
        let path = self.root.display().to_string();
        let forward = path.replace('\\', "/");
        fn walk(value: Value, replace: &dyn Fn(&str) -> String) -> Value {
            match value {
                Value::String(text) => Value::String(replace(&text)),
                Value::Array(items) => Value::Array(items.into_iter().map(|item| walk(item, replace)).collect()),
                Value::Object(map) => Value::Object(map.into_iter().map(|(key, item)| (key, walk(item, replace))).collect()),
                other => other,
            }
        }
        walk(value, &|text: &str| {
            if !(text.contains(&url) || text.contains(&path) || text.contains(&forward)) {
                return text.to_string();
            }
            text.replace(&url, "<root-uri>").replace(&path, "<root>").replace('\\', "/").replace(&forward, "<root>")
        })
    }
}

/// The byte offset `nudge` bytes into the first `needle`.
fn at(text: &str, needle: &str, nudge: usize) -> usize {
    text.find(needle).unwrap_or_else(|| panic!("`{needle}` is in the document")) + nudge
}

/// A completion item without its documentation, which is catalog prose
/// outside this contract.
fn completion_contract(item: &CompletionItem) -> Value {
    let mut value = serde_json::to_value(item).expect("completion serializes");
    if let Value::Object(map) = &mut value {
        map.remove("documentation");
    }
    value
}

/// Compares `actual` with the golden at `relative` (under
/// `tests/fixtures/response_contracts/`), or rewrites the golden when
/// `DMLS_UPDATE_GOLDEN` is set.
fn assert_golden(golden: &str, relative: &str, actual: &Value) {
    let actual = format!("{}\n", serde_json::to_string_pretty(actual).expect("JSON"));
    if std::env::var_os("DMLS_UPDATE_GOLDEN").is_some() {
        let path = biscuit_test_harness::manifest_dir!().join("tests/fixtures/response_contracts").join(relative);
        write(&path, &actual);
        return;
    }
    let golden = golden.replace("\r\n", "\n");
    assert!(
        actual == golden,
        "the responses differ from tests/fixtures/response_contracts/{relative}; regenerate it with \
         DMLS_UPDATE_GOLDEN=1 only for an intended change.\n--- actual ---\n{actual}"
    );
}

/// The frozen A3 document, with line endings fixed so offsets are portable.
fn a3_document(raw: &str) -> String {
    raw.replace("\r\n", "\n")
}

#[test]
fn a3_hovers_are_unchanged() {
    let text = a3_document(include_str!("../../../lib/benches/fixtures/pipeline-work/implement-plan.md"));
    let session = Session::new();
    let positions = [
        ("lifecycle verb", at(&text, "ensure_file", 2)),
        ("expression call", at(&text, "file_exists(", 2)),
        ("expression value of a lifecycle guard", at(&text, "!total_phases", 3)),
        ("lifecycle guard key", at(&text, "- when:", 3)),
        ("lifecycle then key", at(&text, "then:", 1)),
        ("loop block key", at(&text, "\nloop:", 2)),
        ("loop until expression", at(&text, "until: \"phase", 9)),
        ("break reason key", at(&text, "reason:", 1)),
        ("schema-typed key", at(&text, "\nplan:", 2)),
        ("schema-typed expression value", at(&text, "dirname(spec)", 2)),
        ("plain scalar value", at(&text, "yolo: true", 7)),
        ("empty string value", at(&text, "commit_message: \"\"", 16)),
        ("list item marker", at(&text, "- ensure_file", 0)),
        ("comment line", at(&text, "# The implementation", 3)),
        ("body", at(&text, "# Implement Phase", 3)),
    ];
    let actual = session.with_document(&text, |ctx, registry| {
        let hovers: Vec<Value> = positions
            .iter()
            .map(|(label, offset)| json!({ "at": label, "hover": registry.hover(ctx, *offset) }))
            .collect();
        Value::Array(hovers)
    });
    assert_golden(include_str!("../fixtures/response_contracts/a3_hovers.json"), "a3_hovers.json", &session.portable(actual));
}

#[test]
fn a3_completions_are_unchanged() {
    let text = a3_document(include_str!("../../../lib/benches/fixtures/pipeline-work/implement-plan.md"));
    let session = Session::new();
    let positions = [
        ("lifecycle verb key", at(&text, "warn:", 2)),
        ("lifecycle guard value", at(&text, "!total_phases", 1)),
        ("loop block key", at(&text, "until:", 2)),
        ("top-level key", at(&text, "yolo:", 2)),
        ("schema-typed value", at(&text, "commit_message: \"\"", 16)),
    ];
    let actual = session.with_document(&text, |ctx, registry| {
        let completions: Vec<Value> = positions
            .iter()
            .map(|(label, offset)| {
                let items: Vec<Value> = registry.completion(ctx, *offset).iter().map(completion_contract).collect();
                json!({ "at": label, "items": items })
            })
            .collect();
        Value::Array(completions)
    });
    assert_golden(
        include_str!("../fixtures/response_contracts/a3_completions.json"),
        "a3_completions.json",
        &session.portable(actual),
    );
}

#[test]
fn a3_diagnostics_are_unchanged() {
    let text = a3_document(include_str!("../../../lib/benches/fixtures/pipeline-work/implement-plan.md"));
    let session = Session::new();
    let actual = session.with_document(&text, |ctx, registry| {
        // A second publish on the same request context answers the same.
        let first = registry.diagnostics(ctx);
        assert_eq!(registry.diagnostics(ctx), first, "a repeated publish is identical");
        serde_json::to_value(first).expect("diagnostics serialize")
    });
    assert_golden(
        include_str!("../fixtures/response_contracts/a3_diagnostics.json"),
        "a3_diagnostics.json",
        &session.portable(actual),
    );
}

/// The A3 document with its `plan` and `spec` file values authored as paths
/// beside it instead of expressions, so navigation has targets.
#[test]
fn a3_navigation_is_unchanged() {
    let raw = a3_document(include_str!("../../../lib/benches/fixtures/pipeline-work/implement-plan.md"));
    let plan_line = "plan: \"{{ spec ? dirname(spec) + '/plan.md'  : null }}\"\n";
    let spec_start = raw.find("spec: |-\n").expect("the `spec` value");
    let spec_end = raw[spec_start..].find("# The implementation").expect("the `log` comment") + spec_start;
    let text = format!(
        "{}spec: ./notes/spec.md\n{}",
        &raw[..spec_start],
        &raw[spec_end..]
    )
    .replacen(plan_line, "plan: ./notes/plan.md\n", 1);
    assert_ne!(text, raw, "both values are rewritten");
    let session = Session::new();
    let tasks = session.path.parent().expect("tasks").to_path_buf();
    write(&tasks.join("notes/plan.md"), "# plan\n");
    write(&tasks.join("notes/spec.md"), "# spec\n");
    let positions = [
        ("file value", at(&text, "./notes/plan.md", 4)),
        ("second file value", at(&text, "./notes/spec.md", 4)),
        ("file-typed key", at(&text, "\nplan:", 2)),
        ("file-typed expression value", at(&text, "dirname(spec || plan)", 2)),
        ("lifecycle verb value", at(&text, "ensure_file: \"{{log}}\"", 17)),
        ("schema property type", at(&text, "file(eager; required", 2)),
        ("body", at(&text, "# Implement Phase", 3)),
    ];
    let actual = session.with_document(&text, |ctx, registry| {
        let definitions: Vec<Value> = positions
            .iter()
            .map(|(label, offset)| json!({ "at": label, "definition": registry.definition(ctx, *offset) }))
            .collect();
        json!({ "definitions": definitions, "document_links": registry.document_links(ctx) })
    });
    assert_golden(
        include_str!("../fixtures/response_contracts/a3_navigation.json"),
        "a3_navigation.json",
        &session.portable(actual),
    );
}
