//! DMLS serves a YAML document claimed by a shipped `kind` (`kind: schema`,
//! `kind: schema-trigger`) from that kind's definition: the definition's
//! problems are published as `darkmatter.schema` diagnostics ranged by the
//! frontmatter convention, completion offers the definition's top-level keys
//! and `kind` values, and hover shows a key's definition description. Driven
//! in-process through `OverlayState::for_document` and the provider registry;
//! the stdio wiring is `kind_document_diagnostics_reach_a_stdio_client`.

use std::collections::BTreeMap;
use std::sync::Arc;

use dmls::capabilities::ClientProfile;
use dmls::config::DmlsConfig;
use dmls::context::RepositoryContexts;
use dmls::graph::{NoContexts, WorkspaceGraph};
use dmls::overlay::OverlayState;
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use lsp_types::{Diagnostic, InitializeParams, NumberOrString, Position, Uri};

use crate::common::{LspFixture, LspWorkspace};

const SCHEMA_SOURCE: &str = "darkmatter.schema";

/// A valid `kind: schema` document.
const SCHEMA_CONTROL: &str = "kind: schema\ndescription: a schema\n$schema:\n    title: string\n";

/// A valid `kind: schema-trigger` document.
const TRIGGER_CONTROL: &str =
    "kind: schema-trigger\nmatch:\n    $path: \"**/*.md\"\n$schema:\n    title: string\n";

