//! The `discriminator` of a component `oneOf`/`anyOf`, resolved per alternative.
//!
//! A union whose alternatives all resolve to a tag-value set is generated with a
//! dispatching `Deserialize`; otherwise it stays a first-match untagged enum and
//! the import reports why.

use openapiv3::{Discriminator, OpenAPI, ReferenceOr, Schema, SchemaKind, Type};

use super::super::diagnostics::OpenApiDiagnostic;
use crate::models::UnionDiscriminator;

const COMPONENT_PREFIX: &str = "#/components/schemas/";

/// Nesting allowed through references, `allOf` members, and nested unions;
/// a deeper (or cyclic) chain leaves the union untagged.
const MAX_DEPTH: usize = 8;

/// Tag values for every alternative of the union at `location`, or `None`
/// with a warning when some alternative has none.
///
/// Per alternative, in order: the `mapping` keys naming it, else the
/// single- or multi-valued string `enum` of the discriminator property, else
/// the OpenAPI implicit value (its component name) when it declares the
/// property as a plain string or not at all. An alternative that declares the
/// property optional cannot be dispatched on, since a value without the
/// property would still be valid for it.
pub(super) fn resolve_discriminator(
    doc: &OpenAPI,
    discriminator: &Discriminator,
    alternatives: &[ReferenceOr<Schema>],
    location: &str,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<UnionDiscriminator> {
    let property = &discriminator.property_name;
    let mut tags = Vec::with_capacity(alternatives.len());
    for (index, alternative) in alternatives.iter().enumerate() {
        let mapped: Vec<String> = component_name(alternative)
            .map(|name| {
                discriminator
                    .mapping
                    .iter()
                    .filter(|(_, target)| target.strip_prefix(COMPONENT_PREFIX).unwrap_or(target) == name)
                    .map(|(value, _)| value.clone())
                    .collect()
            })
            .unwrap_or_default();
        let resolved = if mapped.is_empty() {
            match property_tags(doc, alternative, property, false, 0) {
                Ok(Tags::Values(values)) => Ok(values),
                Ok(Tags::Undeclared) => component_name(alternative)
                    .map(|name| vec![name.to_string()])
                    .ok_or_else(|| "an inline alternative has no implicit value".to_string()),
                Err(reason) => Err(reason),
            }
        } else {
            Ok(mapped)
        };
        match resolved {
            Ok(values) => tags.push(values),
            Err(reason) => {
                let label = component_name(alternative).map_or_else(|| format!("alternative {index}"), |name| format!("`{name}`"));
                diagnostics.push(OpenApiDiagnostic::warn(
                    location.to_string(),
                    format!(
                        "discriminator `{property}` has no value for {label} ({reason}); the union decodes as the first alternative that fits"
                    ),
                ));
                return None;
            }
        }
    }
    Some(UnionDiscriminator { property: property.clone(), tags })
}

/// What an alternative says about the discriminator property.
enum Tags {
    /// It requires the property and limits it to these string values.
    Values(Vec<String>),
    /// It does not declare the property, or declares it as an unrestricted
    /// string, so the OpenAPI implicit value applies.
    Undeclared,
}

fn component_name(schema: &ReferenceOr<Schema>) -> Option<&str> {
    match schema {
        ReferenceOr::Reference { reference } => reference.strip_prefix(COMPONENT_PREFIX),
        ReferenceOr::Item(_) => None,
    }
}

fn resolve<'a>(doc: &'a OpenAPI, schema: &'a ReferenceOr<Schema>, depth: usize) -> Result<&'a Schema, String> {
    match schema {
        ReferenceOr::Item(schema) => Ok(schema),
        ReferenceOr::Reference { reference } => resolve_reference(doc, reference, depth),
    }
}

fn resolve_reference<'a>(doc: &'a OpenAPI, reference: &str, depth: usize) -> Result<&'a Schema, String> {
    if depth >= MAX_DEPTH {
        return Err("references nest too deeply".to_string());
    }
    let target = reference
        .strip_prefix(COMPONENT_PREFIX)
        .and_then(|name| doc.components.as_ref()?.schemas.get(name))
        .ok_or_else(|| format!("`{reference}` does not resolve"))?;
    resolve(doc, target, depth + 1)
}

