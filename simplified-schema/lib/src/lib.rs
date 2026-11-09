//! The SimplifiedSchema language, independent of any Markdown host.
//!
//! The crate holds the language and everything that judges values with it:
//!
//! - [`numeric`]: admission of written numbers into finite `f64`s;
//! - [`span`]: byte spans over the exact supplied text;
//! - [`expression`]: passive `{{ … }}` syntax (scan, lexer, parser, AST),
//!   never evaluated;
//! - [`pending`]: the `{{!data:v1:…}}` literal-token codec;
//! - [`simplified`]: the AST ([`SchemaRoot`], [`SchemaShape`],
//!   [`PropertyDef`], …), the type grammar, source maps, cursors, queries,
//!   serializers, subtyping, and JSON Schema lowering;
//! - [`resolve`]: named-type expansion, cycle checks, use-site merges, and
//!   layering over explicitly supplied sources ([`embedded::ShippedSources`]);
//! - [`claims`]: the string values a resolved schema claims for a property
//!   ([`claimed_values`]);
//! - [`coercion`] and [`coerce`]: the conversion engine and frontmatter
//!   conversion against a JSON Schema;
//! - [`validate`], [`format`], [`file_match`]: compiled validators, the
//!   `darkmatter-*` formats, and `x-darkmatter-*` keywords;
//! - [`prepared`] and [`outcome`]: a schema compiled once ([`Prepared`]) and
//!   its strict and soft validation, which return
//!   `Result<`[`ValidationSuccess`]`, `[`ValidationFailure`]`>`;
//! - [`schema`]: the [`SimplifiedSchema`] object, which builds from content
//!   or a file, validates strictly or softly without a compile step, layers,
//!   answers type queries, and exports;
//! - [`property_path`]: the dotted path grammar, its typed walk over a value,
//!   and the pure edits at a path;
//! - [`setter_value`]: the shared reader for a value written on a command line;
//! - [`SchemaError`], in [`errors`].
//!
//! A host (Darkmatter) keeps its own policy — baseline selection, schema
//! roots, trigger discovery, pending classification — and calls in here. See
//! the package's `docs/` for the crate boundary.

pub mod advisory;
pub mod claims;
pub mod coerce;
pub mod coercion;
pub mod completion;
pub mod discriminant;
pub mod embedded;
pub mod envelope;
pub mod errors;
pub mod example;
pub mod expression;
pub mod file_context;
pub mod file_match;
pub mod format;
pub mod frontmatter_shape;
pub mod json_shape;
pub mod mapping_orders;
pub mod numeric;
pub mod origin;
pub mod outcome;
pub mod pending;
pub mod phase;
pub mod positions;
pub mod prepared;
pub mod property_path;
pub mod reference;
pub mod resolve;
pub mod schema;
pub mod scope;
pub mod setter_value;
pub mod simplified;
pub mod slots;
pub mod span;
pub mod timings;
pub mod validate;
pub mod validation;
#[cfg(feature = "work-counters")]
pub mod work_counters;

/// Records one unit of `work_counters::Work`:
/// `note_work!(ExpressionParse)`. Expands to nothing without the
/// `work-counters` feature.
macro_rules! note_work {
    ($work:ident) => {
        #[cfg(feature = "work-counters")]
        $crate::work_counters::add($crate::work_counters::Work::$work, 1);
    };
}
pub(crate) use note_work;

