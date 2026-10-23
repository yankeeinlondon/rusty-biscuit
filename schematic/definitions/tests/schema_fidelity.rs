//! Schema fidelity of the schemars-to-OpenAPI bridge over every shipped
//! registry (spec `2026-10-18-schematic-fidelity-polish`, R5 and R6).
//!
//! Each schema position the export keeps (component roots, properties, array
//! items, composition members, and map values) is paired with its schemars
//! source and classified. An untyped output is deliberate free-form when the
//! source carried no constraint, as `serde_json::Value` produces, and lost
//! otherwise; a typed output that dropped a source constraint is a typed
//! loss. A shipped registry with any lost or typed-loss position fails; the
//! table and the deliberate free-form positions print with `--no-capture`.
//!
//! Every component is scanned as a root, so a `$ref` is compared as a
//! reference and never followed.

use std::collections::BTreeMap;

use schematic_define::openapi::{ExportOptions, export};
use schematic_define::{DefinitionContext, RestApi};
use serde_json::{Map, Value};

/// Source keys that describe a schema without constraining its values.
const ANNOTATIONS: &[&str] =
    &["description", "title", "default", "examples", "deprecated", "readOnly", "writeOnly", "$comment"];

/// Output keys that give an OpenAPI schema a type or a shape.
const TYPING: &[&str] =
    &["type", "$ref", "allOf", "oneOf", "anyOf", "not", "enum", "properties", "items", "additionalProperties"];

/// Value constraints compared key for key between source and output.
const BOUNDS: &[&str] = &[
    "format", "pattern", "minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf",
    "minLength", "maxLength", "minItems", "maxItems", "uniqueItems", "minProperties", "maxProperties", "default",
];

const COMPOSITIONS: [&str; 3] = ["oneOf", "anyOf", "allOf"];

/// Where a schema position sits in its parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    Component,
    Property,
    Item,
    Member,
    MapValue,
}

/// The fidelity of one exported schema position.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Fidelity {
    /// Typed and every source constraint kept.
    Kept,
    /// Typed, but these source constraints were dropped.
    TypedLoss(Vec<&'static str>),
    /// Untyped because the source was free-form.
    Deliberate,
    /// Untyped although the source carried these constraints.
    Lost(Vec<&'static str>),
}

#[derive(Debug)]
struct Slot {
    path: String,
    position: Position,
    fidelity: Fidelity,
}

/// Whether `source` admits any value: `true`, `{}`, or annotations only.
fn is_free_form(source: &Value) -> bool {
    match source {
        Value::Bool(admits) => *admits,
        Value::Object(object) => object.keys().all(|key| ANNOTATIONS.contains(&key.as_str())),
        _ => false,
    }
}

/// Whether the exported `output` carries no type or shape.
fn is_untyped(output: &Value) -> bool {
    match output {
        Value::Object(object) => !TYPING.iter().any(|key| object.contains_key(*key)),
        _ => true,
    }
}

/// The typing keywords `source` carried, as named in R5.
fn source_constraints(source: &Value) -> Vec<&'static str> {
    let Some(object) = source.as_object() else { return vec![] };
    let mut carried = Vec::new();
    match object.get("type") {
        Some(Value::Array(types)) if types.iter().any(|kind| kind == "null") => carried.push("nullable type array"),
        Some(Value::Array(_)) => carried.push("type array"),
        Some(_) => carried.push("type"),
        None => {}
    }
    for key in ["$ref", "enum", "const", "oneOf", "anyOf", "allOf", "additionalProperties", "properties", "items"] {
        if object.contains_key(key) {
            carried.push(key);
        }
    }
    if carried.is_empty() && !is_free_form(source) {
        carried.push("other keyword");
    }
    carried
}

/// Whether `schema` admits only `null`, in any spelling the source or the
/// export uses.
fn admits_only_null(schema: &Value) -> bool {
    let Some(object) = schema.as_object() else { return false };
    let only_null = |value: &Value| value.as_array().is_some_and(|values| !values.is_empty() && values.iter().all(Value::is_null));
    (object.get("type").is_some_and(|kind| kind == "null") || object.get("enum").is_some_and(only_null) || object.get("const") == Some(&Value::Null))
        && object.keys().all(|key| ANNOTATIONS.contains(&key.as_str()) || ["type", "nullable", "enum", "const"].contains(&key.as_str()))
}

