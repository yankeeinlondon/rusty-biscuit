//! Narrowed types and the narrowing operations guards apply to them.

use serde_json::Value;

use super::GuardOrigin;
use super::projection::DeclaredType;
use crate::markdown::compose::expression::equality::values_equal;
use crate::markdown::schemas::completion::alternative_accepts;
use crate::markdown::schemas::{Constraint, PropertyAtom, PropertyDef, SimplifiedType, TypeExpr, flattened_alternatives};

/// The values a subject may hold at one site.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// No usable type. `non_null` is the internal fact that the value is not
    /// null: presence evidence only, never compatibility with a concrete type.
    Unknown { non_null: bool },
    /// A finite union of non-null alternatives, plus `null` when `nullable`.
    /// Array alternatives carry `is_array`.
    Arms { arms: Vec<PropertyAtom>, nullable: bool },
}

/// One runtime value family, as the type predicates test it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Family {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

/// Why a subject is present at a site.
#[derive(Debug, Clone, PartialEq)]
pub enum PresenceEvidence {
    /// The schema declares the property required.
    Declaration,
    /// A `ctx` descriptor is required or has a non-null default.
    Descriptor,
    /// The static frontmatter holds a concrete scalar, and nothing has
    /// written the property since.
    StaticValue,
    /// An enclosing guard proves it on every route to the site.
    Guard(GuardOrigin),
}

/// A subject's type at one site: its declaration, refined by every enclosing
/// guard since the last write.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopedType {
    declared: DeclaredType,
    shape: Shape,
    evidence: Vec<PresenceEvidence>,
    refinements: Vec<GuardOrigin>,
    written: bool,
}

#[allow(missing_docs)]
impl ScopedType {
    pub fn declared(&self) -> &DeclaredType {
        &self.declared
    }

    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    /// The guards that narrowed the declaration at this site.
    pub fn refinements(&self) -> &[GuardOrigin] {
        &self.refinements
    }

    pub fn is_refined(&self) -> bool {
        !self.refinements.is_empty()
    }

    /// Whether a possible write since the declared layer was built means the
    /// static frontmatter no longer describes the subject.
    pub fn was_written(&self) -> bool {
        self.written
    }

    pub fn presence_evidence(&self) -> &[PresenceEvidence] {
        &self.evidence
    }

    pub fn admits_null(&self) -> bool {
        self.shape.admits_null()
    }

    /// Whether the type excludes `null`: statically proven present. This is
    /// conditional evidence at the site; it never replaces checking a value.
    pub fn proves_present(&self) -> bool {
        !self.shape.admits_null()
    }

    /// The first guard that proves presence, when one does. Only this
    /// evidence, never a declaration, permits a discovery-time null to wait
    /// for its guarded site.
    pub fn guard_presence(&self) -> Option<&GuardOrigin> {
        self.evidence.iter().find_map(|evidence| match evidence {
            PresenceEvidence::Guard(origin) => Some(origin),
            _ => None,
        })
    }

    /// Whether `value` is one the type admits. Constraints of the remaining
    /// alternatives are checked; an unknown shape admits every non-null value.
    pub fn admits(&self, value: &Value) -> bool {
        match (&self.shape, value) {
            (shape, Value::Null) => shape.admits_null(),
            (Shape::Unknown { .. }, _) => true,
            (Shape::Arms { arms, .. }, value) => arms.iter().any(|arm| alternative_accepts(arm, value)),
        }
    }

    /// The non-null alternatives as a definition, or `None` for an unknown
    /// or null-only shape.
    pub fn narrowed_definition(&self) -> Option<PropertyDef> {
        match &self.shape {
            Shape::Arms { arms, .. } => match arms.as_slice() {
                [] => None,
                [single] => Some(PropertyDef::Single(single.clone())),
                arms => Some(PropertyDef::Union(arms.to_vec())),
            },
            Shape::Unknown { .. } => None,
        }
    }

