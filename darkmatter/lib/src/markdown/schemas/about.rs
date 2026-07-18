//! Typed descriptor catalog for the SimplifiedSchema authoring language.
//!
//! This catalog is the **authoritative implementation-bound reference** for
//! `md schema about`. Every supported type keyword, constraint, schema shape,
//! inline object rule, and coercion rule is described as a typed descriptor
//! rather than as hand-maintained prose so the CLI report and library callers
//! see exactly the same content, and so the parity tests below catch
//! additions, removals, and renames in the underlying grammar (see
//! [`super::simplified::types`]).
//!
//! The catalog is a static, compile-time constant. Accessing it performs no
//! document parsing, no context capture, no `EffectEngine` construction, no
//! file-resolution, and no network access.
//!
//! See `darkmatter/features/2026-06-10-schema-improvement/spec.md` for the
//! authoring reference that the descriptor surface mirrors.

/// All schema-language type keywords, in display order.
///
/// The set is **closed**: the test
/// [`simplified_type_keyword_set_matches_descriptor_set`] asserts the
/// `SimplifiedType` enum and this descriptor list agree exactly.
pub const SCHEMA_TYPE_DESCRIPTORS: &[SchemaTypeDescriptor] = &[
    SchemaTypeDescriptor {
        keyword: "string",
        description: "Text.",
        accepted_constraints: "min, max, not-empty, pattern, suggest, default, required",
        json_schema_effect: "{ \"type\": \"string\" } plus minLength / maxLength / pattern / default",
    },
    SchemaTypeDescriptor {
        keyword: "date",
        description: "ISO-8601 date (`YYYY-MM-DD`).",
        accepted_constraints: "default, required",
        json_schema_effect: "{ \"type\": \"string\", \"format\": \"date\" }",
    },
    SchemaTypeDescriptor {
        keyword: "datetime",
        description: "ISO-8601 datetime; the timezone offset is optional.",
        accepted_constraints: "default, required",
        json_schema_effect: "{ \"type\": \"string\", \"format\": \"darkmatter-datetime\" }",
    },
    SchemaTypeDescriptor {
        keyword: "time",
        description: "Time of day with optional timezone.",
        accepted_constraints: "default, required",
        json_schema_effect: "{ \"type\": \"string\", \"format\": \"darkmatter-time\" }",
    },
    SchemaTypeDescriptor {
        keyword: "number",
        description: "A numeric value.",
        accepted_constraints: "min, max, integer, suggest, default, required",
        json_schema_effect: "{ \"type\": \"number\" } (or `integer` when `integer` is set) with optional minimum / maximum",
    },
    SchemaTypeDescriptor {
        keyword: "numberlike",
        description: "A number, or a string that can be coerced to a number during compose.",
        accepted_constraints: "default, required",
        json_schema_effect: "anyOf: [ { \"type\": \"number\" }, { \"type\": \"string\", \"pattern\": \"^-?\\d+(\\.\\d+)?$\" } ]",
    },
    SchemaTypeDescriptor {
        keyword: "boolean",
        description: "A true or false value.",
        accepted_constraints: "default, required",
        json_schema_effect: "{ \"type\": \"boolean\" } with optional default",
    },
    SchemaTypeDescriptor {
        keyword: "boolish",
        description: "A boolean, or a `true` / `false` string that can be coerced during compose.",
        accepted_constraints: "default, required",
        json_schema_effect: "anyOf: [ { \"type\": \"boolean\" }, { \"enum\": [\"true\", \"false\", \"True\", \"False\", \"TRUE\", \"FALSE\"] } ]",
    },
    SchemaTypeDescriptor {
        keyword: "object",
        description: "Any object shape. Use an inline object literal when you want to validate nested fields.",
        accepted_constraints: "default, required",
        json_schema_effect: "{ \"type\": \"object\" } with no `additionalProperties: false` (root-schema behavior preserved)",
    },
    SchemaTypeDescriptor {
        keyword: "file",
        description: "A path-like file reference. Lazy by default (syntax-only); add `eager` to require the file to exist. Use `match(...)` to suggest accepted paths.",
        accepted_constraints: "eager, match(glob, ...), default, required",
        json_schema_effect: "{ \"type\": \"string\", \"format\": \"darkmatter-file-reference\" } (lazy); `file(eager)` emits `format: darkmatter-file` (eager existence check). `match(...)` is suggestion metadata only and is not emitted.",
    },
    SchemaTypeDescriptor {
        keyword: "enum",
        description: "Value must be one of the constraint-supplied members.",
        accepted_constraints: "<members>; default; required",
        json_schema_effect: "{ \"enum\": [ ... ] } with optional default",
    },
    SchemaTypeDescriptor {
        keyword: "url",
        description: "Absolute URL.",
        accepted_constraints: "scheme(http, https, ...), default, required",
        json_schema_effect: "{ \"type\": \"string\", \"format\": \"uri\" } plus x-darkmatter-url-scheme when `scheme(...)` is set",
    },
    SchemaTypeDescriptor {
        keyword: "email",
        description: "RFC 5322 addr-spec.",
        accepted_constraints: "default, required",
        json_schema_effect: "{ \"type\": \"string\", \"format\": \"email\" }",
    },
    SchemaTypeDescriptor {
        keyword: "literal",
        description: "The value must equal exactly one scalar of any scalar type. Bare `true`/`false` types as boolean, a bare numberlike token as number, and quoted or any other bare token as string. Replaces the single-member enum and makes discriminated unions first-class. Coerces a document value to the literal's scalar type (`\"2\"` against `literal(2)` → `2`); string literals never coerce.",
        accepted_constraints: "<value>; default; required",
        json_schema_effect: "{ \"const\": <value> } with the value's lexed type, wrapped by the optional-nullable wrapper; `literal(x)[]` places the `const` under `items`",
    },
    SchemaTypeDescriptor {
        keyword: "expression",
        description: "A string that must parse under the Darkmatter expression grammar. Parse-only and never evaluated — the third content-format string type alongside `yaml` and `json`. Native boolean/number values coerce to their string forms (`true` → `\"true\"`); mappings and sequences are type mismatches.",
        accepted_constraints: "default, required, generated",
        json_schema_effect: "{ \"type\": \"string\", \"format\": \"darkmatter-expression\" }",
    },
    SchemaTypeDescriptor {
        keyword: "type-definition",
        description: "One complete SimplifiedSchema property definition carried by a YAML string, mapping, or sequence. Parse-only and side-effect-free; DMLS uses it for schema-language completion, hover, and diagnostics.",
        accepted_constraints: "default, required, generated",
        json_schema_effect: "{ \"type\": [\"string\", \"object\", \"array\"], \"x-darkmatter-type-definition\": true }",
    },
    SchemaTypeDescriptor {
        keyword: "schema",
        description: "One complete `$schema` declaration carried by a YAML string, mapping, or sequence. Parse-only and side-effect-free; DMLS uses it for declaration completion, hover, and diagnostics.",
        accepted_constraints: "default, required, generated",
        json_schema_effect: "{ \"type\": [\"string\", \"object\", \"array\"], \"x-darkmatter-schema\": true }",
    },
    SchemaTypeDescriptor {
        keyword: "any",
        description: "Anything. Useful when you only care that a property exists.",
        accepted_constraints: "required",
        json_schema_effect: "{} (empty schema)",
    },
];