/// The non-null members of `schema`'s `keyword` composition.
fn members<'a>(schema: &'a Map<String, Value>, keyword: &str) -> Vec<&'a Value> {
    schema.get(keyword).and_then(Value::as_array).into_iter().flatten().filter(|member| !admits_only_null(member)).collect()
}

/// Whether the source admits `null`.
fn source_nullable(source: &Map<String, Value>) -> bool {
    matches!(source.get("type"), Some(Value::Array(types)) if types.iter().any(|kind| kind == "null"))
        || source.get("nullable") == Some(&Value::Bool(true))
        || ["anyOf", "oneOf"].iter().any(|keyword| source.get(*keyword).and_then(Value::as_array).is_some_and(|m| m.iter().any(admits_only_null)))
}

/// Whether the output admits `null`: `nullable`, or a null member.
fn output_nullable(output: &Map<String, Value>) -> bool {
    output.get("nullable") == Some(&Value::Bool(true))
        || output.get("anyOf").and_then(Value::as_array).is_some_and(|members| members.iter().any(admits_only_null))
}

/// The sole non-null member of a source `anyOf`/`oneOf` that the export
/// folded into `output` (a null member removed, one member left).
fn folded_member<'a>(source: &'a Map<String, Value>, output: &Map<String, Value>) -> Option<&'a Value> {
    ["anyOf", "oneOf"].into_iter().find_map(|keyword| {
        let remaining = members(source, keyword);
        let removed_null = source.get(keyword).and_then(Value::as_array).is_some_and(|all| all.len() > remaining.len());
        (removed_null && remaining.len() == 1 && !output.contains_key(keyword)).then(|| remaining[0])
    })
}

/// The source constraints a typed `output` dropped. A keyword moved into the
/// output's `anyOf` members (a split `type` array) is kept.
fn typed_losses(source: &Value, output: &Value) -> Vec<&'static str> {
    let (Some(source), Some(output)) = (source.as_object(), output.as_object()) else { return vec![] };
    let mut dropped = Vec::new();
    let in_members = |key: &str| members(output, "anyOf").iter().any(|member| member.get(key).is_some());
    let has = |key: &str| output.contains_key(key) || in_members(key);
    if source.contains_key("enum") && !has("enum") {
        dropped.push("enum");
    }
    if source.contains_key("const") && !has("enum") {
        dropped.push("const");
    }
    if source.contains_key("additionalProperties") && !has("additionalProperties") {
        dropped.push("additionalProperties");
    }
    if source.contains_key("items") && !has("items") {
        dropped.push("items");
    }
    let composed = COMPOSITIONS.iter().any(|key| !members(source, key).is_empty());
    if composed && !COMPOSITIONS.iter().any(|key| output.contains_key(*key)) && folded_member(source, output).is_none() {
        dropped.push("composition");
    }
    if source.get("required").and_then(Value::as_array).is_some_and(|required| !required.is_empty())
        && output.get("required") != source.get("required")
        && !in_members("required")
    {
        dropped.push("required");
    }
    if source_nullable(source) && !output_nullable(output) {
        dropped.push("nullable");
    }
    // A reference survives bare, or as a member wrapping it for its siblings.
    let references = |schema: &Map<String, Value>| {
        let mut found: Vec<Value> = schema.get("$ref").cloned().into_iter().collect();
        for keyword in COMPOSITIONS {
            found.extend(members(schema, keyword).into_iter().filter_map(|member| member.get("$ref").cloned()));
        }
        found
    };
    if let Some(reference) = source.get("$ref")
        && !references(output).contains(reference)
    {
        dropped.push("$ref");
    }
    let kinds = |schema: &Map<String, Value>| -> Vec<Value> {
        let mut kinds: Vec<Value> = match schema.get("type") {
            Some(Value::Array(kinds)) => kinds.clone(),
            Some(kind) => vec![kind.clone()],
            None => vec![],
        };
        kinds.extend(members(schema, "anyOf").into_iter().filter_map(|member| member.get("type").cloned()));
        kinds.retain(|kind| kind != "null");
        kinds
    };
    let output_kinds = kinds(output);
    if kinds(source).iter().any(|kind| !output_kinds.contains(kind)) {
        dropped.push("type");
    }
    for key in BOUNDS {
        // `openapiv3` reads `"default": null` as no default; a nullable output
        // still says absence means null.
        let null_default = *key == "default" && source.get("default") == Some(&Value::Null) && output_nullable(output);
        if source.contains_key(*key) && !has(key) && !null_default {
            dropped.push(key);
        }
    }
    dropped
}

