//! Host selection (`config::registration::{select, RegisteredHosts}`) and the
//! registered layer of `DarkmatterSchemas`: which host a document belongs
//! to, how the identified host's triggers layer between the baseline and the
//! schema roots' triggers, ordinary trigger semantics (literals, `$path`,
//! malformed operands, matcher-only triggers), the absence of any root-catalog
//! host, moved copies, conflicts, ambiguity, registered YAML kinds, cache
//! identity, inspection, and composition with and without a caller's own
//! host.

use std::path::PathBuf;
use std::sync::Arc;

use crate::host_fixtures::*;
use darkmatter::config::registration::{
    HOST_AMBIGUOUS_CODE, HOST_REGISTRATION_CODE, HostSelection, RegisteredHosts, select,
};
use darkmatter::markdown::compose::expression::{BindingView, HostGlobals, ScopeId};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use darkmatter::markdown::{Markdown, MarkdownError};
use simplified_schema::triggers::TriggerSource;

const HOST: &str = "../host/darkmatter-host.yaml";

/// A Widget document: identified by `widget`, matched by both triggers.
const WIDGET_DOCUMENT: &str = "---\nwidget:\n  size: big\ngadget: name\n---\nBody.\n";

/// The trigger files of `effective`'s trigger layers, in layer order.
fn trigger_files(effective: &EffectiveSchema) -> Vec<PathBuf> {
    effective
        .layers()
        .triggers
        .iter()
        .map(|source| match source {
            TriggerSource::File(path) => path.clone(),
            other => panic!("a file trigger: {other:?}"),
        })
        .collect()
}

/// The paths of `effective`'s problems for `markdown`'s frontmatter.
fn problem_paths(effective: &EffectiveSchema, markdown: &Markdown) -> Vec<String> {
    let report = effective.validate(&darkmatter::markdown::schemas::frontmatter_as_json(markdown));
    let mut paths: Vec<String> = report.problems.iter().map(|problem| problem.path.clone()).collect();
    paths.sort();
    paths
}

fn effective_schema(schemas: &DarkmatterSchemas, markdown: &Markdown) -> EffectiveSchema {
    schemas.effective_for(markdown).expect("the schema assembles").expect("a schema applies")
}

// ── identification and layering ─────────────────────────────────────────────

#[test]
fn an_identified_document_layers_the_host_triggers_in_bundle_path_order() {
    let fixture = Fixture::new();
    let hosts = fixture.hosts(&[HOST]);
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", WIDGET_DOCUMENT);
    let schemas = DarkmatterSchemas::new(context).with_registered_hosts(Arc::clone(&hosts));

    let selection = schemas.registered_selection_for(&markdown);
    assert!(selection.diagnostics.is_empty(), "{:#?}", selection.diagnostics);
    let HostSelection::Registered(profile) = &selection.selection else { panic!("acme: {:?}", selection.selection) };
    assert_eq!(profile.host(), "acme");
    assert_eq!(selection.matched, [0, 1], "gadget and widget both match");

    let effective = effective_schema(&schemas, &markdown);
    let widget = fixture.path("host/schemas/widget.yaml");
    let gadget = fixture.path("host/schemas/gadget.yaml");
    assert_eq!(trigger_files(&effective), [gadget.clone(), widget.clone()]);
    let registered = effective.layers().registered.as_ref().expect("a registered origin");
    assert_eq!((registered.host.as_str(), registered.version.as_str()), ("acme", "1.2.0"));
    assert_eq!(registered.manifest, fixture.manifest());
    assert_eq!(registered.revision, profile.revision());
    assert_eq!(registered.triggers, [gadget.clone(), widget.clone()]);
    // `size: big` breaks the registered `widget.size: number`; the payload's
    // `widget@./library.yaml` import resolved from the trigger's own folder.
    assert_eq!(problem_paths(&effective, &markdown), ["/widget/size"]);
    assert!(effective.dependencies().contains(&fixture.path("host/schemas/library.yaml")), "{:?}", effective.dependencies());
}

