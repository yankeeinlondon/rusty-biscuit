//! Borrowed, read-only views of a [`SimplifiedSchema`]'s typed definitions,
//! and the passive type query that finds them.
//!
//! A query names a place in the *schema*, not in data: `people[0].name` is
//! the type of `name` in an element of the `people` list. It never evaluates
//! anything and never reads a value. It is answered by the completion
//! module's path walk ([`crate::completion::def_at_path`]), so its results
//! are the ones editors already get: references are followed one level at a
//! time through the schema's stored definitions, a property several union
//! arms declare is the union of their types, and an empty path or a missing
//! key is `None`.

use std::borrow::Cow;
use std::fmt;

use serde_json::Value;

use super::SimplifiedSchema;
use super::layer::{Typed, declares, json_declares};
use crate::completion::{AlternativeSite, PathStep, def_at_path};
use crate::simplified::query::resolved_alternatives;
use crate::simplified::{
    Alternative, CallExample, Constraint, DefinitionTable, Slot, FunctionParameter, FunctionType, NarrowsClause, PropertyAtom,
    PropertyDef, SchemaArm, SchemaRoot, SchemaShape, TupleType, TypeExpr, serialize::serialize_property_def,
};

/// One type in a schema, borrowed from it.
///
/// Usually borrows the declaration in place; a property declared by several
/// root-union arms is answered with the union of their types, which the view
/// then owns ([`Self::is_borrowed`] tells which).
#[derive(Debug, Clone)]
pub struct TypeDefinition<'a> {
    def: Cow<'a, PropertyDef>,
    site: AlternativeSite,
    definitions: &'a DefinitionTable,
}

impl<'a> TypeDefinition<'a> {
    fn new(def: Cow<'a, PropertyDef>, site: AlternativeSite, definitions: &'a DefinitionTable) -> Self {
        Self { def, site, definitions }
    }

    /// The declaration, as the parser produced it (named types expanded).
    pub fn def(&self) -> &PropertyDef {
        &self.def
    }

    /// Where the type sits: [`AlternativeSite::Value`] for a property,
    /// [`AlternativeSite::Item`] for an element of a list property (the
    /// declaration is then the list's, and its alternatives the element's).
    pub fn site(&self) -> AlternativeSite {
        self.site
    }

    /// Whether the view borrows the declaration in place rather than owning
    /// a union assembled for the query.
    pub fn is_borrowed(&self) -> bool {
        matches!(self.def, Cow::Borrowed(_))
    }

    /// The declaration in canonical SimplifiedSchema spelling.
    pub fn type_text(&self) -> String {
        serialize_property_def(&self.def)
    }

    /// The `-> description` of a single-atom declaration.
    pub fn description(&self) -> Option<&str> {
        match self.def.as_ref() {
            PropertyDef::Single(atom) => atom.description.as_deref(),
            PropertyDef::Union(_) => None,
        }
    }

    /// Whether the property is effectively required ([`PropertyDef::is_required`]).
    pub fn is_required(&self) -> bool {
        self.def.is_required()
    }

    /// The leaf alternatives at this site, with references opened one level
    /// through the schema's stored definitions.
    pub fn alternatives(&self) -> Vec<Alternative<'_>> {
        let is_item = self.site == AlternativeSite::Item;
        resolved_alternatives(&self.def, self.definitions)
            .into_iter()
            .filter(|alternative| alternative.is_array == is_item)
            .collect()
    }

    /// The function signature of a declaration that is exactly one
    /// `function(...)` type (not a list of them).
    pub fn function(&self) -> Option<FunctionView<'_>> {
        let PropertyDef::Single(atom) = self.def.as_ref() else {
            return None;
        };
        FunctionView::of(atom)
    }

    /// The unresolved `{{ … }}` slots this type holds, directly or through a
    /// stored definition it references, each once. Non-empty means the type
    /// is part of a template: its `enum(...)`/`suggest(...)` lists show only
    /// their literal elements and are not complete, and its template constraints
    /// are not known.
    pub fn pending_slots(&self) -> Vec<Slot> {
        crate::slots::reachable_slots(&self.def, self.definitions)
    }

    /// The stored definitions references in this type point into.
    pub fn definitions(&self) -> &'a DefinitionTable {
        self.definitions
    }
}

