//! Default preparation: the one step that fills a property from its
//! `default(...)`.
//!
//! Preparing defaults and validating are separate steps. Preparation builds
//! the completed candidate; validation then judges that candidate and sets
//! nothing. [`DefaultPlan::compile`] reads a lowered schema (or a raw JSON
//! Schema) once, and [`prepare_defaults`] applies the plan to each value.
//! [`crate::Prepared`] and a host's own compiled schema (Darkmatter's
//! `EffectiveSchema`) hold a plan beside their validators.
//!
//! The rules, in the lowered JSON Schema's terms:
//!
//! - A property whose value is missing or `null` takes its declared
//!   `default`, whether or not it is `required`. A supplied non-null value
//!   always wins: `0`, `false`, `""`, `[]`, and `{}` are values.
//! - A `null` default fills nothing, except that a missing key of a
//!   `required` property is inserted as `null` (and reported); a written
//!   `null` already meets that contract.
//! - Defaults are collected per object location, across `$ref` targets,
//!   `allOf` members, and every `anyOf`/`oneOf` arm, without choosing an arm.
//!   Equal declarations count as one; different values for one property are
//!   [`SchemaError::ConflictingDefaults`] when the plan is compiled, used or
//!   not. Which arm the completed value then matches is validation's
//!   question.
//! - A child is filled only inside a parent that exists. A missing or `null`
//!   parent with no default of its own stays as it is. A parent filled from
//!   its own default is first checked against its complete definition
//!   (required children included), then descended into so its optional
//!   children are completed too.
//! - Array items and function-style tuple positions are never inserted;
//!   objects inside existing items are completed.
//! - Every default used is checked strictly, without I/O or conversion,
//!   against each property definition that declares it; a violation is
//!   [`SchemaError::InvalidDefault`]. An unused default is never judged.
//! - Object properties are visited in declaration order and array items in
//!   ascending index order, depth first, so a parent always comes before its
//!   children; each changed path is reported once.
//! - A value at or below one of the caller's nested templates
//!   ([`NestedTemplates`]) is pending and is not filled.
//!
//! A `{{ … }}` slot in a `default(...)` never reaches this module: every
//! lowering entry refuses an unresolved slot, so a pending default stays
//! pending until the host resolves it.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use indexmap::IndexMap;
use jsonschema::Validator;
use serde_json::Value;

use crate::coerce::NestedTemplates;
use crate::validate::{SchemaIdentity, ValidatorCache};
use crate::{PropertyPath, SchemaError, scope, simplified};

/// A default that preparation placed in the value.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedDefault {
    /// Where the default goes, from the document root.
    pub path: PropertyPath,
    /// The property's resolved default.
    pub value: Value,
}

/// The value to judge once missing and `null` properties hold their
/// defaults.
#[derive(Debug, Clone)]
pub struct DefaultPreparation<'input> {
    /// The input with every applied default in place; borrowed exactly when
    /// `defaults` is empty.
    pub candidate: Cow<'input, Value>,
    /// The defaults applied, in walk order (see the module docs).
    pub defaults: Vec<AppliedDefault>,
}

/// How many compiled plans [`DefaultPlan::for_identity`] keeps before it
/// starts over.
const PLAN_CACHE_CAPACITY: usize = 64;

static PLAN_CACHE: LazyLock<Mutex<HashMap<SchemaIdentity, Arc<DefaultPlan>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn reset_process_caches() {
    PLAN_CACHE.lock().expect("default plan cache poisoned").clear();
}

/// The defaults one schema declares, per object location, compiled once
/// and applied to every value by [`prepare_defaults`].
pub struct DefaultPlan {
    schema: Arc<Value>,
    /// Compiles the structural validators that check a used default.
    cache: ValidatorCache,
    /// Object locations; `None` when the schema declares no structure.
    root: Option<usize>,
    scopes: Vec<Scope>,
    /// Whether any location declares a default.
    declares_defaults: bool,
}

impl std::fmt::Debug for DefaultPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultPlan").field("scopes", &self.scopes.len()).finish_non_exhaustive()
    }
}

/// One object location: the merge of every schema node that can describe
/// the value there.
#[derive(Default)]
struct Scope {
    properties: Vec<Property>,
    /// The location of an undeclared key's value.
    additional: Option<usize>,
    /// The location of each leading array item.
    prefix: Vec<Option<usize>>,
    /// The location of every array item after `prefix`.
    rest: Option<usize>,
}

