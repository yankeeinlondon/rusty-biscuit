//! Schema↔catalog compatibility gate.
//!
//! Runs BEFORE any value mapping: the registry declares the SimplifiedSchema
//! shape it expects at each research path, and this module verifies the
//! topic sidecar against it. In particular, a sidecar `enum(...)` feeding a
//! Rust enum must declare members that are a subset of the strum-introspected
//! variant names — contract drift fails here, earlier than any value-level
//! check and before any research runs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use darkmatter::markdown::schemas::{
    Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, SimplifiedType,
    TypeExpr, parse_standalone_schema_document, parse_yaml_schema,
};

use crate::errors::GenError;
use crate::memo::FileMemo;
use crate::registry::{DeclaredSource, RegistryEntry, SchemaExpectation};

/// Loads a topic sidecar (`_schema.yaml`) as a [`SimplifiedSchema`].
///
/// The sidecar must wrap its properties under a root `$schema:` key —
/// darkmatter's resolver otherwise classifies the file as raw JSON Schema.
/// A top-level property typed as an object from the topic's own types file
/// (`name@./_types.yaml`) is inlined, so the gate checks a named record type
/// exactly like an inline-object one.
///
/// The result is reused while the sidecar and the types files it inlines are
/// unchanged: every provider pass gates on the same topic sidecars.
pub fn load_sidecar_schema(path: &Path) -> Result<SimplifiedSchema, GenError> {
    static SIDECARS: LazyLock<FileMemo<SimplifiedSchema>> = LazyLock::new(FileMemo::new);
    SIDECARS.get_or_try_insert(format!("{path:?}"), || {
        let mut schema = parse_sidecar_schema(path)?;
        let mut inputs = inline_local_object_types(&mut schema, path)?;
        inputs.push(path.to_path_buf());
        Ok((schema, inputs))
    })
}

/// Parses a topic sidecar without inlining its local object types, for a
/// caller that reads only top-level property declarations.
pub(crate) fn parse_sidecar_schema(path: &Path) -> Result<SimplifiedSchema, GenError> {
    let text = std::fs::read_to_string(path).map_err(|source| GenError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&text).map_err(|err| GenError::Yaml {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
    let root = value
        .get("$schema")
        .ok_or_else(|| GenError::SidecarMissing {
            path: path.to_path_buf(),
        })?;
    parse_yaml_schema(root).map_err(|err| GenError::SidecarInvalid {
        path: path.to_path_buf(),
        message: err.to_string(),
    })
}

/// Replaces each top-level `name@./file` import of an object type with that
/// type's shape. Imports of shared types (`../`) and of non-object types
/// stay as they are; no registry entry reads through them. Returns the types
/// files read.
fn inline_local_object_types(schema: &mut SimplifiedSchema, sidecar: &Path) -> Result<Vec<PathBuf>, GenError> {
    let SimplifiedSchema::Single(shape) = schema else {
        return Ok(Vec::new());
    };
    // WHY: a sidecar imports many record types from the same types file, and
    // parsing that file once per import dominated generation time.
    let mut types_files: HashMap<PathBuf, SchemaShape> = HashMap::new();
    for def in shape.properties.values_mut() {
        let PropertyDef::Single(atom) = def else {
            continue;
        };
        let TypeExpr::Imported { name, reference } = &atom.ty else {
            continue;
        };
        let Some(relative) = reference.strip_prefix("./") else {
            continue;
        };
        let types_path = sidecar.with_file_name(relative);
        if !types_files.contains_key(&types_path) {
            types_files.insert(types_path.clone(), local_types(&types_path)?);
        }
        if let Some(object) = local_object_type(&types_files[&types_path], &types_path, name)? {
            atom.ty = TypeExpr::InlineObject(object);
        }
    }
    Ok(types_files.into_keys().collect())
}

/// The `types` mapping of the `kind: schema` types file at `path`.
fn local_types(path: &Path) -> Result<SchemaShape, GenError> {
    let text = std::fs::read_to_string(path).map_err(|source| GenError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let invalid = |message: String| GenError::SidecarInvalid {
        path: path.to_path_buf(),
        message,
    };
    let document = parse_standalone_schema_document(&text, path)
        .map_err(|err| invalid(err.to_string()))?
        .ok_or_else(|| invalid("not a `kind: schema` types file".into()))?;
    let Some(SimplifiedSchema::Single(types)) = document.schema() else {
        return Err(invalid("the `types` mapping did not parse as one schema".into()));
    };
    Ok(types.clone())
}

/// The shape of the object type `name` in `types`, the mapping of the types
/// file at `path`, or `None` when `name` is a non-object type.
fn local_object_type(types: &SchemaShape, path: &Path, name: &str) -> Result<Option<SchemaShape>, GenError> {
    match types.properties.get(name) {
        Some(PropertyDef::Single(PropertyAtom {
            ty: TypeExpr::InlineObject(object),
            ..
        })) => Ok(Some(object.clone())),
        Some(_) => Ok(None),
        None => Err(GenError::SidecarInvalid {
            path: path.to_path_buf(),
            message: format!("declares no type named `{name}`"),
        }),
    }
}

/// Checks every research-sourced registry entry against its topic sidecar.
///
/// `sidecar_for` maps a topic name to its loaded schema; entries whose
/// topic is absent fail with [`GenError::SchemaPathMissing`].
pub fn check_entries<'a>(
    entries: &[RegistryEntry],
    mut sidecar_for: impl FnMut(&str) -> Option<&'a SimplifiedSchema>,
) -> Result<(), GenError> {
    for entry in entries {
        let DeclaredSource::Research { topic, path } = entry.source else {
            continue;
        };
        let schema = sidecar_for(topic).ok_or_else(|| GenError::SchemaPathMissing {
            field: entry.field,
            path: format!("{topic}/_schema.yaml"),
        })?;
        let atom = resolve_path(schema, path).ok_or_else(|| GenError::SchemaPathMissing {
            field: entry.field,
            path: path.to_string(),
        })?;
        check_atom(entry, path, atom)?;
    }
    Ok(())
}

/// Resolves a dot-separated property path to its [`PropertyAtom`], descending
/// through inline-object literals (including array-of-record elements).
fn resolve_path<'s>(schema: &'s SimplifiedSchema, path: &str) -> Option<&'s PropertyAtom> {
    let SimplifiedSchema::Single(shape) = schema else {
        // Root unions never appear in topic sidecars; unsupported here.
        return None;
    };
    let mut shape: &SchemaShape = shape;
    let mut segments = path.split('.').peekable();
    while let Some(segment) = segments.next() {
        let def = shape.properties.get(segment)?;
        let PropertyDef::Single(atom) = def else {
            // Property-level unions are not consumed by the registry yet.
            return None;
        };
        if segments.peek().is_none() {
            return Some(atom);
        }
        let TypeExpr::InlineObject(inner) = &atom.ty else {
            return None;
        };
        shape = inner;
    }
    None
}

