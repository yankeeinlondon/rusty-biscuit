//! Required properties that are absent but declare a `default(...)`.
//!
//! A required property whose definition carries a resolved default satisfies
//! its contract when absent: validation judges a candidate with the default
//! in place and reports the default it relied on. [`prepare_defaults`] is the
//! one walk that builds that candidate; [`crate::Prepared`] and a host's own
//! compiled schema (Darkmatter's `EffectiveSchema`) both call it, each with
//! the lowered schema and validators it already holds.
//!
//! The rules, in the lowered JSON Schema's terms:
//!
//! - Only a key listed in its object's `required` whose property schema has a
//!   `default` (directly, or on the local `$ref` target it names) is filled,
//!   and only when the key is absent. An explicit `null` is present and is
//!   judged by the declared type. An optional property is never listed in
//!   `required`, so its default stays an annotation.
//! - A child is filled only inside a parent that exists. A parent filled from
//!   its own default is not descended into: its default was validated
//!   against the complete contract, nested required children included.
//! - Object properties are visited in declaration order and array items in
//!   ascending index order, depth first, so a parent always comes before its
//!   children; a path is filled once, by its first occurrence.
//! - Every default used is validated strictly, without I/O or conversion,
//!   against the property's complete schema; a violation is
//!   [`SchemaError::InvalidDefault`].
//! - A union (a root union's arms, or a nested `anyOf`/`oneOf` with more than
//!   one arm that could hold the value) is judged per arm on the complete
//!   candidate that arm produces, discriminants and sibling assertions
//!   included. A value some arm accepts as written needs no default. Equal
//!   successful candidates count as one; different ones are
//!   [`SchemaError::AmbiguousDefaults`].
//! - A value at or below one of the caller's nested templates
//!   ([`NestedTemplates`]) is pending and is not filled.
//!
//! A `{{ … }}` slot in a `default(...)` never reaches this walk: every
//! lowering entry refuses an unresolved slot, so a pending default stays
//! pending until the host resolves it.

use std::borrow::Cow;
use std::sync::Arc;

use jsonschema::Validator;
use serde_json::Value;

use crate::coerce::NestedTemplates;
use crate::prepared::CompiledValidators;
use crate::validate::ValidatorCache;
use crate::{PropertyPath, SchemaError, discriminant, scope, simplified};

/// A default that stands in for an absent required property.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedDefault {
    /// Where the default goes, from the document root.
    pub path: PropertyPath,
    /// The property's resolved default.
    pub value: Value,
}

/// The value to judge once absent required properties hold their defaults.
#[derive(Debug, Clone)]
pub struct DefaultPreparation<'input> {
    /// The input with every applied default inserted; borrowed exactly when
    /// `defaults` is empty.
    pub candidate: Cow<'input, Value>,
    /// The defaults inserted, in walk order (see the module docs).
    pub defaults: Vec<AppliedDefault>,
}

/// Completes `instance` with the defaults of its absent required properties.
///
/// `json_schema` is the lowered schema `validators` were compiled from.
/// `validators` judges a root union's per-arm candidates; nested unions and
/// the defaults themselves are judged by structural validators (file values
/// by syntax alone) compiled through `cache`. Nothing is read from the
/// process or the file system beyond what `validators` already does.
///
/// An instance `validators` accepts as written is returned borrowed with no
/// defaults. The candidate is not judged as a whole here; the caller
/// validates it as it validates any value.
///
/// ## Errors
///
/// [`SchemaError::InvalidDefault`] when a default the candidate needs breaks
/// its own property's contract; [`SchemaError::AmbiguousDefaults`] when union
/// arms complete the value differently; [`SchemaError::RecursiveDefault`]
/// when a `$ref` chain returns to itself without descending into the value;
/// [`SchemaError::BuildValidator`] when a default's schema does not compile.
pub fn prepare_defaults<'input>(
    json_schema: &Arc<Value>,
    instance: Cow<'input, Value>,
    templates: &NestedTemplates,
    validators: &CompiledValidators,
    cache: &ValidatorCache,
) -> Result<DefaultPreparation<'input>, SchemaError> {
    if validators.accepts(&instance) {
        return Ok(DefaultPreparation { candidate: instance, defaults: Vec::new() });
    }
    prepare_unaccepted(json_schema, instance, templates, validators, cache)
}