/// The order of `files` is no part of a host: reordering it keeps the
/// revision, the selection, the matched triggers, and the layer order.
#[test]
fn reordered_files_keep_the_revision_and_the_selection() {
    let fixture = Fixture::new();
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", WIDGET_DOCUMENT);
    let before = fixture.hosts(&[HOST]);
    fixture.edit_manifest(|manifest| {
        manifest.replace("    - ./schemas/widget.yaml\n    - ./schemas/gadget.yaml\n", "    - ./schemas/gadget.yaml\n    - ./schemas/widget.yaml\n")
    });
    let after = fixture.hosts(&[HOST]);
    assert_eq!(before.loaded().profiles[0].revision(), after.loaded().profiles[0].revision());

    let selected = |hosts: &Arc<RegisteredHosts>| {
        let schemas = DarkmatterSchemas::new(context.clone()).with_registered_hosts(Arc::clone(hosts));
        let selection = schemas.registered_selection_for(&markdown);
        (selection.matched_triggers().map(|trigger| trigger.bundle_path.clone()).collect::<Vec<_>>(), trigger_files(&effective_schema(&schemas, &markdown)))
    };
    assert_eq!(selected(&before), selected(&after));
}

#[test]
fn an_unrelated_document_gets_no_host_schema_globals_or_functions() {
    let fixture = Fixture::new();
    let hosts = fixture.hosts(&[HOST]);
    let (markdown, context) = fixture.document(&fixture.context(), "work/notes.md", "---\ntitle: Notes\nsize: 3\n---\n");
    let schemas = DarkmatterSchemas::new(context.clone()).with_registered_hosts(Arc::clone(&hosts));

    assert!(matches!(schemas.registered_selection_for(&markdown).selection, HostSelection::Md));
    assert!(schemas.effective_for(&markdown).unwrap().is_none(), "no layer applies");
    let selected = select(Some(&hosts), &darkmatter::markdown::schemas::frontmatter_as_json(&markdown), None, &context);
    assert!(matches!(selected.selection, HostSelection::Md), "no globals, functions, or lifecycle: {:?}", selected.selection);
}

#[test]
fn a_same_basename_root_trigger_overrides_a_declaration_without_shadowing_the_registered_one() {
    let fixture = Fixture::new();
    let hosts = fixture.hosts(&[HOST]);
    // A root `widget.yaml` (the registered trigger's name) redeclaring
    // `gadget` as a number.
    fixture.write(
        "work/schemas/widget.yaml",
        "kind: schema-trigger\nmatch:\n    widget: object(required)\n$schema:\n    gadget: number\n",
    );
    let (markdown, context) = fixture.document(&fixture.repository(None), "work/doc.md", WIDGET_DOCUMENT);
    let schemas = DarkmatterSchemas::new(context).with_trigger_discovery().unwrap().with_registered_hosts(hosts);

    let effective = effective_schema(&schemas, &markdown);
    assert_eq!(
        trigger_files(&effective),
        [fixture.path("host/schemas/gadget.yaml"), fixture.path("host/schemas/widget.yaml"), fixture.path("work/schemas/widget.yaml")]
    );
    // Both apply: the registered `widget` still rejects `size: big`, and the
    // root's `gadget: number` wins over the registered `gadget: string`.
    assert_eq!(problem_paths(&effective, &markdown), ["/gadget", "/widget/size"]);
}

#[test]
fn a_registered_trigger_also_visible_in_a_root_applies_once() {
    let fixture = Fixture::new();
    // The host is installed at the repository root, so its `schemas/` is the
    // tree's schema root too.
    fixture.install("work");
    let hosts = fixture.hosts(&["../work/darkmatter-host.yaml"]);
    let (markdown, context) = fixture.document(&fixture.repository(None), "work/doc.md", WIDGET_DOCUMENT);
    let schemas = DarkmatterSchemas::new(context).with_trigger_discovery().unwrap().with_registered_hosts(hosts);

    let files = trigger_files(&effective_schema(&schemas, &markdown));
    let widget = fixture.path("work/schemas/widget.yaml");
    let gadget = fixture.path("work/schemas/gadget.yaml");
    assert_eq!(files.iter().filter(|file| **file == widget).count(), 1, "{files:?}");
    assert_eq!(files.iter().filter(|file| **file == gadget).count(), 1, "{files:?}");
    assert_eq!(&files[..2], [gadget, widget], "kept as the registered occurrences: {files:?}");
}

