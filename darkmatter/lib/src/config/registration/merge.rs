//! The checked layer of a registered host's matched triggers: one host
//! layer built from every matched payload, or a `dm.host.trigger_conflict`
//! when two of them cannot be layered as one.
//!
//! Payloads are compared after named types and object composition resolved,
//! so a property inherited through `$extends` compares as declared, and a
//! stored named type compares as the definition it references, with its
//! use site's description and rules (`resolved`). For each
//! top-level property two or more payloads declare, every pair is compared:
//! the underlying types by coverage in both directions, using the same
//! property and list item alternatives that pair descriptions and rules.
//! Presence, null admission, and the property's default compare separately
//! from descriptions and every other constraint (examples included) at the
//! alternative and level they govern: each alternative of the narrower
//! declaration against an alternative of the wider one that covers it, and
//! two lists by their list rules, then item alternative by item
//! alternative with their element rules. Equivalent identical
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

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use simplified_schema::simplified::{DefKey, DefinitionTable};
use simplified_schema::triggers::assemble::ResolvedPayload;
use simplified_schema::{Constraint, PropertyAtom, PropertyDef, SchemaError, SchemaShape, TypeExpr, is_subtype_with_definitions};

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
    /// The conflict as the document's schema preparation error, each
    /// trigger named by its valid display `name` in `profile`, when it
    /// declares one.
    #[must_use]
    pub fn into_error(self, profile: &super::RegisteredHostProfile, document: Option<&Path>) -> SchemaError {
        let name = |file: &Path| {
            profile.triggers.iter().find(|trigger| trigger.location == file).and_then(|trigger| trigger.trigger.envelope.name.clone())
        };
        SchemaError::HostTriggerConflict(Box::new(simplified_schema::HostTriggerConflict {
            document: document.map(Path::to_path_buf),
            host: profile.host().to_string(),
            first_name: name(&self.first),
            first: self.first,
            second_name: name(&self.second),
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

/// How `a` and `b` compare after resolving named types. Direction and
/// rule pairing use the same coverage relation on these alternatives.
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
    let (a, b) = (&resolved(a, definitions), &resolved(b, definitions));
    let (a_arms, b_arms) = (arms(a, definitions), arms(b, definitions));
    let narrower = covers_all(&b_arms, &a_arms, definitions);
    let wider = covers_all(&a_arms, &b_arms, definitions);
    if !narrower && !wider {
        return Relation::Conflict("their types are incomparable".into());
    }
    match (narrower, wider) {
        (true, true) => {
            if Same::new(definitions).definitions(a, b) {
                Relation::Same
            } else {
                Relation::Conflict(
                    rules_differ(a, b, &a_arms, &b_arms, definitions)
                        .unwrap_or_else(|| "their equivalent types differ in a nested declaration".into()),
                )
            }
        }
        (true, false) => rules_differ(a, b, &a_arms, &b_arms, definitions).map_or(Relation::FirstNarrower, Relation::Conflict),
        _ => rules_differ(b, a, &b_arms, &a_arms, definitions).map_or(Relation::SecondNarrower, Relation::Conflict),
    }
}

/// `definition` with every reference that is not recursive replaced by the
/// definition it names, in nested declarations too (object properties,
/// dictionary values, tuple elements), so a stored named type and the same
/// type written inline compare alike.
///
/// A use site's description and rules stay at the level the lowered schema
/// applies them: beside a reference to a list they are the list's
/// (`Models(min(1)) -> d` is `(…)[](min(1)) -> d`), otherwise they join the
/// value's own. A union reached this way, or written as a group, becomes
/// alternatives of the enclosing union when nothing but `required` rides on
/// it. A reference already being expanded on the current path stays a
/// reference, as does one whose expansion has no inline spelling (`[]` over
/// a list definition).
fn resolved(definition: &PropertyDef, definitions: &DefinitionTable) -> PropertyDef {
    Resolver { definitions, open: Vec::new() }.definition(definition)
}

struct Resolver<'t> {
    definitions: &'t DefinitionTable,
    /// The references being expanded on the current path.
    open: Vec<DefKey>,
}

impl Resolver<'_> {
    fn definition(&mut self, definition: &PropertyDef) -> PropertyDef {
        let mut alternatives = Vec::new();
        for atom in atoms(definition) {
            alternatives.extend(self.atom(atom));
        }
        match <[PropertyAtom; 1]>::try_from(alternatives) {
            Ok([single]) => PropertyDef::Single(single),
            Err(alternatives) => PropertyDef::Union(alternatives),
        }
    }

    /// `atom` resolved, as the alternatives it stands for.
    fn atom(&mut self, atom: &PropertyAtom) -> Vec<PropertyAtom> {
        if let TypeExpr::Ref(key) = &atom.ty {
            let expanded = (!self.open.contains(key))
                .then(|| self.definitions.get(key))
                .flatten()
                .and_then(|definition| expand(atom, &definition.body));
            let Some(expanded) = expanded else { return vec![atom.clone()] };
            self.open.push(key.clone());
            let alternatives = self.atom(&expanded);
            self.open.pop();
            return alternatives;
        }
        let mut atom = atom.clone();
        match &mut atom.ty {
            TypeExpr::InlineObject(shape) => {
                for definition in shape.properties.values_mut() {
                    *definition = self.definition(definition);
                }
                for pattern in &mut shape.pattern_keys {
                    pattern.def = self.definition(&pattern.def);
                }
            }
            TypeExpr::Tuple(tuple) => {
                for element in &mut tuple.elements {
                    element.def = self.definition(&element.def);
                }
                if let Some(spread) = &mut tuple.spread {
                    // A spread is one list atom; it keeps its own form.
                    if let [single] = self.atom(spread).as_slice() {
                        **spread = single.clone();
                    }
                }
            }
            TypeExpr::Union(arms) => {
                let mut flat = Vec::new();
                for arm in arms.iter() {
                    flat.extend(self.atom(arm));
                }
                *arms = flat;
            }
            _ => {}
        }
        let spliced = !atom.is_array
            && atom.description.is_none()
            && atom.constraints.iter().all(|constraint| matches!(constraint, Constraint::Required));
        match atom.ty {
            TypeExpr::Union(arms) if spliced => arms
                .into_iter()
                .map(|mut arm| {
                    arm.constraints.extend(atom.constraints.iter().cloned());
                    arm
                })
                .collect(),
            ty => vec![PropertyAtom { ty, ..atom }],
        }
    }
}

/// The inline atom a use of a stored definition whose body is `body`
/// stands for, with the use's description and rules; `None` when it has
/// none (`[]` over a list body, which the resolver refuses).
fn expand(usage: &PropertyAtom, body: &PropertyDef) -> Option<PropertyAtom> {
    let description = usage.description.clone();
    let joined = |own: &[Constraint], added: &[Constraint]| own.iter().chain(added).cloned().collect::<Vec<_>>();
    Some(match body {
        PropertyDef::Single(value) if usage.is_array && value.is_array => return None,
        PropertyDef::Single(list) if list.is_array => PropertyAtom {
            array_constraints: joined(&list.array_constraints, &usage.constraints),
            description,
            ..list.clone()
        },
        PropertyDef::Single(value) => PropertyAtom {
            ty: value.ty.clone(),
            is_array: usage.is_array,
            constraints: joined(&value.constraints, &usage.constraints),
            array_constraints: joined(&value.array_constraints, &usage.array_constraints),
            description,
        },
        PropertyDef::Union(arms) => PropertyAtom {
            ty: TypeExpr::Union(arms.clone()),
            is_array: usage.is_array,
            constraints: usage.constraints.clone(),
            array_constraints: usage.array_constraints.clone(),
            description,
        },
    })
}

/// How `narrow` (a subtype of `wide`, both [`resolved`]) disagrees with
/// `wide` apart from their underlying types, or `None` when it may replace
/// `wide` whole.
///
/// Presence (`required`, `generated`), null admission, and the property's
/// default belong to the whole property. Everything else belongs to one alternative: each alternative
/// of `narrow` must agree with an alternative of `wide` that covers it, so
/// a description or rule never moves to another alternative. Two lists are
/// compared at two levels: their own description and list rules
/// (`string[](…)`) once, then each item alternative of `narrow`'s list
/// against an item alternative of `wide`'s that covers it, with its element
/// rules (`string(…)[]`). An alternative of `wide` that `narrow` drops takes
/// its rules with it. An alternative no single alternative of `wide` covers
/// has no counterpart to agree with, so it conflicts: a list mixing strings
/// and numbers is no single one of a string list and a number list.
fn rules_differ(
    narrow: &PropertyDef,
    wide: &PropertyDef,
    narrow_arms: &[Arm<'_>],
    wide_arms: &[Arm<'_>],
    definitions: &DefinitionTable,
) -> Option<String> {
    if narrow.is_required() != wide.is_required() {
        return Some("one is required and the other is not".into());
    }
    if narrow.admits_null() != wide.admits_null() {
        return Some("one admits null and the other does not".into());
    }
    if is_generated(narrow) != is_generated(wide) {
        return Some("one is generated and the other is not".into());
    }
    if !same_set(&property_defaults(narrow_arms), &property_defaults(wide_arms)) {
        return Some("their defaults differ".into());
    }
    let many = narrow_arms.len() > 1;
    for (index, arm) in narrow_arms.iter().enumerate() {
        let reason = match paired(arm, wide_arms, definitions) {
            Ok(()) => continue,
            Err(Pairing::Uncovered(None)) => {
                return Some(format!("alternative {} narrows no single alternative of the other declaration", index + 1));
            }
            Err(Pairing::Uncovered(Some(item))) => format!("item alternative {item} narrows no single item alternative"),
            Err(Pairing::Differs(reason)) => reason,
        };
        return Some(if many { format!("{reason} at alternative {}", index + 1) } else { reason });
    }
    None
}

/// Why an alternative has no agreeing counterpart.
enum Pairing {
    /// No counterpart covers it; for a list's item alternative, its number.
    Uncovered(Option<usize>),
    /// Every covering counterpart disagrees; the first one's reason.
    Differs(String),
}

/// Whether some alternative of `wide` covers `narrow` and agrees with it.
fn paired(narrow: &Arm<'_>, wide: &[Arm<'_>], definitions: &DefinitionTable) -> Result<(), Pairing> {
    let mut first = None;
    for other in wide.iter().filter(|other| covers(other, narrow, definitions)) {
        match arm_differs(narrow, other, definitions) {
            None => return Ok(()),
            Some(pairing) => {
                first.get_or_insert(pairing);
            }
        }
    }
    Err(first.unwrap_or(Pairing::Uncovered(None)))
}

/// Each narrow alternative must fit a single wide alternative. In
/// particular, several homogeneous lists cannot cover one mixed list.
fn covers_all(wide: &[Arm<'_>], narrow: &[Arm<'_>], definitions: &DefinitionTable) -> bool {
    narrow.iter().all(|arm| wide.iter().any(|other| covers(other, arm, definitions)))
}

/// Type coverage before comparing rules. Lists keep item alternatives
/// inside the list so property-level coverage never distributes over them.
fn covers(wide: &Arm<'_>, narrow: &Arm<'_>, definitions: &DefinitionTable) -> bool {
    if narrow.atom.is_array && wide.atom.is_array {
        covers_all(&wide.items, &narrow.items, definitions)
    } else {
        is_subtype_with_definitions(&narrow.atom, &wide.atom, definitions)
    }
}

/// How two alternatives, `narrow` covered by `wide`, disagree: their
/// descriptions, value rules or list rules, each item alternative of two
/// lists against a covering one, and, for equivalent types, every nested
/// declaration.
fn arm_differs(narrow: &Arm<'_>, wide: &Arm<'_>, definitions: &DefinitionTable) -> Option<Pairing> {
    let differs = |reason: &str| Some(Pairing::Differs(reason.into()));
    if narrow.description != wide.description {
        return differs("their descriptions differ");
    }
    if !same_set(&narrow.rules, &wide.rules) {
        return differs(if narrow.item { "their element constraints differ" } else { "their constraints differ" });
    }
    if !same_set(&narrow.list_rules, &wide.list_rules) {
        return differs("their list constraints differ");
    }
    if narrow.atom.is_array && wide.atom.is_array {
        for (index, item) in narrow.items.iter().enumerate() {
            match paired(item, &wide.items, definitions) {
                Ok(()) => {}
                Err(Pairing::Uncovered(None)) => return Some(Pairing::Uncovered(Some(index + 1))),
                Err(pairing) => return Some(pairing),
            }
        }
        return None;
    }
    let equivalent = is_subtype_with_definitions(&wide.atom, &narrow.atom, definitions);
    if equivalent && !Same::new(definitions).types(&narrow.atom.ty, &wide.atom.ty) {
        return differs("their equivalent types differ in a nested declaration");
    }
    None
}

/// One alternative of a declaration, or of a list's items, with the rules
/// that govern it.
struct Arm<'a> {
    /// The alternative of the resolved declaration; a list item that is not
    /// a union is its list's atom without `[]`.
    atom: Cow<'a, PropertyAtom>,
    /// Whether it is an alternative of a list's items.
    item: bool,
    /// Its own description, or the nearest enclosing union's; a list item
    /// never inherits its list's, which the list compares itself.
    description: Option<&'a String>,
    /// The rules on its value, with those of every enclosing union; empty
    /// for a list, whose element rules its items carry.
    rules: Vec<&'a Constraint>,
    /// The rules on the list itself; empty for a scalar.
    list_rules: Vec<&'a Constraint>,
    /// A list's item alternatives; empty for a scalar.
    items: Vec<Arm<'a>>,
}

/// `definition`'s alternatives (a [`resolved`] declaration): each atom of a
/// property-level union, with a scalar union type (`string | number`) and a
/// remaining (recursive) reference to a union definition opened into their
/// arms, each definition at most once on a path. A list is one alternative
/// whose items open the same way: `(string | number)[]` has two item
/// alternatives, `string[]` one.
fn arms<'a>(definition: &'a PropertyDef, definitions: &'a DefinitionTable) -> Vec<Arm<'a>> {
    struct Walk<'a> {
        definitions: &'a DefinitionTable,
        opened: Vec<&'a DefKey>,
    }
    impl<'a> Walk<'a> {
        /// The alternatives `atom`'s type opens into, ignoring `[]`: a
        /// grouped union's, or those of a union definition it names that
        /// is not already open, with that definition.
        fn opening(&self, atom: &'a PropertyAtom) -> Option<(&'a [PropertyAtom], Option<&'a DefKey>)> {
            if let Some(inner) = atom.union_arms() {
                return Some((inner, None));
            }
            let TypeExpr::Ref(key) = &atom.ty else { return None };
            if self.opened.contains(&key) {
                return None;
            }
            let body = &self.definitions.get(key)?.body;
            let union = matches!(body, PropertyDef::Union(_))
                || matches!(body, PropertyDef::Single(single) if !single.is_array && single.union_arms().is_some());
            union.then(|| (atoms(body), Some(key)))
        }

        fn push(
            &mut self,
            atom: &'a PropertyAtom,
            item: bool,
            description: Option<&'a String>,
            inherited: &[&'a Constraint],
            out: &mut Vec<Arm<'a>>,
        ) {
            let description = atom.description.as_ref().or(description);
            let mut rules = inherited.to_vec();
            rules.extend(rules_of(&atom.constraints));
            let opening = self.opening(atom);
            if !atom.is_array {
                if let Some((inner, key)) = opening {
                    self.opened.extend(key);
                    inner.iter().for_each(|arm| self.push(arm, item, description, &rules, out));
                    if key.is_some() {
                        self.opened.pop();
                    }
                    return;
                }
                return out.push(Arm { atom: Cow::Borrowed(atom), item, description, rules, list_rules: Vec::new(), items: Vec::new() });
            }
            // `generated` marks the list whichever side of `[]` it is
            // written on (`string(generated)[]` is `string[](generated)`).
            let generated = is_generated_marker as fn(&&'a Constraint) -> bool;
            let mut list_rules: Vec<&'a Constraint> = rules_of(&atom.array_constraints).collect();
            list_rules.extend(rules.iter().copied().filter(generated));
            rules.retain(|constraint| !generated(constraint));
            let mut items = Vec::new();
            match opening {
                Some((inner, key)) => {
                    self.opened.extend(key);
                    inner.iter().for_each(|arm| self.push(arm, true, None, &rules, &mut items));
                    if key.is_some() {
                        self.opened.pop();
                    }
                }
                None => {
                    let single = PropertyAtom { is_array: false, array_constraints: Vec::new(), description: None, ..atom.clone() };
                    items.push(Arm { atom: Cow::Owned(single), item: true, description: None, rules, list_rules: Vec::new(), items: Vec::new() });
                }
            }
            out.push(Arm {
                atom: Cow::Borrowed(atom),
                item,
                description,
                rules: Vec::new(),
                list_rules,
                items,
            });
        }
    }
    let mut walk = Walk { definitions, opened: Vec::new() };
    let mut out = Vec::new();
    for atom in atoms(definition) {
        walk.push(atom, false, None, &[], &mut out);
    }
    out
}

/// Every constraint in `constraints` other than its type identity (`enum`
/// members, a `literal` value, `alphanumeric` options), which the subtype
/// relation already compared, and presence, compared on its own.
fn rules_of(constraints: &[Constraint]) -> impl Iterator<Item = &Constraint> {
    constraints.iter().filter(|constraint| {
        !matches!(constraint, Constraint::Members(_) | Constraint::LiteralValue(_) | Constraint::Options(_) | Constraint::Required)
    })
}

/// Whether `definition` marks its property as host-generated, from any
/// alternative and either side of `[]`.
fn is_generated(definition: &PropertyDef) -> bool {
    fn marked(atom: &PropertyAtom) -> bool {
        atom.property_constraints().any(|constraint| matches!(constraint, Constraint::Generated))
            || atom.union_arms().is_some_and(|arms| arms.iter().any(marked))
    }
    atoms(definition).iter().any(marked)
}

fn is_generated_marker(constraint: &&Constraint) -> bool {
    matches!(constraint, Constraint::Generated)
}

/// `constraints` without the presence markers (`generated`, `required`),
/// which [`Same`] compares on their own.
fn unmarked(constraints: &[Constraint]) -> Vec<&Constraint> {
    constraints
        .iter()
        .filter(|constraint| !is_generated_marker(constraint) && !matches!(constraint, Constraint::Required))
        .collect()
}

/// The property's own defaults: a scalar alternative's, and a list's
/// default for the whole list, never an element default.
fn property_defaults<'c>(arms: &[Arm<'c>]) -> Vec<&'c Value> {
    arms.iter()
        .flat_map(|arm| arm.rules.iter().chain(&arm.list_rules))
        .copied()
        .filter_map(|constraint| match constraint {
            Constraint::Default(value) => Some(value),
            _ => None,
        })
        .collect()
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