/// [`prepare_defaults`] for an instance the caller already knows
/// `validators` rejects as written.
pub(crate) fn prepare_unaccepted<'input>(
    json_schema: &Arc<Value>,
    instance: Cow<'input, Value>,
    templates: &NestedTemplates,
    validators: &CompiledValidators,
    cache: &ValidatorCache,
) -> Result<DefaultPreparation<'input>, SchemaError> {
    let mut walk = Walk { root: json_schema.as_ref(), templates, cache, references: Vec::new() };
    let insertions = match validators {
        CompiledValidators::Object(_) => {
            let mut out = Vec::new();
            walk.visit(json_schema.as_ref(), "", &instance, &mut Vec::new(), &mut out, false)?;
            out
        }
        CompiledValidators::RootUnion(arm_validators) => walk.root_union(json_schema, &instance, arm_validators)?,
    };
    Ok(finish(instance, insertions))
}

/// One step from a value to a child.
#[derive(Debug, Clone, PartialEq)]
enum Step {
    Key(String),
    Index(usize),
}

/// A default to insert at `steps`.
#[derive(Debug, Clone)]
struct Insertion {
    steps: Vec<Step>,
    value: Value,
}

struct Walk<'a> {
    root: &'a Value,
    templates: &'a NestedTemplates,
    cache: &'a ValidatorCache,
    /// The `$ref`s being followed, each with the instance depth it was
    /// followed at: meeting one again at the same depth is a loop.
    references: Vec<(String, usize)>,
}