/// A `function(...)` type, borrowed: its parameters with their names,
/// order, types, and optional/rest markers, its return type, fallibility,
/// examples, `narrows(...)` fact, and the metadata on its declaration.
#[derive(Debug, Clone, Copy)]
pub struct FunctionView<'a> {
    atom: &'a PropertyAtom,
    function: &'a FunctionType,
}

impl<'a> FunctionView<'a> {
    /// The view of `atom` when it is a single (non-list) function type.
    pub fn of(atom: &'a PropertyAtom) -> Option<Self> {
        match &atom.ty {
            TypeExpr::Function(function) if !atom.is_array => Some(Self { atom, function }),
            _ => None,
        }
    }

    /// The parameters in declaration order; empty for `parameters(void)`.
    pub fn parameters(&self) -> &'a [FunctionParameter] {
        self.function.parameters.as_slice()
    }

    /// The type returned on success.
    pub fn returns(&self) -> &'a PropertyDef {
        &self.function.returns
    }

    /// Whether the function may return an error (`fallible`).
    pub fn fallible(&self) -> bool {
        self.function.fallible
    }

    /// Inline `example(call => result)` constraints, in authored order.
    pub fn examples(&self) -> &'a [CallExample] {
        &self.function.examples
    }

    /// What a boolean result proves about an argument, if declared.
    pub fn narrows(&self) -> Option<&'a NarrowsClause> {
        self.function.narrows.as_ref()
    }

    /// The declaration's `-> description`.
    pub fn description(&self) -> Option<&'a str> {
        self.atom.description.as_deref()
    }

    /// The declaration's constraints (`category(...)`, file `example(...)`).
    pub fn constraints(&self) -> &'a [Constraint] {
        &self.atom.constraints
    }

    /// The whole signature.
    pub fn function_type(&self) -> &'a FunctionType {
        self.function
    }
}

/// A type-query path that does not parse.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid schema path `{path}` at byte {offset}: {reason}")]
pub struct PathSyntaxError {
    /// The path as written.
    pub path: String,
    /// Byte offset of the problem.
    pub offset: usize,
    /// What was expected.
    pub reason: &'static str,
}

/// One parsed path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Key(String),
    Index(usize),
}

/// Parses `name.child`, `list[0]`, and `map["a.b"]` (or `['a.b']`, with
/// `\` escaping the quote and itself) into segments. The empty path is no
/// segments.
fn parse_path(path: &str) -> Result<Vec<Segment>, PathSyntaxError> {
    let error = |offset: usize, reason: &'static str| PathSyntaxError { path: path.to_string(), offset, reason };
    let bytes = path.as_bytes();
    let mut segments = Vec::new();
    let mut at = 0;
    // Whether a key may start here without a `.` (the path start only).
    let mut expect_key = true;
    while at < bytes.len() {
        match bytes[at] {
            b'[' => {
                let open = at;
                at += 1;
                match bytes.get(at) {
                    Some(quote @ (b'"' | b'\'')) => {
                        let quote = *quote;
                        at += 1;
                        let mut key = String::new();
                        loop {
                            let Some(&byte) = bytes.get(at) else {
                                return Err(error(open, "unclosed quoted key"));
                            };
                            if byte == b'\\' {
                                let escaped = path[at + 1..].chars().next().ok_or_else(|| error(at, "dangling `\\`"))?;
                                key.push(escaped);
                                at += 1 + escaped.len_utf8();
                            } else if byte == quote {
                                at += 1;
                                break;
                            } else {
                                let ch = path[at..].chars().next().expect("in bounds");
                                key.push(ch);
                                at += ch.len_utf8();
                            }
                        }
                        if bytes.get(at) != Some(&b']') {
                            return Err(error(at, "expected `]` after a quoted key"));
                        }
                        segments.push(Segment::Key(key));
                    }
                    _ => {
                        let start = at;
                        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                            at += 1;
                        }
                        if at == start || bytes.get(at) != Some(&b']') {
                            return Err(error(open, "expected `[N]` or a quoted key `[\"…\"]`"));
                        }
                        let index = path[start..at].parse().map_err(|_| error(start, "index out of range"))?;
                        segments.push(Segment::Index(index));
                    }
                }
                at += 1;
                expect_key = false;
            }
            b'.' => {
                if segments.is_empty() {
                    return Err(error(at, "a path cannot start with `.`"));
                }
                at += 1;
                if at >= bytes.len() || matches!(bytes[at], b'.' | b'[') {
                    return Err(error(at, "expected a key after `.`"));
                }
                expect_key = true;
            }
            _ if expect_key => {
                let start = at;
                while at < bytes.len() && !matches!(bytes[at], b'.' | b'[') {
                    at += 1;
                }
                segments.push(Segment::Key(path[start..at].to_string()));
                expect_key = false;
            }
            _ => return Err(error(at, "expected `.` or `[` between segments")),
        }
    }
    Ok(segments)
}