/// Structural identity of [`resolved`] declarations, independent of union-arm
/// order and of which side of `[]` carries a presence marker. A remaining
/// reference is recursive: it equals another reference whose resolved
/// definition is the same, and an inline type that equals its unfolding
/// (`Node` and `{ name: string, children: Node[] }`), so the depth at which
/// [`resolved`] stopped expanding either side never decides the result.
/// Both are coinductive over the operands being compared: a pair met again
/// while it is still being compared is assumed equal, and every first
/// visit compares its rules.
struct Same<'t> {
    definitions: &'t DefinitionTable,
    in_progress: Vec<(DefKey, DefKey)>,
    /// The alternative lists being compared through an unfolded reference.
    unfolding: Vec<(Vec<PropertyAtom>, Vec<PropertyAtom>)>,
}

impl<'t> Same<'t> {
    fn new(definitions: &'t DefinitionTable) -> Self {
        Self { definitions, in_progress: Vec::new(), unfolding: Vec::new() }
    }

    /// Whether the alternatives `a` and `b` pair one to one, as written or
    /// with their references unfolded.
    fn alternatives(&mut self, a: &[PropertyAtom], b: &[PropertyAtom]) -> bool {
        matched_all(a, b, |x, y| self.atoms(x, y)) || (a.iter().chain(b).any(is_reference) && self.unfolded(a, b))
    }