/// Classifies one exported position against its schemars source.
fn classify(source: &Value, output: &Value) -> Fidelity {
    if is_untyped(output) {
        return if is_free_form(source) { Fidelity::Deliberate } else { Fidelity::Lost(source_constraints(source)) };
    }
    let dropped = typed_losses(source, output);
    if dropped.is_empty() { Fidelity::Kept } else { Fidelity::TypedLoss(dropped) }
}

/// Classifies `source`/`output` at `path` and every child position the output
/// kept; an untyped position's children are part of its loss.
fn walk(source: &Value, output: &Value, path: String, position: Position, slots: &mut Vec<Slot>) {
    let fidelity = classify(source, output);
    let untyped = matches!(fidelity, Fidelity::Deliberate | Fidelity::Lost(_));
    slots.push(Slot { path: path.clone(), position, fidelity });
    let (Some(source), Some(output)) = (source.as_object(), output.as_object()) else { return };
    if untyped {
        return;
    }
    let empty = Map::new();
    let output_properties = output.get("properties").and_then(Value::as_object).unwrap_or(&empty);
    for (name, child) in source.get("properties").and_then(Value::as_object).unwrap_or(&empty) {
        let kept = output_properties.get(name).unwrap_or(&Value::Null);
        walk(child, kept, format!("{path}.{name}"), Position::Property, slots);
    }
    if let (Some(child), Some(kept)) = (source.get("items"), output.get("items")) {
        walk(child, kept, format!("{path}[]"), Position::Item, slots);
    }
    if let Some(member) = folded_member(source, output) {
        walk(member, &Value::Object(output.clone()), format!("{path}|folded"), Position::Member, slots);
    }
    for keyword in COMPOSITIONS {
        // Null members are the position's nullability, checked above.
        for (index, (member, kept)) in members(source, keyword).into_iter().zip(members(output, keyword)).enumerate() {
            walk(member, kept, format!("{path}|{keyword}[{index}]"), Position::Member, slots);
        }
    }
    if let (Some(child @ Value::Object(_)), Some(kept)) = (source.get("additionalProperties"), output.get("additionalProperties")) {
        walk(child, kept, format!("{path}{{}}"), Position::MapValue, slots);
    }
}

/// Exports `api` and classifies every position of every component.
fn scan(api: &RestApi) -> Vec<Slot> {
    let document = serde_json::to_value(export(api, &ExportOptions::new()).expect("registry exports"))
        .expect("OpenAPI document serializes");
    let exported = &document["components"]["schemas"];
    let mut slots = Vec::new();
    for component in &api.schemas {
        let output = exported.get(&component.name).unwrap_or(&Value::Null);
        walk(&component.schema, output, component.name.clone(), Position::Component, &mut slots);
    }
    slots
}

#[derive(Debug, Default)]
struct Tally {
    total: usize,
    untyped: usize,
    deliberate: usize,
    lost: usize,
    typed_loss: usize,
    /// Typed losses that dropped more than a `default`.
    typed_loss_beyond_default: usize,
}

impl Tally {
    fn absorb(&mut self, other: &Tally) {
        self.total += other.total;
        self.untyped += other.untyped;
        self.deliberate += other.deliberate;
        self.lost += other.lost;
        self.typed_loss += other.typed_loss;
        self.typed_loss_beyond_default += other.typed_loss_beyond_default;
    }

