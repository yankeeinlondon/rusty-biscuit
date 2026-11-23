//! The checked layer of a registered host's matched triggers: one host
//! layer built from every matched payload, or a `dm.host.trigger_conflict`
//! when two of them cannot be layered as one.
//!
//! Payloads are compared after named types and object composition resolved,
//! so a property inherited through `$extends` compares as declared. For each
//! top-level property two or more payloads declare, every pair is compared:
//! the underlying types by
//! [`is_definition_subtype_with_definitions`] in both directions, and
//! presence, null admission, defaults, descriptions, and every other
//! constraint (examples included) separately. Equivalent identical
//! declarations coalesce; a strictly narrower one wins when everything else
//! agrees; anything else is a conflict. The winner is the narrowest class,
//! represented by its first payload in bundle-path order, and every other
//! payload's declaration of the property is removed, so the payloads layer
//! in any order to the same schema and each property keeps its winner's
//! origin.
//!
//! A single matched payload keeps its root rules. When two or more
//! contribute, at most one may be a root union, none may carry root rules
//! (`$constraints`, pattern keys, or `closed`), and no other payload may
//! declare a key that defines the union's alternatives (one some arm omits,
//! or that differs by arm). Ordinary properties of the other payloads then
//! merge into each arm, arm order kept.
//!
//! The user-facing rules are `docs/topics/host-registration.md`
//! ("When two triggers declare one property").

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use simplified_schema::simplified::{DefKey, DefinitionTable};
use simplified_schema::triggers::assemble::ResolvedPayload;
use simplified_schema::{
    Constraint, PropertyAtom, PropertyDef, SchemaError, SchemaShape, TypeExpr, is_definition_subtype_with_definitions,
};

/// The code of a conflict between two matched triggers of one host.
pub const HOST_TRIGGER_CONFLICT_CODE: &str = "dm.host.trigger_conflict";

/// One matched payload's share of the host layer: its schema with every
/// declaration another payload won removed.
#[derive(Debug, Clone)]
pub struct CheckedPayload {
    /// The trigger's index in the profile's
    /// [`triggers`](super::RegisteredHostProfile::triggers).
    pub trigger: usize,
    /// The payload's JSON Schema, the resolved one itself when nothing was
    /// removed.
    pub json_schema: Arc<Value>,
    /// Its object shape, with the same declarations removed; `None` for a
    /// root union or a raw JSON Schema payload.
    pub shape: Option<Arc<SchemaShape>>,
    /// The inline arms of the host layer's root union, with the same
    /// declarations removed from every arm; `None` for an object payload.
    pub union_arms: Option<Arc<[SchemaShape]>>,
}

impl CheckedPayload {
    /// Whether the payload is the host layer's root union.
    #[must_use]
    pub fn is_union(&self) -> bool {
        self.union_arms.is_some()
    }
}

/// A host's matched payloads, checked: the object payloads in bundle-path
/// order, then the root union, if any.
#[derive(Debug, Clone, Default)]
pub struct CheckedLayer {
    pub payloads: Vec<CheckedPayload>,
}

/// Two matched triggers that cannot be layered as one host layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerConflict {
    pub first: PathBuf,
    pub second: PathBuf,
    /// The property, or `None` for the root rules.
    pub subject: Option<String>,
    pub reason: String,
}

impl TriggerConflict {
    /// The conflict as the document's schema preparation error.
    #[must_use]
    pub fn into_error(self, host: &str, document: Option<&Path>) -> SchemaError {
        SchemaError::HostTriggerConflict(Box::new(simplified_schema::HostTriggerConflict {
            document: document.map(Path::to_path_buf),
            host: host.to_string(),
            first: self.first,
            second: self.second,
            subject: self.subject,
            reason: self.reason,
        }))
    }
}

/// One matched, payload-bearing trigger to check.
#[derive(Clone, Copy)]
pub struct MatchedPayload<'a> {
    /// Its index in the profile's triggers.
    pub trigger: usize,
    /// The trigger file.
    pub file: &'a Path,
    /// Its concrete payload (a template already resolved).
    pub payload: &'a ResolvedPayload,
}

