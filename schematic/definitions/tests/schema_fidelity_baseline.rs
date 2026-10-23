//! Schema fidelity of the schemars-to-OpenAPI bridge over every shipped
//! registry (spec `2026-10-18-schematic-fidelity-polish`, R5 and R6).
//!
//! Each schema position the export keeps (component roots, properties, array
//! items, composition members, and map values) is paired with its schemars
//! source and classified. An untyped output is deliberate free-form when the
//! source carried no constraint, as `serde_json::Value` produces, and lost
//! otherwise; a typed output that dropped a source constraint is a typed
//! loss. The table prints with `--no-capture`. The assertions hold before and
//! after the bridge is fixed; the lost-count gate is R5's, not this file's.

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

/// The source constraints a typed `output` dropped.
fn typed_losses(source: &Value, output: &Value) -> Vec<&'static str> {
    let (Some(source), Some(output)) = (source.as_object(), output.as_object()) else { return vec![] };
    let mut dropped = Vec::new();
    let has = |key: &str| output.contains_key(key);
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
    if COMPOSITIONS.iter().any(|key| source.contains_key(*key)) && !COMPOSITIONS.iter().any(|key| has(key)) {
        dropped.push("composition");
    }
    if source.get("required").and_then(Value::as_array).is_some_and(|required| !required.is_empty())
        && output.get("required") != source.get("required")
    {
        dropped.push("required");
    }
    if matches!(source.get("type"), Some(Value::Array(types)) if types.iter().any(|kind| kind == "null"))
        && output.get("nullable") != Some(&Value::Bool(true))
    {
        dropped.push("nullable");
    }
    for key in BOUNDS {
        if source.contains_key(*key) && !has(key) {
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
    for keyword in COMPOSITIONS {
        let (Some(members), Some(kept)) =
            (source.get(keyword).and_then(Value::as_array), output.get(keyword).and_then(Value::as_array))
        else {
            continue;
        };
        for (index, (member, kept)) in members.iter().zip(kept).enumerate() {
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

/// Prints the R6 table for every shipped registry and checks the durable
/// facts: every registry exports and the walk reaches properties.
#[test]
fn every_registry_exports_and_reports_its_schema_fidelity() {
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
    println!("\nlost components and nested positions ({}):\n{}", lost_elsewhere.len(), lost_elsewhere.join("\n"));
    println!("\ndeliberate free-form positions ({}):\n{}", deliberate.len(), deliberate.join("\n"));

    assert!(all_properties.total > 0, "the walk reaches component properties");
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