#[test]
fn the_authored_schema_overrides_a_registered_trigger() {
    let fixture = Fixture::new();
    let hosts = fixture.hosts(&[HOST]);
    let (markdown, context) = fixture.document(
        &fixture.context(),
        "work/doc.md",
        "---\n$schema:\n  widget:\n    size: string\nwidget:\n  size: big\n---\n",
    );
    let schemas = DarkmatterSchemas::new(context).with_registered_hosts(hosts);

    let effective = effective_schema(&schemas, &markdown);
    assert_eq!(trigger_files(&effective), [fixture.path("host/schemas/widget.yaml")]);
    assert!(effective.layers().document);
    assert!(problem_paths(&effective, &markdown).is_empty(), "the document's `size: string` wins");
}

// ── no root-catalog host ────────────────────────────────────────────────────

#[test]
fn a_root_local_globals_lifecycle_and_function_catalog_supply_no_host() {
    let fixture = Fixture::new();
    fixture.root_catalog("work/schemas", "other");
    let (markdown, context) = fixture.document(&fixture.repository(None), "work/notes.md", "---\ntitle: Notes\nrun:\n  count: 1\n---\n");
    let frontmatter = darkmatter::markdown::schemas::frontmatter_as_json(&markdown);

    let unregistered = select(None, &frontmatter, Some(&fixture.path("work/notes.md")), &context);
    let registered = select(Some(&fixture.hosts(&[HOST])), &frontmatter, Some(&fixture.path("work/notes.md")), &context);

    assert!(matches!(unregistered.selection, HostSelection::Md), "{:?}", unregistered.selection);
    assert!(matches!(registered.selection, HostSelection::Md), "{:?}", registered.selection);
    assert!(unregistered.diagnostics.is_empty() && registered.diagnostics.is_empty());
}

#[test]
fn a_registered_host_s_catalog_in_its_package_area_root_supplies_nothing_more() {
    let fixture = Fixture::new();
    let hosts = fixture.hosts(&[HOST]);
    // A Claudine-like package area holding the host's catalog, lifecycle
    // artifact, and function catalog in its schema root.
    fixture.root_catalog("work/acme/schemas", "acme");
    let (markdown, context) =
        fixture.document(&fixture.repository(Some("acme")), "work/acme/notes.md", "---\ninitialize:\n  - say: hi\n---\n");
    let frontmatter = darkmatter::markdown::schemas::frontmatter_as_json(&markdown);
    let document = fixture.path("work/acme/notes.md");

    let selected = select(Some(&hosts), &frontmatter, Some(&document), &context);
    assert!(matches!(selected.selection, HostSelection::Md), "a shared-only document stays md: {:?}", selected.selection);
    assert!(matches!(select(None, &frontmatter, Some(&document), &context).selection, HostSelection::Md));
}

// ── trigger semantics ───────────────────────────────────────────────────────

/// `fixture` with the `acme` bundle plus the trigger `schemas/extra.yaml`
/// (`text`), and the registered hosts.
fn with_trigger(fixture: &Fixture, text: &str) -> Arc<RegisteredHosts> {
    fixture.install_bundle("host", &Bundle::acme().with_file("schemas/extra.yaml", text));
    let hosts = fixture.hosts(&[HOST]);
    assert!(hosts.diagnostics().is_empty(), "{:#?}", hosts.diagnostics());
    hosts
}

/// The selection of the frontmatter `yaml` for the document `relative`.
fn select_yaml(fixture: &Fixture, hosts: &RegisteredHosts, relative: &str, yaml: &str) -> darkmatter::config::registration::Selected {
    let (markdown, context) = fixture.document(&fixture.context(), relative, &format!("---\n{yaml}---\nBody.\n"));
    hosts.identify(&darkmatter::markdown::schemas::frontmatter_as_json(&markdown), Some(&fixture.path(relative)), &context)
}

