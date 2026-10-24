//! The grammar translator: SimplifiedSchema type descriptors → coercion
//! [`Shape`]s.
//!
//! It maps syntax only; the coercion core alone decides keep, convert, or
//! error. Every construct means what `schemas::simplified::convert` lowers it
//! to in JSON Schema, so binding a value through this translator and through
//! the lowered JSON Schema gives the same outcome (the conformance test the
//! spec's "Spike Rulings" require). Where the lowering differs between a
//! property and a bare value, both are offered:
//!
//! - **bare** ([`shape_from_property_def`]): a function parameter or return,
//!   a tuple position, a dictionary value. No optional-`null` wrapper.
//! - **property** ([`property_shape_from_def`]): a property of an object.
//!   An optional property also admits `null`, and an optional scalar `file`
//!   also admits `""`, exactly as the lowering's wrappers do.
//!
//! A recursive named type becomes a reusable [`ShapeRef`] to its bare
//! shape, translated once, so declared conversions apply at every depth the
//! engine walks: a [`TypeExpr::Ref`] through the [`DefinitionTable`] the
//! `*_with_definitions` entry points take, and a named type the
//! [`TypeResolver`] supplies when it refers back to itself below a value
//! descent (a property, an item, a tuple slot, a dictionary value). A
//! reference loop that never descends still fails with
//! [`TranslateError::TooDeep`].
//!
//! Refinement formats (`date`, `datetime`, `time`, `url`, `email`, `file`,
//! `expression`, `shell`, `type-definition`, `schema`) become passive
//! [`Assertion`]s judged by the same format validators JSON Schema
//! validation registers. `file` is judged by syntax only: a passive check
//! never looks for the file, even for `file(eager)`.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use serde_json::{Value, json};

use crate::markdown::coercion::{
    Additional, ArrayShape, Assertion, ContentFormat, IntegerRule, KeyMatcher, NumberShape, NumericBounds,
    ObjectShape, PatternScope, PatternSlot, PropertySlot, RefSlot, Shape, ShapeRef, TextPattern, TextShape,
    TupleShape, TupleSlot, accepts_exactly,
};
use crate::markdown::numeric::json_number;
use crate::markdown::schemas::format::{
    DARKMATTER_DATETIME_FORMAT, DARKMATTER_EXPRESSION_FORMAT, DARKMATTER_FILE_REFERENCE_FORMAT,
    DARKMATTER_SCHEMA_KEYWORD, DARKMATTER_SHELL_FORMAT, DARKMATTER_TIME_FORMAT, DARKMATTER_TYPE_DEFINITION_KEYWORD,
    DARKMATTER_URL_SCHEME_KEYWORD,
};
use crate::markdown::schemas::simplified::{
    AlphanumericOptions, Constraint, DefKey, DefinitionOuter, DefinitionTable, PatternKey, PropertyAtom, PropertyDef,
    SchemaArm, SchemaShape, SimplifiedSchema, SimplifiedType, TupleType, TypeExpr,
};

/// How many named-type references deep translation follows before failing.
/// A recursive named type counts once per path, since its uses below a value
/// descent are reusable references; one that never descends (an alias loop,
/// a union arm naming its own type) reaches this limit.
pub const MAX_REFERENCE_DEPTH: usize = 32;

/// The table the entry points without `_with_definitions` translate against:
/// every [`TypeExpr::Ref`] is then a missing definition.
static NO_DEFINITIONS: LazyLock<DefinitionTable> = LazyLock::new(DefinitionTable::default);

/// Supplies the definition of a named type (`Name` / `Name@reference`).
///
/// Named types are resolved by the catalog that owns them; the translator
/// only asks. A closure `Fn(&str, &str) -> Option<PropertyDef>` is a
/// resolver.
pub trait TypeResolver {
    /// The definition of `name` from `reference` (`"this"` for the writing
    /// schema), or `None` when there is no such type.
    fn resolve(&self, name: &str, reference: &str) -> Option<PropertyDef>;
}

impl<F> TypeResolver for F
where
    F: Fn(&str, &str) -> Option<PropertyDef>,
{
    fn resolve(&self, name: &str, reference: &str) -> Option<PropertyDef> {
        self(name, reference)
    }
}

/// A resolver that knows no named types: every reference is a
/// [`TranslateError::UnresolvedReference`].
#[derive(Debug, Clone, Copy, Default)]
pub struct NoNamedTypes;

impl TypeResolver for NoNamedTypes {
    fn resolve(&self, _name: &str, _reference: &str) -> Option<PropertyDef> {
        None
    }
}

/// Why a grammar type could not become a [`Shape`]. `path` names the place
/// in the type (`a.b`, `p[1]`); empty for the type itself.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TranslateError {
    #[error("{}: named type `{name}@{reference}` is not defined", at(path))]
    UnresolvedReference { path: String, name: String, reference: String },
    /// A `function` type where a value is expected: function values do not
    /// exist at runtime.
    #[error("{}: a `function` type describes a callable and cannot type a value", at(path))]
    FunctionValue { path: String },
    #[error("{}: constraint `{constraint}` is not valid on `{ty}`", at(path))]
    InvalidConstraint { path: String, ty: String, constraint: String },
    /// Contradictory constraints, an invalid pattern, or another load-time
    /// defect the JSON Schema lowering also rejects.
    #[error("{}: {message}", at(path))]
    Invalid { path: String, message: String },
    /// A chain of named types deeper than [`MAX_REFERENCE_DEPTH`], or a
    /// named type that refers back to itself without descending into a child
    /// value.
    #[error("{}: named types nest deeper than {limit} references", at(path))]
    TooDeep { path: String, limit: usize },
}

impl TranslateError {
    pub fn path(&self) -> &str {
        match self {
            Self::UnresolvedReference { path, .. }
            | Self::FunctionValue { path }
            | Self::InvalidConstraint { path, .. }
            | Self::Invalid { path, .. }
            | Self::TooDeep { path, .. } => path,
        }
    }
}

fn at(path: &str) -> &str {
    if path.is_empty() { "<type>" } else { path }
}

/// The shape of a bare value of `def`: a parameter, a return, a tuple
/// position, or a dictionary value. No optional-`null` wrapper is added.
///
/// ## Errors
///
/// A [`TranslateError`] for an unresolved reference, a `function` type, or a
/// constraint the JSON Schema lowering also rejects.
pub fn shape_from_property_def(def: &PropertyDef, resolver: &dyn TypeResolver) -> Result<Shape, TranslateError> {
    shape_from_property_def_with_definitions(def, resolver, &NO_DEFINITIONS)
}

