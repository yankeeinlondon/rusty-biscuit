//! SimplifiedSchema parsing and AST.
//!
//! The two layers of the SimplifiedSchema parser live here:
//!
//! - [`types`] — AST nodes (`SimplifiedSchema`, `SchemaShape`, `SchemaArm`,
//!   `PropertyDef`, `PropertyAtom`, `SimplifiedType`, `Constraint`).
//! - [`grammar`] — string lexer/parser for the type-and-constraint mini DSL.
//!
//! This file glues them together with a YAML-shape layer that walks a
//! `serde_yaml_ng::Value` and produces a [`SimplifiedSchema`].

mod alphanumeric;
pub mod convert;
mod cursor;
mod definitions;
pub mod grammar;
mod lint;
pub mod query;
mod serialize;
mod signature;
mod source;
mod standalone;
mod subtype;
pub mod types;
pub mod yaml_scalar;

pub use alphanumeric::{AlphanumericOption, AlphanumericOptions, AsciiCharSet};
pub use subtype::{
    is_definition_subtype, is_definition_subtype_with_definitions, is_string_like, is_subtype, is_subtype_with_definitions,
};
pub use convert::{DARKMATTER_DEFERRED_KEYWORD, DARKMATTER_OPERANDS_KEYWORD, DARKMATTER_UNCOUNTED_KEYWORD, DRAFT_2020_12, to_json_schema, to_json_schema_with_definitions};
pub use definitions::{
    DefKey, DefNamespace, Definition, DefinitionOuter, DefinitionTable, definition_pointer,
};
pub use cursor::{
    SchemaCursor, SchemaCursorRole, is_union_arm_path, locate_schema_declaration_cursor,
    locate_schema_declaration_cursor_in_scalar, locate_type_definition_cursor,
    locate_type_definition_cursor_in_scalar,
};
pub use lint::{SuggestionLintProblem, SuggestionLintReason, lint_suggestions};
pub use query::{
    Alternative, SuggestionItem, SuggestionQuery, flattened_alternatives, suggestions_for_def,
    suggestions_for_path,
};
pub use serialize::{serialize_property_atom, serialize_property_def};
pub use source::{
    LocatedDefinitionFailure, SchemaConstraintConflictKind, SchemaConstraintOrigin,
    SchemaConstraintParticipant, SchemaReferenceSite, SchemaSourceId,
    SchemaSourceMap, SchemaSourcePath, SchemaSourcePathSegment, SchemaSpanKind, SchemaUnionRejection,
    SchemaValueEntry, SchemaValueKind, SchemaValueNode, SourceAware, flow_collection_end, locate_schema_value,
    locate_standalone_definition_failure,
    mapping_separator,
    parse_property_definition_with_source, parse_schema_declaration_with_source,
    parse_yaml_schema_with_source, project_suggestion_spans,
};
pub(crate) use source::locate_frontmatter_value;
pub use standalone::{
    SchemaCategory, StandaloneSchemaDocument, StandaloneSchemaEnvelope, parse_standalone_schema_document,
};
pub use types::{
    CallExample, Constraint, ExampleOutcome, FunctionParameter, FunctionParameters, FunctionType,
    LOCAL_REFERENCE, OperandForm, PatternKey, PatternKeyDef, PropertyAtom, PropertyDef, SchemaArm, SchemaShape,
    SimplifiedSchema, SimplifiedType, SuggestionCandidate, TupleElement, TupleType, TypeExpr, operands_verb,
};
#[cfg(feature = "work-counters")]
pub use yaml_scalar::alias_search_work;
pub use yaml_scalar::{
    DecodedScalar, decode_alias_definition, decode_partial_scalar_at, decode_scalar,
    decode_plain_scalar_node, decode_scalar_at, decode_scalar_node,
};

use indexmap::IndexMap;
use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::{
    errors::SchemaError,
    reference::{SchemaReference, classify_schema_reference},
};

/// The passive semantic product of one complete `$schema` declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaDeclaration {
    /// An inline mapping or non-empty root union parsed as SimplifiedSchema.
    Schema(SimplifiedSchema),
    /// One syntax-checked local file reference.
    Reference(SchemaReference),
}

/// Parses and classifies one complete `$schema` declaration without resolving
/// references or reading external state.
///
/// ## Errors
///
/// Returns a structured [`SchemaError`] for unsupported declaration shapes,
/// malformed local references, remote references, or invalid inline schema
/// grammar.
pub fn parse_schema_declaration(value: &YamlValue) -> Result<SchemaDeclaration, SchemaError> {
    parse_schema_declaration_located(value).map_err(SchemaError::from)
}

/// A semantic failure from the shared schema parser together with the
/// structural path of the node it rejected.
///
/// `path` is relative to the root of the value handed to the parse call (a
/// declaration, or one property definition); a caller that parsed a value
/// nested in a larger document prefixes it with [`SchemaSourcePath::join`].
/// Where the parser has not attributed a failure to a deeper node, `path`
/// names the nearest containing value, never a sibling. Source-free entry
/// points discard `path`, so their errors are unchanged.
#[derive(Debug)]
pub(crate) struct DefinitionFailure {
    pub(crate) path: SchemaSourcePath,
    /// Boxed to keep `Result<_, DefinitionFailure>` within clippy's
    /// `result_large_err` budget.
    pub(crate) error: Box<SchemaError>,
}

impl DefinitionFailure {
    fn at(path: &SchemaSourcePath) -> impl FnOnce(SchemaError) -> Self + '_ {
        move |error| Self { path: path.clone(), error: Box::new(error) }
    }
}

impl From<DefinitionFailure> for SchemaError {
    fn from(failure: DefinitionFailure) -> Self {
        *failure.error
    }
}

/// [`parse_schema_declaration`], keeping the rejected node's path.
pub(crate) fn parse_schema_declaration_located(
    value: &YamlValue,
) -> Result<SchemaDeclaration, DefinitionFailure> {
    let root = SchemaSourcePath::root();
    if let YamlValue::String(reference) = value {
        return classify_schema_reference(reference)
            .map(SchemaDeclaration::Reference)
            .map_err(DefinitionFailure::at(&root));
    }
    let schema = parse_yaml_schema_located(value)?;
    if let SimplifiedSchema::Union(arms) = &schema {
        for (index, arm) in arms.iter().enumerate() {
            if let SchemaArm::FileRef(reference) = arm {
                classify_schema_reference(reference)
                    .map_err(DefinitionFailure::at(&root.union_arm(index)))?;
            }
        }
    }
    Ok(SchemaDeclaration::Schema(schema))
}