/// Checks `matched` (bundle-path order) and builds the host layer.
///
/// ## Errors
///
/// The first conflict found, comparing properties in the order the payloads
/// declare them.
pub fn check(matched: &[MatchedPayload<'_>]) -> Result<CheckedLayer, TriggerConflict> {
    let unchanged = |entry: &MatchedPayload<'_>| CheckedPayload {
        trigger: entry.trigger,
        json_schema: Arc::clone(&entry.payload.json_schema),
        shape: entry.payload.shape.clone(),
        union_arms: entry.payload.union_arms.clone(),
    };
    if matched.len() < 2 {
        return Ok(CheckedLayer { payloads: matched.iter().map(unchanged).collect() });
    }
    let views: Vec<View<'_>> = matched.iter().map(View::of).collect();
    check_roots(&views)?;
    let mut definitions = DefinitionTable::default();
    for entry in matched {
        definitions.absorb(&entry.payload.definitions);
    }

    // Every key in first-declaration order, with each view's declaration.
    let mut keys: Vec<&str> = Vec::new();
    for view in &views {
        for key in view.keys() {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }
    let mut losses: Vec<Vec<String>> = vec![Vec::new(); views.len()];
    for key in keys {
        let mut declared: Vec<(usize, Declaration<'_>)> = Vec::new();
        for (index, view) in views.iter().enumerate() {
            match view.declaration(key) {
                Some(Declared::Ordinary(declaration)) => declared.push((index, declaration)),
                Some(Declared::Alternative) => {
                    if let Some(other) = views.iter().enumerate().find(|(other, view)| *other != index && view.declares(key)) {
                        return Err(TriggerConflict {
                            first: view.file.to_path_buf(),
                            second: other.1.file.to_path_buf(),
                            subject: Some(key.to_string()),
                            reason: "the property defines the root union's alternatives, so no other matched trigger may \
                                     declare it"
                                .into(),
                        });
                    }
                }
                None => {}
            }
        }
        if declared.len() < 2 {
            continue;
        }
        let winner = narrowest(key, &declared, &views, &definitions)?;
        for (index, _) in declared.iter().filter(|(index, _)| *index != winner) {
            losses[*index].push(key.to_string());
        }
    }

    let mut payloads: Vec<CheckedPayload> = matched
        .iter()
        .zip(&losses)
        .map(|(entry, lost)| {
            if lost.is_empty() {
                return unchanged(entry);
            }
            let mut json = entry.payload.json_schema.as_ref().clone();
            remove_properties(&mut json, lost);
            let pruned = |shape: &SchemaShape| {
                let mut shape = shape.clone();
                shape.properties.retain(|name, _| !lost.contains(name));
                shape
            };
            let shape = entry.payload.shape.as_deref().map(|shape| Arc::new(pruned(shape)));
            let union_arms = entry.payload.union_arms.as_deref().map(|arms| arms.iter().map(pruned).collect());
            CheckedPayload { trigger: entry.trigger, json_schema: Arc::new(json), shape, union_arms }
        })
        .collect();
    // The union is the top of the host layer, where the others merge into
    // each of its arms.
    payloads.sort_by_key(CheckedPayload::is_union);
    Ok(CheckedLayer { payloads })
}

/// Refuses a second root union and root rules among two or more payloads.
fn check_roots(views: &[View<'_>]) -> Result<(), TriggerConflict> {
    let conflict = |first: &View<'_>, second: &View<'_>, reason: &str| TriggerConflict {
        first: first.file.to_path_buf(),
        second: second.file.to_path_buf(),
        subject: None,
        reason: reason.to_string(),
    };
    let unions: Vec<&View<'_>> = views.iter().filter(|view| view.union).collect();
    if let [first, second, ..] = unions.as_slice() {
        return Err(conflict(first, second, "both are root unions; one matched trigger at most may own the root union"));
    }
    if let Some(index) = views.iter().position(View::has_root_rules) {
        let other = views.iter().enumerate().find(|(other, _)| *other != index).map(|(_, view)| view).expect("two views");
        return Err(conflict(
            &views[index],
            other,
            "root rules (`$constraints`, pattern keys, or `closed`) are allowed only when one matched trigger contributes \
             a payload",
        ));
    }
    Ok(())
}

/// The winning declaration's view among `declared` (two or more), after
/// every pair proved compatible.
fn narrowest(
    key: &str,
    declared: &[(usize, Declaration<'_>)],
    views: &[View<'_>],
    definitions: &DefinitionTable,
) -> Result<usize, TriggerConflict> {
    let conflict = |a: usize, b: usize, reason: String| TriggerConflict {
        first: views[a].file.to_path_buf(),
        second: views[b].file.to_path_buf(),
        subject: Some(key.to_string()),
        reason,
    };
    // narrower[i][j]: declaration i is at least as narrow as j.
    let count = declared.len();
    let mut at_most = vec![vec![false; count]; count];
    for i in 0..count {
        at_most[i][i] = true;
        for j in i + 1..count {
            match relation(&declared[i].1, &declared[j].1, definitions) {
                Relation::Same => {
                    at_most[i][j] = true;
                    at_most[j][i] = true;
                }
                Relation::FirstNarrower => at_most[i][j] = true,
                Relation::SecondNarrower => at_most[j][i] = true,
                Relation::Conflict(reason) => return Err(conflict(declared[i].0, declared[j].0, reason)),
            }
        }
    }
    match (0..count).find(|&i| (0..count).all(|j| at_most[i][j])) {
        Some(i) => Ok(declared[i].0),
        None => Err(conflict(declared[0].0, declared[1].0, "no declaration is the narrowest of all".into())),
    }
}

/// How two declarations of one property compare.
enum Relation {
    Same,
    FirstNarrower,
    SecondNarrower,
    Conflict(String),
}

fn relation(a: &Declaration<'_>, b: &Declaration<'_>, definitions: &DefinitionTable) -> Relation {
    let (a, b) = match (a, b) {
        (Declaration::Typed(a), Declaration::Typed(b)) => (*a, *b),
        (Declaration::Raw { schema: a, required: ra }, Declaration::Raw { schema: b, required: rb }) => {
            return if a == b && ra == rb {
                Relation::Same
            } else {
                Relation::Conflict("raw JSON Schema declarations coalesce only when identical".into())
            };
        }
        _ => return Relation::Conflict("a raw JSON Schema declaration coalesces only with an identical one".into()),
    };
    let narrower = is_definition_subtype_with_definitions(a, b, definitions);
    let wider = is_definition_subtype_with_definitions(b, a, definitions);
    match (narrower, wider) {
        (true, true) => {
            if Same::new(definitions).definitions(a, b) {
                Relation::Same
            } else {
                Relation::Conflict(
                    rules_differ(a, b).unwrap_or_else(|| "their equivalent types differ in a nested declaration".into()),
                )
            }
        }
        (true, false) => rules_differ(a, b).map_or(Relation::FirstNarrower, Relation::Conflict),
        (false, true) => rules_differ(a, b).map_or(Relation::SecondNarrower, Relation::Conflict),
        (false, false) => Relation::Conflict("their types are incomparable".into()),
    }
}

/// How `a` and `b` disagree apart from their underlying types, or `None`
/// when they agree.
fn rules_differ(a: &PropertyDef, b: &PropertyDef) -> Option<String> {
    if a.is_required() != b.is_required() {
        return Some("one is required and the other is not".into());
    }
    if a.admits_null() != b.admits_null() {
        return Some("one admits null and the other does not".into());
    }
    let (rules_a, rules_b) = (rules(a), rules(b));
    if defaults(&rules_a) != defaults(&rules_b) {
        return Some("their defaults differ".into());
    }
    if !same_set(&descriptions(a), &descriptions(b)) {
        return Some("their descriptions differ".into());
    }
    if !same_set(&rules_a, &rules_b) {
        return Some("their constraints differ".into());
    }
    None
}

/// Every constraint of `definition` other than its type identity (`enum`
/// members, a `literal` value, `alphanumeric` options), which the subtype
/// relation already compared, and presence, compared on its own.
fn rules(definition: &PropertyDef) -> Vec<&Constraint> {
    atoms(definition)
        .iter()
        .flat_map(|atom| atom.constraints.iter().chain(&atom.array_constraints))
        .filter(|constraint| {
            !matches!(
                constraint,
                Constraint::Members(_) | Constraint::LiteralValue(_) | Constraint::Options(_) | Constraint::Required
            )
        })
        .collect()
}

fn defaults<'c>(rules: &[&'c Constraint]) -> Vec<&'c Value> {
    rules
        .iter()
        .filter_map(|constraint| match constraint {
            Constraint::Default(value) => Some(value),
            _ => None,
        })
        .collect()
}

fn descriptions(definition: &PropertyDef) -> Vec<&String> {
    atoms(definition).iter().filter_map(|atom| atom.description.as_ref()).collect()
}

fn atoms(definition: &PropertyDef) -> &[PropertyAtom] {
    match definition {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms,
    }
}

/// Whether `a` and `b` hold the same items, in any order.
fn same_set<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a.iter().all(|item| b.contains(item)) && b.iter().all(|item| a.contains(item))
}

/// Whether two greedy matchings of `a` onto `b` pair every item.
fn matched_all<T>(a: &[T], b: &[T], mut same: impl FnMut(&T, &T) -> bool) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut used = vec![false; b.len()];
    a.iter().all(|item| match (0..b.len()).find(|&index| !used[index] && same(item, &b[index])) {
        Some(index) => {
            used[index] = true;
            true
        }
        None => false,
    })
}

/// Structural identity of resolved declarations, independent of how their
/// stored definitions are keyed and of union-arm order. Coinductive over
/// recursive definition pairs.
struct Same<'t> {
    definitions: &'t DefinitionTable,
    in_progress: Vec<(DefKey, DefKey)>,
}

