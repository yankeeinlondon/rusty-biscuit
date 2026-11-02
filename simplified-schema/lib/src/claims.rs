//! Preparation-time extraction of the string values a schema claims for one
//! property (a `yaml-schema`'s `kind`).
//!
//! The `schema(property(kind, type-definition(...)))` meta-check is
//! parse-only: it skips reference arms and passes named-type alternatives
//! provisionally. [`claimed_values`] is the check that closes those gaps. It
//! reads the resolved lowering, where aliases, `Name@file` imports, and file
//! arms are already expanded (and their cycles already rejected by the
//! resolver), so every arm is judged the same way whatever its authored form.
//!
//! ## Examples
//!
//! ```ignore
//! let resolved = resolve_yaml_schema(&declaration, dir, &context, ShippedSources::NONE)?;
//! let kinds = simplified_schema::claimed_values(&resolved, "kind")?; // ["roster"]
//! ```

use std::fmt;

use serde_json::{Map, Value};

use crate::resolve::ResolvedSchema;
use crate::simplified::SchemaSourcePath;

/// Why one root arm does not claim string values for the property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClaimCause {
    /// The arm is not an object schema.
    NotObject,
    /// The arm does not declare the property.
    Missing,
    /// The property is not listed in the arm's `required`; an optional
    /// declaration, including one made optional by a null alternative
    /// (`null | literal(a)`), lands here.
    NotRequired,
    /// The required property's schema admits `null`.
    NullAdmitted,
    /// The property's schema is not a `const`, an `enum`, or a union of them,
    /// so its values are not a finite set (`string`, `unknown`).
    NotEnumerable,
    /// A choice is not a string (`literal(1)`, `literal(true)`).
    NonStringChoice,
    /// A choice is the empty string, or an `enum` has no members.
    EmptyChoice,
}

/// A root arm that does not claim non-empty string values for a property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimProblem {
    /// The property whose claim was checked.
    pub property: String,
    /// The root-union arm, or `None` for a single-root schema. An arm that a
    /// file reference resolved to a root union of its own keeps the index of
    /// the referencing arm.
    pub arm_index: Option<usize>,
    /// What is wrong with the arm.
    pub cause: ClaimCause,
    /// The property (or, for [`ClaimCause::NotObject`], the arm) relative to
    /// the `$schema` declaration, addressed as if declared inline. When the
    /// arm was a reference, the authored source has no node at this path; a
    /// caller locating the problem falls back to its nearest existing
    /// ancestor, the reference itself.
    pub path: SchemaSourcePath,
}

impl fmt::Display for ClaimProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(index) = self.arm_index {
            write!(f, "root-union arm {index}: ")?;
        }
        write!(f, "`{}` must be declared as `literal(...)` or `enum(...)`", self.property)
    }
}

impl std::error::Error for ClaimProblem {}

/// The non-empty strings every resolved root arm of `root` claims for
/// `property`, repeats removed, in first-seen order.
///
/// Each arm must be an object schema that lists `property` in `required`
/// and declares it, null-free, as a `const` string, an `enum` of strings, or
/// a union of those (through `$ref`s into the root `$defs`).
///
/// ## Errors
///
/// The first arm that fails, as a [`ClaimProblem`].
pub fn claimed_values(root: &ResolvedSchema, property: &str) -> Result<Vec<String>, ClaimProblem> {
    let schema = &root.json_schema;
    let defs = schema.get("$defs").and_then(Value::as_object);
    let mut claims = Vec::new();
    match root_union_arms(schema) {
        Some(arms) => {
            for (index, arm) in arms.iter().enumerate() {
                claim_arm(arm, defs, property, Some(index), &mut claims)?;
            }
        }
        None => claim_arm(schema, defs, property, None, &mut claims)?,
    }
    Ok(claims)
}

/// The arms of a root union: a top-level `anyOf` with no object keywords
/// beside it.
fn root_union_arms(schema: &Value) -> Option<&Vec<Value>> {
    let map = schema.as_object()?;
    if map.contains_key("properties") || map.contains_key("type") {
        return None;
    }
    map.get("anyOf").and_then(Value::as_array)
}

