//! The checked layer of a registered host's matched triggers
//! (`config::registration::merge`, through `DarkmatterSchemas::effective_for`):
//! equivalent declarations coalesce, a strictly narrower one wins when its
//! presence, null admission, and default agree and its descriptions and
//! constraints agree alternative by alternative (element and list rules
//! each at their own level),
//! and anything else is a `dm.host.trigger_conflict` schema preparation
//! error naming the host, the property or root rule, and both triggers.
//! Every pair is compared, inherited properties included; only matched
//! triggers count; one root union may own the root; a higher layer refines
//! a prepared layer but never clears a conflict.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use crate::host_fixtures::*;
use darkmatter::config::registration::RegisteredHosts;
use darkmatter::config::registration::merge::HOST_TRIGGER_CONFLICT_CODE;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::report_diagnostics::schema_error_diagnostics;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use simplified_schema::{SchemaError, SchemaOriginKind, SchemaPhase, SchemaSourceId};

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
    fixture: Rc<Fixture>,
    hosts: Arc<RegisteredHosts>,
    schemas: DarkmatterSchemas,
    markdown: Markdown,
}

impl Case {
    fn new(bundle: &Bundle, document: &str) -> Self {
        let fixture = Fixture::new();
        fixture.install_bundle("host", bundle);
        let hosts = fixture.hosts(&[HOST]);
        assert!(hosts.diagnostics().is_empty(), "the bundle loads: {:#?}", hosts.diagnostics());
        Self::analyze(Rc::new(fixture), hosts, document)
    }

    /// `document` judged under this case's loaded bundle, with fresh schemas:
    /// loading the bundle is most of a case, so a helper judging several
    /// documents against one bundle loads it once, as one `md` run does.
    fn with_document(&self, document: &str) -> Self {
        Self::analyze(Rc::clone(&self.fixture), Arc::clone(&self.hosts), document)
    }

