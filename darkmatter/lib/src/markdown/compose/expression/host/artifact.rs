//! The host facts artifact: the JSON companion of a host function catalog
//! that carries the Rust-side registration facts to a process that cannot
//! run the host's registration (the editor).
//!
//! The writer ([`HostFunctions::facts_artifact`]) emits deterministic bytes
//! (keys sorted at every level, pretty-printed, one trailing newline). The
//! reader ([`HostFunctions::analysis_only`]) treats the file as inert data
//! and rebuilds every registration through the same [`RegistrationBuilder`]
//! from the catalog's signatures, so a fact the catalog cannot carry fails
//! exactly as it would in the host. It is strict: a duplicate key, trailing
//! content, an unknown field, or a field that is absent, `null`, or of the
//! wrong type is never defaulted. A problem with the whole file makes every
//! catalog function incomplete; a problem with one function's entry makes
//! that function incomplete.
//!
//! ```json
//! {
//!   "catalog_hash": "<BLAKE3 hex of the catalog's bytes>",
//!   "format_version": 1,
//!   "functions": {
//!     "is_project_file": {
//!       "aliases": [],
//!       "conversion_recovery": null,
//!       "facts": [{ "kind": "true_implies", "parameter": "value", "position": 0, "type": "file" }],
//!       "lazy": false,
//!       "malformed_reference": null,
//!       "source_exclusions": []
//!     }
//!   }
//! }
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value, json};

use super::{FunctionState, HostFunction, HostFunctionCatalog, HostFunctionError, HostFunctions, validate_names};
use crate::markdown::coercion::ValueKind;
use crate::markdown::compose::expression::call_binding::{
    EvaluationMode, FactKind, FactType, FunctionRegistration, RegistrationBuilder,
};

/// The artifact format this library writes and reads.
pub const FACTS_ARTIFACT_FORMAT_VERSION: u64 = 1;

/// A fact's `type` for [`FactType::NonNull`]; no type text has this form.
pub const NON_NULL_FACT_TYPE: &str = "<non-null>";

const TOP_FIELDS: &[&str] = &["catalog_hash", "format_version", "functions"];
const FUNCTION_FIELDS: &[&str] = &[
    "aliases",
    "conversion_recovery",
    "facts",
    "lazy",
    "malformed_reference",
    "source_exclusions",
];
const FACT_FIELDS: &[&str] = &["kind", "parameter", "position", "type"];
const RECOVERY_FIELDS: &[&str] = &["fallback", "subject"];
const EXCLUSION_FIELDS: &[&str] = &["kinds", "parameter"];
const REFERENCE_FIELDS: &[&str] = &["parameter", "position"];

/// The artifact beside `catalog`: its file stem plus `.facts.json`
/// (`functions.yaml` → `functions.facts.json`).
pub fn facts_artifact_path(catalog: &Path) -> PathBuf {
    catalog.with_extension("facts.json")
}