impl<'t> Same<'t> {
    fn new(definitions: &'t DefinitionTable) -> Self {
        Self { definitions, in_progress: Vec::new() }
    }

    fn definitions(&mut self, a: &PropertyDef, b: &PropertyDef) -> bool {
        matched_all(atoms(a), atoms(b), |x, y| self.atoms(x, y))
    }

    fn atoms(&mut self, a: &PropertyAtom, b: &PropertyAtom) -> bool {
        a.is_array == b.is_array
            && same_set(&a.constraints, &b.constraints)
            && same_set(&a.array_constraints, &b.array_constraints)
            && a.description == b.description
            && self.types(&a.ty, &b.ty)
    }

    fn shapes(&mut self, a: &SchemaShape, b: &SchemaShape) -> bool {
        a.properties.len() == b.properties.len()
            && a.pattern_keys == b.pattern_keys
            && same_set(&a.constraints, &b.constraints)
            && a.properties.iter().all(|(name, definition)| {
                b.properties.get(name).is_some_and(|other| self.definitions(definition, other))
            })
    }

    fn types(&mut self, a: &TypeExpr, b: &TypeExpr) -> bool {
        match (a, b) {
            (TypeExpr::Ref(x), TypeExpr::Ref(y)) if x == y => true,
            (TypeExpr::Ref(x), TypeExpr::Ref(y)) => {
                let pair = (x.clone(), y.clone());
                if self.in_progress.contains(&pair) {
                    return true;
                }
                let (Some(x), Some(y)) = (self.definitions.get(x), self.definitions.get(y)) else { return false };
                self.in_progress.push(pair);
                let same = self.definitions(&x.body, &y.body);
                self.in_progress.pop();
                same
            }
            (TypeExpr::Ref(key), other) | (other, TypeExpr::Ref(key)) => {
                // A stored definition equals an inline type only through a
                // plain single-atom body.
                match self.definitions.get(key).map(|definition| &definition.body) {
                    Some(PropertyDef::Single(atom))
                        if !atom.is_array && atom.constraints.is_empty() && atom.description.is_none() =>
                    {
                        let body = atom.ty.clone();
                        self.types(&body, other)
                    }
                    _ => false,
                }
            }
            (TypeExpr::InlineObject(x), TypeExpr::InlineObject(y)) => self.shapes(x, y),
            (TypeExpr::Union(x), TypeExpr::Union(y)) => matched_all(x, y, |p, q| self.atoms(p, q)),
            _ => a == b,
        }
    }
}