    fn analyze(fixture: Rc<Fixture>, hosts: Arc<RegisteredHosts>, document: &str) -> Self {
        let (markdown, context) = fixture.document(&fixture.context(), "work/doc.md", document);
        let schemas = DarkmatterSchemas::new(context).with_registered_hosts(Arc::clone(&hosts));
        Self { fixture, hosts, schemas, markdown }
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

    /// The problem paths of the document's frontmatter judged at `phase`.
    fn phase_problems(&self, phase: SchemaPhase) -> Vec<String> {
        let frontmatter = darkmatter::markdown::schemas::frontmatter_as_json(&self.markdown);
        let report = self.effective().validate_for_phase(&frontmatter, phase).expect("the phase projection assembles");
        report.problems.iter().map(|problem| problem.path.clone()).collect()
    }

    /// The undeclared keys a closed root refuses, and nothing else.
    fn unknown_keys(&self) -> Vec<String> {
        unknown_keys(&self.effective(), &self.markdown)
    }

    /// The conflict `effective_for` refuses with: (subject, first, second).
    fn conflict(&self) -> (Option<String>, PathBuf, PathBuf, String) {
        match self.schemas.effective_for(&self.markdown) {
            Err(SchemaError::HostTriggerConflict(conflict)) => {
                let simplified_schema::HostTriggerConflict { document, host, first, first_name, second, second_name, subject, reason } =
                    *conflict;
                assert_eq!(host, "acme");
                assert_eq!((first_name, second_name), (None, None), "an unnamed trigger has no label");
                assert_eq!(document, Some(self.fixture.path("work/doc.md")));
                (subject, first, second, reason)
            }
            Err(other) => panic!("a trigger conflict, not {other:?}"),
            Ok(_) => panic!("a trigger conflict, not a schema"),
        }
    }
}

/// The keys `effective` refuses as undeclared in `markdown`'s frontmatter;
/// panics on any other problem.
fn unknown_keys(effective: &EffectiveSchema, markdown: &Markdown) -> Vec<String> {
    let report = effective.validate(&darkmatter::markdown::schemas::frontmatter_as_json(markdown));
    report
        .problems
        .iter()
        .map(|problem| {
            assert_eq!(problem.code, simplified_schema::ValidationProblemCode::UnknownKey, "{problem:#?}");
            problem.offending_property.clone().expect("the refused key")
        })
        .collect()
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

// ── rules stay with their union alternative ────────────────────────────────

/// `review-model` as a scalar-or-list union, each alternative described.
const REVIEW_MODEL: &str = "    review-model:\n        - \"string -> single model\"\n        - \"string[] -> model choices\"\n";
/// A narrower `review-model` whose descriptions stayed on their alternatives.
const NARROWED: &str = "    review-model:\n        - \"enum(claude, codex) -> single model\"\n        - \"string[] -> model choices\"\n";
/// [`NARROWED`] with the two descriptions swapped between alternatives.
const SWAPPED: &str = "    review-model:\n        - \"enum(claude, codex) -> model choices\"\n        - \"string[] -> single model\"\n";

/// `payload`'s top-level declarations repeated in both arms of [`INLINE`]'s
/// root union.
fn in_both_arms(payload: &str) -> String {
    let body: String = payload.lines().map(|line| format!("  {line}\n")).collect();
    format!("    - prompt: string(required)\n      sections: null\n{body}    - sections: object(required)\n      prompt: null\n{body}")
}

/// Expects `bundle` to conflict on `review-model` because `said`.
fn conflicts_on_review_model(bundle: &Bundle, said: &str) {
    let case = Case::new(bundle, &document("prompt: hi\n"));
    let (subject, first, second, reason) = case.conflict();
    assert_eq!(subject.as_deref(), Some("review-model"), "{reason}");
    assert_eq!((first, second), (case.path(A), case.path(B)), "{reason}");
    assert!(reason.contains(said), "{reason}");
}

#[test]
fn a_narrowing_that_keeps_each_description_on_its_alternative_wins() {
    for narrowed in [NARROWED.to_string(), in_both_arms(NARROWED)] {
        let bundle = bundle(&[(A, &trigger(REVIEW_MODEL)), (B, &trigger(&narrowed))]);
        let case = Case::new(&bundle, &document("prompt: hi\nreview-model: gemini\n"));
        assert_eq!(case.problems().iter().filter(|path| path.as_str() == "/review-model").count(), 1, "the enum judges: {narrowed}");
        let case = Case::new(&bundle, &document("prompt: hi\nreview-model: [gemini, codex]\n"));
        assert!(case.problems().is_empty(), "{narrowed}: {:?}", case.problems());
    }
}

#[test]
fn descriptions_swapped_between_alternatives_conflict_between_object_payloads() {
    conflicts_on_review_model(&bundle(&[(A, &trigger(REVIEW_MODEL)), (B, &trigger(SWAPPED))]), "descriptions differ at alternative 1");
}

#[test]
fn descriptions_swapped_between_alternatives_conflict_against_every_arm_of_a_root_union() {
    conflicts_on_review_model(&bundle(&[(A, &trigger(REVIEW_MODEL)), (B, &trigger(&in_both_arms(SWAPPED)))]), "descriptions differ");
}

#[test]
fn a_rule_moved_from_the_scalar_to_the_list_elements_conflicts() {
    let wide = "    review-model:\n        - \"string(no-coerce) -> single model\"\n        - \"string[] -> model choices\"\n";
    let moved = "    review-model:\n        - \"enum(claude, codex) -> single model\"\n        - \"string(no-coerce)[] -> model choices\"\n";
    conflicts_on_review_model(&bundle(&[(A, &trigger(wide)), (B, &trigger(moved))]), "constraints differ at alternative 1");
    conflicts_on_review_model(&bundle(&[(A, &trigger(wide)), (B, &trigger(&in_both_arms(moved)))]), "constraints differ");
}

#[test]
fn element_and_list_rules_are_compared_at_their_own_level() {
    let wide = "    review-model:\n        - string\n        - \"string(min(1))[]\"\n";
    let moved = "    review-model:\n        - \"enum(claude, codex)\"\n        - \"string[](min(1))\"\n";
    conflicts_on_review_model(&bundle(&[(A, &trigger(wide)), (B, &trigger(moved))]), "constraints differ at alternative 2");
}

#[test]
fn an_element_default_is_not_the_property_default() {
    let wide = "    review-model:\n        - \"string(default(claude))\"\n        - \"string[]\"\n";
    let moved = "    review-model:\n        - \"enum(claude, codex)\"\n        - \"string(default(claude))[]\"\n";
    conflicts_on_review_model(&bundle(&[(A, &trigger(wide)), (B, &trigger(moved))]), "defaults differ");
}

#[test]
fn a_nested_rule_changed_inside_an_equivalent_alternative_of_a_narrowing_conflicts() {
    let wide = "    review-model:\n        - string\n        - mode: string\n";
    let changed = "    review-model:\n        - \"enum(claude, codex)\"\n        - mode: string(min(1))\n";
    conflicts_on_review_model(&bundle(&[(A, &trigger(wide)), (B, &trigger(changed))]), "nested declaration at alternative 2");
}

#[test]
fn an_alternative_left_out_by_the_narrower_declaration_takes_its_rules_with_it() {
    let bundle = bundle(&[(A, &trigger("    review-model:\n        - \"string -> single model\"\n        - \"number(min(1)) -> a count\"\n")), (B, &trigger("    review-model: \"enum(claude, codex) -> single model\"\n"))]);
    let case = Case::new(&bundle, &document("review-model: 3\n"));
    assert_eq!(case.problems(), ["/review-model"], "the enum alone judges");
}

#[test]
fn a_named_union_type_is_compared_by_its_alternatives() {
    let library = "kind: schema\ntypes:\n    model:\n        - \"string -> single model\"\n        - \"string[] -> model choices\"\n";
    let narrowed = bundle(&[(A, &trigger("    review-model: model@./model.yaml\n")), (B, &trigger(NARROWED)), ("schemas/model.yaml", library)]);
    let case = Case::new(&narrowed, &document("review-model: gemini\n"));
    assert_eq!(case.problems(), ["/review-model"], "the named union narrows alternative by alternative");

    let swapped = bundle(&[(A, &trigger("    review-model: model@./model.yaml\n")), (B, &trigger(SWAPPED)), ("schemas/model.yaml", library)]);
    conflicts_on_review_model(&swapped, "descriptions differ");
}

// ── a list's items are alternatives too ────────────────────────────────────

/// A trigger like [`trigger`] that also declares private `types`.
fn typed_trigger(types: &str, payload: &str) -> String {
    format!("kind: schema-trigger\nmatch:\n    team: string(required)\ntypes:\n{types}$schema:\n{payload}")
}

/// `review-model: "{type}"`, as a payload body.
fn review_model(ty: &str) -> String {
    format!("    review-model: \"{ty}\"\n")
}

/// Every `review-model` property schema in `schema`, in each root arm.
fn review_model_schemas(schema: &serde_json::Value) -> Vec<serde_json::Value> {
    let arms = schema.get("anyOf").and_then(serde_json::Value::as_array).cloned().unwrap_or_else(|| vec![schema.clone()]);
    arms.iter().filter_map(|arm| arm.pointer("/properties/review-model").cloned()).collect()
}

/// For three payload forms (two objects, and either one the same key in
/// both arms of a root union) and both declaration orders of `wide` and
/// `narrow` (trigger files): the layer prepares, every arm's `review-model`
/// is `narrow`'s alone, `accepted` (frontmatter) is valid and `rejected`,
/// if any, is refused at `review-model`. (Validation converts a number or
/// boolean item to a string unless `no-coerce` says otherwise, so a type
/// narrowing to strings alone shows in the schema.)
fn narrows(wide: &str, narrow: &str, accepted: &str, rejected: Option<&str>) {
    let alone = Case::new(&bundle(&[(B, narrow)]), &document(&format!("prompt: hi\n{accepted}")));
    let [expected] = review_model_schemas(&alone.effective().json_schema).try_into().expect("one declaration");
    let object: fn(&str) -> String = str::to_string;
    for (wide_form, narrow_form) in [(object, object), (in_both_arms as fn(&str) -> String, object), (object, in_both_arms)] {
        let (wide, narrow) = (form_of(wide, wide_form), form_of(narrow, narrow_form));
        for files in [[(A, wide.as_str()), (B, narrow.as_str())], [(A, narrow.as_str()), (B, wide.as_str())]] {
            let bundle = bundle(&files);
            let case = Case::new(&bundle, &document(&format!("prompt: hi\n{accepted}")));
            let Ok(Some(effective)) = case.schemas.effective_for(&case.markdown) else {
                panic!("{wide} narrows to {narrow}: {:?}", case.schemas.effective_for(&case.markdown).err())
            };
            let kept = review_model_schemas(&effective.json_schema);
            assert!(!kept.is_empty() && kept.iter().all(|schema| *schema == expected), "the narrow declaration is kept whole: {kept:#?}");
            let winner = if files[0].1 == narrow { A } else { B };
            assert_eq!(effective.origins.get("review-model").and_then(|origin| origin.source.clone()), Some(SchemaSourceId::File(case.path(winner))));
            assert_eq!(case.problems(), Vec::<String>::new(), "{narrow} accepts {accepted}");
            let Some(rejected) = rejected else { continue };
            let case = case.with_document(&document(&format!("prompt: hi\n{rejected}")));
            assert!(case.problems().iter().any(|path| path.starts_with("/review-model")), "{narrow} refuses {rejected}: {:?}", case.problems());
        }
    }
}

/// `text` (a trigger file) with its `$schema` body moved into both arms of
/// the root union when `form` is [`in_both_arms`].
fn form_of(text: &str, form: fn(&str) -> String) -> String {
    let (head, body) = text.split_once("$schema:\n").expect("a payload");
    format!("{head}$schema:\n{}", form(body))
}

#[test]
fn a_list_of_one_item_alternative_narrows_a_list_of_a_grouped_union() {
    narrows(
        &trigger(&review_model("(string | number)[]")),
        &trigger(&review_model("string[]")),
        "review-model: [claude]\n",
        None,
    );
}

#[test]
fn list_item_types_narrow_inside_each_covering_list() {
    for wide in ["string[]", "(string | number)[]", "string[] | number[]"] {
        narrows(
            &trigger(&review_model(wide)),
            &trigger(&review_model("enum(claude, codex)[]")),
            "review-model: [claude]\n",
            Some("review-model: [gemini]\n"),
        );
    }
    narrows(
        &trigger(&review_model("(string | number | boolean)[]")),
        &trigger(&review_model("(enum(claude, codex) | number)[]")),
        "review-model: [claude, 3]\n",
        Some("review-model: [gemini, 3]\n"),
    );
    narrows(
        &typed_trigger("    ModelIds: [string, number]\n", "    review-model: ModelIds[]\n    spare: ModelIds[]\n"),
        &trigger(&review_model("enum(claude, codex)[]")),
        "review-model: [codex]\n",
        Some("review-model: [claude, 3]\n"),
    );
    narrows(
        &stored("    Models: \"(string | number)[]\"\n", "Models"),
        &stored("    Models: \"enum(claude, codex)[]\"\n", "Models"),
        "review-model: [codex]\n",
        Some("review-model: [gemini]\n"),
    );
}

#[test]
fn homogeneous_list_alternatives_narrow_a_mixed_list() {
    narrows(
        &trigger(&review_model("(string(no-coerce) | number)[]")),
        &trigger(&review_model("string(no-coerce)[] | number[]")),
        "review-model: [claude]\n",
        Some("review-model: [claude, 3]\n"),
    );
}

#[test]
fn list_item_narrowing_requires_agreement_at_both_rule_levels() {
    let wide = trigger(&review_model("(string(min(2)) | number)[](min(1)) -> model choices"));
    narrows(
        &wide,
        &trigger(&review_model("alphanumeric(min(2))[](min(1)) -> model choices")),
        "review-model: [claude]\n",
        Some("review-model: [c]\n"),
    );
    for (ty, reason) in [
        ("alphanumeric(min(3))[](min(1)) -> model choices", "element constraints differ"),
        ("alphanumeric(min(2))[] -> model choices", "list constraints differ"),
    ] {
        let narrow = trigger(&review_model(ty));
        for pair in [[(A, wide.as_str()), (B, narrow.as_str())], [(A, narrow.as_str()), (B, wide.as_str())]] {
            conflicts_on_review_model(&bundle(&pair), reason);
        }
    }
}

#[test]
fn a_root_union_keeps_its_other_fields_when_a_shared_list_narrows() {
    let bundle = bundle(&[(A, &trigger(&in_both_arms(&review_model("(string | number)[]")))), (B, &trigger(&review_model("string[]")))]);
    let neither = Case::new(&bundle, &document("review-model: [claude]\n"));
    assert!(!neither.problems().is_empty(), "the union still requires one of its arms");
    let sections = Case::new(&bundle, &document("sections: {}\nreview-model: [claude]\n"));
    assert_eq!(sections.problems(), Vec::<String>::new());
}

#[test]
fn grouped_and_named_item_unions_are_compared_by_their_item_alternatives() {
    narrows(
        &trigger(&review_model("(string | number | boolean)[]")),
        &trigger(&review_model("(string | number)[]")),
        "review-model: [claude, 1]\n",
        None,
    );
    narrows(
        // A second use makes `ModelIds` a stored definition, reached by reference.
        &typed_trigger("    ModelIds: [string, number]\n", &format!("{}    spare: ModelIds[]\n", review_model("ModelIds[]"))),
        &trigger(&review_model("string[]")),
        "review-model: [claude]\n",
        None,
    );
}

#[test]
fn list_and_item_rules_kept_at_their_own_level_narrow() {
    narrows(
        &trigger(&review_model("(string | number)[](min(1))")),
        &trigger(&review_model("string[](min(1))")),
        "review-model: [claude]\n",
        Some("review-model: []\n"),
    );
    narrows(
        &trigger(&review_model("(string(no-coerce) | number)[]")),
        &trigger(&review_model("string(no-coerce)[]")),
        "review-model: [claude]\n",
        Some("review-model: [1]\n"),
    );
}

#[test]
fn a_list_rule_moved_to_the_items_or_a_dropped_item_rule_conflicts() {
    for (wide, narrow, said) in [
        ("(string | number)[](min(1))", "string(min(1))[]", "constraints differ"),
        ("(string(no-coerce) | number)[]", "string[]", "element constraints differ"),
    ] {
        for narrow in [review_model(narrow), in_both_arms(&review_model(narrow))] {
            conflicts_on_review_model(&bundle(&[(A, &trigger(&review_model(wide))), (B, &trigger(&narrow))]), said);
        }
    }
}

#[test]
fn equivalent_and_whole_list_controls_keep_their_results() {
    for (wide, narrow, accepted) in [
        ("string[]", "string[]", "review-model: [claude]\n"),
        ("(string | number)[]", "(string | number)[]", "review-model: [claude, 1]\n"),
        ("string[] | number[]", "string[]", "review-model: [claude]\n"),
        ("string | number", "enum(claude, codex)", "review-model: claude\n"),
    ] {
        for narrow in [review_model(narrow), in_both_arms(&review_model(narrow))] {
            let case = Case::new(&bundle(&[(A, &trigger(&review_model(wide))), (B, &trigger(&narrow))]), &document(&format!("prompt: hi\n{accepted}")));
            assert_eq!(case.problems(), Vec::<String>::new(), "{wide} → {narrow}");
        }
    }
    let scalar = bundle(&[(A, &trigger(&review_model("string | number"))), (B, &trigger(&review_model("enum(claude, codex)")))]);
    let case = Case::new(&scalar, &document("review-model: [claude]\n"));
    assert_eq!(case.problems(), ["/review-model"], "the enum refuses a list as a value error");
}

#[test]
fn object_items_and_list_aliases_narrow_by_their_item_alternatives() {
    narrows(
        &trigger(&review_model("(string | { name: string })[]")),
        &trigger(&review_model("{ name: string }[]")),
        "review-model: [{ name: claude }]\n",
        Some("review-model: [claude]\n"),
    );
    narrows(
        &typed_trigger("    Model: \"{ name: string }\"\n", &format!("{}    spare: Model\n", review_model("(string | Model)[]"))),
        &typed_trigger("    Model: \"{ name: string }\"\n", &review_model("Model[]")),
        "review-model: [{ name: claude }]\n",
        Some("review-model: [claude]\n"),
    );
    // Structured bodies used twice are stored, so these stay references:
    // a list of a union definition, and a definition that is a list.
    narrows(
        &typed_trigger("    Ids: [string, \"{ id: string }\"]\n", &format!("{}    spare: Ids[]\n", review_model("Ids[]"))),
        &trigger(&review_model("{ id: string }[]")),
        "review-model: [{ id: claude }]\n",
        Some("review-model: [claude]\n"),
    );
    narrows(
        &typed_trigger("    Models: \"(string | { id: string })[]\"\n", "    review-model: Models\n    spare: Models\n"),
        &trigger(&review_model("{ id: string }[]")),
        "review-model: [{ id: claude }]\n",
        Some("review-model: [claude]\n"),
    );
}

/// A mixed list is no single homogeneous list, so the narrowing is refused,
/// although each homogeneous list fits the grouped one.
#[test]
fn a_list_of_a_union_never_narrows_a_union_of_whole_lists() {
    let wide = "    review-model:\n        - \"string[]\"\n        - \"number[]\"\n        - boolean\n";
    for narrow in [review_model("(string | number)[]"), in_both_arms(&review_model("(string | number)[]"))] {
        conflicts_on_review_model(&bundle(&[(A, &trigger(wide)), (B, &trigger(&narrow))]), "incomparable");
    }
}

#[test]
fn a_narrowed_list_refuses_the_mixed_list_the_wider_trigger_accepts() {
    let mixed = document("review-model: [claude, 1]\n");
    let wide = trigger(&review_model("(string(no-coerce) | number)[]"));
    let alone = Case::new(&bundle(&[(A, &wide)]), &mixed);
    assert_eq!(alone.problems(), Vec::<String>::new(), "the wider trigger alone accepts a mixed list");
    let narrowed = Case::new(&bundle(&[(A, &wide), (B, &trigger(&review_model("string(no-coerce)[]")))]), &mixed);
    assert_eq!(narrowed.problems(), ["/review-model/1"], "the narrowed layer prepares and refuses the number");
}

/// A constant template supplying either side of a list narrowing prepares
/// once resolved, and stays pending in the passive trace.
#[test]
fn a_template_supplying_either_side_of_a_list_narrowing_prepares_once_resolved() {
    use darkmatter::markdown::compose::DataRuntime;
    use darkmatter::markdown::schemas::slots::SlotSession;
    let constant = |ty: &str| review_model(&format!("{{{{ '{ty}' }}}}"));
    let pairs = [
        (constant("(string(no-coerce) | number)[]"), review_model("string(no-coerce)[]")),
        (review_model("(string(no-coerce) | number)[]"), constant("string(no-coerce)[]")),
        (constant("(string | number)[]"), review_model("enum(claude, codex)[]")),
        (review_model("(string | number)[]"), constant("enum(claude, codex)[]")),
    ];
    for (wide, narrow) in pairs {
        for narrow in [narrow.clone(), in_both_arms(&narrow)] {
            let bundle = bundle(&[(A, &trigger(&wide)), (B, &trigger(&narrow))]);
            for (frontmatter, problems) in [("review-model: [claude]\n", 0), ("review-model: [claude, 1]\n", 1)] {
                let case = Case::new(&bundle, &document(&format!("prompt: hi\n{frontmatter}")));
                assert!(matches!(case.schemas.effective_for(&case.markdown), Err(SchemaError::UnresolvedSlots(_))), "passive without a session");
                let trace = case.schemas.registered_trace(&case.markdown).expect("identified");
                assert_eq!(trace.conflict, None, "{trace:#?}");
                assert!(trace.triggers.iter().any(|entry| entry.matched && entry.pending), "{trace:#?}");
                let schemas = case.schemas.clone().with_slot_session(Arc::new(SlotSession::new(DataRuntime::new())));
                let effective = schemas.effective_for(&case.markdown).expect("the resolved template narrows").expect("a schema");
                let report = effective.validate(&darkmatter::markdown::schemas::frontmatter_as_json(&case.markdown));
                assert_eq!(report.problems.len(), problems, "{wide} / {narrow} / {frontmatter}: {:#?}", report.problems);
            }
        }
    }
}

// ── how a named type is stored never decides a comparison ────────────────────

/// `Models`, the wide list, as a private type.
const MODELS: &str = "    Models: \"(string | { id: string })[]\"\n";

/// A trigger declaring `review-model: "{ty}"` beside `spare: {ty}`, so the
/// named type `ty` uses is stored and reached by reference.
fn stored(types: &str, ty: &str) -> String {
    let spare = ty.split([' ', '(']).next().expect("a type name");
    typed_trigger(types, &format!("{}    spare: {spare}\n", review_model(ty)))
}

/// Whether `text` (a trigger file) alone lowers `review-model` through a
/// `$defs` reference, which proves the fixture exercises a stored type.
fn is_stored(text: &str) -> bool {
    let alone = Case::new(&bundle(&[(A, text)]), &document("prompt: hi\n"));
    review_model_schemas(&alone.effective().json_schema).iter().all(|schema| schema.to_string().contains("$ref"))
}

/// For three payload forms and both file orders of `a` and `b`, the layer
/// prepares, `accepted` is valid, `rejected` is refused at `review-model`,
/// and, between two object payloads, the lexically first bundle path
/// represents the property.
fn coalesces(a: &str, b: &str, accepted: &str, rejected: &str) {
    let object: fn(&str) -> String = str::to_string;
    for (a_form, b_form) in [(object, object), (in_both_arms as fn(&str) -> String, object), (object, in_both_arms)] {
        let (a, b) = (form_of(a, a_form), form_of(b, b_form));
        for files in [[(A, a.as_str()), (B, b.as_str())], [(A, b.as_str()), (B, a.as_str())]] {
            let bundle = bundle(&files);
            let case = Case::new(&bundle, &document(&format!("prompt: hi\n{accepted}")));
            let Ok(Some(effective)) = case.schemas.effective_for(&case.markdown) else {
                panic!("{} coalesces with {}: {:?}", files[0].1, files[1].1, case.schemas.effective_for(&case.markdown).err())
            };
            assert_eq!(case.problems(), Vec::<String>::new(), "{accepted}");
            if a_form as usize == object as usize && b_form as usize == object as usize {
                let origin = effective.origins.get("review-model").expect("an origin");
                assert_eq!(origin.source, Some(SchemaSourceId::File(case.path(A))), "the first bundle path represents the class");
            }
            let case = case.with_document(&document(&format!("prompt: hi\n{rejected}")));
            assert!(case.problems().iter().any(|path| path.starts_with("/review-model")), "{rejected}: {:?}", case.problems());
        }
    }
}

#[test]
fn a_described_stored_list_narrows_like_its_inline_body() {
    let wide = stored(MODELS, "Models -> Models reviewed");
    assert!(is_stored(&wide));
    narrows(&wide, &trigger(&review_model("{ id: string }[] -> Models reviewed")), "review-model: [{ id: claude }]\n", Some("review-model: [claude]\n"));

    // Each root-union arm stores a private type under its own `$defs` key,
    // so the kept stored declaration is recognized by its reference and
    // description rather than compared byte for byte.
    let narrow = stored("    Ids: \"{ id: string }[]\"\n", "Ids -> Models reviewed");
    assert!(is_stored(&narrow));
    let inline_wide = trigger(&review_model("(string | { id: string })[] -> Models reviewed"));
    let object: fn(&str) -> String = str::to_string;
    for (wide_form, narrow_form) in [(object, object), (in_both_arms as fn(&str) -> String, object), (object, in_both_arms)] {
        let (wide, narrow) = (form_of(&inline_wide, wide_form), form_of(&narrow, narrow_form));
        for files in [[(A, wide.as_str()), (B, narrow.as_str())], [(A, narrow.as_str()), (B, wide.as_str())]] {
            let case = Case::new(&bundle(&files), &document("prompt: hi\nreview-model: [{ id: claude }]\n"));
            let Ok(Some(effective)) = case.schemas.effective_for(&case.markdown) else { panic!("{narrow} narrows {wide}") };
            let kept = review_model_schemas(&effective.json_schema);
            assert!(
                !kept.is_empty() && kept.iter().all(|schema| schema["description"] == "Models reviewed" && schema.to_string().contains("$ref")),
                "the stored narrow declaration is kept as written: {kept:#?}"
            );
            assert_eq!(case.problems(), Vec::<String>::new());
            let case = Case::new(&bundle(&files), &document("prompt: hi\nreview-model: [claude]\n"));
            assert!(case.problems().iter().any(|path| path.starts_with("/review-model")), "{:?}", case.problems());
        }
    }

    for narrow in [review_model("{ id: string }[] -> other"), in_both_arms(&review_model("{ id: string }[] -> other"))] {
        conflicts_on_review_model(&bundle(&[(A, &wide), (B, &trigger(&narrow))]), "descriptions differ");
    }
}

#[test]
fn a_list_rule_at_a_stored_list_use_is_the_lists_rule() {
    let wide = stored(MODELS, "Models(min(1))");
    assert!(is_stored(&wide));
    let narrow = trigger(&review_model("{ id: string }[](min(1))"));
    narrows(&wide, &narrow, "review-model: [{ id: claude }]\n", Some("review-model: []\n"));
    narrows(
        &stored("    Models: \"(string | { id: string })[](min(1))\"\n", "Models"),
        &narrow,
        "review-model: [{ id: claude }]\n",
        Some("review-model: []\n"),
    );
    for narrow in [review_model("{ id: string }[]"), in_both_arms(&review_model("{ id: string }[]"))] {
        conflicts_on_review_model(&bundle(&[(A, &wide), (B, &trigger(&narrow))]), "list constraints differ");
    }
}

#[test]
fn a_stored_list_coalesces_with_its_exact_inline_body() {
    let (accepted, rejected) = ("review-model: [claude, { id: codex }]\n", "review-model: [{ id: [1] }]\n");
    let inline = trigger(&review_model("(string | { id: string })[]"));
    assert!(is_stored(&stored(MODELS, "Models")));
    coalesces(&stored(MODELS, "Models"), &inline, accepted, rejected);
    coalesces(&typed_trigger(MODELS, &review_model("Models")), &inline, accepted, rejected);
    coalesces(&stored(MODELS, "Models"), &stored("    Wide: \"(string | { id: string })[]\"\n", "Wide"), accepted, rejected);
}

#[test]
fn a_stored_union_coalesces_with_the_same_written_alternatives() {
    let models = "    Models: [string, \"{ id: string }\"]\n";
    assert!(is_stored(&stored(models, "Models")) && is_stored(&stored(models, "Models[]")));
    let written = "    review-model:\n        - string\n        - \"{ id: string }\"\n";
    coalesces(&stored(models, "Models"), &trigger(written), "review-model: { id: codex }\n", "review-model: { id: [1] }\n");
    coalesces(
        &stored(models, "Models[]"),
        &trigger(&review_model("(string | { id: string })[]")),
        "review-model: [claude, { id: codex }]\n",
        "review-model: [{ id: [1] }]\n",
    );
    narrows(&stored(models, "Models"), &trigger(&review_model("{ id: string }")), "review-model: { id: claude }\n", Some("review-model: claude\n"));
    narrows(&stored(models, "Models[]"), &trigger(&review_model("{ id: string }[]")), "review-model: [{ id: claude }]\n", Some("review-model: [claude]\n"));
}

#[test]
fn a_nested_stored_list_coalesces_with_its_inline_body() {
    let nested = typed_trigger(MODELS, "    review-model:\n        models: Models\n    spare: Models\n");
    let inline = trigger("    review-model:\n        models: \"(string | { id: string })[]\"\n");
    coalesces(&nested, &inline, "review-model:\n  models: [claude]\n", "review-model:\n  models: [{ id: [1] }]\n");

    let chain = format!("{MODELS}    Alias: Models\n");
    let chained = typed_trigger(&chain, "    review-model: Alias\n    spare: Alias\n    other: Models\n");
    narrows(&chained, &trigger(&review_model("{ id: string }[]")), "review-model: [{ id: claude }]\n", Some("review-model: [claude]\n"));
    coalesces(
        &stored("    Model: \"{ id: string }\"\n", "Model[]"),
        &trigger(&review_model("{ id: string }[]")),
        "review-model: [{ id: claude }]\n",
        "review-model: [claude]\n",
    );
}

/// A stored type used as a dictionary value or a tuple element compares
/// like its inline body.
#[test]
fn a_stored_type_in_a_dictionary_value_or_tuple_element_coalesces_with_its_inline_body() {
    let dictionary = typed_trigger(MODELS, "    review-model:\n        <string>: Models\n    spare: Models\n");
    let inline = trigger("    review-model:\n        <string>: \"(string | { id: string })[]\"\n");
    coalesces(&dictionary, &inline, "review-model:\n  first: [claude]\n", "review-model:\n  first: [{ id: [1] }]\n");

    let pair = "    Pair: \"{ id: string }\"\n";
    let tuple = typed_trigger(pair, "    review-model: \"tuple([Pair, string])\"\n    spare: Pair\n");
    let inline = trigger("    review-model: \"tuple([{ id: string }, string])\"\n");
    coalesces(&tuple, &inline, "review-model: [{ id: claude }, x]\n", "review-model: [{ id: [1] }, x]\n");
}

#[test]
fn a_root_union_shares_a_key_one_arm_stores_and_another_writes_inline() {
    let union = typed_trigger(
        MODELS,
        "    - prompt: string(required)\n      sections: null\n      review-model: \"(string | { id: string })[]\"\n    - sections: object(required)\n      prompt: null\n      review-model: Models\n      spare: Models\n",
    );
    let object = trigger(&review_model("(string | { id: string })[]"));
    for files in [[(A, union.as_str()), (B, object.as_str())], [(A, object.as_str()), (B, union.as_str())]] {
        let case = Case::new(&bundle(&files), &document("prompt: hi\nreview-model: [claude]\n"));
        let result = case.schemas.effective_for(&case.markdown);
        assert!(matches!(result, Ok(Some(_))), "the key is shared: {:?}", result.err().map(|error| error.to_string()));
        assert_eq!(case.problems(), Vec::<String>::new());
        let case = Case::new(&bundle(&files), &document("prompt: hi\nreview-model: [{ id: [1] }]\n"));
        assert!(case.problems().iter().any(|path| path.starts_with("/review-model")), "{:?}", case.problems());
    }
}

/// A constant template supplying the narrower described list against a
/// stored wider one prepares once resolved, and stays pending in the
/// passive trace.
#[test]
fn a_template_narrowing_a_described_stored_list_prepares_once_resolved() {
    use darkmatter::markdown::compose::DataRuntime;
    use darkmatter::markdown::schemas::slots::SlotSession;
    let wide = stored(MODELS, "Models -> Models reviewed");
    let narrow = trigger(&review_model("{{ '{ id: string }[] -> Models reviewed' }}"));
    let pairs = [(wide.clone(), narrow.clone()), (wide.clone(), form_of(&narrow, in_both_arms)), (form_of(&wide, in_both_arms), narrow)];
    for (wide, narrow) in pairs {
        let bundle = bundle(&[(A, &wide), (B, &narrow)]);
        for (frontmatter, refused) in [("review-model: [{ id: claude }]\n", false), ("review-model: [claude]\n", true)] {
            let case = Case::new(&bundle, &document(&format!("prompt: hi\n{frontmatter}")));
            assert!(matches!(case.schemas.effective_for(&case.markdown), Err(SchemaError::UnresolvedSlots(_))), "passive without a session");
            let trace = case.schemas.registered_trace(&case.markdown).expect("identified");
            assert_eq!(trace.conflict, None, "{trace:#?}");
            assert!(trace.triggers.iter().any(|entry| entry.matched && entry.pending), "{trace:#?}");
            let schemas = case.schemas.clone().with_slot_session(Arc::new(SlotSession::new(DataRuntime::new())));
            let effective = schemas.effective_for(&case.markdown).expect("the resolved template narrows").expect("a schema");
            let report = effective.validate(&darkmatter::markdown::schemas::frontmatter_as_json(&case.markdown));
            assert_eq!(!report.problems.is_empty(), refused, "{wide} / {narrow} / {frontmatter}: {:#?}", report.problems);
        }
    }
}

/// Storage never merges a mixed list into separate homogeneous lists.
#[test]
fn a_stored_mixed_list_never_narrows_a_union_of_whole_lists() {
    let wide = "    review-model:\n        - \"string[]\"\n        - \"{ id: string }[]\"\n        - boolean\n";
    for narrow in [stored(MODELS, "Models"), form_of(&stored(MODELS, "Models"), in_both_arms)] {
        conflicts_on_review_model(&bundle(&[(A, &trigger(wide)), (B, &narrow)]), "incomparable");
    }
}

// ── a recursive reference against its inline unfolding ──────────────────────

/// `Node`, a recursive tree, as a private type.
const NODE: &str = "    Node: \"{ name: string, children: Node[] }\"\n";
/// `Node`'s body written inline: one unfolding of the reference.
const TREE: &str = "{ name: string, children: Node[] }";
/// A tree two levels deep, which every tree declaration accepts.
const GOOD_TREE: &str = "{ name: root, children: [{ name: leaf, children: [{ name: deep }] }] }";
/// [`GOOD_TREE`] with a list for the deepest child's name.
const BAD_TREE: &str = "{ name: root, children: [{ name: leaf, children: [{ name: [1] }] }] }";

/// `review-model: {value}` as frontmatter.
fn tree_value(value: &str) -> String {
    format!("review-model: {value}\n")
}

#[test]
fn a_recursive_reference_coalesces_with_its_inline_unfolding() {
    let (named, inline) = (typed_trigger(NODE, &review_model("Node")), typed_trigger(NODE, &review_model(TREE)));
    coalesces(&named, &inline, &tree_value(GOOD_TREE), &tree_value(BAD_TREE));

    let library = "kind: schema\ntypes:\n    Node: \"{ name: string, children: Node[] }\"\n";
    let imported = trigger(&review_model("Node@./lib.yaml"));
    let written = trigger(&review_model("{ name: string, children: Node[]@./lib.yaml }"));
    let object: fn(&str) -> String = str::to_string;
    for (a_form, b_form) in [(object, object), (in_both_arms as fn(&str) -> String, object), (object, in_both_arms)] {
        let (a, b) = (form_of(&imported, a_form), form_of(&written, b_form));
        for files in [[(A, a.as_str()), (B, b.as_str())], [(A, b.as_str()), (B, a.as_str())]] {
            let bundle = bundle(&[files[0], files[1], ("schemas/lib.yaml", library)]);
            let case = Case::new(&bundle, &document(&format!("prompt: hi\n{}", tree_value(GOOD_TREE))));
            let Ok(Some(effective)) = case.schemas.effective_for(&case.markdown) else {
                panic!("{} coalesces with {}: {:?}", files[0].1, files[1].1, case.schemas.effective_for(&case.markdown).err())
            };
            assert_eq!(case.problems(), Vec::<String>::new());
            if a_form as usize == object as usize && b_form as usize == object as usize {
                let origin = effective.origins.get("review-model").expect("an origin");
                assert_eq!(origin.source, Some(SchemaSourceId::File(case.path(A))), "the first bundle path represents the class");
            }
            let case = Case::new(&bundle, &document(&format!("prompt: hi\n{}", tree_value(BAD_TREE))));
            assert!(case.problems().iter().any(|path| path.starts_with("/review-model/children")), "{:?}", case.problems());
        }
    }
}

#[test]
fn a_recursive_reference_and_its_inline_unfolding_narrow_and_pair_by_alternative() {
    // `narrows`, except that each root-union arm stores `Node` under its
    // own `$defs` key, so the kept declaration is compared with that key
    // spelled as in the narrow trigger alone.
    let (wide, narrow) = (typed_trigger(NODE, &review_model("Node | string")), typed_trigger(NODE, &review_model(TREE)));
    let unkeyed = |schema: &serde_json::Value| {
        let text = schema.to_string();
        let mut out = String::new();
        let mut rest = text.as_str();
        while let Some(at) = rest.find("$defs/Node-") {
            out.push_str(&rest[..at + "$defs/Node".len()]);
            rest = rest[at + "$defs/Node-".len()..].trim_start_matches(|c: char| c.is_ascii_digit());
        }
        out + rest
    };
    let alone = Case::new(&bundle(&[(B, &narrow)]), &document("prompt: hi\n"));
    let [expected] = review_model_schemas(&alone.effective().json_schema).try_into().expect("one declaration");
    let object: fn(&str) -> String = str::to_string;
    for (wide_form, narrow_form) in [(object, object), (in_both_arms as fn(&str) -> String, object), (object, in_both_arms)] {
        let (wide, narrow) = (form_of(&wide, wide_form), form_of(&narrow, narrow_form));
        for files in [[(A, wide.as_str()), (B, narrow.as_str())], [(A, narrow.as_str()), (B, wide.as_str())]] {
            let case = Case::new(&bundle(&files), &document(&format!("prompt: hi\n{}", tree_value(GOOD_TREE))));
            let Ok(Some(effective)) = case.schemas.effective_for(&case.markdown) else {
                panic!("{wide} narrows to {narrow}: {:?}", case.schemas.effective_for(&case.markdown).err())
            };
            let kept = review_model_schemas(&effective.json_schema);
            assert!(!kept.is_empty() && kept.iter().all(|schema| unkeyed(schema) == unkeyed(&expected)), "the inline tree is kept whole: {kept:#?}");
            assert_eq!(case.problems(), Vec::<String>::new());
            for refused in ["review-model: claude\n".to_string(), tree_value(BAD_TREE)] {
                let case = case.with_document(&document(&format!("prompt: hi\n{refused}")));
                assert!(case.problems().iter().any(|path| path.starts_with("/review-model")), "{refused}: {:?}", case.problems());
            }
        }
    }
    coalesces(
        &typed_trigger(NODE, &review_model("Node | string")),
        &typed_trigger(NODE, &review_model(&format!("{TREE} | string"))),
        &tree_value(GOOD_TREE),
        &tree_value(BAD_TREE),
    );
}

#[test]
fn a_root_union_shares_a_recursive_key_one_arm_names_and_another_writes_inline() {
    let union = typed_trigger(
        NODE,
        &format!(
            "    - prompt: string(required)\n      sections: null\n      review-model: Node\n    - sections: object(required)\n      prompt: null\n      review-model: \"{TREE}\"\n"
        ),
    );
    let object = typed_trigger(NODE, &review_model("Node"));
    for files in [[(A, union.as_str()), (B, object.as_str())], [(A, object.as_str()), (B, union.as_str())]] {
        let case = Case::new(&bundle(&files), &document(&format!("prompt: hi\n{}", tree_value(GOOD_TREE))));
        let result = case.schemas.effective_for(&case.markdown);
        assert!(matches!(result, Ok(Some(_))), "the key is shared: {:?}", result.err().map(|error| error.to_string()));
        assert_eq!(case.problems(), Vec::<String>::new());
        let sections = case.with_document(&document(&format!("sections: {{}}\n{}", tree_value(GOOD_TREE))));
        assert_eq!(sections.problems(), Vec::<String>::new(), "the sections arm stays");
        let neither = case.with_document(&document(&tree_value(GOOD_TREE)));
        assert!(!neither.problems().is_empty(), "the union still requires one of its arms");
        let case = case.with_document(&document(&format!("prompt: hi\n{}", tree_value(BAD_TREE))));
        assert!(case.problems().iter().any(|path| path.starts_with("/review-model")), "{:?}", case.problems());
    }
}

/// A recursive reference and its inline unfolding coalesce at a nested
/// position: `wrap` (a payload body) holds the type at `{}`.
fn a_recursive_reference_and_its_inline_unfolding_coalesce_inside(wrap: &str, accepted: &str, rejected: &str) {
    let named = typed_trigger(NODE, &wrap.replace("{}", "Node"));
    let inline = typed_trigger(NODE, &wrap.replace("{}", TREE));
    coalesces(&named, &inline, accepted, rejected);
}

// One test per nested position: each runs `coalesces`' twelve cases.
#[test]
fn a_recursive_reference_and_its_inline_unfolding_coalesce_in_an_object_property() {
    a_recursive_reference_and_its_inline_unfolding_coalesce_inside(
        "    review-model:\n        tree: \"{}\"\n",
        &format!("review-model:\n  tree: {GOOD_TREE}\n"),
        &format!("review-model:\n  tree: {BAD_TREE}\n"),
    );
}

#[test]
fn a_recursive_reference_and_its_inline_unfolding_coalesce_in_a_pattern_dictionary() {
    a_recursive_reference_and_its_inline_unfolding_coalesce_inside(
        "    review-model:\n        <string>: \"{}\"\n",
        &format!("review-model:\n  first: {GOOD_TREE}\n"),
        &format!("review-model:\n  first: {BAD_TREE}\n"),
    );
}

#[test]
fn a_recursive_reference_and_its_inline_unfolding_coalesce_in_a_tuple_element() {
    a_recursive_reference_and_its_inline_unfolding_coalesce_inside(
        "    review-model: \"tuple([{}, string])\"\n",
        &format!("review-model: [{GOOD_TREE}, x]\n"),
        &format!("review-model: [{BAD_TREE}, x]\n"),
    );
}

#[test]
fn a_recursive_reference_and_its_inline_unfolding_coalesce_in_a_tuple_rest() {
    a_recursive_reference_and_its_inline_unfolding_coalesce_inside(
        "    review-model: \"tuple([string, ...{}[]])\"\n",
        &format!("review-model: [x, {GOOD_TREE}]\n"),
        &format!("review-model: [x, {BAD_TREE}]\n"),
    );
}

#[test]
fn a_recursive_reference_and_its_inline_unfolding_coalesce_in_a_list() {
    a_recursive_reference_and_its_inline_unfolding_coalesce_inside(
        "    review-model: \"{}[]\"\n",
        &format!("review-model: [{GOOD_TREE}]\n"),
        &format!("review-model: [{BAD_TREE}]\n"),
    );
}

#[test]
fn recursive_declarations_that_differ_only_past_an_unfolding_conflict() {
    let deeper = "{ name: string, children: { name: string(min(1)), children: Node[] }[] }";
    conflicts_on_review_model(&bundle(&[(A, &typed_trigger(NODE, &review_model("Node"))), (B, &typed_trigger(NODE, &review_model(deeper)))]), "");
    conflicts_on_review_model(&bundle(&[(A, &typed_trigger(NODE, &review_model(deeper))), (B, &typed_trigger(NODE, &review_model("Node")))]), "");

    // Two named recursive types, the difference reachable only through
    // each one's own reference.
    let other = "    Other: \"{ name: string, children: { name: string(min(1)), children: Other[] }[] }\"\n";
    let types = format!("{NODE}{other}");
    let named = typed_trigger(&types, "    review-model: Node\n    spare: Other\n");
    let differs = typed_trigger(&types, "    review-model: Other\n    spare: Node\n");
    let case = Case::new(&bundle(&[(A, &named), (B, &differs)]), &document("prompt: hi\n"));
    assert_eq!(case.conflict().0.as_deref(), Some("review-model"));
}

/// The recursive pair's checked layer is memoized like any other.
#[test]
fn a_recursive_pairs_static_result_is_memoized_per_matched_set() {
    let bundle = bundle(&[(A, &typed_trigger(NODE, &review_model("Node"))), (B, &typed_trigger(NODE, &review_model(TREE)))]);
    let case = Case::new(&bundle, &document(&tree_value(GOOD_TREE)));
    let selected = case.schemas.registered_selection_for(&case.markdown);
    let darkmatter::config::registration::HostSelection::Registered(profile) = &selected.selection else { panic!("acme") };
    let first = profile.checked_layer(&selected.matched).expect("checked");
    let again = profile.checked_layer(&selected.matched).expect("checked");
    assert!(Arc::ptr_eq(&first, &again));
}

/// A constant template returning the inline unfolding of an imported tree
/// prepares once resolved, and stays pending in the passive trace.
#[test]
fn a_template_returning_an_imported_trees_unfolding_prepares_once_resolved() {
    use darkmatter::markdown::compose::DataRuntime;
    use darkmatter::markdown::schemas::slots::SlotSession;
    use darkmatter::markdown::schemas::slots::grants::{Grant, GrantDecision, GrantScope, GrantTarget};
    let library = "kind: schema\ntypes:\n    Node: \"{ name: string, children: Node[] }\"\n";
    let imported = trigger(&review_model("Node@./lib.yaml"));
    let template = trigger(&review_model("{{ '{ name: string, children: Node[]@./lib.yaml }' }}"));
    let forms = [
        (imported.clone(), template.clone()),
        (imported.clone(), form_of(&template, in_both_arms)),
        (form_of(&imported, in_both_arms), template),
    ];
    for (imported, template) in forms {
        for files in [[(A, imported.as_str()), (B, template.as_str())], [(A, template.as_str()), (B, imported.as_str())]] {
            let bundle = bundle(&[files[0], files[1], ("schemas/lib.yaml", library)]);
            for (value, refused) in [(GOOD_TREE, false), (BAD_TREE, true)] {
                let case = Case::new(&bundle, &document(&format!("prompt: hi\n{}", tree_value(value))));
                assert!(matches!(case.schemas.effective_for(&case.markdown), Err(SchemaError::UnresolvedSlots(_))), "passive without a session");
                let trace = case.schemas.registered_trace(&case.markdown).expect("identified");
                assert_eq!(trace.conflict, None, "{trace:#?}");
                assert!(trace.triggers.iter().any(|entry| entry.matched && entry.pending), "{trace:#?}");
                // The template's import reads the library, which this run approves.
                let library = Grant { decision: GrantDecision::Allow, target: GrantTarget::File(case.path("schemas/lib.yaml")), scope: GrantScope::Once };
                let session = SlotSession::new(DataRuntime::new()).with_run_grants([library]);
                let schemas = case.schemas.clone().with_slot_session(Arc::new(session));
                let effective = schemas.effective_for(&case.markdown).expect("the resolved template coalesces").expect("a schema");
                let report = effective.validate(&darkmatter::markdown::schemas::frontmatter_as_json(&case.markdown));
                assert_eq!(!report.problems.is_empty(), refused, "{} / {} / {value}: {:#?}", files[0].1, files[1].1, report.problems);
            }
        }
    }
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
    let wide = trigger(&review_model("(string | number)[]"));
    let narrow = trigger(&review_model("enum(claude, codex)[]"));
    let case = Case::new(&bundle(&[(A, &wide), (B, &narrow)]), &document("review-model: [claude]\n"));
    let selected = case.schemas.registered_selection_for(&case.markdown);
    let darkmatter::config::registration::HostSelection::Registered(profile) = &selected.selection else { panic!("acme") };
    let first = profile.checked_layer(&selected.matched).expect("checked");
    let again = profile.checked_layer(&selected.matched).expect("checked");
    assert!(Arc::ptr_eq(&first, &again));
    let pruned = first.payloads.iter().find(|part| profile.triggers()[part.trigger].bundle_path == A).expect("a");
    assert!(pruned.json_schema["properties"].get("review-model").is_none(), "`a` lost `review-model` to `b`");
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

/// A closed document `$schema` declaring `team` and `prompt`, plus `rest`.
fn closed_document(rest: &str) -> String {
    document(&format!("$schema:\n  $constraints: {{ closed: true }}\n  team: string\n  prompt: string\n{rest}"))
}

#[test]
fn a_closed_document_schema_closes_every_arm_of_a_host_union() {
    let bundle = bundle(&[(A, &trigger(INLINE))]);
    let case = Case::new(&bundle, &closed_document("prompt: hi\nextra: hi\n"));
    assert_eq!(case.unknown_keys(), ["extra"], "the document's closed root refuses `extra` in the arm that holds `prompt`");
    let valid = Case::new(&bundle, &closed_document("prompt: hi\n"));
    assert!(valid.problems().is_empty(), "{:?}", valid.problems());
    let sections = Case::new(&bundle, &closed_document("sections:\n  intro: hi\n"));
    assert!(sections.problems().is_empty(), "the host arm's `sections` is declared: {:?}", sections.problems());
    // A closed document that leaves `prompt` to the host keeps the host's
    // exactly-one rule; one that redeclares `prompt` replaces it per key.
    let only_team = |rest: &str| document(&format!("$schema:\n  $constraints: {{ closed: true }}\n  team: string\n{rest}"));
    let both = Case::new(&bundle, &only_team("prompt: hi\nsections:\n  intro: hi\n"));
    assert!(!both.problems().is_empty(), "the host's exactly-one rule still holds");
    assert_eq!(Case::new(&bundle, &only_team("prompt: hi\nextra: hi\n")).unknown_keys(), ["extra"]);
}

#[test]
fn a_closed_root_trigger_closes_every_arm_of_a_host_union() {
    let fixture = Fixture::new();
    fixture.install_bundle("host", &bundle(&[(A, &trigger(INLINE))]));
    let hosts = fixture.hosts(&[HOST]);
    fixture.write(
        "work/schemas/closed.yaml",
        "kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n    $constraints: { closed: true }\n    team: string\n",
    );
    let refused = |document: &str| {
        let (markdown, context) = fixture.document(&fixture.repository(None), "work/doc.md", document);
        let schemas = DarkmatterSchemas::new(context).with_registered_hosts(hosts.clone()).with_trigger_discovery().expect("discovery");
        let effective = schemas.effective_for(&markdown).expect("assembles").expect("a schema");
        unknown_keys(&effective, &markdown)
    };
    assert_eq!(refused(&document("prompt: hi\nextra: hi\n")), ["extra"], "the trigger's closed root holds over the union");
    assert!(refused(&document("prompt: hi\n")).is_empty());
}

/// A document union: one arm requires `x`, the other a numeric `y`.
const DOCUMENT_UNION: &str = "$schema:\n  - x: string(required)\n  - y: number(required)\n";

#[test]
fn a_document_union_over_a_host_union_requires_one_arm_of_each() {
    let bundle = bundle(&[(A, &trigger(INLINE)), (B, &trigger("    agent: string\n"))]);
    let valid = Case::new(&bundle, &document(&format!("{DOCUMENT_UNION}prompt: hi\nx: a\n")));
    let effective = valid.effective();
    assert_eq!(effective.json_schema["anyOf"].as_array().map(Vec::len), Some(4), "host arm × document arm");
    assert!(valid.problems().is_empty(), "{:?}", valid.problems());
    let other = Case::new(&bundle, &document(&format!("{DOCUMENT_UNION}sections:\n  intro: hi\ny: 3\nagent: me\n")));
    assert!(other.problems().is_empty(), "{:?}", other.problems());

    for (rest, why) in [
        ("prompt: hi\n", "no document arm matches"),
        ("prompt: hi\ny: three\n", "`y` is a number"),
        ("x: a\n", "no host arm matches"),
        ("prompt: hi\nsections:\n  intro: hi\nx: a\n", "the host's exactly-one rule"),
        ("prompt: hi\nx: a\nagent: [1]\n", "the lower trigger's `agent` is a string in every arm"),
    ] {
        let case = Case::new(&bundle, &document(&format!("{DOCUMENT_UNION}{rest}")));
        assert!(!case.problems().is_empty(), "{why}");
        assert!(!case.phase_problems(SchemaPhase::Completion).is_empty(), "{why}, at completion");
    }
    assert!(valid.phase_problems(SchemaPhase::Launch).is_empty(), "{:?}", valid.phase_problems(SchemaPhase::Launch));
}

#[test]
fn a_document_union_arm_redeclaring_a_host_key_wins_in_its_arms() {
    let bundle = bundle(&[(A, &trigger(INLINE))]);
    let union = "$schema:\n  - prompt: number(required)\n  - z: string(required)\n";
    let case = Case::new(&bundle, &document(&format!("{union}prompt: 3\n")));
    assert!(case.problems().is_empty(), "the document arm's `prompt: number` wins: {:?}", case.problems());
    let case = Case::new(&bundle, &document(&format!("{union}prompt: hi\nz: a\n")));
    assert!(case.problems().is_empty(), "the other document arm keeps the host's `prompt`: {:?}", case.problems());
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

/// A template payload is checked once resolved, by the same alternative-by-
/// alternative comparison, and stays pending without a session.
#[test]
fn a_template_resolving_to_swapped_descriptions_conflicts_once_resolved() {
    use darkmatter::markdown::compose::DataRuntime;
    use darkmatter::markdown::schemas::slots::SlotSession;
    let template = "    review-model:\n        - \"enum({{ ['claude', 'codex'] }}) -> model choices\"\n        - \"string[] -> single model\"\n";
    let preserved = "    review-model:\n        - \"enum({{ ['claude', 'codex'] }}) -> single model\"\n        - \"string[] -> model choices\"\n";
    let swapped = Case::new(&bundle(&[(A, &trigger(REVIEW_MODEL)), (B, &trigger(template))]), &document(""));
    assert!(matches!(swapped.schemas.effective_for(&swapped.markdown), Err(SchemaError::UnresolvedSlots(_))), "passive without a session");
    assert_eq!(swapped.schemas.registered_trace(&swapped.markdown).expect("identified").conflict, None, "the trace leaves the template pending");

    let session = || Arc::new(SlotSession::new(DataRuntime::new()));
    let resolved = swapped.schemas.clone().with_slot_session(session());
    let Err(SchemaError::HostTriggerConflict(conflict)) = resolved.effective_for(&swapped.markdown) else {
        panic!("the resolved template conflicts")
    };
    assert_eq!(conflict.subject.as_deref(), Some("review-model"));
    assert!(conflict.reason.contains("descriptions differ"), "{}", conflict.reason);

    let kept = Case::new(&bundle(&[(A, &trigger(REVIEW_MODEL)), (B, &trigger(preserved))]), &document("review-model: gemini\n"));
    let schemas = kept.schemas.clone().with_slot_session(session());
    let effective = schemas.effective_for(&kept.markdown).expect("the resolved template narrows").expect("a schema");
    assert!(!effective.validate(&darkmatter::markdown::schemas::frontmatter_as_json(&kept.markdown)).valid, "the enum judges");
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

/// Two named triggers' conflict names each by its label beside its path,
/// keeping the code and both paths.
#[test]
fn a_conflict_between_named_triggers_shows_both_labels() {
    let named = |name: &str, payload: &str| format!("name: {name}\n{}", trigger(payload));
    let case = Case::new(&bundle(&[(A, &named("alpha-schema", "    agent: number\n")), (B, &named("beta-schema", "    agent: boolean\n"))]), &document(""));
    let Some(SchemaError::HostTriggerConflict(conflict)) = case.schemas.effective_for(&case.markdown).err() else {
        panic!("a trigger conflict")
    };
    assert_eq!((conflict.first.clone(), conflict.second.clone()), (case.path(A), case.path(B)), "the paths are kept");
    assert_eq!((conflict.first_name.as_deref(), conflict.second_name.as_deref()), (Some("alpha-schema"), Some("beta-schema")));
    let error = SchemaError::HostTriggerConflict(conflict);
    let expected = format!(
        "triggers `alpha-schema ({})` and `beta-schema ({})` conflict on `agent`",
        case.path(A).display(),
        case.path(B).display()
    );
    assert!(error.to_string().contains(&expected), "{error}");
    let records = schema_error_diagnostics(&error, std::path::Path::new("unknown")).expect("coded");
    let [record] = records.as_slice() else { panic!("one record: {records:#?}") };
    assert_eq!(record.code, HOST_TRIGGER_CONFLICT_CODE);
    assert!(record.message.contains(&expected), "{}", record.message);
    let trace = case.schemas.registered_trace(&case.markdown).expect("identified").conflict.expect("the trace's conflict");
    assert_eq!((trace.first_name.as_deref(), trace.second_name.as_deref()), (Some("alpha-schema"), Some("beta-schema")));
}

// ── the passive trace (`md schema triggers`) ────────────────────────────────

#[test]
fn the_registered_trace_reports_the_conflict_effective_for_refuses_with() {
    let conflicting = Case::new(&bundle(&[(A, &trigger("    agent: number\n")), (B, &trigger("    agent: boolean\n"))]), &document(""));
    let compatible = Case::new(&bundle(&[(A, &trigger("    agent: number\n")), (B, &trigger("    agent: number\n"))]), &document(""));

    let trace = conflicting.schemas.registered_trace(&conflicting.markdown).expect("the host identifies the document");
    let Some(SchemaError::HostTriggerConflict(refused)) = conflicting.schemas.effective_for(&conflicting.markdown).err() else {
        panic!("preparation refuses the document")
    };
    assert_eq!(trace.triggers.iter().filter(|entry| entry.matched).count(), 2, "both matches are listed: {trace:#?}");
    assert_eq!(trace.conflict.as_ref(), Some(refused.as_ref()), "the trace reports the same checked merge");
    assert_eq!(compatible.schemas.registered_trace(&compatible.markdown).expect("identified").conflict, None);
}

#[test]
fn a_registered_template_stays_pending_in_the_trace_while_static_pairs_are_checked() {
    let template = "    agent: enum({{ load_data(\"./agents.yaml\", \"string[]\") }})\n";
    let with_template = Case::new(&bundle(&[(A, &trigger(template)), (B, &trigger("    agent: boolean\n"))]), &document(""));
    let static_pair = Case::new(
        &bundle(&[(A, &trigger(template)), (B, &trigger("    mode: number\n")), (C, &trigger("    mode: boolean\n"))]),
        &document(""),
    );

    let trace = with_template.schemas.registered_trace(&with_template.markdown).expect("identified");
    assert_eq!(trace.conflict, None, "a template is not evaluated by a passive trace: {trace:#?}");
    assert!(trace.triggers.iter().any(|entry| entry.matched && entry.pending), "{trace:#?}");
    let conflict = static_pair.schemas.registered_trace(&static_pair.markdown).expect("identified").conflict.expect("the static pair conflicts");
    assert_eq!((conflict.first, conflict.second), (static_pair.path(B), static_pair.path(C)));
}

/// A presence marker lowers to one schema whichever side of `[]` carries
/// it, so the two spellings coalesce. Rules that lower differently do not:
/// an element `eager` is not the list's, and `generated` on a different
/// union alternative annotates that alternative.
#[test]
fn a_presence_marker_on_either_side_of_a_list_coalesces() {
    let named = "    U: [file, number]\n";
    for (types, a, b) in [("", "string(generated)[]", "string[](generated)"), (named, "U(eager)[]", "U[](eager)"), (named, "U(generated)[]", "U[](generated)")] {
        let file = |ty: &str| if types.is_empty() { trigger(&review_model(ty)) } else { typed_trigger(types, &review_model(ty)) };
        for form in [str::to_string as fn(&str) -> String, in_both_arms] {
            let case = Case::new(&bundle(&[(A, &form_of(&file(a), form)), (B, &file(b))]), &document("prompt: hi\n"));
            let result = case.schemas.effective_for(&case.markdown);
            assert!(matches!(result, Ok(Some(_))), "{a} coalesces with {b}: {:?}", result.err().map(|e| e.to_string()));
        }
    }
    conflicts_on_review_model(&bundle(&[(A, &trigger(&review_model("file(eager)[]"))), (B, &trigger(&review_model("file[](eager)")))]), "list constraints differ");
    conflicts_on_review_model(
        &bundle(&[(A, &trigger(&review_model("string(generated) | number"))), (B, &trigger(&review_model("string | number(generated)")))]),
        "constraints differ at alternative 1",
    );
}
