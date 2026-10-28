//! Declared projection of collection types: what iterating a source
//! yields, how a callback pattern binds an entry, and the result shapes of
//! computed object and array literals.
//!
//! Everything here is passive. A source is typed from its declaration and
//! the guards that enclose it, never from a sampled value, and a callback is
//! typed by its pattern and body without running either.
//!
//! An entry is one [`SourceProjection::Entries`] shape: the element type of
//! an array or tuple, or `{key: string, value: V}` for an object, where `V`
//! joins the types of the properties that are present. An optional property
//! that is absent yields no entry, so its value type carries no `null` of its
//! own, while a declared `null` is preserved.
//!
//! The entry a callback binds is one [`ItemFrame`], so every callback that
//! names the same original item differently shares its facts, and a map
//! result is a different frame.

use std::sync::Arc;

use super::projection::item_shape;
use super::{LexicalTypes, Polarity, Shape, TypeScope, TypedLocal};
use crate::markdown::compose::expression::ast::{BindingPattern, Callback, SpannedCallback};
use crate::markdown::compose::expression::binding::{LocalId, next_frame_id};
use crate::markdown::compose::expression::{Expr, expression_type};
use crate::markdown::schemas::simplified::serialize_property_atom;
use crate::markdown::schemas::{
    Constraint, DefinitionTable, PatternKey, PropertyAtom, PropertyDef, SchemaShape, SimplifiedType, TypeExpr,
    flattened_alternatives,
};

/// What iterating a declared source yields.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceProjection {
    /// An iterable declaration. The shape is the type of one entry: an
    /// element, or a `{key, value}` object. A source that is only `null`
    /// yields no entry, so its shape admits no value.
    Entries(Shape),
    /// No usable declaration: an undeclared or `unknown` source, or a named
    /// type that cannot be resolved. This is the only case a recovery may
    /// waive; its entries are unknown elements or `{key, value: unknown}`.
    UnknownShape,
    /// A declaration with a non-null alternative that is never iterable. Each
    /// text names one such alternative. A guard must narrow it away at the
    /// iteration site; a recovery never excuses it.
    Unsuitable(Vec<String>),
}

impl SourceProjection {
    /// The entries a recovery assumes for an unknown source: unknown elements
    /// or `{key: string, value: unknown}` objects, joined as `unknown`.
    pub fn recovered_entries() -> Shape {
        Shape::Unknown { non_null: false }
    }
}

/// Why a pattern cannot bind an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternMismatch {
    /// An object pattern over an entry no object alternative of which can
    /// hold fields.
    NotAnObject {
        /// The entry type, as written.
        entry: String,
    },
    /// An object pattern naming a field no object alternative declares.
    MissingField {
        /// The undeclared field.
        field: String,
        /// The entry type, as written.
        entry: String,
    },
}

impl std::fmt::Display for PatternMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAnObject { entry } => write!(f, "a field pattern needs an object, but the entry is `{entry}`"),
            Self::MissingField { field, entry } => write!(f, "the entry `{entry}` declares no field `{field}`"),
        }
    }
}

impl std::error::Error for PatternMismatch {}

/// The projection of `shape`, a source's type at its site. See
/// [`SourceProjection`]. `definitions` resolves named types: pass
/// [`TypeScope::definitions`] so the effective declaration owner answers.
pub fn project_source(shape: &Shape, definitions: &DefinitionTable) -> SourceProjection {
    let Shape::Arms { arms, .. } = shape else {
        return SourceProjection::UnknownShape;
    };
    let mut entries = empty();
    let mut unsuitable = Vec::new();
    let mut unresolved = false;
    for arm in arms {
        match arm_entries(arm, definitions, definitions.len()) {
            ArmEntries::Entries(shape) => entries = entries.union(&shape),
            ArmEntries::Unresolved => unresolved = true,
            ArmEntries::Scalar(text) => unsuitable.push(text),
        }
    }
    if !unsuitable.is_empty() {
        SourceProjection::Unsuitable(unsuitable)
    } else if unresolved {
        SourceProjection::UnknownShape
    } else {
        SourceProjection::Entries(entries)
    }
}

enum ArmEntries {
    Entries(Shape),
    Unresolved,
    Scalar(String),
}

fn empty() -> Shape {
    Shape::Arms { arms: Vec::new(), nullable: false }
}

fn unknown() -> Shape {
    Shape::Unknown { non_null: false }
}

