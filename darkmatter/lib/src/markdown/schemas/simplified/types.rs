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
    /// Declaration-order map of property name to definition. `IndexMap`
    /// preserves insertion order so generated JSON Schemas are deterministic
    /// and diff-friendly.
    pub properties: IndexMap<String, PropertyDef>,
}

impl SchemaShape {
    /// Creates an empty `SchemaShape`.
    pub fn new() -> Self {
        Self::default()
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

/// A type expression is either a primitive type keyword or an inline object
/// literal declared with `{ ... }` syntax.
///
/// `TypeExpr` is `Clone` but not `Copy` because the inline-object arm carries
/// an owned [`SchemaShape`]. Call sites that previously matched on
/// `atom.ty: SimplifiedType` now match on `atom.ty: TypeExpr` and handle the
/// `Primitive` / `InlineObject` arms.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Built-in primitive type keyword (`string`, `number`, `object`, etc.).
    Primitive(SimplifiedType),
    /// Inline object literal: `{ foo: string, bar: number }`.
    InlineObject(SchemaShape),
}

/// One arm of a property-level union (or the body of a non-union property).
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyAtom {
    /// The type expression for this atom (primitive or inline object).
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

    // ── enum ─────────────────────────────────────────────────────────────
    /// Enumerated members.
    Members(Vec<String>),

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
            Constraint::Members(_) => "<members>",
            Constraint::Match(_) => "match",
            Constraint::Eager => "eager",
            Constraint::Scheme(_) => "scheme",
            Constraint::Unique => "unique",
            Constraint::MinItems(_) => "min",
            Constraint::MaxItems(_) => "max",
        }
    }
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
    fn schema_shape_default_is_empty() {
        let shape = SchemaShape::new();
        assert!(shape.properties.is_empty());
    }

    #[test]
    fn generated_keyword_is_canonical() {
        assert_eq!(Constraint::Generated.keyword(), "generated");
    }
}