/// Returns all schema-language type descriptors in display order.
pub fn schema_type_descriptors() -> &'static [SchemaTypeDescriptor] {
    SCHEMA_TYPE_DESCRIPTORS
}

/// All schema-language constraints, in display order.
///
/// The test [`constraint_set_matches_descriptor_set`] asserts the
/// `Constraint` enum and the descriptor `keyword`s agree exactly, so adding
/// a new constraint variant breaks the test until the descriptor is updated.
pub const SCHEMA_CONSTRAINT_DESCRIPTORS: &[SchemaConstraintDescriptor] = &[
    // ── universal ──────────────────────────────────────────────────────
    SchemaConstraintDescriptor {
        name: "required",
        keyword: "required",
        form: "required",
        target_types: "all types",
        argument_arity: "0",
        description: "The property must be present.",
        json_schema_effect: "parent's `required` list contains the property name",
    },
    SchemaConstraintDescriptor {
        name: "default",
        keyword: "default",
        form: "default(value)",
        target_types: "all types",
        argument_arity: "1",
        description: "Provides the value to use when the property is absent.",
        json_schema_effect: "parent's `default` set to the argument",
    },
    SchemaConstraintDescriptor {
        name: "generated",
        keyword: "generated",
        form: "generated",
        target_types: "all types",
        argument_arity: "0",
        description: "The value is supplied by the host runtime (e.g. Darkmatter context capture), not authored in static frontmatter. Orthogonal to `required`, which controls type/nullability.",
        json_schema_effect: "emits `x-darkmatter-generated: true`; suppresses the property's static-`required` entry so authored documents validate cleanly when the host has not yet supplied the value",
    },
    // ── shared scalar / array bounds ───────────────────────────────────
    SchemaConstraintDescriptor {
        name: "min",
        keyword: "min",
        form: "min(number)",
        target_types: "string | number | array",
        argument_arity: "1",
        description: "Sets the minimum allowed size or value.",
        json_schema_effect: "string: minLength; number: minimum; array: minItems",
    },
    SchemaConstraintDescriptor {
        name: "max",
        keyword: "max",
        form: "max(number)",
        target_types: "string | number | array",
        argument_arity: "1",
        description: "Sets the maximum allowed size or value.",
        json_schema_effect: "string: maxLength; number: maximum; array: maxItems",
    },
    // ── numeric ────────────────────────────────────────────────────────
    SchemaConstraintDescriptor {
        name: "integer",
        keyword: "integer",
        form: "integer",
        target_types: "number",
        argument_arity: "0",
        description: "Restrict to integer values.",
        json_schema_effect: "type is `integer` instead of `number`",
    },
    // ── string ─────────────────────────────────────────────────────────
    SchemaConstraintDescriptor {
        name: "not-empty",
        keyword: "not-empty",
        form: "not-empty",
        target_types: "string",
        argument_arity: "0",
        description: "Disallow empty / whitespace-only strings.",
        json_schema_effect: "pattern set to \"\\\\S\"",
    },
    SchemaConstraintDescriptor {
        name: "pattern",
        keyword: "pattern",
        form: "pattern(regex)",
        target_types: "string",
        argument_arity: "1",
        description: "ECMA-262 regex the value must match.",
        json_schema_effect: "pattern set to the supplied regex",
    },
    // ── enum ───────────────────────────────────────────────────────────
    SchemaConstraintDescriptor {
        name: "<members>",
        keyword: "<members>",
        form: "enum(member, member, ...)",
        target_types: "enum",
        argument_arity: "1+",
        description: "Lists the allowed values. At least one member is required.",
        json_schema_effect: "enum list of supplied members",
    },
    // ── literal ────────────────────────────────────────────────────────
    SchemaConstraintDescriptor {
        name: "<value>",
        keyword: "<value>",
        form: "literal(value)",
        target_types: "literal",
        argument_arity: "1",
        description: "The single scalar value the property must equal. Typed like YAML: bare `true`/`false` → boolean, bare numberlike → number, quoted or any other bare token → string. Exactly one value; use `enum(...)` for a set.",
        json_schema_effect: "const set to the typed value",
    },
    // ── file ───────────────────────────────────────────────────────────
    SchemaConstraintDescriptor {
        name: "eager",
        keyword: "eager",
        form: "eager",
        target_types: "file",
        argument_arity: "0",
        description: "Require the referenced file to exist at validation time. Without it, `file` is lazy (syntax-only).",
        json_schema_effect: "emits `format: darkmatter-file` (eager) instead of the lazy `darkmatter-file-reference`",
    },
    SchemaConstraintDescriptor {
        name: "match",
        keyword: "match",
        form: "match(glob, ...)",
        target_types: "file",
        argument_arity: "1+",
        description: "Glob patterns that shape file path suggestions (completion). Patterns starting with `!` exclude. Suggestion metadata only — not validated.",
        json_schema_effect: "none (suggestion metadata; never lowered into the compiled JSON Schema)",
    },
    // ── url ────────────────────────────────────────────────────────────
    SchemaConstraintDescriptor {
        name: "scheme",
        keyword: "scheme",
        form: "scheme(http, https, ...)",
        target_types: "url",
        argument_arity: "1+",
        description: "Allowed URL schemes (lowercased).",
        json_schema_effect: "x-darkmatter-url-scheme set to the scheme list",
    },
    // ── advisory suggestions ───────────────────────────────────────────
    SchemaConstraintDescriptor {
        name: "suggest",
        keyword: "suggest",
        form: "suggest(value, ...)",
        target_types: "string | number",
        argument_arity: "1+",
        description: "Advisory completion candidates for DMLS. Non-validating: a document value is valid when it satisfies the underlying type and constraints, whether or not it appears in the list. At most one `suggest(...)` per complete property definition (including across property-union atoms). Array forms (`string[]`, `number[]`) describe individual elements, not whole arrays.",
        json_schema_effect: "emits `x-darkmatter-suggest: [interpreted values]` on the scalar or array `items` schema in declaration order; never lowered to `examples` or `x-darkmatter-example`",
    },
    // ── array ──────────────────────────────────────────────────────────
    SchemaConstraintDescriptor {
        name: "unique",
        keyword: "unique",
        form: "unique",
        target_types: "array",
        argument_arity: "0",
        description: "Array items must be distinct.",
        json_schema_effect: "uniqueItems set to true",
    },
];