/// One open document's providers, in a temporary workspace.
struct Probe {
    workspace: LspWorkspace,
    contexts: RepositoryContexts,
    config: DmlsConfig,
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
        std::fs::create_dir_all(workspace.path().join("schemas")).unwrap();
        let contexts = RepositoryContexts::new(LspFixture::fixture_snapshot(&workspace));
        Self { workspace, contexts, config }
    }

    /// Runs `inspect` over the context of `text` opened as `name`.
    fn with<T>(&self, name: &str, text: &str, inspect: impl FnOnce(&DocumentContext) -> T) -> T {
        let path = self.workspace.path().join(name);
        std::fs::write(&path, text).unwrap();
        let uri: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        let root = self.workspace.path().to_path_buf();
        let resolution = self.contexts.for_document(&path);
        let overlay = OverlayState::default().for_document(
            &uri,
            text,
            &path,
            &self.config,
            std::slice::from_ref(&root),
            &resolution,
        );
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

    /// The schema-problem diagnostics only (the kind report's, or for
    /// Markdown the frontmatter's): `darkmatter.schema` diagnostics carrying
    /// an `instance_path`, which the envelope diagnostics do not.
    fn schema_diagnostics(&self, name: &str, text: &str) -> Vec<Diagnostic> {
        self.diagnostics(name, text)
            .into_iter()
            .filter(|diagnostic| diagnostic.source.as_deref() == Some(SCHEMA_SOURCE))
            .filter(|diagnostic| {
                diagnostic.data.as_ref().is_some_and(|data| data.get("instance_path").is_some())
            })
            .collect()
    }

    fn claims_kind(&self, name: &str, text: &str) -> bool {
        self.with(name, text, |ctx| ctx.overlay.is_some_and(|overlay| overlay.kind_document.is_some()))
    }

    fn completion_labels(&self, name: &str, text: &str, offset: usize) -> Vec<String> {
        self.with(name, text, |ctx| {
            ProviderRegistry::with_substrate()
                .completion(ctx, offset)
                .into_iter()
                .map(|item| item.label)
                .collect()
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

fn code_of(diagnostic: &Diagnostic) -> &str {
    match &diagnostic.code {
        Some(NumberOrString::String(code)) => code,
        _ => "",
    }
}

fn start(diagnostic: &Diagnostic) -> (u32, u32) {
    (diagnostic.range.start.line, diagnostic.range.start.character)
}

/// The zero-based `(line, character)` of the first `needle` in `text`.
fn position_of(text: &str, needle: &str) -> (u32, u32) {
    let offset = text.find(needle).expect("needle present");
    let line = text[..offset].matches('\n').count() as u32;
    let column = offset - text[..offset].rfind('\n').map_or(0, |newline| newline + 1);
    (line, column as u32)
}

#[test]
fn valid_kind_documents_have_no_kind_diagnostics() {
    let probe = Probe::strict();
    assert_eq!(probe.schema_diagnostics("shape.yaml", SCHEMA_CONTROL), Vec::new());
    assert_eq!(probe.schema_diagnostics("schemas/trigger.yaml", TRIGGER_CONTROL), Vec::new());
    // The shipped definitions are themselves valid `kind: schema` documents.
    let schema = include_str!("../../../schemas/schema.yaml");
    let trigger = include_str!("../../../schemas/schema-trigger.yaml");
    assert_eq!(probe.schema_diagnostics("schema.yaml", schema), Vec::new());
    assert_eq!(probe.schema_diagnostics("schema-trigger.yaml", trigger), Vec::new());
}

#[test]
fn a_wrong_description_type_ranges_the_value() {
    let text = SCHEMA_CONTROL.replace("description: a schema", "description: 5");
    let diagnostics = Probe::new().schema_diagnostics("shape.yaml", &text);
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("one kind diagnostic: {diagnostics:#?}");
    };
    assert_eq!(code_of(diagnostic), "dm.schema.type_mismatch", "{diagnostic:#?}");
    assert_eq!(start(diagnostic), position_of(&text, "5"), "{diagnostic:#?}");
    assert_eq!(diagnostic.range.end, Position::new(1, 14), "{diagnostic:#?}");
}

#[test]
fn a_trigger_with_the_wrong_kind_literal_is_unclaimed_and_a_wrong_match_type_ranges_the_value() {
    let probe = Probe::new();
    // `kind` decides the claim, so a different literal is another document,
    // not a wrong value of this one.
    let other = TRIGGER_CONTROL.replace("kind: schema-trigger", "kind: schema-triggers");
    assert_eq!(probe.schema_diagnostics("schemas/other.yaml", &other), Vec::new());

    let text = TRIGGER_CONTROL.replace("match:\n    $path: \"**/*.md\"", "match: 5");
    let diagnostics = probe.schema_diagnostics("schemas/trigger.yaml", &text);
    assert!(
        diagnostics.iter().any(|diagnostic| start(diagnostic) == position_of(&text, "5")),
        "a diagnostic on the `match` value: {diagnostics:#?}"
    );
    // The trigger envelope's own error is still published beside it.
    let all = probe.diagnostics("schemas/trigger.yaml", &text);
    assert!(all.len() > diagnostics.len(), "the envelope diagnostic stands: {all:#?}");
}

#[test]
fn a_missing_match_ranges_the_root_mapping_under_strict_mode_only() {
    let text = "kind: schema-trigger\n$schema:\n    title: string\n";
    let strict = Probe::strict().schema_diagnostics("trigger.yaml", text);
    let [diagnostic] = strict.as_slice() else {
        panic!("one kind diagnostic: {strict:#?}");
    };
    assert_eq!(code_of(diagnostic), "dm.schema.missing_required", "{diagnostic:#?}");
    assert!(diagnostic.message.contains("match"), "{diagnostic:#?}");
    assert_eq!(start(diagnostic), (0, 0), "the root mapping: {diagnostic:#?}");
    assert_eq!(diagnostic.range.end, Position::new(3, 0), "the root mapping: {diagnostic:#?}");
    // A missing key is a strict-mode diagnostic, for frontmatter and kind
    // documents alike.
    assert_eq!(Probe::new().schema_diagnostics("trigger.yaml", text), Vec::new());
}

#[test]
fn invalid_yaml_keeps_only_its_parse_diagnostic_and_unclaimed_yaml_has_none() {
    let probe = Probe::strict();
    let broken = "kind: schema\ndescription: [\n";
    assert!(!probe.claims_kind("shape.yaml", broken));
    assert_eq!(probe.schema_diagnostics("shape.yaml", broken), Vec::new());
    let diagnostics = probe.diagnostics("shape.yaml", broken);
    assert!(!diagnostics.is_empty(), "the load failure is still reported: {diagnostics:#?}");

    for unclaimed in [
        "kind: recipe\ndescription: 5\n",
        "description: 5\n",
        "kind:\ndescription: 5\n",
        "- kind: schema\n",
    ] {
        assert_eq!(probe.diagnostics("other.yaml", unclaimed), Vec::new(), "{unclaimed}");
    }
}

#[test]
fn markdown_frontmatter_is_never_a_kind_document() {
    // `kind: schema-trigger` in Markdown frontmatter is frontmatter, judged by
    // the Darkmatter baseline: the trigger definition's required `match` never
    // applies, and the baseline's own problems are still reported.
    let probe = Probe::strict();
    let text = "---\nkind: schema-trigger\nmetadata: 5\n---\n\nBody\n";
    assert!(!probe.claims_kind("doc.md", text));
    let diagnostics = probe.schema_diagnostics("doc.md", text);
    assert!(
        diagnostics.iter().all(|diagnostic| !diagnostic.message.contains("match")),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| start(diagnostic) == position_of(text, "5")),
        "the baseline still judges `metadata`: {diagnostics:#?}"
    );
}

#[test]
fn completion_offers_the_definitions_missing_top_level_keys() {
    let text = "kind: schema\ndescription: a schema\n\n";
    let labels = Probe::new().completion_labels("shape.yaml", text, text.len() - 1);
    for key in ["categories", "types", "$schema"] {
        assert!(labels.iter().any(|label| label == key), "`{key}` offered: {labels:?}");
    }
    for present in ["kind", "description"] {
        assert!(!labels.iter().any(|label| label == present), "`{present}` is present: {labels:?}");
    }
    // The definition's keys alone, not the Markdown baseline's.
    assert!(!labels.iter().any(|label| label == "title"), "{labels:?}");

    let trigger = "kind: schema-trigger\n\n";
    let labels = Probe::new().completion_labels("trigger.yaml", trigger, trigger.len() - 1);
    for key in ["match", "$schema"] {
        assert!(labels.iter().any(|label| label == key), "`{key}` offered: {labels:?}");
    }
}

#[test]
fn completion_offers_the_claiming_kind_for_the_kind_value() {
    // Each definition fixes its own `kind` literal, so a claimed buffer is
    // offered that literal; a buffer with an empty `kind` is claimed by no
    // definition and gets none.
    let probe = Probe::new();
    let schema = "kind: schema\ndescription: a schema\n";
    let labels = probe.completion_labels("shape.yaml", schema, "kind: ".len());
    assert!(labels.iter().any(|label| label == "schema"), "{labels:?}");
    let trigger = "kind: schema-trigger\nmatch: {}\n";
    let labels = probe.completion_labels("trigger.yaml", trigger, "kind: ".len());
    assert!(labels.iter().any(|label| label == "schema-trigger"), "{labels:?}");
    let empty = "kind: \ndescription: a schema\n";
    let labels = probe.completion_labels("shape.yaml", empty, "kind: ".len());
    assert!(!labels.iter().any(|label| label.starts_with("schema")), "{labels:?}");
}

#[test]
fn hovering_a_key_shows_its_definition_description() {
    let probe = Probe::new();
    let description = probe
        .hover("shape.yaml", SCHEMA_CONTROL, SCHEMA_CONTROL.find("description").unwrap() + 2)
        .expect("a hover on `description`");
    assert!(description.contains("what the schema is for"), "{description}");

    let trigger = probe
        .hover("trigger.yaml", TRIGGER_CONTROL, TRIGGER_CONTROL.find("match").unwrap() + 1)
        .expect("a hover on `match`");
    assert!(trigger.contains("the conditions that select a document"), "{trigger}");
}

#[test]
fn hovering_the_schema_key_or_value_resolves_no_reference() {
    // In a kind document `$schema` is data: a string value is not a schema
    // file to look up, as it is in Markdown frontmatter.
    let probe = Probe::new();
    let markdown = "---\n$schema: ./elsewhere.yaml\n---\n";
    let reference = probe
        .hover("doc.md", markdown, markdown.find("./elsewhere").unwrap() + 2)
        .expect("frontmatter `$schema` is a reference");
    assert!(reference.contains("elsewhere.yaml"), "{reference}");

    let text = "kind: schema\n$schema: ./elsewhere.yaml\n";
    assert!(probe.claims_kind("shape.yaml", text));
    let on_key = probe.hover("shape.yaml", text, text.find("$schema").unwrap() + 1).unwrap_or_default();
    assert!(on_key.contains("the exported document schema"), "{on_key}");
    let on_value = probe.hover("shape.yaml", text, text.find("./elsewhere").unwrap() + 2);
    assert!(on_value.as_ref().is_none_or(|value| !value.contains("elsewhere.yaml")), "{on_value:?}");
    // Nor is it a link or a definition target.
    std::fs::write(probe.workspace.path().join("elsewhere.yaml"), "kind: schema\ntypes: {}\n").unwrap();
    let (links, definitions) = probe.with("shape.yaml", text, |ctx| {
        let registry = ProviderRegistry::with_substrate();
        (registry.document_links(ctx), registry.definition(ctx, text.find("./elsewhere").unwrap() + 2))
    });
    assert_eq!((links, definitions), (Vec::new(), Vec::new()));
    let (links, _) = probe.with("doc.md", markdown, |ctx| {
        (ProviderRegistry::with_substrate().document_links(ctx), ())
    });
    assert_eq!(links.len(), 1, "the frontmatter reference links: {links:#?}");
}

#[test]
fn a_rejected_definition_is_reported_once_by_the_envelope_when_it_reports() {
    let probe = Probe::new();
    // The standalone envelope locates the bad option token; the kind report's
    // coarser problem for the same text is not repeated.
    let schema = "kind: schema\ntypes:\n  a: alphanumeric(allow-dash)\n";
    assert!(probe.claims_kind("types.yaml", schema));
    let diagnostics = probe.diagnostics("types.yaml", schema);
    let rejections: Vec<&Diagnostic> =
        diagnostics.iter().filter(|diagnostic| diagnostic.message.contains("allow-dash")).collect();
    let [rejection] = rejections.as_slice() else {
        panic!("one report of the rejected option: {diagnostics:#?}");
    };
    assert_eq!(code_of(rejection), "dm.schema.invalid_type_definition", "{rejection:#?}");

    // Outside `schemas/` no trigger envelope diagnostic runs, so the kind
    // report's own problem for a rejected condition stands.
    let trigger = "kind: schema-trigger\nmatch:\n  title: alphanumeric(allow-dash)\n";
    let diagnostics = probe.schema_diagnostics("trigger.yaml", trigger);
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.message.contains("allow-dash")),
        "{diagnostics:#?}"
    );
}
