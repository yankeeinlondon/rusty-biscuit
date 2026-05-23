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

pub mod convert;
pub mod grammar;
mod serialize;
pub mod types;

pub use convert::{DRAFT_2020_12, to_json_schema};
pub use serialize::serialize_property_atom;
pub use types::{
    Constraint, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema, SimplifiedType,
};

use indexmap::IndexMap;
use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

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
    match value {
        YamlValue::Mapping(_) => Ok(SimplifiedSchema::Single(parse_schema_shape(
            "<root>", value,
        )?)),
        YamlValue::Sequence(items) => {
            let mut arms = Vec::with_capacity(items.len());
            for (idx, item) in items.iter().enumerate() {
                let label = format!("<arm[{idx}]>");
                let arm = match item {
                    YamlValue::Mapping(_) => SchemaArm::Inline(parse_schema_shape(&label, item)?),
                    YamlValue::String(s) => SchemaArm::FileRef(s.clone()),
                    other => {
                        return Err(SchemaError::Grammar {
                            property: label,
                            message: format!(
                                "root union arms must be a mapping or a file-reference string, got {}",
                                describe_yaml(other)
                            ),
                            span: 0..0,
                        });
                    }
                };
                arms.push(arm);
            }
            if arms.is_empty() {
                return Err(SchemaError::Grammar {
                    property: "<root>".into(),
                    message: "root union must have at least one arm".into(),
                    span: 0..0,
                });
            }
            Ok(SimplifiedSchema::Union(arms))
        }
        other => Err(SchemaError::Grammar {
            property: "<root>".into(),
            message: format!(
                "$schema must be a mapping or a sequence, got {}",
                describe_yaml(other)
            ),
            span: 0..0,
        }),
    }
}

fn parse_schema_shape(context: &str, value: &YamlValue) -> Result<SchemaShape, SchemaError> {
    let map = value.as_mapping().ok_or_else(|| SchemaError::Grammar {
        property: context.to_string(),
        message: "expected a mapping of property names to type expressions".into(),
        span: 0..0,
    })?;

    let mut properties: IndexMap<String, PropertyDef> = IndexMap::with_capacity(map.len());
    for (key, val) in map {
        let key_str = key.as_str().ok_or_else(|| SchemaError::Grammar {
            property: context.to_string(),
            message: "property names must be strings".into(),
            span: 0..0,
        })?;
        let def = parse_property_def(key_str, val)?;
        properties.insert(key_str.to_string(), def);
    }
    Ok(SchemaShape { properties })
}

fn parse_property_def(name: &str, value: &YamlValue) -> Result<PropertyDef, SchemaError> {
    match value {
        YamlValue::String(s) => Ok(PropertyDef::Single(grammar::parse_type_expr(name, s)?)),
        YamlValue::Sequence(items) => {
            let mut arms = Vec::with_capacity(items.len());
            for (idx, item) in items.iter().enumerate() {
                match item {
                    YamlValue::String(s) => {
                        arms.push(grammar::parse_type_expr(name, s)?);
                    }
                    YamlValue::Mapping(_) => {
                        return Err(SchemaError::Grammar {
                            property: name.to_string(),
                            message: format!(
                                "property-level union arm[{idx}] is a mapping; nested object schemas as union arms are not supported"
                            ),
                            span: 0..0,
                        });
                    }
                    other => {
                        return Err(SchemaError::Grammar {
                            property: name.to_string(),
                            message: format!(
                                "property-level union arm[{idx}] must be a string type expression, got {}",
                                describe_yaml(other)
                            ),
                            span: 0..0,
                        });
                    }
                }
            }
            if arms.is_empty() {
                return Err(SchemaError::Grammar {
                    property: name.to_string(),
                    message: "property-level union must have at least one arm".into(),
                    span: 0..0,
                });
            }
            Ok(PropertyDef::Union(arms))
        }
        YamlValue::Mapping(_) => Err(SchemaError::Grammar {
            property: name.to_string(),
            message: "mapping property values are reserved for future nested object schemas; \
                 use a string type expression"
                .into(),
            span: 0..0,
        }),
        other => Err(SchemaError::Grammar {
            property: name.to_string(),
            message: format!(
                "property value must be a string type expression or a sequence of expressions, got {}",
                describe_yaml(other)
            ),
            span: 0..0,
        }),
    }
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
                assert_eq!(arms[0].ty, SimplifiedType::String);
                assert_eq!(arms[1].ty, SimplifiedType::Number);
            }
            _ => panic!("expected Union"),
        }
    }

    #[test]
    fn rejects_property_level_union_with_mapping_arm() {
        let v = yaml("foo:\n  - string\n  - {nested: thing}");
        let err = parse_yaml_schema(&v).unwrap_err();
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("nested object schemas"));
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
    fn rejects_property_value_mapping() {
        let v = yaml("foo:\n  bar: baz");
        let err = parse_yaml_schema(&v).unwrap_err();
        let SchemaError::Grammar {
            property, message, ..
        } = err
        else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert_eq!(property, "foo");
        assert!(message.contains("nested object schemas"));
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
        let v = yaml("title: widget");
        let err = parse_yaml_schema(&v).unwrap_err();
        let SchemaError::Grammar { property, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert_eq!(property, "title");
    }
}