/// Returns all schema-language constraint descriptors in display order.
pub fn schema_constraint_descriptors() -> &'static [SchemaConstraintDescriptor] {
    SCHEMA_CONSTRAINT_DESCRIPTORS
}

/// All supported `$schema` shapes, in display order.
pub const SCHEMA_SHAPE_DESCRIPTORS: &[SchemaShapeDescriptor] = &[
    SchemaShapeDescriptor {
        name: "Single object",
        form: "single mapping",
        example: "title: string(required)",
        description: "Use this for the common case: property names mapped to type expressions.",
    },
    SchemaShapeDescriptor {
        name: "Root-level union",
        form: "sequence of mappings / file references",
        example: "- title: string\n- ./schemas/post.yaml",
        description: "Use this when a document may follow one of several schema alternatives.",
    },
    SchemaShapeDescriptor {
        name: "Property-level union",
        form: "sequence of strings under a property",
        example: "data: [string, \"{ foo: string }\"]",
        description: "Use this when one property may have one of several allowed types.",
    },
    SchemaShapeDescriptor {
        name: "Inline object literal",
        form: "{ prop: type-expr, ... }",
        example: "{ foo: string(required), bar: number }",
        description: "Use this to validate a nested object without creating a separate schema file.",
    },
    SchemaShapeDescriptor {
        name: "Nested mapping object",
        form: "YAML mapping under a property",
        example: "addr:\n  street: string\n  zip: number",
        description: "Use a YAML mapping as a property value to declare a nested object shape. Equivalent to the inline object literal for the cases both forms can express; the mapping form additionally permits sequence union arms without quoting.",
    },
];

/// Returns all schema shape descriptors in display order.
pub fn schema_shape_descriptors() -> &'static [SchemaShapeDescriptor] {
    SCHEMA_SHAPE_DESCRIPTORS
}

/// Inline object grammar rules and limits, in display order.
pub const INLINE_OBJECT_RULE_DESCRIPTORS: &[InlineObjectRuleDescriptor] = &[
    InlineObjectRuleDescriptor {
        name: "Object shape",
        rule: "`{ title: string, rating: number }`",
        description: "Write fields inside braces as a comma-separated list.",
    },
    InlineObjectRuleDescriptor {
        name: "Whitespace",
        rule: "Multi-line objects parse the same as one-line objects.",
        description: "Add spaces or line breaks wherever they make the schema easier to read.",
    },
    InlineObjectRuleDescriptor {
        name: "Property names",
        rule: "`name`, `foo_id`, `x-custom`, `api2_version`, and `123abc` are valid.",
        description: "Use unquoted names made from letters, numbers, `_`, and `-`.",
    },
    InlineObjectRuleDescriptor {
        name: "Property values",
        rule: "Properties can be bare, constrained, or described.",
        description: "Use the same type expression syntax inside an inline object that you use at the top level.",
    },
    InlineObjectRuleDescriptor {
        name: "Descriptions",
        rule: "A `->` description ends at the next comma or closing brace for that object.",
        description: "Keep inline descriptions short and avoid commas inside them.",
    },
    InlineObjectRuleDescriptor {
        name: "Arrays",
        rule: "`{ title: string }[]`",
        description: "Add `[]` after the object when the property is an array of objects.",
    },
    InlineObjectRuleDescriptor {
        name: "Object constraints",
        rule: "`{ title: string }(required)` and `{ title: string }[](min(1); required)`",
        description: "Put constraints after the closing brace, or after `[]` for an array of objects.",
    },
    InlineObjectRuleDescriptor {
        name: "Nesting depth",
        rule: "Maximum depth: 32 inline object levels.",
        description: "Deeply nested schemas are supported, but they are usually harder to read.",
    },
    InlineObjectRuleDescriptor {
        name: "Extra keys",
        rule: "Inline objects use `additionalProperties: false`.",
        description: "A nested object may only contain the fields declared in the inline object.",
    },
    InlineObjectRuleDescriptor {
        name: "Nested required fields",
        rule: "`required` stays with the object where it is written.",
        description: "A required nested field is required inside its parent object, not at the document root.",
    },
];