/// [`shape_from_property_def`] for a definition whose [`TypeExpr::Ref`]s
/// point into `definitions`.
///
/// ## Errors
///
/// As [`shape_from_property_def`], plus [`TranslateError::Invalid`] for a
/// reference whose definition `definitions` does not hold.
pub fn shape_from_property_def_with_definitions(
    def: &PropertyDef,
    resolver: &dyn TypeResolver,
    definitions: &DefinitionTable,
) -> Result<Shape, TranslateError> {
    Translator::new(resolver, definitions).bare_def("", def)
}

/// A property's shape and whether the object requires it, with the
/// lowering's presence rules: an optional property also admits `null` (and
/// an optional scalar `file` also `""`); `generated` keeps the value
/// non-nullable but not required.
#[derive(Debug, Clone)]
pub struct PropertyShape {
    pub shape: Shape,
    pub required: bool,
}

/// The shape of `def` as a property of an object.
///
/// ## Errors
///
/// As [`shape_from_property_def`].
pub fn property_shape_from_def(def: &PropertyDef, resolver: &dyn TypeResolver) -> Result<PropertyShape, TranslateError> {
    property_shape_from_def_with_definitions(def, resolver, &NO_DEFINITIONS)
}

/// [`property_shape_from_def`] for a definition whose [`TypeExpr::Ref`]s
/// point into `definitions`.
///
/// ## Errors
///
/// As [`shape_from_property_def_with_definitions`].
pub fn property_shape_from_def_with_definitions(
    def: &PropertyDef,
    resolver: &dyn TypeResolver,
    definitions: &DefinitionTable,
) -> Result<PropertyShape, TranslateError> {
    let (shape, required) = Translator::new(resolver, definitions).property("", def)?;
    Ok(PropertyShape { shape, required })
}

/// The shape of a whole document schema: an open object (closed once it
/// declares pattern keys), or a union of such objects.
///
/// Root-union `file(match(...))` arm selection reads the filesystem, so it is
/// not part of this passive shape.
///
/// ## Errors
///
/// As [`shape_from_property_def`], plus an unresolved root-union file arm.
pub fn shape_from_simplified_schema(
    schema: &SimplifiedSchema,
    resolver: &dyn TypeResolver,
) -> Result<Shape, TranslateError> {
    shape_from_simplified_schema_with_definitions(schema, resolver, &NO_DEFINITIONS)
}

/// [`shape_from_simplified_schema`] for a resolved schema whose
/// [`TypeExpr::Ref`]s point into `definitions` (`ResolvedSchema::definitions`,
/// `EffectiveSchema::definitions`).
///
/// ## Errors
///
/// As [`shape_from_simplified_schema`], plus [`TranslateError::Invalid`] for
/// a reference whose definition `definitions` does not hold.
pub fn shape_from_simplified_schema_with_definitions(
    schema: &SimplifiedSchema,
    resolver: &dyn TypeResolver,
    definitions: &DefinitionTable,
) -> Result<Shape, TranslateError> {
    let mut translator = Translator::new(resolver, definitions);
    match schema {
        SimplifiedSchema::Single(shape) => translator.root_object(shape),
        SimplifiedSchema::Union(arms) => {
            let mut options = Vec::with_capacity(arms.len());
            for (index, arm) in arms.iter().enumerate() {
                match arm {
                    SchemaArm::Inline(shape) => options.push(translator.root_object(shape)?),
                    SchemaArm::FileRef(reference) => {
                        return Err(TranslateError::Invalid {
                            path: format!("<arm[{index}]>"),
                            message: format!("root-union file reference `{reference}` must be resolved first"),
                        });
                    }
                }
            }
            Ok(Shape::Union(options))
        }
    }
}

/// The constraints that govern presence, timing, deferral, or
/// documentation, never a value's type.
fn is_annotation(constraint: &Constraint) -> bool {
    matches!(
        constraint,
        Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Deferred
            | Constraint::Example(_)
            | Constraint::Category(_)
            | Constraint::Operands(_)
            | Constraint::Eager
    )
}

/// A named type, as the reusable references of one translation key it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NamedKey {
    /// `name@reference`, supplied by the [`TypeResolver`].
    Imported(String, String),
    /// A cyclic definition in the [`DefinitionTable`].
    Definition(DefKey),
}

/// A named type being translated on the current path.
struct Frame {
    key: NamedKey,
    /// [`Translator::descent`] when the type was entered.
    descent: usize,
    /// Set while the type's reusable bare shape is being built; `None` for a
    /// use inlined in place.
    slot: Option<RefSlot>,
}

/// A bare reference to a named type with no value constraints.
enum Inlined {
    /// The definition, with the use's markers, to translate in place.
    Definition(NamedKey, PropertyDef),
    /// A use below a value descent inside the type's own translation: a
    /// reusable reference, and the definition for the use's presence rules.
    Recursive(Shape, PropertyDef),
}

struct Translator<'r> {
    resolver: &'r dyn TypeResolver,
    definitions: &'r DefinitionTable,
    depth: usize,
    /// Value descents on the current path; see [`Frame::descent`].
    descent: usize,
    frames: Vec<Frame>,
    /// Finished reusable shapes, shared by every later use.
    finished: HashMap<NamedKey, ShapeRef>,
}

impl<'r> Translator<'r> {
    fn new(resolver: &'r dyn TypeResolver, definitions: &'r DefinitionTable) -> Self {
        Self {
            resolver,
            definitions,
            depth: 0,
            descent: 0,
            frames: Vec::new(),
            finished: HashMap::new(),
        }
    }

    /// Runs `translate` on a child value: a property, an item, a tuple slot,
    /// or a dictionary value.
    fn descend<T>(&mut self, translate: impl FnOnce(&mut Self) -> T) -> T {
        self.descent += 1;
        let result = translate(self);
        self.descent -= 1;
        result
    }

    /// Runs `translate` on the definition of `key`, inlined in place.
    fn within<T>(&mut self, key: NamedKey, translate: impl FnOnce(&mut Self) -> T) -> T {
        self.frames.push(Frame {
            key,
            descent: self.descent,
            slot: None,
        });
        self.depth += 1;
        let result = translate(self);
        self.depth -= 1;
        self.frames.pop();
        result
    }