/// What iterating one alternative yields. A named type opens its definition
/// body; `depth` bounds a chain of bodies that are themselves references.
fn arm_entries(atom: &PropertyAtom, definitions: &DefinitionTable, depth: usize) -> ArmEntries {
    if atom.is_array {
        return ArmEntries::Entries(item_shape(atom));
    }
    match &atom.ty {
        TypeExpr::Tuple(tuple) => {
            let mut joined = empty();
            for element in &tuple.elements {
                joined = joined.union(&Shape::of_type(&element.def));
            }
            if let Some(spread) = &tuple.spread {
                joined = joined.union(&item_shape(spread));
            }
            ArmEntries::Entries(joined)
        }
        TypeExpr::InlineObject(shape) => ArmEntries::Entries(object_entry(&closed_values(shape))),
        TypeExpr::Primitive(SimplifiedType::Object) => ArmEntries::Entries(object_entry(&unknown())),
        TypeExpr::Primitive(SimplifiedType::Unknown) | TypeExpr::Imported { .. } => ArmEntries::Unresolved,
        TypeExpr::Ref(key) => {
            let Some(definition) = definitions.get(key).filter(|_| depth > 0) else {
                return ArmEntries::Unresolved;
            };
            let mut joined = empty();
            let mut unresolved = false;
            for alternative in flattened_alternatives(&definition.body) {
                let mut arm = alternative.atom.clone();
                arm.is_array |= alternative.is_array;
                if !arm.is_array && matches!(arm.ty, TypeExpr::Primitive(SimplifiedType::Null)) {
                    continue;
                }
                match arm_entries(&arm, definitions, depth - 1) {
                    ArmEntries::Entries(shape) => joined = joined.union(&shape),
                    ArmEntries::Unresolved => unresolved = true,
                    scalar @ ArmEntries::Scalar(_) => return scalar,
                }
            }
            if unresolved { ArmEntries::Unresolved } else { ArmEntries::Entries(joined) }
        }
        TypeExpr::Union(arms) => {
            let mut joined = empty();
            for arm in arms {
                match arm_entries(arm, definitions, depth) {
                    ArmEntries::Entries(shape) => joined = joined.union(&shape),
                    other => return other,
                }
            }
            ArmEntries::Entries(joined)
        }
        TypeExpr::Function(_) | TypeExpr::Primitive(_) => ArmEntries::Scalar(serialize_property_atom(atom)),
    }
}

/// The values of a closed object's present entries: every declared property
/// and pattern-keyed value, each without optionality.
fn closed_values(shape: &SchemaShape) -> Shape {
    let mut joined = empty();
    for definition in shape.properties.values() {
        joined = joined.union(&Shape::of_type(definition));
    }
    for pattern in &shape.pattern_keys {
        joined = joined.union(&Shape::of_type(&pattern.def));
    }
    joined
}

/// The `{key: string, value}` entry of an object whose values are `value`.
fn object_entry(value: &Shape) -> Shape {
    let mut key = PropertyAtom::bare(SimplifiedType::String);
    key.constraints.push(Constraint::Required);
    let mut entry = SchemaShape::new();
    entry.properties.insert("key".to_string(), PropertyDef::Single(key));
    entry.properties.insert("value".to_string(), member_definition(value));
    Shape::of_type(&PropertyDef::Single(PropertyAtom::bare_inline_object(entry)))
}

/// The declaration a property of type `shape` has: required unless the
/// shape admits `null`, and `unknown` for an unknown or empty shape.
fn member_definition(shape: &Shape) -> PropertyDef {
    match shape {
        Shape::Arms { arms, nullable } if !arms.is_empty() || *nullable => {
            shape.to_definition().unwrap_or_else(|| PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Unknown)))
        }
        _ => PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Unknown)),
    }
}

impl Shape {
    /// The type of an object literal whose members have the given types, in
    /// order: a closed object declaring each member.
    pub fn object_of(members: Vec<(String, Shape)>) -> Shape {
        let mut object = SchemaShape::new();
        for (name, shape) in members {
            object.properties.insert(name, member_definition(&shape));
        }
        Shape::of_type(&PropertyDef::Single(PropertyAtom::bare_inline_object(object)))
    }

    /// The type of an array literal whose items have the joined type `items`.
    /// An unknown or empty item type makes `unknown[]`.
    pub fn array_of(items: &Shape) -> Shape {
        let atom = match items {
            Shape::Arms { arms, nullable } if !arms.is_empty() || *nullable => {
                let single = match arms.as_slice() {
                    [only] if !*nullable && !only.is_array && only.union_arms().is_none() => Some(only.clone()),
                    _ => None,
                };
                match single {
                    Some(mut atom) => {
                        atom.is_array = true;
                        atom
                    }
                    None => {
                        let mut members = arms.clone();
                        if *nullable {
                            members.push(PropertyAtom::bare(SimplifiedType::Null));
                        }
                        let mut atom = PropertyAtom::bare(SimplifiedType::Unknown);
                        atom.ty = TypeExpr::Union(members);
                        atom.is_array = true;
                        atom
                    }
                }
            }
            _ => {
                let mut atom = PropertyAtom::bare(SimplifiedType::Unknown);
                atom.is_array = true;
                atom
            }
        };
        Shape::Arms { arms: vec![atom], nullable: false }
    }
}