/// The facts artifact as the reader received it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactsArtifact<'a> {
    /// The file's text.
    Text(&'a str),
    /// No file beside the catalog.
    Absent,
    /// The file exists but could not be read; the reason.
    Unreadable(&'a str),
}

impl HostFunctions {
    /// The facts artifact for this set, from its registrations: per function
    /// its aliases, narrowing facts, laziness, conversion recovery, source
    /// exclusions, and malformed-reference rule, plus the catalog's version
    /// and [`FACTS_ARTIFACT_FORMAT_VERSION`]. Incomplete functions are
    /// omitted. The bytes depend only on the registrations and the catalog.
    pub fn facts_artifact(&self) -> String {
        let mut functions = Map::new();
        for function in &self.inner.functions {
            if let FunctionState::Complete { registration, .. } = &function.state {
                functions.insert(function.canonical.clone(), function_entry(function, registration));
            }
        }
        let root = json!({
            "catalog_hash": self.inner.catalog.version(),
            "format_version": FACTS_ARTIFACT_FORMAT_VERSION,
            "functions": Value::Object(functions),
        });
        let mut text = serde_json::to_string_pretty(&sorted(root)).expect("a JSON value serializes");
        text.push('\n');
        text
    }

    /// An analysis-only set (no handlers) for `catalog`, with each function's
    /// facts read from `artifact`. A function whose facts cannot be trusted
    /// is incomplete, with a human-readable reason; see the module docs.
    ///
    /// ## Errors
    ///
    /// [`HostFunctionError::BuiltinCollision`] when the catalog declares a
    /// function a built-in already names; the artifact is not consulted.
    pub fn analysis_only(catalog: HostFunctionCatalog, artifact: FactsArtifact<'_>) -> Result<Self, HostFunctionError> {
        let names = catalog.function_names();
        validate_names(names.iter().map(|name| (*name, &[][..])))?;
        let path = facts_artifact_path(catalog.path());
        let whole = |reason: String| -> Vec<HostFunction> {
            names.iter().map(|name| incomplete(name, reason.clone())).collect()
        };
        let functions = match read_entries(&catalog, &names, artifact, &path) {
            Err(reason) => whole(reason),
            Ok(entries) => {
                let functions: Vec<HostFunction> = names
                    .iter()
                    .map(|name| match entries.get(*name) {
                        None => incomplete(name, "the facts artifact has no entry for it".to_string()),
                        Some(entry) => match rebuild(&catalog, name, entry) {
                            Ok((aliases, registration)) => HostFunction {
                                canonical: (*name).to_string(),
                                aliases,
                                state: FunctionState::Complete {
                                    registration: Box::new(registration),
                                    handler: None,
                                },
                            },
                            Err(reason) => incomplete(name, format!("its facts artifact entry {reason}")),
                        },
                    })
                    .collect();
                match validate_names(functions.iter().map(|function| (function.canonical.as_str(), function.aliases.as_slice()))) {
                    Ok(()) => functions,
                    Err(error) => whole(format!("the facts artifact `{}` is invalid: {error}", path.display())),
                }
            }
        };
        Ok(Self::new(catalog, functions))
    }
}

fn incomplete(name: &str, reason: String) -> HostFunction {
    HostFunction {
        canonical: name.to_string(),
        aliases: Vec::new(),
        state: FunctionState::Incomplete { reason },
    }
}

// ── writer ──────────────────────────────────────────────────────────────────

fn function_entry(function: &HostFunction, registration: &FunctionRegistration) -> Value {
    let mut aliases = function.aliases.clone();
    aliases.sort();
    let facts: Vec<Value> = registration
        .narrowing_fact()
        .map(|fact| {
            json!({
                "parameter": fact.parameter(),
                "position": fact.index(),
                "kind": kind_text(fact.kind()),
                "type": match fact.fact_type() {
                    FactType::NonNull => NON_NULL_FACT_TYPE.to_string(),
                    FactType::Type(text) => text.to_string(),
                },
            })
        })
        .into_iter()
        .collect();
    let recovery = registration.numeric_recovery().map_or(Value::Null, |recovery| {
        json!({ "subject": recovery.subject, "fallback": recovery.fallback })
    });
    // Exclusions apply to every overload declaring the parameter, so the
    // first declaration found carries them.
    let mut exclusions: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for signature in registration.overloads() {
        for parameter in signature.parameters() {
            if !parameter.excluded_sources().is_empty() {
                exclusions.entry(parameter.name()).or_insert_with(|| {
                    let mut kinds: Vec<String> = parameter.excluded_sources().iter().map(ToString::to_string).collect();
                    kinds.sort();
                    kinds
                });
            }
        }
    }
    let exclusions: Vec<Value> = exclusions
        .into_iter()
        .map(|(parameter, kinds)| json!({ "parameter": parameter, "kinds": kinds }))
        .collect();
    let malformed = registration.malformed_reference_parameter().map_or(Value::Null, |position| {
        let parameter = registration.overloads()[0].parameters()[position].name();
        json!({ "parameter": parameter, "position": position })
    });
    json!({
        "aliases": aliases,
        "facts": facts,
        "lazy": registration.evaluation() == EvaluationMode::Lazy,
        "conversion_recovery": recovery,
        "source_exclusions": exclusions,
        "malformed_reference": malformed,
    })
}

fn kind_text(kind: FactKind) -> &'static str {
    match kind {
        FactKind::TrueImplies => "true_implies",
        FactKind::TrueIff => "true_iff",
        FactKind::FalseImplies => "false_implies",
    }
}

/// `value` with every object's keys in byte order (the crate's `serde_json`
/// preserves insertion order).
fn sorted(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(entries.into_iter().map(|(key, value)| (key, sorted(value))).collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        other => other,
    }
}

// ── reader ──────────────────────────────────────────────────────────────────