    /// Whether a use of `key` here closes a cycle through a value descent.
    fn recurses(&self, key: &NamedKey) -> bool {
        self.frames
            .iter()
            .rev()
            .find(|frame| frame.key == *key)
            .is_some_and(|frame| self.descent > frame.descent)
    }

    /// The reusable reference to `key`'s bare shape: a back reference inside
    /// its own body, the finished shape after it, else built now from
    /// `definition`.
    fn reusable(&mut self, path: &str, key: NamedKey, definition: &PropertyDef) -> Result<Shape, TranslateError> {
        if let Some(slot) = self.frames.iter().rev().find_map(|frame| frame.slot.as_ref().filter(|_| frame.key == key)) {
            return Ok(Shape::Ref(slot.back_reference()));
        }
        if let Some(finished) = self.finished.get(&key) {
            return Ok(Shape::Ref(finished.clone()));
        }
        if self.depth >= MAX_REFERENCE_DEPTH {
            return Err(TranslateError::TooDeep {
                path: path.to_string(),
                limit: MAX_REFERENCE_DEPTH,
            });
        }
        let name = match &key {
            NamedKey::Imported(name, _) => name.clone(),
            NamedKey::Definition(key) => key.name.clone(),
        };
        self.frames.push(Frame {
            key: key.clone(),
            descent: self.descent,
            slot: Some(RefSlot::new(name)),
        });
        self.depth += 1;
        let body = self.bare_def(path, definition);
        self.depth -= 1;
        let slot = self.frames.pop().and_then(|frame| frame.slot).expect("the frame pushed above");
        let finished = slot.finish(body?);
        self.finished.insert(key, finished.clone());
        Ok(Shape::Ref(finished))
    }

    /// A use of the cyclic definition `key`: its reusable shape, with the
    /// use's value constraints checked beside it, as the lowering writes
    /// them as `$ref` siblings.
    fn definition_reference(&mut self, path: &str, key: &DefKey, constraints: &[Constraint]) -> Result<Shape, TranslateError> {
        let Some(definition) = self.definitions.get(key) else {
            return Err(invalid(path, format!("the recursive named type `{}` has no definition", key.name)));
        };
        let body = definition.body.clone();
        let shape = self.reusable(path, NamedKey::Definition(key.clone()), &body)?;
        let value_constraints: Vec<&Constraint> =
            constraints.iter().filter(|constraint| !is_annotation(constraint)).collect();
        if value_constraints.is_empty() {
            return Ok(shape);
        }
        let mut fragment = serde_json::Map::new();
        let allowed = match self.definitions.outer(key) {
            Some(DefinitionOuter::Array) => {
                let owned: Vec<Constraint> = value_constraints.iter().map(|constraint| (*constraint).clone()).collect();
                if let Shape::Array(bounds) = array(path, Shape::Any, &owned)? {
                    if let Some(min) = bounds.min_items {
                        fragment.insert("minItems".into(), json!(min));
                    }
                    if let Some(max) = bounds.max_items {
                        fragment.insert("maxItems".into(), json!(max));
                    }
                    if bounds.unique_items {
                        fragment.insert("uniqueItems".into(), json!(true));
                    }
                }
                true
            }
            Some(DefinitionOuter::Object) => {
                let mut bounds = ObjectShape::opaque();
                object_arity(path, &mut bounds, value_constraints.iter().copied())?;
                if let Some(min) = bounds.min_properties {
                    fragment.insert("minProperties".into(), json!(min));
                }
                if let Some(max) = bounds.max_properties {
                    fragment.insert("maxProperties".into(), json!(max));
                }
                value_constraints
                    .iter()
                    .all(|constraint| matches!(constraint, Constraint::MinKeys(_) | Constraint::MaxKeys(_)))
            }
            _ => false,
        };
        if !allowed {
            let constraint = value_constraints[0];
            return Err(invalid(
                path,
                format!(
                    "constraint `{}` is not valid on a reference to the recursive named type `{}`",
                    constraint.keyword(),
                    key.name
                ),
            ));
        }
        let description = value_constraints
            .iter()
            .map(|constraint| constraint.keyword())
            .collect::<Vec<_>>()
            .join("; ");
        Ok(shape.assert(format_assertion(&description, Value::Object(fragment))?))
    }

    /// The lowering's `pattern_value_schema`.
    fn bare_def(&mut self, path: &str, def: &PropertyDef) -> Result<Shape, TranslateError> {
        match self.inline_reference(path, def)? {
            Some(Inlined::Definition(key, resolved)) => return self.within(key, |this| this.bare_def(path, &resolved)),
            Some(Inlined::Recursive(shape, _)) => return Ok(shape),
            None => {}
        }
        match def {
            PropertyDef::Single(atom) => Ok(self.atom(path, atom, false)?.0),
            PropertyDef::Union(arms) => {
                let mut options = Vec::with_capacity(arms.len());
                for arm in arms {
                    options.push(self.atom(path, arm, false)?.0);
                }
                Ok(Shape::Union(options))
            }
        }
    }

    /// The lowering's `property_def_to_schema`: the shape and whether the
    /// enclosing object lists the property as required.
    fn property(&mut self, path: &str, def: &PropertyDef) -> Result<(Shape, bool), TranslateError> {
        match self.inline_reference(path, def)? {
            Some(Inlined::Definition(key, resolved)) => return self.within(key, |this| this.property(path, &resolved)),
            Some(Inlined::Recursive(shape, target)) => {
                // The presence rules the inlined target would get: a type
                // that admits null is never required, an optional use also
                // admits null.
                return Ok(if target.admits_null() {
                    (shape, false)
                } else if def.is_required() {
                    (shape, !matches!(def, PropertyDef::Single(atom) if is_generated(atom)))
                } else {
                    (Shape::Union(vec![Shape::Null, shape]), false)
                });
            }
            None => {}
        }
        match def {
            PropertyDef::Single(atom) => match atom.union_arms() {
                Some(arms) if !atom.is_array => self.union_property(path, arms, Some(atom)),
                _ => self.atom_property(path, atom),
            },
            PropertyDef::Union(arms) => self.union_property(path, arms, None),
        }
    }

