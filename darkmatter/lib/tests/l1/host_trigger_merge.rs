//! The checked layer of a registered host's matched triggers
//! (`config::registration::merge`, through `DarkmatterSchemas::effective_for`):
//! equivalent declarations coalesce, a strictly narrower one wins when its
//! presence, null admission, defaults, descriptions, and constraints agree,
//! and anything else is a `dm.host.trigger_conflict` schema preparation
//! error naming the host, the property or root rule, and both triggers.
//! Every pair is compared, inherited properties included; only matched
//! triggers count; one root union may own the root; a higher layer refines
//! a prepared layer but never clears a conflict.

use std::path::PathBuf;
use std::sync::Arc;

use crate::host_fixtures::*;
use darkmatter::config::registration::merge::HOST_TRIGGER_CONFLICT_CODE;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::report_diagnostics::schema_error_diagnostics;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use simplified_schema::{SchemaError, SchemaOriginKind, SchemaSourceId};

const HOST: &str = "../host/darkmatter-host.yaml";

/// A trigger matching every document with a `team` string, declaring
/// `payload` (an indented `$schema` body).
fn trigger(payload: &str) -> String {
    format!("kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n{payload}")
}

/// The `acme` bundle plus `files` (bundle path, text).
fn bundle(files: &[(&str, &str)]) -> Bundle {
    files.iter().fold(Bundle::acme(), |bundle, (path, text)| bundle.with_file(path, text))
}

/// A fixture with `bundle` installed under `host/` and `document` written
/// at `work/doc.md`, analyzed in the plain (non-repository) context.
struct Case {
    fixture: Fixture,
    schemas: DarkmatterSchemas,
    markdown: Markdown,
}

impl Case {
    fn new(bundle: &Bundle, document: &str) -> Self {
        let fixture = Fixture::new();
        fixture.install_bundle("host", bundle);
        let hosts = fixture.hosts(&[HOST]);
        assert!(hosts.diagnostics().is_empty(), "the bundle loads: {:#?}", hosts.diagnostics());
        let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", document);
        let schemas = DarkmatterSchemas::new(context).with_registered_hosts(hosts);
        Self { fixture, schemas, markdown }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.fixture.path(&format!("host/{relative}"))
    }

    fn effective(&self) -> EffectiveSchema {
        self.schemas.effective_for(&self.markdown).expect("the schema assembles").expect("a schema applies")
    }

    /// The problem paths of the document's frontmatter.
    fn problems(&self) -> Vec<String> {
        let report = self.effective().validate(&darkmatter::markdown::schemas::frontmatter_as_json(&self.markdown));
        let mut paths: Vec<String> = report.problems.iter().map(|problem| problem.path.clone()).collect();
        paths.sort();
        paths
    }

    /// The conflict `effective_for` refuses with: (subject, first, second).
    fn conflict(&self) -> (Option<String>, PathBuf, PathBuf, String) {
        match self.schemas.effective_for(&self.markdown) {
            Err(SchemaError::HostTriggerConflict(conflict)) => {
                let simplified_schema::HostTriggerConflict { document, host, first, second, subject, reason } = *conflict;
                assert_eq!(host, "acme");
                assert_eq!(document, Some(self.fixture.path("work/doc.md")));
                (subject, first, second, reason)
            }
            Err(other) => panic!("a trigger conflict, not {other:?}"),
            Ok(_) => panic!("a trigger conflict, not a schema"),
        }
    }
}

/// `team: core` plus `rest`, as a document.
fn document(rest: &str) -> String {
    format!("---\nteam: core\n{rest}---\nBody.\n")
}

const A: &str = "schemas/a.yaml";
const B: &str = "schemas/b.yaml";
const C: &str = "schemas/c.yaml";

// ── coalescing and narrowing ─────────────────────────────────────────────────

#[test]
fn equivalent_declarations_coalesce_under_the_first_trigger() {
    let case = Case::new(&bundle(&[(A, &trigger("    agent: \"string -> who runs it\"\n")), (B, &trigger("    agent: \"string -> who runs it\"\n"))]), &document("agent: [1]\n"));
    let effective = case.effective();
    assert_eq!(case.problems(), ["/agent"]);
    let origin = effective.origins.get("agent").expect("an origin");
    assert_eq!(origin.kind, SchemaOriginKind::Trigger);
    assert_eq!(origin.source, Some(SchemaSourceId::File(case.path(A))), "the first bundle path represents the class");
}

