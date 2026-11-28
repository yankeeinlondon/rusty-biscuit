//! DMLS judges the four configuration files by their location: `.darkmatter.yaml`
//! and `.claudine/config.yaml`, each under the captured home (the user
//! contract) and the repository root (the repository contract). The
//! contract is chosen before any `kind` or `$schema` is read, needs no host
//! registration, and serves an empty, newly created buffer. A file of the
//! same name anywhere else is an ordinary YAML document.
//!
//! Driven in-process through one `OverlayState` per probe, in a hermetic
//! repository whose captured home is a folder inside it; the last test drives
//! the same behavior through a stdio session.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::config::schema::ConfigScope;
use darkmatter::markdown::schemas::Route;
use dmls::capabilities::ClientProfile;
use dmls::config::DmlsConfig;
use dmls::config_contracts::{CLAUDINE_CONFIG_SCHEMA_SOURCE, ConfigHost, ConfigurationFile};
use dmls::context::RepositoryContexts;
use dmls::graph::{NoContexts, WorkspaceGraph};
use dmls::overlay::{OverlayState, YamlDocumentOutcome};
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use lsp_types::{Diagnostic, InitializeParams, NumberOrString, Uri};
use serde_json::{Value, json};

use crate::common::{LspFixture, LspWorkspace};

const USER_DARKMATTER: &str = "home/.darkmatter.yaml";
const REPOSITORY_DARKMATTER: &str = ".darkmatter.yaml";
const USER_CLAUDINE: &str = "home/.claudine/config.yaml";
const REPOSITORY_CLAUDINE: &str = ".claudine/config.yaml";

/// A repository with its captured home at `home/`, and one overlay cache.
struct Probe {
    workspace: LspWorkspace,
    contexts: RepositoryContexts,
    config: DmlsConfig,
    state: OverlayState,
}

impl Probe {
    fn new() -> Self {
        let workspace = LspWorkspace::new();
        init_repository(workspace.path());
        let contexts = RepositoryContexts::new(LspFixture::fixture_snapshot(&workspace));
        Self { workspace, contexts, config: DmlsConfig::default(), state: OverlayState::default() }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.workspace.path().join(name)
    }

    /// Runs `inspect` over `text` opened as the buffer of `name`; the file
    /// itself is never written, as for a newly created file, but its folder
    /// exists (a document's context is anchored at it).
    fn with<T>(&self, name: &str, text: &str, inspect: impl FnOnce(&DocumentContext) -> T) -> T {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let uri: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        let root = self.workspace.path().to_path_buf();
        let resolution = self.contexts.for_document(&path);
        let overlay =
            self.state.for_document(&uri, text, &path, &self.config, std::slice::from_ref(&root), &resolution);
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let graph = WorkspaceGraph::build(&BTreeMap::new(), 1, &NoContexts);
        let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
        let ctx = DocumentContext {
            uri: &uri,
            path: &path,
            text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config: &self.config,
            profile: &profile,
            overlay: overlay.as_ref(),
            resolution: &resolution,
            host: None,
        };
        inspect(&ctx)
    }

    /// The route and definition the document was judged by; `None` when it
    /// was not judged.
    fn route(&self, name: &str, text: &str) -> Option<(Route, String)> {
        self.with(name, text, |ctx| {
            let yaml = ctx.overlay.and_then(|overlay| overlay.yaml_document.as_deref()).expect("a YAML overlay");
            match &yaml.outcome {
                YamlDocumentOutcome::Validated(validation) => {
                    Some((validation.route.clone(), validation.definition.clone()))
                }
                _ => None,
            }
        })
    }

    fn schema_diagnostics(&self, name: &str, text: &str) -> Vec<Diagnostic> {
        self.with(name, text, |ctx| ProviderRegistry::with_substrate().diagnostics(ctx))
            .into_iter()
            .filter(|diagnostic| {
                matches!(diagnostic.source.as_deref(), Some("darkmatter.schema" | "darkmatter.frontmatter"))
            })
            .collect()
    }

