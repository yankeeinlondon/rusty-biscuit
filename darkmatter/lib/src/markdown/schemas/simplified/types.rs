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
    /// Whether any atom marks this property as required.
    ///
    /// Property-level unions hoist `required` from any arm, matching JSON
    /// Schema lowering. Both value constraints and post-array constraints are
    /// considered by [`PropertyAtom::is_required`].
    pub(crate) fn is_required(&self) -> bool {
        match self {
            Self::Single(atom) => atom.is_required(),
            Self::Union(atoms) => atoms.iter().any(PropertyAtom::is_required),
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
}

/// A type expression is a primitive type keyword, an inline object literal
/// declared with `{ ... }` syntax, or a cross-file named-type import
/// (`Name@fileref`).
///
/// `TypeExpr` is `Clone` but not `Copy` because the inline-object and import
/// arms carry owned data. Call sites that previously matched on
/// `atom.ty: SimplifiedType` now match on `atom.ty: TypeExpr` and handle the
/// `Primitive` / `InlineObject` / `Imported` arms.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Built-in primitive type keyword (`string`, `number`, `object`, etc.).
    Primitive(SimplifiedType),
    /// Inline object literal: `{ foo: string, bar: number }`.
    InlineObject(SchemaShape),
    /// Cross-file named-type import: `Name@fileref` (Feature B). Left of `@`
    /// is the named type to inline; right is the file reference (`this` = the
    /// current file). Postfix `[]` and `(constraints)` ride on the enclosing
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

    /// Whether this atom marks its containing property as required.
    pub(crate) fn is_required(&self) -> bool {
        self.property_constraints()
            .any(|constraint| matches!(constraint, Constraint::Required))
    }

    /// Whether this atom supplies a default for its containing property.
    pub(crate) fn has_default(&self) -> bool {
        self.property_constraints()
            .any(|constraint| matches!(constraint, Constraint::Default(_)))
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
    /// Anything.
    Any,
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
            SimplifiedType::Any => "any",
        }
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
            "any" => SimplifiedType::Any,
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

    // ── file ─────────────────────────────────────────────────────────────
    /// Glob patterns the resolved path must match. Patterns starting with `!`
    /// exclude.
    Match(Vec<String>),

    /// Opt in to eager existence validation: a present `file` value must
    /// resolve to an existing file. `file`-only; bare `file` is lazy.
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

    // ── documentation ──────────────────────────────────────────────────────
    /// One or more example artifact file references (`example(...)`, Feature
    /// A). Non-validating for the annotated property — each referenced file is
    /// itself validated at schema-load time. Raw authored reference strings are
    /// preserved here; resolution is deferred to `resolve.rs` (Phase 6).
    Example(Vec<String>),
}

impl Constraint {
    /// Returns the keyword name as it appears in the SimplifiedSchema grammar.
    pub fn keyword(&self) -> &'static str {
        match self {
            Constraint::Required => "required",
            Constraint::Default(_) => "default",
            Constraint::Generated => "generated",
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
            Constraint::Match(_) => "match",
            Constraint::Eager => "eager",
            Constraint::Scheme(_) => "scheme",
            Constraint::Unique => "unique",
            Constraint::MinItems(_) => "min",
            Constraint::MaxItems(_) => "max",
            Constraint::MinKeys(_) => "min-keys",
            Constraint::MaxKeys(_) => "max-keys",
            Constraint::Example(_) => "example",
        }
    }
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
            SimplifiedType::Any,
        ] {
            let kw = ty.as_keyword();
            assert_eq!(SimplifiedType::from_keyword(kw), Some(ty));
        }
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