#[test]
fn a_narrower_enum_wins_over_a_string_whose_other_rules_agree() {
    let bundle = bundle(&[(A, &trigger("    agent: string\n")), (B, &trigger("    agent: enum(claude, codex)\n"))]);
    let case = Case::new(&bundle, &document("agent: gemini\n"));
    assert_eq!(case.problems(), ["/agent"], "the enum judges");
    let origin = case.effective().origins.get("agent").cloned().expect("an origin");
    assert_eq!(origin.source, Some(SchemaSourceId::File(case.path(B))), "the winning declaration keeps its origin");

    let case = Case::new(&bundle, &document("agent: claude\n"));
    assert!(case.problems().is_empty());
}

#[test]
fn a_required_string_and_an_optional_enum_conflict_on_presence() {
    let case = Case::new(&bundle(&[(A, &trigger("    agent: string(required)\n")), (B, &trigger("    agent: enum(claude, codex)\n"))]), &document("agent: claude\n"));
    let (subject, first, second, reason) = case.conflict();
    assert_eq!(subject.as_deref(), Some("agent"));
    assert_eq!((first, second), (case.path(A), case.path(B)));
    assert!(reason.contains("required"), "{reason}");
}

#[test]
fn equivalent_types_that_differ_in_nullability_description_constraint_or_default_conflict() {
    // No `example(...)` case: a registered bundle cannot carry a `kind:
    // example` file (it is neither listable nor readable unlisted), so a
    // registered payload never declares one.
    for (a, b, said) in [
        ("    agent: string\n", "    agent: \"string | null\"\n", "null"),
        ("    agent: \"string -> one\"\n", "    agent: \"string -> two\"\n", "description"),
        ("    agent: string\n", "    agent: string(min(2))\n", "constraint"),
        ("    agent: string(default(a))\n", "    agent: string(default(b))\n", "default"),
    ] {
        let case = Case::new(&bundle(&[(A, &trigger(a)), (B, &trigger(b))]), &document(""));
        let (subject, _, _, reason) = case.conflict();
        assert_eq!(subject.as_deref(), Some("agent"), "{a} / {b}");
        assert!(reason.contains(said), "{a} / {b}: {reason}");
    }
}

#[test]
fn nested_declarations_are_walked() {
    let same = bundle(&[(A, &trigger("    config:\n        mode: string\n")), (B, &trigger("    config:\n        mode: string\n"))]);
    let case = Case::new(&same, &document("config:\n  mode: [1]\n"));
    assert_eq!(case.problems(), ["/config/mode"]);

    let differ = bundle(&[(A, &trigger("    config:\n        mode: string\n")), (B, &trigger("    config:\n        mode: string(min(1))\n"))]);
    let case = Case::new(&differ, &document(""));
    assert_eq!(case.conflict().0.as_deref(), Some("config"));
}

#[test]
fn recursive_definitions_compare_by_structure_not_storage() {
    const TREE_A: &str = "kind: schema\ntypes:\n    tree:\n        name: string\n        children: tree[]\n";
    const TREE_B: &str = "kind: schema\ntypes:\n    node:\n        name: string\n        children: node[]\n";
    let bundle = bundle(&[
        (A, &trigger("    tree: tree@./tree-a.yaml\n")),
        (B, &trigger("    tree: node@./tree-b.yaml\n")),
        ("schemas/tree-a.yaml", TREE_A),
        ("schemas/tree-b.yaml", TREE_B),
    ]);
    let case = Case::new(&bundle, &document("tree:\n  name: root\n  children:\n    - name: [1]\n"));
    assert_eq!(case.problems(), ["/tree/children/0/name"], "the two recursive types coalesce");
}

#[test]
fn a_narrower_third_declaration_cannot_hide_an_incompatible_pair() {
    let bundle = bundle(&[
        (A, &trigger("    agent: \"string | number\"\n")),
        (B, &trigger("    agent: \"string | boolean\"\n")),
        (C, &trigger("    agent: string\n")),
    ]);
    let case = Case::new(&bundle, &document(""));
    let (subject, first, second, reason) = case.conflict();
    assert_eq!(subject.as_deref(), Some("agent"));
    assert_eq!((first, second), (case.path(A), case.path(B)), "{reason}");
    assert!(reason.contains("incomparable"), "{reason}");
}