    fn add(&mut self, fidelity: &Fidelity) {
        self.total += 1;
        match fidelity {
            Fidelity::Kept => {}
            Fidelity::TypedLoss(dropped) => {
                self.typed_loss += 1;
                self.typed_loss_beyond_default += usize::from(dropped.iter().any(|key| *key != "default"));
            }
            Fidelity::Deliberate => {
                self.untyped += 1;
                self.deliberate += 1;
            }
            Fidelity::Lost(_) => {
                self.untyped += 1;
                self.lost += 1;
            }
        }
    }
}

fn columns(tally: &Tally) -> String {
    format!(
        "{} | {} | {} | {} | {} ({})",
        tally.total, tally.untyped, tally.deliberate, tally.lost, tally.typed_loss, tally.typed_loss_beyond_default
    )
}

/// Prints the R6 table for every shipped registry and fails on any lost or
/// typed-loss position (R5, R6).
#[test]
fn every_registry_exports_without_losing_a_schema_constraint() {
    let apis = schematic_definitions::all_apis();
    assert_eq!(apis.len(), 18, "the R6 baseline covers all 18 shipped registries");

    let mut report = vec![
        "| API | props | untyped | deliberate | lost | typed-loss (beyond default) | other positions: total, untyped, deliberate, lost, typed-loss | components lost |".to_string(),
        "|---|---|---|---|---|---|---|---|".to_string(),
    ];
    let mut all_properties = Tally::default();
    let mut all_others = Tally::default();
    let mut all_components = Tally::default();
    let mut lost_reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut typed_reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut samples = Vec::new();
    let mut deliberate = Vec::new();
    let mut lost_elsewhere = Vec::new();
    for api in &apis {
        let slots = scan(api);
        let mut properties = Tally::default();
        let mut others = Tally::default();
        let mut components = Tally::default();
        let mut api_samples = Vec::new();
        for slot in &slots {
            match slot.position {
                Position::Property => properties.add(&slot.fidelity),
                Position::Component => components.add(&slot.fidelity),
                _ => others.add(&slot.fidelity),
            }
            match &slot.fidelity {
                Fidelity::Lost(reasons) => {
                    reasons.iter().for_each(|reason| *lost_reasons.entry(reason).or_default() += 1);
                    if slot.position != Position::Property {
                        lost_elsewhere.push(format!("{}: {} ({})", api.name, slot.path, reasons.join(", ")));
                    } else if api_samples.len() < 5 {
                        api_samples.push(format!("{} ({})", slot.path, reasons.join(", ")));
                    }
                }
                Fidelity::TypedLoss(reasons) => {
                    reasons.iter().for_each(|reason| *typed_reasons.entry(reason).or_default() += 1);
                    lost_elsewhere.push(format!("{}: {} typed loss ({})", api.name, slot.path, reasons.join(", ")));
                }
                Fidelity::Deliberate => deliberate.push(format!("{}: {}", api.name, slot.path)),
                Fidelity::Kept => {}
            }
        }
        report.push(format!(
            "| {} | {} | {} | {} of {} |",
            api.name,
            columns(&properties),
            columns(&others).replace(" | ", ", "),
            components.lost,
            components.total,
        ));
        samples.push(format!("{}: {}", api.name, api_samples.join("; ")));
        all_properties.absorb(&properties);
        all_others.absorb(&others);
        all_components.absorb(&components);
    }
    report.push(format!(
        "| **all (per-API sum)** | {} | {} | {} of {} |",
        columns(&all_properties),
        columns(&all_others).replace(" | ", ", "),
        all_components.lost,
        all_components.total,
    ));

    println!("{}", report.join("\n"));
    println!("\nlost-position reasons (all positions): {lost_reasons:?}");
    println!("typed-loss reasons (all positions): {typed_reasons:?}");
    println!("\nsample lost properties:\n{}", samples.join("\n"));
    println!("\nlost components, lost nested positions, and typed losses ({}):\n{}", lost_elsewhere.len(), lost_elsewhere.join("\n"));
    println!("\ndeliberate free-form positions ({}):\n{}", deliberate.len(), deliberate.join("\n"));

    assert!(all_properties.total > 0, "the walk reaches component properties");
    let lost = all_properties.lost + all_others.lost + all_components.lost;
    let typed = all_properties.typed_loss + all_others.typed_loss + all_components.typed_loss;
    assert_eq!((lost, typed), (0, 0), "lost and typed-loss positions: {lost_reasons:?} {typed_reasons:?}\n{}", lost_elsewhere.join("\n"));
}

