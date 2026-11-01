//! DMLS serves a whole YAML document (`.yaml`, `.yml`) from the schema it
//! selects: a `kind` claimed by a `kind: yaml-schema` definition (shipped or
//! in a schema root) selects that definition, otherwise a root `$schema` is
//! the contract, and a document with neither is not validated. Diagnostics
//! carry the frontmatter `dm.schema.*` codes ranged on the offending key or
//! value; completion and hover describe the selected schema alone.
//!
//! Driven in-process through one `OverlayState` per probe (so successive
//! texts of one document reuse and replace its cached state) and the
//! provider registry. The protocol wiring is `yaml_document_session.rs`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use dmls::capabilities::ClientProfile;
use dmls::config::DmlsConfig;
use dmls::context::RepositoryContexts;
use dmls::graph::{NoContexts, WorkspaceGraph};
use dmls::overlay::{OverlayState, YamlDocumentOutcome};
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use lsp_types::{Diagnostic, InitializeParams, NumberOrString, Position, Uri};

use crate::common::{LspFixture, LspWorkspace};

/// A user kind's definition in the workspace's tree schema root.
const ROSTER_DEFINITION: &str = "\
kind: yaml-schema
types:
    platform:
        name: string(required) -> the platform's name
$schema:
    kind: literal(roster; required) -> the roster kind
    roster_version: literal(1; required) -> the roster format version
    owner: string -> who maintains the roster
    port: number(integer)
    platforms: platform[](required) -> every supported platform
    $constraints: { closed: true }
";

/// A valid roster.
const ROSTER: &str = "kind: roster\nroster_version: 1\nplatforms:\n    - name: slack\n";

/// One workspace (a repository whose `schemas/` folder is its tree schema
/// root) and one overlay cache, so the texts a test opens as one document
/// replace each other's state.
struct Probe {
    workspace: LspWorkspace,
    contexts: RepositoryContexts,
    config: DmlsConfig,
    state: OverlayState,
}

impl Probe {
    fn new() -> Self {
        Self::with_config(DmlsConfig::default())
    }

    fn strict() -> Self {
        let mut config = DmlsConfig::default();
        config.schema.strict = true;
        Self::with_config(config)
    }

    fn with_config(config: DmlsConfig) -> Self {
        let workspace = LspWorkspace::new();
        init_repository(workspace.path());
        write(&workspace.path().join("schemas/roster.yaml"), ROSTER_DEFINITION);
        let contexts = RepositoryContexts::new(LspFixture::fixture_snapshot(&workspace));
        Self { workspace, contexts, config, state: OverlayState::default() }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.workspace.path().join(name)
    }

    /// Runs `inspect` over the context of `text` opened as `name` (the file
    /// is written too, as a saved buffer would be).
    fn with<T>(&self, name: &str, text: &str, inspect: impl FnOnce(&DocumentContext) -> T) -> T {
        let path = self.path(name);
        write(&path, text);
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
        };
        inspect(&ctx)
    }

    fn diagnostics(&self, name: &str, text: &str) -> Vec<Diagnostic> {
        self.with(name, text, |ctx| ProviderRegistry::with_substrate().diagnostics(ctx))
    }

    /// The `darkmatter.schema` and `darkmatter.frontmatter` diagnostics.
    fn schema_diagnostics(&self, name: &str, text: &str) -> Vec<Diagnostic> {
        self.diagnostics(name, text)
            .into_iter()
            .filter(|diagnostic| {
                matches!(diagnostic.source.as_deref(), Some("darkmatter.schema" | "darkmatter.frontmatter"))
            })
            .collect()
    }

    fn outcome(&self, name: &str, text: &str) -> &'static str {
        self.with(name, text, |ctx| {
            let yaml = ctx.overlay.and_then(|overlay| overlay.yaml_document.as_deref()).expect("a YAML overlay");
            match &yaml.outcome {
                YamlDocumentOutcome::LoadFailed(_) => "load-failed",
                YamlDocumentOutcome::Unvalidated => "unvalidated",
                YamlDocumentOutcome::NoSchema => "no-schema",
                YamlDocumentOutcome::Failed(_) => "failed",
                YamlDocumentOutcome::Validated(_) => "validated",
            }
        })
    }

    fn completion_labels(&self, name: &str, text: &str, offset: usize) -> Vec<String> {
        self.with(name, text, |ctx| {
            ProviderRegistry::with_substrate().completion(ctx, offset).into_iter().map(|item| item.label).collect()
        })
    }

    fn hover(&self, name: &str, text: &str, offset: usize) -> Option<String> {
        self.with(name, text, |ctx| {
            ProviderRegistry::with_substrate().hover(ctx, offset).map(|hover| match hover.contents {
                lsp_types::HoverContents::Markup(markup) => markup.value,
                other => format!("{other:?}"),
            })
        })
    }
}