/// The fields `pattern` binds against an entry of type `entry`, with each
/// field's type, in authored order. An identifier binds the whole entry.
///
/// ## Errors
///
/// [`PatternMismatch`] when no alternative of `entry` can supply the fields.
/// An alternative that merely might lack a field, or an open or unresolved
/// one, is left to the runtime check.
pub fn project_pattern(
    pattern: &BindingPattern,
    entry: &Shape,
    definitions: &DefinitionTable,
) -> Result<Vec<(String, Shape)>, PatternMismatch> {
    let fields = match pattern {
        BindingPattern::Identifier(name) => return Ok(vec![(name.clone(), entry.clone())]),
        BindingPattern::Fields(fields) => fields,
    };
    let Shape::Arms { arms, .. } = entry else {
        return Ok(fields.iter().map(|field| (field.clone(), unknown())).collect());
    };
    if arms.is_empty() {
        return Ok(fields.iter().map(|field| (field.clone(), unknown())).collect());
    }
    let mut found: Vec<Option<Shape>> = vec![None; fields.len()];
    let mut object_capable = false;
    for arm in arms {
        let Some(members) = arm_fields(arm, fields, definitions, definitions.len()) else {
            continue;
        };
        object_capable = true;
        merge_members(&mut found, members);
    }
    if !object_capable {
        return Err(PatternMismatch::NotAnObject { entry: shape_text(entry) });
    }
    let mut bound = Vec::with_capacity(fields.len());
    for (field, shape) in fields.iter().zip(found) {
        match shape {
            Some(shape) => bound.push((field.clone(), shape)),
            None => return Err(PatternMismatch::MissingField { field: field.clone(), entry: shape_text(entry) }),
        }
    }
    Ok(bound)
}

/// Folds one alternative's field types into the running union: a field is
/// declared when any alternative declares it.
fn merge_members(into: &mut [Option<Shape>], members: Vec<Option<Shape>>) {
    for (slot, member) in into.iter_mut().zip(members) {
        *slot = match (slot.take(), member) {
            (Some(left), Some(right)) => Some(left.union(&right)),
            (only, None) | (None, only) => only,
        };
    }
}

/// The type of each of `fields` in one alternative, or `None` when the
/// alternative can hold no object. A field it provably lacks is `None`.
fn arm_fields(
    atom: &PropertyAtom,
    fields: &[String],
    definitions: &DefinitionTable,
    depth: usize,
) -> Option<Vec<Option<Shape>>> {
    if atom.is_array {
        return None;
    }
    match &atom.ty {
        TypeExpr::InlineObject(shape) => Some(fields.iter().map(|field| inline_field(shape, field)).collect()),
        TypeExpr::Primitive(SimplifiedType::Object | SimplifiedType::Unknown) | TypeExpr::Imported { .. } => {
            Some(fields.iter().map(|_| Some(unknown())).collect())
        }
        TypeExpr::Ref(key) => {
            if depth == 0 {
                return Some(fields.iter().map(|_| Some(unknown())).collect());
            }
            let Some(definition) = definitions.get(key) else {
                return Some(fields.iter().map(|_| Some(unknown())).collect());
            };
            let mut merged: Option<Vec<Option<Shape>>> = None;
            for alternative in flattened_alternatives(&definition.body) {
                let mut arm = alternative.atom.clone();
                arm.is_array |= alternative.is_array;
                if let Some(members) = arm_fields(&arm, fields, definitions, depth - 1) {
                    merge_members(merged.get_or_insert_with(|| vec![None; fields.len()]), members);
                }
            }
            merged
        }
        TypeExpr::Union(arms) => {
            let mut merged: Option<Vec<Option<Shape>>> = None;
            for arm in arms {
                if let Some(members) = arm_fields(arm, fields, definitions, depth) {
                    merge_members(merged.get_or_insert_with(|| vec![None; fields.len()]), members);
                }
            }
            merged
        }
        TypeExpr::Tuple(_) | TypeExpr::Function(_) | TypeExpr::Primitive(_) => None,
    }
}

/// A field of a closed object: a declared property, else the value type of a
/// pattern key that can match its name, else absent. A regular-expression key
/// may match anything, so it keeps the field possible and unknown.
fn inline_field(shape: &SchemaShape, field: &str) -> Option<Shape> {
    if let Some(definition) = shape.properties.get(field) {
        return Some(Shape::of_type(definition));
    }
    let mut found: Option<Shape> = None;
    for pattern in &shape.pattern_keys {
        let matches = match &pattern.key {
            PatternKey::CatchAll => true,
            PatternKey::Starting(prefix) => field.starts_with(prefix.as_str()),
            PatternKey::Ending(suffix) => field.ends_with(suffix.as_str()),
            PatternKey::Pattern(_) => {
                return Some(unknown());
            }
        };
        if matches {
            let value = Shape::of_type(&pattern.def);
            found = Some(found.map_or(value.clone(), |previous| previous.union(&value)));
        }
    }
    found
}

fn shape_text(shape: &Shape) -> String {
    match shape {
        Shape::Unknown { .. } => "unknown".to_string(),
        Shape::Arms { arms, nullable } => {
            let mut parts: Vec<String> = arms.iter().map(serialize_property_atom).collect();
            if *nullable {
                parts.push("null".to_string());
            }
            if parts.is_empty() { "never".to_string() } else { parts.join(" | ") }
        }
    }
}

