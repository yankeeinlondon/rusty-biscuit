//! AST types for the SimplifiedSchema authoring grammar.
//!
//! These types model the result of parsing a `$schema` value from a Markdown
//! document's frontmatter. Two cooperating layers feed them:
//!
//! - The **YAML-shape** layer (in `simplified::mod`) inspects the
//!   `serde_yaml_ng::Value` rooted at `$schema` and decides whether the schema
//!   is a single object shape or a root-level union, and whether each property
//!   is a single atom or a property-level union.
//! - The **string** layer (in `simplified::grammar`) parses each individual
//!   type-and-constraint expression into a [`PropertyAtom`].
//!
//! See `darkmatter/features/2026-05-11-schemas/spec.md` for the authoring
//! reference.

use indexmap::IndexMap;

use crate::markdown::span::SourceSpan;

use super::alphanumeric::{AlphanumericOption, AlphanumericOptions};

/// A SimplifiedSchema is either a single object shape or a root-level union of
/// object shapes.
#[derive(Debug, Clone, PartialEq)]
pub enum SimplifiedSchema {
    /// A single object schema body.
    Single(SchemaShape),

    /// A root-level union: the document is valid if its frontmatter satisfies
    /// at least one arm.
    Union(Vec<SchemaArm>),
}

/// A single object schema body — a map of property names to (possibly union)
/// property definitions.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SchemaShape {
    /// Declaration-order map of literal property name to definition. `IndexMap`
    /// preserves insertion order so generated JSON Schemas are deterministic
    /// and diff-friendly.
    pub properties: IndexMap<String, PropertyDef>,

    /// Pattern / dictionary keys (`<string>`, `<starting::…>`, `<ending::…>`,
    /// `<pattern::…>`) in declaration order (Feature C). Kept separate from the
    /// literal [`SchemaShape::properties`] so literal keys retain precedence
    /// when lowered to JSON Schema `patternProperties` (conversion is Phase 3).
    pub pattern_keys: Vec<PatternKeyDef>,

    /// Object-level constraints authored via the reserved `$constraints`
    /// metadata key on a block-form schema object — currently `min-keys` /
    /// `max-keys` object arity (O-C3). The postfix inline form
    /// (`{ … }(min-keys(1))`) carries the equivalent constraints on the
    /// enclosing [`PropertyAtom`] instead; both surfaces desugar to the same
    /// canonical [`Constraint`] variants.
    pub constraints: Vec<Constraint>,
}

impl SchemaShape {
    /// Creates an empty `SchemaShape`.
    pub fn new() -> Self {
        Self::default()
    }
}

/// One pattern (dictionary) key of a schema object paired with its value
/// definition (Feature C — `darkmatter/features/2026-07-08-schema-plus`).
///
/// Declaration order is preserved by storing these in a `Vec` on
/// [`SchemaShape::pattern_keys`].
#[derive(Debug, Clone, PartialEq)]
pub struct PatternKeyDef {
    /// The parsed pattern-key form.
    pub key: PatternKey,
    /// The value definition applied to matching keys.
    pub def: PropertyDef,
}

/// A pattern / dictionary key form (Feature C). Authored inside a schema
/// object with `<...>` syntax; kept verbatim until conversion (Phase 3) lowers
/// each form to JSON Schema `additionalProperties` / `patternProperties`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternKey {
    /// `<string>` — matches any string key. Lowers to `additionalProperties`.
    CatchAll,
    /// `<starting::PREFIX>` — keys beginning with the literal string `PREFIX`.
    /// Desugars to the anchored regex `^PREFIX`.
    Starting(String),
    /// `<ending::SUFFIX>` — keys ending with the literal string `SUFFIX`.
    /// Desugars to the regex `SUFFIX$`.
    Ending(String),
    /// `<pattern::RE>` — a raw ECMA-262 regex escape hatch.
    Pattern(String),
}

impl PatternKey {
    /// Returns `true` if `key` uses the `<...>` pattern-key syntax.
    pub fn is_pattern_key(key: &str) -> bool {
        key.len() >= 2 && key.starts_with('<') && key.ends_with('>')
    }

    /// Parses a `<...>` pattern-key string. `key` must already satisfy
    /// [`PatternKey::is_pattern_key`].
    ///
    /// ## Errors
    ///
    /// Returns `Err` with a human-readable reason when the inner form is not
    /// one of the four recognized shapes (`<string>`, `<starting::…>`,
    /// `<ending::…>`, `<pattern::…>`).
    pub fn parse(key: &str) -> Result<PatternKey, String> {
        let inner = &key[1..key.len() - 1];
        if inner == "string" {
            Ok(PatternKey::CatchAll)
        } else if let Some(prefix) = inner.strip_prefix("starting::") {
            Ok(PatternKey::Starting(prefix.to_string()))
        } else if let Some(suffix) = inner.strip_prefix("ending::") {
            Ok(PatternKey::Ending(suffix.to_string()))
        } else if let Some(re) = inner.strip_prefix("pattern::") {
            Ok(PatternKey::Pattern(re.to_string()))
        } else {
            Err(format!(
                "unknown pattern key `{key}`; expected `<string>`, \
                 `<starting::…>`, `<ending::…>`, or `<pattern::…>`"
            ))
        }
    }
}

/// One arm of a root-level union. File references are resolved by the
/// resolution layer before validator construction.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaArm {
    /// An inline shape declared directly in the YAML.
    Inline(SchemaShape),

    /// An unresolved file reference (resolved later via `biscuit-file`).
    FileRef(String),
}

/// A property definition is either a single atom or a property-level union of
/// atoms.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyDef {
    /// A single typed atom, e.g. `string(required)`.
    Single(PropertyAtom),

    /// A property-level union, e.g. `[string, number]`.
    Union(Vec<PropertyAtom>),
}

impl PropertyDef {
    /// Whether this property is effectively required: some atom marks it
    /// `required` and its whole value does not admit null.
    ///
    /// This is the one presence calculation shared by lowering, trigger
    /// matching, optional-property materialization, completion metadata, and
    /// phase projection. Property-level unions hoist `required` from any arm,
    /// but a union with a `null` or `unknown` arm is never required, whatever
    /// its arms, a named type, or `eager`/`generated` say. Expects a
    /// **resolved** definition: an unresolved reference never admits null.
    pub fn is_required(&self) -> bool {
        let marked = match self {
            Self::Single(atom) => atom.marks_required(),
            Self::Union(atoms) => atoms.iter().any(PropertyAtom::marks_required),
        };
        marked && !self.admits_null()
    }