#[test]
fn an_inherited_property_is_compared_as_declared() {
    let library = "kind: schema\ntypes:\n    base:\n        agent: string(required)\n";
    let bundle = bundle(&[
        (A, &trigger("    $extends: base@./base.yaml\n    mode: string\n")),
        (B, &trigger("    agent: string\n")),
        ("schemas/base.yaml", library),
    ]);
    let case = Case::new(&bundle, &document(""));
    assert_eq!(case.conflict().0.as_deref(), Some("agent"));
}

// ── which triggers count ────────────────────────────────────────────────────

#[test]
fn conflicting_triggers_that_never_match_together_are_no_conflict() {
    let crew = "kind: schema-trigger\nmatch:\n    crew: string(required)\n$schema:\n    agent: number\n";
    let bundle = bundle(&[(A, &trigger("    agent: string\n")), (B, crew)]);
    let team_only = Case::new(&bundle, &document("agent: [1]\n"));
    assert_eq!(team_only.problems(), ["/agent"], "only `a` matched");

    let both = Case::new(&bundle, &document("crew: night\n"));
    assert_eq!(both.conflict().0.as_deref(), Some("agent"));
}

#[test]
fn a_static_result_is_memoized_per_matched_set() {
    let case = Case::new(&bundle(&[(A, &trigger("    agent: string\n")), (B, &trigger("    agent: enum(x)\n"))]), &document(""));
    let selected = case.schemas.registered_selection_for(&case.markdown);
    let darkmatter::config::registration::HostSelection::Registered(profile) = &selected.selection else { panic!("acme") };
    let first = profile.checked_layer(&selected.matched).expect("checked");
    let again = profile.checked_layer(&selected.matched).expect("checked");
    assert!(Arc::ptr_eq(&first, &again));
    let pruned = first.payloads.iter().find(|part| profile.triggers()[part.trigger].bundle_path == A).expect("a");
    assert!(pruned.json_schema["properties"].get("agent").is_none(), "`a` lost `agent` to `b`");
}

// ── root rules and root unions ──────────────────────────────────────────────

/// A root union with arms told apart by `prompt` and `sections`.
const INLINE: &str = "    - prompt: string(required)\n      sections: null\n    - sections: object(required)\n      prompt: null\n";

#[test]
fn ordinary_properties_merge_into_each_arm_of_the_root_union_in_arm_order() {
    let bundle = bundle(&[(A, &trigger(INLINE)), (B, &trigger("    agent: string\n"))]);
    let case = Case::new(&bundle, &document("prompt: hi\nagent: [1]\n"));
    assert_eq!(case.problems().iter().filter(|path| path.as_str() == "/agent").count(), 1, "{:?}", case.problems());
    let effective = case.effective();
    let arms = effective.json_schema["anyOf"].as_array().expect("the union is the root").clone();
    assert_eq!(arms.len(), 2);
    assert!(arms[0]["properties"].get("prompt").is_some() && arms[1]["properties"].get("sections").is_some(), "arm order kept");
    assert!(arms.iter().all(|arm| arm["properties"].get("agent").is_some()), "agent in every arm");

    let valid = Case::new(&bundle, &document("sections:\n  intro: hi\nagent: me\n"));
    assert!(valid.problems().is_empty(), "{:?}", valid.problems());
    let neither = Case::new(&bundle, &document("agent: me\n"));
    assert!(!neither.problems().is_empty(), "one of `prompt` or `sections` is required");
}

#[test]
fn another_trigger_may_not_declare_a_key_that_defines_the_union() {
    let bundle = bundle(&[(A, &trigger(INLINE)), (B, &trigger("    prompt: string\n"))]);
    let case = Case::new(&bundle, &document(""));
    let (subject, first, second, _) = case.conflict();
    assert_eq!(subject.as_deref(), Some("prompt"));
    assert_eq!((first, second), (case.path(A), case.path(B)));
}