/// `required` is set when an enclosing `allOf` already requires the property.
fn property_tags(doc: &OpenAPI, schema: &ReferenceOr<Schema>, property: &str, required: bool, depth: usize) -> Result<Tags, String> {
    if depth >= MAX_DEPTH {
        return Err("alternatives nest too deeply".to_string());
    }
    let schema = resolve(doc, schema, depth)?;
    let own = |names: &[String]| required || names.iter().any(|name| name == property);
    match &schema.schema_kind {
        SchemaKind::Type(Type::Object(object)) => declared_tags(doc, object.properties.get(property), own(&object.required), depth),
        SchemaKind::Any(any) if !any.properties.is_empty() => declared_tags(doc, any.properties.get(property), own(&any.required), depth),
        SchemaKind::AllOf { all_of } => {
            // One member may declare the property and another require it.
            let required = required || all_of.iter().any(|member| requires(doc, member, property, depth + 1));
            for member in all_of {
                match property_tags(doc, member, property, required, depth + 1)? {
                    Tags::Undeclared => continue,
                    values => return Ok(values),
                }
            }
            Ok(Tags::Undeclared)
        }
        // A nested union accepts every value its own alternatives accept.
        SchemaKind::OneOf { one_of: members } | SchemaKind::AnyOf { any_of: members } => {
            let mut values = Vec::new();
            for member in members {
                match property_tags(doc, member, property, required, depth + 1)? {
                    Tags::Values(member_values) => values.extend(member_values),
                    Tags::Undeclared => return Err("a nested alternative does not restrict the property".to_string()),
                }
            }
            Ok(Tags::Values(values))
        }
        _ => Err("it is not an object schema".to_string()),
    }
}

fn requires(doc: &OpenAPI, schema: &ReferenceOr<Schema>, property: &str, depth: usize) -> bool {
    let Ok(schema) = resolve(doc, schema, depth) else { return false };
    match &schema.schema_kind {
        SchemaKind::Type(Type::Object(object)) => object.required.iter().any(|name| name == property),
        SchemaKind::Any(any) => any.required.iter().any(|name| name == property),
        SchemaKind::AllOf { all_of } if depth < MAX_DEPTH => all_of.iter().any(|member| requires(doc, member, property, depth + 1)),
        _ => false,
    }
}