    /// Whether the property's whole value admits null: a scalar `null` or
    /// `unknown`, or a property-level union with such an arm.
    ///
    /// Arrays never admit null as a whole (`null[]` is a list of nulls), and a
    /// nullable child of an object leaves the object non-nullable.
    pub fn admits_null(&self) -> bool {
        match self {
            Self::Single(atom) => atom.admits_null(),
            Self::Union(atoms) => atoms.iter().any(PropertyAtom::admits_null),
        }
    }

    /// Whether any atom supplies a property default.
    ///
    /// Valid union defaults are identical and collapse to one value during
    /// lowering; conflicting defaults fail schema preparation before a
    /// consumer can query the effective schema.
    pub(crate) fn has_default(&self) -> bool {
        match self {
            Self::Single(atom) => atom.has_default(),
            Self::Union(atoms) => atoms.iter().any(PropertyAtom::has_default),
        }
    }

    /// The property-level alternatives of this definition, however they were
    /// written.
    ///
    /// Returns the arms of a [`PropertyDef::Union`], and the arms of a single
    /// non-array atom whose type is a [`TypeExpr::Union`] (a postfixed union
    /// reference such as `U(required)`). Returns `None` for any other
    /// definition, including an array of a union, whose alternatives describe
    /// its items (see [`PropertyAtom::union_arms`]).
    pub fn alternatives(&self) -> Option<&[PropertyAtom]> {
        match self {
            Self::Union(arms) => Some(arms),
            Self::Single(atom) if !atom.is_array => atom.union_arms(),
            Self::Single(_) => None,
        }
    }
}

/// A type expression is a primitive type keyword, an inline object literal
/// declared with `{ ... }` syntax, a named-type reference (`Name@fileref`
/// or a bare local `Name`), a union of alternatives (a grouped `(A | B)[]`,
/// or after resolution a postfixed union-typed reference), a `tuple`, or a
/// description-only `function`.
///
/// `TypeExpr` is `Clone` but not `Copy` because every variant but
/// [`TypeExpr::Primitive`] carries owned data.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Built-in primitive type keyword (`string`, `number`, `object`, etc.).
    Primitive(SimplifiedType),
    /// Inline object literal: `{ foo: string, bar: number }`.
    InlineObject(SchemaShape),
    /// Named-type reference: `Name@fileref` (Feature B), or a bare
    /// non-keyword `Name`, which is `Name@this` ([`TypeExpr::local`]). Left
    /// of `@` is the named type to inline; right is the file reference
    /// (`this` = the current file). Postfix `[]` and `(constraints)` ride on the enclosing
    /// [`PropertyAtom`] (`is_array` / `constraints`), so this variant only
    /// carries the two parts that survive until resolution. The resolver
    /// (Phase 4) inline-expands imports before conversion; `to_json_schema`
    /// rejects any unresolved import that reaches it.
    Imported {
        /// The named type to inline (left of `@`).
        name: String,
        /// The raw file reference (right of `@`); `"this"` is the current file.
        reference: String,
    },
    /// The ordered, flattened alternatives of a union-typed named type that a
    /// use site referenced with a postfix (`U[]`, `U[](min(1))`,
    /// `U(required)`, `U -> text`).
    ///
    /// Only the resolver produces this variant; the passive parser keeps the
    /// authored [`TypeExpr::Imported`] reference. On an atom with `is_array`
    /// it is the one representation of an array whose items are a union: the
    /// atom's `array_constraints` and `description` belong to the container
    /// and each arm keeps its own constraints and description. Without
    /// `is_array` the atom carries the property's presence/timing markers and
    /// description over the union. Presence and generation are read through
    /// the arms rather than copied onto the atom, so an arm lowers exactly as
    /// it does in a property-level union. A bare reference `U` is still
    /// inlined as a [`PropertyDef::Union`].
    ///
    /// The parser also produces it for a grouped union followed by `[]`:
    /// `(string | number)[]` is this variant with `is_array` set, the same
    /// value a list of a union-typed named type resolves to.
    Union(Vec<PropertyAtom>),
    /// A positional tuple: `tuple([string, number?, ...number[]])`.
    Tuple(TupleType),
    /// A description-only function signature: `function(parameters(...);
    /// returns(...))`. It never validates data; a document schema that
    /// reaches one fails to convert.
    Function(Box<FunctionType>),
    /// A use of a stored named type, by its identity in the resolution's
    /// [`DefinitionTable`](super::DefinitionTable).
    ///
    /// Only the resolver produces this variant: for every use of a cyclic
    /// definition, and for each use of a shared acyclic definition that a
    /// reference can represent (other named types inline). The atom carrying it
    /// holds the use site's presence markers and annotations, inherited from
    /// the definition and merged with the use site's postfix; the shared body
    /// in the table carries none. With `is_array` the atom is a list whose
    /// items are the definition.
    Ref(super::DefKey),
}

/// The positions of a `tuple([...])` type.
///
/// Elements are ordered required first, then optional, then at most one
/// final spread; the parser rejects every other order. An optional position
/// may be omitted but, when supplied, never accepts `null` unless its type
/// does: tuple positions get no optional-null wrapper.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TupleType {
    /// The positional elements, in order.
    pub elements: Vec<TupleElement>,
    /// The final spread (`...number[]`): an array-typed atom (`is_array` is
    /// always set) whose items type every position after the elements.
    pub spread: Option<Box<PropertyAtom>>,
}

impl TupleType {
    /// How many leading positions must be present.
    pub fn required_len(&self) -> usize {
        self.elements.iter().filter(|element| !element.optional).count()
    }

    /// Whether `predicate` holds for some option of some position or for the
    /// spread.
    pub fn any_atom(&self, mut predicate: impl FnMut(&PropertyAtom) -> bool) -> bool {
        self.elements.iter().any(|element| match &element.def {
            PropertyDef::Single(atom) => predicate(atom),
            PropertyDef::Union(arms) => arms.iter().any(&mut predicate),
        }) || self.spread.as_deref().is_some_and(predicate)
    }