schematic_define::definition_type! {
    /// A fixture whose `payload` is free-form and whose `name` is typed.
    #[derive(schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
    #[allow(dead_code)]
    struct Envelope {
        name: String,
        payload: serde_json::Value,
    }
}

#[test]
fn a_serde_json_value_field_exports_as_deliberate_free_form() {
    let mut context = DefinitionContext::types_in("::fixture");
    context.register::<Envelope>().unwrap();
    let api = RestApi::builder(&context, "Fixture", "https://fixture.test").describes("Fixture").no_auth().build().unwrap();

    let slots = scan(&api);
    let fidelity_of = |path: &str| &slots.iter().find(|slot| slot.path == path).expect(path).fidelity;
    assert_eq!(fidelity_of("Envelope.payload"), &Fidelity::Deliberate);
    assert_eq!(fidelity_of("Envelope.name"), &Fidelity::Kept);
}

#[test]
fn an_untyped_output_of_a_constrained_source_is_lost() {
    let source = serde_json::json!({"type": ["string", "null"], "description": "x"});
    assert_eq!(classify(&source, &serde_json::json!({"description": "x"})), Fidelity::Lost(vec!["nullable type array"]));
    let enumerated = serde_json::json!({"enum": ["a", "b"]});
    assert_eq!(classify(&enumerated, &serde_json::json!({})), Fidelity::Lost(vec!["enum"]));
}

#[test]
fn a_typed_output_that_drops_a_constraint_is_a_typed_loss() {
    let source = serde_json::json!({"type": "string", "enum": ["a"], "format": "uuid"});
    assert_eq!(classify(&source, &serde_json::json!({"type": "string"})), Fidelity::TypedLoss(vec!["enum", "format"]));
    assert_eq!(classify(&source, &source), Fidelity::Kept);
    assert_eq!(classify(&serde_json::json!(true), &serde_json::json!({})), Fidelity::Deliberate);
}


/// Generic instantiations whose schemars name keeps the Rust spelling. They
/// are struct components, not containers, but R53 also forbids `<` in a key
/// (and OpenAPI 3.0.3 restricts keys to `^[a-zA-Z0-9.\-_]+$`); renaming them
/// is a definitions change outside this bridge, recorded for review.
const GENERIC_COMPONENT_NAMES: [&str; 9] = [
    "PaginatedResponse<SourceEntry>", "PaginatedResponse<PullRequest>", "PaginatedResponse<PullRequestComment>",
    "PaginatedResponse<Issue>", "PaginatedResponse<IssueComment>", "PaginatedResponse<IssueChange>",
    "PaginatedResponse<Tag>", "PaginatedResponse<Download>", "PaginatedResponse<Repository>",
];

#[test]
fn every_registry_closes_its_references_and_names_no_container_component() {
    let mut generic = Vec::new();
    for api in schematic_definitions::all_apis() {
        // API build already ran the closure; this is the backstop's own pass.
        api.validate_schema_closure().unwrap_or_else(|error| panic!("{}: {error}", api.name));
        let document = serde_json::to_value(export(&api, &ExportOptions::new()).expect("registry exports")).unwrap();
        for name in document["components"]["schemas"].as_object().expect("components.schemas").keys() {
            let container = ["Array_of", "Map_of", "Nullable_", "Vec<", "Option<", "HashMap<", "BTreeMap<", "IndexMap<"]
                .iter()
                .any(|prefix| name.starts_with(prefix));
            assert!(!container, "{}: container component {name}", api.name);
            if name.contains('<') {
                generic.push((api.name.clone(), name.clone()));
            }
        }
    }
    let mut expected: Vec<_> = GENERIC_COMPONENT_NAMES.iter().map(|name| ("Bitbucket".to_string(), name.to_string())).collect();
    expected.sort();
    generic.sort();
    assert_eq!(generic, expected);
}