/// One declaration of a top-level property.
enum Declaration<'a> {
    Typed(&'a PropertyDef),
    /// A raw JSON Schema payload's property schema and whether it is
    /// required.
    Raw { schema: &'a Value, required: bool },
}

/// What a payload says about a key.
enum Declared<'a> {
    /// One declaration for the whole payload (every arm alike, for a union).
    Ordinary(Declaration<'a>),
    /// The key tells a root union's arms apart.
    Alternative,
}

/// One matched payload as the check reads it.
struct View<'a> {
    file: &'a Path,
    json: &'a Value,
    /// The object shape, or each root-union arm; empty for a raw payload.
    arms: Vec<&'a SchemaShape>,
    union: bool,
}

impl<'a> View<'a> {
    fn of(entry: &MatchedPayload<'a>) -> Self {
        let (arms, union) = match (&entry.payload.union_arms, &entry.payload.shape) {
            (Some(arms), _) => (arms.iter().collect(), true),
            (None, Some(shape)) => (vec![shape.as_ref()], false),
            (None, None) => (Vec::new(), false),
        };
        Self { file: entry.file, json: &entry.payload.json_schema, arms, union }
    }

    fn raw_properties(&self) -> Option<&'a serde_json::Map<String, Value>> {
        self.json.get("properties").and_then(Value::as_object)
    }

    fn keys(&self) -> Vec<&'a str> {
        if self.arms.is_empty() {
            return self.raw_properties().into_iter().flat_map(|properties| properties.keys()).map(String::as_str).collect();
        }
        let mut keys: Vec<&'a str> = Vec::new();
        for arm in &self.arms {
            for key in arm.properties.keys() {
                if !keys.contains(&key.as_str()) {
                    keys.push(key);
                }
            }
        }
        keys
    }

    fn declares(&self, key: &str) -> bool {
        self.keys().contains(&key)
    }

    fn declaration(&self, key: &str) -> Option<Declared<'a>> {
        if self.arms.is_empty() {
            let schema = self.raw_properties()?.get(key)?;
            let required = self
                .json
                .get("required")
                .and_then(Value::as_array)
                .is_some_and(|required| required.iter().any(|name| name == key));
            return Some(Declared::Ordinary(Declaration::Raw { schema, required }));
        }
        let declarations: Vec<Option<&'a PropertyDef>> = self.arms.iter().map(|arm| arm.properties.get(key)).collect();
        if declarations.iter().all(Option::is_none) {
            return None;
        }
        // An ordinary key is declared alike in every arm (the one arm of an
        // object payload).
        match declarations[0] {
            Some(first) if declarations.iter().all(|declaration| *declaration == Some(first)) => {
                Some(Declared::Ordinary(Declaration::Typed(first)))
            }
            _ => Some(Declared::Alternative),
        }
    }

    fn has_root_rules(&self) -> bool {
        let closed = |schema: &Value| schema.get("additionalProperties") == Some(&Value::Bool(false));
        let arms_closed = self.json.get("anyOf").and_then(Value::as_array).is_some_and(|arms| arms.iter().any(closed));
        closed(self.json)
            || arms_closed
            || self.arms.iter().any(|arm| !arm.constraints.is_empty() || !arm.pattern_keys.is_empty())
            || (self.arms.is_empty() && self.json.get("patternProperties").is_some())
    }
}

/// Removes `keys` from `schema`'s top-level properties (each arm's, for a
/// root union) and from the matching `required` lists.
pub(crate) fn remove_properties(schema: &mut Value, keys: &[String]) {
    let strip = |object: &mut Value| {
        if let Some(properties) = object.get_mut("properties").and_then(Value::as_object_mut) {
            properties.retain(|name, _| !keys.contains(name));
        }
        if let Some(required) = object.get_mut("required").and_then(Value::as_array_mut) {
            required.retain(|name| !name.as_str().is_some_and(|name| keys.iter().any(|key| key == name)));
        }
    };
    match schema.get_mut("anyOf").and_then(Value::as_array_mut) {
        Some(arms) => arms.iter_mut().for_each(strip),
        None => strip(schema),
    }
}
