//! Replacing slots with the values a host produced for them: the
//! result-shape rules of `docs/topics/language/slots.md`.
//!
//! Values reach the schema as typed constraints, never as re-parsed text, so
//! a value holding `,`, `;`, `)`, or `{{` stays one value.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;

use super::{Template, collect_slots};
use crate::errors::{SchemaError, SlotResultKind, UnresolvedSlots};
use crate::numeric::{admit_json_number, format_number, json_number};
use crate::simplified::grammar::interpret_suggestion_parts;
use crate::simplified::{
    Constraint, DefinitionTable, DynamicElement, DynamicKind, DynamicList, FunctionParameters, PropertyAtom, PropertyDef,
    SchemaArm, SchemaRoot, SchemaShape, SimplifiedType, Slot, SuggestionCandidate, TypeExpr, TemplateConstraintKind,
};

/// The identity of one slot across the use sites it is inlined into: a
/// named type with a slot, used twice, is evaluated once.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlotKey {
    /// [`Slot::origin`].
    pub origin: Option<PathBuf>,
    /// Where the slot starts in its source ([`Slot::span`]).
    pub start: usize,
    /// [`Slot::expression`].
    pub expression: String,
}

impl SlotKey {
    /// The key of `slot`.
    pub fn of(slot: &Slot) -> Self {
        Self { origin: slot.origin.clone(), start: slot.span.start, expression: slot.expression.clone() }
    }
}

/// The values a host produced, by [`SlotKey`]. Cloning shares the map.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SlotValues {
    values: Arc<HashMap<SlotKey, Value>>,
}

impl SlotValues {
    /// Records `value` as `slot`'s, returning the value it replaced.
    pub fn insert(&mut self, slot: &Slot, value: Value) -> Option<Value> {
        Arc::make_mut(&mut self.values).insert(SlotKey::of(slot), value)
    }

    /// The value recorded for `slot`.
    pub fn get(&self, slot: &Slot) -> Option<&Value> {
        self.values.get(&SlotKey::of(slot))
    }

    #[allow(missing_docs)]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[allow(missing_docs)]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

thread_local! {
    static ACTIVE: RefCell<Option<SlotValues>> = const { RefCell::new(None) };
    static OMITTED: RefCell<Option<Arc<[String]>>> = const { RefCell::new(None) };
}

/// Runs `resolve` with the top-level properties `omitted` left out of the
/// next schema lowered on this thread (the replayed entry's), restoring what
/// was in force before.
pub(crate) fn omitting<T>(omitted: &Arc<[String]>, resolve: impl FnOnce() -> T) -> T {
    struct Restore(Option<Arc<[String]>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            OMITTED.with_borrow_mut(|active| *active = previous);
        }
    }
    let previous = OMITTED.with_borrow_mut(|active| active.replace(Arc::clone(omitted)));
    let _restore = Restore(previous);
    resolve()
}

/// Runs `resolve` with `values` in force on this thread, restoring what was
/// in force before, even when `resolve` panics. Every lowering site inside
/// substitutes from them right after import expansion.
pub fn with_values<T>(values: &SlotValues, resolve: impl FnOnce() -> T) -> T {
    struct Restore(Option<SlotValues>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            ACTIVE.with_borrow_mut(|active| *active = previous);
        }
    }
    let previous = ACTIVE.with_borrow_mut(|active| active.replace(values.clone()));
    let _restore = Restore(previous);
    resolve()
}

/// The values in force on this thread, if a [`with_values`] scope is running.
pub fn active_values() -> Option<SlotValues> {
    ACTIVE.with_borrow(Clone::clone)
}

/// What a lowering site runs after import expansion: inside a
/// [`with_values`] scope, [`substitute`]; outside one, a template is
/// [`SchemaError::UnresolvedSlots`] carrying its expanded form.
///
/// Inside an [`omitting`] scope, the first lowering takes the omitted
/// properties out first: a resolution lowers its entry schema once, after
/// every import is expanded, so that lowering is the replayed template's.
pub(crate) fn apply_active(root: &mut SchemaRoot, definitions: &mut DefinitionTable) -> Result<(), SchemaError> {
    if let Some(omitted) = OMITTED.with_borrow_mut(Option::take) {
        super::omit_properties(root, definitions, &omitted);
    }
    apply_values(root, definitions)
}