    /// A bare reference with no value constraints names its target's whole
    /// definition, as the resolver inlines it: the reference's presence and
    /// documentation markers move onto every option. A use that closes a
    /// cycle through a value descent is a reusable reference instead.
    fn inline_reference(&mut self, path: &str, def: &PropertyDef) -> Result<Option<Inlined>, TranslateError> {
        let PropertyDef::Single(atom) = def else {
            return Ok(None);
        };
        let TypeExpr::Imported { name, reference } = &atom.ty else {
            return Ok(None);
        };
        if atom.is_array || !atom.constraints.iter().all(is_annotation) {
            return Ok(None);
        }
        let key = NamedKey::Imported(name.clone(), reference.clone());
        if self.recurses(&key) {
            let target = self.resolve(path, name, reference)?;
            let shape = self.reusable(path, key, &target)?;
            return Ok(Some(Inlined::Recursive(shape, target)));
        }
        let mut target = self.resolve(path, name, reference)?;
        let markers = atom.constraints.clone();
        let mark = |option: &mut PropertyAtom| {
            if option.is_array {
                option.array_constraints.extend(markers.iter().cloned());
            } else {
                option.constraints.extend(markers.iter().cloned());
            }
        };
        match &mut target {
            PropertyDef::Single(option) => mark(option),
            PropertyDef::Union(options) => options.iter_mut().for_each(mark),
        }
        Ok(Some(Inlined::Definition(key, target)))
    }

    fn resolve(&self, path: &str, name: &str, reference: &str) -> Result<PropertyDef, TranslateError> {
        if self.depth >= MAX_REFERENCE_DEPTH {
            return Err(TranslateError::TooDeep {
                path: path.to_string(),
                limit: MAX_REFERENCE_DEPTH,
            });
        }
        self.resolver
            .resolve(name, reference)
            .ok_or_else(|| TranslateError::UnresolvedReference {
                path: path.to_string(),
                name: name.to_string(),
                reference: reference.to_string(),
            })
    }

    /// The lowering's `union_property_to_schema`.
    fn union_property(
        &mut self,
        path: &str,
        arms: &[PropertyAtom],
        container: Option<&PropertyAtom>,
    ) -> Result<(Shape, bool), TranslateError> {
        let admits_null = container.map_or_else(|| arms.iter().any(PropertyAtom::admits_null), PropertyAtom::admits_null);
        let required = !admits_null
            && container.map_or_else(|| arms.iter().any(marks_required), marks_required);
        let generated = container.map_or_else(|| arms.iter().any(is_generated), is_generated);
        let mut options = Vec::with_capacity(arms.len() + 1);
        if !required && !admits_null {
            options.push(Shape::Null);
        }
        let mut hoisted: Option<Value> = None;
        for arm in arms {
            options.push(self.atom(path, arm, !required)?.0);
            if let Some(default) = atom_default(path, arm)? {
                agree(path, &mut hoisted, default, "conflicting `default(...)` values across union arms")?;
            }
        }
        Ok((Shape::Union(options), required && !generated))
    }

    /// The lowering's `atom_to_schema`.
    fn atom_property(&mut self, path: &str, atom: &PropertyAtom) -> Result<(Shape, bool), TranslateError> {
        let (shape, required) = self.atom(path, atom, true)?;
        if atom.admits_null() {
            return Ok((shape, false));
        }
        if required {
            return Ok((shape, !is_generated(atom)));
        }
        let wrapped = match shape {
            // The optional scalar `file` already carries its `""` option.
            Shape::Union(mut options) if is_scalar_file(atom) => {
                options.insert(0, Shape::Null);
                Shape::Union(options)
            }
            other => Shape::Union(vec![Shape::Null, other]),
        };
        Ok((wrapped, false))
    }

    /// The lowering's `atom_fragment_without_null_wrap`: the atom's shape and
    /// whether it is effectively required.
    fn atom(
        &mut self,
        path: &str,
        atom: &PropertyAtom,
        allow_optional_empty_file: bool,
    ) -> Result<(Shape, bool), TranslateError> {
        let required = atom.is_required();
        atom_default(path, atom)?;
        let value_constraints: Vec<Constraint> = atom
            .constraints
            .iter()
            .filter(|constraint| {
                !matches!(constraint, Constraint::Category(_) | Constraint::Operands(_) | Constraint::Deferred)
            })
            .cloned()
            .collect();
        let deferred_value = atom.constraints.contains(&Constraint::Deferred);
        let deferred_list = atom.is_array && atom.array_constraints.contains(&Constraint::Deferred);
        let inner = if atom.is_array {
            let item = self.descend(|this| this.atom_type(path, atom, &value_constraints, allow_optional_empty_file, required))?;
            if deferred_value { item.deferrable() } else { item }
        } else {
            self.atom_type(path, atom, &value_constraints, allow_optional_empty_file, required)?
        };
        let optional_empty_file = allow_optional_empty_file && is_scalar_file(atom) && !required;
        let shape = if atom.is_array {
            array(path, inner, &atom.array_constraints)?
        } else if optional_empty_file {
            Shape::Union(vec![Shape::Literal(json!("")), inner])
        } else {
            inner
        };
        // The lowering marks the same node: the list, or the value itself.
        let shape = if deferred_list || (deferred_value && !atom.is_array) { shape.deferrable() } else { shape };
        Ok((shape, required))
    }

    /// The shape of the atom's type itself, before any list wrapper: the
    /// item shape of `T[]`.
    fn atom_type(
        &mut self,
        path: &str,
        atom: &PropertyAtom,
        value_constraints: &[Constraint],
        allow_optional_empty_file: bool,
        required: bool,
    ) -> Result<Shape, TranslateError> {
        Ok(match &atom.ty {
            TypeExpr::Primitive(ty) => primitive(path, *ty, value_constraints)?,
            TypeExpr::InlineObject(shape) => self.inline_object(path, shape, value_constraints)?,
            TypeExpr::Tuple(tuple) => self.tuple(path, tuple)?,
            TypeExpr::Function(_) => {
                return Err(TranslateError::FunctionValue { path: path.to_string() });
            }
            TypeExpr::Union(arms) => {
                let mut options = Vec::with_capacity(arms.len());
                let mut hoisted: Option<Value> = None;
                for arm in arms {
                    options.push(self.atom(path, arm, allow_optional_empty_file && !atom.is_array && !required)?.0);
                    if let Some(default) = atom_default(path, arm)? {
                        if atom.is_array {
                            return Err(invalid(
                                path,
                                format!("an item alternative carries `default({default})`, which cannot default the array"),
                            ));
                        }
                        agree(path, &mut hoisted, default, "conflicting `default(...)` values")?;
                    }
                }
                Shape::Union(options)
            }
            TypeExpr::Imported { name, reference } => self.reference(path, atom, name, reference)?,
            TypeExpr::Ref(key) => self.definition_reference(path, key, value_constraints)?,
        })
    }