fn matched_paths(selected: &darkmatter::config::registration::Selected) -> Vec<String> {
    selected.matched_triggers().map(|trigger| trigger.bundle_path.clone()).collect()
}

#[test]
fn a_literal_trigger_matches_only_an_equal_value() {
    let fixture = Fixture::new();
    let hosts = with_trigger(&fixture, "kind: schema-trigger\nmatch:\n    kind: literal(sequence; required)\n");

    let equal = select_yaml(&fixture, &hosts, "work/doc.md", "kind: sequence\n");
    let other = select_yaml(&fixture, &hosts, "work/doc.md", "kind: other\n");
    let schema_key = select_yaml(&fixture, &hosts, "work/doc.md", "$schema:\n  kind: string\nkind: sequence\n");

    assert!(matches!(equal.selection, HostSelection::Registered(_)), "{:?}", equal.selection);
    assert_eq!(matched_paths(&equal), ["schemas/extra.yaml"]);
    assert!(matches!(other.selection, HostSelection::Md), "{:?}", other.selection);
    assert_eq!(matched_paths(&schema_key), ["schemas/extra.yaml"], "a `$schema` key is ignored");
}

#[test]
fn a_path_qualified_trigger_matches_only_under_its_pattern() {
    let fixture = Fixture::new();
    let hosts = with_trigger(&fixture, "kind: schema-trigger\nmatch:\n    all:\n        - bolt: string(required)\n        - $path: \"notes/**\"\n");

    // Bare patterns read from the document's tree root: the repository.
    let repository = fixture.repository(None);
    let at = |relative: &str| {
        let (markdown, context) = fixture.document(&repository, relative, "---\nbolt: x\n---\n");
        hosts.identify(&darkmatter::markdown::schemas::frontmatter_as_json(&markdown), Some(&fixture.path(relative)), &context)
    };
    let inside = at("work/notes/a.md");
    let outside = at("work/a.md");
    let detached = hosts.identify(&serde_json::json!({ "bolt": "x" }), None, &fixture.context());

    assert_eq!(matched_paths(&inside), ["schemas/extra.yaml"]);
    assert!(matches!(outside.selection, HostSelection::Md), "{:?}", outside.selection);
    assert!(matches!(detached.selection, HostSelection::Md), "no path defeats `$path`: {:?}", detached.selection);
}

#[test]
fn a_matcher_only_trigger_identifies_the_host_and_layers_nothing() {
    let fixture = Fixture::new();
    let hosts = with_trigger(&fixture, "kind: schema-trigger\nmatch:\n    bolt: string(required)\n");
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", "---\nbolt: x\n---\n");
    let schemas = DarkmatterSchemas::new(context).with_registered_hosts(hosts);

    let selected = schemas.registered_selection_for(&markdown);

    let HostSelection::Registered(profile) = &selected.selection else { panic!("acme: {:?}", selected.selection) };
    let index = profile.triggers().iter().position(|trigger| trigger.bundle_path == "schemas/extra.yaml").unwrap();
    assert_eq!(selected.matched, [index]);
    let layered = schemas.effective_for(&markdown).expect("the schema assembles");
    assert!(layered.as_ref().is_none_or(|effective| trigger_files(effective).is_empty()), "no payload layers");
}

#[test]
fn matched_lists_exactly_the_matching_triggers() {
    let fixture = Fixture::new();
    let hosts = fixture.hosts(&[HOST]);

    let gadget = select_yaml(&fixture, &hosts, "work/doc.md", "gadget: name\n");
    let widget = select_yaml(&fixture, &hosts, "work/doc.md", "widget:\n  size: 3\n");

    assert_eq!((gadget.matched.clone(), matched_paths(&gadget)), (vec![0], vec!["schemas/gadget.yaml".to_string()]));
    assert_eq!((widget.matched.clone(), matched_paths(&widget)), (vec![1], vec!["schemas/widget.yaml".to_string()]));
}