/// [`apply_active`] for one arm of a root union, which an [`omitting`]
/// scope never reaches. The union marks a template the arm reports as its
/// arm ([`Template::mark_union_arm`]), once it knows what every arm declares.
pub(crate) fn apply_active_to_arm(root: &mut SchemaRoot, definitions: &mut DefinitionTable) -> Result<(), SchemaError> {
    apply_values(root, definitions)
}

fn apply_values(root: &mut SchemaRoot, definitions: &mut DefinitionTable) -> Result<(), SchemaError> {
    let slots = collect_slots(root, definitions);
    if slots.is_empty() {
        return Ok(());
    }
    match active_values() {
        Some(values) => substitute(root, definitions, &values),
        None => Err(SchemaError::UnresolvedSlots(Box::new(UnresolvedSlots::with_template(
            slots,
            Template::new(root.clone(), Arc::new(definitions.clone())),
        )))),
    }
}

/// Replaces every dynamic constraint whose slots all have a value in
/// `values` with the concrete constraint it stands for; a constraint with a
/// slot `values` lacks is left as it is, so lowering still reports it.
///
/// - an `enum(...)` slot yields an array of strings, a `suggest(...)` slot an
///   array of strings, numbers, booleans, and nulls (a number as its
///   canonical text); each array is flattened one level, `[]` adds nothing,
///   the first occurrence of each value is kept, and a list left with no
///   element is [`SlotResultKind::Empty`];
/// - `literal(...)` takes a string, number, or boolean; `default(...)` any
///   value but `null`, judged against the property's type; `min`/`max` a
///   number (a non-negative integer where it counts); `pattern(...)` a string
///   the linear engine compiles ([`check_slot_pattern`]);
/// - a slot bound beside another bound of its kind keeps the stricter one,
///   and a lower bound above its upper is refused at the slot; a slot default
///   beside another default must equal it.
///
/// ## Errors
///
/// [`SchemaError::SlotResult`] at the first slot whose value cannot be used.
pub fn substitute(root: &mut SchemaRoot, definitions: &mut DefinitionTable, values: &SlotValues) -> Result<(), SchemaError> {
    let mut run = Substitution { values, defaults: Vec::new() };
    for body in definitions.bodies_mut() {
        run.def(body)?;
    }
    match root {
        SchemaRoot::Single(shape) => run.shape(shape)?,
        SchemaRoot::Union(arms) => {
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    run.shape(shape)?;
                }
            }
        }
    }
    let checks = std::mem::take(&mut run.defaults);
    // A default is judged by lowering its property, which needs every slot
    // gone; with one left, lowering reports it instead.
    if !checks.is_empty() && collect_slots(root, definitions).is_empty() {
        for check in checks {
            judge_default(check, definitions)?;
        }
    }
    Ok(())
}

/// A slot default to judge against its property once nothing is dynamic.
struct DefaultCheck {
    def: PropertyDef,
    value: Value,
    slot: Slot,
}

struct Substitution<'v> {
    values: &'v SlotValues,
    defaults: Vec<DefaultCheck>,
}

/// Which constraint list of an atom is being substituted.
#[derive(Clone, Copy, PartialEq, Eq)]
enum List {
    /// The value's (or, for an array, each item's) constraints.
    Value,
    /// An array's own constraints, where `min`/`max` count items.
    Array,
}