/// Returns all inline object rule descriptors in display order.
pub fn inline_object_rule_descriptors() -> &'static [InlineObjectRuleDescriptor] {
    INLINE_OBJECT_RULE_DESCRIPTORS
}

/// Compose-time coercion rules, in display order.
pub const COERCION_RULE_DESCRIPTORS: &[CoercionRuleDescriptor] = &[
    CoercionRuleDescriptor {
        name: "Booleans",
        rule: "`true` and `false`, in any case, become real booleans.",
        description: "When the schema expects `boolean` or `boolish`, common string forms are converted for you.",
    },
    CoercionRuleDescriptor {
        name: "Numbers",
        rule: "Numeric strings become real numbers.",
        description: "When the schema expects `number` or `numberlike`, strings such as `42` or `3.14` are converted for you.",
    },
    CoercionRuleDescriptor {
        name: "Nested values",
        rule: "Coercion recurses into inline objects and arrays of inline objects.",
        description: "Nested fields get the same boolean and number treatment as top-level fields.",
    },
    CoercionRuleDescriptor {
        name: "Clear union match",
        rule: "Exactly one union arm must match after coercion.",
        description: "Darkmatter only commits a coerced union value when the result is unambiguous.",
    },
    CoercionRuleDescriptor {
        name: "No union match",
        rule: "The original value is kept.",
        description: "If no union arm matches, validation reports the problem without guessing a conversion.",
    },
    CoercionRuleDescriptor {
        name: "Ambiguous union match",
        rule: "The original value is kept.",
        description: "If more than one union arm matches, Darkmatter leaves the value alone instead of choosing for you.",
    },
    CoercionRuleDescriptor {
        name: "Shell-expanded values",
        rule: "Values containing `$(...)` are deferred until shell expansion has run.",
        description: "Darkmatter waits to coerce values that are not final yet.",
    },
];

/// Returns all compose-time coercion rule descriptors in display order.
pub fn coercion_rule_descriptors() -> &'static [CoercionRuleDescriptor] {
    COERCION_RULE_DESCRIPTORS
}

/// Validation behavior notes that affect how the schema language is consumed,
/// in display order.
pub const VALIDATION_BEHAVIOR_DESCRIPTORS: &[ValidationBehaviorDescriptor] = &[
    ValidationBehaviorDescriptor {
        name: "Extra root keys",
        rule: "Root schemas allow extra frontmatter keys.",
        description: "A document may contain frontmatter properties that are not named in `$schema`.",
    },
    ValidationBehaviorDescriptor {
        name: "Extra nested keys",
        rule: "Inline objects use `additionalProperties: false`.",
        description: "Nested inline objects are explicit shapes; undeclared keys fail validation.",
    },
    ValidationBehaviorDescriptor {
        name: "Plain objects",
        rule: "`object` accepts any object shape.",
        description: "Use an inline object when you want Darkmatter to validate fields inside the object.",
    },
    ValidationBehaviorDescriptor {
        name: "File references",
        rule: "`file` is lazy (syntax-only); `file(eager)` checks existence.",
        description: "Bare `file` only validates that the value parses as a biscuit-file reference. Add `eager` to require the resolved file to exist; relative paths resolve from the current document context.",
    },
    ValidationBehaviorDescriptor {
        name: "Schema detection",
        rule: "`md schema detect` emits `object` for object-shaped values.",
        description: "Write inline object schemas by hand when you want nested validation.",
    },
    ValidationBehaviorDescriptor {
        name: "Coercion timing",
        rule: "Coercion runs after `--set`, `--state`, and interpolation.",
        description: "Later compose stages see the coerced booleans and numbers.",
    },
    ValidationBehaviorDescriptor {
        name: "Baseline merge",
        rule: "Baseline properties merge into the effective schema.",
        description: "A configured baseline schema is combined with the document schema, or used alone when the document has no `$schema`.",
    },
    ValidationBehaviorDescriptor {
        name: "Validator cache",
        rule: "Compiled validators are LRU-cached per process.",
        description: "Repeated validation against the same effective schema reuses the compiled validator.",
    },
];

/// Returns all validation behavior descriptors in display order.
pub fn validation_behavior_descriptors() -> &'static [ValidationBehaviorDescriptor] {
    VALIDATION_BEHAVIOR_DESCRIPTORS
}