/// The relay's presence triggers (`unknown[](required)`): an operand of the
/// wrong type defeats the gate, so the document stays `md`; a document with
/// only shared lifecycle keys does too.
#[test]
fn a_malformed_presence_operand_or_a_shared_event_only_document_stays_md() {
    let fixture = Fixture::new();
    fixture.install_relay("relay");
    let hosts = fixture.hosts(&["../relay/darkmatter-host.yaml"]);
    assert!(hosts.diagnostics().is_empty(), "{:#?}", hosts.diagnostics());

    let control = select_yaml(&fixture, &hosts, "work/doc.md", "sequence: []\n");
    let malformed = select_yaml(&fixture, &hosts, "work/doc.md", "sequence: 42\n");
    let shared = select_yaml(&fixture, &hosts, "work/doc.md", "composed:\n  - info: c\nfailed:\n  - info: f\ninitialize:\n  - info: i\n");

    assert!(matches!(control.selection, HostSelection::Registered(_)), "{:?}", control.selection);
    assert!(matches!(malformed.selection, HostSelection::Md), "{:?}", malformed.selection);
    assert!(matches!(shared.selection, HostSelection::Md), "{:?}", shared.selection);
    assert!(malformed.diagnostics.is_empty() && shared.diagnostics.is_empty());
}

// ── registered YAML kinds ───────────────────────────────────────────────────

#[test]
fn a_registered_yaml_kind_is_indexed_by_the_kinds_it_claims() {
    let fixture = Fixture::new();
    fixture.install_bundle(
        "host",
        &Bundle::acme().with_file(
            "schemas/roster.yaml",
            "kind: yaml-schema\n$schema:\n    kind: literal(roster; required)\n    platforms: string[](required)\n",
        ),
    );
    let hosts = fixture.hosts(&[HOST]);
    assert!(hosts.diagnostics().is_empty(), "{:#?}", hosts.diagnostics());

    let [definition] = hosts.loaded().profiles[0].yaml_kinds() else { panic!("one definition") };
    assert_eq!(definition.source().path, "schemas/roster.yaml");
    assert_eq!(definition.source().file, fixture.path("host/schemas/roster.yaml"));
    let kinds: Vec<&str> = hosts.yaml_kinds().claims().map(|(kind, _)| kind).collect();
    assert_eq!(kinds, ["roster"]);
    assert!(matches!(hosts.yaml_kinds().claim("roster"), Some(simplified_schema::yaml_documents::Claim::Healthy(_))));
}

// ── copies, conflicts, and ambiguity ────────────────────────────────────────

#[test]
fn a_moved_copy_collapses_into_one_agreed_claim_and_selects_once() {
    let fixture = Fixture::new();
    fixture.install("moved/acme");
    let hosts = fixture.hosts(&[HOST, "../moved/acme/darkmatter-host.yaml"]);
    assert!(hosts.diagnostics().is_empty(), "{:#?}", hosts.diagnostics());
    assert_eq!(hosts.claims().len(), 1);
    assert!(matches!(&hosts.claims()[0], darkmatter::config::registration::HostClaim::Agreed { collapsed, .. } if collapsed.len() == 1));

    let selected = select_yaml(&fixture, &hosts, "work/doc.md", "gadget: name\n");

    let HostSelection::Registered(profile) = &selected.selection else { panic!("acme: {:?}", selected.selection) };
    assert_eq!(profile.manifest().path, fixture.manifest(), "the first claim stands for both");
    assert!(selected.diagnostics.is_empty());
}


#[test]
fn conflicting_claims_of_one_host_identify_nothing_and_never_fall_back() {
    let fixture = Fixture::new();
    fixture.install("other");
    fixture.write("other/darkmatter-host.yaml", &fixture.manifest_text("other").replace("version: 1.2.0", "version: 1.3.0"));
    let hosts = fixture.hosts(&[HOST, "../other/darkmatter-host.yaml"]);
    assert_eq!(hosts.diagnostics().iter().filter(|diagnostic| diagnostic.code == HOST_REGISTRATION_CODE).count(), 2);
    fixture.root_catalog("work/schemas", "legacy");
    let (markdown, context) = fixture.document(&fixture.repository(None), "work/doc.md", WIDGET_DOCUMENT);
    let frontmatter = darkmatter::markdown::schemas::frontmatter_as_json(&markdown);

    let selected = select(Some(&hosts), &frontmatter, None, &context);
    let HostSelection::Ambiguous(named) = &selected.selection else { panic!("refused: {:?}", selected.selection) };
    assert_eq!(named, &["acme".to_string()]);
    assert!(!selected.diagnostics.is_empty());
    assert!(selected.diagnostics.iter().all(|diagnostic| diagnostic.code == HOST_REGISTRATION_CODE), "{:#?}", selected.diagnostics);

    let schemas = DarkmatterSchemas::new(context).with_registered_hosts(hosts);
    assert!(schemas.effective_for(&markdown).unwrap().is_none(), "neither claim layers");
}