    /// The type before any guard: the declaration, with descriptor and
    /// static-value presence unless the subject was written.
    pub(super) fn declared_base(declared: DeclaredType, static_scalar: bool, written: bool) -> Self {
        let mut evidence = Vec::new();
        let mut shape = match &declared {
            DeclaredType::Property(def) => {
                if def.is_required() {
                    evidence.push(PresenceEvidence::Declaration);
                }
                Shape::from_definition(def)
            }
            DeclaredType::Context { present } => {
                if *present {
                    evidence.push(PresenceEvidence::Descriptor);
                }
                Shape::Unknown { non_null: *present }
            }
            _ => Shape::Unknown { non_null: false },
        };
        if static_scalar && !written {
            evidence.push(PresenceEvidence::StaticValue);
            shape = shape.without_null();
        }
        Self {
            declared,
            shape,
            evidence,
            refinements: Vec::new(),
            written,
        }
    }

    /// This type narrowed to `shape` by the guard at `origin`, or `None`
    /// when the guard contradicts it (no value survives): an impossible
    /// branch keeps its incoming type and gains no presence evidence.
    pub(super) fn refined(&self, shape: Shape, origin: &GuardOrigin) -> Option<Self> {
        if shape.is_empty() {
            return None;
        }
        let mut refined = self.clone();
        if shape != self.shape {
            refined.refinements.push(origin.clone());
        }
        if !shape.admits_null() && refined.guard_presence().is_none() {
            refined.evidence.push(PresenceEvidence::Guard(origin.clone()));
        }
        refined.shape = shape;
        Some(refined)
    }

    /// What is known on either of two routes to one site.
    pub(super) fn join(&self, other: &Self) -> Self {
        let shape = self.shape.join(&other.shape);
        let guards_on_both = self.guard_presence().is_some() && other.guard_presence().is_some();
        let mut evidence: Vec<PresenceEvidence> = self
            .evidence
            .iter()
            .filter(|evidence| match evidence {
                PresenceEvidence::Guard(_) => guards_on_both,
                base => other.evidence.contains(base),
            })
            .cloned()
            .collect();
        if guards_on_both {
            for guard in &other.evidence {
                if matches!(guard, PresenceEvidence::Guard(_)) && !evidence.contains(guard) {
                    evidence.push(guard.clone());
                }
            }
        }
        if shape.admits_null() {
            evidence.clear();
        }
        let mut refinements = self.refinements.clone();
        for origin in &other.refinements {
            if !refinements.contains(origin) {
                refinements.push(origin.clone());
            }
        }
        Self {
            declared: self.declared.clone(),
            shape,
            evidence,
            refinements,
            written: self.written || other.written,
        }
    }
}

impl Shape {
    /// The shape of a declaration: its leaf alternatives, with `null` unless
    /// the shared presence rule makes the property required. An `unknown`
    /// alternative makes the whole shape unknown.
    pub(super) fn from_definition(def: &PropertyDef) -> Self {
        let mut arms = Vec::new();
        let mut nullable = !def.is_required();
        for alternative in flattened_alternatives(def) {
            let mut atom = alternative.atom.clone();
            atom.is_array |= alternative.is_array;
            match (&atom.ty, atom.is_array) {
                (TypeExpr::Primitive(SimplifiedType::Null), false) => nullable = true,
                (TypeExpr::Primitive(SimplifiedType::Unknown), false) => return Self::Unknown { non_null: false },
                _ => arms.push(atom),
            }
        }
        Self::Arms { arms, nullable }
    }

    pub fn admits_null(&self) -> bool {
        match self {
            Self::Unknown { non_null } => !non_null,
            Self::Arms { nullable, .. } => *nullable,
        }
    }

    fn is_empty(&self) -> bool {
        matches!(self, Self::Arms { arms, nullable: false } if arms.is_empty())
    }

    pub(super) fn without_null(&self) -> Self {
        match self {
            Self::Unknown { .. } => Self::Unknown { non_null: true },
            Self::Arms { arms, .. } => Self::Arms { arms: arms.clone(), nullable: false },
        }
    }