    /// A reference that carries value constraints or `[]`: the target's bare
    /// shape, with the constraints merged into a single non-list target the
    /// way the resolver merges them.
    fn reference(
        &mut self,
        path: &str,
        atom: &PropertyAtom,
        name: &str,
        reference: &str,
    ) -> Result<Shape, TranslateError> {
        let target = self.resolve(path, name, reference)?;
        let constraints: Vec<Constraint> =
            atom.constraints.iter().filter(|constraint| !is_annotation(constraint)).cloned().collect();
        let key = NamedKey::Imported(name.to_string(), reference.to_string());
        if self.recurses(&key) {
            if constraints.is_empty() {
                return self.reusable(path, key, &target);
            }
            return Err(invalid(
                path,
                format!("constraints on `{name}@{reference}` cannot apply to a use inside its own definition"),
            ));
        }
        self.within(key, |this| {
            if constraints.is_empty() {
                this.bare_def(path, &target)
            } else {
                match target {
                    PropertyDef::Single(mut target) if !target.is_array && target.union_arms().is_none() => {
                        target.constraints.extend(constraints);
                        this.atom(path, &target, false).map(|(shape, _)| shape)
                    }
                    _ => Err(invalid(
                        path,
                        format!("constraints on `{name}@{reference}` apply only to a single, non-list named type"),
                    )),
                }
            }
        })
    }

    /// The lowering's `inline_object_fragment`: closed unless it declares a
    /// `<string>` catch-all.
    fn inline_object(
        &mut self,
        path: &str,
        shape: &SchemaShape,
        atom_constraints: &[Constraint],
    ) -> Result<Shape, TranslateError> {
        for constraint in atom_constraints {
            if !matches!(
                constraint,
                Constraint::Required
                    | Constraint::Default(_)
                    | Constraint::Generated
                    | Constraint::Example(_)
                    | Constraint::Eager
                    | Constraint::MinKeys(_)
                    | Constraint::MaxKeys(_)
            ) {
                return Err(invalid(
                    path,
                    format!("constraint `{}` is not valid on an inline object", constraint.keyword()),
                ));
            }
        }
        let mut object = self.object_body(path, shape, Additional::Forbidden)?;
        reject_block_constraints(path, shape, "an inline object")?;
        object_arity(path, &mut object, shape.constraints.iter().chain(atom_constraints))?;
        Ok(Shape::Object(object))
    }

    /// The lowering's `shape_to_object_schema`: open unless it declares
    /// pattern keys.
    fn root_object(&mut self, shape: &SchemaShape) -> Result<Shape, TranslateError> {
        let mut object = self.object_body("", shape, Additional::Any)?;
        reject_block_constraints("<root>", shape, "the root object")?;
        object_arity("<root>", &mut object, shape.constraints.iter())?;
        Ok(Shape::Object(object))
    }

    /// The lowering's `object_body_from_shape`. Literal keys win over pattern
    /// keys (the lowering excludes them from every pattern), which is
    /// [`PatternScope::UndeclaredKeys`]; the `<string>` catch-all is the
    /// `additionalProperties` schema, so it covers only keys no other
    /// pattern matches.
    fn object_body(
        &mut self,
        path: &str,
        shape: &SchemaShape,
        default_additional: Additional,
    ) -> Result<ObjectShape, TranslateError> {
        let mut properties = Vec::with_capacity(shape.properties.len());
        for (name, def) in &shape.properties {
            let child = child_key(path, name);
            let (property, required) = self.descend(|this| this.property(&child, def))?;
            properties.push(PropertySlot {
                name: name.clone(),
                shape: property,
                required,
            });
        }
        let mut catch_all = None;
        let mut patterns = Vec::new();
        for pattern_key in &shape.pattern_keys {
            let value = self.descend(|this| this.bare_def(path, &pattern_key.def))?;
            let matcher = match &pattern_key.key {
                PatternKey::CatchAll => {
                    catch_all = Some(value);
                    continue;
                }
                PatternKey::Starting(prefix) => KeyMatcher::Prefix(prefix.clone()),
                PatternKey::Ending(suffix) => KeyMatcher::Suffix(suffix.clone()),
                PatternKey::Pattern(source) => KeyMatcher::Pattern(compile_pattern(path, source)?),
            };
            patterns.push(PatternSlot { matcher, shape: value });
        }
        let additional = match catch_all {
            Some(value) => Additional::Shape(Box::new(value)),
            None if shape.pattern_keys.is_empty() => default_additional,
            None => Additional::Forbidden,
        };
        Ok(ObjectShape {
            properties,
            patterns,
            pattern_scope: PatternScope::UndeclaredKeys,
            additional,
            min_properties: None,
            max_properties: None,
            uncounted: Vec::new(),
        })
    }

    /// The lowering's `tuple_fragment`: positions are bare (an optional
    /// position may be missing, never `null` unless its type says so).
    fn tuple(&mut self, path: &str, tuple: &TupleType) -> Result<Shape, TranslateError> {
        let mut prefix = Vec::with_capacity(tuple.elements.len());
        for (index, element) in tuple.elements.iter().enumerate() {
            prefix.push(TupleSlot {
                shape: self.descend(|this| this.bare_def(&format!("{path}[{index}]"), &element.def))?,
                optional: element.optional,
            });
        }
        let rest = match &tuple.spread {
            Some(spread) => {
                let mut item = spread.as_ref().clone();
                item.is_array = false;
                item.array_constraints.clear();
                Some(Box::new(self.descend(|this| this.atom(&format!("{path}[...]"), &item, false))?.0))
            }
            None => None,
        };
        Ok(Shape::Tuple(TupleShape { prefix, rest }))
    }
}

fn marks_required(atom: &PropertyAtom) -> bool {
    atom.property_constraints().any(|constraint| matches!(constraint, Constraint::Required))
        || atom.union_arms().is_some_and(|arms| arms.iter().any(marks_required))
}