impl Walk<'_> {
    /// Collects into `out` the defaults `node` needs for `value`, which sits
    /// at `steps`. `skip_unions` leaves the node's own `anyOf`/`oneOf` to the
    /// caller (a root union's arms are judged by the caller's validators).
    fn visit(
        &mut self,
        node: &Value,
        pointer: &str,
        value: &Value,
        steps: &mut Vec<Step>,
        out: &mut Vec<Insertion>,
        skip_unions: bool,
    ) -> Result<(), SchemaError> {
        let Some(map) = node.as_object() else {
            return Ok(());
        };
        if !self.templates.is_empty() && self.templates.holds(&instance_pointer(steps)) {
            return Ok(());
        }
        if let Some(Value::String(reference)) = map.get("$ref")
            && let Some(target_pointer) = reference.strip_prefix('#')
        {
            let key = (reference.clone(), steps.len());
            if self.references.contains(&key) {
                return Err(SchemaError::RecursiveDefault { path: property_path(steps), reference: reference.clone() });
            }
            if let Some(target) = self.root.pointer(target_pointer) {
                self.references.push(key);
                let result = self.visit(target, target_pointer, value, steps, out, false);
                self.references.pop();
                result?;
            }
        }
        if let Some(Value::Array(members)) = map.get("allOf") {
            for (index, member) in members.iter().enumerate() {
                self.visit(member, &format!("{pointer}/allOf/{index}"), value, steps, out, false)?;
            }
        }
        if !skip_unions {
            for keyword in ["anyOf", "oneOf"] {
                if let Some(Value::Array(arms)) = map.get(keyword) {
                    self.union(node, &format!("{pointer}/{keyword}"), arms, value, steps, out)?;
                }
            }
        }
        match value {
            Value::Object(object) => self.object(map, pointer, object, steps, out),
            Value::Array(items) => self.array(map, pointer, items, steps, out),
            _ => Ok(()),
        }
    }

    fn object(
        &mut self,
        map: &serde_json::Map<String, Value>,
        pointer: &str,
        object: &serde_json::Map<String, Value>,
        steps: &mut Vec<Step>,
        out: &mut Vec<Insertion>,
    ) -> Result<(), SchemaError> {
        let required: Vec<&str> = match map.get("required") {
            Some(Value::Array(names)) => names.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        let properties = map.get("properties").and_then(Value::as_object);
        for (key, schema) in properties.into_iter().flatten() {
            let at = format!("{pointer}/properties/{}", escape(key));
            steps.push(Step::Key(key.clone()));
            let result = match object.get(key) {
                Some(child) => self.visit(schema, &at, child, steps, out, false),
                None if required.contains(&key.as_str()) => self.fill(schema, &at, steps, out),
                None => Ok(()),
            };
            steps.pop();
            result?;
        }
        // A key a pattern matches is not judged by `additionalProperties`;
        // without a regex engine here, such objects are left alone.
        if map.get("patternProperties").is_none()
            && let Some(additional @ Value::Object(_)) = map.get("additionalProperties")
        {
            let at = format!("{pointer}/additionalProperties");
            for (key, child) in object {
                if properties.is_some_and(|declared| declared.contains_key(key)) {
                    continue;
                }
                steps.push(Step::Key(key.clone()));
                let result = self.visit(additional, &at, child, steps, out, false);
                steps.pop();
                result?;
            }
        }
        Ok(())
    }

    fn array(
        &mut self,
        map: &serde_json::Map<String, Value>,
        pointer: &str,
        items: &[Value],
        steps: &mut Vec<Step>,
        out: &mut Vec<Insertion>,
    ) -> Result<(), SchemaError> {
        let prefix = map.get("prefixItems").and_then(Value::as_array);
        let rest = map.get("items").filter(|items| items.is_object());
        for (index, item) in items.iter().enumerate() {
            let (schema, at) = match prefix.and_then(|prefix| prefix.get(index)) {
                Some(schema) => (schema, format!("{pointer}/prefixItems/{index}")),
                None => match rest {
                    Some(schema) => (schema, format!("{pointer}/items")),
                    None => continue,
                },
            };
            steps.push(Step::Index(index));
            let result = self.visit(schema, &at, item, steps, out, false);
            steps.pop();
            result?;
        }
        Ok(())
    }

    /// Records the default of the absent required property `schema`
    /// describes, after validating it.
    fn fill(&mut self, schema: &Value, pointer: &str, steps: &[Step], out: &mut Vec<Insertion>) -> Result<(), SchemaError> {
        let Some((default, default_pointer)) = declared_default(self.root, schema, pointer) else {
            return Ok(());
        };
        let Some(path) = property_path(steps) else {
            // A path no property path can spell (an empty key among several
            // segments) cannot be reported, so the property stays absent.
            return Ok(());
        };
        let validator = self.structural(schema)?;
        let messages: Vec<String> = validator.iter_errors(default).map(|error| error.to_string()).collect();
        if !messages.is_empty() {
            return Err(SchemaError::InvalidDefault { path, pointer: default_pointer, message: messages.join("; ") });
        }
        out.push(Insertion { steps: steps.to_vec(), value: default.clone() });
        Ok(())
    }

    /// A nested union: each arm that could hold `value` is judged on the
    /// candidate its own defaults produce.
    fn union(
        &mut self,
        node: &Value,
        pointer: &str,
        arms: &[Value],
        value: &Value,
        steps: &mut Vec<Step>,
        out: &mut Vec<Insertion>,
    ) -> Result<(), SchemaError> {
        if value.is_null() {
            return Ok(());
        }
        let holders: Vec<usize> = (0..arms.len()).filter(|&index| !is_null_only(&arms[index])).collect();
        if let [only] = holders.as_slice() {
            return self.visit(&arms[*only], &format!("{pointer}/{only}"), value, steps, out, false);
        }
        let mut per_arm = Vec::new();
        for &index in &holders {
            let mut arm_out = Vec::new();
            self.visit(&arms[index], &format!("{pointer}/{index}"), value, steps, &mut arm_out, false)?;
            per_arm.push((index, arm_out));
        }
        if per_arm.iter().all(|(_, arm_out)| arm_out.is_empty()) {
            return Ok(());
        }
        let holder = scope::with_root_definitions(node, self.root);
        let base = instance_pointer(steps);
        let mut judges = Vec::with_capacity(per_arm.len());
        for (index, _) in &per_arm {
            let wrapped = crate::validate::wrap_arm_as_root_schema(&arms[*index], &holder);
            judges.push(self.cache.structural_validator_for(&wrapped)?);
        }
        if judges.iter().any(|judge| self.accepts(judge, value, &base)) {
            return Ok(());
        }
        let depth = steps.len();
        let successes: Vec<(usize, Value, Vec<Insertion>)> = per_arm
            .into_iter()
            .zip(&judges)
            .filter(|((_, arm_out), _)| !arm_out.is_empty())
            .filter_map(|((index, arm_out), judge)| {
                let candidate = apply(value.clone(), &arm_out, depth);
                self.accepts(judge, &candidate, &base).then_some((index, candidate, arm_out))
            })
            .collect();
        match settle(successes) {
            Settled::None => Ok(()),
            Settled::One(insertions) => {
                out.extend(insertions);
                Ok(())
            }
            Settled::Ambiguous(arms) => {
                Err(SchemaError::AmbiguousDefaults { path: property_path(steps), pointer: pointer.to_string(), arms })
            }
        }
    }

    /// The root union's arms, each judged by its compiled validator on the
    /// complete candidate it produces. With no successful arm, the arm a
    /// literal discriminant selects keeps its defaults, so the report that
    /// follows names that arm's real problems.
    fn root_union(&mut self, root: &Value, instance: &Value, validators: &[Arc<Validator>]) -> Result<Vec<Insertion>, SchemaError> {
        let Some(arms) = root.get("anyOf").and_then(Value::as_array) else {
            return Ok(Vec::new());
        };
        let mut shared = Vec::new();
        self.visit(root, "", instance, &mut Vec::new(), &mut shared, true)?;
        let mut per_arm = Vec::with_capacity(arms.len());
        for (index, arm) in arms.iter().enumerate() {
            let mut arm_out = shared.clone();
            self.visit(arm, &format!("/anyOf/{index}"), instance, &mut Vec::new(), &mut arm_out, false)?;
            per_arm.push((index, arm_out));
        }
        let successes: Vec<(usize, Value, Vec<Insertion>)> = per_arm
            .iter()
            .filter(|(_, arm_out)| !arm_out.is_empty())
            .filter_map(|(index, arm_out)| {
                let candidate = apply(instance.clone(), arm_out, 0);
                let judge = validators.get(*index)?;
                self.accepts(judge, &candidate, "").then(|| (*index, candidate, arm_out.clone()))
            })
            .collect();
        match settle(successes) {
            Settled::One(insertions) => Ok(insertions),
            Settled::Ambiguous(arms) => {
                Err(SchemaError::AmbiguousDefaults { path: None, pointer: "/anyOf".to_string(), arms })
            }
            Settled::None => Ok(discriminant::select_literal_discriminant_arm(arms, instance)
                .and_then(|selected| per_arm.into_iter().find(|(index, _)| *index == selected))
                .map_or(shared, |(_, arm_out)| arm_out)),
        }
    }

    /// Whether `judge` accepts `candidate`, at instance location `base`,
    /// with every problem a nested template owes excused.
    fn accepts(&self, judge: &Validator, candidate: &Value, base: &str) -> bool {
        if judge.is_valid(candidate) {
            return true;
        }
        !self.templates.is_empty()
            && judge
                .iter_errors(candidate)
                .all(|error| self.templates.excuses_location(&format!("{base}{}", error.instance_path().as_str())))
    }

    /// A structural validator for `schema` as a root of its own, the root's
    /// definitions in scope when it refers to them.
    fn structural(&self, schema: &Value) -> Result<Arc<Validator>, SchemaError> {
        let mut standalone =
            if refers(schema) { scope::with_root_definitions(schema, self.root) } else { schema.clone() };
        if let Some(map) = standalone.as_object_mut() {
            map.entry("$schema").or_insert_with(|| Value::String(simplified::DRAFT_2020_12.to_string()));
        }
        self.cache.structural_validator_for(&standalone)
    }
}