/// One original item as a callback sees it: its entry type and the identity
/// every name for it shares.
///
/// Build one frame per source and bind it under each callback's pattern with
/// [`Self::enter`]. An identifier pattern names the item's identity and a
/// field pattern names each field as that identity's member path, so `p`,
/// `q`, `p.members`, and a destructured `members` all resolve to subjects of
/// one item, and a fact established while evaluating one callback holds under
/// the others whichever spelling it used. A map result is a new frame built
/// from the result type; nothing carries over by spelling.
#[derive(Debug, Clone)]
pub struct ItemFrame {
    entry: Shape,
    id: LocalId,
}

impl ItemFrame {
    /// A frame for entries of type `entry`, with an identity of its own.
    pub fn new(entry: Shape) -> Self {
        Self { entry, id: LocalId::new(next_frame_id(), 0) }
    }

    /// The type of one entry.
    pub fn entry(&self) -> &Shape {
        &self.entry
    }

    /// The identity an identifier pattern aliases.
    pub fn id(&self) -> LocalId {
        self.id
    }

    /// `scope` with `pattern` bound to this item, in a layer below `parent`.
    /// Facts already in `scope` stay; the locals alias this frame's ids.
    ///
    /// Pass the lexical layer of the scope the directive itself sits in as
    /// `parent`, not that of a sibling callback, so another callback's
    /// parameters never leak into this one.
    ///
    /// ## Errors
    ///
    /// [`PatternMismatch`] when a field pattern cannot bind this item type.
    pub fn enter<'d>(
        &self,
        scope: &TypeScope<'d>,
        parent: Option<&Arc<LexicalTypes>>,
        pattern: &BindingPattern,
    ) -> Result<TypeScope<'d>, PatternMismatch> {
        let projected = project_pattern(pattern, &self.entry, scope.definitions())?;
        let locals = projected
            .into_iter()
            .map(|(name, shape)| match pattern {
                BindingPattern::Identifier(_) => TypedLocal::aliasing(name, self.id, shape),
                BindingPattern::Fields(_) => TypedLocal::aliasing_field(name.clone(), self.id, name, shape),
            })
            .collect();
        let layer = match parent {
            Some(parent) => parent.child(locals),
            None => LexicalTypes::new(locals),
        };
        Ok(scope.within(layer))
    }

    /// The scope of a body that keeps the items `predicate` holds (or fails,
    /// for [`Polarity::False`]) on: the predicate is typed under its own
    /// parameters over `outer`, the shared narrowing engine applies its
    /// `polarity` side, and `body` re-binds the same item under its pattern.
    /// The body sees the item, never the predicate's result.
    ///
    /// ## Errors
    ///
    /// [`PatternMismatch`] from either pattern.
    pub fn retaining<'d>(
        &self,
        outer: &TypeScope<'d>,
        predicate: &Callback,
        polarity: Polarity,
        body: &BindingPattern,
    ) -> Result<TypeScope<'d>, PatternMismatch> {
        let parent = outer.lexical();
        let evaluating = self.enter(outer, parent, &predicate.pattern)?;
        let kept = evaluating.assume(&predicate.body, polarity);
        self.enter(&kept, parent, body)
    }

    /// [`Self::retaining`] for the true side: a filter body.
    ///
    /// ## Errors
    ///
    /// [`PatternMismatch`] from either pattern.
    pub fn selected<'d>(
        &self,
        outer: &TypeScope<'d>,
        predicate: &Callback,
        body: &BindingPattern,
    ) -> Result<TypeScope<'d>, PatternMismatch> {
        self.retaining(outer, predicate, Polarity::True, body)
    }

    /// The type `mapper` returns for this item, typed under its parameters
    /// over `outer` without evaluating it. An expression whose type is not
    /// known is `unknown`.
    ///
    /// ## Errors
    ///
    /// [`PatternMismatch`] when the mapper's pattern cannot bind this item.
    pub fn mapped_shape(&self, outer: &TypeScope<'_>, mapper: &SpannedCallback) -> Result<Shape, PatternMismatch> {
        let pattern = mapper.pattern.erase();
        let evaluating = self.enter(outer, outer.lexical(), &pattern)?;
        Ok(expression_type(&mapper.body, &evaluating))
    }

    /// The scope of a map body: `body` bound to a new frame holding the
    /// mapper's result, a subject distinct from the input even when the body
    /// reuses the input's spelling. Facts about the input do not appear.
    ///
    /// ## Errors
    ///
    /// [`PatternMismatch`] from the mapper's pattern, or when the result
    /// cannot satisfy `body` (`{key, value} => value` over a scalar).
    pub fn mapped<'d>(
        &self,
        outer: &TypeScope<'d>,
        mapper: &SpannedCallback,
        body: &BindingPattern,
    ) -> Result<(ItemFrame, TypeScope<'d>), PatternMismatch> {
        let result = ItemFrame::new(self.mapped_shape(outer, mapper)?);
        let scope = result.enter(outer, outer.lexical(), body)?;
        Ok((result, scope))
    }
}