#[test]
fn two_hosts_identifying_one_document_are_ambiguous_and_analyzed_as_md() {
    let fixture = Fixture::new();
    fixture.install_host("bolt", "bolt");
    let hosts = fixture.hosts(&[HOST, "../bolt/darkmatter-host.yaml"]);
    assert!(hosts.diagnostics().is_empty(), "{:#?}", hosts.diagnostics());
    fixture.root_catalog("work/schemas", "legacy");
    let (markdown, context) = fixture.document(&fixture.repository(None), "work/doc.md", WIDGET_DOCUMENT);
    let frontmatter = darkmatter::markdown::schemas::frontmatter_as_json(&markdown);

    let selected = select(Some(&hosts), &frontmatter, Some(&fixture.path("work/doc.md")), &context);
    let HostSelection::Ambiguous(named) = &selected.selection else { panic!("ambiguous: {:?}", selected.selection) };
    assert_eq!(named, &["acme".to_string(), "bolt".to_string()]);
    let [diagnostic] = selected.diagnostics.as_slice() else { panic!("one warning: {:#?}", selected.diagnostics) };
    assert_eq!(diagnostic.code, HOST_AMBIGUOUS_CODE);
    assert_eq!(diagnostic.file, fixture.path("work/doc.md"));
    assert!(diagnostic.message.contains("`acme`") && diagnostic.message.contains("`bolt`"), "{}", diagnostic.message);

    let schemas = DarkmatterSchemas::new(context).with_registered_hosts(hosts);
    assert!(schemas.effective_for(&markdown).unwrap().is_none(), "md analysis: no host layers");
}

// ── cache identity and inspection ───────────────────────────────────────────

#[test]
fn a_changed_host_revision_never_reuses_a_validator() {
    let fixture = Fixture::new();
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", WIDGET_DOCUMENT);
    let first = fixture.hosts(&[HOST]);
    let again = fixture.hosts(&[HOST]);
    // Only the namespace's description changes: the trigger payloads are
    // equal, the revision is not.
    let description = RUN_NAMESPACE.replace("The current pass.", "The pass, renamed.");
    fixture.write("host/schemas/run.yaml", &description);
    fixture.write_member("host", "schemas/globals.lifecycle.json", &lifecycle_artifact_for("acme", &[("schemas/run.yaml", &description)]));
    let changed = fixture.hosts(&[HOST]);
    assert_ne!(first.loaded().profiles[0].revision(), changed.loaded().profiles[0].revision());

    let validator_of = |hosts: &Arc<RegisteredHosts>| {
        let effective = effective_schema(&DarkmatterSchemas::new(context.clone()).with_registered_hosts(Arc::clone(hosts)), &markdown);
        let validator = effective.validator().expect("an object schema") as *const _ as usize;
        (effective.json_schema.clone(), validator)
    };
    let (first_schema, first_validator) = validator_of(&first);
    let (_, again_validator) = validator_of(&again);
    let (changed_schema, changed_validator) = validator_of(&changed);
    assert_eq!(first_schema, changed_schema, "the same merged schema");
    assert_eq!(first_validator, again_validator, "an unchanged revision reuses its validator");
    assert_ne!(first_validator, changed_validator, "a changed revision compiles its own");
}