    /// Whether `a` and `b` pair one to one once each reference among them
    /// is replaced by the alternatives it [`resolved`] to, with its use
    /// site's rules (`expand`); `false` when none opens.
    fn unfolded(&mut self, a: &[PropertyAtom], b: &[PropertyAtom]) -> bool {
        let pair = (a.to_vec(), b.to_vec());
        if self.unfolding.contains(&pair) {
            return true;
        }
        let definitions = self.definitions;
        let unfold = |atoms: &[PropertyAtom]| -> Vec<PropertyAtom> {
            atoms
                .iter()
                .flat_map(|atom| {
                    if is_reference(atom) { Resolver { definitions, open: Vec::new() }.atom(atom) } else { vec![atom.clone()] }
                })
                .collect()
        };
        let (x, y) = (unfold(a), unfold(b));
        if x == a && y == b {
            return false;
        }
        self.unfolding.push(pair);
        let same = matched_all(&x, &y, |p, q| self.atoms(p, q));
        self.unfolding.pop();
        same
    }

    /// `required` belongs to the whole property, as in [`rules_differ`].
    fn definitions(&mut self, a: &PropertyDef, b: &PropertyDef) -> bool {
        a.is_required() == b.is_required()
            && is_generated(a) == is_generated(b)
            && self.alternatives(atoms(a), atoms(b))
    }