impl Substitution<'_> {
    fn shape(&mut self, shape: &mut SchemaShape) -> Result<(), SchemaError> {
        for def in shape.properties.values_mut() {
            self.def(def)?;
        }
        for pattern in &mut shape.pattern_keys {
            self.def(&mut pattern.def)?;
        }
        // Object-level constraints take no template constraint; a declaration option
        // may hold a nested definition that does.
        self.list(&mut shape.constraints, List::Value, SimplifiedType::Unknown)?;
        Ok(())
    }

    fn def(&mut self, def: &mut PropertyDef) -> Result<(), SchemaError> {
        let defaults = match def {
            PropertyDef::Single(atom) => self.atom(atom)?,
            PropertyDef::Union(atoms) => {
                let mut defaults = Vec::new();
                for atom in atoms {
                    defaults.extend(self.atom(atom)?);
                }
                defaults
            }
        };
        // A default belongs to the whole property, whichever atom wrote it.
        for (slot, value) in defaults {
            self.defaults.push(DefaultCheck { def: def.clone(), value, slot });
        }
        Ok(())
    }

    /// Substitutes `atom` and everything nested in it, returning the slot
    /// defaults its own lists took, for the enclosing property to judge.
    fn atom(&mut self, atom: &mut PropertyAtom) -> Result<Vec<(Slot, Value)>, SchemaError> {
        let mut defaults = Vec::new();
        match &mut atom.ty {
            TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Ref(_) => {}
            TypeExpr::InlineObject(shape) => self.shape(shape)?,
            TypeExpr::Union(atoms) => {
                for inner in atoms {
                    defaults.extend(self.atom(inner)?);
                }
            }
            TypeExpr::Tuple(tuple) => {
                for element in &mut tuple.elements {
                    self.def(&mut element.def)?;
                }
                if let Some(spread) = &mut tuple.spread {
                    defaults.extend(self.atom(spread)?);
                }
            }
            TypeExpr::Function(function) => {
                if let FunctionParameters::List(parameters) = &mut function.parameters {
                    for parameter in parameters {
                        self.def(&mut parameter.def)?;
                    }
                }
                self.def(&mut function.returns)?;
            }
        }
        let target = match &atom.ty {
            TypeExpr::Primitive(primitive) => *primitive,
            _ => SimplifiedType::Unknown,
        };
        defaults.extend(self.list(&mut atom.constraints, List::Value, target)?);
        defaults.extend(self.list(&mut atom.array_constraints, List::Array, target)?);
        Ok(defaults)
    }

    /// Substitutes one constraint list, then re-applies the merge rules a
    /// slot bound or default skipped while it was unknown.
    fn list(
        &mut self,
        constraints: &mut Vec<Constraint>,
        list: List,
        atom_type: SimplifiedType,
    ) -> Result<Vec<(Slot, Value)>, SchemaError> {
        let mut substituted: Vec<(usize, Slot)> = Vec::new();
        for (index, constraint) in constraints.iter_mut().enumerate() {
            let replacement = match constraint {
                Constraint::Property { declaration, .. } => {
                    self.def(declaration)?;
                    None
                }
                Constraint::Dynamic(dynamic) => self.dynamic_list(dynamic)?,
                Constraint::TemplateConstraint { kind, target, slot } => match self.values.get(slot) {
                    Some(value) => {
                        let target = if *target == SimplifiedType::Unknown { atom_type } else { *target };
                        let concrete = value_constraint(*kind, target, list, slot, value)?;
                        substituted.push((index, slot.clone()));
                        Some(concrete)
                    }
                    None => None,
                },
                _ => None,
            };
            if let Some(replacement) = replacement {
                *constraint = replacement;
            }
        }
        if substituted.is_empty() {
            return Ok(Vec::new());
        }
        reconcile(constraints, substituted)
    }

    /// The concrete list `dynamic` stands for, or `None` while one of its
    /// slots has no value.
    fn dynamic_list(&self, dynamic: &DynamicList) -> Result<Option<Constraint>, SchemaError> {
        let mut produced = Vec::with_capacity(dynamic.elements.len());
        for element in &dynamic.elements {
            match element {
                DynamicElement::Slot(slot) => match self.values.get(slot) {
                    Some(value) => produced.push(Produced::Slot(slot, value)),
                    None => return Ok(None),
                },
                DynamicElement::Member(member) => produced.push(Produced::Member(member)),
                DynamicElement::Suggestion(candidate) => produced.push(Produced::Suggestion(candidate)),
            }
        }
        let first_slot = || dynamic.slots().next().expect("a dynamic list holds a slot").clone();
        match dynamic.kind {
            DynamicKind::Members => {
                let mut members: Vec<String> = Vec::new();
                for element in produced {
                    match element {
                        Produced::Member(member) => push_member(&mut members, member.clone()),
                        Produced::Slot(slot, value) => {
                            for item in slot_array(slot, value, "an array of strings")? {
                                let Value::String(member) = item else {
                                    return Err(wrong_shape(slot, "an array of strings", &format!("an array holding a {}", json_kind(item))));
                                };
                                push_member(&mut members, member.clone());
                            }
                        }
                        Produced::Suggestion(_) => unreachable!("an enum list holds no suggestion"),
                    }
                }
                if members.is_empty() {
                    return Err(slot_error(first_slot(), SlotResultKind::Empty));
                }
                Ok(Some(Constraint::Members(members)))
            }
            DynamicKind::Suggest => {
                let mut candidates: Vec<SuggestionCandidate> = Vec::new();
                for element in produced {
                    match element {
                        Produced::Suggestion(candidate) => push_candidate(&mut candidates, candidate.clone()),
                        Produced::Slot(slot, value) => {
                            let expected = "an array of strings, numbers, booleans, or nulls";
                            for item in slot_array(slot, value, expected)? {
                                let text = match item {
                                    Value::String(text) => text.clone(),
                                    Value::Number(number) => format_number(admit_number(slot, number)?),
                                    Value::Bool(flag) => flag.to_string(),
                                    Value::Null => "null".to_string(),
                                    other => {
                                        return Err(wrong_shape(slot, expected, &format!("an array holding a {}", json_kind(other))));
                                    }
                                };
                                push_candidate(
                                    &mut candidates,
                                    interpret_suggestion_parts(&text, slot.span.clone(), dynamic.target),
                                );
                            }
                        }
                        Produced::Member(_) => unreachable!("a suggestion list holds no member"),
                    }
                }
                if candidates.is_empty() {
                    return Err(slot_error(first_slot(), SlotResultKind::Empty));
                }
                Ok(Some(Constraint::Suggest(candidates)))
            }
        }
    }
}