    pub(super) fn join(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Unknown { non_null: left }, Self::Unknown { non_null: right }) => {
                Self::Unknown { non_null: *left && *right }
            }
            (Self::Unknown { non_null }, Self::Arms { nullable, .. })
            | (Self::Arms { nullable, .. }, Self::Unknown { non_null }) => {
                Self::Unknown { non_null: *non_null && !nullable }
            }
            (Self::Arms { arms: left, nullable: left_null }, Self::Arms { arms: right, nullable: right_null }) => {
                let mut arms = left.clone();
                for arm in right {
                    if !arms.contains(arm) {
                        arms.push(arm.clone());
                    }
                }
                Self::Arms { arms, nullable: *left_null || *right_null }
            }
        }
    }

    /// The true side of a truthiness test: `null` and always-falsy literals
    /// removed. The false side learns nothing, since `0`, `''`, `false`, and
    /// empty containers are falsy too.
    pub(super) fn truthy(&self) -> Self {
        match self {
            Self::Unknown { .. } => Self::Unknown { non_null: true },
            Self::Arms { arms, .. } => Self::Arms {
                arms: arms
                    .iter()
                    .filter(|arm| !arm.literal_value().is_some_and(|value| !is_truthy_literal(value)))
                    .cloned()
                    .collect(),
                nullable: false,
            },
        }
    }

    /// Only `null`, when `null` is possible; `None` from unknown would need
    /// no narrowing.
    pub(super) fn only_null(&self) -> Self {
        Self::Arms { arms: Vec::new(), nullable: self.admits_null() }
    }

    /// The true side of a family predicate: the alternatives that may be of
    /// `family`, without `null`. From unknown, the bare family type.
    pub(super) fn of_family(&self, family: Family) -> Self {
        match self {
            Self::Unknown { .. } => Self::Arms { arms: vec![family.bare_atom()], nullable: false },
            Self::Arms { arms, .. } => Self::Arms {
                arms: arms.iter().filter(|arm| families(arm).contains(&family)).cloned().collect(),
                nullable: false,
            },
        }
    }

    /// The false side of a family predicate: alternatives that must be of
    /// `family` removed. Unknown stays unknown, since "not a string" has no
    /// representation.
    pub(super) fn without_family(&self, family: Family) -> Self {
        match self {
            Self::Unknown { .. } => self.clone(),
            Self::Arms { arms, nullable } => Self::Arms {
                arms: arms.iter().filter(|arm| families(arm) != [family].as_slice()).cloned().collect(),
                nullable: *nullable,
            },
        }
    }

    /// The true side of `x == value`: the values that may strictly equal
    /// `value` (`values_equal`).
    ///
    /// Against `null`, only `null`. Otherwise `null` is gone, alternatives of
    /// another runtime family are dropped, matching literals stay, and an
    /// unconstrained `string`, `enum`, `boolean`, or `number` narrows to the
    /// literal (a `number` only when that literal admits every storage of the
    /// value). Constrained alternatives and containers keep their arm.
    /// Unknown narrows the way an unconstrained alternative would.
    pub(super) fn equal_to(&self, value: &Value) -> Self {
        if value.is_null() {
            return self.only_null();
        }
        match self {
            Self::Unknown { .. } => Self::Arms { arms: vec![pinned_atom(value)], nullable: false },
            Self::Arms { arms, .. } => Self::Arms {
                arms: arms.iter().filter_map(|arm| arm_equal(arm, value)).collect(),
                nullable: false,
            },
        }
    }

    /// The true side of `x != value`: the values that may differ from it.
    /// Against `null`, every non-null value; otherwise literals (and `enum`
    /// members, and the matching `boolean`) that must equal it are removed,
    /// and `null` stays possible.
    pub(super) fn differs_from(&self, value: &Value) -> Self {
        if value.is_null() {
            return self.without_null();
        }
        match self {
            Self::Unknown { .. } => self.clone(),
            Self::Arms { arms, nullable } => Self::Arms {
                arms: arms.iter().filter_map(|arm| arm_differs(arm, value)).collect(),
                nullable: *nullable,
            },
        }
    }

    /// Every value of this shape, when it is a finite set of scalars: each
    /// alternative a literal, an `enum`, or a `boolean` with no ordinary
    /// constraints. `null` is included when nullable.
    pub(super) fn finite_values(&self) -> Option<Vec<Value>> {
        let Self::Arms { arms, nullable } = self else {
            return None;
        };
        let mut values = Vec::new();
        for arm in arms {
            if arm.is_array || has_ordinary_constraints(arm) {
                return None;
            }
            match arm.ty {
                TypeExpr::Primitive(SimplifiedType::Literal) => values.push(arm.literal_value()?.clone()),
                TypeExpr::Primitive(SimplifiedType::Enum) => {
                    values.extend(enum_members(arm)?.iter().map(|member| Value::String(member.clone())));
                }
                TypeExpr::Primitive(SimplifiedType::Boolean) => values.extend([Value::Bool(true), Value::Bool(false)]),
                _ => return None,
            }
        }
        if *nullable {
            values.push(Value::Null);
        }
        Some(values)
    }
}