    /// `generated` marks an alternative whichever side of `[]` carries it.
    fn atoms(&mut self, a: &PropertyAtom, b: &PropertyAtom) -> bool {
        let generated = |atom: &PropertyAtom| atom.property_constraints().any(|constraint| is_generated_marker(&constraint));
        a.is_array == b.is_array
            && generated(a) == generated(b)
            && same_set(&unmarked(&a.constraints), &unmarked(&b.constraints))
            && same_set(&unmarked(&a.array_constraints), &unmarked(&b.array_constraints))
            && a.description == b.description
            && self.types(&a.ty, &b.ty)
    }

    fn shapes(&mut self, a: &SchemaShape, b: &SchemaShape) -> bool {
        a.properties.len() == b.properties.len()
            && matched_all(&a.pattern_keys, &b.pattern_keys, |x, y| x.key == y.key && self.definitions(&x.def, &y.def))
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
                let body = |key: &DefKey| {
                    let body = &self.definitions.get(key)?.body;
                    Some(Resolver { definitions: self.definitions, open: vec![key.clone()] }.definition(body))
                };
                let (Some(x), Some(y)) = (body(x), body(y)) else { return false };
                self.in_progress.push(pair);
                let same = self.definitions(&x, &y);
                self.in_progress.pop();
                same
            }
            // The rules beside these types were compared by the caller, so
            // each side is unfolded bare.
            (TypeExpr::Ref(_), other) => self.unfolded(&[bare(a)], &bare_alternatives(other)),
            (other, TypeExpr::Ref(_)) => self.unfolded(&bare_alternatives(other), &[bare(b)]),
            (TypeExpr::InlineObject(x), TypeExpr::InlineObject(y)) => self.shapes(x, y),
            (TypeExpr::Union(x), TypeExpr::Union(y)) => self.alternatives(x, y),
            (TypeExpr::Tuple(x), TypeExpr::Tuple(y)) => {
                x.elements.len() == y.elements.len()
                    && x.elements.iter().zip(&y.elements).all(|(p, q)| p.optional == q.optional && self.definitions(&p.def, &q.def))
                    && match (&x.spread, &y.spread) {
                        (None, None) => true,
                        (Some(p), Some(q)) => self.alternatives(std::slice::from_ref(p), std::slice::from_ref(q)),
                        _ => false,
                    }
            }
            _ => a == b,
        }
    }
}