/// The artifact's `functions` map, after every whole-file check; `Err` is
/// the reason every function is incomplete.
fn read_entries(
    catalog: &HostFunctionCatalog,
    names: &[&str],
    artifact: FactsArtifact<'_>,
    path: &Path,
) -> Result<Map<String, Value>, String> {
    let file = format!("the facts artifact `{}`", path.display());
    let text = match artifact {
        FactsArtifact::Text(text) => text,
        FactsArtifact::Absent => return Err(format!("{file} is absent")),
        FactsArtifact::Unreadable(error) => return Err(format!("{file} cannot be read: {error}")),
    };
    let root = parse_strict(text).map_err(|error| format!("{file} {error}"))?;
    let Value::Object(root) = root else {
        return Err(format!("{file} is {} instead of an object", describe(&root)));
    };
    reject_unknown(&root, TOP_FIELDS).map_err(|error| format!("{file} {error}"))?;
    let version = required(&root, "format_version").map_err(|error| format!("{file} {error}"))?;
    match version.as_u64() {
        Some(FACTS_ARTIFACT_FORMAT_VERSION) => {}
        Some(other) => {
            return Err(format!(
                "{file} has unknown `format_version` {other}; this reader understands {FACTS_ARTIFACT_FORMAT_VERSION}"
            ));
        }
        None => return Err(format!("{file} has `format_version` {} instead of an integer", describe(version))),
    }
    let hash = string(required(&root, "catalog_hash").map_err(|error| format!("{file} {error}"))?, "catalog_hash")
        .map_err(|error| format!("{file} {error}"))?;
    if hash != catalog.version() {
        return Err(format!(
            "{file} records catalog hash `{hash}`, but the loaded catalog's hash is `{}`; regenerate the artifact",
            catalog.version()
        ));
    }
    let functions = required(&root, "functions").map_err(|error| format!("{file} {error}"))?;
    let Value::Object(functions) = functions else {
        return Err(format!("{file} has `functions` {} instead of an object", describe(functions)));
    };
    if let Some(stray) = functions.keys().find(|key| !names.contains(&key.as_str())) {
        return Err(format!("{file} has an entry for `{stray}`, which the catalog does not declare"));
    }
    Ok(functions.clone())
}

/// One function's aliases and registration, rebuilt from its entry; `Err`
/// completes "its facts artifact entry …".
fn rebuild(
    catalog: &HostFunctionCatalog,
    name: &str,
    entry: &Value,
) -> Result<(Vec<String>, FunctionRegistration), String> {
    let Value::Object(entry) = entry else {
        return Err(format!("is {} instead of an object", describe(entry)));
    };
    reject_unknown(entry, FUNCTION_FIELDS)?;

    let aliases = array(required(entry, "aliases")?, "aliases")?
        .iter()
        .enumerate()
        .map(|(index, alias)| string(alias, &format!("aliases[{index}]")).map(str::to_string))
        .collect::<Result<Vec<_>, _>>()?;

    let facts = array(required(entry, "facts")?, "facts")?;
    let mut parsed_facts = Vec::with_capacity(facts.len());
    for (index, fact) in facts.iter().enumerate() {
        parsed_facts.push(parse_fact(fact, &format!("facts[{index}]"))?);
    }
    if parsed_facts.len() > 1 {
        return Err(format!("lists {} facts; a function registers at most one", parsed_facts.len()));
    }

    let lazy = required(entry, "lazy")?;
    match lazy {
        Value::Bool(false) => {}
        Value::Bool(true) => return Err("has `lazy: true`; host functions are evaluated eagerly".to_string()),
        other => return Err(format!("has `lazy` {} instead of a boolean", describe(other))),
    }

    let recovery = match nullable(entry, "conversion_recovery")? {
        None => None,
        Some(value) => {
            let object = object(value, "conversion_recovery")?;
            reject_unknown(object, RECOVERY_FIELDS).map_err(|error| format!("`conversion_recovery` {error}"))?;
            let subject = string(required_in(object, "subject", "conversion_recovery")?, "conversion_recovery.subject")?;
            let fallback =
                string(required_in(object, "fallback", "conversion_recovery")?, "conversion_recovery.fallback")?;
            Some((subject.to_string(), fallback.to_string()))
        }
    };

    let exclusions = array(required(entry, "source_exclusions")?, "source_exclusions")?;
    let mut parsed_exclusions = Vec::with_capacity(exclusions.len());
    for (index, exclusion) in exclusions.iter().enumerate() {
        let field = format!("source_exclusions[{index}]");
        let object = object(exclusion, &field)?;
        reject_unknown(object, EXCLUSION_FIELDS).map_err(|error| format!("`{field}` {error}"))?;
        let parameter = string(required_in(object, "parameter", &field)?, &format!("{field}.parameter"))?;
        let kinds_field = format!("{field}.kinds");
        let kinds = array(required_in(object, "kinds", &field)?, &kinds_field)?
            .iter()
            .enumerate()
            .map(|(kind_index, kind)| value_kind(kind, &format!("{kinds_field}[{kind_index}]")))
            .collect::<Result<Vec<_>, _>>()?;
        if kinds.is_empty() {
            return Err(format!("has an empty `{kinds_field}`; an exclusion names at least one kind"));
        }
        parsed_exclusions.push((parameter.to_string(), kinds));
    }

    let malformed = match nullable(entry, "malformed_reference")? {
        None => None,
        Some(value) => {
            let object = object(value, "malformed_reference")?;
            reject_unknown(object, REFERENCE_FIELDS).map_err(|error| format!("`malformed_reference` {error}"))?;
            let parameter = string(required_in(object, "parameter", "malformed_reference")?, "malformed_reference.parameter")?;
            let position = position(required_in(object, "position", "malformed_reference")?, "malformed_reference.position")?;
            Some((parameter.to_string(), position))
        }
    };

    let mut builder: RegistrationBuilder =
        catalog.registration_builder(name).expect("the reader rebuilds only catalog functions");
    for (parameter, kinds) in &parsed_exclusions {
        builder = builder.reject_sources(parameter.clone(), kinds);
    }
    if let Some((subject, fallback)) = &recovery {
        builder = builder.numeric_recovery(subject.clone(), fallback.clone());
    }
    if let Some(fact) = parsed_facts.first() {
        builder = builder.narrows(fact.parameter.clone(), fact.kind, fact.ty.clone());
    }
    if let Some((parameter, _)) = &malformed {
        builder = builder.false_for_malformed_reference(parameter.clone());
    }
    let registration = catalog.build(builder).map_err(|error| format!("does not register: {error}"))?;

    if let (Some(fact), Some(built)) = (parsed_facts.first(), registration.narrowing_fact())
        && fact.position != built.index()
    {
        return Err(format!(
            "records `facts[0].position` {}, but `{}` is at position {} in the catalog",
            fact.position,
            fact.parameter,
            built.index()
        ));
    }
    if let (Some((parameter, recorded)), Some(built)) = (&malformed, registration.malformed_reference_parameter())
        && *recorded != built
    {
        return Err(format!(
            "records `malformed_reference.position` {recorded}, but `{parameter}` is at position {built} in the catalog"
        ));
    }
    Ok((aliases, registration))
}