/// Parses a `$schema` value (already extracted from frontmatter) into a
/// [`SimplifiedSchema`].
///
/// The expected YAML shapes are:
///
/// - **Mapping** → a single [`SchemaShape`].
/// - **Sequence whose items are all mappings or strings** → a root-level
///   union ([`SimplifiedSchema::Union`]). String items become
///   [`SchemaArm::FileRef`] arms; mapping items become [`SchemaArm::Inline`].
///
/// Anything else (including a string at the root) is rejected with
/// [`SchemaError::Grammar`].
///
/// File references found at the root are returned as `SchemaArm::FileRef`
/// without validation — the resolution layer (Phase 3) is responsible for
/// loading them.
pub fn parse_yaml_schema(value: &YamlValue) -> Result<SimplifiedSchema, SchemaError> {
    parse_yaml_schema_located(value).map_err(SchemaError::from)
}

/// [`parse_yaml_schema`], keeping the rejected node's path.
pub(crate) fn parse_yaml_schema_located(
    value: &YamlValue,
) -> Result<SimplifiedSchema, DefinitionFailure> {
    let root = SchemaSourcePath::root();
    match value {
        YamlValue::Mapping(_) => Ok(SimplifiedSchema::Single(parse_schema_shape(
            "<root>", value, 0, &root,
        )?)),
        YamlValue::Sequence(items) => {
            let mut arms = Vec::with_capacity(items.len());
            for (idx, item) in items.iter().enumerate() {
                let label = format!("<arm[{idx}]>");
                let arm_path = root.union_arm(idx);
                let arm = match item {
                    YamlValue::Mapping(_) => {
                        SchemaArm::Inline(parse_schema_shape(&label, item, 0, &arm_path)?)
                    }
                    YamlValue::String(s) => SchemaArm::FileRef(s.clone()),
                    other => {
                        return Err(DefinitionFailure::at(&arm_path)(SchemaError::Grammar {
                            property: label,
                            message: format!(
                                "root union arms must be a mapping or a file-reference string, got {}",
                                describe_yaml(other)
                            ),
                            span: 0..0,
                        }));
                    }
                };
                arms.push(arm);
            }
            if arms.is_empty() {
                return Err(DefinitionFailure::at(&root)(SchemaError::Grammar {
                    property: "<root>".into(),
                    message: "root union must have at least one arm".into(),
                    span: 0..0,
                }));
            }
            Ok(SimplifiedSchema::Union(arms))
        }
        other => Err(DefinitionFailure::at(&root)(SchemaError::Grammar {
            property: "<root>".into(),
            message: format!(
                "$schema must be a mapping or a sequence, got {}",
                describe_yaml(other)
            ),
            span: 0..0,
        })),
    }
}

/// Parses one property definition through the same YAML-shape and string
/// grammar used by [`parse_yaml_schema`].
///
/// This function is passive: it parses the supplied value without resolving
/// imports or file references and without reading external state.
///
/// ## Errors
///
/// Returns [`SchemaError::Grammar`] when `value` is not a valid scalar,
/// mapping, or non-empty union property definition.
pub fn parse_property_definition(
    property: &str,
    value: &YamlValue,
) -> Result<PropertyDef, SchemaError> {
    parse_property_definition_located(property, value).map_err(SchemaError::from)
}

/// [`parse_property_definition`], keeping the rejected node's path.
pub(crate) fn parse_property_definition_located(
    property: &str,
    value: &YamlValue,
) -> Result<PropertyDef, DefinitionFailure> {
    parse_property_definition_at_depth(property, value, 0, &SchemaSourcePath::root())
}

/// Parses one schema object whose structural path is `path`.
fn parse_schema_shape(
    context: &str,
    value: &YamlValue,
    nested_object_depth: usize,
    path: &SchemaSourcePath,
) -> Result<SchemaShape, DefinitionFailure> {
    let map = value.as_mapping().ok_or_else(|| {
        DefinitionFailure::at(path)(SchemaError::Grammar {
            property: context.to_string(),
            message: "expected a mapping of property names to type expressions".into(),
            span: 0..0,
        })
    })?;

    let mut properties: IndexMap<String, PropertyDef> = IndexMap::with_capacity(map.len());
    let mut pattern_keys: Vec<types::PatternKeyDef> = Vec::new();
    let mut constraints: Vec<Constraint> = Vec::new();
    for (key, val) in map {
        let key_str = key.as_str().ok_or_else(|| {
            DefinitionFailure::at(path)(SchemaError::Grammar {
                property: context.to_string(),
                message: "property names must be strings".into(),
                span: 0..0,
            })
        })?;
        // `$constraints` is reserved schema metadata (object arity, O-C3): it is
        // stripped before shape assembly and never participates in property
        // matching. It is reserved only inside authored schema objects, so it
        // imposes no restriction on user frontmatter data.
        if key_str == "$constraints" {
            constraints.append(&mut parse_dollar_constraints(
                context,
                val,
                &path.property(key_str),
            )?);
            continue;
        }
        // Pattern / dictionary keys (`<string>`, `<starting::…>`, …) collect
        // separately from literal properties (Feature C).
        if types::PatternKey::is_pattern_key(key_str) {
            // A malformed key is attributed to the object holding it.
            let pattern = types::PatternKey::parse(key_str).map_err(|message| {
                DefinitionFailure::at(path)(SchemaError::Grammar {
                    property: context.to_string(),
                    message,
                    span: 0..0,
                })
            })?;
            let def = parse_property_definition_at_depth(
                key_str,
                val,
                nested_object_depth,
                &path.property(key_str),
            )?;
            pattern_keys.push(types::PatternKeyDef { key: pattern, def });
            continue;
        }
        let def = parse_property_definition_at_depth(
            key_str,
            val,
            nested_object_depth,
            &path.property(key_str),
        )?;
        properties.insert(key_str.to_string(), def);
    }
    Ok(SchemaShape {
        properties,
        pattern_keys,
        constraints,
    })
}