fn is_reference(atom: &PropertyAtom) -> bool {
    matches!(atom.ty, TypeExpr::Ref(_))
}

/// `ty` as a scalar atom with no description or rules.
fn bare(ty: &TypeExpr) -> PropertyAtom {
    PropertyAtom { ty: ty.clone(), is_array: false, constraints: Vec::new(), array_constraints: Vec::new(), description: None }
}

/// The bare alternatives `ty` stands for: a union's arms, otherwise itself.
fn bare_alternatives(ty: &TypeExpr) -> Vec<PropertyAtom> {
    match ty {
        TypeExpr::Union(arms) => arms.clone(),
        ty => vec![bare(ty)],
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
    /// The payload's stored definitions, which its declarations reference.
    definitions: &'a DefinitionTable,
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
        Self { file: entry.file, json: &entry.payload.json_schema, definitions: &entry.payload.definitions, arms, union }
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
        // object payload). Alike is [`Same`] over the resolved declarations:
        // each inline arm stores the trigger's private types under its own
        // keys, and one arm may store a type another writes inline.
        let alike = |first: &PropertyDef, other: Option<&PropertyDef>| {
            other.is_some_and(|other| {
                other == first
                    || Same::new(self.definitions)
                        .definitions(&resolved(first, self.definitions), &resolved(other, self.definitions))
            })
        };
        match declarations[0] {
            Some(first) if declarations.iter().all(|declaration| alike(first, *declaration)) => {
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