impl<'d> TypeScope<'d> {
    /// What iterating `source` yields at this site, with `gate` (the
    /// directive's own condition) assumed true. The source is read as the
    /// stable subject it names; one that names none is unknown.
    ///
    /// The enclosing guards already in this scope apply as well, so a type
    /// predicate in the gate can establish iterability.
    pub fn project_source(&self, gate: Option<&Expr>, source: &Expr) -> SourceProjection {
        let gated;
        let scope = match gate {
            Some(gate) => {
                gated = self.assume(gate, Polarity::True);
                &gated
            }
            None => self,
        };
        match scope.type_of_expr(source) {
            Some(ty) => project_source(ty.shape(), scope.definitions()),
            None => SourceProjection::UnknownShape,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::{DeclaredTypes, parse, parse_callback, parse_callback_spanned, parse_spanned};
    use crate::markdown::schemas::{SimplifiedSchema, parse_yaml_schema};

    fn schema(yaml: &str) -> SimplifiedSchema {
        parse_yaml_schema(&serde_yaml_ng::from_str(yaml).unwrap()).unwrap()
    }

    fn project(yaml: &str, source: &str) -> SourceProjection {
        let schema = schema(yaml);
        let declared = DeclaredTypes::new(Some(&schema));
        TypeScope::unguarded(&declared).project_source(None, &parse(source).unwrap())
    }

    fn entries(yaml: &str, source: &str) -> Shape {
        match project(yaml, source) {
            SourceProjection::Entries(shape) => shape,
            other => panic!("{source} should be iterable, got {other:?}"),
        }
    }

    /// The type of the object entry's `value` field, as text.
    fn entry_value(entry: &Shape) -> String {
        let fields = project_pattern(
            &BindingPattern::Fields(vec!["key".into(), "value".into()]),
            entry,
            &DefinitionTable::default(),
        )
        .unwrap();
        assert_eq!(shape_text(&fields[0].1), "string", "the key is a string");
        shape_text(&fields[1].1)
    }

    fn fields(names: &[&str]) -> BindingPattern {
        BindingPattern::Fields(names.iter().map(|name| (*name).to_string()).collect())
    }

    #[test]
    fn an_array_yields_its_element_declaration() {
        assert_eq!(shape_text(&entries("xs: string[](required)", "xs")), "string");
    }

    #[test]
    fn a_tuple_joins_its_declared_members() {
        let joined = entries("t: tuple([string, number?, ...boolean[]])", "t");
        assert_eq!(shape_text(&joined), "boolean | number | string");
    }

    #[test]
    fn an_object_yields_key_value_entries_joining_heterogeneous_values() {
        let entry = entries("o:\n  name: string(required)\n  count: number(required)\n", "o");
        assert_eq!(entry_value(&entry), "number | string");
    }

    #[test]
    fn an_open_object_includes_unknown_values() {
        let entry = entries("o: object(required)", "o");
        assert_eq!(entry_value(&entry), "unknown");
    }

    #[test]
    fn an_absent_optional_property_never_becomes_an_entry() {
        let entry = entries("o:\n  a: string(required)\n  b: string\n", "o");
        let fields = project_pattern(&fields(&["value"]), &entry, &DefinitionTable::default()).unwrap();
        assert!(!fields[0].1.admits_null(), "an optional property contributes its type, not an extra null");
        assert_eq!(shape_text(&fields[0].1), "string");
    }

    #[test]
    fn a_declared_null_value_is_preserved() {
        let entry = entries("o:\n  a: string(required)\n  b: null(required)\n", "o");
        assert_eq!(entry_value(&entry), "string | null");
        let element = entries("xs: (string | null)[](required)", "xs");
        assert!(element.admits_null(), "null elements are preserved");
    }

    #[test]
    fn a_union_of_iterables_joins_array_and_object_entries() {
        let joined = entries("u:\n  - string[](required)\n  - object(required)\n", "u");
        let Shape::Arms { arms, .. } = &joined else { panic!("known entries") };
        assert_eq!(arms.len(), 2, "an element and a key/value entry");
    }

    #[test]
    fn optional_and_nullable_sources_are_accepted() {
        assert_eq!(shape_text(&entries("xs: string[]", "xs")), "string");
        assert_eq!(shape_text(&entries("xs:\n  - string[](required)\n  - 'null'\n", "xs")), "string");
    }

    #[test]
    fn a_scalar_or_a_union_with_a_scalar_arm_is_unsuitable() {
        assert_eq!(project("s: string(required)", "s"), SourceProjection::Unsuitable(vec!["string".to_string()]));
        match project("u:\n  - string(required)\n  - string[](required)\n", "u") {
            SourceProjection::Unsuitable(arms) => assert_eq!(arms, ["string"]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn only_an_unknown_shape_is_recoverable() {
        assert_eq!(project("x: string", "nope"), SourceProjection::UnknownShape, "undeclared");
        assert_eq!(project("x: unknown", "x"), SourceProjection::UnknownShape, "`any` alone is not iterable");
        assert!(matches!(project("x: unknown[](required)", "x"), SourceProjection::Entries(Shape::Unknown { .. })));
        assert_eq!(SourceProjection::recovered_entries(), Shape::Unknown { non_null: false });
    }

    #[test]
    fn the_source_is_checked_on_the_true_side_of_its_own_gate() {
        let schema = schema("u:\n  - string(required)\n  - string[](required)\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let scope = TypeScope::unguarded(&declared);
        let source = parse("u").unwrap();
        assert!(matches!(scope.project_source(None, &source), SourceProjection::Unsuitable(_)));
        let gate = parse("is_array(u)").unwrap();
        assert!(matches!(scope.project_source(Some(&gate), &source), SourceProjection::Entries(_)));
        let negated = parse("!is_array(u)").unwrap();
        assert!(matches!(scope.project_source(Some(&negated), &source), SourceProjection::Unsuitable(_)));
        let enclosing = scope.assume(&gate, Polarity::True);
        assert!(matches!(enclosing.project_source(None, &source), SourceProjection::Entries(_)), "an enclosing guard applies too");
    }

    #[test]
    fn a_named_definition_resolves_through_its_owner() {
        use crate::markdown::Markdown;
        use crate::markdown::compose::ComposeSource;
        use crate::markdown::schemas::DarkmatterSchemas;

        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("types.yaml"),
            "kind: schema\ntypes:\n    node:\n        label: string(required)\n        children: node[](required)\n",
        )
        .unwrap();
        let path = dir.path().join("doc.md");
        std::fs::write(&path, "---\n$schema:\n  tree: node(required)@./types.yaml\n  trees: node[](required)@./types.yaml\n---\nBody\n").unwrap();
        let markdown = Markdown::try_from(path.as_path()).unwrap().with_source(ComposeSource::File(path.clone()));
        let context = biscuit_file::FileResolutionContext::new(dir.path()).with_repository_root(dir.path());
        let effective = DarkmatterSchemas::new(context).effective_for(&markdown).unwrap().unwrap();
        let declared = DeclaredTypes::for_schema(Some(&effective));
        let scope = TypeScope::unguarded(&declared);

        let SourceProjection::Entries(object_entry) = scope.project_source(None, &parse("tree").unwrap()) else {
            panic!("a named object is iterable");
        };
        let projected = project_pattern(&fields(&["key", "value"]), &object_entry, scope.definitions()).unwrap();
        assert!(shape_text(&projected[1].1).contains("string"), "{}", shape_text(&projected[1].1));
        let SourceProjection::Entries(element) = scope.project_source(None, &parse("trees").unwrap()) else {
            panic!("a named array is iterable");
        };
        let label = project_pattern(&fields(&["label"]), &element, scope.definitions()).unwrap();
        assert_eq!(shape_text(&label[0].1), "string", "the pattern opens the named element through the owner's definitions");
        assert!(project_pattern(&fields(&["nope"]), &element, scope.definitions()).is_err());
    }

    #[test]
    fn a_pattern_binds_an_identifier_or_shallow_fields() {
        let definitions = DefinitionTable::default();
        let element = entries("people: '{ name: string(required), age: number }[](required)'", "people");
        let whole = project_pattern(&BindingPattern::Identifier("p".into()), &element, &definitions).unwrap();
        assert_eq!(whole, [("p".to_string(), element.clone())]);
        let some = project_pattern(&fields(&["name", "age"]), &element, &definitions).unwrap();
        assert_eq!(shape_text(&some[0].1), "string");
        assert_eq!(shape_text(&some[1].1), "number");
        let extra_ignored = project_pattern(&fields(&["name"]), &element, &definitions).unwrap();
        assert_eq!(extra_ignored.len(), 1);
    }

    #[test]
    fn a_pattern_that_cannot_match_is_a_mismatch() {
        let definitions = DefinitionTable::default();
        let element = entries("people: '{ name: string(required) }[](required)'", "people");
        let missing = project_pattern(&fields(&["name", "nope"]), &element, &definitions).unwrap_err();
        assert_eq!(missing, PatternMismatch::MissingField { field: "nope".into(), entry: shape_text(&element) });
        assert!(missing.to_string().contains("nope"));
        let strings = entries("xs: string[](required)", "xs");
        assert!(matches!(
            project_pattern(&fields(&["a"]), &strings, &definitions),
            Err(PatternMismatch::NotAnObject { .. })
        ));
        let unknown = entries("xs: unknown[](required)", "xs");
        assert!(project_pattern(&fields(&["a"]), &unknown, &definitions).is_ok(), "uncertain shapes are checked at runtime");
        let open = entries("xs: object[](required)", "xs");
        assert!(project_pattern(&fields(&["a"]), &open, &definitions).is_ok());
    }

    fn expression(text: &str, scope: &TypeScope<'_>) -> String {
        shape_text(&expression_type(&parse_spanned(text).unwrap(), scope))
    }

    #[test]
    fn computed_object_and_array_shapes_follow_their_members() {
        let schema = schema("name: string(required)\nn: number\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let scope = TypeScope::unguarded(&declared);
        let object = expression_type(&parse_spanned("{ a: name, b: n, c: upper(name), d: name || 'x' }").unwrap(), &scope);
        let projected = project_pattern(&fields(&["a", "b", "c", "d"]), &object, &DefinitionTable::default()).unwrap();
        let texts: Vec<_> = projected.iter().map(|(_, shape)| shape_text(shape)).collect();
        assert_eq!(texts, ["string", "number | null", "string | null", "literal(x) | string"]);
        assert_eq!(expression("[name, 1 + 1]", &scope), "unknown[]", "an unknown member makes unknown items");
        assert_eq!(expression("[name, name]", &scope), "string[]");
        let mixed = expression("[name, n]", &scope);
        assert!(mixed.ends_with("[]") && mixed.contains("string") && mixed.contains("number") && mixed.contains("null"), "{mixed}");
        assert_eq!(expression("1 + 1", &scope), "unknown", "other expressions stay unknown");
    }

    #[test]
    fn computed_shapes_include_shorthand_members() {
        let schema = schema("x: string(required)\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let scope = TypeScope::unguarded(&declared);
        let object = expression_type(&parse_spanned("{x, y: x}").unwrap(), &scope);
        let projected = project_pattern(&fields(&["x", "y"]), &object, &DefinitionTable::default()).unwrap();
        assert_eq!(shape_text(&projected[0].1), "string");
        assert_eq!(shape_text(&projected[1].1), "string");
        assert!(project_pattern(&fields(&["z"]), &object, &DefinitionTable::default()).is_err());
    }

    fn names_present(scope: &TypeScope<'_>, name: &str) -> bool {
        scope.type_of_expr(&parse(name).unwrap()).unwrap().guard_presence().is_some()
    }

    fn callback(text: &str) -> Callback {
        parse_callback(text).unwrap()
    }

    #[test]
    fn differently_named_callback_parameters_share_the_input_subject() {
        let schema = schema("x: string\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let outer = TypeScope::unguarded(&declared);
        let predicate = callback("item -> item");
        let body = BindingPattern::Identifier("person".into());
        let frame = ItemFrame::new(Shape::Arms { arms: vec![PropertyAtom::bare(SimplifiedType::String)], nullable: true });

        let inside = frame.enter(&outer, None, &predicate.pattern).unwrap();
        assert!(!names_present(&inside, "item"));
        let selected = frame.selected(&outer, &predicate, &body).unwrap();
        assert!(names_present(&selected, "person"), "the predicate's fact about `item` holds for `person`");
        assert!(!selected.type_of_expr(&parse("person").unwrap()).unwrap().admits_null());
        assert!(!selected.resolves_to_local("item"), "the predicate's parameter does not leak into the body");
        let a = frame.enter(&outer, None, &BindingPattern::Identifier("a".into())).unwrap();
        let b = frame.enter(&outer, None, &BindingPattern::Identifier("b".into())).unwrap();
        assert_eq!(a.subject(&parse("a").unwrap()), b.subject(&parse("b").unwrap()));
        assert!(!names_present(&a.assume(&parse("a").unwrap(), Polarity::False), "a"));
    }

    #[test]
    fn field_patterns_share_field_subjects_across_callbacks() {
        let schema = schema("x: string\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let outer = TypeScope::unguarded(&declared);
        let entry = Shape::of_type(&PropertyDef::Single(PropertyAtom::bare_inline_object({
            let mut object = SchemaShape::new();
            object.properties.insert("name".into(), PropertyDef::Union(vec![
                PropertyAtom::bare(SimplifiedType::String),
                PropertyAtom::bare(SimplifiedType::Null),
            ]));
            object
        })));
        let predicate = callback("{ name } -> name");
        let body = fields(&["name"]);
        let frame = ItemFrame::new(entry);
        let selected = frame.selected(&outer, &predicate, &body).unwrap();
        assert!(names_present(&selected, "name"));
        let mismatch = frame.selected(&outer, &callback("{ nope } -> nope"), &body);
        assert!(matches!(mismatch, Err(PatternMismatch::MissingField { .. })));
    }

    /// `{ members: unknown }` entries: a field whose array type only a guard
    /// can establish.
    fn members_entry() -> Shape {
        entries("data: '{ members: unknown(required), other: unknown(required) }[](required)'", "data")
    }

    fn is_array_of(scope: &TypeScope<'_>, read: &str) -> bool {
        let ty = scope.type_of_expr(&parse(read).unwrap()).unwrap();
        matches!(project_source(ty.shape(), scope.definitions()), SourceProjection::Entries(_))
    }

    #[test]
    fn a_destructured_field_and_an_object_member_are_one_subject_in_both_directions() {
        let schema = schema("x: string\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let outer = TypeScope::unguarded(&declared);
        let frame = ItemFrame::new(members_entry());
        let object = BindingPattern::Identifier("p".into());
        let destructured = fields(&["members"]);

        let via_member = frame.selected(&outer, &callback("p => is_array(p.members)"), &destructured).unwrap();
        assert!(is_array_of(&via_member, "members"), "a fact about `p.members` holds for a destructured `members`");
        let via_field = frame.selected(&outer, &callback("{ members } => is_array(members)"), &object).unwrap();
        assert!(is_array_of(&via_field, "p.members"), "a fact about a destructured `members` holds for `p.members`");
        assert!(!is_array_of(&via_field, "p.other"), "a sibling field gains nothing");

        let p = frame.enter(&outer, None, &object).unwrap();
        let members = frame.enter(&outer, None, &destructured).unwrap();
        assert_eq!(p.subject(&parse("p.members").unwrap()), members.subject(&parse("members").unwrap()));
        assert_eq!(p.subject(&parse("p['members']").unwrap()), members.subject(&parse("members").unwrap()));
    }

    #[test]
    fn a_map_result_field_gains_input_facts_only_through_its_mapping_expression() {
        let schema = schema("x: string\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let outer = TypeScope::unguarded(&declared);
        let frame = ItemFrame::new(members_entry());
        let guarded = frame
            .enter(&outer, None, &BindingPattern::Identifier("p".into()))
            .unwrap()
            .assume(&parse("is_array(p.members)").unwrap(), Polarity::True)
            .with_lexical(None);

        let carried = parse_callback_spanned("{ members } => { members }").unwrap();
        let (_, body) = frame.mapped(&guarded, &carried, &fields(&["members"])).unwrap();
        assert!(is_array_of(&body, "members"), "the mapping expression carries the proven type");

        let swapped = parse_callback_spanned("{ members, other } => { members: other, other: members }").unwrap();
        let (result, body) = frame.mapped(&guarded, &swapped, &fields(&["members", "other"])).unwrap();
        assert!(!is_array_of(&body, "members"), "a result field named like the input field is a new subject");
        assert!(is_array_of(&body, "other"));
        assert_ne!(result.id(), frame.id());
    }

    #[test]
    fn a_shadowing_destructured_field_of_another_item_does_not_inherit_outer_facts() {
        let schema = schema("x: string\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let outer_scope = TypeScope::unguarded(&declared);
        let outer = ItemFrame::new(members_entry());
        let selected = outer.selected(&outer_scope, &callback("p => is_array(p.members)"), &fields(&["members"])).unwrap();
        assert!(is_array_of(&selected, "members"));

        let inner = ItemFrame::new(members_entry());
        let shadowed = inner.enter(&selected, selected.lexical(), &fields(&["members"])).unwrap();
        assert!(!is_array_of(&shadowed, "members"), "the inner item's `members` is a different subject");
    }

    #[test]
    fn a_false_polarity_retains_the_complement_facts() {
        let schema = schema("x: string\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let outer = TypeScope::unguarded(&declared);
        let predicate = callback("p -> p == null");
        let body = BindingPattern::Identifier("p".into());
        let frame = ItemFrame::new(Shape::Arms { arms: vec![PropertyAtom::bare(SimplifiedType::String)], nullable: true });
        let kept = frame.retaining(&outer, &predicate, Polarity::False, &body).unwrap();
        assert!(!kept.type_of_expr(&parse("p").unwrap()).unwrap().admits_null());
        let selected = frame.selected(&outer, &predicate, &body).unwrap();
        assert!(selected.type_of_expr(&parse("p").unwrap()).unwrap().admits_null(), "the true side keeps only null");
    }

    #[test]
    fn a_map_result_is_a_new_subject_even_for_the_identity_map() {
        let schema = schema("x: string\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let outer = TypeScope::unguarded(&declared);
        let input = BindingPattern::Identifier("p".into());
        let frame = ItemFrame::new(Shape::Arms { arms: vec![PropertyAtom::bare(SimplifiedType::String)], nullable: true });
        let narrowed_input = frame.enter(&outer, None, &input).unwrap().assume(&parse("p").unwrap(), Polarity::True);
        assert!(names_present(&narrowed_input, "p"));

        let mapper = parse_callback_spanned("p => p").unwrap();
        let (result, body) = frame.mapped(&narrowed_input, &mapper, &input).unwrap();
        assert_ne!(result.id(), frame.id());
        assert!(!names_present(&body, "p"), "input facts do not migrate by spelling");
        assert_ne!(body.subject(&parse("p").unwrap()), narrowed_input.subject(&parse("p").unwrap()));
    }

    #[test]
    fn a_map_result_is_typed_by_its_body_and_checked_against_the_body_pattern() {
        let schema = schema("x: string\n");
        let declared = DeclaredTypes::new(Some(&schema));
        let outer = TypeScope::unguarded(&declared);
        let entry = entries("o:\n  a: string(required)\n", "o");
        let pattern = fields(&["key", "value"]);
        let frame = ItemFrame::new(entry);

        let reshaped = parse_callback_spanned("{ key, value } => { key, value: upper(value) }").unwrap();
        let (result, scope) = frame.mapped(&outer, &reshaped, &pattern).unwrap();
        let value = shape_text(scope.type_of_expr(&parse("value").unwrap()).unwrap().shape());
        assert!(value.contains("string"), "{value}");
        assert!(matches!(result.entry(), Shape::Arms { .. }));

        let scalar = parse_callback_spanned("{ key, value } => value").unwrap();
        let rejected = frame.mapped(&outer, &scalar, &pattern);
        assert!(matches!(rejected, Err(PatternMismatch::NotAnObject { .. })), "a returned scalar cannot satisfy the object pattern");

        let opaque = parse_callback_spanned("{ key, value } => value + 1").unwrap();
        assert_eq!(frame.mapped_shape(&outer, &opaque).unwrap(), Shape::Unknown { non_null: false });
    }
}