/// Parses a reserved `$constraints` metadata mapping (block-form object arity,
/// O-C3) into canonical [`Constraint`]s — the same variants the postfix
/// `(min-keys(1); …)` form produces. Numeric entries carry an integer argument;
/// flag entries are written `name: true`.
///
/// `path` is the `$constraints` value's own path; a failing entry is
/// attributed to `path.property(key)`.
fn parse_dollar_constraints(
    context: &str,
    value: &YamlValue,
    path: &SchemaSourcePath,
) -> Result<Vec<Constraint>, DefinitionFailure> {
    let map = value.as_mapping().ok_or_else(|| {
        DefinitionFailure::at(path)(SchemaError::Grammar {
            property: context.to_string(),
            message: "`$constraints` must be a mapping of constraint keywords to values".into(),
            span: 0..0,
        })
    })?;
    let mut out = Vec::with_capacity(map.len());
    for (k, v) in map {
        let key = k.as_str().ok_or_else(|| {
            DefinitionFailure::at(path)(SchemaError::Grammar {
                property: context.to_string(),
                message: "`$constraints` keys must be strings".into(),
                span: 0..0,
            })
        })?;
        out.push(
            parse_dollar_constraint(context, key, v).map_err(DefinitionFailure::at(&path.property(key)))?,
        );
    }
    Ok(out)
}

/// One `$constraints` entry.
fn parse_dollar_constraint(context: &str, key: &str, v: &YamlValue) -> Result<Constraint, SchemaError> {
    let constraint = match key {
        "min-keys" => Constraint::MinKeys(constraints_usize(context, key, v)?),
        "max-keys" => Constraint::MaxKeys(constraints_usize(context, key, v)?),
        "uncounted-keys" => Constraint::UncountedKeys(constraints_keys(context, key, v)?),
        "min-items" => Constraint::MinItems(constraints_usize(context, key, v)?),
        "max-items" => Constraint::MaxItems(constraints_usize(context, key, v)?),
        "required" => {
            constraints_flag(context, key, v)?;
            Constraint::Required
        }
        "not-empty" => {
            constraints_flag(context, key, v)?;
            Constraint::NotEmpty
        }
        "unique" => {
            constraints_flag(context, key, v)?;
            Constraint::Unique
        }
        "generated" => {
            constraints_flag(context, key, v)?;
            Constraint::Generated
        }
        "integer" => {
            constraints_flag(context, key, v)?;
            Constraint::Integer
        }
        other => {
            return Err(SchemaError::Grammar {
                property: context.to_string(),
                message: format!("unknown `$constraints` key `{other}`"),
                span: 0..0,
            });
        }
    };
    Ok(constraint)
}

fn constraints_usize(context: &str, key: &str, value: &YamlValue) -> Result<usize, SchemaError> {
    value
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| SchemaError::Grammar {
            property: context.to_string(),
            message: format!("`$constraints.{key}` requires a non-negative integer"),
            span: 0..0,
        })
}

fn constraints_keys(context: &str, key: &str, value: &YamlValue) -> Result<Vec<String>, SchemaError> {
    let grammar = |message: String| SchemaError::Grammar {
        property: context.to_string(),
        message,
        span: 0..0,
    };
    let keys = value
        .as_sequence()
        .ok_or_else(|| grammar(format!("`$constraints.{key}` requires a list of key names")))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| grammar(format!("`$constraints.{key}` lists key names, which are strings")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    types::check_uncounted_keys(&keys).map_err(grammar)?;
    Ok(keys)
}

fn constraints_flag(context: &str, key: &str, value: &YamlValue) -> Result<(), SchemaError> {
    if value.as_bool() == Some(true) {
        Ok(())
    } else {
        Err(SchemaError::Grammar {
            property: context.to_string(),
            message: format!("`$constraints.{key}` is a flag constraint and must be `true`"),
            span: 0..0,
        })
    }
}

/// Parses one property definition whose structural path is `path`.
///
/// A failure names the rejected node: a nested mapping's key, the YAML index
/// of a property-level union arm (each sequence item is exactly one arm), or
/// `path` itself for a scalar definition or a structural failure of the
/// whole value.
fn parse_property_definition_at_depth(
    name: &str,
    value: &YamlValue,
    nested_object_depth: usize,
    path: &SchemaSourcePath,
) -> Result<PropertyDef, DefinitionFailure> {
    match value {
        YamlValue::String(s) => grammar::parse_type_definition(name, s).map_err(DefinitionFailure::at(path)),
        // The YAML null scalar (`null`, `Null`, `NULL`, `~`, or an empty
        // value) at a type position is the `null` type, as is the string
        // `"null"`. A root `$schema` or root-union arm never reaches here.
        YamlValue::Null => Ok(PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Null))),
        // A YAML mapping at a property position lowers to an inline object
        // shape: each entry's value is recursively parsed through the same
        // property-definition entry point, so nested mappings, sequence unions, and
        // string type expressions all work. A mapping without an explicit
        // `type:` key is an object shape, not a future long-form descriptor
        // (spec §"Nested Object Syntax Requirement" rule 6).
        YamlValue::Mapping(_) => Ok(PropertyDef::Single(parse_mapping_atom(
            name,
            value,
            nested_object_depth,
            path,
        )?)),
        YamlValue::Sequence(items) => {
            let mut arms = Vec::with_capacity(items.len());
            for (idx, item) in items.iter().enumerate() {
                let arm_path = path.union_arm(idx);
                match item {
                    // A list option is one type; `A | B` inside a list would
                    // be a second spelling of the same union.
                    YamlValue::String(s) => match grammar::parse_type_definition(name, s)
                        .map_err(DefinitionFailure::at(&arm_path))?
                    {
                        PropertyDef::Single(atom) => arms.push(atom),
                        PropertyDef::Union(_) => {
                            return Err(DefinitionFailure::at(&arm_path)(SchemaError::Grammar {
                                property: name.to_string(),
                                message: format!(
                                    "property-level union arm[{idx}] is itself a `|` union; list each option separately"
                                ),
                                span: 0..s.len(),
                            }));
                        }
                    },
                    YamlValue::Null => arms.push(PropertyAtom::bare(SimplifiedType::Null)),
                    // A mapping arm lowers to an inline object shape, so a
                    // sequence union may mix type-expression strings with
                    // nested mapping object shapes (spec rule 4).
                    YamlValue::Mapping(_) => {
                        let label = format!("{name}[{idx}]");
                        arms.push(parse_mapping_atom(
                            &label,
                            item,
                            nested_object_depth,
                            &arm_path,
                        )?);
                    }
                    other => {
                        return Err(DefinitionFailure::at(&arm_path)(SchemaError::Grammar {
                            property: name.to_string(),
                            message: format!(
                                "property-level union arm[{idx}] must be a string type expression, a nested mapping object, or null, got {}",
                                describe_yaml(other)
                            ),
                            span: 0..0,
                        }));
                    }
                }
            }
            // `arms[i]` came from `items[i]`, so an index into `arms` is the
            // YAML index of the arm that introduced a cross-arm violation.
            if arms.is_empty() {
                return Err(DefinitionFailure::at(path)(SchemaError::Grammar {
                    property: name.to_string(),
                    message: "property-level union must have at least one arm".into(),
                    span: 0..0,
                }));
            }
            if let Some((idx, span)) = grammar::second_suggestion_span(&arms) {
                return Err(DefinitionFailure::at(&path.union_arm(idx))(SchemaError::Grammar {
                    property: name.to_string(),
                    message: "a property definition may contain at most one `suggest` constraint"
                        .into(),
                    span,
                }));
            }
            if let Some((idx, message)) = signature::duplicate_overload_arm(&arms, None) {
                return Err(DefinitionFailure::at(&path.union_arm(idx))(SchemaError::Grammar {
                    property: name.to_string(),
                    message,
                    span: 0..0,
                }));
            }
            Ok(PropertyDef::Union(arms))
        }
        other => Err(DefinitionFailure::at(path)(SchemaError::Grammar {
            property: name.to_string(),
            message: format!(
                "property value must be a string type expression, a nested mapping object, null, or a sequence of expressions, got {}",
                describe_yaml(other)
            ),
            span: 0..0,
        })),
    }
}