fn declared_tags(doc: &OpenAPI, declared: Option<&ReferenceOr<Box<Schema>>>, required: bool, depth: usize) -> Result<Tags, String> {
    let Some(declared) = declared else { return Ok(Tags::Undeclared) };
    if !required {
        return Err("the property is optional".to_string());
    }
    let declared = match declared {
        ReferenceOr::Item(schema) => schema.as_ref(),
        ReferenceOr::Reference { reference } => resolve_reference(doc, reference, depth)?,
    };
    match &declared.schema_kind {
        SchemaKind::Type(Type::String(string)) if string.enumeration.is_empty() => Ok(Tags::Undeclared),
        SchemaKind::Type(Type::String(string)) => {
            string.enumeration.iter().map(|value| value.clone().ok_or_else(|| "its enum allows null".to_string())).collect::<Result<_, _>>().map(Tags::Values)
        }
        _ => Err("the property is not a string".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{ModelDef, TypeAlias, UnionDiscriminator};
    use crate::openapi::OpenApiSource;
    use crate::openapi::import::{OpenApiDiagnostic, OpenApiImport};

    /// Imports `schemas` (YAML under `components.schemas`) and returns the
    /// `Pet` alias with every warning.
    fn import_pet(schemas: &str) -> (TypeAlias, Vec<OpenApiDiagnostic>) {
        let indented: String = schemas.lines().map(|line| format!("    {line}\n")).collect();
        let yaml = format!("openapi: \"3.0.0\"\ninfo:\n  title: Test\n  version: \"1.0.0\"\nservers:\n  - url: https://api.test\npaths: {{}}\ncomponents:\n  schemas:\n{indented}");
        let result = OpenApiImport::new(OpenApiSource::yaml(&yaml)).build().expect("the document imports");
        let pet = result
            .models
            .types
            .iter()
            .find_map(|model| match model {
                ModelDef::Alias(alias) if alias.name == "Pet" => Some(alias.clone()),
                _ => None,
            })
            .expect("Pet imports as a union alias");
        let warnings = result.diagnostics.into_iter().filter(OpenApiDiagnostic::is_warn).collect();
        (pet, warnings)
    }

    fn tags(values: &[&[&str]]) -> Vec<Vec<String>> {
        values.iter().map(|set| set.iter().map(|value| value.to_string()).collect()).collect()
    }

    const CAT: &str = "Cat:
  type: object
  required: [kind, meow]
  properties:
    kind: {type: string, enum: [cat]}
    meow: {type: boolean}";

    #[test]
    fn discriminator_mapping_keys_select_their_alternative() {
        let (pet, warnings) = import_pet(&format!(
            "Pet:
  oneOf:
    - $ref: '#/components/schemas/Cat'
    - $ref: '#/components/schemas/Dog'
  discriminator:
    propertyName: kind
    mapping:
      kitty: '#/components/schemas/Cat'
      feline: Cat
{CAT}
Dog:
  type: object
  required: [kind]
  properties:
    kind: {{type: string, enum: [dog]}}"
        ));

        assert_eq!(pet.discriminator, Some(UnionDiscriminator { property: "kind".into(), tags: tags(&[&["kitty", "feline"], &["dog"]]) }));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn discriminator_without_mapping_reads_enum_through_all_of_and_nested_unions() {
        let (pet, warnings) = import_pet(&format!(
            "Pet:
  anyOf:
    - $ref: '#/components/schemas/Cat'
    - $ref: '#/components/schemas/Wolf'
    - $ref: '#/components/schemas/Birds'
  discriminator:
    propertyName: kind
{CAT}
Wolf:
  allOf:
    - type: object
      properties:
        kind: {{type: string, enum: [wolf, dire_wolf]}}
    - type: object
      required: [kind]
Birds:
  oneOf:
    - type: object
      required: [kind]
      properties:
        kind: {{type: string, enum: [owl]}}
    - type: object
      required: [kind]
      properties:
        kind: {{$ref: '#/components/schemas/HawkKind'}}
HawkKind:
  type: string
  enum: [hawk]"
        ));

        assert_eq!(
            pet.discriminator,
            Some(UnionDiscriminator { property: "kind".into(), tags: tags(&[&["cat"], &["wolf", "dire_wolf"], &["owl", "hawk"]]) })
        );
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn discriminator_falls_back_to_the_component_name() {
        let (pet, warnings) = import_pet(&format!(
            "Pet:
  oneOf:
    - $ref: '#/components/schemas/Cat'
    - $ref: '#/components/schemas/Dog'
    - $ref: '#/components/schemas/Fish'
  discriminator:
    propertyName: kind
{CAT}
Dog:
  type: object
  properties:
    bark: {{type: boolean}}
Fish:
  type: object
  required: [kind]
  properties:
    kind: {{type: string}}"
        ));

        assert_eq!(pet.discriminator, Some(UnionDiscriminator { property: "kind".into(), tags: tags(&[&["cat"], &["Dog"], &["Fish"]]) }));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn discriminator_unresolvable_alternative_keeps_the_union_untagged_with_a_warning() {
        let cases = [
            ("optional tag", "Dog:\n  type: object\n  properties:\n    kind: {type: string, enum: [dog]}", "`Dog` (the property is optional)"),
            ("non-string tag", "Dog:\n  type: object\n  required: [kind]\n  properties:\n    kind: {type: integer}", "`Dog` (the property is not a string)"),
            ("nullable enum", "Dog:\n  type: object\n  required: [kind]\n  properties:\n    kind: {type: string, nullable: true, enum: [dog, null]}", "`Dog` (its enum allows null)"),
        ];
        for (case, dog, reason) in cases {
            let (pet, warnings) = import_pet(&format!(
                "Pet:
  oneOf:
    - $ref: '#/components/schemas/Cat'
    - $ref: '#/components/schemas/Dog'
  discriminator:
    propertyName: kind
{CAT}
{dog}"
            ));

            assert_eq!(pet.discriminator, None, "{case}");
            let [warning] = warnings.as_slice() else { panic!("{case}: expected one warning, got {warnings:?}") };
            assert_eq!(warning.location, "#/components/schemas/Pet", "{case}");
            assert!(warning.message.contains(reason) && warning.message.contains("first alternative that fits"), "{case}: {}", warning.message);
        }
    }

    #[test]
    fn discriminator_inline_alternative_needs_its_own_value() {
        let (pet, warnings) = import_pet(
            "Pet:
  oneOf:
    - type: object
      required: [kind]
      properties:
        kind: {type: string, enum: [cat]}
    - type: object
      properties:
        bark: {type: boolean}
  discriminator:
    propertyName: kind",
        );

        assert_eq!(pet.discriminator, None);
        let [warning] = warnings.as_slice() else { panic!("expected one warning, got {warnings:?}") };
        assert!(warning.message.contains("alternative 1 (an inline alternative has no implicit value)"), "{}", warning.message);
    }

    #[test]
    fn union_without_discriminator_has_none() {
        let (pet, warnings) = import_pet(&format!(
            "Pet:
  oneOf:
    - $ref: '#/components/schemas/Cat'
    - type: string
{CAT}"
        ));

        assert_eq!(pet.discriminator, None);
        assert!(warnings.is_empty(), "{warnings:?}");
    }
}