enum Produced<'a> {
    Member(&'a String),
    Suggestion(&'a SuggestionCandidate),
    Slot(&'a Slot, &'a Value),
}

fn push_member(members: &mut Vec<String>, member: String) {
    if !members.contains(&member) {
        members.push(member);
    }
}

fn push_candidate(candidates: &mut Vec<SuggestionCandidate>, candidate: SuggestionCandidate) {
    if !candidates.iter().any(|existing| existing.interpreted == candidate.interpreted) {
        candidates.push(candidate);
    }
}

/// A list slot's value: an array, never `null` or a scalar.
fn slot_array<'v>(slot: &Slot, value: &'v Value, expected: &'static str) -> Result<&'v [Value], SchemaError> {
    match value {
        Value::Array(items) => Ok(items),
        other => Err(wrong_shape(slot, expected, json_kind(other))),
    }
}

/// The concrete constraint a template-constraint slot stands for.
fn value_constraint(
    kind: TemplateConstraintKind,
    target: SimplifiedType,
    list: List,
    slot: &Slot,
    value: &Value,
) -> Result<Constraint, SchemaError> {
    match kind {
        TemplateConstraintKind::Literal => match value {
            Value::String(_) | Value::Bool(_) => Ok(Constraint::LiteralValue(value.clone())),
            Value::Number(number) => Ok(Constraint::LiteralValue(admitted_number(slot, number)?)),
            other => Err(wrong_shape(slot, "a string, number, or boolean", json_kind(other))),
        },
        TemplateConstraintKind::Default => match value {
            Value::Null => Err(wrong_shape(slot, "a value other than null", "null")),
            Value::Number(number) => Ok(Constraint::Default(admitted_number(slot, number)?)),
            other => Ok(Constraint::Default(other.clone())),
        },
        TemplateConstraintKind::Min | TemplateConstraintKind::Max => {
            let Value::Number(number) = value else {
                return Err(wrong_shape(slot, "a number", json_kind(value)));
            };
            let bound = admit_number(slot, number)?;
            let lower = kind == TemplateConstraintKind::Min;
            let keyword = kind.as_keyword();
            Ok(if list == List::Array {
                let count = count(slot, keyword, bound)?;
                if lower { Constraint::MinItems(count) } else { Constraint::MaxItems(count) }
            } else if matches!(target, SimplifiedType::String | SimplifiedType::Alphanumeric) {
                let count = count(slot, keyword, bound)?;
                if lower { Constraint::MinLen(count) } else { Constraint::MaxLen(count) }
            } else if lower {
                Constraint::Min(bound)
            } else {
                Constraint::Max(bound)
            })
        }
        TemplateConstraintKind::Pattern => match value {
            Value::String(pattern) => {
                check_slot_pattern(pattern).map_err(|reason| slot_error(slot.clone(), SlotResultKind::PatternRejected(reason)))?;
                Ok(Constraint::Pattern(pattern.clone()))
            }
            other => Err(wrong_shape(slot, "a string", json_kind(other))),
        },
    }
}

/// The most source text a slot-supplied pattern may have.
pub const MAX_SLOT_PATTERN_BYTES: usize = 4 * 1024;

/// Whether a pattern a slot supplied may be used: the data must never move
/// a schema onto the backtracking engine that lookaround needs, nor hand the
/// linear engine a pattern it refuses.
///
/// ## Errors
///
/// Why the pattern is refused: a lookaround or backreference, more than
/// [`MAX_SLOT_PATTERN_BYTES`] of source, or a pattern
/// [`regex::RegexBuilder`] rejects under its default size limit.
pub fn check_slot_pattern(pattern: &str) -> Result<(), String> {
    if pattern.len() > MAX_SLOT_PATTERN_BYTES {
        return Err(format!(
            "a pattern from a slot may be at most {MAX_SLOT_PATTERN_BYTES} bytes; this one is {}",
            pattern.len()
        ));
    }
    if ["(?=", "(?!", "(?<=", "(?<!"].iter().any(|opener| pattern.contains(opener)) {
        return Err("a pattern from a slot cannot use lookaround".to_string());
    }
    if has_backreference(pattern) {
        return Err("a pattern from a slot cannot use a backreference".to_string());
    }
    regex::RegexBuilder::new(pattern)
        .build()
        .map(|_| ())
        .map_err(|error| format!("the pattern does not compile: {error}"))
}

/// `\1`…`\9` or `\k<name>` outside an escaped backslash.
fn has_backreference(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b'\\' {
            match bytes[index + 1] {
                b'1'..=b'9' => return true,
                b'k' if bytes.get(index + 2) == Some(&b'<') => return true,
                _ => index += 2,
            }
        } else {
            index += 1;
        }
    }
    false
}