impl Family {
    fn bare_atom(self) -> PropertyAtom {
        match self {
            Self::String => PropertyAtom::bare(SimplifiedType::String),
            Self::Number => PropertyAtom::bare(SimplifiedType::Number),
            Self::Boolean => PropertyAtom::bare(SimplifiedType::Boolean),
            Self::Object => PropertyAtom::bare(SimplifiedType::Object),
            Self::Array => {
                let mut atom = PropertyAtom::bare(SimplifiedType::Unknown);
                atom.is_array = true;
                atom
            }
        }
    }
}

const ALL_FAMILIES: &[Family] = &[Family::String, Family::Number, Family::Boolean, Family::Array, Family::Object];

/// The runtime families an alternative's values may belong to. A type whose
/// accepted values are not pinned to one family here keeps every family.
fn families(atom: &PropertyAtom) -> &'static [Family] {
    if atom.is_array {
        return &[Family::Array];
    }
    match &atom.ty {
        TypeExpr::InlineObject(_) => &[Family::Object],
        TypeExpr::Primitive(ty) => match ty {
            SimplifiedType::Literal => match atom.literal_value() {
                Some(Value::String(_)) => &[Family::String],
                Some(Value::Number(_)) => &[Family::Number],
                Some(Value::Bool(_)) => &[Family::Boolean],
                _ => ALL_FAMILIES,
            },
            SimplifiedType::Number => &[Family::Number],
            SimplifiedType::Boolean => &[Family::Boolean],
            SimplifiedType::Object => &[Family::Object],
            _ if crate::markdown::schemas::is_string_like(atom) => &[Family::String],
            _ => ALL_FAMILIES,
        },
        TypeExpr::Imported { .. } | TypeExpr::Union(_) => ALL_FAMILIES,
    }
}

fn is_truthy_literal(value: &Value) -> bool {
    crate::markdown::compose::expression::is_truthy(value)
}

/// Constraints that restrict values beyond the type identity. Markers and
/// annotations (`required`, defaults, suggestions, examples) do not.
fn has_ordinary_constraints(atom: &PropertyAtom) -> bool {
    atom.constraints.iter().chain(&atom.array_constraints).any(|constraint| {
        !matches!(
            constraint,
            Constraint::Required
                | Constraint::Default(_)
                | Constraint::Generated
                | Constraint::Eager
                | Constraint::Suggest(_)
                | Constraint::Example(_)
                | Constraint::Members(_)
                | Constraint::LiteralValue(_)
        )
    })
}

fn enum_members(atom: &PropertyAtom) -> Option<&[String]> {
    atom.constraints.iter().find_map(|constraint| match constraint {
        Constraint::Members(members) => Some(members.as_slice()),
        _ => None,
    })
}

fn literal_atom(value: Value) -> PropertyAtom {
    let mut atom = PropertyAtom::bare(SimplifiedType::Literal);
    atom.constraints.push(Constraint::LiteralValue(value));
    atom
}

fn with_members(atom: &PropertyAtom, members: Vec<String>) -> PropertyAtom {
    let mut atom = atom.clone();
    for constraint in &mut atom.constraints {
        if let Constraint::Members(existing) = constraint {
            *existing = members.clone();
        }
    }
    atom
}

impl Family {
    /// The family of a non-null value.
    fn of(value: &Value) -> Option<Self> {
        match value {
            Value::String(_) => Some(Self::String),
            Value::Number(_) => Some(Self::Number),
            Value::Bool(_) => Some(Self::Boolean),
            Value::Array(_) => Some(Self::Array),
            Value::Object(_) => Some(Self::Object),
            Value::Null => None,
        }
    }
}