struct ParsedFact {
    parameter: String,
    position: usize,
    kind: FactKind,
    ty: FactType,
}

fn parse_fact(value: &Value, field: &str) -> Result<ParsedFact, String> {
    let object = object(value, field)?;
    reject_unknown(object, FACT_FIELDS).map_err(|error| format!("`{field}` {error}"))?;
    let parameter = string(required_in(object, "parameter", field)?, &format!("{field}.parameter"))?;
    let position = position(required_in(object, "position", field)?, &format!("{field}.position"))?;
    let kind = match string(required_in(object, "kind", field)?, &format!("{field}.kind"))? {
        "true_implies" => FactKind::TrueImplies,
        "true_iff" => FactKind::TrueIff,
        "false_implies" => FactKind::FalseImplies,
        other => {
            return Err(format!(
                "has `{field}.kind` `{other}`; expected `true_implies`, `true_iff`, or `false_implies`"
            ));
        }
    };
    let ty = match string(required_in(object, "type", field)?, &format!("{field}.type"))? {
        NON_NULL_FACT_TYPE => FactType::NonNull,
        text => FactType::Type(text.to_string().into()),
    };
    Ok(ParsedFact {
        parameter: parameter.to_string(),
        position,
        kind,
        ty,
    })
}

fn value_kind(value: &Value, field: &str) -> Result<ValueKind, String> {
    Ok(match string(value, field)? {
        "text" => ValueKind::Text,
        "number" => ValueKind::Number,
        "boolean" => ValueKind::Boolean,
        "null" => ValueKind::Null,
        "list" => ValueKind::List,
        "object" => ValueKind::Object,
        other => {
            return Err(format!(
                "has `{field}` `{other}`; expected one of text, number, boolean, null, list, object"
            ));
        }
    })
}

// ── strict field access: absent, null, and wrong type are distinct ─────────

/// "a string", "null", …, for messages.
fn describe(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

fn reject_unknown(object: &Map<String, Value>, known: &[&str]) -> Result<(), String> {
    match object.keys().find(|key| !known.contains(&key.as_str())) {
        Some(key) => Err(format!("has unknown field `{key}`")),
        None => Ok(()),
    }
}

/// The non-null value of a top-level or entry field.
fn required<'v>(object: &'v Map<String, Value>, key: &str) -> Result<&'v Value, String> {
    match object.get(key) {
        None => Err(format!("has no `{key}` field")),
        Some(Value::Null) => Err(format!("has `{key}: null`")),
        Some(value) => Ok(value),
    }
}