/// Parses a mapping at a property or union-arm position, whose structural
/// path is `path`, as an inline object atom.
///
/// A mapping nested deeper than [`grammar::MAX_INLINE_OBJECT_DEPTH`] is
/// rejected at `path`.
fn parse_mapping_atom(
    context: &str,
    value: &YamlValue,
    nested_object_depth: usize,
    path: &SchemaSourcePath,
) -> Result<PropertyAtom, DefinitionFailure> {
    if nested_object_depth >= grammar::MAX_INLINE_OBJECT_DEPTH {
        return Err(DefinitionFailure::at(path)(SchemaError::Grammar {
            property: context.to_string(),
            message: format!(
                "schema objects may not nest more than {} levels deep",
                grammar::MAX_INLINE_OBJECT_DEPTH,
            ),
            span: 0..0,
        }));
    }
    let mut shape = parse_schema_shape(context, value, nested_object_depth + 1, path)?;
    // A block `required` / `generated` on a property's object means the same
    // as the postfix on that property, so it moves onto the atom where every
    // presence consumer (`is_required`, phase projection, lowering) reads it.
    let (presence, block): (Vec<_>, Vec<_>) = shape
        .constraints
        .into_iter()
        .partition(|c| matches!(c, Constraint::Required | Constraint::Generated));
    shape.constraints = block;
    let mut atom = PropertyAtom::bare_inline_object(shape);
    atom.constraints = presence;
    Ok(atom)
}