    /// Rebuilds the tuple with each position's definition, and the spread
    /// (as a single-atom definition), passed through `map` with its index;
    /// the spread's index is the element count.
    ///
    /// ## Errors
    ///
    /// The first error `map` returns.
    pub fn try_map_defs<E>(
        self,
        mut map: impl FnMut(usize, PropertyDef) -> Result<PropertyDef, E>,
    ) -> Result<Self, E> {
        let count = self.elements.len();
        let elements = self
            .elements
            .into_iter()
            .enumerate()
            .map(|(index, element)| {
                Ok(TupleElement {
                    def: map(index, element.def)?,
                    optional: element.optional,
                })
            })
            .collect::<Result<_, E>>()?;
        let spread = match self.spread {
            None => None,
            Some(atom) => Some(Box::new(match map(count, PropertyDef::Single(*atom))? {
                PropertyDef::Single(atom) => atom,
                PropertyDef::Union(arms) => PropertyAtom {
                    ty: TypeExpr::Union(arms),
                    is_array: true,
                    constraints: Vec::new(),
                    array_constraints: Vec::new(),
                    description: None,
                },
            })),
        };
        Ok(Self { elements, spread })
    }
}

/// One positional element of a [`TupleType`].
#[derive(Debug, Clone, PartialEq)]
pub struct TupleElement {
    /// The element's type; a [`PropertyDef::Union`] for `A | B`.
    pub def: PropertyDef,
    /// Whether the position may be omitted (`T?`).
    pub optional: bool,
}

/// A `function(...)` signature. Its `category(...)` and file-form
/// `example(./file)` constraints ride on the enclosing [`PropertyAtom`] as
/// [`Constraint::Category`] and [`Constraint::Example`]; its `-> description`
/// is the atom's description.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    /// `parameters(...)`.
    pub parameters: FunctionParameters,
    /// `returns(...)`: the value returned on success.
    pub returns: PropertyDef,
    /// `fallible`: the function may return an error for inputs that bound.
    pub fallible: bool,
    /// Inline `example(call => result)` constraints, in authored order.
    pub examples: Vec<CallExample>,
    /// `narrows(...)`: what a `true` (or `false`) result proves about one
    /// argument. `None` is meaningful: the function declares no fact.
    pub narrows: Option<NarrowsClause>,
}

/// What a predicate's boolean result proves about its original argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NarrowsKind {
    /// `true-implies`: a `true` result proves the argument is the target.
    TrueImplies,
    /// `true-iff`: `true` proves the target and `false` proves its absence.
    TrueIff,
    /// `false-implies`: a `false` result proves the argument is the target.
    FalseImplies,
}

impl NarrowsKind {
    /// The authored keyword.
    pub fn keyword(self) -> &'static str {
        match self {
            Self::TrueImplies => "true-implies",
            Self::TrueIff => "true-iff",
            Self::FalseImplies => "false-implies",
        }
    }

    /// The kind spelled by `keyword`, if it is one of the three.
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        match keyword {
            "true-implies" => Some(Self::TrueImplies),
            "true-iff" => Some(Self::TrueIff),
            "false-implies" => Some(Self::FalseImplies),
            _ => None,
        }
    }
}

/// A `narrows(name: T; kind)` or presence-only `narrows(name; kind)` clause.
///
/// Equality ignores the spans: they record where the clause was authored,
/// not what it says.
#[derive(Debug, Clone)]
pub struct NarrowsClause {
    /// The narrowed parameter's name.
    pub parameter: String,
    /// Byte span of [`NarrowsClause::parameter`] in the type-expression string.
    pub parameter_span: SourceSpan,
    /// The target type; `None` for a presence-only clause (non-null).
    pub target: Option<PropertyAtom>,
    /// Byte span of the target, or of the parameter when there is none.
    pub target_span: SourceSpan,
    /// The implication.
    pub kind: NarrowsKind,
    /// Byte span of the implication keyword.
    pub kind_span: SourceSpan,
    /// Byte span of the whole clause, keyword through closing `)`.
    pub clause_span: SourceSpan,
}

impl PartialEq for NarrowsClause {
    fn eq(&self, other: &Self) -> bool {
        self.parameter == other.parameter && self.target == other.target && self.kind == other.kind
    }
}

/// Whether `returns` is exactly the single primitive `boolean`: the only
/// return type a `narrows` clause may accompany. `boolean | null`, `unknown`,
/// `boolish`, unions, and arrays do not qualify.
pub(crate) fn returns_exact_boolean(returns: &PropertyDef) -> bool {
    matches!(
        returns,
        PropertyDef::Single(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Boolean),
            is_array: false,
            ..
        })
    )
}

/// Why `target` cannot be a narrowing target, if it cannot: a named type
/// (reference) is not self-contained.
pub(crate) fn narrows_target_problem(target: &PropertyAtom) -> Option<&'static str> {
    match &target.ty {
        TypeExpr::Imported { .. } => {
            Some("a named type cannot be a narrowing target; write a self-contained type")
        }
        _ => None,
    }
}

/// The `parameters(...)` of a [`FunctionType`].
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionParameters {
    /// `parameters(void)`: the function takes no arguments.
    Void,
    /// Named parameters, ordered required, then optional, then at most one
    /// final rest parameter.
    List(Vec<FunctionParameter>),
}

impl FunctionParameters {
    /// The parameters, empty for [`FunctionParameters::Void`].
    pub fn as_slice(&self) -> &[FunctionParameter] {
        match self {
            Self::Void => &[],
            Self::List(parameters) => parameters,
        }
    }
}

/// One named parameter of a [`FunctionType`].
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParameter {
    /// The parameter's name; documentation only, never part of a binding
    /// contract.
    pub name: String,
    /// The parameter's type. For a rest parameter it types each remaining
    /// argument.
    pub def: PropertyDef,
    /// `name?: T`: the argument may be omitted (but not passed `null` unless
    /// `T` admits it).
    pub optional: bool,
    /// `...name: T`: zero or more trailing arguments of `T`.
    pub rest: bool,
}

