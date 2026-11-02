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

    /// A probe whose `SCHEMAS_DIR` is the folder of `relative`, a
    /// repository-relative definition path holding `definition`.
    fn with_schemas_dir(relative: &str, definition: &str) -> Self {
        let workspace = LspWorkspace::new();
        init_repository(workspace.path());
        let copy = workspace.path().join(relative);
        write(&copy, definition);
        let folder = copy.parent().expect("definition folder").to_string_lossy().into_owned();
        let snapshot = LspFixture::fixture_snapshot(&workspace)
            .with_env(std::collections::HashMap::from([("SCHEMAS_DIR".to_string(), folder)]));
        let contexts = RepositoryContexts::new(snapshot);
        Self { workspace, contexts, config: DmlsConfig::default(), state: OverlayState::default() }
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
                YamlDocumentOutcome::Pending => "pending",
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

/// A document holding only an inline root `$schema` (once read as an
/// informal standalone schema) is data on the root-`$schema` route, so the
/// property it lacks is an error with the add-key action, and a declaration
/// failure is that route's one prepare error.
#[test]
fn an_informal_standalone_schema_is_judged_as_data_and_each_problem_is_published_once() {
    let probe = Probe::new();
    let text = "$schema:\n    owner: string(required)\n";
    assert_eq!(probe.outcome("team.yaml", text), "validated");
    let diagnostics = probe.schema_diagnostics("team.yaml", text);
    let diagnostic = only(&diagnostics);
    assert_eq!(code_of(diagnostic), "dm.schema.missing_required", "{diagnostic:#?}");
    let titles: Vec<String> = probe.with("team.yaml", text, |ctx| {
        dmls::providers::code_actions::code_actions(ctx, &diagnostics)
            .into_iter()
            .map(|action| match action {
                lsp_types::CodeActionOrCommand::CodeAction(action) => action.title,
                lsp_types::CodeActionOrCommand::Command(command) => command.title,
            })
            .collect()
    });
    assert!(titles.iter().any(|title| title == "Add required key `owner`"), "{titles:?}");

    // A sibling `types` is data, so `team` names no type: the route's one
    // named-type error, whatever `types` holds.
    let text = "$schema:\n    owner: team\ntypes:\n    team: missing\n";
    assert_eq!(probe.outcome("team.yaml", text), "failed");
    let diagnostics = probe.schema_diagnostics("team.yaml", text);
    let diagnostic = only(&diagnostics);
    assert_eq!(code_of(diagnostic), "dm.schema.invalid_type_definition", "{diagnostic:#?}");
}

/// The kind route's own judgment of a `kind: schema` document, apart from
/// the envelope that also reports a broken import: the `schema` kind check
/// reads the imported type library's unsaved text, and the cached judgment
/// is rebuilt when that text changes or the buffer closes.
#[test]
fn a_schema_documents_kind_check_reads_an_imported_type_librarys_unsaved_text() {
    let probe = Probe::new();
    let library = probe.path("shared/types.yaml");
    write(&library, "kind: schema\ntypes:\n    Owner: string\n");
    let text = "kind: schema\n$schema:\n    owner: Owner@./shared/types.yaml\n";
    let kind_check_valid = || {
        probe.with("contract.yaml", text, |ctx| {
            let yaml = ctx.overlay.and_then(|overlay| overlay.yaml_document.as_deref()).expect("a YAML overlay");
            match &yaml.outcome {
                YamlDocumentOutcome::Validated(validation) => validation.report.valid,
                _ => panic!("the `schema` kind validates the document"),
            }
        })
    };
    assert!(kind_check_valid(), "saved control");

    probe.state.set_schema_buffer(&library, Some("kind: schema\ntypes:\n    Team: string\n"));
    assert!(!kind_check_valid(), "the unsaved library no longer defines `Owner`");

    probe.state.set_schema_buffer(&library, None);
    assert!(kind_check_valid(), "closed unsaved");
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

// ── Kinds that claim nothing ────────────────────────────────────────────────

/// Every `kind` that claims no kind and no trigger envelope: absent, null
/// (explicit and empty-value), non-string scalars and collections, the empty
/// string, and a string no definition claims.
const UNCLAIMED_KINDS: &[Option<&str>] = &[
    None,
    Some(" null"),
    Some(""),
    Some(" 42"),
    Some(" true"),
    Some(" [roster]"),
    Some(" []"),
    Some(" [roster, 1]"),
    Some(" [1, 2]"),
    Some(" {a: b}"),
    Some(" {}"),
    Some(" \"\""),
    Some(" inventory"),
];

/// `rest` behind each [`UNCLAIMED_KINDS`] line, opened inside the tree's
/// schema root and outside it: a file in a `schemas/` folder whose `kind`
/// does not name the trigger envelope is judged by whole-document selection
/// alone, exactly as the same file elsewhere.
fn each_unclaimed_kind(rest: &str, mut check: impl FnMut(&str, &str)) {
    for name in ["schemas/data.yaml", "docs/data.yaml"] {
        for kind in UNCLAIMED_KINDS {
            let text = match kind {
                Some(value) => format!("kind:{value}\n{rest}"),
                None => rest.to_string(),
            };
            check(name, &text);
        }
    }
}

#[test]
fn an_unclaimed_kind_without_a_root_schema_is_not_validated_inside_or_outside_a_schema_folder() {
    let probe = Probe::new();
    each_unclaimed_kind("owner: ops\n", |name, text| {
        assert_eq!(probe.outcome(name, text), "no-schema", "{name}: {text:?}");
        assert_eq!(probe.diagnostics(name, text), Vec::new(), "{name}: {text:?}");
    });
}

#[test]
fn an_unclaimed_kind_with_a_missing_root_schema_reports_only_the_reference_error() {
    let probe = Probe::new();
    each_unclaimed_kind("$schema: ./missing.yaml\nowner: ops\n", |name, text| {
        assert_eq!(probe.outcome(name, text), "failed", "{name}: {text:?}");
        let diagnostics = probe.diagnostics(name, text);
        let diagnostic = only(&diagnostics);
        assert_eq!(code_of(diagnostic), "dm.schema.prepare", "{name}: {diagnostic:#?}");
        assert_eq!(start(diagnostic), position_of(text, "./missing.yaml"), "{name}: {diagnostic:#?}");
    });
}

#[test]
fn an_unclaimed_kind_with_an_inline_root_schema_is_judged_by_it_alone() {
    let probe = Probe::new();
    each_unclaimed_kind("$schema:\n    owner: string\nowner: ops\n", |name, text| {
        assert_eq!(fallback_diagnostics(&probe, name, text), Vec::new(), "{name}: {text:?}");
    });
}

/// An unloadable file in a schema folder whose text still declares the
/// trigger envelope reports its parse problem once, as the trigger's error;
/// one that declares no envelope reports the loader's.
#[test]
fn an_unloadable_file_in_a_schema_folder_reports_its_parse_problem_once() {
    let probe = Probe::new();
    for (text, expected) in [
        ("kind: schema-trigger\nmatch: [\n", "dm.schema.prepare"),
        ("kind:\nowner: [\n", "dm.frontmatter.yaml_parse"),
        ("kind: 42\nowner: [\n", "dm.frontmatter.yaml_parse"),
    ] {
        assert_eq!(probe.outcome("schemas/broken.yaml", text), "load-failed", "{text:?}");
        let diagnostics = probe.diagnostics("schemas/broken.yaml", text);
        assert_eq!(code_of(only(&diagnostics)), expected, "{text:?}: {diagnostics:#?}");
    }
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

// ── The selected contract alone judges data ─────────────────────────────────
//
// A document with no claimed `kind` selects its root `$schema` as its
// contract. Its other root fields are data of that contract, even when
// their names (`description`, `categories`, `types`) are those of a schema
// document's metadata, so nothing judges them as a schema envelope.

/// The data shapes each metadata-looking field takes under a selected
/// `{field}: unknown`; every one must be clean.
const ANY_SHAPES: &[&str] = &["null", "123", "[ops, 123]", "[123]", "\"\"", "[]", "{}", "ops"];

/// The diagnostics of `text`, opened as `name`, which its root `$schema`
/// must validate.
fn fallback_diagnostics(probe: &Probe, name: &str, text: &str) -> Vec<Diagnostic> {
    assert_eq!(probe.outcome(name, text), "validated", "{text}");
    probe.diagnostics(name, text)
}

macro_rules! any_shape_of_a_metadata_named_field_is_data {
    ($($test:ident: $field:literal,)*) => {$(
        #[test]
        fn $test() {
            let probe = Probe::new();
            let absent = format!("$schema:\n    {field}: unknown\nowner: ops\n", field = $field);
            assert_eq!(fallback_diagnostics(&probe, "data.yaml", &absent), Vec::new(), "absent");
            for shape in ANY_SHAPES {
                let text = format!("$schema:\n    {field}: unknown\n{field}: {shape}\n", field = $field);
                assert_eq!(fallback_diagnostics(&probe, "data.yaml", &text), Vec::new(), "{text}");
            }
        }
    )*};
}

any_shape_of_a_metadata_named_field_is_data! {
    any_description_shape_is_data_of_the_selected_contract: "description",
    any_categories_shape_is_data_of_the_selected_contract: "categories",
    any_types_shape_is_data_of_the_selected_contract: "types",
}

#[test]
fn metadata_named_data_the_selected_contract_accepts_is_clean() {
    let probe = Probe::new();
    write(&probe.path("contract.yaml"), "$schema:\n    description: string\n");
    for text in [
        // An optional `string` admits null.
        "$schema:\n    description: string\ndescription: null\n",
        // The same with a declared ordinary property beside it.
        "$schema:\n    description: string\n    owner: string\ndescription: null\nowner: ops\n",
        "$schema:\n    categories: string[]\ncategories: [ops, ops]\n",
        "$schema:\n    types: string[]\ntypes: [ops]\n",
        "$schema:\n    types: unknown\ntypes: { mode: production }\n",
        "$schema: ./contract.yaml\ndescription: null\n",
        "$schema:\n    - { description: string, kind: literal(a) }\n    - { description: string, kind: literal(b) }\ndescription: null\n",
    ] {
        assert_eq!(fallback_diagnostics(&probe, "data.yaml", text), Vec::new(), "{text}");
    }
}

/// The selected contract still judges the same fields: a value it rejects
/// is its problem, reported once.
#[test]
fn metadata_named_data_the_selected_contract_rejects_is_its_problem_alone() {
    let probe = Probe::new();
    let text = "$schema:\n    categories: number[]\ncategories: [ops]\n";
    let diagnostics = fallback_diagnostics(&probe, "data.yaml", text);
    let diagnostic = only(&diagnostics);
    assert_eq!(code_of(diagnostic), "dm.schema.type_mismatch", "{diagnostic:#?}");
    assert_eq!(start(diagnostic), position_of(text, "ops]"), "{diagnostic:#?}");
}

/// A sibling `types` of a fallback document is data, not a type library its
/// `$schema` can name: the bare name is the route's one prepare error, as
/// `md schema validate` reports it, and nothing navigates into `types`.
#[test]
fn a_fallback_schema_names_no_sibling_types() {
    let probe = Probe::new();
    let text = "$schema:\n    owner: team\ntypes:\n    team: string\n";
    assert_eq!(probe.outcome("team.yaml", text), "failed");
    let diagnostics = probe.diagnostics("team.yaml", text);
    let diagnostic = only(&diagnostics);
    assert_eq!(code_of(diagnostic), "dm.schema.invalid_type_definition", "{diagnostic:#?}");
    assert!(diagnostic.message.contains("unknown named type"), "{diagnostic:#?}");
    assert_eq!(start(diagnostic), position_of(text, "team\n"), "{diagnostic:#?}");
}

/// `shipped` with every root-closure line removed.
fn opened(shipped: &str) -> String {
    let opened: String =
        shipped.lines().filter(|line| line.trim() != "$constraints: { closed: true }").map(|line| format!("{line}\n")).collect();
    assert_ne!(opened, shipped, "the definition had a root closure to remove");
    opened
}

/// A trigger whose `match` passes the trigger grammar.
const VALID_TRIGGER: &str = "kind: schema-trigger\nmatch:\n    all:\n        - $path: \"docs/**\"\n        - title: string(required)\n";

/// A checkout copy of a shipped definition with its root closure removed
/// is the selected definition of its kind, so a key that copy admits is
/// clean; the compiled envelope and the trigger parser never judge the
/// document's shape again. The trigger is judged both as an ordinary
/// document and as a trigger file in a schema root.
#[test]
fn an_opened_checkout_definition_alone_judges_its_documents_shape() {
    let cases = [
        (
            "simplified-schema/schemas/schema.yaml",
            include_str!("../../../../simplified-schema/schemas/schema.yaml"),
            "docs/contract.yaml",
            "kind: schema\ncategories: [text]\n$schema:\n    title: string(category(text))\n",
        ),
        (
            "simplified-schema/schemas/yaml-schema.yaml",
            include_str!("../../../../simplified-schema/schemas/yaml-schema.yaml"),
            "docs/inventory.yaml",
            "kind: yaml-schema\n$schema:\n    kind: literal(inventory; required)\n    count: number\n",
        ),
        ("darkmatter/schemas/schema-trigger.yaml", include_str!("../../../schemas/schema-trigger.yaml"), "docs/trigger.yaml", VALID_TRIGGER),
        (
            "darkmatter/schemas/schema-trigger.yaml",
            include_str!("../../../schemas/schema-trigger.yaml"),
            "darkmatter/schemas/review.yaml",
            VALID_TRIGGER,
        ),
    ];
    for (relative, shipped, name, text) in cases {
        let probe = Probe::with_schemas_dir(relative, &opened(shipped));
        assert_eq!(probe.outcome(name, text), "validated", "{name}");
        assert_eq!(probe.schema_diagnostics(name, text), Vec::new(), "{name}: control");
        let added = format!("{text}note: hi\n");
        let errors: Vec<Diagnostic> = probe
            .schema_diagnostics(name, &added)
            .into_iter()
            .filter(|diagnostic| diagnostic.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
            .collect();
        assert_eq!(errors, Vec::new(), "{name}: added `note`");
    }
}