fn is_generated(atom: &PropertyAtom) -> bool {
    atom.property_constraints().any(|constraint| matches!(constraint, Constraint::Generated))
        || atom.union_arms().is_some_and(|arms| arms.iter().any(is_generated))
}

fn is_scalar_file(atom: &PropertyAtom) -> bool {
    matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File)) && !atom.is_array
}

fn child_key(path: &str, key: &str) -> String {
    if path.is_empty() { key.to_string() } else { format!("{path}.{key}") }
}

fn invalid(path: &str, message: String) -> TranslateError {
    TranslateError::Invalid {
        path: path.to_string(),
        message,
    }
}

fn invalid_constraint(path: &str, ty: &str, constraint: &Constraint) -> TranslateError {
    TranslateError::InvalidConstraint {
        path: path.to_string(),
        ty: ty.to_string(),
        constraint: constraint.keyword().to_string(),
    }
}

/// The atom's one `default(...)`; repeated defaults must agree.
fn atom_default(path: &str, atom: &PropertyAtom) -> Result<Option<Value>, TranslateError> {
    let mut found = None;
    for constraint in atom.property_constraints() {
        if let Constraint::Default(value) = constraint {
            agree(path, &mut found, normalize_number(value.clone()), "conflicting `default(...)` values")?;
        }
    }
    Ok(found)
}

fn agree(path: &str, slot: &mut Option<Value>, value: Value, message: &str) -> Result<(), TranslateError> {
    match slot {
        Some(existing) if *existing != value => Err(invalid(path, format!("{message}: {existing} vs {value}"))),
        Some(_) => Ok(()),
        None => {
            *slot = Some(value);
            Ok(())
        }
    }
}

/// An integral float as an integer (`3.0` is `3`), as the lowering writes
/// defaults and literal values.
fn normalize_number(value: Value) -> Value {
    match value {
        Value::Number(number) if number.is_i64() || number.is_u64() => Value::Number(number),
        Value::Number(number) => match number.as_f64().and_then(json_number) {
            Some(normalized) => Value::Number(normalized),
            None => Value::Number(number),
        },
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_number).collect()),
        Value::Object(map) => Value::Object(map.into_iter().map(|(key, item)| (key, normalize_number(item))).collect()),
        other => other,
    }
}

/// Repeated bounds keep the stricter value; a lower bound above the upper is
/// a contradiction.
struct Bounds<T> {
    lower: Option<(T, String)>,
    upper: Option<(T, String)>,
}

impl<T: PartialOrd + Copy> Bounds<T> {
    fn new() -> Self {
        Self { lower: None, upper: None }
    }

    fn lower(&mut self, value: T, written: String) {
        if self.lower.as_ref().is_none_or(|(current, _)| value > *current) {
            self.lower = Some((value, written));
        }
    }

    fn upper(&mut self, value: T, written: String) {
        if self.upper.as_ref().is_none_or(|(current, _)| value < *current) {
            self.upper = Some((value, written));
        }
    }

    fn finish(self, path: &str) -> Result<(Option<T>, Option<T>), TranslateError> {
        if let (Some((low, low_written)), Some((high, high_written))) = (&self.lower, &self.upper)
            && low > high
        {
            return Err(invalid(
                path,
                format!("contradictory bounds: `{low_written}` is greater than `{high_written}`"),
            ));
        }
        Ok((self.lower.map(|(value, _)| value), self.upper.map(|(value, _)| value)))
    }
}

fn array(path: &str, items: Shape, constraints: &[Constraint]) -> Result<Shape, TranslateError> {
    let mut count = Bounds::new();
    let mut unique_items = false;
    for constraint in constraints {
        match constraint {
            constraint if is_annotation(constraint) => {}
            Constraint::MinItems(n) => count.lower(*n, format!("min({n})")),
            Constraint::MaxItems(n) => count.upper(*n, format!("max({n})")),
            Constraint::Unique => unique_items = true,
            other => {
                return Err(invalid(
                    path,
                    format!("constraint `{}` is not valid at the array level", other.keyword()),
                ));
            }
        }
    }
    let (min_items, max_items) = count.finish(path)?;
    Ok(Shape::Array(ArrayShape {
        items: Box::new(items),
        min_items,
        max_items,
        unique_items,
    }))
}

fn reject_block_constraints(path: &str, shape: &SchemaShape, place: &str) -> Result<(), TranslateError> {
    match shape
        .constraints
        .iter()
        .find(|constraint| !matches!(constraint, Constraint::MinKeys(_) | Constraint::MaxKeys(_)))
    {
        Some(constraint) => Err(invalid(
            path,
            format!("`$constraints` key `{}` is not valid on {place}", block_key(constraint)),
        )),
        None => Ok(()),
    }
}

fn block_key(constraint: &Constraint) -> &'static str {
    match constraint {
        Constraint::MinItems(_) => "min-items",
        Constraint::MaxItems(_) => "max-items",
        other => other.keyword(),
    }
}

fn object_arity<'c>(
    path: &str,
    object: &mut ObjectShape,
    constraints: impl Iterator<Item = &'c Constraint>,
) -> Result<(), TranslateError> {
    let mut keys = Bounds::new();
    for constraint in constraints {
        match constraint {
            Constraint::MinKeys(n) => keys.lower(*n, format!("min-keys({n})")),
            Constraint::MaxKeys(n) => keys.upper(*n, format!("max-keys({n})")),
            _ => {}
        }
    }
    (object.min_properties, object.max_properties) = keys.finish(path)?;
    Ok(())
}

/// Rejects every constraint beyond the universal ones, as the lowering does
/// for types that take no constraint of their own.
fn reject_unsupported(path: &str, ty: &str, constraints: &[Constraint]) -> Result<(), TranslateError> {
    match constraints.iter().find(|constraint| !is_annotation(constraint)) {
        Some(constraint) => Err(invalid_constraint(path, ty, constraint)),
        None => Ok(()),
    }
}

fn compile_pattern(path: &str, source: &str) -> Result<TextPattern, TranslateError> {
    TextPattern::new(source).map_err(|error| invalid(path, format!("invalid pattern `{source}`: {error}")))
}