/// An inline `example(CALL => RESULT)` on a function type.
///
/// Equality ignores the spans: they record where the example was authored,
/// not what it says.
#[derive(Debug, Clone)]
pub struct CallExample {
    /// The authored call, trimmed; it parses as a Darkmatter function-call
    /// expression.
    pub call: String,
    /// Byte span of [`CallExample::call`] in the type-expression string.
    pub call_span: SourceSpan,
    /// The expected outcome.
    pub expected: ExampleOutcome,
    /// Byte span of the authored result in the type-expression string.
    pub expected_span: SourceSpan,
    /// `display-only: REASON`: shown in documentation, never executed.
    pub display_only: Option<String>,
}

impl PartialEq for CallExample {
    fn eq(&self, other: &Self) -> bool {
        self.call == other.call
            && self.expected == other.expected
            && self.display_only == other.display_only
    }
}

/// What an inline function example expects its call to produce.
#[derive(Debug, Clone, PartialEq)]
pub enum ExampleOutcome {
    /// A literal value, compared structurally.
    Value(serde_json::Value),
    /// `=> error`: the call must fail.
    Error,
}

/// The [`TypeExpr::Imported`] reference that names the current schema's
/// own named types, written `Name@this` or as a bare `Name`.
pub const LOCAL_REFERENCE: &str = "this";

impl TypeExpr {
    /// A reference to the named type `name` in the schema that writes it.
    ///
    /// A bare `Name` and `Name@this` parse to this same value: they resolve
    /// through one namespace, and serialization chooses the spelling (see
    /// [`TypeExpr::is_local_reference`]).
    pub fn local(name: impl Into<String>) -> Self {
        Self::Imported {
            name: name.into(),
            reference: LOCAL_REFERENCE.to_string(),
        }
    }

    /// Whether this is a reference to a named type of the writing schema,
    /// in either spelling. A local reference never adds a file dependency.
    pub fn is_local_reference(&self) -> bool {
        matches!(self, Self::Imported { reference, .. } if reference == LOCAL_REFERENCE)
    }
}

/// One arm of a property-level union (or the body of a non-union property).
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyAtom {
    /// The type expression for this atom (primitive, inline object, or import).
    pub ty: TypeExpr,

    /// Whether the type was suffixed with `[]`.
    pub is_array: bool,

    /// Constraints inside the parens that precede `[]` (or the only paren list
    /// when `is_array` is `false`). These bind to *items* for arrays and to
    /// the value otherwise.
    pub constraints: Vec<Constraint>,

    /// Constraints in the parens that follow `[]`. Empty for non-array atoms.
    pub array_constraints: Vec<Constraint>,

    /// Optional human-readable description supplied via `-> {description}`.
    pub description: Option<String>,
}

impl PropertyAtom {
    /// Convenience constructor for a bare-typed atom with no constraints.
    pub fn bare(ty: SimplifiedType) -> Self {
        Self {
            ty: TypeExpr::Primitive(ty),
            is_array: false,
            constraints: Vec::new(),
            array_constraints: Vec::new(),
            description: None,
        }
    }

    /// Convenience constructor for a bare inline-object atom.
    pub fn bare_inline_object(shape: SchemaShape) -> Self {
        Self {
            ty: TypeExpr::InlineObject(shape),
            is_array: false,
            constraints: Vec::new(),
            array_constraints: Vec::new(),
            description: None,
        }
    }

    /// Iterates constraints that govern the containing property's presence
    /// and annotations, including constraints written after an array suffix.
    pub(crate) fn property_constraints(&self) -> impl Iterator<Item = &Constraint> {
        self.constraints.iter().chain(self.array_constraints.iter())
    }

    /// Whether this atom, standing alone as a property, is effectively
    /// required: it carries a `required` marker and its whole value does not
    /// admit null (see [`PropertyDef::is_required`]).
    pub fn is_required(&self) -> bool {
        self.marks_required() && !self.admits_null()
    }

    /// Whether this atom carries a `required` marker, written, inherited, or
    /// hoisted from a union type's arms, regardless of null admission.
    pub(crate) fn marks_required(&self) -> bool {
        self.property_constraints()
            .any(|constraint| matches!(constraint, Constraint::Required))
            || self
                .union_arms()
                .is_some_and(|arms| arms.iter().any(PropertyAtom::marks_required))
    }

    /// Whether this atom's whole value admits null: a non-array `null` or
    /// `unknown`, or a non-array union type with such an arm.
    pub fn admits_null(&self) -> bool {
        if self.is_array {
            return false;
        }
        match &self.ty {
            TypeExpr::Primitive(SimplifiedType::Null | SimplifiedType::Unknown) => true,
            TypeExpr::Union(arms) => arms.iter().any(PropertyAtom::admits_null),
            _ => false,
        }
    }

    /// Whether this atom marks its containing property as host-generated.
    /// Like [`PropertyAtom::is_required`], a union type's arms count.
    pub(crate) fn is_generated(&self) -> bool {
        self.property_constraints()
            .any(|constraint| matches!(constraint, Constraint::Generated))
            || self
                .union_arms()
                .is_some_and(|arms| arms.iter().any(PropertyAtom::is_generated))
    }

    /// Whether this atom supplies a default for its containing property.
    /// Like [`PropertyAtom::is_required`], a union type's arms count.
    pub(crate) fn has_default(&self) -> bool {
        self.property_constraints()
            .any(|constraint| matches!(constraint, Constraint::Default(_)))
            || self
                .union_arms()
                .is_some_and(|arms| arms.iter().any(PropertyAtom::has_default))
    }

    /// The alternatives of a [`TypeExpr::Union`] type: the item alternatives
    /// when `is_array` is set, the property alternatives otherwise.
    pub fn union_arms(&self) -> Option<&[PropertyAtom]> {
        match &self.ty {
            TypeExpr::Union(arms) => Some(arms),
            _ => None,
        }
    }

    /// The canonical type options of an `alphanumeric` atom; `None` for every
    /// other type.
    pub fn alphanumeric_options(&self) -> Option<AlphanumericOptions> {
        if !matches!(self.ty, TypeExpr::Primitive(SimplifiedType::Alphanumeric)) {
            return None;
        }
        Some(
            self.constraints
                .iter()
                .filter_map(|constraint| match constraint {
                    Constraint::Options(options) => Some(options.iter().copied()),
                    _ => None,
                })
                .flatten()
                .collect::<AlphanumericOptions>()
                .canonical(),
        )
    }