/// Verifies one atom against an entry's expectation list.
///
/// Expectations are OR alternatives keyed by shape kind. A shape that
/// matches an alternative's kind but violates its constraint — an
/// `enum(...)` whose members are not a subset of the Rust variants — is a
/// loud [`GenError::EnumNotSubset`], never a silent fall-through.
fn check_atom(entry: &RegistryEntry, path: &str, atom: &PropertyAtom) -> Result<(), GenError> {
    for expectation in entry.expected {
        match expectation {
            SchemaExpectation::String => {
                if !atom.is_array && atom.ty == TypeExpr::Primitive(SimplifiedType::String) {
                    return Ok(());
                }
            }
            SchemaExpectation::Boolean => {
                if !atom.is_array
                    && matches!(
                        atom.ty,
                        TypeExpr::Primitive(SimplifiedType::Boolean)
                            | TypeExpr::Primitive(SimplifiedType::Boolish)
                    )
                {
                    return Ok(());
                }
            }
            SchemaExpectation::StringArray => {
                if atom.is_array && atom.ty == TypeExpr::Primitive(SimplifiedType::String) {
                    return Ok(());
                }
            }
            SchemaExpectation::Record { required_fields } => {
                if !atom.is_array
                    && let TypeExpr::InlineObject(shape) = &atom.ty
                {
                    let missing: Vec<&str> = required_fields
                        .iter()
                        .filter(|f| !shape.properties.contains_key(**f))
                        .copied()
                        .collect();
                    if missing.is_empty() {
                        return Ok(());
                    }
                    return Err(GenError::SchemaIncompatible {
                        field: entry.field,
                        path: path.to_string(),
                        found: format!("record missing fields [{}]", missing.join(", ")),
                        expected: expectation.label(),
                    });
                }
            }
            SchemaExpectation::EnumSubsetOf {
                rust_enum,
                variants,
            } => {
                if !atom.is_array && atom.ty == TypeExpr::Primitive(SimplifiedType::Enum) {
                    let members = enum_members(atom);
                    let offending: Vec<&str> = members
                        .iter()
                        .filter(|m| !variants.contains(&m.as_str()))
                        .map(String::as_str)
                        .collect();
                    if offending.is_empty() {
                        return Ok(());
                    }
                    return Err(GenError::EnumNotSubset {
                        field: entry.field,
                        path: path.to_string(),
                        rust_enum,
                        offending: offending.join(", "),
                        variants: variants.join(", "),
                    });
                }
            }
            SchemaExpectation::RecordArray { required_fields } => {
                if atom.is_array
                    && let TypeExpr::InlineObject(shape) = &atom.ty
                {
                    let missing: Vec<&str> = required_fields
                        .iter()
                        .filter(|f| !shape.properties.contains_key(**f))
                        .copied()
                        .collect();
                    if missing.is_empty() {
                        return Ok(());
                    }
                    return Err(GenError::SchemaIncompatible {
                        field: entry.field,
                        path: path.to_string(),
                        found: format!("record array missing fields [{}]", missing.join(", ")),
                        expected: expectation.label(),
                    });
                }
            }
        }
    }
    Err(GenError::SchemaIncompatible {
        field: entry.field,
        path: path.to_string(),
        found: describe_atom(atom),
        expected: entry
            .expected
            .iter()
            .map(SchemaExpectation::label)
            .collect::<Vec<_>>()
            .join(", "),
    })
}

fn enum_members(atom: &PropertyAtom) -> Vec<String> {
    atom.constraints
        .iter()
        .find_map(|c| match c {
            Constraint::Members(members) => Some(members.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn describe_atom(atom: &PropertyAtom) -> String {
    let suffix = if atom.is_array { "[]" } else { "" };
    match &atom.ty {
        TypeExpr::Primitive(ty) => format!("{base}{suffix}", base = ty.as_keyword()),
        TypeExpr::InlineObject(_) => format!("inline object{suffix}"),
        TypeExpr::Imported { name, reference } => format!("{name}{suffix}@{reference}"),
    }
}