/// The narrowest alternative holding every value strictly equal to the
/// non-null `value`: its literal, or for a container (and a number no
/// literal can pin) the bare family.
fn pinned_atom(value: &Value) -> PropertyAtom {
    match value {
        Value::String(_) | Value::Bool(_) => literal_atom(value.clone()),
        Value::Number(_) => pinned_number(value).unwrap_or_else(|| PropertyAtom::bare(SimplifiedType::Number)),
        _ => Family::of(value).unwrap_or(Family::Object).bare_atom(),
    }
}

/// `literal(value)`, when schema validation of that literal admits every
/// storage of the number that is strictly equal to it (`4` and `4.0`), so
/// the narrowed type never rejects a live value the evaluator sends there.
fn pinned_number(value: &Value) -> Option<PropertyAtom> {
    let atom = literal_atom(value.clone());
    equal_storages(value).iter().all(|storage| alternative_accepts(&atom, storage)).then_some(atom)
}

/// `value` together with its other `i64`/`u64`/`f64` storages that are
/// strictly equal to it.
fn equal_storages(value: &Value) -> Vec<Value> {
    let Value::Number(number) = value else {
        return vec![value.clone()];
    };
    let mut storages = vec![value.clone()];
    let float = number.as_f64().filter(|float| float.fract() == 0.0);
    let candidates = [
        float.and_then(serde_json::Number::from_f64),
        float.filter(|float| float.abs() < i64::MAX as f64).map(|float| serde_json::Number::from(float as i64)),
        float.filter(|float| *float >= 0.0 && *float < u64::MAX as f64).map(|float| serde_json::Number::from(float as u64)),
    ];
    for candidate in candidates.into_iter().flatten().map(Value::Number) {
        if values_equal(&candidate, value) && !storages.contains(&candidate) {
            storages.push(candidate);
        }
    }
    storages
}

fn arm_equal(arm: &PropertyAtom, value: &Value) -> Option<PropertyAtom> {
    if let Some(literal) = arm.literal_value() {
        return values_equal(literal, value).then(|| arm.clone());
    }
    if !families(arm).contains(&Family::of(value)?) {
        return None;
    }
    let plain = !has_ordinary_constraints(arm);
    if let (TypeExpr::Primitive(SimplifiedType::Enum), Value::String(text)) = (&arm.ty, value) {
        let members = enum_members(arm)?;
        return members.iter().any(|member| member == text).then(|| {
            if plain {
                literal_atom(value.clone())
            } else {
                with_members(arm, vec![text.clone()])
            }
        });
    }
    if arm.is_array || !plain {
        return Some(arm.clone());
    }
    match (&arm.ty, value) {
        (TypeExpr::Primitive(SimplifiedType::String | SimplifiedType::Boolean), _) => Some(literal_atom(value.clone())),
        (TypeExpr::Primitive(SimplifiedType::Number), _) => Some(pinned_number(value).unwrap_or_else(|| arm.clone())),
        _ => Some(arm.clone()),
    }
}

fn arm_differs(arm: &PropertyAtom, value: &Value) -> Option<PropertyAtom> {
    if let Some(literal) = arm.literal_value() {
        return (!values_equal(literal, value)).then(|| arm.clone());
    }
    if arm.is_array || has_ordinary_constraints(arm) {
        return Some(arm.clone());
    }
    match (&arm.ty, value) {
        (TypeExpr::Primitive(SimplifiedType::Enum), Value::String(text)) => {
            let remaining: Vec<String> = enum_members(arm)?.iter().filter(|member| *member != text).cloned().collect();
            match remaining.as_slice() {
                [] => None,
                [single] => Some(literal_atom(Value::String(single.clone()))),
                _ => Some(with_members(arm, remaining)),
            }
        }
        (TypeExpr::Primitive(SimplifiedType::Boolean), Value::Bool(flag)) => Some(literal_atom(Value::Bool(!flag))),
        _ => Some(arm.clone()),
    }
}