pub use advisory::{SchemaAdvisory, SchemaAdvisoryKind};
pub use claims::{ClaimCause, ClaimProblem, claimed_values};
pub use discriminant::select_literal_discriminant_arm;
pub use errors::{
    CycleReference, DRAFT_2020_12_DIALECT, GrammarRelated, KindSelectionCause, NotSchemaReason, SchemaError, SlotResolutionFailure, SlotResultKind, UnresolvedSlots,
};
pub use frontmatter_shape::{
    FrontmatterExpressionValue, expression_atom, frontmatter_expression_values,
    frontmatter_expression_values_with_definitions, layered_property_shape, nested_property_shape,
    nested_property_shape_with_definitions, property_def_at_path, property_def_at_path_with_definitions,
};
pub use json_shape::shape_from_json_schema;
pub use mapping_orders::MappingOrders;
pub use origin::{SchemaOrigin, SchemaOriginKind, SchemaOriginMap};
pub use outcome::{ValidationFailure, ValidationSuccess};
pub use phase::SchemaPhase;
pub use prepared::Prepared;
pub use property_path::{
    PathEditError, PropertyPath, PropertyPathError, RemovalPlan, ResolveStop, Resolved, remove_at, set_at,
};
pub use reference::{SchemaReference, SchemaReferenceKind, classify_schema_reference};
pub use setter_value::{SetterValue, SetterValueError};
pub use schema::{
    FunctionView, PathSyntaxError, SchemaProvenance, SimplifiedSchema, SimplifiedSchemaError, TypeDefinition,
};
#[cfg(feature = "file")]
pub use schema::{SchemaSource, repository_scope_catalog};
#[cfg(feature = "work-counters")]
pub use simplified::alias_search_work;
pub use simplified::{
    Alternative, AlphanumericOption, AlphanumericOptions, AsciiCharSet, Constraint, DRAFT_2020_12, DecodedScalar,
    DynamicElement, DynamicKind, DynamicList, Slot, TemplateConstraintKind,
    DefKey, DefNamespace, Definition, DefinitionOuter, DefinitionTable, LocatedDefinitionFailure, LocatedRelated,
    PatternKey, PatternKeyDef, PropertyAtom, PropertyDef, SchemaArm, SchemaCategory, SchemaConstraintConflictKind,
    SchemaConstraintOrigin, SchemaConstraintParticipant, SchemaCursor, SchemaCursorRole, SchemaDeclaration,
    SchemaReferenceSite, SchemaRoot, SchemaShape, SchemaSourceId, SchemaSourceMap, SchemaSourcePath,
    SchemaSourcePathSegment, SchemaSpanKind, SchemaUnionRejection, SchemaValueEntry, SchemaValueKind,
    SchemaValueNode, SimplifiedType, SourceAware, StandaloneSchemaDocument, StandaloneSchemaEnvelope,
    SuggestionItem, SuggestionLintProblem, SuggestionLintReason, SuggestionQuery, TypeExpr,
    decode_alias_definition, decode_partial_scalar_at, decode_plain_scalar_node, decode_scalar, decode_scalar_at,
    decode_scalar_node, flattened_alternatives, flow_collection_end, is_definition_subtype,
    is_definition_subtype_with_definitions, is_string_like, is_subtype, is_subtype_with_definitions,
    is_union_arm_path, lint_suggestions, locate_schema_declaration_cursor,
    locate_schema_declaration_cursor_in_scalar, locate_schema_value, locate_standalone_definition_failure,
    locate_type_definition_cursor, locate_type_definition_cursor_in_scalar, mapping_separator,
    parse_property_definition, parse_property_definition_with_source, parse_schema_declaration,
    parse_type_definition_value,
    parse_schema_declaration_with_source, parse_selected_standalone_schema_document, parse_standalone_schema_document,
    parse_yaml_schema,
    suggestions_for_def, suggestions_for_path, to_json_schema, to_json_schema_with_definitions,
};
pub use span::{SourceSpan, Spanned, line_col_of_offset, line_of_offset};
pub use timings::{CompileCache, ValidationStage, ValidationTimings};
pub use validate::{CACHE_SIZE_ENV, DEFAULT_CACHE_SIZE, PositionMap, ValidatorCache};
pub use validation::{
    CallerFileReferenceProvenance, FileReferenceDiagnostic, JsonPointer, SCHEMA_CONSTRAINT_CODE,
    SCHEMA_INVALID_FILE_REFERENCE_CODE, SCHEMA_KIND_CHECK_CODE, SCHEMA_MISSING_REQUIRED_CODE, SCHEMA_PREPARE_CODE,
    SCHEMA_TYPE_MISMATCH_CODE, SCHEMA_UNKNOWN_KEY_CODE, ValidationProblem, ValidationProblemCode,
    ValidationProblemKind, authored_node_at,
};

/// Benchmarking seam: empties this crate's process-wide schema caches so the
/// next validation does its schema work cold.
///
/// Cleared: the shared, conversion, completion-alternative, and
/// example-target validator caches; the conversion-plan, format-validator,
/// example-artifact, emitted-pattern, named-type parse, and root-arm caches. A host
/// clears its own memos beside this call.
#[doc(hidden)]
pub fn reset_process_caches() {
    validate::SHARED_VALIDATOR_CACHE.clear();
    coerce::reset_process_caches();
    completion::reset_process_caches();
    example::reset_process_caches();
    json_shape::reset_process_caches();
    simplified::convert::reset_process_caches();
    resolve::reset_process_caches();
    prepared::reset_process_caches();
}