/// The lowering's per-type fragment builders.
fn primitive(path: &str, ty: SimplifiedType, constraints: &[Constraint]) -> Result<Shape, TranslateError> {
    let keyword = ty.as_keyword();
    match ty {
        SimplifiedType::String => text(path, keyword, TextShape::default(), Vec::new(), constraints),
        SimplifiedType::Date => refined(path, keyword, constraints, json!({ "format": "date" })),
        SimplifiedType::DateTime => refined(path, keyword, constraints, json!({ "format": DARKMATTER_DATETIME_FORMAT })),
        SimplifiedType::Time => refined(path, keyword, constraints, json!({ "format": DARKMATTER_TIME_FORMAT })),
        SimplifiedType::Email => refined(path, keyword, constraints, json!({ "format": "email" })),
        SimplifiedType::Number => number(path, constraints),
        SimplifiedType::NumberLike => {
            reject_unsupported(path, keyword, constraints)?;
            Ok(Shape::Number(NumberShape {
                as_sent: true,
                bounds: NumericBounds::default(),
            }))
        }
        SimplifiedType::Boolean | SimplifiedType::Boolish => {
            reject_unsupported(path, keyword, constraints)?;
            Ok(Shape::Boolean {
                as_sent: ty == SimplifiedType::Boolish,
            })
        }
        SimplifiedType::Object => {
            reject_unsupported(path, keyword, constraints)?;
            Ok(Shape::Object(ObjectShape::opaque()))
        }
        SimplifiedType::File => {
            if let Some(constraint) = constraints
                .iter()
                .find(|constraint| !is_annotation(constraint) && !matches!(constraint, Constraint::Match(_)))
            {
                return Err(invalid_constraint(path, keyword, constraint));
            }
            // Syntax only: existence is never checked passively, even for
            // `file(eager)`.
            format_text(keyword, json!({ "format": DARKMATTER_FILE_REFERENCE_FORMAT }))
        }
        SimplifiedType::Enum => {
            let mut members = None;
            for constraint in constraints {
                match constraint {
                    constraint if is_annotation(constraint) => {}
                    Constraint::Members(written) => members = Some(written.clone()),
                    other => return Err(invalid_constraint(path, keyword, other)),
                }
            }
            let members = members.ok_or_else(|| invalid(path, "`enum` requires at least one member".to_string()))?;
            Ok(Shape::Text(TextShape {
                members: Some(members),
                ..TextShape::default()
            }))
        }
        SimplifiedType::Url => url(path, constraints),
        SimplifiedType::Yaml | SimplifiedType::Json => {
            reject_unsupported(path, keyword, constraints)?;
            Ok(Shape::Content(if ty == SimplifiedType::Json {
                ContentFormat::Json
            } else {
                ContentFormat::Yaml
            }))
        }
        SimplifiedType::Literal => literal(path, constraints),
        SimplifiedType::Expression => {
            reject_unsupported(path, keyword, constraints)?;
            for constraint in constraints {
                if let Constraint::Default(value) = constraint
                    && crate::markdown::compose::expression::parse_condition(&expression_source(value)).is_err()
                {
                    return Err(invalid(path, format!("`default({value})` is not a valid Darkmatter expression")));
                }
            }
            format_text(keyword, json!({ "format": DARKMATTER_EXPRESSION_FORMAT }))
        }
        SimplifiedType::Shell => shell(path, constraints),
        SimplifiedType::TypeDefinition | SimplifiedType::Schema => {
            reject_unsupported(path, keyword, constraints)?;
            let flag = if ty == SimplifiedType::Schema {
                DARKMATTER_SCHEMA_KEYWORD
            } else {
                DARKMATTER_TYPE_DEFINITION_KEYWORD
            };
            let structure = Shape::Union(vec![
                Shape::text(),
                Shape::Object(ObjectShape::opaque()),
                Shape::array(Shape::Any),
            ]);
            Ok(structure.assert(format_assertion(keyword, json!({ flag: true }))?))
        }
        SimplifiedType::Unknown => {
            reject_unsupported(path, keyword, constraints)?;
            Ok(Shape::Any)
        }
        SimplifiedType::Null => {
            reject_unsupported(path, keyword, constraints)?;
            if let Some(default) = constraints.iter().find_map(|constraint| match constraint {
                Constraint::Default(value) if !value.is_null() => Some(value),
                _ => None,
            }) {
                return Err(invalid(
                    path,
                    format!("`default({default})` is not valid on `null`; a `null` default must be null"),
                ));
            }
            Ok(Shape::Null)
        }
        SimplifiedType::Alphanumeric => alphanumeric(path, constraints),
    }
}

/// Text with string constraints. `base_patterns` stay separate from every
/// authored pattern; all of them must match.
fn text(
    path: &str,
    ty: &str,
    mut shape: TextShape,
    mut patterns: Vec<String>,
    constraints: &[Constraint],
) -> Result<Shape, TranslateError> {
    let mut length = Bounds::new();
    for constraint in constraints {
        match constraint {
            constraint if is_annotation(constraint) => {}
            Constraint::Suggest(_) => {}
            Constraint::MinLen(n) => length.lower(*n, format!("min({n})")),
            Constraint::MaxLen(n) => length.upper(*n, format!("max({n})")),
            Constraint::NotEmpty => push_unique(&mut patterns, r"\S"),
            Constraint::Pattern(source) => push_unique(&mut patterns, source),
            other => return Err(invalid_constraint(path, ty, other)),
        }
    }
    (shape.min_length, shape.max_length) = length.finish(path)?;
    let mut sources = patterns.into_iter();
    if let Some(first) = sources.next() {
        shape.pattern = Some(compile_pattern(path, &first)?);
    }
    let mut result = Shape::Text(shape);
    for source in sources {
        let pattern = compile_pattern(path, &source)?;
        result = result.assert(Assertion::new(format!("pattern({source})"), move |value| {
            value.as_str().is_some_and(|text| pattern.is_match(text))
        }));
    }
    Ok(result)
}

fn push_unique(patterns: &mut Vec<String>, pattern: &str) {
    if !patterns.iter().any(|existing| existing == pattern) {
        patterns.push(pattern.to_string());
    }
}