struct Property {
    name: String,
    required: bool,
    child: Option<usize>,
    default: Option<Declared>,
}

/// A property's one default.
struct Declared {
    value: Value,
    /// The JSON Pointer of the first `default` keyword declaring it.
    pointer: String,
    /// Each property definition that declares it, which the default must
    /// satisfy when used.
    definitions: Vec<String>,
    /// Their structural validators, compiled on first use.
    validators: OnceLock<Vec<Arc<Validator>>>,
}

impl DefaultPlan {
    /// Reads every default `schema` declares, a lowered or raw JSON Schema,
    /// following local `$ref`s, `allOf`, `anyOf`, `oneOf`, properties,
    /// `additionalProperties`, and array items. Recursive references compile
    /// once. A used default is checked by structural validators compiled
    /// through `cache`.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::ConflictingDefaults`] when one object location declares
    /// different defaults for one property (on different union arms, say);
    /// [`SchemaError::RecursiveDefault`] when a `$ref` chain returns to itself
    /// without describing a child value.
    pub fn compile(schema: Arc<Value>, cache: &ValidatorCache) -> Result<Self, SchemaError> {
        let (root, scopes) = Compiler::run(&schema)?;
        let declares_defaults =
            scopes.iter().flat_map(|scope| &scope.properties).any(|property| property.default.is_some());
        Ok(Self { schema, cache: cache.clone(), root, scopes, declares_defaults })
    }

    /// [`Self::compile`] for a schema whose identity the caller knows (the
    /// identity its validators are cached under), compiled once per identity
    /// for the process.
    ///
    /// ## Errors
    ///
    /// As [`Self::compile`]; an error is not remembered.
    pub fn for_identity(identity: SchemaIdentity, schema: &Arc<Value>, cache: &ValidatorCache) -> Result<Arc<Self>, SchemaError> {
        if let Some(plan) = PLAN_CACHE.lock().expect("default plan cache poisoned").get(&identity) {
            return Ok(Arc::clone(plan));
        }
        let plan = Arc::new(Self::compile(Arc::clone(schema), cache)?);
        let mut plans = PLAN_CACHE.lock().expect("default plan cache poisoned");
        if plans.len() >= PLAN_CACHE_CAPACITY {
            plans.clear();
        }
        plans.insert(identity, Arc::clone(&plan));
        Ok(plan)
    }

    /// Whether the schema declares any default at all, so preparation has
    /// nothing to do for any value.
    pub fn is_empty(&self) -> bool {
        !self.declares_defaults
    }
}

/// Fills the missing and `null` properties of `instance` from `plan`.
///
/// The candidate is not judged as a whole here; the caller validates it as
/// it validates any value. Nothing is read from the process or the file
/// system, and no expression is evaluated.
///
/// ## Errors
///
/// [`SchemaError::InvalidDefault`] when a default the value needs breaks its
/// own property's definition; [`SchemaError::RecursiveDefault`] when filling
/// a default keeps creating another defaulted parent;
/// [`SchemaError::BuildValidator`] when a default's definition does not
/// compile.
pub fn prepare_defaults<'input>(
    plan: &DefaultPlan,
    instance: Cow<'input, Value>,
    templates: &NestedTemplates,
) -> Result<DefaultPreparation<'input>, SchemaError> {
    let Some(root) = plan.root.filter(|_| !plan.is_empty()) else {
        return Ok(DefaultPreparation { candidate: instance, defaults: Vec::new() });
    };
    let mut fill = Fill { plan, templates, active: Vec::new(), out: Vec::new() };
    fill.visit(root, &instance, &mut Vec::new())?;
    Ok(finish(instance, fill.out))
}

// ── Compilation ───────────────────────────────────────────────────────────

struct Compiler<'s> {
    root: &'s Value,
    scopes: Vec<Scope>,
    /// Each location's id, keyed by the sorted pointers of its nodes.
    known: HashMap<Vec<String>, usize>,
}

impl<'s> Compiler<'s> {
    fn run(root: &'s Value) -> Result<(Option<usize>, Vec<Scope>), SchemaError> {
        let mut compiler = Compiler { root, scopes: Vec::new(), known: HashMap::new() };
        let root_scope = compiler.scope(&[String::new()])?;
        Ok((root_scope, compiler.scopes))
    }