impl SimplifiedSchema {
    /// The type at `path`: dotted keys (`settings.timeout`), list or tuple
    /// indices (`people[0].name`, `coords[1]`), and quoted keys that contain
    /// a dot (`settings["a.b"]`).
    ///
    /// `Ok(None)` for the empty path, a key the schema does not declare, or a
    /// step the type does not support (a key below a scalar, an index into
    /// an element). Lookup is passive and borrows the schema; it neither
    /// clones the schema nor expands recursive types.
    ///
    /// ## Errors
    ///
    /// [`PathSyntaxError`] when `path` does not parse.
    pub fn query(&self, path: &str) -> Result<Option<TypeDefinition<'_>>, PathSyntaxError> {
        let segments = parse_path(path)?;
        let steps: Vec<PathStep<'_>> = segments
            .iter()
            .map(|segment| match segment {
                Segment::Key(key) => PathStep::Key(key),
                Segment::Index(index) => PathStep::Index(*index),
            })
            .collect();
        Ok(self.query_steps(&steps))
    }

    /// [`Self::query`] over already split steps, with no string parsing.
    pub fn query_steps(&self, steps: &[PathStep<'_>]) -> Option<TypeDefinition<'_>> {
        let PathStep::Key(first) = steps.first()? else {
            return None;
        };
        let typed = self.declaring_layer(first)?;
        let (def, site) = def_at_path(&typed.root, typed.definitions.as_ref(), &Value::Null, steps)?;
        let view = TypeDefinition::new(def, site, &typed.definitions);
        match steps.last() {
            Some(PathStep::Index(index)) => Some(tuple_position(view, *index)),
            _ => Some(view),
        }
    }

    /// The top-level property `key`, without path parsing: `key` is always
    /// one literal key, dots included.
    pub fn property(&self, key: impl AsRef<str>) -> Option<TypeDefinition<'_>> {
        self.query_steps(&[PathStep::Key(key.as_ref())])
    }

    /// Every top-level property, in merged order: the highest layer's in
    /// declaration order, then each lower layer's that no higher layer
    /// declares. A root union contributes each key any inline arm declares.
    pub fn properties(&self) -> Vec<(&str, TypeDefinition<'_>)> {
        let mut seen: Vec<&str> = Vec::new();
        let mut out = Vec::new();
        for layer in &self.definition.layers {
            let names: Vec<&str> = match &layer.typed {
                Some(typed) => root_keys(&typed.root),
                None => layer
                    .lowered
                    .as_ref()
                    .ok()
                    .and_then(|json| json.get("properties").and_then(Value::as_object))
                    .map(|properties| properties.keys().map(String::as_str).collect())
                    .unwrap_or_default(),
            };
            for name in names {
                if seen.contains(&name) {
                    continue;
                }
                seen.push(name);
                if let Some(view) = self.property(name) {
                    out.push((name, view));
                }
            }
        }
        out
    }

    /// The named type `name` from the schema's own `types` table (or, for a
    /// `kind: schema` document without an exported `$schema`, its payload),
    /// as authored: local references are not expanded.
    pub fn named_type(&self, name: &str) -> Option<TypeDefinition<'_>> {
        let layer = self.receiver();
        let super::layer::Authored::Simplified(authored) = &layer.authored else {
            return None;
        };
        let table: &SchemaShape = match (&authored.local_types, authored.schema()) {
            (Some(types), _) => types,
            (None, Some(SchemaRoot::Single(shape))) if authored.envelope == crate::StandaloneSchemaEnvelope::Tagged => shape,
            _ => return None,
        };
        let definitions = layer.typed.as_ref().map_or(empty_definitions(), |typed| typed.definitions.as_ref());
        table
            .properties
            .get(name)
            .map(|def| TypeDefinition::new(Cow::Borrowed(def), AlternativeSite::Value, definitions))
    }

    /// The highest typed layer declaring `key`; `None` when no layer does or
    /// the highest one that does is raw JSON Schema (whose declaration
    /// replaces any typed one beneath it and has no typed form).
    fn declaring_layer(&self, key: &str) -> Option<&Typed> {
        for layer in &self.definition.layers {
            match &layer.typed {
                Some(typed) if declares(typed, key) => return Some(typed),
                Some(_) => {}
                None => {
                    if layer.lowered.as_ref().is_ok_and(|json| json_declares(json, key)) {
                        return None;
                    }
                }
            }
        }
        None
    }
}