    /// The scalar const value this atom pins when it is a `literal(x)` type,
    /// with its lexed JSON type preserved (string / number / boolean).
    ///
    /// Returns `None` for every non-Literal type, and for a malformed Literal
    /// atom that somehow lacks its required [`Constraint::LiteralValue`]. This is
    /// the Literal analog of reading [`Constraint::Members`] off an `enum` atom:
    /// it never erases the scalar type (a string `'2'` stays a JSON string, a
    /// bare `2` stays a JSON number), so consumers can serialize the value back
    /// to correctly-typed YAML.
    pub fn literal_value(&self) -> Option<&serde_json::Value> {
        if !matches!(self.ty, TypeExpr::Primitive(SimplifiedType::Literal)) {
            return None;
        }
        self.constraints.iter().find_map(|constraint| match constraint {
            Constraint::LiteralValue(value) => Some(value),
            _ => None,
        })
    }
}

/// The full type vocabulary of the SimplifiedSchema grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimplifiedType {
    /// Any YAML string scalar.
    String,
    /// ISO-8601 date (`YYYY-MM-DD`).
    Date,
    /// ISO-8601 datetime.
    DateTime,
    /// Time-of-day with optional timezone.
    Time,
    /// Any JSON number.
    Number,
    /// JSON number or numeric-looking string.
    NumberLike,
    /// Any JSON boolean.
    Boolean,
    /// JSON boolean or `"true"` / `"false"` string forms.
    Boolish,
    /// Any YAML/JSON object (opaque in v1).
    Object,
    /// File reference resolved via `biscuit-file::FileReference`.
    File,
    /// Enumeration; constraint members are required.
    Enum,
    /// Absolute URL.
    Url,
    /// Email address (RFC 5322 addr-spec).
    Email,
    /// A string whose content parses as valid YAML (accepts JSON too).
    Yaml,
    /// A string whose content parses as strict JSON.
    Json,
    /// A property whose value must equal exactly one scalar. The value is
    /// carried by the required [`Constraint::LiteralValue`] (keeps this enum
    /// `Copy`), lexed with its scalar type: bare `true`/`false` → boolean,
    /// bare numberlike token → number, everything else / quoted → string.
    /// Compiles to JSON Schema `const`; the single-member-enum replacement.
    Literal,
    /// A string that must parse under the Darkmatter expression grammar
    /// (parse-only, never evaluated). The third content-format string type
    /// alongside [`SimplifiedType::Yaml`] and [`SimplifiedType::Json`].
    Expression,
    /// One complete SimplifiedSchema property definition. Its YAML carrier may
    /// be a string type expression, mapping object definition, or non-empty
    /// property union sequence; semantic validation is parse-only.
    TypeDefinition,
    /// One complete `$schema` declaration. Its YAML carrier may be a local
    /// reference string, inline mapping, or non-empty root union sequence;
    /// semantic validation is parse-only.
    Schema,
    /// Every value, including `null`, and omission. `any` is an equivalent
    /// spelling; `unknown` is canonical.
    Unknown,
    /// Only an explicit `null` value (or omission).
    Null,
    /// A string of ASCII letters and digits, widened or narrowed by the
    /// [`AlphanumericOption`]s carried in a positional
    /// [`Constraint::Options`] (keeps this enum `Copy`).
    Alphanumeric,
    /// A string holding a shell command: at least one non-whitespace
    /// character, never parsed, approved, or executed.
    Shell,
}

impl SimplifiedType {
    /// Returns the canonical lowercase keyword used in the grammar.
    pub fn as_keyword(self) -> &'static str {
        match self {
            SimplifiedType::String => "string",
            SimplifiedType::Date => "date",
            SimplifiedType::DateTime => "datetime",
            SimplifiedType::Time => "time",
            SimplifiedType::Number => "number",
            SimplifiedType::NumberLike => "numberlike",
            SimplifiedType::Boolean => "boolean",
            SimplifiedType::Boolish => "boolish",
            SimplifiedType::Object => "object",
            SimplifiedType::File => "file",
            SimplifiedType::Enum => "enum",
            SimplifiedType::Url => "url",
            SimplifiedType::Email => "email",
            SimplifiedType::Yaml => "yaml",
            SimplifiedType::Json => "json",
            SimplifiedType::Literal => "literal",
            SimplifiedType::Expression => "expression",
            SimplifiedType::TypeDefinition => "type-definition",
            SimplifiedType::Schema => "schema",
            SimplifiedType::Unknown => "unknown",
            SimplifiedType::Null => "null",
            SimplifiedType::Alphanumeric => "alphanumeric",
            SimplifiedType::Shell => "shell",
        }
    }

    /// Alternative spellings [`SimplifiedType::from_keyword`] accepts for this
    /// type besides its canonical [`SimplifiedType::as_keyword`].
    pub fn aliases(self) -> &'static [&'static str] {
        match self {
            SimplifiedType::Unknown => &["any"],
            _ => &[],
        }
    }

    /// Whether `name` is a built-in type keyword, including the structural
    /// `tuple` and `function`, which have no [`SimplifiedType`] of their own.
    /// Built-ins win over a local named type of the same name, so such a type
    /// is reachable only as `name@this`.
    pub fn is_keyword(name: &str) -> bool {
        Self::from_keyword(name).is_some() || matches!(name, "tuple" | "function")
    }

    /// Parses a type keyword. Returns `None` if `keyword` is not a known type.
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        Some(match keyword {
            "string" => SimplifiedType::String,
            "date" => SimplifiedType::Date,
            "datetime" => SimplifiedType::DateTime,
            "time" => SimplifiedType::Time,
            "number" => SimplifiedType::Number,
            "numberlike" => SimplifiedType::NumberLike,
            "boolean" => SimplifiedType::Boolean,
            "boolish" => SimplifiedType::Boolish,
            "object" => SimplifiedType::Object,
            "file" => SimplifiedType::File,
            "enum" => SimplifiedType::Enum,
            "url" => SimplifiedType::Url,
            "email" => SimplifiedType::Email,
            "yaml" => SimplifiedType::Yaml,
            "json" => SimplifiedType::Json,
            "literal" => SimplifiedType::Literal,
            "expression" => SimplifiedType::Expression,
            "type-definition" => SimplifiedType::TypeDefinition,
            "schema" => SimplifiedType::Schema,
            "unknown" | "any" => SimplifiedType::Unknown,
            "null" => SimplifiedType::Null,
            "alphanumeric" => SimplifiedType::Alphanumeric,
            "shell" => SimplifiedType::Shell,
            _ => return None,
        })
    }
}