    fn node(&self, pointer: &str) -> Option<&'s serde_json::Map<String, Value>> {
        self.root.pointer(pointer).and_then(Value::as_object)
    }

    /// Every schema node that applies wherever `pointer`'s node applies:
    /// itself, its local `$ref` target, and its `allOf`/`anyOf`/`oneOf`
    /// members, transitively, in that order.
    fn expand(&self, pointer: &str, chain: &mut Vec<String>, out: &mut Vec<String>) -> Result<(), SchemaError> {
        if out.iter().any(|seen| seen == pointer) {
            return Ok(());
        }
        let Some(map) = self.node(pointer) else {
            return Ok(());
        };
        out.push(pointer.to_string());
        chain.push(pointer.to_string());
        if let Some(Value::String(reference)) = map.get("$ref")
            && let Some(target) = reference.strip_prefix('#')
        {
            if chain.iter().any(|ancestor| ancestor == target) {
                return Err(SchemaError::RecursiveDefault { path: None, reference: reference.clone() });
            }
            self.expand(target, chain, out)?;
        }
        for keyword in ["allOf", "anyOf", "oneOf"] {
            if let Some(Value::Array(members)) = map.get(keyword) {
                for index in 0..members.len() {
                    self.expand(&format!("{pointer}/{keyword}/{index}"), chain, out)?;
                }
            }
        }
        chain.pop();
        Ok(())
    }

    fn expand_all(&self, pointers: &[String]) -> Result<Vec<String>, SchemaError> {
        let mut out = Vec::new();
        for pointer in pointers {
            self.expand(pointer, &mut Vec::new(), &mut out)?;
        }
        Ok(out)
    }

    /// The location described by the nodes at `pointers`; `None` when none
    /// of them describes an object or array.
    fn scope(&mut self, pointers: &[String]) -> Result<Option<usize>, SchemaError> {
        let nodes: Vec<String> = self.expand_all(pointers)?.into_iter().filter(|pointer| self.is_structural(pointer)).collect();
        if nodes.is_empty() {
            return Ok(None);
        }
        let mut key = nodes.clone();
        key.sort();
        key.dedup();
        if let Some(&known) = self.known.get(&key) {
            return Ok(Some(known));
        }
        let id = self.scopes.len();
        self.scopes.push(Scope::default());
        self.known.insert(key, id);

        let mut declared: IndexMap<&'s str, Vec<String>> = IndexMap::new();
        let mut required: Vec<&'s str> = Vec::new();
        let mut additional = Vec::new();
        // Each node's leading item schemas and the schema of the items after them.
        let mut arrays: Vec<(Vec<String>, Option<String>)> = Vec::new();
        for pointer in &nodes {
            let Some(map) = self.node(pointer) else { continue };
            if let Some(Value::Object(properties)) = map.get("properties") {
                for name in properties.keys() {
                    declared.entry(name.as_str()).or_default().push(format!("{pointer}/properties/{}", escape(name)));
                }
            }
            if let Some(Value::Array(names)) = map.get("required") {
                required.extend(names.iter().filter_map(Value::as_str));
            }
            // A key a pattern matches is not judged by `additionalProperties`;
            // without a regex engine here, such objects are left alone.
            if !map.contains_key("patternProperties") && map.get("additionalProperties").is_some_and(Value::is_object) {
                additional.push(format!("{pointer}/additionalProperties"));
            }
            let items = map.get("items").is_some_and(Value::is_object).then(|| format!("{pointer}/items"));
            let prefix: Vec<String> = match map.get("prefixItems") {
                Some(Value::Array(prefix)) => (0..prefix.len()).map(|index| format!("{pointer}/prefixItems/{index}")).collect(),
                _ => Vec::new(),
            };
            if !prefix.is_empty() || items.is_some() {
                arrays.push((prefix, items));
            }
        }

        let mut properties = Vec::with_capacity(declared.len());
        for (name, definitions) in declared {
            let child = self.scope(&definitions)?;
            let default = self.declared(&nodes[0], name, definitions)?;
            properties.push(Property { name: name.to_string(), required: required.contains(&name), child, default });
        }
        let additional = if additional.is_empty() { None } else { self.scope(&additional)? };
        let longest = arrays.iter().map(|(prefix, _)| prefix.len()).max().unwrap_or(0);
        let mut prefix = Vec::with_capacity(longest);
        for index in 0..longest {
            let at: Vec<String> =
                arrays.iter().filter_map(|(prefix, items)| prefix.get(index).or(items.as_ref()).cloned()).collect();
            prefix.push(if at.is_empty() { None } else { self.scope(&at)? });
        }
        let rest: Vec<String> = arrays.into_iter().filter_map(|(_, items)| items).collect();
        let rest = if rest.is_empty() { None } else { self.scope(&rest)? };
        self.scopes[id] = Scope { properties, additional, prefix, rest };
        Ok(Some(id))
    }

    /// Whether the node at `pointer` describes an object's or an array's
    /// members.
    fn is_structural(&self, pointer: &str) -> bool {
        self.node(pointer).is_some_and(|map| {
            ["properties", "required", "additionalProperties", "items", "prefixItems"].iter().any(|key| map.contains_key(*key))
        })
    }

    /// The one default the property `name` declares across `definitions`,
    /// its declarations at the location whose first node is `location`.
    fn declared(&self, location: &str, name: &str, definitions: Vec<String>) -> Result<Option<Declared>, SchemaError> {
        let mut found: Vec<(&'s Value, String)> = Vec::new();
        let mut declaring = Vec::new();
        for definition in definitions {
            let mut held = false;
            for node in self.expand_all(std::slice::from_ref(&definition))? {
                if let Some(value) = self.node(&node).and_then(|map| map.get("default")) {
                    held = true;
                    found.push((value, format!("{node}/default")));
                }
            }
            if held {
                declaring.push(definition);
            }
        }
        let Some((first, pointer)) = found.first().cloned() else {
            return Ok(None);
        };
        if found.iter().any(|(value, _)| *value != first) {
            return Err(SchemaError::ConflictingDefaults {
                pointer: location.to_string(),
                property: name.to_string(),
                declarations: found.into_iter().map(|(_, pointer)| pointer).collect(),
            });
        }
        Ok(Some(Declared { value: first.clone(), pointer, definitions: declaring, validators: OnceLock::new() }))
    }
}