/// Trigger-schema grammar forms, in display order.
pub const TRIGGER_GRAMMAR_DESCRIPTORS: &[TriggerGrammarDescriptor] = &[
    TriggerGrammarDescriptor {
        name: "Envelope",
        form: "kind: trigger-schema",
        description: "Claims a discovered YAML file as a trigger envelope; `$schema` is its merge-compatible object-schema payload.",
    },
    TriggerGrammarDescriptor {
        name: "Property condition",
        form: "property: type-expr",
        description: "A bare expression is an optional guard; `required` makes it a presence gate. Present values must satisfy the type and match-safe constraints.",
    },
    TriggerGrammarDescriptor {
        name: "$path",
        form: "$path: glob | [glob, ...]",
        description: "Matches the case-sensitive, boundary-relative path with `/` separators. Basename globs match in any directory and `!` patterns exclude.",
    },
    TriggerGrammarDescriptor {
        name: "all",
        form: "all: [condition, ...]",
        description: "Matches when every child condition matches.",
    },
    TriggerGrammarDescriptor {
        name: "any",
        form: "any: [condition, ...]",
        description: "Matches when at least one child condition matches.",
    },
    TriggerGrammarDescriptor {
        name: "none",
        form: "none: [condition, ...]",
        description: "Matches when no child condition matches.",
    },
    TriggerGrammarDescriptor {
        name: "min-match",
        form: "min-match: { count: N, of: [condition, ...] }",
        description: "Matches when at least N of the child conditions match.",
    },
    TriggerGrammarDescriptor {
        name: "Outer OR arms",
        form: "match: [arm, ...]",
        description: "A mapping is one arm; a sequence of arm mappings is OR'd. Mixing property and structural keys in one mapping is invalid.",
    },
    TriggerGrammarDescriptor {
        name: "Vacuous-arm lint",
        form: "each arm requires a positive gate",
        description: "Every satisfiable arm must require presence through `required` or `$path`, outside `none`; an arm that could match every document is rejected.",
    },
];

/// Returns trigger-schema grammar descriptors in display order.
pub fn trigger_grammar_descriptors() -> &'static [TriggerGrammarDescriptor] {
    TRIGGER_GRAMMAR_DESCRIPTORS
}

/// Constraints accepted by trigger property conditions.
pub const MATCH_SAFE_CONSTRAINT_DESCRIPTORS: &[MatchSafeConstraintDescriptor] = &[
    MatchSafeConstraintDescriptor { keyword: "required", description: "Requires the property to be present." },
    MatchSafeConstraintDescriptor { keyword: "<members>", description: "Restricts `enum` to literal members." },
    MatchSafeConstraintDescriptor { keyword: "pattern", description: "Matches a string against a regular expression." },
    MatchSafeConstraintDescriptor { keyword: "min", description: "Checks a minimum value, length, or item count." },
    MatchSafeConstraintDescriptor { keyword: "max", description: "Checks a maximum value, length, or item count." },
    MatchSafeConstraintDescriptor { keyword: "integer", description: "Requires an integral numeric value." },
    MatchSafeConstraintDescriptor { keyword: "not-empty", description: "Requires non-whitespace string content." },
    MatchSafeConstraintDescriptor { keyword: "unique", description: "Requires distinct array items." },
    MatchSafeConstraintDescriptor { keyword: "min-keys", description: "Checks a minimum object key count." },
    MatchSafeConstraintDescriptor { keyword: "max-keys", description: "Checks a maximum object key count." },
];

/// Returns the constraint subset accepted in trigger matches.
pub fn match_safe_constraint_descriptors() -> &'static [MatchSafeConstraintDescriptor] {
    MATCH_SAFE_CONSTRAINT_DESCRIPTORS
}

// ── Descriptor types ────────────────────────────────────────────────────

/// Descriptor for a single schema-language type keyword.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaTypeDescriptor {
    /// Canonical lowercase keyword used in the grammar.
    pub keyword: &'static str,
    /// Human-readable description of the type's meaning.
    pub description: &'static str,
    /// Comma-separated list of constraints accepted on this type.
    pub accepted_constraints: &'static str,
    /// Effect on the generated Draft 2020-12 JSON Schema fragment.
    pub json_schema_effect: &'static str,
}

/// Descriptor for a single schema-language constraint.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaConstraintDescriptor {
    /// Display name of the constraint. May be disambiguated with a type
    /// context, e.g. `min (number)`, when the same keyword has different
    /// behaviors on different types.
    pub name: &'static str,
    /// Canonical keyword as it appears in the grammar. Mirrors
    /// [`Constraint::keyword`] so descriptor ↔ implementation parity is
    /// testable.
    pub keyword: &'static str,
    /// Surface form of the constraint, including the `(...)` suffix when applicable.
    pub form: &'static str,
    /// Types the constraint applies to.
    pub target_types: &'static str,
    /// Number of arguments the constraint accepts.
    pub argument_arity: &'static str,
    /// Short description of the constraint's behavior.
    pub description: &'static str,
    /// Effect on the generated Draft 2020-12 JSON Schema fragment.
    pub json_schema_effect: &'static str,
}

/// Descriptor for a single schema shape / grammar form.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaShapeDescriptor {
    /// Display name of the shape.
    pub name: &'static str,
    /// Surface form (YAML / grammar shape).
    pub form: &'static str,
    /// Short example.
    pub example: &'static str,
    /// Short description.
    pub description: &'static str,
}

/// Descriptor for a single inline object grammar rule.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineObjectRuleDescriptor {
    /// Display name of the rule.
    pub name: &'static str,
    /// One-line rule summary.
    pub rule: &'static str,
    /// Short description of the rule and its consequences.
    pub description: &'static str,
}

/// Descriptor for a single compose-time coercion rule.
#[derive(Debug, Clone, PartialEq)]
pub struct CoercionRuleDescriptor {
    /// Display name of the rule.
    pub name: &'static str,
    /// One-line rule summary.
    pub rule: &'static str,
    /// Short description of the rule and its non-coercion cases.
    pub description: &'static str,
}

/// Descriptor for a single validation-behavior note.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationBehaviorDescriptor {
    /// Display name of the note.
    pub name: &'static str,
    /// One-line rule summary.
    pub rule: &'static str,
    /// Short description.
    pub description: &'static str,
}