fn write(path: &std::path::Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A minimal repository Git discovery accepts, without running `git`.
fn init_repository(root: &std::path::Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
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

fn start(diagnostic: &Diagnostic) -> (u32, u32) {
    (diagnostic.range.start.line, diagnostic.range.start.character)
}

/// The zero-based `(line, UTF-16 character)` of the first `needle` in `text`.
fn position_of(text: &str, needle: &str) -> (u32, u32) {
    let offset = text.find(needle).expect("needle present");
    let line = text[..offset].matches('\n').count() as u32;
    let line_start = text[..offset].rfind('\n').map_or(0, |newline| newline + 1);
    (line, text[line_start..offset].encode_utf16().count() as u32)
}

fn only(diagnostics: &[Diagnostic]) -> &Diagnostic {
    let [diagnostic] = diagnostics else {
        panic!("exactly one diagnostic expected: {diagnostics:#?}");
    };
    diagnostic
}

// ── Kind route ──────────────────────────────────────────────────────────────

#[test]
fn a_claimed_repository_kind_is_validated_by_its_definition_alone() {
    let probe = Probe::strict();
    assert_eq!(probe.outcome("roster.data.yaml", ROSTER), "validated");
    assert_eq!(probe.schema_diagnostics("roster.data.yaml", ROSTER), Vec::new());

    // A wrong value ranges the value.
    let text = ROSTER.replace("roster_version: 1", "roster_version: 2");
    let diagnostic = only(&probe.schema_diagnostics("roster.data.yaml", &text)).clone();
    assert_eq!(code_of(&diagnostic), "dm.schema.constraint", "{diagnostic:#?}");
    assert_eq!(start(&diagnostic), position_of(&text, "2\n"), "{diagnostic:#?}");

    // An undeclared key ranges the key; a Markdown baseline key such as
    // `title` has no meaning here.
    let text = format!("{ROSTER}title: hello\n");
    let diagnostic = only(&probe.schema_diagnostics("roster.data.yaml", &text)).clone();
    assert_eq!(code_of(&diagnostic), "dm.schema.unknown_key", "{diagnostic:#?}");
    assert_eq!(start(&diagnostic), position_of(&text, "title"), "{diagnostic:#?}");
}

#[test]
fn a_shipped_kind_is_claimed_without_any_schema_root() {
    let probe = Probe::new();
    let text = "kind: schema\ndescription: [a, b]\n$schema:\n    title: string\n";
    // Published once: the envelope's admission against the same definition
    // is not repeated beside it.
    let diagnostic = only(&probe.schema_diagnostics("shape.yaml", text)).clone();
    assert_eq!(code_of(&diagnostic), "dm.schema.type_mismatch", "{diagnostic:#?}");
    assert_eq!(start(&diagnostic), position_of(text, "[a, b]"), "{diagnostic:#?}");
}

#[test]
fn a_missing_required_key_is_an_error_without_strict_mode_and_offers_the_add_keys_action() {
    let probe = Probe::new();
    let text = "kind: roster\nroster_version: 1\n";
    let diagnostics = probe.schema_diagnostics("roster.data.yaml", text);
    let diagnostic = only(&diagnostics);
    assert_eq!(code_of(diagnostic), "dm.schema.missing_required", "{diagnostic:#?}");
    assert_eq!(diagnostic.severity, Some(lsp_types::DiagnosticSeverity::ERROR));
    assert_eq!(start(diagnostic), (0, 0), "a root key ranges the root mapping: {diagnostic:#?}");

    let titles: Vec<String> = probe.with("roster.data.yaml", text, |ctx| {
        dmls::providers::code_actions::code_actions(ctx, &diagnostics)
            .into_iter()
            .map(|action| match action {
                lsp_types::CodeActionOrCommand::CodeAction(action) => action.title,
                lsp_types::CodeActionOrCommand::Command(command) => command.title,
            })
            .collect()
    });
    assert!(titles.iter().any(|title| title == "Add required key `platforms`"), "{titles:?}");
}

#[test]
fn a_kind_check_is_published_once_on_the_node_it_judges() {
    let probe = Probe::new();
    // A definition problem of a trigger is published once too.
    let text = "kind: schema-trigger\nmatch: 5\n$schema:\n    title: string\n";
    let diagnostic = only(&probe.schema_diagnostics("schemas/bad.trigger.yaml", text)).clone();
    assert_eq!(start(&diagnostic), position_of(text, "5"), "{diagnostic:#?}");
    let text = "kind: schema-trigger\nmatch:\n    $path: \"@prompts/**\"\n$schema:\n    title: string\n";
    let diagnostics = probe.schema_diagnostics("schemas/bad.trigger.yaml", text);
    let diagnostic = only(&diagnostics);
    assert_eq!(code_of(diagnostic), "dm.schema.kind_check", "{diagnostic:#?}");
    assert_eq!(diagnostic.range.start.line, 2, "{diagnostic:#?}");
}

#[test]
fn a_kind_claimed_twice_is_one_error_on_the_kind_naming_both_definitions() {
    let probe = Probe::new();
    write(&probe.path("schemas/roster-copy.yaml"), ROSTER_DEFINITION);
    let diagnostics = probe.schema_diagnostics("roster.data.yaml", ROSTER);
    let diagnostic = only(&diagnostics);
    assert_eq!(code_of(diagnostic), "dm.schema.prepare", "{diagnostic:#?}");
    assert_eq!(start(diagnostic), position_of(ROSTER, "roster\n"), "{diagnostic:#?}");
    assert!(diagnostic.message.contains("more than one definition"), "{diagnostic:#?}");
    let related = diagnostic.related_information.as_deref().unwrap_or_default();
    let files: Vec<&str> = related.iter().map(|info| info.location.uri.as_str()).collect();
    assert_eq!(files.len(), 2, "{related:#?}");
    assert!(files.iter().any(|uri| uri.ends_with("schemas/roster.yaml")), "{files:?}");
    assert!(files.iter().any(|uri| uri.ends_with("schemas/roster-copy.yaml")), "{files:?}");
}

#[test]
fn completion_and_hover_describe_the_selected_definition_alone() {
    let probe = Probe::new();
    let text = format!("{ROSTER}\n");
    let labels = probe.completion_labels("roster.data.yaml", &text, text.len());
    assert!(labels.iter().any(|label| label == "owner"), "{labels:?}");
    assert!(
        labels.iter().all(|label| label != "title" && label != "content_policy"),
        "no Markdown baseline key is offered: {labels:?}"
    );
    let hover = probe.hover("roster.data.yaml", ROSTER, ROSTER.find("roster_version").unwrap() + 2);
    assert!(hover.as_deref().is_some_and(|body| body.contains("the roster format version")), "{hover:?}");
}

#[test]
fn a_claimed_schema_document_keeps_schema_authoring_inside_its_exports() {
    let probe = Probe::new();
    // Inside the exported `$schema` value, the type grammar is offered.
    let text = "kind: schema\n$schema:\n    title: str\n";
    let offset = text.find("str").unwrap() + 3;
    let labels = probe.completion_labels("shape.yaml", text, offset);
    assert!(labels.iter().any(|label| label == "string"), "{labels:?}");
    // At the root, the `schema` kind's own keys are offered.
    let text = "kind: schema\n$schema:\n    title: string\n\n";
    let labels = probe.completion_labels("shape.yaml", text, text.len());
    assert!(labels.iter().any(|label| label == "description"), "{labels:?}");
}

#[test]
fn a_user_kinds_schema_key_is_data_not_a_reference() {
    let probe = Probe::new();
    write(
        &probe.path("schemas/note.yaml"),
        "kind: yaml-schema\n$schema:\n    kind: literal(note; required)\n    $schema: string\n",
    );
    let text = "kind: note\n$schema: not-a-file\n";
    assert_eq!(probe.outcome("note.yaml", text), "validated");
    assert_eq!(probe.schema_diagnostics("note.yaml", text), Vec::new());
}

// ── Root `$schema` route ────────────────────────────────────────────────────

#[test]
fn a_root_schema_validates_softly_and_is_not_an_unknown_key() {
    let probe = Probe::strict();
    write(&probe.path("contract.yaml"), "$schema:\n    port: number(integer)\n    $constraints: { closed: true }\n");
    let text = "$schema: ./contract.yaml\nport: \"8080\"\n";
    assert_eq!(probe.outcome("service.yaml", text), "validated");
    assert_eq!(probe.schema_diagnostics("service.yaml", text), Vec::new());
    assert_eq!(std::fs::read_to_string(probe.path("service.yaml")).unwrap(), text, "nothing is written back");

    let text = "$schema: ./contract.yaml\nport: \"2.5\"\nextra: 1\n";
    let diagnostics = probe.schema_diagnostics("service.yaml", text);
    let mut found = codes(&diagnostics);
    found.sort_unstable();
    assert_eq!(found, vec!["dm.schema.type_mismatch", "dm.schema.unknown_key"], "{diagnostics:#?}");
    let unknown = diagnostics.iter().find(|diagnostic| code_of(diagnostic) == "dm.schema.unknown_key").unwrap();
    assert_eq!(start(unknown), position_of(text, "extra"), "{unknown:#?}");
}

#[test]
fn an_interpolation_string_is_ordinary_data_in_a_yaml_document() {
    let probe = Probe::new();
    write(&probe.path("contract.yaml"), "$schema:\n    port: number\n    label: string\n");
    let text = "$schema: ./contract.yaml\nport: \"{{ 1 }}\"\nlabel: \"{{ name }}\"\n";
    let diagnostic = only(&probe.schema_diagnostics("service.yaml", text)).clone();
    assert_eq!(code_of(&diagnostic), "dm.schema.type_mismatch", "{diagnostic:#?}");
    assert_eq!(start(&diagnostic), position_of(text, "\"{{ 1 }}\""), "{diagnostic:#?}");
    // No Markdown body provider reads a YAML document.
    let offset = text.find("{{ name").unwrap() + 3;
    let labels = probe.completion_labels("service.yaml", text, offset);
    assert!(labels.is_empty(), "{labels:?}");
}

#[test]
fn an_unresolvable_root_schema_is_a_prepare_error_on_its_value() {
    let probe = Probe::new();
    let text = "$schema: ./missing.yaml\nport: 1\n";
    assert_eq!(probe.outcome("service.yaml", text), "failed");
    let diagnostics = probe.schema_diagnostics("service.yaml", text);
    let diagnostic = only(&diagnostics);
    assert!(code_of(diagnostic).starts_with("dm.schema."), "{diagnostic:#?}");
    assert_eq!(start(diagnostic), position_of(text, "./missing.yaml"), "{diagnostic:#?}");
}

#[test]
fn a_root_schema_naming_a_bad_definition_relates_the_failure_into_that_file() {
    let probe = Probe::new();
    write(&probe.path("contract.yaml"), "$schema:\n    port: number(nope)\n");
    let text = "$schema: ./contract.yaml\nport: 1\n";
    let diagnostics = probe.schema_diagnostics("service.yaml", text);
    let diagnostic = only(&diagnostics);
    assert_eq!(start(diagnostic), position_of(text, "./contract.yaml"), "the reference is the failure site: {diagnostic:#?}");
    let related = diagnostic.related_information.as_deref().unwrap_or_default();
    assert!(
        related.iter().any(|info| info.location.uri.as_str().ends_with("contract.yaml")),
        "the definition's failure is related into its file: {diagnostic:#?}"
    );
}

// ── Unvalidated and unloadable documents ────────────────────────────────────

#[test]
fn a_document_with_neither_kind_nor_schema_is_not_validated() {
    let probe = Probe::strict();
    for text in ["name: plain\ntitle: 5\n", "kind: unclaimed\nother: 1\n", "- a\n- b\n", "just a scalar\n", ""] {
        assert_eq!(probe.outcome("plain.yaml", text), "no-schema", "{text:?}");
        assert_eq!(probe.schema_diagnostics("plain.yaml", text), Vec::new(), "{text:?}");
        let labels = probe.completion_labels("plain.yaml", text, text.len());
        assert!(labels.iter().all(|label| label != "title"), "{text:?}: {labels:?}");
    }
}

#[test]
fn a_document_that_does_not_load_gets_one_parse_error_and_no_schema_problems() {
    let probe = Probe::new();
    for text in [
        "kind: roster\nroster_version: [1\n",
        "kind: roster\nkind: roster\nplatforms: []\n",
        "kind: roster\n---\nkind: roster\n",
    ] {
        assert_eq!(probe.outcome("roster.data.yaml", text), "load-failed", "{text:?}");
        let diagnostics = probe.schema_diagnostics("roster.data.yaml", text);
        let diagnostic = only(&diagnostics);
        assert_eq!(code_of(diagnostic), "dm.frontmatter.yaml_parse", "{text:?}: {diagnostic:#?}");
    }
    // The parser's position is the diagnostic's.
    let text = "kind: roster\nroster_version: [1\n";
    let diagnostic = only(&probe.schema_diagnostics("roster.data.yaml", text)).clone();
    assert_eq!(diagnostic.range.start.line, 2, "{diagnostic:#?}");
}

#[test]
fn document_markers_are_yaml_not_frontmatter_fences() {
    let probe = Probe::new();
    let text = format!("---\n{ROSTER}...\n");
    assert_eq!(probe.outcome("roster.data.yaml", &text), "validated");
    assert_eq!(probe.schema_diagnostics("roster.data.yaml", &text), Vec::new());
    // Upper- and mixed-case extensions route as YAML too.
    assert_eq!(probe.outcome("ROSTER.YAML", ROSTER), "validated");
    assert_eq!(probe.outcome("roster.Yml", ROSTER), "validated");
}

// ── Ranges ──────────────────────────────────────────────────────────────────

#[test]
fn ranges_count_utf16_after_unicode_and_survive_crlf_line_endings() {
    let probe = Probe::new();
    let text = "kind: roster\nroster_version: 1\nowner: \"😀 ü\"\nport: bad\nplatforms:\n    - name: slack\n";
    let diagnostic = only(&probe.schema_diagnostics("roster.data.yaml", text)).clone();
    assert_eq!(start(&diagnostic), position_of(text, "bad"), "{diagnostic:#?}");

    let text = "kind: roster\r\nroster_version: 1\r\nowner: 😀\r\nport: bad\r\nplatforms:\r\n    - name: slack\r\n";
    let diagnostic = only(&probe.schema_diagnostics("roster.data.yaml", text)).clone();
    assert_eq!(start(&diagnostic), (3, 6), "{diagnostic:#?}");
    assert_eq!(diagnostic.range.end, Position::new(3, 9), "{diagnostic:#?}");

    // A byte-order mark is not a character of the first line's key.
    let text = format!("\u{feff}{}", ROSTER.replace("roster_version: 1", "roster_version: 2"));
    let diagnostic = only(&probe.schema_diagnostics("roster.data.yaml", &text)).clone();
    assert_eq!(start(&diagnostic), position_of(&text, "2\n"), "{diagnostic:#?}");
}

// ── Route changes ───────────────────────────────────────────────────────────

#[test]
fn changing_or_removing_the_kind_or_schema_clears_the_earlier_results() {
    let probe = Probe::new();
    write(&probe.path("contract.yaml"), "$schema:\n    port: number\n");
    let broken = ROSTER.replace("roster_version: 1", "roster_version: 2");
    assert_eq!(probe.schema_diagnostics("doc.yaml", &broken).len(), 1);
    assert!(probe.completion_labels("doc.yaml", &format!("{broken}\n"), broken.len() + 1).contains(&"owner".to_string()));

    // Removing `kind` leaves a document with no schema: nothing stays.
    let unkinded = broken.replace("kind: roster\n", "");
    assert_eq!(probe.outcome("doc.yaml", &unkinded), "no-schema");
    assert_eq!(probe.schema_diagnostics("doc.yaml", &unkinded), Vec::new());
    let labels = probe.completion_labels("doc.yaml", &format!("{unkinded}\n"), unkinded.len() + 1);
    assert!(!labels.contains(&"owner".to_string()), "{labels:?}");

    // A root `$schema` selects its own contract instead.
    let rerouted = "$schema: ./contract.yaml\nport: nope\n";
    let diagnostic = only(&probe.schema_diagnostics("doc.yaml", rerouted)).clone();
    assert_eq!(start(&diagnostic), position_of(rerouted, "nope"), "{diagnostic:#?}");
    let labels = probe.completion_labels("doc.yaml", &format!("{rerouted}\n"), rerouted.len() + 1);
    assert!(!labels.contains(&"owner".to_string()), "{labels:?}");
}

#[test]
fn the_root_schema_route_completes_its_contract_and_assists_the_reference() {
    let probe = Probe::new();
    write(
        &probe.path("contract.yaml"),
        "description: the service contract\n$schema:\n    port: number -> the listening port\n    host: string\n",
    );
    let text = "$schema: ./contract.yaml\nport: 1\n\n";
    let labels = probe.completion_labels("service.yaml", text, text.len());
    assert!(labels.iter().any(|label| label == "host"), "{labels:?}");
    assert!(labels.iter().all(|label| label != "title"), "{labels:?}");
    let hover = probe.hover("service.yaml", text, text.find("port").unwrap() + 1);
    assert!(hover.as_deref().is_some_and(|body| body.contains("the listening port")), "{hover:?}");

    // The `$schema` value is a reference: its hover previews the file it
    // selects, as a Markdown document's does.
    let hover = probe.hover("service.yaml", text, text.find("contract").unwrap());
    assert!(hover.as_deref().is_some_and(|body| body.contains("the service contract")), "{hover:?}");
}

#[test]
fn an_undeclared_key_of_a_closed_definition_is_an_error_without_strict_mode() {
    let probe = Probe::new();
    let text = format!("{ROSTER}extra: 1\n");
    let diagnostic = only(&probe.schema_diagnostics("roster.data.yaml", &text)).clone();
    assert_eq!(code_of(&diagnostic), "dm.schema.unknown_key", "{diagnostic:#?}");
    assert_eq!(diagnostic.severity, Some(lsp_types::DiagnosticSeverity::ERROR), "{diagnostic:#?}");
}