fn claim_arm(
    arm: &Value,
    defs: Option<&Map<String, Value>>,
    property: &str,
    arm_index: Option<usize>,
    claims: &mut Vec<String>,
) -> Result<(), ClaimProblem> {
    let arm_path = arm_index.map_or_else(SchemaSourcePath::root, |index| SchemaSourcePath::root().union_arm(index));
    let problem = |cause, path: &SchemaSourcePath| ClaimProblem {
        property: property.to_string(),
        arm_index,
        cause,
        path: path.clone(),
    };
    let mut seen = Vec::new();
    let arm = dereference(arm, defs, &mut seen).ok_or_else(|| problem(ClaimCause::NotObject, &arm_path))?;
    // A file arm whose own schema is a root union contributes each of its
    // arms under the referencing arm's index.
    if let Some(nested) = root_union_arms(arm) {
        return nested
            .iter()
            .try_for_each(|nested_arm| claim_arm(nested_arm, defs, property, arm_index, claims));
    }
    let map = arm.as_object().ok_or_else(|| problem(ClaimCause::NotObject, &arm_path))?;
    let is_object = map.get("type").and_then(Value::as_str) == Some("object") || map.contains_key("properties");
    if !is_object {
        return Err(problem(ClaimCause::NotObject, &arm_path));
    }
    let path = arm_path.property(property);
    let declared = map
        .get("properties")
        .and_then(|properties| properties.get(property))
        .ok_or_else(|| problem(ClaimCause::Missing, &path))?;
    let required = map
        .get("required")
        .and_then(Value::as_array)
        .is_some_and(|names| names.iter().any(|name| name.as_str() == Some(property)));
    if !required {
        return Err(problem(ClaimCause::NotRequired, &path));
    }
    let mut choices = Vec::new();
    collect_choices(declared, defs, &mut Vec::new(), &mut choices).map_err(|cause| problem(cause, &path))?;
    for choice in choices {
        if !claims.contains(&choice) {
            claims.push(choice);
        }
    }
    Ok(())
}

/// Follows `$ref`s into the root `$defs`; `None` for an unknown or cyclic
/// reference.
fn dereference<'v>(schema: &'v Value, defs: Option<&'v Map<String, Value>>, seen: &mut Vec<&'v str>) -> Option<&'v Value> {
    let Some(pointer) = schema.get("$ref").and_then(Value::as_str) else {
        return Some(schema);
    };
    if seen.contains(&pointer) {
        return None;
    }
    seen.push(pointer);
    let key = pointer.strip_prefix("#/$defs/")?.replace("~1", "/").replace("~0", "~");
    dereference(defs?.get(&key)?, defs, seen)
}

fn collect_choices<'v>(
    schema: &'v Value,
    defs: Option<&'v Map<String, Value>>,
    seen: &mut Vec<&'v str>,
    choices: &mut Vec<String>,
) -> Result<(), ClaimCause> {
    let schema = dereference(schema, defs, seen).ok_or(ClaimCause::NotEnumerable)?;
    let Some(map) = schema.as_object() else {
        return Err(ClaimCause::NotEnumerable);
    };
    if let Some(value) = map.get("const") {
        return push_choice(value, choices);
    }
    if let Some(members) = map.get("enum") {
        let members = members.as_array().ok_or(ClaimCause::NotEnumerable)?;
        if members.is_empty() {
            return Err(ClaimCause::EmptyChoice);
        }
        return members.iter().try_for_each(|member| push_choice(member, choices));
    }
    if let Some(arms) = map.get("anyOf").or_else(|| map.get("oneOf")).and_then(Value::as_array) {
        if arms.is_empty() {
            return Err(ClaimCause::NotEnumerable);
        }
        return arms.iter().try_for_each(|arm| collect_choices(arm, defs, &mut seen.clone(), choices));
    }
    let admits_null = match map.get("type") {
        Some(Value::String(ty)) => ty == "null",
        Some(Value::Array(types)) => types.iter().any(|ty| ty == "null"),
        _ => false,
    };
    Err(if admits_null { ClaimCause::NullAdmitted } else { ClaimCause::NotEnumerable })
}

fn push_choice(value: &Value, choices: &mut Vec<String>) -> Result<(), ClaimCause> {
    match value {
        Value::String(text) if text.is_empty() => Err(ClaimCause::EmptyChoice),
        Value::String(text) => {
            choices.push(text.clone());
            Ok(())
        }
        Value::Null => Err(ClaimCause::NullAdmitted),
        _ => Err(ClaimCause::NonStringChoice),
    }
}