fn describe_yaml(value: &YamlValue) -> &'static str {
    match value {
        YamlValue::Null => "null",
        YamlValue::Bool(_) => "boolean",
        YamlValue::Number(_) => "number",
        YamlValue::String(_) => "string",
        YamlValue::Sequence(_) => "sequence",
        YamlValue::Mapping(_) => "mapping",
        YamlValue::Tagged(_) => "tagged-value",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(input: &str) -> YamlValue {
        serde_yaml_ng::from_str(input).expect("yaml parse failed")
    }

    #[test]
    fn parses_single_shape() {
        let v = yaml("name: string\nage: number");
        let schema = parse_yaml_schema(&v).unwrap();
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single"),
        };
        assert_eq!(shape.properties.len(), 2);
        assert!(shape.properties.contains_key("name"));
        assert!(shape.properties.contains_key("age"));
    }

    #[test]
    fn preserves_property_order() {
        let v = yaml("zeta: string\nalpha: number\nmiddle: boolean");
        let schema = parse_yaml_schema(&v).unwrap();
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single"),
        };
        let keys: Vec<&str> = shape.properties.keys().map(String::as_str).collect();
        assert_eq!(keys, vec!["zeta", "alpha", "middle"]);
    }

    #[test]
    fn parses_property_level_union() {
        let v = yaml("foo:\n  - string\n  - number");
        let schema = parse_yaml_schema(&v).unwrap();
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single"),
        };
        let foo = shape.properties.get("foo").unwrap();
        match foo {
            PropertyDef::Union(arms) => {
                assert_eq!(arms.len(), 2);
                assert_eq!(arms[0].ty, TypeExpr::Primitive(SimplifiedType::String));
                assert_eq!(arms[1].ty, TypeExpr::Primitive(SimplifiedType::Number));
            }
            _ => panic!("expected Union"),
        }
    }

    #[test]
    fn accepts_property_level_union_with_mapping_arm() {
        // A mapping arm in a property-level union lowers to an inline object
        // shape (spec §"Nested Object Syntax Requirement" rule 4).
        let v = yaml("foo:\n  - string\n  - bar: number");
        let schema = parse_yaml_schema(&v).unwrap();
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single"),
        };
        let foo = shape.properties.get("foo").unwrap();
        match foo {
            PropertyDef::Union(arms) => {
                assert_eq!(arms.len(), 2);
                assert_eq!(arms[0].ty, TypeExpr::Primitive(SimplifiedType::String));
                match &arms[1].ty {
                    TypeExpr::InlineObject(inner) => {
                        assert!(inner.properties.contains_key("bar"));
                    }
                    other => panic!("expected InlineObject for mapping arm, got {other:?}"),
                }
            }
            _ => panic!("expected Union"),
        }
    }

    #[test]
    fn parses_root_union_inline() {
        let v = yaml("- title: string\n  body: string\n- name: string");
        let schema = parse_yaml_schema(&v).unwrap();
        match schema {
            SimplifiedSchema::Union(arms) => {
                assert_eq!(arms.len(), 2);
                assert!(matches!(arms[0], SchemaArm::Inline(_)));
                assert!(matches!(arms[1], SchemaArm::Inline(_)));
            }
            _ => panic!("expected Union"),
        }
    }

    #[test]
    fn parses_root_union_file_refs() {
        let v = yaml("- ./schemas/post.yaml\n- ./schemas/page.yaml");
        let schema = parse_yaml_schema(&v).unwrap();
        match schema {
            SimplifiedSchema::Union(arms) => {
                assert_eq!(arms.len(), 2);
                match &arms[0] {
                    SchemaArm::FileRef(p) => assert_eq!(p, "./schemas/post.yaml"),
                    _ => panic!("expected FileRef"),
                }
            }
            _ => panic!("expected Union"),
        }
    }

    #[test]
    fn parses_mixed_root_union() {
        let v = yaml("- ./schemas/post.yaml\n- title: string");
        let schema = parse_yaml_schema(&v).unwrap();
        match schema {
            SimplifiedSchema::Union(arms) => {
                assert_eq!(arms.len(), 2);
                assert!(matches!(arms[0], SchemaArm::FileRef(_)));
                assert!(matches!(arms[1], SchemaArm::Inline(_)));
            }
            _ => panic!("expected Union"),
        }
    }

    #[test]
    fn rejects_root_string_scalar() {
        let v = yaml("\"./schemas/post.yaml\"");
        let err = parse_yaml_schema(&v).unwrap_err();
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("mapping or a sequence"));
    }

    #[test]
    fn accepts_property_value_mapping_as_nested_object() {
        // A mapping at a property position lowers to an inline object shape
        // (spec §"Nested Object Syntax Requirement" rule 1). The leaf values
        // go through the same type-expression grammar, so `baz` here is the
        // primitive `string` type.
        let v = yaml("foo:\n  bar: string");
        let schema = parse_yaml_schema(&v).unwrap();
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single"),
        };
        let foo = shape.properties.get("foo").unwrap();
        let atom = match foo {
            PropertyDef::Single(a) => a,
            other => panic!("expected Single atom, got {other:?}"),
        };
        let inner = match &atom.ty {
            TypeExpr::InlineObject(s) => s,
            other => panic!("expected InlineObject, got {other:?}"),
        };
        let bar = match inner.properties.get("bar") {
            Some(PropertyDef::Single(a)) => a,
            other => panic!("expected Single `bar`, got {other:?}"),
        };
        assert_eq!(bar.ty, TypeExpr::Primitive(SimplifiedType::String));
    }

    #[test]
    fn rejects_empty_root_union() {
        let v: YamlValue = serde_yaml_ng::from_str("[]").unwrap();
        let err = parse_yaml_schema(&v).unwrap_err();
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("at least one arm"));
    }

    #[test]
    fn property_grammar_errors_propagate_with_property_name() {
        let v = yaml("title: string(min(nope))");
        let err = parse_yaml_schema(&v).unwrap_err();
        let SchemaError::Grammar { property, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert_eq!(property, "title");
    }

    // ── Phase 1 grammar-extension coverage ───────────────────────────────

    /// Spec test 8 (Phase 1): a nested mapping at a property position must
    /// lower to the same `SchemaShape` as the equivalent quoted inline
    /// object literal `{ foo: string, bar: number }`.
    #[test]
    fn nested_mapping_lowers_to_same_shape_as_inline_literal() {
        let nested = yaml("obj:\n  foo: string\n  bar: number");
        let inline = yaml(r#"obj: "{ foo: string, bar: number }""#);
        let nested_schema = parse_yaml_schema(&nested).unwrap();
        let inline_schema = parse_yaml_schema(&inline).unwrap();
        let nested_shape = match &nested_schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single for nested form"),
        };
        let inline_shape = match &inline_schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single for inline form"),
        };
        // Both top-level shapes must declare exactly `obj`, and the two
        // `obj` definitions must be equal atom-for-atom (including the
        // nested SchemaShape).
        assert_eq!(nested_shape, inline_shape);
    }

    /// Spec rule 3: nesting is recursive. A mapping inside a mapping inside
    /// a mapping must lower all the way down to typed leaf atoms.
    #[test]
    fn nested_mapping_recurses() {
        let v = yaml("outer:\n  mid:\n    leaf: number");
        let schema = parse_yaml_schema(&v).unwrap();
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single"),
        };
        let outer = shape.properties.get("outer").unwrap();
        let outer_atom = match outer {
            PropertyDef::Single(a) => a,
            other => panic!("expected Single atom, got {other:?}"),
        };
        let mid_shape = match &outer_atom.ty {
            TypeExpr::InlineObject(s) => s,
            other => panic!("expected InlineObject, got {other:?}"),
        };
        let mid = mid_shape.properties.get("mid").unwrap();
        let mid_atom = match mid {
            PropertyDef::Single(a) => a,
            other => panic!("expected Single atom, got {other:?}"),
        };
        let leaf_shape = match &mid_atom.ty {
            TypeExpr::InlineObject(s) => s,
            other => panic!("expected InlineObject, got {other:?}"),
        };
        let leaf = match leaf_shape.properties.get("leaf") {
            Some(PropertyDef::Single(a)) => a,
            other => panic!("expected Single `leaf`, got {other:?}"),
        };
        assert_eq!(leaf.ty, TypeExpr::Primitive(SimplifiedType::Number));
    }

    /// Spec rule 4: a sequence union arm may be a nested mapping object.
    /// Combined with leaf descriptions and constraints, this covers the
    /// motivating `hash` example from the spec.
    #[test]
    fn sequence_union_with_mapping_arm_supports_constraints_and_descriptions() {
        let v = yaml(
            "hash:\n  - \"string -> shorthand hash\"\n  - kind: \"enum(simple, structured, detailed; default(simple))\"\n    value: any\n    ignored: \"string[]\"",
        );
        let schema = parse_yaml_schema(&v).unwrap();
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single"),
        };
        let hash = shape.properties.get("hash").unwrap();
        let arms = match hash {
            PropertyDef::Union(arms) => arms,
            other => panic!("expected Union, got {other:?}"),
        };
        assert_eq!(arms.len(), 2);
        // First arm: a string type with a description.
        assert_eq!(
            arms[0].ty,
            TypeExpr::Primitive(SimplifiedType::String)
        );
        assert_eq!(arms[0].description.as_deref(), Some("shorthand hash"));
        // Second arm: an inline object with three properties.
        let obj_shape = match &arms[1].ty {
            TypeExpr::InlineObject(s) => s,
            other => panic!("expected InlineObject for mapping arm, got {other:?}"),
        };
        assert_eq!(obj_shape.properties.len(), 3);
        let kind = match obj_shape.properties.get("kind") {
            Some(PropertyDef::Single(a)) => a,
            other => panic!("expected Single `kind`, got {other:?}"),
        };
        assert_eq!(kind.ty, TypeExpr::Primitive(SimplifiedType::Enum));
    }

    /// Spec rule 2: leaf values inside a nested mapping use the full
    /// type-expression grammar, including `-> description` and
    /// `default(...)`.
    #[test]
    fn nested_mapping_leaf_uses_full_type_expression_grammar() {
        let v = yaml(
            "obj:\n  described: \"string -> leaf description\"\n  defaulted: \"boolean(default(false))\"",
        );
        let schema = parse_yaml_schema(&v).unwrap();
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single"),
        };
        let obj = shape.properties.get("obj").unwrap();
        let atom = match obj {
            PropertyDef::Single(a) => a,
            other => panic!("expected Single atom, got {other:?}"),
        };
        let inner = match &atom.ty {
            TypeExpr::InlineObject(s) => s,
            other => panic!("expected InlineObject, got {other:?}"),
        };
        let described = match inner.properties.get("described") {
            Some(PropertyDef::Single(a)) => a,
            other => panic!("expected Single `described`, got {other:?}"),
        };
        assert_eq!(described.description.as_deref(), Some("leaf description"));
        let defaulted = match inner.properties.get("defaulted") {
            Some(PropertyDef::Single(a)) => a,
            other => panic!("expected Single `defaulted`, got {other:?}"),
        };
        assert_eq!(
            defaulted.constraints,
            vec![Constraint::Default(serde_json::Value::Bool(false))]
        );
    }

    /// Spec rule 6 (parity with future long-form): a mapping without an
    /// explicit `type:` key is treated as an object shape. A mapping that
    /// happens to use `type` as a property name is still treated as an
    /// object shape, not a long-form descriptor.
    #[test]
    fn nested_mapping_with_type_key_is_object_shape_not_descriptor() {
        let v = yaml("obj:\n  type: string\n  value: any");
        let schema = parse_yaml_schema(&v).unwrap();
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single"),
        };
        let obj = shape.properties.get("obj").unwrap();
        let atom = match obj {
            PropertyDef::Single(a) => a,
            other => panic!("expected Single atom, got {other:?}"),
        };
        let inner = match &atom.ty {
            TypeExpr::InlineObject(s) => s,
            other => panic!("expected InlineObject, got {other:?}"),
        };
        // `type` is a regular property inside the object shape, not a
        // long-form descriptor switch.
        assert!(inner.properties.contains_key("type"));
        assert!(inner.properties.contains_key("value"));
    }

    /// The motivating example: a nested mapping that uses `generated` on
    /// its leaf values. This is the form `darkmatter.yaml` uses for `ctx.*`.
    #[test]
    fn nested_mapping_supports_generated_constraint() {
        let v = yaml("ctx:\n  today: \"date(generated; required) -> today's date\"");
        let schema = parse_yaml_schema(&v).unwrap();
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            _ => panic!("expected Single"),
        };
        let ctx = shape.properties.get("ctx").unwrap();
        let atom = match ctx {
            PropertyDef::Single(a) => a,
            other => panic!("expected Single atom, got {other:?}"),
        };
        let inner = match &atom.ty {
            TypeExpr::InlineObject(s) => s,
            other => panic!("expected InlineObject, got {other:?}"),
        };
        let today = match inner.properties.get("today") {
            Some(PropertyDef::Single(a)) => a,
            other => panic!("expected Single `today`, got {other:?}"),
        };
        assert_eq!(today.ty, TypeExpr::Primitive(SimplifiedType::Date));
        assert_eq!(
            today.constraints,
            vec![Constraint::Generated, Constraint::Required]
        );
        assert_eq!(today.description.as_deref(), Some("today's date"));
    }

    /// The draft baseline schema lives in the repo and uses both grammar
    /// extensions delivered in Phase 1. This is a smoke test that it parses
    /// cleanly so a regression in either extension is caught immediately.
    #[test]
    fn darkmatter_baseline_schema_yaml_parses() {
        let raw = include_str!("../../../../../schemas/partials/fm.yaml");
        let frontmatter: YamlValue =
            serde_yaml_ng::from_str(raw).expect("baseline yaml must parse");
        // The `fm` partial is a types-only library: the baseline body is its
        // `fm` type, not the top-level mapping itself.
        let schema_value = frontmatter
            .get("types")
            .and_then(|types| types.get("fm"))
            .expect("the fm partial must declare the `fm` type");
        let schema = parse_yaml_schema(schema_value).expect("baseline schema must parse");
        let shape = match schema {
            SimplifiedSchema::Single(s) => s,
            other => panic!("expected Single, got {other:?}"),
        };
        // Spot-check a handful of properties the spec calls out as
        // baseline-owned. The full property-list audit is Phase 3.
        for key in [
            "$schema",
            "title",
            "description",
            "tags",
            "draft",
            "metadata",
            "last_updated",
            "hash",
            "style",
            "change",
            "replace",
            "ctx",
            "prologue",
            "epilogue",
            "ignore_invalid",
            "interpolate_code_blocks",
        ] {
            assert!(shape.properties.contains_key(key), "missing baseline key: {key}");
        }
        // Non-Goal 6: the deprecated root-level `hr` frontmatter property is
        // deliberately excluded from the baseline; `style.hr.*` is the only
        // horizontal-rule surface.
        assert!(
            !shape.properties.contains_key("hr"),
            "deprecated root-level `hr` must not appear in the baseline schema"
        );
    }

    /// The frozen baseline schema must convert to a baseline-compatible
    /// Draft 2020-12 JSON Schema without errors (spec testing requirement 2;
    /// Phase 3 validation checkpoint). The converter exercises every baseline
    /// property atom and keeps broad object surfaces open where runtime parsers
    /// remain authoritative. The baseline imports named types, so it converts
    /// only after preparation expands them.
    #[test]
    fn darkmatter_baseline_schema_converts_to_json_schema() {
        let raw = include_str!("../../../../../schemas/partials/fm.yaml");
        assert!(raw.contains("content_policy: "), "the baseline declares an import");
        let json = crate::markdown::schemas::darkmatter_base_json_schema();
        let obj = json.as_object().expect("root must be a JSON object");
        assert_eq!(
            obj.get("type").and_then(|v| v.as_str()),
            Some("object"),
            "baseline root must be an object schema"
        );
        let properties = obj
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("baseline must emit a properties map");
        // Every baseline-owned key survives the conversion.
        for key in [
            "$schema",
            "title",
            "ctx",
            "style",
            "hash",
            "prologue",
            "epilogue",
            "replace",
            "ignore_invalid",
            "interpolate_code_blocks",
        ] {
            assert!(
                properties.contains_key(key),
                "converted schema missing property `{key}`"
            );
        }
        // Non-Goal 6: the deprecated root-level `hr` is not a baseline surface.
        assert!(
            !properties.contains_key("hr"),
            "deprecated root-level `hr` must not appear in the converted baseline"
        );
        // `ctx` is generated by the host and must not become a statically
        // required authored-frontmatter property.
        if let Some(required) = obj.get("required").and_then(|v| v.as_array()) {
            assert!(
                !required.iter().any(|v| v.as_str() == Some("ctx")),
                "`ctx` must not appear in the root required array"
            );
        }
    }

    /// The baseline schema models `ctx` as Darkmatter-owned generated runtime
    /// context. Authored documents may omit `ctx`, but arbitrary custom ctx
    /// keys are not part of the base-schema contract.
    #[test]
    fn darkmatter_baseline_schema_ctx_reflects_runtime_semantics() {
        use serde_json::{Value, json};

        let json = crate::markdown::schemas::darkmatter_base_json_schema();

        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .build(&json)
            .expect("converted schema must build a validator");

        fn assert_valid(validator: &jsonschema::Validator, doc: Value, message: &str) {
            assert!(validator.is_valid(&doc), "{message}");
        }

        fn assert_invalid(validator: &jsonschema::Validator, doc: Value, message: &str) {
            assert!(!validator.is_valid(&doc), "{message}");
        }

        assert_valid(
            &validator,
            json!({}),
            "authored documents may omit generated ctx",
        );
        assert_valid(
            &validator,
            json!({ "ctx": { "today": "2026-07-04" } }),
            "known generated ctx leaves must validate when present",
        );
        assert_invalid(
            &validator,
            json!({ "ctx": { "project_slug": "biscuit" } }),
            "custom user ctx keys are not part of the generated context schema",
        );
        assert_invalid(&validator, json!({ "ctx": "hello" }), "ctx must remain an object");
    }

    /// The located parse returns the source-free parser's error unchanged,
    /// plus the structural path of the value it rejected.
    fn declaration_failure(input: &str) -> DefinitionFailure {
        let value = yaml(input);
        let failure = parse_schema_declaration_located(&value).expect_err("declaration must fail");
        let plain = parse_schema_declaration(&value).expect_err("declaration must fail");
        assert_eq!(format!("{:?}", failure.error), format!("{plain:?}"), "source-free error drifted");
        failure
    }

    /// [`declaration_failure`] for one property definition.
    fn property_failure(input: &str) -> DefinitionFailure {
        let value = yaml(input);
        let failure = parse_property_definition_located("colour", &value).expect_err("must fail");
        let plain = parse_property_definition("colour", &value).expect_err("must fail");
        assert_eq!(format!("{:?}", failure.error), format!("{plain:?}"), "source-free error drifted");
        failure
    }

    fn root() -> SchemaSourcePath {
        SchemaSourcePath::root()
    }

    #[test]
    fn definition_failure_names_a_literal_key_as_one_segment() {
        let failure = declaration_failure("ok: string\n\"a.b[0]/ü\": string(min(\n");
        assert_eq!(failure.path, SchemaSourcePath::root().property("a.b[0]/ü"));
        assert!(matches!(*failure.error, SchemaError::Grammar { .. }), "{:?}", failure.error);
    }

    #[test]
    fn definition_failure_indexes_root_union_arms() {
        let failure = declaration_failure("- name: string\n- 7\n");
        assert_eq!(failure.path, SchemaSourcePath::root().union_arm(1));
        let failure = declaration_failure("- name: string\n- title: nope(\n");
        assert_eq!(failure.path, SchemaSourcePath::root().union_arm(1).property("title"));
    }

    #[test]
    fn definition_failure_addresses_dollar_constraints_and_structural_roots() {
        let failure = declaration_failure("name: string\n$constraints:\n  min-keys: -1\n");
        assert_eq!(failure.path, root().property("$constraints").property("min-keys"));
        let failure = declaration_failure("name: string\n$constraints: [1]\n");
        assert_eq!(failure.path, root().property("$constraints"));
        let failure = declaration_failure("tags:\n  a: string\n  $constraints:\n    nope: true\n");
        assert_eq!(failure.path, root().property("tags").property("$constraints").property("nope"));
        // A structural failure without a scalar token names the value itself.
        assert_eq!(declaration_failure("[]").path, SchemaSourcePath::root());
        assert_eq!(declaration_failure("7").path, SchemaSourcePath::root());
    }

    #[test]
    fn property_definition_failure_starts_at_the_definition() {
        assert_eq!(property_failure("string(min(").path, root());
        assert_eq!(property_failure("7").path, root());
        assert_eq!(property_failure("[string, \"number(max(\"]").path, root().union_arm(1));
        assert_eq!(property_failure("[{a: \"x(\"}]").path, root().union_arm(0).property("a"));
    }

    #[test]
    fn definition_failure_names_nested_literal_keys() {
        let failure = declaration_failure("outer:\n  \"a.b\":\n    \"c[0]/ü\": \"string(min(\"\n");
        assert_eq!(failure.path, root().property("outer").property("a.b").property("c[0]/ü"));
        let failure = declaration_failure("outer:\n  \"x/y\": [string, {\"[z]\": 7}]\n");
        assert_eq!(
            failure.path,
            root().property("outer").property("x/y").union_arm(1).property("[z]")
        );
    }

    #[test]
    fn definition_failure_indexes_property_union_members() {
        let failure = declaration_failure("tags: [string, {x: \"bogus(\"}]\n");
        assert_eq!(failure.path, root().property("tags").union_arm(1).property("x"));
        // The second member shares the first member's prefix; only its index
        // tells them apart.
        let failure = declaration_failure("tags: [string, \"string(min(\"]\n");
        assert_eq!(failure.path, root().property("tags").union_arm(1));
        let failure = declaration_failure("tags: [string, 7]\n");
        assert_eq!(failure.path, root().property("tags").union_arm(1));
        let failure = declaration_failure("tags: [string, \"number | boolean\"]\n");
        assert_eq!(failure.path, root().property("tags").union_arm(1));
        // An empty union has no member to name.
        assert_eq!(declaration_failure("tags: []\n").path, root().property("tags"));
    }

    #[test]
    fn definition_failure_names_the_second_suggestion_arm() {
        let source = "string(suggest(blue))";
        let value = yaml(&format!("colour: [string(suggest(red)), \"{source}\"]\n"));
        let failure = parse_schema_declaration_located(&value).expect_err("must fail");
        let plain = parse_schema_declaration(&value).expect_err("must fail");
        assert_eq!(format!("{:?}", failure.error), format!("{plain:?}"));
        assert_eq!(failure.path, root().property("colour").union_arm(1));
        let SchemaError::Grammar { span, .. } = *failure.error else {
            panic!("expected a grammar error, got {:?}", failure.error);
        };
        assert_eq!(&source[span], "blue", "span is relative to arm 1's own scalar");
    }

    #[test]
    fn definition_failure_names_the_later_duplicate_overload() {
        let failure = declaration_failure(
            "call:\n  - \"function(parameters(a: string); returns(string))\"\n  - \"function(parameters(a: number); returns(string))\"\n  - \"function(parameters(b: string); returns(number))\"\n",
        );
        assert_eq!(failure.path, root().property("call").union_arm(2));
        assert!(failure.error.to_string().contains("overloads 1 and 3"), "{}", failure.error);
    }

    #[test]
    fn definition_failure_names_nested_root_union_arm_keys() {
        let failure = declaration_failure("- {a: {b: \"x(\"}}\n");
        assert_eq!(failure.path, root().union_arm(0).property("a").property("b"));
    }

    #[test]
    fn definition_failure_names_the_mapping_past_the_depth_limit() {
        let levels = grammar::MAX_INLINE_OBJECT_DEPTH + 1;
        let input = format!("{}x: string{}", "k: {".repeat(levels), "}".repeat(levels));
        let failure = declaration_failure(&input);
        let expected = (0..levels).fold(root(), |path, _| path.property("k"));
        assert_eq!(failure.path, expected);
        assert!(failure.error.to_string().contains("levels deep"), "{}", failure.error);
    }

    #[test]
    fn failure_paths_join_under_an_envelope_prefix() {
        let failure = declaration_failure("inner.key: string(min(\n");
        let envelope = SchemaSourcePath::root().property("types").property("slug");
        assert_eq!(
            envelope.join(&failure.path),
            SchemaSourcePath::root().property("types").property("slug").property("inner.key")
        );
        assert_eq!(envelope.join(&SchemaSourcePath::root()), envelope);
    }
}