#[test]
fn the_registered_trace_names_the_manifest_revision_and_each_trigger() {
    let fixture = Fixture::new();
    let hosts = fixture.hosts(&[HOST]);
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", "---\ngadget: name\n---\n");
    let schemas = DarkmatterSchemas::new(context).with_registered_hosts(Arc::clone(&hosts));

    let trace = schemas.registered_trace(&markdown).expect("acme identifies the document");
    assert_eq!(trace.origin.host, "acme");
    assert_eq!(trace.origin.manifest, fixture.manifest());
    assert_eq!(trace.origin.revision, hosts.loaded().profiles[0].revision());
    assert_eq!(trace.origin.triggers, [fixture.path("host/schemas/gadget.yaml")]);
    let matched: Vec<(String, bool)> =
        trace.triggers.iter().map(|trigger| (trigger.source.to_string(), trigger.matched)).collect();
    assert_eq!(matched.len(), 2);
    assert!(matched[0].1 && !matched[1].1, "gadget matched, widget did not: {matched:?}");
}

// ── composition ─────────────────────────────────────────────────────────────

fn compose(fixture: &Fixture, configure: impl FnOnce(ComposeOptions) -> ComposeOptions) -> Result<Markdown, MarkdownError> {
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", WIDGET_DOCUMENT);
    let options = configure(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(&fixture.path("work"), ""))
            .with_source_file(fixture.path("work/doc.md"))
            .with_trigger_schemas(true),
    );
    let request = ComposeRequest::with_context(options, context).expect("a request");
    markdown.compose_with(&request).map(|(composed, _)| composed)
}

#[test]
fn composition_applies_the_registered_schema_of_an_identified_document() {
    let fixture = Fixture::new();
    let hosts = fixture.hosts(&[HOST]);
    if let Err(error) = compose(&fixture, |options| options) {
        panic!("no registration: nothing judges `size`: {error:?}");
    }
    let error = compose(&fixture, |options| options.with_registered_hosts(Arc::clone(&hosts))).expect_err("rejected");
    let MarkdownError::SchemaValidationFailed { problems, .. } = &error else { panic!("a schema failure: {error:?}") };
    let paths: Vec<&str> = problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/widget/size"], "{problems:#?}");
}

/// An executable host's own (empty) globals, as Claudine supplies its own.
fn caller_globals() -> HostGlobals {
    let view = BindingView::builder(ScopeId::new("caller.host")).build().unwrap();
    HostGlobals::associate(Arc::new(view), []).unwrap()
}

#[test]
fn a_caller_with_its_own_host_globals_keeps_them_and_gets_no_registered_layer() {
    let fixture = Fixture::new();
    let hosts = fixture.hosts(&[HOST]);
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", WIDGET_DOCUMENT);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&fixture.path("work"), ""))
        .with_host_globals(caller_globals())
        .with_registered_hosts(Arc::clone(&hosts));
    assert!(matches!(options.host_selection_for(&markdown, &context).selection, HostSelection::Md));
    let composed = compose(&fixture, |options| options.with_host_globals(caller_globals()).with_registered_hosts(hosts));
    assert!(composed.is_ok(), "the registered schema never judges a caller's host run: {composed:?}");
}

// ── passivity ───────────────────────────────────────────────────────────────

/// Every file under `root` with its bytes, so a test can show nothing was
/// written, created, or removed.
fn tree(root: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
    }
    files
}

#[test]
fn loading_selecting_and_composing_with_a_registration_writes_nothing() {
    let fixture = Fixture::new();
    fixture.write("work/doc.md", WIDGET_DOCUMENT);
    let before = tree(&fixture.root);

    let hosts = fixture.hosts(&[HOST]);
    let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", WIDGET_DOCUMENT);
    let schemas = DarkmatterSchemas::new(context.clone()).with_registered_hosts(Arc::clone(&hosts));
    assert!(matches!(schemas.host_selection_for(&markdown).selection, HostSelection::Registered(_)));
    let _ = effective_schema(&schemas, &markdown);
    // No approval handler, no network host, and no executable host: a
    // registration supplies types only, so composition asks and fetches
    // nothing and installs no handler.
    let composed = compose(&fixture, |options| options.with_registered_hosts(Arc::clone(&hosts)));
    assert!(composed.is_err(), "the registered schema judged the document");

    assert_eq!(tree(&fixture.root), before, "registration, selection, and composition wrote nothing");
}