/// One constraint applied to a [`PropertyAtom`].
///
/// Every constraint variant maps to either a JSON Schema keyword or an
/// `x-darkmatter-*` extension once converted. See `convert.rs` (Phase 2) for
/// the mapping.
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    // ── universal ────────────────────────────────────────────────────────
    /// The property must be present.
    Required,

    /// Default value (emitted as JSON Schema `default`).
    Default(serde_json::Value),

    /// The value is supplied by the host runtime (e.g. Darkmatter context
    /// capture) rather than authored in static frontmatter. Static
    /// authored-document validation skips `required` enforcement for
    /// `generated` properties; runtime/effective validation still type-checks
    /// host-supplied values. Orthogonal to `Required`, which controls
    /// type/nullability.
    Generated,

    /// A value here that is a `{{ … }}` template (as
    /// [`crate::markdown::compose::expression::is_deferred_template`]
    /// classifies it) is supplied later, when the template is rendered, so
    /// schema validation accepts it without judging it; every literal value
    /// here is judged as usual. Marks the node the atom lowers to: the item
    /// when written before `[]`, the list when written after it. Lowers to
    /// `x-darkmatter-deferred: true`.
    Deferred,

    // ── numeric ──────────────────────────────────────────────────────────
    /// Inclusive minimum.
    Min(f64),

    /// Inclusive maximum.
    Max(f64),

    /// Restrict to integer values.
    Integer,

    // ── string ───────────────────────────────────────────────────────────
    /// Minimum length in Unicode code points.
    MinLen(usize),

    /// Maximum length in Unicode code points.
    MaxLen(usize),

    /// Disallow empty/whitespace-only strings.
    NotEmpty,

    /// ECMA-262 regex the value must match.
    Pattern(String),

    /// Ordered, non-validating completion candidates authored with
    /// `suggest(...)`.
    ///
    /// Available only on exact `string` and `number` types (including their
    /// array forms). At most one `suggest(...)` per complete property
    /// definition (including across property-union atoms). Candidates are
    /// advisory — they never restrict what document values are valid.
    ///
    /// See [`SuggestionCandidate`] for the per-candidate data model.
    Suggest(Vec<SuggestionCandidate>),

    // ── enum ─────────────────────────────────────────────────────────────
    /// Enumerated members.
    Members(Vec<String>),

    // ── literal ──────────────────────────────────────────────────────────
    /// The single scalar value a `literal(...)` property must equal, carried
    /// with its lexed JSON type (string / number / boolean). Positional like
    /// [`Constraint::Members`]; `keyword()` → `"<value>"`. Compiles to JSON
    /// Schema `const` (Phase 3).
    LiteralValue(serde_json::Value),

    // ── alphanumeric ─────────────────────────────────────────────────────
    /// The `alphanumeric` type options written before the first `;`, in
    /// authored order and possibly redundant. Positional like
    /// [`Constraint::Members`]; [`Constraint::keyword`] is the first authored
    /// option, which is how the option list is located in source. Identity
    /// is [`AlphanumericOptions::canonical`] of the set
    /// ([`PropertyAtom::alphanumeric_options`]). On a named-type reference
    /// the list is kept until resolution compares it with the target's.
    Options(Vec<AlphanumericOption>),

    // ── file ─────────────────────────────────────────────────────────────
    /// Glob patterns the resolved path must match. Patterns starting with `!`
    /// exclude.
    Match(Vec<String>),

    /// Require the containing property at stabilized launch and completion.
    /// On `file` values this also requires each present reference to exist.
    Eager,

    // ── url ──────────────────────────────────────────────────────────────
    /// Allowed URL schemes (lowercased).
    Scheme(Vec<String>),

    // ── array ────────────────────────────────────────────────────────────
    /// Items must be distinct.
    Unique,

    /// Minimum number of items in an array. Only legal in `array_constraints`.
    MinItems(usize),

    /// Maximum number of items in an array. Only legal in `array_constraints`.
    MaxItems(usize),

    // ── object arity ───────────────────────────────────────────────────────
    /// Minimum number of object properties (`min-keys`). Object-analog of
    /// [`Constraint::MinItems`]; lowers to JSON Schema `minProperties`
    /// (Feature C / O-C3).
    MinKeys(usize),

    /// Maximum number of object properties (`max-keys`). Object-analog of
    /// [`Constraint::MaxItems`]; lowers to JSON Schema `maxProperties`.
    MaxKeys(usize),

    /// `uncounted-keys(when, no_error)`: keys that `min-keys` and `max-keys`
    /// do not count, so `{ a, b, when }` with `max-keys(1)` admits one key
    /// besides `when`. Needs `min-keys` or `max-keys` on the same object and
    /// names at most [`MAX_UNCOUNTED_KEYS`] keys. Lowers to one `if`/`then`
    /// pair per combination of the named keys present, each bounding
    /// `minProperties`/`maxProperties` by that many more.
    UncountedKeys(Vec<String>),

    // ── documentation ──────────────────────────────────────────────────────
    /// One or more example artifact file references (`example(...)`, Feature
    /// A). Non-validating for the annotated property — each referenced file is
    /// itself validated at schema-load time. Raw authored reference strings are
    /// preserved here; resolution is deferred to `resolve.rs` (Phase 6).
    Example(Vec<String>),

    /// `category(Name)`: documentation metadata available on every type. The
    /// name is the trimmed raw text and is case-sensitive. On a data type it
    /// lowers to the `x-darkmatter-category` annotation; a schema file that
    /// declares `categories:` rejects a name it does not list.
    Category(String),

    // ── lifecycle ──────────────────────────────────────────────────────────
    /// `operands(<verb>-short)` or `operands(<verb>-long)`: the value is the
    /// short or long form of the Darkmatter lifecycle verb `<verb>`, so
    /// schema validation binds its operands through that verb's runtime
    /// registration and the binder's verdict replaces the type's own for
    /// them. The lifecycle schema generator writes it on every generated verb
    /// arm; a verb the parser owns itself (`say`, `proxy`, `set`, …) carries
    /// it on its hand-written type, where it only checks the strings the
    /// parser reads as templates. Lowers to the `x-darkmatter-operands`
    /// annotation, which never fails a value by itself.
    Operands(String),
}