/// Phase 1 (TDD scaffolding) for the schema-plus composition primitives
/// (`darkmatter/features/2026-07-08-schema-plus/`). These exercise the
/// YAML-shape layer: pattern keys and the reserved `$constraints` metadata key
/// authored as block-form mapping keys (the form the fixtures use). Each is
/// `#[ignore]`-gated until its phase lands. Run with
/// `cargo nextest run -p darkmatter --run-ignored ignored-only schema_plus`.
#[cfg(test)]
mod schema_plus_phase1 {
    use super::*;

    fn yaml(input: &str) -> YamlValue {
        serde_yaml_ng::from_str(input).expect("yaml parse failed")
    }

    fn single_shape(input: &str) -> SchemaShape {
        match parse_yaml_schema(&yaml(input)).expect("schema must parse") {
            SimplifiedSchema::Single(s) => s,
            other => panic!("expected Single, got {other:?}"),
        }
    }

    /// Reaches the inline-object shape rooted at `key` inside `shape`.
    fn inner_inline<'a>(shape: &'a SchemaShape, key: &str) -> &'a SchemaShape {
        match shape.properties.get(key) {
            Some(PropertyDef::Single(a)) => match &a.ty {
                TypeExpr::InlineObject(s) => s,
                other => panic!("expected inline object at `{key}`, got {other:?}"),
            },
            other => panic!("expected single property `{key}`, got {other:?}"),
        }
    }

    #[test]
    fn catch_all_key_is_not_a_literal_property() {
        let shape = single_shape("parameter:\n  \"<string>\": any");
        let param = inner_inline(&shape, "parameter");
        assert!(
            !param.properties.contains_key("<string>"),
            "`<string>` must be recognized as a catch-all pattern key, not a literal property"
        );
        assert_eq!(param.pattern_keys.len(), 1);
        assert_eq!(param.pattern_keys[0].key, PatternKey::CatchAll);
    }

    #[test]
    fn starting_key_is_not_a_literal_property() {
        let shape = single_shape("headers:\n  \"<starting::x-509>\": string");
        let headers = inner_inline(&shape, "headers");
        assert!(
            !headers.properties.contains_key("<starting::x-509>"),
            "`<starting::…>` must be recognized as a pattern key"
        );
        assert_eq!(
            headers.pattern_keys[0].key,
            PatternKey::Starting("x-509".into())
        );
    }

    #[test]
    fn ending_key_is_not_a_literal_property() {
        let shape = single_shape("assets:\n  \"<ending::.md>\": string");
        let assets = inner_inline(&shape, "assets");
        assert!(
            !assets.properties.contains_key("<ending::.md>"),
            "`<ending::…>` must be recognized as a pattern key"
        );
        assert_eq!(assets.pattern_keys[0].key, PatternKey::Ending(".md".into()));
    }

    #[test]
    fn regex_pattern_key_is_not_a_literal_property() {
        let shape = single_shape("codes:\n  \"<pattern::[0-9_]$>\": number");
        let codes = inner_inline(&shape, "codes");
        assert!(
            !codes.properties.contains_key("<pattern::[0-9_]$>"),
            "`<pattern::…>` must be recognized as a pattern key"
        );
        assert_eq!(
            codes.pattern_keys[0].key,
            PatternKey::Pattern("[0-9_]$".into())
        );
    }

    #[test]
    fn dollar_constraints_block_is_parsed_and_stripped() {
        // Matches the `types.yaml` fixture: a pattern-keyed dictionary with a
        // reserved `$constraints` metadata mapping carrying object-arity
        // constraints. `$constraints` must parse (its values are numbers, not
        // type-expression strings) and must not survive as a literal property.
        let res = parse_yaml_schema(&yaml(
            "parameter:\n  \"<string>\": any\n  $constraints:\n    min-keys: 1\n    max-keys: 1",
        ));
        assert!(
            res.is_ok(),
            "`$constraints` block form should parse, got {res:?}"
        );
        let shape = match res.unwrap() {
            SimplifiedSchema::Single(s) => s,
            other => panic!("expected Single, got {other:?}"),
        };
        let param = inner_inline(&shape, "parameter");
        assert!(
            !param.properties.contains_key("$constraints"),
            "`$constraints` is schema metadata and must not be a literal property"
        );
    }
}