/// [`required`] for a field of the nested object `owner`.
fn required_in<'v>(object: &'v Map<String, Value>, key: &str, owner: &str) -> Result<&'v Value, String> {
    match object.get(key) {
        None => Err(format!("has no `{owner}.{key}` field")),
        Some(Value::Null) => Err(format!("has `{owner}.{key}: null`")),
        Some(value) => Ok(value),
    }
}

/// A field whose explicit `null` means "none"; absence is still an error.
fn nullable<'v>(object: &'v Map<String, Value>, key: &str) -> Result<Option<&'v Value>, String> {
    match object.get(key) {
        None => Err(format!("has no `{key}` field (write `null` for none)")),
        Some(Value::Null) => Ok(None),
        Some(value) => Ok(Some(value)),
    }
}

fn string<'v>(value: &'v Value, field: &str) -> Result<&'v str, String> {
    match value {
        Value::String(text) => Ok(text),
        other => Err(format!("has `{field}` {} instead of a string", describe(other))),
    }
}

fn array<'v>(value: &'v Value, field: &str) -> Result<&'v Vec<Value>, String> {
    match value {
        Value::Array(items) => Ok(items),
        other => Err(format!("has `{field}` {} instead of an array", describe(other))),
    }
}

fn object<'v>(value: &'v Value, field: &str) -> Result<&'v Map<String, Value>, String> {
    match value {
        Value::Object(object) => Ok(object),
        other => Err(format!("has `{field}` {} instead of an object", describe(other))),
    }
}

fn position(value: &Value, field: &str) -> Result<usize, String> {
    value
        .as_u64()
        .and_then(|position| usize::try_from(position).ok())
        .ok_or_else(|| format!("has `{field}` {} instead of a non-negative integer", describe(value)))
}

// ── strict JSON ─────────────────────────────────────────────────────────────

/// Parses `text` as one JSON value, rejecting a duplicate object key
/// (`serde_json` keeps the last) and anything after the value. `Err`
/// completes "the facts artifact `…` …".
fn parse_strict(text: &str) -> Result<Value, String> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let StrictValue(value) =
        StrictValue::deserialize(&mut deserializer).map_err(|error| format!("is not valid JSON: {error}"))?;
    deserializer.end().map_err(|error| format!("has content after its JSON value: {error}"))?;
    Ok(value)
}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictVisitor).map(StrictValue)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("a number is not finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(StrictValue(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate key `{key}`")));
            }
            let StrictValue(value) = map.next_value()?;
            object.insert(key, value);
        }
        Ok(Value::Object(object))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strict_parser_rejects_duplicate_keys_at_any_depth_and_trailing_content() {
        assert_eq!(parse_strict(r#"{"a": {"b": 1}}"#), Ok(json!({"a": {"b": 1}})));
        for text in [r#"{"a": 1, "a": 2}"#, r#"{"a": {"b": 1, "b": 1}}"#, r#"[{"x": 1, "x": 1}]"#] {
            let error = parse_strict(text).unwrap_err();
            assert!(error.contains("duplicate key"), "{text}: {error}");
        }
        assert!(parse_strict("{} {}").unwrap_err().contains("after its JSON value"));
        assert!(parse_strict("{").unwrap_err().contains("not valid JSON"));
        assert_eq!(parse_strict(" {}\n\n"), Ok(json!({})));
    }

    #[test]
    fn sorting_orders_every_level() {
        let value = sorted(json!({"b": [{"z": 1, "a": 2}], "a": null}));
        assert_eq!(serde_json::to_string(&value).unwrap(), r#"{"a":null,"b":[{"a":2,"z":1}]}"#);
    }

    #[test]
    fn absent_null_and_wrong_type_read_differently() {
        let object = json!({"n": null, "s": 1});
        let object = object.as_object().unwrap();
        assert_eq!(required(object, "x").unwrap_err(), "has no `x` field");
        assert_eq!(required(object, "n").unwrap_err(), "has `n: null`");
        assert_eq!(string(required(object, "s").unwrap(), "s").unwrap_err(), "has `s` a number instead of a string");
        assert_eq!(nullable(object, "n"), Ok(None));
        assert!(nullable(object, "x").is_err());
    }

    #[test]
    fn the_artifact_sits_beside_its_catalog() {
        assert_eq!(facts_artifact_path(Path::new("a/functions.yaml")), PathBuf::from("a/functions.facts.json"));
    }
}