impl Constraint {
    /// Returns the keyword name as it appears in the SimplifiedSchema grammar.
    pub fn keyword(&self) -> &'static str {
        match self {
            Constraint::Required => "required",
            Constraint::Default(_) => "default",
            Constraint::Generated => "generated",
            Constraint::Deferred => "deferred",
            Constraint::Min(_) => "min",
            Constraint::Max(_) => "max",
            Constraint::Integer => "integer",
            Constraint::MinLen(_) => "min",
            Constraint::MaxLen(_) => "max",
            Constraint::NotEmpty => "not-empty",
            Constraint::Pattern(_) => "pattern",
            Constraint::Suggest(_) => "suggest",
            Constraint::Members(_) => "<members>",
            Constraint::LiteralValue(_) => "<value>",
            Constraint::Options(options) => options
                .first()
                .map_or("<options>", |option| option.as_keyword()),
            Constraint::Match(_) => "match",
            Constraint::Eager => "eager",
            Constraint::Scheme(_) => "scheme",
            Constraint::Unique => "unique",
            Constraint::MinItems(_) => "min",
            Constraint::MaxItems(_) => "max",
            Constraint::MinKeys(_) => "min-keys",
            Constraint::MaxKeys(_) => "max-keys",
            Constraint::UncountedKeys(_) => "uncounted-keys",
            Constraint::Example(_) => "example",
            Constraint::Category(_) => "category",
            Constraint::Operands(_) => "operands",
        }
    }
}

/// The most keys one [`Constraint::UncountedKeys`] may name: its lowering
/// holds one condition per combination of them.
pub const MAX_UNCOUNTED_KEYS: usize = 8;

/// Checks the key list of a [`Constraint::UncountedKeys`]: one to
/// [`MAX_UNCOUNTED_KEYS`] distinct, nonempty names. The error is the message
/// both authoring surfaces report.
pub(crate) fn check_uncounted_keys(keys: &[String]) -> Result<(), String> {
    if keys.is_empty() {
        return Err("`uncounted-keys` requires at least one key".to_string());
    }
    if keys.len() > MAX_UNCOUNTED_KEYS {
        return Err(format!(
            "`uncounted-keys` names at most {MAX_UNCOUNTED_KEYS} keys, got {}",
            keys.len()
        ));
    }
    if keys.iter().any(|key| key.is_empty()) {
        return Err("`uncounted-keys` cannot name an empty key".to_string());
    }
    if let Some(repeated) = keys.iter().enumerate().find_map(|(index, key)| keys[..index].contains(key).then_some(key)) {
        return Err(format!("`uncounted-keys` names `{repeated}` twice"));
    }
    Ok(())
}

/// Which spelling of a lifecycle verb an [`Constraint::Operands`] arm types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperandForm {
    /// `null`, a list of positional operands, or one scalar operand.
    Short,
    /// A mapping of named operands.
    Long,
}

/// The verb and form an [`Constraint::Operands`] name (`number-short`)
/// spells; `None` when it is not `<verb>-short` or `<verb>-long` with a
/// nonempty verb made of letters, digits, and underscores.
pub fn operands_verb(name: &str) -> Option<(&str, OperandForm)> {
    let (verb, form) = if let Some(verb) = name.strip_suffix("-short") {
        (verb, OperandForm::Short)
    } else {
        (name.strip_suffix("-long")?, OperandForm::Long)
    };
    let valid = !verb.is_empty() && verb.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    valid.then_some((verb, form))
}

/// One interpreted argument from a `suggest(...)` constraint.
///
/// Each candidate retains decoded text, a target-directed interpreted JSON
/// value, optional canonical decimal text for numeric candidates, and the
/// exact authored argument byte span.
///
/// `span` is a byte range into the source handed to the relevant parser. The
/// string grammar initially records a range into its type-expression string;
/// source-aware YAML parsing projects it into the complete YAML or Markdown
/// document without normalizing line endings.
#[derive(Debug, Clone, PartialEq)]
pub struct SuggestionCandidate {
    /// Argument text after SimplifiedSchema quote and escape processing.
    pub decoded: String,
    /// Target-directed value retained as suggestion metadata.
    ///
    /// For `string(suggest(...))`, every candidate is a JSON string. For
    /// `number(suggest(...))`, a losslessly representable candidate is a JSON
    /// number; a non-representable one is retained as its canonical decimal
    /// string so the invalid metadata stays available for linting.
    pub interpreted: serde_json::Value,
    /// Normalized simple-decimal spelling for numeric candidates.
    ///
    /// Populated when the decoded text uses simple-decimal syntax — even when
    /// the value is not losslessly representable by the supported JSON number
    /// model. `None` for string candidates.
    pub canonical_decimal: Option<String>,
    /// Exact authored argument byte span, including argument quotes.
    pub span: SourceSpan,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplified_type_keyword_round_trip() {
        for ty in [
            SimplifiedType::String,
            SimplifiedType::Date,
            SimplifiedType::DateTime,
            SimplifiedType::Time,
            SimplifiedType::Number,
            SimplifiedType::NumberLike,
            SimplifiedType::Boolean,
            SimplifiedType::Boolish,
            SimplifiedType::Object,
            SimplifiedType::File,
            SimplifiedType::Enum,
            SimplifiedType::Url,
            SimplifiedType::Email,
            SimplifiedType::Yaml,
            SimplifiedType::Json,
            SimplifiedType::Literal,
            SimplifiedType::Expression,
            SimplifiedType::TypeDefinition,
            SimplifiedType::Schema,
            SimplifiedType::Unknown,
            SimplifiedType::Null,
            SimplifiedType::Alphanumeric,
            SimplifiedType::Shell,
        ] {
            let kw = ty.as_keyword();
            assert_eq!(SimplifiedType::from_keyword(kw), Some(ty));
        }
    }

    #[test]
    fn any_is_an_alias_of_canonical_unknown() {
        assert_eq!(SimplifiedType::from_keyword("any"), Some(SimplifiedType::Unknown));
        assert_eq!(SimplifiedType::Unknown.as_keyword(), "unknown");
        assert_eq!(SimplifiedType::Unknown.aliases(), ["any"]);
        assert!(SimplifiedType::String.aliases().is_empty());
        for keyword in ["unknown", "any", "null", "alphanumeric", "shell"] {
            assert!(SimplifiedType::is_keyword(keyword), "{keyword}");
        }
    }