/// A bound a list holds: its family, direction, and value.
fn bound_of(constraint: &Constraint) -> Option<(u8, bool, f64)> {
    Some(match constraint {
        Constraint::Min(n) => (0, true, *n),
        Constraint::Max(n) => (0, false, *n),
        Constraint::MinLen(n) => (1, true, *n as f64),
        Constraint::MaxLen(n) => (1, false, *n as f64),
        Constraint::MinItems(n) => (2, true, *n as f64),
        Constraint::MaxItems(n) => (2, false, *n as f64),
        _ => return None,
    })
}

/// Applies the rules `resolve.rs::merge_constraints` applies to concrete
/// constraints to the ones substituted at `substituted` (index, slot): the
/// stricter of two bounds of a kind stays, crossing bounds are refused at
/// the slot, and a second default must equal the first. Returns the slot
/// defaults that remain, for judging against the property.
fn reconcile(constraints: &mut Vec<Constraint>, substituted: Vec<(usize, Slot)>) -> Result<Vec<(Slot, Value)>, SchemaError> {
    let mut tagged: Vec<(Constraint, Option<Slot>)> = std::mem::take(constraints).into_iter().map(|c| (c, None)).collect();
    for (index, slot) in substituted {
        tagged[index].1 = Some(slot);
    }
    let mut kept: Vec<(Constraint, Option<Slot>)> = Vec::with_capacity(tagged.len());
    let mut defaults = Vec::new();
    for (constraint, slot) in tagged {
        if let Some((family, lower, value)) = bound_of(&constraint) {
            let existing = kept.iter().position(|(other, _)| {
                bound_of(other).is_some_and(|(other_family, other_lower, _)| other_family == family && other_lower == lower)
            });
            if let Some(position) = existing {
                let (_, _, current) = bound_of(&kept[position].0).expect("a bound");
                let stricter = if lower { value > current } else { value < current };
                if stricter {
                    kept[position] = (constraint, slot);
                }
                continue;
            }
        }
        if let Constraint::Default(value) = &constraint {
            let existing = kept.iter().find_map(|(other, _)| match other {
                Constraint::Default(other) => Some(other.clone()),
                _ => None,
            });
            if let Some(existing) = existing {
                if crate::simplified::convert::normalize_json_number(existing.clone())
                    != crate::simplified::convert::normalize_json_number(value.clone())
                {
                    let slot = slot.or_else(|| kept.iter().rev().find_map(|(_, slot)| slot.clone())).expect("a slot default");
                    return Err(slot_error(
                        slot,
                        SlotResultKind::Inadmissible(format!("the default {value} disagrees with the default {existing}")),
                    ));
                }
                continue;
            }
            if let Some(slot) = &slot {
                defaults.push((slot.clone(), value.clone()));
            }
        }
        kept.push((constraint, slot));
    }
    for family in 0..3u8 {
        let find = |lower: bool| {
            kept.iter().find_map(|(constraint, slot)| {
                bound_of(constraint)
                    .filter(|(other_family, other_lower, _)| *other_family == family && *other_lower == lower)
                    .map(|(_, _, value)| (value, slot.clone()))
            })
        };
        if let (Some((low, low_slot)), Some((high, high_slot))) = (find(true), find(false))
            && low > high
        {
            // The later of the two slots is the one reported.
            let slot = match (low_slot, high_slot) {
                (Some(a), Some(b)) => if a.span.start > b.span.start { a } else { b },
                (Some(slot), None) | (None, Some(slot)) => slot,
                (None, None) => continue,
            };
            return Err(slot_error(
                slot,
                SlotResultKind::Inadmissible(format!(
                    "the lower bound {} is above the upper bound {}",
                    format_number(low),
                    format_number(high)
                )),
            ));
        }
    }
    *constraints = kept.into_iter().map(|(constraint, _)| constraint).collect();
    Ok(defaults)
}