enum Settled {
    None,
    One(Vec<Insertion>),
    Ambiguous(Vec<usize>),
}

/// One candidate when every success completes the value identically (the
/// first arm's insertions stand for them all).
fn settle(successes: Vec<(usize, Value, Vec<Insertion>)>) -> Settled {
    let mut successes = successes.into_iter();
    let Some((first_index, first, insertions)) = successes.next() else {
        return Settled::None;
    };
    let conflicting: Vec<usize> = successes.filter(|(_, candidate, _)| *candidate != first).map(|(index, ..)| index).collect();
    if conflicting.is_empty() {
        Settled::One(insertions)
    } else {
        Settled::Ambiguous(std::iter::once(first_index).chain(conflicting).collect())
    }
}

/// `instance` with `insertions` applied, each first occurrence of a path
/// only, and the defaults that were.
fn finish(instance: Cow<'_, Value>, insertions: Vec<Insertion>) -> DefaultPreparation<'_> {
    let mut unique: Vec<Insertion> = Vec::with_capacity(insertions.len());
    for insertion in insertions {
        if !unique.iter().any(|seen| seen.steps == insertion.steps) {
            unique.push(insertion);
        }
    }
    if unique.is_empty() {
        return DefaultPreparation { candidate: instance, defaults: Vec::new() };
    }
    let candidate = apply(instance.into_owned(), &unique, 0);
    let defaults = unique
        .into_iter()
        .filter_map(|insertion| Some(AppliedDefault { path: property_path(&insertion.steps)?, value: insertion.value }))
        .collect();
    DefaultPreparation { candidate: Cow::Owned(candidate), defaults }
}

/// `value` with each insertion placed at its steps below the first `depth`
/// (the location `value` itself sits at). A key already present is kept.
fn apply(mut value: Value, insertions: &[Insertion], depth: usize) -> Value {
    for insertion in insertions {
        if let Some((Step::Key(last), parents)) = insertion.steps[depth..].split_last()
            && let Some(Value::Object(object)) = descend(&mut value, parents)
            && !object.contains_key(last)
        {
            object.insert(last.clone(), insertion.value.clone());
        }
    }
    value
}

/// The value at `steps` below `value`, if every step exists.
fn descend<'v>(value: &'v mut Value, steps: &[Step]) -> Option<&'v mut Value> {
    steps.iter().try_fold(value, |holder, step| match (step, holder) {
        (Step::Key(key), Value::Object(object)) => object.get_mut(key),
        (Step::Index(index), Value::Array(items)) => items.get_mut(*index),
        _ => None,
    })
}