    #[test]
    fn alphanumeric_options_are_canonical_and_only_on_alphanumeric() {
        let mut atom = PropertyAtom::bare(SimplifiedType::Alphanumeric);
        assert_eq!(atom.alphanumeric_options(), Some(AlphanumericOptions::base()));
        atom.constraints.push(Constraint::Options(vec![
            AlphanumericOption::BlockLeadingDash,
            AlphanumericOption::AllowPeriods,
        ]));
        assert_eq!(
            atom.alphanumeric_options(),
            Some(AlphanumericOptions::base().with(AlphanumericOption::AllowPeriods))
        );
        assert_eq!(atom.constraints[0].keyword(), "block-leading-dash");
        assert_eq!(PropertyAtom::bare(SimplifiedType::String).alphanumeric_options(), None);
    }

    #[test]
    fn simplified_type_from_keyword_unknown() {
        assert_eq!(SimplifiedType::from_keyword("not-a-type"), None);
        // Case-sensitive: only lowercase keywords are accepted.
        assert_eq!(SimplifiedType::from_keyword("String"), None);
    }

    #[test]
    fn property_atom_bare_defaults_are_empty() {
        let atom = PropertyAtom::bare(SimplifiedType::String);
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::String));
        assert!(!atom.is_array);
        assert!(atom.constraints.is_empty());
        assert!(atom.array_constraints.is_empty());
        assert!(atom.description.is_none());
    }

    #[test]
    fn property_atom_bare_inline_object_defaults_are_empty() {
        let atom = PropertyAtom::bare_inline_object(SchemaShape::new());
        assert_eq!(atom.ty, TypeExpr::InlineObject(SchemaShape::new()));
        assert!(!atom.is_array);
        assert!(atom.constraints.is_empty());
        assert!(atom.array_constraints.is_empty());
        assert!(atom.description.is_none());
    }

    fn required(ty: SimplifiedType) -> PropertyAtom {
        let mut atom = PropertyAtom::bare(ty);
        atom.constraints.push(Constraint::Required);
        atom
    }

    #[test]
    fn null_admitting_whole_values_are_never_required() {
        for ty in [SimplifiedType::Null, SimplifiedType::Unknown] {
            let def = PropertyDef::Single(required(ty));
            assert!(def.admits_null() && !def.is_required(), "{ty:?}(required)");
        }
        let nullable_union = PropertyDef::Union(vec![
            required(SimplifiedType::String),
            PropertyAtom::bare(SimplifiedType::Null),
        ]);
        assert!(nullable_union.admits_null() && !nullable_union.is_required());
        let mut postfixed = PropertyAtom::bare(SimplifiedType::Object);
        postfixed.ty = TypeExpr::Union(vec![
            required(SimplifiedType::String),
            PropertyAtom::bare(SimplifiedType::Unknown),
        ]);
        postfixed.constraints.push(Constraint::Required);
        assert!(!PropertyDef::Single(postfixed).is_required(), "postfixed union with `unknown`");

        // Item nullability and child nullability leave the container required.
        for ty in [SimplifiedType::Null, SimplifiedType::Unknown] {
            let mut list = PropertyAtom::bare(ty);
            list.is_array = true;
            list.array_constraints.push(Constraint::Required);
            assert!(PropertyDef::Single(list).is_required(), "{ty:?}[](required)");
        }
        let mut shape = SchemaShape::new();
        shape
            .properties
            .insert("child".into(), PropertyDef::Single(required(SimplifiedType::Null)));
        let mut parent = PropertyAtom::bare_inline_object(shape);
        parent.constraints.push(Constraint::Required);
        assert!(PropertyDef::Single(parent).is_required());

        // A concrete union and a concrete scalar can still become required.
        assert!(
            PropertyDef::Union(vec![
                required(SimplifiedType::String),
                PropertyAtom::bare(SimplifiedType::Number),
            ])
            .is_required()
        );
        assert!(PropertyDef::Single(required(SimplifiedType::String)).is_required());
    }

    #[test]
    fn property_semantics_include_value_and_array_constraints() {
        let mut value_required = PropertyAtom::bare(SimplifiedType::String);
        value_required.constraints.push(Constraint::Required);
        let mut array_default = PropertyAtom::bare(SimplifiedType::String);
        array_default.is_array = true;
        array_default
            .array_constraints
            .push(Constraint::Default(serde_json::json!([])));

        assert!(PropertyDef::Single(value_required.clone()).is_required());
        assert!(!PropertyDef::Single(value_required).has_default());
        assert!(!PropertyDef::Single(array_default.clone()).is_required());
        assert!(PropertyDef::Single(array_default.clone()).has_default());
        assert!(PropertyDef::Union(vec![
            PropertyAtom::bare(SimplifiedType::Number),
            array_default,
        ])
        .has_default());
    }

    #[test]
    fn schema_shape_default_is_empty() {
        let shape = SchemaShape::new();
        assert!(shape.properties.is_empty());
    }

    #[test]
    fn generated_keyword_is_canonical() {
        assert_eq!(Constraint::Generated.keyword(), "generated");
    }

    #[test]
    fn literal_value_preserves_scalar_type() {
        // A string literal stays a JSON string, even a numberlike one.
        let mut atom = PropertyAtom::bare(SimplifiedType::Literal);
        atom.constraints
            .push(Constraint::LiteralValue(serde_json::Value::from("2")));
        assert_eq!(atom.literal_value(), Some(&serde_json::Value::from("2")));

        // A numeric literal stays a JSON number, a boolean a JSON boolean.
        let mut number = PropertyAtom::bare(SimplifiedType::Literal);
        number
            .constraints
            .push(Constraint::LiteralValue(serde_json::Value::from(2)));
        assert_eq!(number.literal_value(), Some(&serde_json::Value::from(2)));

        // A non-Literal atom never reports a literal value.
        assert!(PropertyAtom::bare(SimplifiedType::String).literal_value().is_none());
        // A Literal atom missing its required value is defensively `None`.
        assert!(PropertyAtom::bare(SimplifiedType::Literal).literal_value().is_none());
    }
}