fn number(path: &str, constraints: &[Constraint]) -> Result<Shape, TranslateError> {
    let mut range = Bounds::new();
    let mut integer = None;
    for constraint in constraints {
        match constraint {
            constraint if is_annotation(constraint) => {}
            Constraint::Suggest(_) => {}
            Constraint::Min(n) => range.lower(*n, format!("min({n})")),
            Constraint::Max(n) => range.upper(*n, format!("max({n})")),
            Constraint::Integer => integer = Some(IntegerRule::WholeBelow2Pow64),
            other => return Err(invalid_constraint(path, "number", other)),
        }
    }
    let (minimum, maximum) = range.finish(path)?;
    Ok(Shape::Number(NumberShape {
        as_sent: false,
        bounds: NumericBounds {
            minimum,
            maximum,
            integer,
            ..NumericBounds::default()
        },
    }))
}

fn refined(path: &str, ty: &str, constraints: &[Constraint], fragment: Value) -> Result<Shape, TranslateError> {
    reject_unsupported(path, ty, constraints)?;
    format_text(ty, fragment)
}

/// Text whose final value must also pass a format validator.
fn format_text(description: &str, fragment: Value) -> Result<Shape, TranslateError> {
    Ok(Shape::text().assert(format_assertion(description, fragment)?))
}

fn url(path: &str, constraints: &[Constraint]) -> Result<Shape, TranslateError> {
    let mut allowed: Option<Vec<String>> = None;
    let mut written = Vec::new();
    for constraint in constraints {
        match constraint {
            constraint if is_annotation(constraint) => {}
            Constraint::Scheme(schemes) => {
                written.push(format!("`scheme({})`", schemes.join(",")));
                allowed = Some(match allowed {
                    None => schemes.clone(),
                    Some(mut current) => {
                        current.retain(|scheme| schemes.contains(scheme));
                        current
                    }
                });
            }
            other => return Err(invalid_constraint(path, "url", other)),
        }
    }
    let mut fragment = json!({ "format": "uri" });
    let mut description = "url".to_string();
    if let Some(schemes) = allowed {
        if schemes.is_empty() {
            return Err(invalid(
                path,
                format!("contradictory constraints: {} share no scheme", written.join(" and ")),
            ));
        }
        description = format!("url(scheme({}))", schemes.join(","));
        fragment[DARKMATTER_URL_SCHEME_KEYWORD] = json!(schemes);
    }
    format_text(&description, fragment)
}

fn literal(path: &str, constraints: &[Constraint]) -> Result<Shape, TranslateError> {
    let mut value = None;
    let mut default = None;
    for constraint in constraints {
        match constraint {
            Constraint::Required | Constraint::Generated | Constraint::Example(_) | Constraint::Eager => {}
            Constraint::LiteralValue(written) => value = Some(normalize_number(written.clone())),
            Constraint::Default(written) => default = Some(normalize_number(written.clone())),
            other => return Err(invalid_constraint(path, "literal", other)),
        }
    }
    let value = value.ok_or_else(|| invalid(path, "`literal` requires a value".to_string()))?;
    if let Some(default) = default
        && default != value
    {
        return Err(invalid(path, format!("`default({default})` must equal the literal value `{value}`")));
    }
    Ok(Shape::Literal(value))
}

fn shell(path: &str, constraints: &[Constraint]) -> Result<Shape, TranslateError> {
    reject_unsupported(path, "shell", constraints)?;
    for constraint in constraints {
        let Constraint::Default(value) = constraint else {
            continue;
        };
        let text = match value {
            Value::String(text) => text.clone(),
            Value::Bool(flag) => flag.to_string(),
            Value::Number(number) => number.to_string(),
            Value::Null | Value::Array(_) | Value::Object(_) => {
                return Err(invalid(
                    path,
                    format!("`default({value})` is not valid on `shell`; a `shell` default must be text"),
                ));
            }
        };
        if text.chars().all(char::is_whitespace) {
            return Err(invalid(
                path,
                format!("`default({value})` is not a valid `shell` value; a command needs a non-whitespace character"),
            ));
        }
    }
    format_text("shell", json!({ "format": DARKMATTER_SHELL_FORMAT }))
}

/// Text of ASCII letters and digits under the canonical options. Unlike
/// `string`, a native number or boolean never converts into it.
fn alphanumeric(path: &str, constraints: &[Constraint]) -> Result<Shape, TranslateError> {
    let options = constraints
        .iter()
        .filter_map(|constraint| match constraint {
            Constraint::Options(options) => Some(options.iter().copied()),
            _ => None,
        })
        .flatten()
        .collect::<AlphanumericOptions>()
        .canonical();
    let rest: Vec<Constraint> = constraints
        .iter()
        .filter(|constraint| !matches!(constraint, Constraint::Options(_)))
        .cloned()
        .collect();
    let base = TextShape {
        scalar_text: false,
        ..TextShape::default()
    };
    let shape = text(path, "alphanumeric", base, vec![options.pattern()], &rest)?;
    for constraint in &rest {
        let (keyword, values): (&str, Vec<&Value>) = match constraint {
            Constraint::Default(value) => ("default", vec![value]),
            Constraint::Suggest(candidates) => ("suggest", candidates.iter().map(|candidate| &candidate.interpreted).collect()),
            _ => continue,
        };
        if let Some(value) = values.into_iter().find(|value| !accepts_exactly(&shape, value)) {
            return Err(invalid(
                path,
                format!("`{keyword}({value})` is not a valid `alphanumeric` value for this type"),
            ));
        }
    }
    Ok(shape)
}

fn expression_source(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        other => other.to_string(),
    }
}

/// A passive assertion judged by the validator JSON Schema validation builds
/// for `fragment`, so both translators agree on what a format accepts.
/// Validators are built once per distinct fragment for the process.
fn format_assertion(description: &str, fragment: Value) -> Result<Assertion, TranslateError> {
    type Validators = Mutex<HashMap<String, Arc<jsonschema::Validator>>>;
    static VALIDATORS: OnceLock<Validators> = OnceLock::new();
    let key = fragment.to_string();
    let validators = VALIDATORS.get_or_init(Default::default);
    let cached = validators.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get(&key).cloned();
    let validator = match cached {
        Some(validator) => validator,
        None => {
            let built = crate::markdown::schemas::validate::build_structural_validator(&fragment)
                .map_err(|error| invalid("", format!("cannot build the `{description}` check: {error}")))?;
            let built = Arc::new(built);
            validators
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(key, Arc::clone(&built));
            built
        }
    };
    Ok(Assertion::new(description, move |value| validator.is_valid(value)))
}