/// The property path of `steps`; `None` at the root or for a path no
/// property path can spell.
fn property_path(steps: &[Step]) -> Option<PropertyPath> {
    if steps.is_empty() {
        return None;
    }
    PropertyPath::from_segments(steps.iter().map(|step| match step {
        Step::Key(key) => key.clone(),
        Step::Index(index) => index.to_string(),
    }))
    .ok()
}

fn instance_pointer(steps: &[Step]) -> String {
    steps
        .iter()
        .map(|step| match step {
            Step::Key(key) => format!("/{}", escape(key)),
            Step::Index(index) => format!("/{index}"),
        })
        .collect()
}

/// `key` as one JSON Pointer reference token.
fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// The default `schema` declares, on itself or on the local `$ref` target it
/// names, with the JSON Pointer of its `default` keyword.
fn declared_default<'s>(root: &'s Value, schema: &'s Value, pointer: &str) -> Option<(&'s Value, String)> {
    let (mut schema, mut pointer) = (schema, pointer.to_string());
    for _ in 0..32 {
        if let Some(default) = schema.get("default") {
            return Some((default, format!("{pointer}/default")));
        }
        let target = schema.get("$ref").and_then(Value::as_str)?.strip_prefix('#')?;
        schema = root.pointer(target)?;
        pointer = target.to_string();
    }
    None
}

/// Whether `arm` admits only `null` (an optional property's absent arm).
fn is_null_only(arm: &Value) -> bool {
    arm.get("type").and_then(Value::as_str) == Some("null")
}

/// Whether `schema` holds a `$ref` anywhere.
fn refers(schema: &Value) -> bool {
    match schema {
        Value::Object(map) => map.contains_key("$ref") || map.values().any(refers),
        Value::Array(items) => items.iter().any(refers),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn walk(schema: &Value, instance: &Value) -> Result<Vec<Insertion>, SchemaError> {
        let cache = ValidatorCache::new();
        let templates = NestedTemplates::default();
        let mut walk = Walk { root: schema, templates: &templates, cache: &cache, references: Vec::new() };
        let mut out = Vec::new();
        walk.visit(schema, "", instance, &mut Vec::new(), &mut out, false)?;
        Ok(out)
    }

    #[test]
    fn a_reference_loop_without_descent_is_refused() {
        let schema = json!({
            "$defs": { "a": { "$ref": "#/$defs/b" }, "b": { "$ref": "#/$defs/a" } },
            "$ref": "#/$defs/a",
        });

        let error = walk(&schema, &json!({})).unwrap_err();

        assert!(matches!(error, SchemaError::RecursiveDefault { path: None, .. }), "{error:?}");
    }

    #[test]
    fn a_recursive_reference_that_descends_is_followed_to_the_value_depth() {
        let node = json!({
            "type": "object",
            "properties": {
                "v": { "type": "number", "default": 1 },
                "next": { "$ref": "#/$defs/node" },
            },
            "required": ["v"],
        });
        let schema = json!({ "$defs": { "node": node }, "$ref": "#/$defs/node" });

        let out = walk(&schema, &json!({ "next": { "next": {} } })).unwrap();

        let paths: Vec<_> = out.iter().map(|insertion| instance_pointer(&insertion.steps)).collect();
        assert_eq!(paths, ["/v", "/next/v", "/next/next/v"]);
    }

    #[test]
    fn an_insertion_below_a_missing_parent_is_dropped() {
        let insertion = Insertion { steps: vec![Step::Key("gone".into()), Step::Key("a".into())], value: json!(1) };

        let value = apply(json!({ "kept": true }), &[insertion], 0);

        assert_eq!(value, json!({ "kept": true }));
    }

    #[test]
    fn the_first_occurrence_of_a_path_wins() {
        let first = Insertion { steps: vec![Step::Key("a".into())], value: json!(1) };
        let second = Insertion { steps: vec![Step::Key("a".into())], value: json!(2) };

        let prepared = finish(Cow::Owned(json!({})), vec![first, second]);

        assert_eq!(prepared.candidate.into_owned(), json!({ "a": 1 }));
        assert_eq!(prepared.defaults.len(), 1);
    }
}
