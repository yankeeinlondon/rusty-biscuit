//! Template types before resolution: a `{{ … }}` slot standing for a
//! property's whole type parses at every allowed position, is refused with
//! one explicit message everywhere else, and is answered as pending by every
//! passive reader. Nothing here resolves a template or evaluates a slot.
//!
//! Boundary: the crate's public API, in-process. Fixtures:
//! `tests/fixtures/dynamic_slots/template-types/`.

use std::ops::Range;

use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;
use simplified_schema::completion::{AlternativeSite, alternatives_for_def};
use simplified_schema::embedded::ShippedSources;
use simplified_schema::simplified::grammar::{parse_type_definition, parse_type_expr};
use simplified_schema::simplified::narrowing::{ALL_FAMILIES, ContainerAdmission, Family, container_admission, families};
use simplified_schema::simplified::{parse_yaml_schema_with_source, serialize_property_atom, serialize_property_def};
use simplified_schema::slots::{SlotAdmissionError, SlotFailure, SlotHost, SlotRequest, collect_slots, definition_slots};
use simplified_schema::{
    DefinitionTable, PropertyAtom, PropertyDef, Requiredness, SchemaDeclaration, SchemaError, SchemaRoot, SimplifiedSchema,
    SimplifiedSchemaError, Slot, TypeExpr, UnresolvedSlots, ValidationFailure, flattened_alternatives,
    is_definition_subtype, is_subtype, parse_property_definition, parse_schema_declaration, parse_type_definition_value,
    parse_yaml_schema, suggestions_for_def, to_json_schema,
    to_json_schema_with_definitions,
};

// ── helpers ──────────────────────────────────────────────────────────────

fn yaml(text: &str) -> YamlValue {
    serde_yaml_ng::from_str(text).unwrap_or_else(|error| panic!("test YAML parses: {error}\n{text}"))
}