/// Descriptor for one trigger-schema grammar form.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerGrammarDescriptor {
    /// Author-facing name of the grammar form.
    pub name: &'static str,
    /// Concise YAML surface form.
    pub form: &'static str,
    /// Semantics and relevant restrictions.
    pub description: &'static str,
}

/// Descriptor for one constraint accepted in trigger matching.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchSafeConstraintDescriptor {
    /// Canonical constraint keyword.
    pub keyword: &'static str,
    /// Match-time behavior.
    pub description: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::simplified::types::{Constraint, SimplifiedType};
    use crate::markdown::schemas::simplified::parse_yaml_schema;
    use crate::markdown::schemas::simplified::grammar;
    use crate::markdown::schemas::simplified::{PropertyDef, SimplifiedSchema, TypeExpr};
    use std::collections::HashSet;

    /// Every `SimplifiedType` variant has exactly one type descriptor, and
    /// every type descriptor maps to a parseable implemented type.
    #[test]
    fn simplified_type_keyword_set_matches_descriptor_set() {
        let descriptor_keywords: HashSet<&str> = SCHEMA_TYPE_DESCRIPTORS
            .iter()
            .map(|d| d.keyword)
            .collect();

        let mut implemented: HashSet<&'static str> = HashSet::new();
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
            SimplifiedType::Literal,
            SimplifiedType::Expression,
            SimplifiedType::TypeDefinition,
            SimplifiedType::Schema,
            SimplifiedType::Any,
        ] {
            implemented.insert(ty.as_keyword());
        }

        let missing_descriptors: Vec<_> = implemented.difference(&descriptor_keywords).collect();
        let extra_descriptors: Vec<_> = descriptor_keywords.difference(&implemented).collect();

        assert!(
            missing_descriptors.is_empty(),
            "SimplifiedType keywords without descriptors: {missing_descriptors:?}"
        );
        assert!(
            extra_descriptors.is_empty(),
            "Descriptor keywords without SimplifiedType: {extra_descriptors:?}"
        );
    }

    /// Every descriptor's `keyword` round-trips through the
    /// `SimplifiedType::from_keyword` parser.
    #[test]
    fn descriptor_keywords_are_parseable() {
        for d in SCHEMA_TYPE_DESCRIPTORS {
            let parsed = SimplifiedType::from_keyword(d.keyword);
            assert_eq!(
                parsed.map(|t| t.as_keyword()),
                Some(d.keyword),
                "descriptor keyword `{}` did not round-trip through from_keyword",
                d.keyword
            );
        }
    }

    /// Every implemented `Constraint` variant has descriptor coverage, and
    /// every descriptor maps to a real variant.
    #[test]
    fn constraint_set_matches_descriptor_set() {
        let descriptor_keywords: HashSet<&str> = SCHEMA_CONSTRAINT_DESCRIPTORS
            .iter()
            .map(|d| d.keyword)
            .collect();

        let mut implemented: HashSet<&'static str> = HashSet::new();
        for c in [
            Constraint::Required,
            Constraint::Default(serde_json::Value::Null),
            Constraint::Generated,
            Constraint::Min(0.0),
            Constraint::Max(0.0),
            Constraint::Integer,
            Constraint::MinLen(0),
            Constraint::MaxLen(0),
            Constraint::NotEmpty,
            Constraint::Pattern(String::new()),
            Constraint::Suggest(Vec::new()),
            Constraint::Members(Vec::new()),
            Constraint::LiteralValue(serde_json::Value::Null),
            Constraint::Eager,
            Constraint::Match(Vec::new()),
            Constraint::Scheme(Vec::new()),
            Constraint::Unique,
            Constraint::MinItems(0),
            Constraint::MaxItems(0),
        ] {
            implemented.insert(c.keyword());
        }

        let missing_descriptors: Vec<_> =
            implemented.difference(&descriptor_keywords).collect();
        let extra_descriptors: Vec<_> =
            descriptor_keywords.difference(&implemented).collect();

        assert!(
            missing_descriptors.is_empty(),
            "Constraint keywords without descriptors: {missing_descriptors:?}"
        );
        assert!(
            extra_descriptors.is_empty(),
            "Descriptor keywords without Constraint: {extra_descriptors:?}"
        );
    }

    /// Traversal order is stable across repeated access.
    #[test]
    fn descriptor_traversal_order_is_deterministic() {
        let types_again: Vec<&str> = SCHEMA_TYPE_DESCRIPTORS.iter().map(|d| d.keyword).collect();
        let types_third: Vec<&str> =
            schema_type_descriptors().iter().map(|d| d.keyword).collect();
        assert_eq!(types_again, types_third);

        let constraints_again: Vec<&str> =
            SCHEMA_CONSTRAINT_DESCRIPTORS.iter().map(|d| d.name).collect();
        let constraints_third: Vec<&str> =
            schema_constraint_descriptors().iter().map(|d| d.name).collect();
        assert_eq!(constraints_again, constraints_third);

        let shapes_again: Vec<&str> = SCHEMA_SHAPE_DESCRIPTORS.iter().map(|d| d.name).collect();
        let shapes_third: Vec<&str> =
            schema_shape_descriptors().iter().map(|d| d.name).collect();
        assert_eq!(shapes_again, shapes_third);

        let rules_again: Vec<&str> =
            INLINE_OBJECT_RULE_DESCRIPTORS.iter().map(|d| d.name).collect();
        let rules_third: Vec<&str> =
            inline_object_rule_descriptors().iter().map(|d| d.name).collect();
        assert_eq!(rules_again, rules_third);

        let coercion_again: Vec<&str> = COERCION_RULE_DESCRIPTORS.iter().map(|d| d.name).collect();
        let coercion_third: Vec<&str> =
            coercion_rule_descriptors().iter().map(|d| d.name).collect();
        assert_eq!(coercion_again, coercion_third);

        let validation_again: Vec<&str> =
            VALIDATION_BEHAVIOR_DESCRIPTORS.iter().map(|d| d.name).collect();
        let validation_third: Vec<&str> =
            validation_behavior_descriptors().iter().map(|d| d.name).collect();
        assert_eq!(validation_again, validation_third);
    }

    /// Descriptor keys / signatures are unique within each catalog.
    #[test]
    fn descriptor_keys_are_unique() {
        let mut seen = HashSet::new();
        for d in SCHEMA_TYPE_DESCRIPTORS {
            assert!(
                seen.insert(d.keyword),
                "duplicate type descriptor keyword: {}",
                d.keyword
            );
        }
        let mut seen = HashSet::new();
        for d in SCHEMA_CONSTRAINT_DESCRIPTORS {
            assert!(
                seen.insert(d.name),
                "duplicate constraint descriptor name: {}",
                d.name
            );
        }
        let mut seen = HashSet::new();
        for d in SCHEMA_SHAPE_DESCRIPTORS {
            assert!(
                seen.insert(d.name),
                "duplicate shape descriptor name: {}",
                d.name
            );
        }
        let mut seen = HashSet::new();
        for d in INLINE_OBJECT_RULE_DESCRIPTORS {
            assert!(
                seen.insert(d.name),
                "duplicate inline-object rule descriptor name: {}",
                d.name
            );
        }
        let mut seen = HashSet::new();
        for d in COERCION_RULE_DESCRIPTORS {
            assert!(
                seen.insert(d.name),
                "duplicate coercion rule descriptor name: {}",
                d.name
            );
        }
        let mut seen = HashSet::new();
        for d in VALIDATION_BEHAVIOR_DESCRIPTORS {
            assert!(
                seen.insert(d.name),
                "duplicate validation behavior descriptor name: {}",
                d.name
            );
        }
    }

    /// Accessing every catalog performs no document parsing, no context
    /// capture, no `EffectEngine` construction, no file resolution, and no
    /// network access. Constructing and reading the catalogs is pure
    /// static metadata access — this test exists as a regression guard so
    /// the catalogs never start touching state on read.
    #[test]
    fn catalog_access_performs_no_capture() {
        let _ = schema_type_descriptors();
        let _ = schema_constraint_descriptors();
        let _ = schema_shape_descriptors();
        let _ = inline_object_rule_descriptors();
        let _ = coercion_rule_descriptors();
        let _ = validation_behavior_descriptors();
        let _ = trigger_grammar_descriptors();
        let _ = match_safe_constraint_descriptors();
    }

    #[test]
    fn trigger_combinator_descriptor_set_matches_grammar() {
        use crate::markdown::schemas::triggers::grammar::{COMBINATOR_KEYS, PATH_KEY};

        let described: HashSet<&str> = TRIGGER_GRAMMAR_DESCRIPTORS
            .iter()
            .filter_map(|descriptor| {
                COMBINATOR_KEYS.contains(&descriptor.name).then_some(descriptor.name)
            })
            .collect();
        assert_eq!(described, COMBINATOR_KEYS.iter().copied().collect());
        assert!(TRIGGER_GRAMMAR_DESCRIPTORS.iter().any(|d| d.name == PATH_KEY));
    }

    #[test]
    fn match_safe_constraint_descriptor_set_matches_implementation() {
        use crate::markdown::schemas::triggers::grammar::is_match_safe_constraint;

        let constraints = [
            Constraint::Required,
            Constraint::Default(serde_json::Value::Null),
            Constraint::Generated,
            Constraint::Min(0.0),
            Constraint::Max(0.0),
            Constraint::Integer,
            Constraint::MinLen(0),
            Constraint::MaxLen(0),
            Constraint::NotEmpty,
            Constraint::Pattern(String::new()),
            Constraint::Suggest(Vec::new()),
            Constraint::Members(Vec::new()),
            Constraint::Eager,
            Constraint::Match(Vec::new()),
            Constraint::Scheme(Vec::new()),
            Constraint::Unique,
            Constraint::MinItems(0),
            Constraint::MaxItems(0),
            Constraint::MinKeys(0),
            Constraint::MaxKeys(0),
        ];
        let implemented: HashSet<&str> = constraints
            .iter()
            .filter(|constraint| is_match_safe_constraint(constraint))
            .map(Constraint::keyword)
            .collect();
        let described: HashSet<&str> = MATCH_SAFE_CONSTRAINT_DESCRIPTORS
            .iter()
            .map(|descriptor| descriptor.keyword)
            .collect();
        assert_eq!(described, implemented);
    }

    /// Descriptor examples in the shape catalog parse to a
    /// `SimplifiedSchema` without error. The shape catalog's examples are
    /// chosen to match the catalog label: the `Single object` example is a
    /// YAML mapping, the `Root-level union` and `Property-level union`
    /// examples are YAML sequences, and the `Inline object literal` example
    /// is a string-layer type expression with braces.
    #[test]
    fn descriptor_shape_examples_parse() {
        for d in SCHEMA_SHAPE_DESCRIPTORS {
            match d.name {
                "Single object" => {
                    let value: serde_yaml_ng::Value =
                        serde_yaml_ng::from_str(d.example).expect("yaml must parse");
                    let schema = parse_yaml_schema(&value)
                        .expect("single-mapping example must parse as a SimplifiedSchema");
                    assert!(
                        matches!(schema, SimplifiedSchema::Single(_)),
                        "Single object example did not parse as Single: {schema:?}"
                    );
                }
                "Root-level union" => {
                    let value: serde_yaml_ng::Value =
                        serde_yaml_ng::from_str(d.example).expect("yaml must parse");
                    let schema = parse_yaml_schema(&value)
                        .expect("root-union example must parse as a SimplifiedSchema");
                    assert!(
                        matches!(schema, SimplifiedSchema::Union(_)),
                        "Root-level union example did not parse as Union: {schema:?}"
                    );
                }
                "Property-level union" => {
                    // The example embeds a brace literal inside a YAML
                    // sequence. Round-trip the inline-object part through
                    // the grammar parser and then assemble a YAML scalar for
                    // the property-level-union path.
                    let brace_start = d.example.find('[').map(|i| i + 1).unwrap_or(0);
                    let brace_end = d.example.rfind(']').unwrap_or(d.example.len());
                    let inner = &d.example[brace_start..brace_end];
                    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
                    assert!(
                        parts.len() >= 2,
                        "Property-level union example must list at least two type expressions"
                    );
                    for part in &parts {
                        let stripped = part.trim_matches('"');
                        let _ = grammar::parse_type_expr("test", stripped)
                            .unwrap_or_else(|e| {
                                panic!("union arm `{stripped}` failed to parse: {e:?}")
                            });
                    }
                    let wrapped = format!("x: [{arm_a}, {arm_b}]", arm_a = parts[0], arm_b = parts[1]);
                    let value: serde_yaml_ng::Value =
                        serde_yaml_ng::from_str(&wrapped).expect("yaml must parse");
                    let schema = parse_yaml_schema(&value)
                        .expect("property-union example must parse as a SimplifiedSchema");
                    let shape = match schema {
                        SimplifiedSchema::Single(s) => s,
                        other => panic!("expected Single, got {other:?}"),
                    };
                    let prop = shape.properties.get("x").expect("x property");
                    assert!(
                        matches!(prop, PropertyDef::Union(_)),
                        "Property-level union example did not parse as PropertyDef::Union: {prop:?}"
                    );
                }
                "Inline object literal" => {
                    let atom = grammar::parse_type_expr("test", d.example)
                        .expect("inline-object example must parse as a type expression");
                    assert!(
                        matches!(atom.ty, TypeExpr::InlineObject(_)),
                        "Inline object literal example did not parse as TypeExpr::InlineObject: {:?}",
                        atom.ty
                    );
                }
                "Nested mapping object" => {
                    // The example is a top-level mapping with one property
                    // whose value is itself a mapping. It must parse as a
                    // Single schema whose sole property lowers to an
                    // inline-object atom.
                    let value: serde_yaml_ng::Value =
                        serde_yaml_ng::from_str(d.example).expect("yaml must parse");
                    let schema = parse_yaml_schema(&value)
                        .expect("nested-mapping example must parse as a SimplifiedSchema");
                    let shape = match schema {
                        SimplifiedSchema::Single(s) => s,
                        other => panic!("expected Single, got {other:?}"),
                    };
                    let prop = shape
                        .properties
                        .values()
                        .next()
                        .expect("nested-mapping example must declare at least one property");
                    let atom = match prop {
                        PropertyDef::Single(a) => a,
                        other => panic!("expected Single atom, got {other:?}"),
                    };
                    assert!(
                        matches!(atom.ty, TypeExpr::InlineObject(_)),
                        "Nested mapping object example did not lower to TypeExpr::InlineObject: {:?}",
                        atom.ty
                    );
                }
                other => panic!("unexpected shape descriptor: {other}"),
            }
        }
    }

    /// The `suggest(...)` descriptor advertises eligibility on `string` and
    /// `number`. This test verifies that every advertised eligible type
    /// accepts the constraint and every excluded type rejects it, so the
    /// catalog and the grammar cannot drift.
    #[test]
    fn suggest_descriptor_eligibility_matches_grammar() {
        fn accepts(type_expr: &str) -> bool {
            grammar::parse_type_expr("test", type_expr).is_ok()
        }

        // Eligible scalar types (per descriptor `target_types`).
        assert!(accepts("string(suggest(red, green))"));
        assert!(accepts("number(suggest(1, 2, 3))"));
        assert!(accepts("number(integer; suggest(80, 443))"));

        // Eligible array forms — candidates describe individual elements.
        assert!(accepts("string(suggest(alpha, beta))[]"));
        assert!(accepts("number(integer; suggest(1, 2, 3))[]"));

        // Ineligible types must reject the constraint.
        let ineligible = [
            "boolean(suggest(true))",
            "boolish(suggest(true))",
            "numberlike(suggest(1))",
            "date(suggest(2024-01-01))",
            "datetime(suggest(2024-01-01T00:00:00Z))",
            "time(suggest(12:00))",
            "url(suggest(https://example.com))",
            "email(suggest(a@b.com))",
            "enum(suggest(x))",
            "any(suggest(x))",
            "object(suggest(x))",
            "file(suggest(x))",
        ];
        for expr in ineligible {
            assert!(
                !accepts(expr),
                "`suggest(...)` should be rejected on `{expr}` per the descriptor catalog"
            );
        }
    }

    /// The descriptor catalog's `suggest` entry uses keyword `suggest`, which
    /// must round-trip through [`Constraint::keyword`].
    #[test]
    fn suggest_descriptor_keyword_matches_constraint() {
        let descriptor = SCHEMA_CONSTRAINT_DESCRIPTORS
            .iter()
            .find(|d| d.keyword == "suggest")
            .expect("suggest descriptor must exist");
        assert_eq!(
            Constraint::Suggest(Vec::new()).keyword(),
            descriptor.keyword,
        );
    }
}