/// The keys a root declares, in declaration order.
fn root_keys(root: &SchemaRoot) -> Vec<&str> {
    match root {
        SchemaRoot::Single(shape) => shape.properties.keys().map(String::as_str).collect(),
        SchemaRoot::Union(arms) => {
            let mut keys: Vec<&str> = Vec::new();
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    for key in shape.properties.keys() {
                        if !keys.contains(&key.as_str()) {
                            keys.push(key);
                        }
                    }
                }
            }
            keys
        }
    }
}

/// Position `index` of the tuple an element view (`coords[1]`) stands on:
/// the element's type, or the spread's past the last element. Any other
/// view is returned unchanged.
fn tuple_position(view: TypeDefinition<'_>, index: usize) -> TypeDefinition<'_> {
    if view.site != AlternativeSite::Item {
        return view;
    }
    let position = match &view.def {
        Cow::Borrowed(def) => tuple_of(def).and_then(|tuple| position_of(tuple, index)),
        Cow::Owned(def) => tuple_of(def)
            .and_then(|tuple| position_of(tuple, index))
            .map(|position| Cow::Owned(position.into_owned())),
    };
    match position {
        Some(def) => TypeDefinition::new(def, AlternativeSite::Value, view.definitions),
        None => view,
    }
}

/// The tuple a non-list single-atom declaration is.
fn tuple_of(def: &PropertyDef) -> Option<&TupleType> {
    match def {
        PropertyDef::Single(PropertyAtom { ty: TypeExpr::Tuple(tuple), is_array: false, .. }) => Some(tuple),
        _ => None,
    }
}

/// A tuple position's type: an element, borrowed, or the spread's item type.
fn position_of(tuple: &TupleType, index: usize) -> Option<Cow<'_, PropertyDef>> {
    match tuple.elements.get(index) {
        Some(element) => Some(Cow::Borrowed(&element.def)),
        None => tuple.spread.as_deref().map(|spread| {
            let mut item = spread.clone();
            item.is_array = false;
            item.array_constraints.clear();
            Cow::Owned(PropertyDef::Single(item))
        }),
    }
}

fn empty_definitions() -> &'static DefinitionTable {
    static EMPTY: std::sync::LazyLock<DefinitionTable> = std::sync::LazyLock::new(DefinitionTable::default);
    &EMPTY
}

impl fmt::Display for TypeDefinition<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.type_text())
    }
}