fn def(text: &str) -> PropertyDef {
    parse_type_definition("p", text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn atom(text: &str) -> PropertyAtom {
    parse_type_expr("p", text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn root(text: &str) -> SchemaRoot {
    parse_yaml_schema(&yaml(text)).unwrap_or_else(|error| panic!("{error}\n{text}"))
}

/// The template type a single-atom definition holds.
#[track_caller]
fn template_slot(atom: &PropertyAtom) -> &Slot {
    match &atom.ty {
        TypeExpr::Slot(slot) => slot,
        other => panic!("expected a template type, got {other:?}"),
    }
}

fn properties(root: &SchemaRoot) -> &simplified_schema::SchemaShape {
    match root {
        SchemaRoot::Single(shape) => shape,
        SchemaRoot::Union(_) => panic!("expected a single object root"),
    }
}

fn single(def: &PropertyDef) -> &PropertyAtom {
    match def {
        PropertyDef::Single(atom) => atom,
        PropertyDef::Union(arms) => panic!("expected one atom, got {} arms", arms.len()),
    }
}

/// The expressions of `slots`, in order.
fn expressions(slots: &[Slot]) -> Vec<&str> {
    slots.iter().map(|slot| slot.expression.as_str()).collect()
}

/// The `{{ … }}` and the expression a slot's spans cover in `text`.
#[track_caller]
fn assert_slot_text(text: &str, slot: &Slot, whole: &str, expression: &str) {
    assert_eq!(&text[slot.span.clone()], whole, "slot span of {slot:?}");
    assert_eq!(&text[slot.expression_span.clone()], expression, "expression span of {slot:?}");
}

/// A host that fails the test if anything consults it: passive operations
/// never take one, and nothing here resolves.
struct PanickingHost;

impl SlotHost for PanickingHost {
    fn admit(&self, slot: &Slot) -> Result<(), SlotAdmissionError> {
        panic!("a passive operation admitted `{}`", slot.expression)
    }

    fn resolve(&self, request: &SlotRequest<'_>) -> Result<Value, SlotFailure> {
        panic!("a passive operation evaluated `{}`", request.slot.expression)
    }

    fn template_reads(&self) -> std::rc::Rc<dyn simplified_schema::slots::TemplateReads> {
        panic!("a passive operation asked for a host's template reads")
    }

    fn is_canceled(&self) -> bool {
        panic!("a passive operation asked a host whether it was canceled")
    }
}

/// Every template type `properties.yaml` writes, by property, in
/// declaration order.
const FIXTURE_SLOTS: &[(&str, &str)] = &[
    ("status", "load_data('./status-type.yaml', 'type-definition')"),
    ("tags", "tag_type"),
    ("owner", "owner_type"),
    ("owners", "owners_type"),
    ("reviewers", "reviewer_type"),
    ("phase", "phase_type"),
    ("either", "either_type"),
    ("fallback", "fallback_type"),
    ("grouped", "grouped_type"),
    ("pair", "pair_type"),
    ("maybe", "maybe_type"),
    ("inline", "inline_type"),
    ("settings", "nested_type"),
];

// ── parsing at every allowed position ────────────────────────────────────

#[test]
fn a_property_type_parses_to_a_slot_atom_with_its_spans() {
    let text = "{{ load_data('./status-type.yaml', 'type-definition') }}";
    let atom = atom(text);
    let slot = template_slot(&atom);
    assert_eq!(slot.expression, "load_data('./status-type.yaml', 'type-definition')");
    assert_slot_text(text, slot, text, "load_data('./status-type.yaml', 'type-definition')");
    assert_eq!(slot.origin, None, "an in-memory type string has no defining file");
    assert!(atom.is_pending_type());
    assert!(!atom.is_array && atom.constraints.is_empty() && atom.array_constraints.is_empty());
    assert_eq!(atom.description, None);
}

#[test]
fn a_property_type_parses_in_the_string_and_yaml_list_forms() {
    let string_form = parse_property_definition("status", &yaml("'{{ t }}'")).expect("string form");
    assert!(single(&string_form).is_pending_type());

    let list_form = parse_property_definition("status", &yaml("- string\n- '{{ t }}'\n")).expect("list form");
    let PropertyDef::Union(arms) = &list_form else { panic!("a YAML list is a union: {list_form:?}") };
    assert_eq!(arms.len(), 2);
    assert!(!arms[0].is_pending_type());
    assert_eq!(template_slot(&arms[1]).expression, "t");
    // Spans index each form's own text, so the forms compare as written.
    assert_eq!(serialize_property_def(&list_form), serialize_property_def(&def("string | {{ t }}")), "one union");

    // The `type-definition` value domain reads both forms the same way.
    assert_eq!(parse_type_definition_value("status", &yaml("'{{ t }}[]'")).unwrap(), def("{{ t }}[]"));
    assert_eq!(parse_type_definition_value("status", &yaml("- string\n- '{{ t }}'\n")).unwrap(), list_form);
}

#[test]
fn a_nested_object_property_takes_a_template_type() {
    // A nested YAML mapping.
    let parsed = root("settings:\n  mode: '{{ mode_type }}'\n  label: string\n");
    let slots = collect_slots(&parsed, &DefinitionTable::default());
    assert_eq!(expressions(&slots), ["mode_type"]);

    // An inline object property.
    let inline = atom("{ mode: {{ mode_type }}(required), label: string }");
    let TypeExpr::InlineObject(shape) = &inline.ty else { panic!("an inline object: {inline:?}") };
    let mode = single(&shape.properties["mode"]);
    assert_eq!(template_slot(mode).expression, "mode_type");
    assert_eq!(shape.properties["mode"].requiredness(), Requiredness::Pending);
    assert_eq!(expressions(&definition_slots(&PropertyDef::Single(inline))), ["mode_type"]);
}

#[test]
fn a_named_type_body_takes_a_template_type() {
    let schema = SimplifiedSchema::try_from(
        "kind: schema\n$schema:\n  state: State\n  state_here: State@this\ntypes:\n  State: '{{ state_type }}[]'\n",
    )
    .expect("a named type body may be a template type");
    assert!(!schema.is_resolved());
    let named = schema.named_type("State").expect("the named type is stored");
    assert_eq!(expressions(&named.pending_slots()), ["state_type"]);
    for property in ["state", "state_here"] {
        let view = schema.property(property).unwrap_or_else(|| panic!("`{property}` is declared"));
        assert_eq!(expressions(&view.pending_slots()), ["state_type"], "`{property}` reaches the body's slot");
    }
}

#[test]
fn a_union_arm_takes_a_template_type_in_either_order() {
    for (text, slot_arm) in [("string | {{ t }}", 1), ("{{ t }} | number", 0)] {
        let PropertyDef::Union(arms) = def(text) else { panic!("{text} is a union") };
        assert_eq!(arms.len(), 2, "{text}");
        assert_eq!(template_slot(&arms[slot_arm]).expression, "t", "{text}");
        assert!(!arms[1 - slot_arm].is_pending_type(), "{text}");
    }
}

#[test]
fn a_grouped_union_takes_a_template_type() {
    let grouped = atom("({{ t }} | string)[]");
    assert!(grouped.is_array);
    let TypeExpr::Union(arms) = &grouped.ty else { panic!("a grouped union: {grouped:?}") };
    assert_eq!(template_slot(&arms[0]).expression, "t");
    assert!(!arms[1].is_pending_type());
    assert!(!grouped.is_pending_type(), "the list itself is a known array; its item is pending");
    assert_eq!(expressions(&definition_slots(&PropertyDef::Single(grouped))), ["t"]);
}

#[test]
fn a_tuple_element_takes_a_template_type_required_or_optional() {
    for (text, index, optional) in [("tuple([number, {{ t }}])", 1, false), ("tuple([{{ t }}?])", 0, true)] {
        let tuple = atom(text);
        let TypeExpr::Tuple(tuple) = &tuple.ty else { panic!("{text} is a tuple") };
        let element = &tuple.elements[index];
        assert_eq!(element.optional, optional, "{text}");
        assert_eq!(template_slot(single(&element.def)).expression, "t", "{text}");
    }
}

#[test]
fn every_fixture_position_parses_and_reports_its_slot() {
    let text = include_str!("../fixtures/dynamic_slots/template-types/properties.yaml");
    let schema = SimplifiedSchema::try_from(text).expect("the fixture is a valid template");
    assert!(!schema.is_resolved());
    for (property, expression) in FIXTURE_SLOTS {
        let view = schema.property(property).unwrap_or_else(|| panic!("`{property}` is declared"));
        assert_eq!(expressions(&view.pending_slots()), [*expression], "`{property}`");
    }
    for property in ["state", "state_here"] {
        assert_eq!(expressions(&schema.property(property).unwrap().pending_slots()), ["state_type"], "`{property}`");
    }
    let mut all: Vec<&str> = FIXTURE_SLOTS.iter().map(|(_, expression)| *expression).collect();
    all.push("state_type");
    let mut found = expressions(schema.pending_slots());
    found.sort_unstable();
    all.sort_unstable();
    assert_eq!(found, all, "each slot is listed once, the named type's included");
}

// ── suffixes ─────────────────────────────────────────────────────────────

#[test]
fn every_suffix_is_recorded_on_the_slot_atom() {
    let plain = atom("{{ t }}[]");
    assert!(plain.is_array && plain.constraints.is_empty() && plain.array_constraints.is_empty());

    let required = atom("{{ t }}(required)");
    assert!(!required.is_array);
    assert_eq!(required.constraints.iter().map(|c| c.keyword()).collect::<Vec<_>>(), ["required"]);

    let required_list = atom("{{ t }}[](required)");
    assert!(required_list.is_array && required_list.constraints.is_empty());
    assert_eq!(required_list.array_constraints.iter().map(|c| c.keyword()).collect::<Vec<_>>(), ["required"]);

    let both = atom("{{ t }}(required)[](min(1))");
    assert!(both.is_array);
    assert_eq!(both.constraints.iter().map(|c| c.keyword()).collect::<Vec<_>>(), ["required"]);
    assert_eq!(both.array_constraints.iter().map(|c| c.keyword()).collect::<Vec<_>>(), ["min"]);

    let described = def("{{ t }}[] -> the record's tags");
    assert_eq!(single(&described).description.as_deref(), Some("the record's tags"));
    assert!(single(&described).is_array);

    for parsed in [&plain, &required, &required_list, &both, single(&described)] {
        let slot = template_slot(parsed);
        assert_eq!(slot.expression, "t");
        assert_eq!(slot.span, 0..7, "the span covers the slot alone, never its suffix");
        assert_eq!(slot.expression_span, 3..4);
    }
}

#[test]
fn requiredness_is_pending_even_when_the_suffix_says_required() {
    for text in ["{{ t }}", "{{ t }}(required)", "{{ t }}[](required)", "{{ t }}(required)[](min(1))", "string(required) | {{ t }}"]
    {
        let parsed = def(text);
        assert!(parsed.has_pending_type(), "{text}");
        assert_eq!(parsed.requiredness(), Requiredness::Pending, "{text}");
    }
    // An item alternative's returned `required` marker would hoist to the
    // property, so it is pending unless the rest already requires it.
    for text in ["({{ t }} | string)[]", "number | ({{ t }} | boolean)[]", "(number | ({{ t }} | boolean)[])[]"] {
        let parsed = def(text);
        assert!(!parsed.has_pending_type(), "{text}: no top-level template type");
        assert_eq!(parsed.requiredness(), Requiredness::Pending, "{text}");
    }
    // Control: a written requirement on the array, or a slot in a tuple,
    // leaves presence known.
    assert_eq!(def("({{ t }} | string)[](required)").requiredness(), Requiredness::Required);
    assert_eq!(def("tuple([{{ t }}])").requiredness(), Requiredness::Optional);
    assert_eq!(def("string(required)").requiredness(), Requiredness::Required);
}

#[test]
fn a_slot_atom_passes_the_union_and_tuple_element_checks_before_resolution() {
    def("string | {{ t }}").check_union_invariants("p").expect("nothing about a pending arm can be judged yet");
    simplified_schema::simplified::types::TupleType::check_element(&def("{{ t }}")).expect("a pending element");
    let described = simplified_schema::simplified::types::TupleType::check_element(&def("{{ t }} -> d"))
        .expect_err("a tuple element takes no description, pending or not");
    assert!(described.contains("description"), "{described}");
}

// ── literal brace text stays literal ─────────────────────────────────────

#[test]
fn quoted_brace_text_in_an_enum_member_or_default_is_no_slot() {
    let parsed = root("kind: \"enum('{{ a }}', b)\"\ntitle: \"string(default('{{ x }}'))\"\n");
    assert!(collect_slots(&parsed, &DefinitionTable::default()).is_empty(), "quoted text is literal");
    let shape = properties(&parsed);
    for property in ["kind", "title"] {
        assert!(!shape.properties[property].has_pending_type(), "`{property}`");
        assert!(definition_slots(&shape.properties[property]).is_empty(), "`{property}`");
    }
    let schema = SimplifiedSchema::try_from("$schema:\n  kind: \"enum('{{ a }}', b)\"\n  title: \"string(default('{{ x }}'))\"\n")
        .expect("a concrete schema");
    assert!(schema.is_resolved());
    let lowered = schema.to_json_schema().expect("it lowers");
    assert!(lowered.to_string().contains("{{ a }}"), "the member is the literal text: {lowered}");
}

// ── passive operations evaluate nothing ──────────────────────────────────

#[test]
fn query_property_and_named_type_answer_from_the_authored_template() {
    let schema =
        SimplifiedSchema::try_from(include_str!("../fixtures/dynamic_slots/template-types/properties.yaml")).unwrap();

    let owners = schema.query("owners").expect("a valid path").expect("declared");
    assert_eq!(owners.type_text(), "{{ owners_type }}[](required)");
    assert_eq!(owners.def().requiredness(), Requiredness::Pending);
    // The view's presence answers pending, never a guessed `required`.
    assert_eq!(owners.requiredness(), Requiredness::Pending);
    assert!(!owners.is_required(), "`(required)` alone cannot decide presence before resolution");
    let static_required = SimplifiedSchema::try_from("$schema:\n  a: string(required)\n").unwrap();
    let a = static_required.property("a").expect("declared");
    assert!(a.is_required());
    assert_eq!(a.requiredness(), Requiredness::Required);

    let phase = schema.property("phase").expect("declared");
    assert_eq!(phase.description(), Some("where the record is in its lifecycle"));

    let mode = schema.query("settings.mode").expect("a valid path").expect("a nested property");
    assert_eq!(expressions(&mode.pending_slots()), ["nested_type"]);

    let state = schema.named_type("State").expect("stored");
    assert_eq!(state.type_text(), "{{ state_type }}");

    let names: Vec<&str> = schema.properties().into_iter().map(|(name, _)| name).collect();
    assert!(names.contains(&"status") && names.contains(&"state_here"), "{names:?}");
    // Nothing above takes a host; a concrete schema never consults one either.
    let concrete = SimplifiedSchema::try_from("$schema:\n  a: string\n").unwrap();
    concrete.resolve_slots(&PanickingHost).expect("a concrete schema resolves to itself without its host");
    assert!(!schema.is_resolved(), "querying resolved nothing");
}

#[test]
fn export_round_trips_a_template_type_in_a_nested_grouped_union() {
    let text = "$schema:\n  status: \"number | ({{ t }} | boolean)[](min(1))\"\n  pair: \"tuple([number | ({{ u }} | boolean)[]])\"\n";
    let schema = SimplifiedSchema::try_from(text).unwrap();
    let exported = schema.to_simplified_yaml().expect("a template exports");
    let document = yaml(&exported);
    // The writer spells a property-level union as a YAML list.
    assert_eq!(document["$schema"]["status"][1].as_str(), Some("({{ t }} | boolean)[](min(1))"), "{exported}");
    assert_eq!(document["$schema"]["pair"].as_str(), Some("tuple([number | ({{ u }} | boolean)[]])"), "{exported}");
    let reparsed = SimplifiedSchema::try_from(exported.as_str()).expect("the export parses again");
    assert_eq!(reparsed.to_simplified_yaml().unwrap(), exported, "export is a fixed point");
    let slots: Vec<&str> = reparsed.pending_slots().iter().map(|slot| slot.expression.as_str()).collect();
    assert_eq!(slots, ["t", "u"], "both nested slots are found");
}

#[test]
fn export_writes_each_template_type_and_its_suffix_verbatim_and_round_trips() {
    let schema =
        SimplifiedSchema::try_from(include_str!("../fixtures/dynamic_slots/template-types/properties.yaml")).unwrap();
    let exported = schema.to_simplified_yaml().expect("a template exports");
    // Compared after YAML decoding: the writer chooses its own quoting.
    let document = yaml(&exported);
    let properties = &document["$schema"];
    for (property, written) in [
        ("status", "{{ load_data('./status-type.yaml', 'type-definition') }}"),
        ("tags", "{{ tag_type }}[]"),
        ("owner", "{{ owner_type }}(required)"),
        ("owners", "{{ owners_type }}[](required)"),
        ("reviewers", "{{ reviewer_type }}(required)[](min(1))"),
        ("phase", "{{ phase_type }} -> where the record is in its lifecycle"),
        ("grouped", "({{ grouped_type }} | string)[]"),
        ("pair", "tuple([number, {{ pair_type }}])"),
        ("maybe", "tuple([{{ maybe_type }}?])"),
    ] {
        assert_eq!(properties[property].as_str(), Some(written), "`{property}` exports as authored:\n{exported}");
    }
    assert_eq!(properties["fallback"][1].as_str(), Some("{{ fallback_type }}"), "{exported}");
    assert_eq!(properties["either"][1].as_str(), Some("{{ either_type }}"), "{exported}");
    assert_eq!(properties["settings"]["mode"].as_str(), Some("{{ nested_type }}"), "{exported}");
    assert_eq!(properties["inline"]["mode"].as_str(), Some("{{ inline_type }}"), "{exported}");
    assert_eq!(document["types"]["State"].as_str(), Some("{{ state_type }}"), "{exported}");

    let reparsed = SimplifiedSchema::try_from(exported.as_str()).expect("the export parses again");
    assert!(!reparsed.is_resolved());
    assert_eq!(reparsed.to_simplified_yaml().unwrap(), exported, "export is a fixed point");
    let types = |schema: &SimplifiedSchema| -> Vec<(String, String)> {
        schema.properties().into_iter().map(|(name, view)| (name.to_string(), view.type_text())).collect()
    };
    assert_eq!(types(&reparsed), types(&schema));
    let mut before = expressions(schema.pending_slots());
    let mut after = expressions(reparsed.pending_slots());
    before.sort_unstable();
    after.sort_unstable();
    assert_eq!(after, before);
}

#[test]
fn serializing_a_slot_atom_writes_the_expression_then_the_suffix() {
    for text in [
        "{{ t }}",
        "{{ t }}[]",
        "{{ t }}(required)",
        "{{ t }}[](required)",
        "{{ t }}(required)[](min(1))",
        "{{ t }}[] -> the record's tags",
        "{{ load_data('./status-type.yaml', 'type-definition') }}",
    ] {
        let parsed = atom(text);
        let written = serialize_property_atom(&parsed);
        assert_eq!(written, text, "canonical spelling of {text}");
        assert_eq!(atom(&written), parsed, "{text} re-parses to the same atom");
    }
    assert_eq!(serialize_property_def(&def("string | {{ t }}")), "string | {{ t }}");
}

// ── lowering, validation, and layering refuse a template ────────────────

/// The template every refusal test lowers: a template type and nothing else.
const TEMPLATE: &str = "$schema:\n  title: string\n  status: '{{ status_type }}(required)'\n";

#[track_caller]
fn assert_lists_template_type(unresolved: &UnresolvedSlots) {
    assert_eq!(expressions(&unresolved.slots), ["status_type"], "the template type's slot is listed");
}

#[track_caller]
fn unresolved_of(error: &SchemaError) -> &UnresolvedSlots {
    match error {
        SchemaError::UnresolvedSlots(unresolved) => unresolved,
        other => panic!("expected `UnresolvedSlots`, got {other}"),
    }
}

#[track_caller]
fn unresolved_facade(error: &SimplifiedSchemaError) -> &UnresolvedSlots {
    match error {
        SimplifiedSchemaError::Unresolved(error) => unresolved_of(error),
        other => panic!("expected `SimplifiedSchemaError::Unresolved`, got {other}"),
    }
}

#[test]
fn to_json_schema_on_the_object_is_unresolved() {
    let template = SimplifiedSchema::try_from(TEMPLATE).unwrap();
    assert_lists_template_type(unresolved_facade(&template.to_json_schema().expect_err("no JSON Schema form")));
}

#[test]
fn validate_strict_is_unprepared_with_unresolved_slots() {
    let template = SimplifiedSchema::try_from(TEMPLATE).unwrap();
    for value in [json!({ "title": "t", "status": "open" }), json!({})] {
        match template.validate_strict(&value) {
            Err(ValidationFailure::Unprepared { error }) => assert_lists_template_type(unresolved_of(&error)),
            other => panic!("{value}: expected `Unprepared`, got {other:?}"),
        }
    }
}

#[test]
fn validate_soft_is_unprepared_with_unresolved_slots() {
    let template = SimplifiedSchema::try_from(TEMPLATE).unwrap();
    for value in [json!({ "title": "t", "status": "open" }), json!({ "status": 5 })] {
        match template.validate_soft(&value) {
            Err(ValidationFailure::Unprepared { error }) => assert_lists_template_type(unresolved_of(&error)),
            other => panic!("{value}: expected `Unprepared`, got {other:?}"),
        }
    }
}

#[test]
fn layer_under_refuses_a_template_on_either_side() {
    let template = SimplifiedSchema::try_from(TEMPLATE).unwrap();
    let concrete = SimplifiedSchema::try_from("$schema:\n  other: number\n").unwrap();

    let mut receiver = concrete.clone();
    let error = receiver.layer_under(&[&template]).expect_err("an incoming template");
    assert_lists_template_type(unresolved_facade(&error));
    assert!(receiver.is_resolved() && receiver.property("status").is_none(), "the call is atomic");

    let mut receiver = template.clone();
    let error = receiver.layer_under(&[&concrete]).expect_err("a template receiver");
    assert_lists_template_type(unresolved_facade(&error));
}

#[test]
fn layer_over_refuses_a_template_on_either_side() {
    let template = SimplifiedSchema::try_from(TEMPLATE).unwrap();
    let concrete = SimplifiedSchema::try_from("$schema:\n  other: number\n").unwrap();

    let mut receiver = concrete.clone();
    let error = receiver.layer_over(&[&template]).expect_err("an incoming template");
    assert_lists_template_type(unresolved_facade(&error));
    assert!(receiver.property("status").is_none(), "the call is atomic");

    let mut receiver = template.clone();
    let error = receiver.layer_over(&[&concrete]).expect_err("a template receiver");
    assert_lists_template_type(unresolved_facade(&error));
}

#[test]
fn lowering_a_parsed_root_is_unresolved() {
    let parsed = root("title: string\nstatus: '{{ status_type }}(required)'\n");
    let error = to_json_schema(&parsed).expect_err("a template has no JSON Schema form");
    let unresolved = unresolved_of(&error);
    assert_lists_template_type(unresolved);
    assert!(unresolved.pending.is_none(), "lowering alone cannot resolve");
}

#[test]
fn lowering_with_definitions_is_unresolved() {
    let parsed = root("status: '({{ status_type }} | string)[]'\n");
    let error = to_json_schema_with_definitions(&parsed, &DefinitionTable::default()).expect_err("unresolved");
    assert_lists_template_type(unresolved_of(&error));
}

#[test]
fn resolve_yaml_schema_is_unresolved_with_a_pending_handle() {
    let dir = std::env::temp_dir();
    let context = biscuit_file::FileResolutionContext::new(&dir);
    let value = yaml("title: string\nstatus: '{{ status_type }}(required)'\n");
    let error = simplified_schema::resolve::resolve_yaml_schema(&value, &dir, &context, ShippedSources::NONE)
        .expect_err("a template does not resolve passively");
    let unresolved = unresolved_of(&error);
    assert_lists_template_type(unresolved);
    let pending = unresolved.pending.as_ref().expect("an entry point attaches a handle");
    assert_eq!(expressions(pending.slots()), ["status_type"]);
}

#[test]
fn resolve_schema_from_json_is_unresolved() {
    let dir = std::env::temp_dir();
    let context = biscuit_file::FileResolutionContext::new(&dir);
    let value = json!({ "title": "string", "status": "string | {{ status_type }}" });
    let error = simplified_schema::resolve::resolve_schema(&value, &dir, &context, ShippedSources::NONE)
        .expect_err("a template does not resolve passively");
    assert_lists_template_type(unresolved_of(&error));
}

// ── readers answer pending ───────────────────────────────────────────────

#[test]
fn suggestions_for_a_template_type_are_pending_and_empty() {
    let query = suggestions_for_def("status", &def("{{ t }}")).expect("a pending type answers, never `None`");
    assert!(query.items.is_empty() && query.pending && !query.is_array);

    let list = suggestions_for_def("status", &def("{{ t }}[]")).expect("pending");
    assert!(list.items.is_empty() && list.pending && list.is_array);

    let mixed = suggestions_for_def("status", &def("string(suggest(a, b)) | {{ t }}")).expect("a static arm suggests");
    let labels: Vec<&str> = mixed.items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, ["a", "b"], "the static arm's candidates stay");
    assert!(mixed.pending, "the pending arm may add candidates");

    assert!(suggestions_for_def("status", &def("string")).is_none(), "control: no list, no answer");
}

#[test]
fn completion_offers_nothing_for_a_template_type_and_says_pending() {
    let completion = alternatives_for_def(&def("{{ t }}"), AlternativeSite::Value, &Value::Null).expect("a value site");
    assert!(completion.pending);
    assert!(completion.keys.is_empty() && completion.values.is_empty(), "no invented keys or values");

    let mixed = alternatives_for_def(&def("enum(a, b) | {{ t }}"), AlternativeSite::Value, &Value::Null).unwrap();
    let labels: Vec<&str> = mixed.values.iter().map(|value| value.label.as_str()).collect();
    assert_eq!(labels, ["a", "b"], "the static arm's members stay");
    assert!(mixed.pending);
}

#[test]
fn a_template_type_is_a_subtype_only_of_the_identical_slot() {
    let slot = atom("{{ t }}");
    assert!(is_subtype(&slot, &atom("{{ t }}")), "the identical slot");
    assert!(!is_subtype(&slot, &atom("{{ u }}")), "another slot is another type");
    assert!(!is_subtype(&slot, &atom("string")), "not known to be a string");
    assert!(!is_subtype(&atom("string"), &slot), "not known to admit strings");
    assert!(!is_subtype(&slot, &atom("unknown")), "a pending atom proves no fact, even against `unknown`");
    assert!(!is_subtype(&atom("{{ t }}[]"), &atom("{{ t }}")), "its list is not the slot itself");
}

#[test]
fn static_union_arms_still_prove_their_subtype_facts() {
    let with_slot = def("string | {{ t }}");
    assert!(is_definition_subtype(&def("string"), &with_slot), "the static arm covers `string`");
    assert!(is_definition_subtype(&def("email"), &with_slot), "and what `string` covers");
    assert!(!is_definition_subtype(&def("number"), &with_slot), "the pending arm proves nothing");
    assert!(!is_definition_subtype(&with_slot, &def("string")), "the pending arm is not known to be a string");
}

#[test]
fn narrowing_and_container_admission_assume_nothing_of_a_template_type() {
    assert_eq!(families(&atom("{{ t }}")), ALL_FAMILIES, "a pending type may be any family");
    assert_eq!(families(&atom("{{ t }}[]")), [Family::Array], "its list is still a list");

    assert_eq!(container_admission(&def("{{ t }}")), ContainerAdmission::Provisional);
    assert_eq!(container_admission(&def("{{ t }}(required)")), ContainerAdmission::Provisional);
    assert_eq!(container_admission(&def("{{ t }}[]")), ContainerAdmission::Container);
    assert_eq!(container_admission(&def("string | {{ t }}")), ContainerAdmission::NotContainer, "a static scalar arm decides");
}

#[test]
fn flattened_alternatives_expose_the_slot_atom() {
    let union = def("string | {{ t }}");
    let alternatives = flattened_alternatives(&union);
    assert_eq!(alternatives.len(), 2);
    assert!(!alternatives[0].atom.is_pending_type());
    assert_eq!(template_slot(alternatives[1].atom).expression, "t");
    assert_eq!(alternatives[1].arm, Some(1));

    let grouped = def("({{ t }} | string)[]");
    let alternatives = flattened_alternatives(&grouped);
    assert_eq!(alternatives.len(), 2);
    assert!(alternatives.iter().all(|alternative| alternative.is_array), "both are list items");
    assert!(alternatives[0].atom.is_pending_type());
}

// ── refused positions ────────────────────────────────────────────────────

const NOT_ALLOWED: &str = "template types are not allowed there";
const ALLOWED_POSITIONS: &[&str] = &[
    "a property's type (including nested properties and named type bodies)",
    "a property-level union arm",
    "a tuple element",
    "`[]`",
    "`(required)`",
];
const REWRITE: &str = "Write the name literally; to choose a type from data, give the property a template type: `name: \"{{ … }}\"`.";

/// A refusal names `position`, says template types are not allowed there,
/// lists the allowed positions, and carries the rewrite sentence exactly
/// when `rewrite` is set.
#[track_caller]
fn assert_refusal(error: &SchemaError, position: &str, rewrite: bool) -> Range<usize> {
    let SchemaError::Grammar { message, span, .. } = error else {
        panic!("expected a grammar error, got {error}");
    };
    assert!(message.contains(&format!("found in {position};")), "names {position}: {message}");
    assert!(message.contains(NOT_ALLOWED), "{message}");
    for allowed in ALLOWED_POSITIONS {
        assert!(message.contains(allowed), "lists `{allowed}`: {message}");
    }
    assert_eq!(message.contains(REWRITE), rewrite, "rewrite sentence expected: {rewrite}: {message}");
    span.clone()
}

#[track_caller]
fn refused_type(text: &str) -> SchemaError {
    match parse_type_definition("p", text) {
        Err(error) => error,
        Ok(parsed) => panic!("`{text}` should be refused, parsed {parsed:?}"),
    }
}

#[track_caller]
fn refused_declaration(text: &str) -> SchemaError {
    match parse_schema_declaration(&yaml(text)) {
        Err(error) => error,
        Ok(parsed) => panic!("`{text}` should be refused, parsed {parsed:?}"),
    }
}

#[test]
fn refused_at_the_root() {
    for text in ["'{{ x }}'", "'{{ x }}[]'", "'{{ x }}(required)'", "'{{ x }} -> d'"] {
        let span = assert_refusal(&refused_declaration(text), "the whole `$schema` root", true);
        assert_eq!(span, 0..7, "{text}");
    }
    // A file reference that only starts with slot text is still a reference.
    let reference = parse_schema_declaration(&yaml("'{{ dir }}/schema.yaml'"));
    assert!(
        !matches!(&reference, Err(SchemaError::Grammar { message, .. }) if message.contains(NOT_ALLOWED)),
        "{reference:?}"
    );
}

#[test]
fn refused_in_a_root_union_arm() {
    let error = refused_declaration("- title: string\n- '{{ x }}'\n");
    assert_refusal(&error, "an arm of a root `$schema` union", true);
}

#[test]
fn refused_as_a_yaml_property_key() {
    let error = refused_declaration("'{{ x }}': string\n");
    assert_refusal(&error, "a property name", true);
    // Brace text that does not open a balanced leading slot stays a name.
    let literal = parse_schema_declaration(&yaml("'a{{ x }}': string\n")).expect("a literal key");
    let SchemaDeclaration::Schema(parsed) = literal else { panic!("an inline schema") };
    assert!(properties(&parsed).properties.contains_key("a{{ x }}"));
}

#[test]
fn refused_as_an_inline_object_property_key() {
    let text = "{ {{x}}: string }";
    let span = assert_refusal(&refused_type(text), "a property name", true);
    assert_eq!(&text[span], "{{x}}");
}

#[test]
fn refused_in_a_pattern_key() {
    let error = refused_declaration("'<starting::{{x}}>': string\n");
    assert_refusal(&error, "a pattern key", true);
}

#[test]
fn refused_in_an_import_path() {
    let text = "Name@{{x}}";
    let span = assert_refusal(&refused_type(text), "an import path (after `@`)", true);
    assert_eq!(&text[span], "{{x}}");
}

#[test]
fn refused_after_a_type_name() {
    let text = "string{{ x }}";
    let span = assert_refusal(&refused_type(text), "part of a type name", true);
    assert_eq!(&text[span], text);
}

#[test]
fn refused_before_a_type_name() {
    let span = assert_refusal(&refused_type("{{ x }}string"), "part of a type name", true);
    assert_eq!(span.start, 0);
}

#[test]
fn refused_before_an_import_suffix() {
    let span = assert_refusal(&refused_type("{{ x }}@./f.yaml"), "part of a type name", true);
    assert_eq!(span.start, 0);
}

#[test]
fn refused_in_a_tuple_spread() {
    let text = "tuple([number, ...{{ x }}[]])";
    let span = assert_refusal(&refused_type(text), "a tuple spread (`...T[]`)", false);
    assert_eq!(&text[span], "{{ x }}");
}

#[test]
fn refused_as_a_function_parameter() {
    let text = "function(parameters(x: {{ t }}); returns(boolean))";
    let span = assert_refusal(&refused_type(text), "a `function(...)` parameter", false);
    assert_eq!(&text[span], "{{ t }}");
}

#[test]
fn refused_as_a_function_return() {
    let text = "function(parameters(x: string); returns({{ t }}))";
    let span = assert_refusal(&refused_type(text), "a `function(...)` return type", false);
    assert_eq!(&text[span], "{{ t }}");
}

#[test]
fn refused_as_a_narrows_target() {
    let text = "function(parameters(x: unknown); returns(boolean); narrows(x: {{ t }}; true-iff))";
    let span = assert_refusal(&refused_type(text), "a `function(...)` `narrows(...)` target", false);
    assert_eq!(&text[span], "{{ t }}");
}

#[test]
fn refused_in_a_property_declaration() {
    let text = "schema(property(kind, {{ t }}))";
    let span = assert_refusal(&refused_type(text), "a `property(...)` declaration", false);
    assert_eq!(&text[span], "{{ t }}");
}

// ── positions in the document ────────────────────────────────────────────

/// The slots of `yaml` parsed with source projection, `offset` bytes into
/// a containing document.
fn projected_slots(yaml_text: &str, offset: usize) -> Vec<Slot> {
    let root = parse_yaml_schema_with_source(&yaml(yaml_text), yaml_text, offset)
        .unwrap_or_else(|error| panic!("{error}\n{yaml_text}"));
    collect_slots(&root, &DefinitionTable::default())
}

/// The slots of a document as the factory reads it.
fn document_slots(text: &str) -> Vec<Slot> {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("{error}\n{text}")).pending_slots().to_vec()
}

#[test]
fn a_plain_scalar_template_type_points_at_its_braces() {
    // A plain scalar cannot start with `{{` (YAML reads a flow mapping), so
    // the template type follows a union arm.
    let text = "status: string | {{ t }}[](required)\n";
    let slots = projected_slots(text, 0);
    assert_eq!(slots.len(), 1, "{slots:?}");
    assert_slot_text(text, &slots[0], "{{ t }}", "t");
}

#[test]
fn quoted_template_types_project_through_their_escapes() {
    let single = "status: '{{ load_data(''./s.yaml'') }}[](required)'\n";
    let slots = projected_slots(single, 0);
    assert_eq!(slots[0].expression, "load_data('./s.yaml')", "the decoded expression");
    assert_slot_text(single, &slots[0], "{{ load_data(''./s.yaml'') }}", "load_data(''./s.yaml'')");

    let double = "status: \"string(default(\\\"a\\\")) | {{ pick(\\\"x\\\") }}\"\n";
    let slots = projected_slots(double, 0);
    assert_eq!(slots[0].expression, "pick(\"x\")");
    assert_slot_text(double, &slots[0], "{{ pick(\\\"x\\\") }}", "pick(\\\"x\\\")");
}

#[test]
fn a_union_in_one_scalar_and_tuple_elements_project() {
    let text = "either: 'string | {{ a }}'\npair: 'tuple([number, {{ b }}, {{ c }}?])'\ngrouped: '({{ d }} | string)[]'\n";
    let found: Vec<(String, &str)> =
        projected_slots(text, 0).iter().map(|slot| (slot.expression.clone(), &text[slot.span.clone()])).collect();
    let expected: Vec<(String, &str)> =
        [("a", "{{ a }}"), ("b", "{{ b }}"), ("c", "{{ c }}"), ("d", "{{ d }}")].map(|(e, t)| (e.to_string(), t)).to_vec();
    assert_eq!(found, expected);
}

#[test]
fn a_yaml_list_arm_projects() {
    let text = "$schema:\n  fallback:\n    - string\n    - '{{ f }}'\n";
    assert_slot_text(text, &document_slots(text)[0], "{{ f }}", "f");
}

#[test]
fn block_scalar_template_types_keep_authored_positions() {
    let literal = "$schema:\n  status: |\n    {{ t }}[](required)\n";
    assert_slot_text(literal, &document_slots(literal)[0], "{{ t }}", "t");

    let folded = "$schema:\n  status: >\n    string |\n    {{ u }}\n";
    assert_slot_text(folded, &document_slots(folded)[0], "{{ u }}", "u");
}

#[test]
fn markdown_frontmatter_template_types_point_into_the_document() {
    let markdown = "---\ntitle: t\n$schema:\n  status: \"{{ t }}[]\"\n---\n# Body {{ not_a_slot }}\n";
    let slots = document_slots(markdown);
    assert_eq!(slots.len(), 1, "body expressions are not schema slots");
    assert_slot_text(markdown, &slots[0], "{{ t }}", "t");

    // The YAML-offset form a host uses for a frontmatter block.
    let block = "status: '{{ t }}[]'\n";
    let document = format!("---\n{block}---\n");
    assert_slot_text(&document, &projected_slots(block, 4)[0], "{{ t }}", "t");
}

#[test]
fn crlf_line_endings_keep_authored_positions() {
    let crlf = "$schema:\r\n  first: string\r\n  status: '{{ t }}(required)'\r\n";
    assert_slot_text(crlf, &document_slots(crlf)[0], "{{ t }}", "t");
    let crlf = "first: string\r\nstatus: 'string | {{ t }}'\r\n";
    assert_slot_text(crlf, &projected_slots(crlf, 0)[0], "{{ t }}", "t");
    let markdown = "---\r\n$schema:\r\n  status: \"{{ t }}\"\r\n---\r\n";
    assert_slot_text(markdown, &document_slots(markdown)[0], "{{ t }}", "t");
}

#[test]
fn a_factory_built_fixture_spans_every_template_type_in_its_text() {
    let text = include_str!("../fixtures/dynamic_slots/template-types/properties.yaml");
    let schema = SimplifiedSchema::try_from(text).unwrap();
    for slot in schema.pending_slots() {
        assert_slot_text(text, slot, &format!("{{{{ {} }}}}", slot.expression), &slot.expression);
    }
}

#[cfg(feature = "file")]
#[test]
fn a_fixture_read_from_its_file_stamps_the_defining_file() {
    use std::path::Path;

    use biscuit_file::FileResolutionContext;
    use simplified_schema::SchemaSource;

    let fixtures = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/template-types");
    let entry = fixtures.join("properties.yaml");
    let schema = SimplifiedSchema::try_from_yaml(SchemaSource::from(&entry).bound(FileResolutionContext::new(&fixtures)))
        .expect("a template file is admitted");
    assert!(!schema.is_resolved());
    for slot in schema.pending_slots() {
        let origin = slot.origin.as_deref().unwrap_or_else(|| panic!("`{}` has a defining file", slot.expression));
        assert_eq!(origin.file_name(), Path::new("properties.yaml").file_name(), "{slot:?}");
    }
    let provenance = schema.provenance();
    assert!(provenance.imports().is_empty(), "{:?}", provenance.imports());
    assert!(
        provenance.referenced_files().iter().chain(provenance.searched()).all(|file| !file.ends_with("status-type.yaml")),
        "a slot's target is not read while parsing"
    );
}

#[test]
fn a_refused_template_type_is_located_at_its_slot_in_the_document() {
    use simplified_schema::simplified::parse_schema_declaration_with_source;

    for text in [
        "title: string\nitems: 'tuple([number, ...{{ x }}[]])'\n",
        "title: string\nitems: \"function(parameters(s: string(default(\\\"a\\\"))); returns({{ x }}))\"\n",
        "title: string\r\nitems: |\r\n  tuple([number, ...{{ x }}[]])\r\n",
    ] {
        let failure = match parse_schema_declaration_with_source(&yaml(text), text, 0) {
            Err(failure) => failure,
            Ok(_) => panic!("refused: {text:?}"),
        };
        assert!(failure.error.to_string().contains(NOT_ALLOWED), "{}", failure.error);
        let span = failure.authored_span.unwrap_or_else(|| panic!("the source proves the slot's place: {text:?}"));
        assert_eq!(&text[span], "{{ x }}", "{text:?}");
    }
}