/// Judges a slot default against its property by lowering the property and
/// validating the default with the structural validator.
fn judge_default(check: DefaultCheck, definitions: &DefinitionTable) -> Result<(), SchemaError> {
    let DefaultCheck { def, value, slot } = check;
    let mut shape = SchemaShape::default();
    shape.properties.insert("value".to_string(), def);
    let schema = crate::simplified::to_json_schema_with_definitions(&SchemaRoot::Single(shape), definitions)?;
    let validator = crate::validate::build_structural_validator(&schema)?;
    let document = serde_json::json!({ "value": value });
    if let Some(error) = validator.iter_errors(&document).next() {
        return Err(slot_error(
            slot,
            SlotResultKind::Inadmissible(format!("the default {value} is not a valid value of the property: {error}")),
        ));
    }
    Ok(())
}

fn count(slot: &Slot, keyword: &str, bound: f64) -> Result<usize, SchemaError> {
    if bound < 0.0 || bound.fract() != 0.0 {
        return Err(slot_error(
            slot.clone(),
            SlotResultKind::Inadmissible(format!("`{keyword}` here counts, so it takes a non-negative integer, not {}", format_number(bound))),
        ));
    }
    Ok(bound as usize)
}

fn admit_number(slot: &Slot, number: &serde_json::Number) -> Result<f64, SchemaError> {
    admit_json_number(number)
        .map_err(|error| slot_error(slot.clone(), SlotResultKind::Inadmissible(format!("the number {number} is not admitted: {error}"))))
}

fn admitted_number(slot: &Slot, number: &serde_json::Number) -> Result<Value, SchemaError> {
    let admitted = admit_number(slot, number)?;
    Ok(json_number(admitted).map_or(Value::Null, Value::Number))
}

fn wrong_shape(slot: &Slot, expected: &'static str, found: &str) -> SchemaError {
    slot_error(slot.clone(), SlotResultKind::WrongShape { expected, found: found.to_string() })
}

fn slot_error(slot: Slot, kind: SlotResultKind) -> SchemaError {
    SchemaError::SlotResult { slot: Box::new(slot), kind }
}

/// The JSON kind of `value`, as diagnostics name it.
pub(crate) fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "mapping",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_pattern_is_refused_for_lookaround_backreferences_size_and_compile_limits() {
        assert_eq!(check_slot_pattern("^[a-z][a-z0-9-]*$"), Ok(()));
        assert_eq!(check_slot_pattern(r"^\\1$"), Ok(()), "an escaped backslash before a digit is no backreference");
        for lookaround in ["^(?=a)", "^(?!a)", "(?<=a)b", "(?<!a)b"] {
            assert!(check_slot_pattern(lookaround).unwrap_err().contains("lookaround"), "{lookaround}");
        }
        for backreference in [r"(a)\1", r"(?<x>a)\k<x>"] {
            assert!(check_slot_pattern(backreference).unwrap_err().contains("backreference"), "{backreference}");
        }
        let long = "a".repeat(MAX_SLOT_PATTERN_BYTES + 1);
        assert!(check_slot_pattern(&long).unwrap_err().contains("at most 4096 bytes"));
        assert!(check_slot_pattern(&"a".repeat(MAX_SLOT_PATTERN_BYTES)).is_ok(), "the cap is inclusive");
        let compiled = check_slot_pattern(r"(?:\w{100}){100}").unwrap_err();
        assert!(compiled.contains("does not compile"), "{compiled}");
        assert!(check_slot_pattern("(unclosed").unwrap_err().contains("does not compile"));
    }
}