#[test]
fn a_second_root_union_or_root_rules_with_two_payloads_conflict_on_the_root() {
    let two = bundle(&[(A, &trigger(INLINE)), (B, &trigger("    - mode: string(required)\n    - kind: string(required)\n"))]);
    let case = Case::new(&two, &document(""));
    assert_eq!(case.conflict().0, None, "the root rules");

    let closed = bundle(&[(A, &trigger("    $constraints: { closed: true }\n    agent: string\n")), (B, &trigger("    mode: string\n"))]);
    let case = Case::new(&closed, &document(""));
    let (subject, first, _, reason) = case.conflict();
    assert_eq!((subject, first), (None, case.path(A)), "{reason}");

    // Alone, a payload keeps its root rules.
    let alone = bundle(&[(A, &trigger("    $constraints: { closed: true }\n    agent: string\n"))]);
    let case = Case::new(&alone, &document("extra: 1\n"));
    assert!(!case.problems().is_empty(), "the closed root refuses `extra`");
}

// ── higher layers ───────────────────────────────────────────────────────────

#[test]
fn a_document_override_applies_to_a_prepared_layer_but_never_clears_a_conflict() {
    let narrowing = bundle(&[(A, &trigger("    agent: string\n")), (B, &trigger("    agent: enum(claude)\n"))]);
    let case = Case::new(&narrowing, &document("$schema:\n  agent: string\nagent: codex\n"));
    assert!(case.problems().is_empty(), "the document's own `$schema` wins: {:?}", case.problems());

    let conflicting = bundle(&[(A, &trigger("    agent: string(required)\n")), (B, &trigger("    agent: enum(claude)\n"))]);
    let case = Case::new(&conflicting, &document("$schema:\n  agent: number\nagent: 3\n"));
    assert_eq!(case.conflict().0.as_deref(), Some("agent"), "a higher layer does not clear it");
}

#[test]
fn a_root_trigger_refines_a_prepared_union_layer() {
    let bundle = bundle(&[(A, &trigger(INLINE)), (B, &trigger("    agent: number\n"))]);
    let fixture = Fixture::new();
    fixture.install_bundle("host", &bundle);
    let hosts = fixture.hosts(&[HOST]);
    fixture.write("work/schemas/refine.yaml", "kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n    agent: string\n");
    let (markdown, context) = fixture.document(&fixture.repository(None), "work/doc.md", &document("prompt: hi\nagent: codex\n"));
    let schemas = DarkmatterSchemas::new(context).with_registered_hosts(hosts).with_trigger_discovery().expect("discovery");
    let effective = schemas.effective_for(&markdown).expect("assembles").expect("a schema");
    let report = effective.validate(&darkmatter::markdown::schemas::frontmatter_as_json(&markdown));
    assert!(report.valid, "the root trigger's `agent: string` wins in every arm: {:#?}", report.problems);
    assert!(effective.json_schema["anyOf"].as_array().is_some_and(|arms| arms.len() == 2));
}

#[test]
fn a_registered_template_stays_passive_and_is_listed_whole() {
    let template = "    agent: enum({{ load_data(\"./agents.yaml\", \"string[]\") }})\n";
    let bundle = bundle(&[(A, &trigger(template)), (B, &trigger("    mode: string\n"))]);
    // The document redeclares `agent`: a root trigger's slot under it would
    // be discarded, a registered one is checked whole.
    let case = Case::new(&bundle, &document("$schema:\n  agent: string\n"));
    let templates = case.schemas.pending_templates_for(&case.markdown);
    assert_eq!(templates.len(), 1, "the registered template is listed");
    assert!(
        matches!(case.schemas.effective_for(&case.markdown), Err(SchemaError::UnresolvedSlots(_))),
        "without a session nothing resolves"
    );
}

// ── the reported record ─────────────────────────────────────────────────────

#[test]
fn the_conflict_is_one_coded_error_on_the_document() {
    let case = Case::new(&bundle(&[(A, &trigger("    agent: number\n")), (B, &trigger("    agent: boolean\n"))]), &document(""));
    let error = case.schemas.effective_for(&case.markdown).err().expect("a conflict");
    let message = error.to_string();
    assert!(message.contains("acme") && message.contains("a.yaml") && message.contains("b.yaml") && message.contains("`agent`"), "{message}");
    let records = schema_error_diagnostics(&error, std::path::Path::new("unknown")).expect("coded");
    let [record] = records.as_slice() else { panic!("one record: {records:#?}") };
    assert_eq!(record.code, HOST_TRIGGER_CONFLICT_CODE);
    assert_eq!(record.source.as_ref().and_then(|source| source.file.clone()), Some(case.fixture.path("work/doc.md")));
}