    fn completion_labels(&self, name: &str, text: &str, offset: usize) -> Vec<String> {
        self.with(name, text, |ctx| {
            ProviderRegistry::with_substrate().completion(ctx, offset).into_iter().map(|item| item.label).collect()
        })
    }

    fn hover(&self, name: &str, text: &str, offset: usize) -> String {
        self.with(name, text, |ctx| {
            ProviderRegistry::with_substrate().hover(ctx, offset).map(|hover| match hover.contents {
                lsp_types::HoverContents::Markup(markup) => markup.value,
                other => format!("{other:?}"),
            })
        })
        .unwrap_or_default()
    }
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A minimal repository Git discovery accepts, without running `git`.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    std::fs::create_dir_all(root.join("home")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

fn code_of(diagnostic: &Diagnostic) -> &str {
    match &diagnostic.code {
        Some(NumberOrString::String(code)) => code,
        _ => "",
    }
}

fn codes(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics.iter().map(code_of).collect()
}

/// The zero-based `(line, UTF-16 character)` of the first `needle` in `text`.
fn position_of(text: &str, needle: &str) -> (u32, u32) {
    let offset = text.find(needle).expect("needle present");
    let line = text[..offset].matches('\n').count() as u32;
    let line_start = text[..offset].rfind('\n').map_or(0, |newline| newline + 1);
    (line, text[line_start..offset].encode_utf16().count() as u32)
}

fn start(diagnostic: &Diagnostic) -> (u32, u32) {
    (diagnostic.range.start.line, diagnostic.range.start.character)
}

#[test]
fn each_configuration_path_selects_its_host_and_scope_contract() {
    let probe = Probe::new();
    for (name, definition) in [
        (USER_DARKMATTER, "`user-config` in darkmatter/schemas/darkmatter-config.yaml"),
        (REPOSITORY_DARKMATTER, "`repository-config` in darkmatter/schemas/darkmatter-config.yaml"),
        (USER_CLAUDINE, "`user-config` in claudine/registration/claudine-config.yaml"),
        (REPOSITORY_CLAUDINE, "`repository-config` in claudine/registration/claudine-config.yaml"),
    ] {
        let text = "permissions:\n    network: deny\n";
        assert_eq!(probe.route(name, text), Some((Route::Configuration, definition.to_string())), "{name}");
        assert_eq!(probe.schema_diagnostics(name, text), Vec::new(), "{name}");
    }
}

#[test]
fn the_darkmatter_scopes_differ_in_their_file_read_default_on_hover() {
    let probe = Probe::new();
    let text = "permissions:\n    network: deny\nregister: []\n";
    let user = probe.hover(USER_DARKMATTER, text, 2);
    let repository = probe.hover(REPOSITORY_DARKMATTER, text, 2);
    assert!(user.contains("./*.txt") && !user.contains("&**/*.txt"), "{user}");
    assert!(repository.contains("&**/*.txt"), "{repository}");
    let register = probe.hover(USER_DARKMATTER, text, text.find("register").unwrap() + 2);
    assert!(register.contains("Host registration manifests to load"), "the setting's doc: {register}");
}

#[test]
fn a_claudine_setting_only_the_user_scope_declares_is_unknown_in_the_repository_file() {
    let probe = Probe::new();
    let text = "logging: false\n";
    assert_eq!(probe.schema_diagnostics(USER_CLAUDINE, text), Vec::new());
    let diagnostics = probe.schema_diagnostics(REPOSITORY_CLAUDINE, text);
    assert_eq!(codes(&diagnostics), ["dm.schema.unknown_key"], "{diagnostics:#?}");
    assert_eq!(start(&diagnostics[0]), position_of(text, "logging"), "{diagnostics:#?}");

    let body = probe.hover(USER_CLAUDINE, text, 2);
    assert!(body.contains("Whether every dispatched hook event"), "the setting's doc: {body}");
    assert!(body.contains("true"), "the schema default: {body}");
}

#[test]
fn an_empty_new_buffer_is_judged_and_completes_its_top_level_keys() {
    let probe = Probe::new();
    for (name, expected) in [
        (USER_DARKMATTER, &["permissions", "register"][..]),
        (REPOSITORY_DARKMATTER, &["permissions", "register"][..]),
        (USER_CLAUDINE, &["logging", "permissions", "tts"][..]),
        (REPOSITORY_CLAUDINE, &["active_messenger", "permissions"][..]),
    ] {
        assert!(!probe.path(name).exists(), "the file is never created: {name}");
        assert!(matches!(probe.route(name, ""), Some((Route::Configuration, _))), "{name}");
        assert_eq!(probe.schema_diagnostics(name, ""), Vec::new(), "{name}");
        let labels = probe.completion_labels(name, "", 0);
        for key in expected {
            assert!(labels.iter().any(|label| label == key), "{name}: `{key}` in {labels:?}");
        }
    }
    let labels = probe.completion_labels(REPOSITORY_CLAUDINE, "", 0);
    assert!(!labels.iter().any(|label| label == "logging"), "user-only setting offered: {labels:?}");
}

/// A buffer for `~/.claudine/config.yaml` opened before `~/.claudine/`
/// exists has the context of its nearest existing folder, so it is routed
/// and judged like the saved file; nothing is created.
#[test]
fn a_configuration_buffer_whose_folder_does_not_exist_yet_is_routed() {
    let probe = Probe::new();
    let path = probe.path(USER_CLAUDINE);
    let folder = path.parent().unwrap().to_path_buf();
    assert!(!folder.exists());
    let resolution = probe.contexts.for_document(&path);
    assert!(resolution.context().is_ok(), "{:?}", resolution.context().err());
    let uri: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
    let root = probe.workspace.path().to_path_buf();
    let overlay = probe
        .state
        .for_document(&uri, "logging: maybe\n", &path, &probe.config, std::slice::from_ref(&root), &resolution)
        .expect("a YAML overlay");
    match &overlay.yaml_document.as_deref().expect("a YAML judgment").outcome {
        YamlDocumentOutcome::Validated(validation) => {
            assert_eq!(validation.route, Route::Configuration);
            assert_eq!(validation.definition, "`user-config` in claudine/registration/claudine-config.yaml");
        }
        _ => panic!("the buffer was not judged by a configuration contract"),
    }
    assert!(!folder.exists(), "the folder is never created");
}

#[test]
fn an_authored_schema_or_kind_does_not_override_the_configuration_contract() {
    let probe = Probe::new();
    write(&probe.path("other.yaml"), "$schema:\n    port: number\n");
    let text = "$schema: ./other.yaml\nkind: schema\npermissions:\n    network: deny\n";
    assert!(matches!(probe.route(REPOSITORY_DARKMATTER, text), Some((Route::Configuration, _))));
    // Both are data the closed configuration contract does not declare.
    let diagnostics = probe.schema_diagnostics(REPOSITORY_DARKMATTER, text);
    let [diagnostic] = diagnostics.as_slice() else { panic!("one diagnostic: {diagnostics:#?}") };
    assert_eq!(code_of(diagnostic), "dm.schema.unknown_key", "{diagnostic:#?}");
    assert_eq!(start(diagnostic), position_of(text, "$schema"), "{diagnostic:#?}");
    assert!(diagnostic.message.contains("'$schema'") && diagnostic.message.contains("'kind'"), "{diagnostic:#?}");
    let labels = probe.completion_labels(REPOSITORY_DARKMATTER, text, text.len());
    assert!(labels.iter().any(|label| label == "register"), "{labels:?}");
    assert!(!labels.iter().any(|label| label == "port"), "the named schema is not the contract: {labels:?}");
}

#[test]
fn a_same_named_file_away_from_the_configuration_paths_routes_normally() {
    let probe = Probe::new();
    for name in ["docs/.darkmatter.yaml", "docs/.claudine/config.yaml", "home/sub/.darkmatter.yaml"] {
        assert_eq!(probe.route(name, "permissions: 5\n"), None, "{name} has no schema");
        let text = "$schema:\n    port: number\nport: http\n";
        assert_eq!(probe.route(name, text).map(|(route, _)| route), Some(Route::RootSchema), "{name}");
        assert_eq!(codes(&probe.schema_diagnostics(name, text)), ["dm.schema.type_mismatch"], "{name}");
    }
}

#[test]
fn a_configuration_error_is_located_at_the_offending_value() {
    let probe = Probe::new();
    let text = "permissions:\n    network: sometimes\n";
    let diagnostics = probe.schema_diagnostics(USER_DARKMATTER, text);
    let [diagnostic] = diagnostics.as_slice() else { panic!("one diagnostic: {diagnostics:#?}") };
    assert_eq!(code_of(diagnostic), "dm.schema.constraint", "{diagnostic:#?}");
    assert_eq!(diagnostic.severity, Some(lsp_types::DiagnosticSeverity::ERROR));
    assert_eq!(start(diagnostic), position_of(text, "sometimes"), "{diagnostic:#?}");
}

/// The compiled-in Claudine contract is Claudine's own file, and every
/// configuration contract prepares from it with its `darkmatter:` import.
#[test]
fn every_compiled_in_configuration_schema_prepares() {
    assert_eq!(CLAUDINE_CONFIG_SCHEMA_SOURCE, include_str!("../../../../claudine/registration/claudine-config.yaml"));
    for host in [ConfigHost::Darkmatter, ConfigHost::Claudine] {
        for scope in [ConfigScope::User, ConfigScope::Repository] {
            let resolved = ConfigurationFile { host, scope }.resolved().unwrap_or_else(|error| panic!("{host:?} {scope:?}: {error}"));
            let properties = &resolved.json_schema["properties"];
            assert!(properties.get("permissions").is_some(), "{host:?} {scope:?}: {properties:#}");
            assert!(resolved.simplified.is_some(), "descriptions are kept for the editor");
        }
    }
    let user = ConfigurationFile { host: ConfigHost::Claudine, scope: ConfigScope::User }.resolved().unwrap();
    assert!(user.json_schema["properties"].get("logging").is_some());
}

// ── Session ────────────────────────────────────────────────────────────────

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn latest_schema_codes(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<String> {
    fixture.flush_server();
    let latest = fixture.wait_for_diagnostics_params(uri);
    while fixture.take_buffered_diagnostics(uri).is_some() {}
    latest["diagnostics"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .filter(|code| code.starts_with("dm.schema."))
        .map(str::to_string)
        .collect()
}

#[test]
fn a_new_claudine_configuration_is_served_through_a_session_without_any_registration() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let document = root.join(USER_CLAUDINE);
    // The new file's folder exists; the file itself is only a buffer.
    std::fs::create_dir_all(document.parent().unwrap()).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(json!({
        "processId": null,
        "capabilities": { "general": { "positionEncodings": ["utf-16"] } },
        "workspaceFolders": [{ "uri": url::Url::from_directory_path(root).unwrap().as_str(), "name": "w" }]
    }));
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(&document), "languageId": "yaml", "version": 1, "text": "" } }),
    );
    assert_eq!(latest_schema_codes(&mut fixture, &uri(&document)), Vec::<String>::new());
    let response = fixture.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri(&document) }, "position": { "line": 0, "character": 0 } }),
    );
    let items = response.result.unwrap_or(Value::Null);
    let items = items.get("items").cloned().unwrap_or(items);
    let labels: Vec<&str> = items.as_array().into_iter().flatten().filter_map(|item| item["label"].as_str()).collect();
    assert!(labels.contains(&"logging") && labels.contains(&"permissions"), "{labels:?}");

    let text = "logging: maybe\n";
    fixture.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": uri(&document), "version": 2 }, "contentChanges": [{ "text": text }] }),
    );
    assert_eq!(latest_schema_codes(&mut fixture, &uri(&document)), ["dm.schema.type_mismatch"]);
    let response = fixture.request(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri(&document) }, "position": { "line": 0, "character": 2 } }),
    );
    let body = response.result.map(|result| result["contents"]["value"].as_str().unwrap_or_default().to_string());
    assert!(body.is_some_and(|body| body.contains("Whether every dispatched hook event")));
    fixture.shutdown();
}