/// Checks that `schema` declares at most one default per property at each
/// object location: the compile-time half of [`DefaultPlan::compile`], for
/// lowering and export.
///
/// ## Errors
///
/// As [`DefaultPlan::compile`].
pub(crate) fn check_declarations(schema: &Value) -> Result<(), SchemaError> {
    Compiler::run(schema).map(drop)
}

// ── Application ───────────────────────────────────────────────────────────

/// One step from a value to a child.
#[derive(Debug, Clone, PartialEq)]
enum Step {
    Key(String),
    Index(usize),
}

/// A default to place at `steps`.
#[derive(Debug, Clone)]
struct Insertion {
    steps: Vec<Step>,
    value: Value,
}

struct Fill<'a> {
    plan: &'a DefaultPlan,
    templates: &'a NestedTemplates,
    /// The defaults being expanded, as `(location, property index)`: meeting
    /// one again inside its own expansion would never end.
    active: Vec<(usize, usize)>,
    out: Vec<Insertion>,
}

impl Fill<'_> {
    fn visit(&mut self, location: usize, value: &Value, steps: &mut Vec<Step>) -> Result<(), SchemaError> {
        if !self.templates.is_empty() && self.templates.holds(&instance_pointer(steps)) {
            return Ok(());
        }
        let plan = self.plan;
        let scope = &plan.scopes[location];
        match value {
            Value::Object(object) => {
                for (index, property) in scope.properties.iter().enumerate() {
                    steps.push(Step::Key(property.name.clone()));
                    let result = match object.get(&property.name) {
                        Some(child) if !child.is_null() => match property.child {
                            Some(child_location) => self.visit(child_location, child, steps),
                            None => Ok(()),
                        },
                        written => self.fill(location, index, property, written.is_some(), steps),
                    };
                    steps.pop();
                    result?;
                }
                if let Some(additional) = scope.additional {
                    for (key, child) in object {
                        if scope.properties.iter().any(|property| property.name == *key) {
                            continue;
                        }
                        steps.push(Step::Key(key.clone()));
                        let result = self.visit(additional, child, steps);
                        steps.pop();
                        result?;
                    }
                }
                Ok(())
            }
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    let at = match scope.prefix.get(index) {
                        Some(at) => *at,
                        None => scope.rest,
                    };
                    let Some(at) = at else { continue };
                    steps.push(Step::Index(index));
                    let result = self.visit(at, item, steps);
                    steps.pop();
                    result?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Fills `property`, whose value at `steps` is missing or (when
    /// `written`) `null`, from its default, then completes the default's own
    /// children.
    fn fill(&mut self, location: usize, index: usize, property: &Property, written: bool, steps: &mut Vec<Step>) -> Result<(), SchemaError> {
        let Some(default) = &property.default else {
            return Ok(());
        };
        if default.value.is_null() && (written || !property.required) {
            return Ok(());
        }
        let Some(path) = property_path(steps) else {
            // A path no property path can spell (an empty key among several
            // segments) cannot be reported, so the property stays as it is.
            return Ok(());
        };
        if self.active.contains(&(location, index)) {
            return Err(SchemaError::RecursiveDefault { path: Some(path), reference: default.pointer.clone() });
        }
        self.check(default, path)?;
        self.out.push(Insertion { steps: steps.clone(), value: default.value.clone() });
        if let Some(child) = property.child {
            self.active.push((location, index));
            let result = self.visit(child, &default.value, steps);
            self.active.pop();
            result?;
        }
        Ok(())
    }

    /// Checks `default`, about to stand at `path`, against every definition
    /// that declares it.
    fn check(&self, default: &Declared, path: PropertyPath) -> Result<(), SchemaError> {
        let validators = match default.validators.get() {
            Some(validators) => validators,
            None => {
                let built = default
                    .definitions
                    .iter()
                    .filter_map(|definition| self.plan.schema.pointer(definition))
                    .map(|definition| self.structural(definition))
                    .collect::<Result<Vec<_>, SchemaError>>()?;
                default.validators.get_or_init(|| built)
            }
        };
        let messages: Vec<String> = validators
            .iter()
            .flat_map(|validator| validator.iter_errors(&default.value).map(|error| error.to_string()).collect::<Vec<_>>())
            .collect();
        if messages.is_empty() {
            Ok(())
        } else {
            Err(SchemaError::InvalidDefault { path, pointer: default.pointer.clone(), message: messages.join("; ") })
        }
    }

    /// A structural validator for `schema` as a root of its own, the root's
    /// definitions in scope when it refers to them.
    fn structural(&self, schema: &Value) -> Result<Arc<Validator>, SchemaError> {
        let root = self.plan.schema.as_ref();
        let mut standalone = if refers(schema) { scope::with_root_definitions(schema, root) } else { schema.clone() };
        if let Some(map) = standalone.as_object_mut() {
            map.entry("$schema").or_insert_with(|| Value::String(simplified::DRAFT_2020_12.to_string()));
        }
        self.plan.cache.structural_validator_for(&standalone)
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
    let candidate = apply(instance.into_owned(), &unique);
    let defaults = unique
        .into_iter()
        .filter_map(|insertion| Some(AppliedDefault { path: property_path(&insertion.steps)?, value: insertion.value }))
        .collect();
    DefaultPreparation { candidate: Cow::Owned(candidate), defaults }
}

/// `value` with each insertion placed at its steps, replacing a missing or
/// `null` value; any other value is kept.
fn apply(mut value: Value, insertions: &[Insertion]) -> Value {
    for insertion in insertions {
        if let Some((Step::Key(last), parents)) = insertion.steps.split_last()
            && let Some(Value::Object(object)) = descend(&mut value, parents)
            && object.get(last).is_none_or(Value::is_null)
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

    fn prepare(schema: Value, instance: &Value) -> Result<DefaultPreparation<'_>, SchemaError> {
        let plan = DefaultPlan::compile(Arc::new(schema), &ValidatorCache::new())?;
        prepare_defaults(&plan, Cow::Borrowed(instance), &NestedTemplates::default())
    }

    fn pointers(prepared: &DefaultPreparation<'_>) -> Vec<String> {
        prepared.defaults.iter().map(|applied| applied.path.to_string()).collect()
    }

    #[test]
    fn a_reference_loop_without_descent_is_refused_when_compiled() {
        let schema = json!({
            "$defs": { "a": { "$ref": "#/$defs/b" }, "b": { "$ref": "#/$defs/a" } },
            "$ref": "#/$defs/a",
        });

        let error = DefaultPlan::compile(Arc::new(schema), &ValidatorCache::new()).unwrap_err();

        assert!(matches!(error, SchemaError::RecursiveDefault { path: None, .. }), "{error:?}");
    }

    #[test]
    fn a_recursive_reference_compiles_once_and_follows_the_value() {
        let node = json!({
            "type": "object",
            "properties": {
                "v": { "type": "number", "default": 1 },
                "next": { "$ref": "#/$defs/node" },
            },
        });
        let schema = json!({ "$defs": { "node": node }, "$ref": "#/$defs/node" });
        let input = json!({ "next": { "next": {} } });

        let prepared = prepare(schema, &input).unwrap();

        assert_eq!(pointers(&prepared), ["v", "next.v", "next.next.v"]);
    }

    #[test]
    fn a_default_that_recreates_its_own_parent_is_refused() {
        let node = json!({
            "type": "object",
            "properties": { "next": { "$ref": "#/$defs/node", "default": {} } },
        });
        let schema = json!({ "$defs": { "node": node }, "$ref": "#/$defs/node" });

        let error = prepare(schema, &json!({})).unwrap_err();

        match error {
            SchemaError::RecursiveDefault { path, reference } => {
                assert_eq!(path.map(|path| path.to_string()).as_deref(), Some("next.next"));
                assert_eq!(reference, "/$defs/node/properties/next/default");
            }
            other => panic!("expected RecursiveDefault, got {other:?}"),
        }
    }

    #[test]
    fn an_insertion_below_a_missing_parent_is_dropped() {
        let insertion = Insertion { steps: vec![Step::Key("gone".into()), Step::Key("a".into())], value: json!(1) };

        let value = apply(json!({ "kept": true }), &[insertion]);

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

    #[test]
    fn tuple_positions_are_never_inserted_but_their_objects_are_completed() {
        let schema = json!({
            "type": "object",
            "properties": {
                "pair": {
                    "type": "array",
                    "prefixItems": [
                        { "type": "number", "default": 0 },
                        { "type": "object", "properties": { "on": { "type": "boolean", "default": true } } },
                    ],
                },
            },
        });

        let short = json!({ "pair": [1] });
        assert!(prepare(schema.clone(), &short).unwrap().defaults.is_empty());
        let full = json!({ "pair": [null, {}] });
        let prepared = prepare(schema, &full).unwrap();
        assert_eq!(pointers(&prepared), ["pair.1.on"]);
        assert_eq!(prepared.candidate.into_owned(), json!({ "pair": [null, { "on": true }] }));
    }

    #[test]
    fn additional_property_values_are_completed() {
        let schema = json!({
            "type": "object",
            "properties": { "fixed": { "type": "number" } },
            "additionalProperties": { "type": "object", "properties": { "n": { "type": "number", "default": 2 } } },
        });
        let input = json!({ "fixed": 1, "extra": {} });

        let prepared = prepare(schema, &input).unwrap();

        assert_eq!(pointers(&prepared), ["extra.n"]);
    }

    #[test]
    fn a_schema_without_defaults_borrows_every_value() {
        let schema = json!({ "type": "object", "properties": { "a": { "type": "number" } } });
        let input = json!({ "a": null });

        let prepared = prepare(schema, &input).unwrap();

        assert!(matches!(prepared.candidate, Cow::Borrowed(_)));
    }

    #[test]
    fn check_declarations_reports_conflicts_on_nested_unions() {
        let schema = json!({
            "type": "object",
            "properties": {
                "shape": {
                    "anyOf": [
                        { "type": "object", "properties": { "n": { "type": "number", "default": 1 } } },
                        { "type": "object", "properties": { "n": { "type": "number", "default": 2 } } },
                    ],
                },
            },
        });

        let error = check_declarations(&schema).unwrap_err();

        match error {
            SchemaError::ConflictingDefaults { pointer, property, declarations } => {
                assert_eq!(pointer, "/properties/shape/anyOf/0");
                assert_eq!(property, "n");
                assert_eq!(
                    declarations,
                    ["/properties/shape/anyOf/0/properties/n/default", "/properties/shape/anyOf/1/properties/n/default"]
                );
            }
            other => panic!("expected ConflictingDefaults, got {other:?}"),
        }
    }
}
