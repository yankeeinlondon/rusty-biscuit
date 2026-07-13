//! Model code generation from `ModelCatalog`.
//!
//! Transforms `ModelCatalog` type definitions into Rust source code with
//! serde derives, doc comments, and correct type mappings.

use std::collections::HashSet;
use std::path::Path;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::models::{
    EnumDef, EnumVariant, FieldDef, ModelCatalog, ModelDef, PrimitiveType, StructDef, TypeAlias,
    TypeRef,
};

use crate::errors::GeneratorError;
use crate::output::{format_code, validate_code, write_atomic};

/// Generates Rust source code from a `ModelCatalog`.
///
/// Produces a `types.rs` file containing struct, enum, and type alias
/// definitions with serde derives and doc comments.
///
/// ## Returns
///
/// The generated Rust source code as a formatted string.
///
/// ## Errors
///
/// Returns `GeneratorError` if:
/// - Generated code fails syn validation
/// - File writing fails (when not dry-run)
pub fn generate_models(
    catalog: &ModelCatalog,
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    let tokens = assemble_model_tokens(catalog);
    let file = validate_code(&tokens)?;
    let formatted = format_code(&file);

    if dry_run {
        println!("=== types.rs ===\n{}\n", formatted);
    } else {
        write_atomic(&output_dir.join("types.rs"), &formatted)?;
    }

    Ok(formatted)
}

/// Assembles token stream for all model definitions.
pub fn assemble_model_tokens(catalog: &ModelCatalog) -> TokenStream {
    let type_defs: Vec<TokenStream> = catalog
        .types
        .iter()
        .map(|model| match model {
            ModelDef::Struct(s) => generate_struct(s),
            ModelDef::Enum(e) => generate_enum(e),
            ModelDef::Alias(a) => generate_alias(a),
            _ => TokenStream::new(),
        })
        .collect();

    quote! {
        //! Generated model types.
        //!
        //! These types were generated from an OpenAPI specification.
        //! Do not edit manually.

        use serde::{Deserialize, Serialize};
        use std::collections::HashMap;

        #(#type_defs)*
    }
}

/// Generates a Rust struct from a `StructDef`.
fn generate_struct(def: &StructDef) -> TokenStream {
    let name = format_ident!("{}", def.name);
    let doc = doc_comment(&def.description);

    let fields: Vec<TokenStream> = def.fields.iter().map(generate_field).collect();

    // If the struct has additional_properties, add a flattened HashMap field
    let extra_field = def.additional_properties.as_ref().map(|type_ref| {
        let value_type = type_ref_to_tokens(type_ref);
        quote! {
            /// Additional properties not captured by named fields.
            #[serde(flatten)]
            pub additional_properties: HashMap<String, #value_type>,
        }
    });

    // Determine if we can derive Default
    // We can if all required fields have types that impl Default (primitives, Option, Vec)
    // For simplicity, derive Default for all structs - users can manually implement if needed
    let derives = quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize)] };

    quote! {
        #doc
        #derives
        pub struct #name {
            #(#fields)*
            #extra_field
        }
    }
}

/// Generates a single struct field.
fn generate_field(def: &FieldDef) -> TokenStream {
    let name = format_ident!("{}", def.name);
    let doc = doc_comment(&def.description);

    let field_type = if def.required {
        type_ref_to_tokens(&def.field_type)
    } else {
        let inner = type_ref_to_tokens(&def.field_type);
        quote! { Option<#inner> }
    };

    let rename = def.serde_rename.as_ref().map(|r| {
        quote! { #[serde(rename = #r)] }
    });

    let skip_none = if !def.required {
        Some(quote! { #[serde(skip_serializing_if = "Option::is_none")] })
    } else {
        None
    };

    quote! {
        #doc
        #rename
        #skip_none
        pub #name: #field_type,
    }
}

/// Generates a Rust enum from an `EnumDef`.
fn generate_enum(def: &EnumDef) -> TokenStream {
    let name = format_ident!("{}", def.name);
    let doc = doc_comment(&def.description);

    let untagged_attr = if def.untagged {
        Some(quote! { #[serde(untagged)] })
    } else {
        None
    };

    // Deriving `Default` (with `#[default]` on the first variant) keeps structs
    // that hold a required field of this enum — and the `..Default::default()`
    // pattern in generated doc examples — compiling. All variants are unit
    // variants, so a first-variant default is always valid.
    let can_default = !def.variants.is_empty();
    let variants: Vec<TokenStream> = def
        .variants
        .iter()
        .enumerate()
        .map(|(i, variant)| generate_variant(variant, can_default && i == 0))
        .collect();

    let derives = if can_default {
        quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize)] }
    } else {
        quote! { #[derive(Debug, Clone, Serialize, Deserialize)] }
    };

    quote! {
        #doc
        #derives
        #untagged_attr
        pub enum #name {
            #(#variants)*
        }
    }
}

/// Generates a single enum variant.
///
/// `is_default` marks the variant with `#[default]` so the enum can derive
/// [`Default`].
fn generate_variant(def: &EnumVariant, is_default: bool) -> TokenStream {
    let name = format_ident!("{}", def.name);
    let doc = doc_comment(&def.description);

    let rename = def.value.as_ref().map(|v| {
        quote! { #[serde(rename = #v)] }
    });

    let default_attr = if is_default {
        Some(quote! { #[default] })
    } else {
        None
    };

    quote! {
        #doc
        #rename
        #default_attr
        #name,
    }
}

/// Generates a type alias from a `TypeAlias`.
fn generate_alias(def: &TypeAlias) -> TokenStream {
    let name = format_ident!("{}", def.name);
    let doc = doc_comment(&def.description);
    let target = type_ref_to_tokens(&def.target);

    quote! {
        #doc
        pub type #name = #target;
    }
}

/// Converts a `TypeRef` to a Rust type token stream.
fn type_ref_to_tokens(type_ref: &TypeRef) -> TokenStream {
    match type_ref {
        TypeRef::Primitive(p) => primitive_to_tokens(p),
        TypeRef::Array(inner) => {
            let inner_tokens = type_ref_to_tokens(inner);
            quote! { Vec<#inner_tokens> }
        }
        TypeRef::Map(inner) => {
            let inner_tokens = type_ref_to_tokens(inner);
            quote! { HashMap<String, #inner_tokens> }
        }
        TypeRef::Named(name) => {
            let ident = format_ident!("{}", name);
            quote! { #ident }
        }
        TypeRef::Optional(inner) => {
            let inner_tokens = type_ref_to_tokens(inner);
            quote! { Option<#inner_tokens> }
        }
        TypeRef::OneOf(_) | TypeRef::AnyOf(_) | TypeRef::AllOf(_) => {
            // Complex combinators fall back to serde_json::Value
            quote! { serde_json::Value }
        }
        TypeRef::Unknown => {
            quote! { serde_json::Value }
        }
        _ => {
            // Future-proof: any new TypeRef variants
            quote! { serde_json::Value }
        }
    }
}

/// Converts a `PrimitiveType` to a Rust type token stream.
fn primitive_to_tokens(prim: &PrimitiveType) -> TokenStream {
    match prim {
        PrimitiveType::String => quote! { String },
        PrimitiveType::Integer => quote! { i64 },
        PrimitiveType::Number => quote! { f64 },
        PrimitiveType::Boolean => quote! { bool },
        PrimitiveType::Bytes => quote! { Vec<u8> },
        PrimitiveType::Json => quote! { serde_json::Value },
        _ => quote! { serde_json::Value },
    }
}

/// Generates a doc comment token stream from an optional description.
fn doc_comment(desc: &Option<String>) -> TokenStream {
    match desc {
        Some(text) => {
            let text = text.trim();
            quote! { #[doc = #text] }
        }
        None => TokenStream::new(),
    }
}

/// Validates model names for uniqueness.
///
/// ## Errors
///
/// Returns an error listing duplicate names.
pub fn validate_model_names(catalog: &ModelCatalog) -> Result<(), GeneratorError> {
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();

    for model in &catalog.types {
        let name = match model {
            ModelDef::Struct(s) => &s.name,
            ModelDef::Enum(e) => &e.name,
            ModelDef::Alias(a) => &a.name,
            _ => continue,
        };

        if !seen.insert(name.clone()) {
            duplicates.push(name.clone());
        }
    }

    if duplicates.is_empty() {
        Ok(())
    } else {
        Err(GeneratorError::CodeGenError(format!(
            "Duplicate model names: {}",
            duplicates.join(", ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_catalog() -> ModelCatalog {
        ModelCatalog {
            module_path: None,
            types: vec![
                ModelDef::Struct(StructDef {
                    name: "Pet".to_string(),
                    description: Some("A pet in the store".to_string()),
                    fields: vec![
                        FieldDef {
                            name: "id".to_string(),
                            serde_rename: None,
                            description: Some("Unique identifier".to_string()),
                            required: true,
                            field_type: TypeRef::Primitive(PrimitiveType::Integer),
                        },
                        FieldDef {
                            name: "name".to_string(),
                            serde_rename: None,
                            description: None,
                            required: true,
                            field_type: TypeRef::Primitive(PrimitiveType::String),
                        },
                        FieldDef {
                            name: "tag".to_string(),
                            serde_rename: None,
                            description: None,
                            required: false,
                            field_type: TypeRef::Primitive(PrimitiveType::String),
                        },
                    ],
                    additional_properties: None,
                }),
                ModelDef::Enum(EnumDef {
                    name: "Status".to_string(),
                    description: Some("Pet status".to_string()),
                    variants: vec![
                        EnumVariant {
                            name: "Available".to_string(),
                            value: Some("available".to_string()),
                            description: None,
                        },
                        EnumVariant {
                            name: "Pending".to_string(),
                            value: Some("pending".to_string()),
                            description: None,
                        },
                        EnumVariant {
                            name: "Sold".to_string(),
                            value: Some("sold".to_string()),
                            description: None,
                        },
                    ],
                    untagged: false,
                }),
            ],
        }
    }

    #[test]
    fn assemble_model_tokens_produces_valid_code() {
        let catalog = make_simple_catalog();
        let tokens = assemble_model_tokens(&catalog);
        let result = validate_code(&tokens);
        assert!(
            result.is_ok(),
            "Generated code is invalid: {:?}",
            result.err()
        );
    }

    #[test]
    fn struct_has_serde_derives() {
        let catalog = make_simple_catalog();
        let tokens = assemble_model_tokens(&catalog);
        let code = tokens.to_string();
        assert!(code.contains("Serialize"));
        assert!(code.contains("Deserialize"));
    }

    #[test]
    fn struct_has_doc_comment() {
        let catalog = make_simple_catalog();
        let tokens = assemble_model_tokens(&catalog);
        let code = tokens.to_string();
        assert!(code.contains("A pet in the store"));
    }

    #[test]
    fn optional_fields_wrapped_in_option() {
        let catalog = make_simple_catalog();
        let tokens = assemble_model_tokens(&catalog);
        let code = tokens.to_string();
        assert!(code.contains("Option <"));
    }

    #[test]
    fn enum_has_rename_attrs() {
        let catalog = make_simple_catalog();
        let tokens = assemble_model_tokens(&catalog);
        let code = tokens.to_string();
        assert!(code.contains("available"));
        assert!(code.contains("pending"));
        assert!(code.contains("sold"));
    }

    #[test]
    fn generated_enum_derives_default_on_first_variant() {
        let catalog = make_simple_catalog();
        let code = assemble_model_tokens(&catalog).to_string();
        // The enum must derive Default and mark its first variant `#[default]`
        // so structs holding a required field of this enum stay Default.
        assert!(code.contains("Default"), "enum should derive Default:\n{code}");
        assert!(
            code.contains("# [default] Available")
                || code.contains("#[default] Available")
                || code.contains("# [default]\nAvailable"),
            "first variant should be marked #[default]:\n{code}"
        );
    }

    #[test]
    fn struct_with_required_enum_field_is_default_compatible() {
        // A struct whose required field is a generated enum must produce
        // syntactically valid code; the enum's derived Default is what makes
        // the struct's own derived Default satisfiable.
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![
                ModelDef::Enum(EnumDef {
                    name: "Kind".to_string(),
                    description: None,
                    variants: vec![
                        EnumVariant {
                            name: "First".to_string(),
                            value: Some("first".to_string()),
                            description: None,
                        },
                        EnumVariant {
                            name: "Second".to_string(),
                            value: Some("second".to_string()),
                            description: None,
                        },
                    ],
                    untagged: false,
                }),
                ModelDef::Struct(StructDef {
                    name: "Item".to_string(),
                    description: None,
                    fields: vec![FieldDef {
                        name: "kind".to_string(),
                        serde_rename: None,
                        description: None,
                        required: true,
                        field_type: TypeRef::Named("Kind".to_string()),
                    }],
                    additional_properties: None,
                }),
            ],
        };
        let tokens = assemble_model_tokens(&catalog);
        assert!(
            validate_code(&tokens).is_ok(),
            "generated code should parse"
        );
        let code = tokens.to_string();
        // Both the struct and the enum derive Default.
        assert!(
            code.matches("Default").count() >= 2,
            "struct and enum should both derive Default:\n{code}"
        );
    }

    #[test]
    fn type_alias_generates_correctly() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Alias(TypeAlias {
                name: "PetId".to_string(),
                description: Some("A pet identifier".to_string()),
                target: TypeRef::Primitive(PrimitiveType::String),
            })],
        };
        let tokens = assemble_model_tokens(&catalog);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
        let code = tokens.to_string();
        assert!(code.contains("PetId"));
        assert!(code.contains("String"));
    }

    #[test]
    fn array_type_generates_vec() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Struct(StructDef {
                name: "PetList".to_string(),
                description: None,
                fields: vec![FieldDef {
                    name: "pets".to_string(),
                    serde_rename: None,
                    description: None,
                    required: true,
                    field_type: TypeRef::Array(Box::new(TypeRef::Named("Pet".to_string()))),
                }],
                additional_properties: None,
            })],
        };
        let tokens = assemble_model_tokens(&catalog);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
        let code = tokens.to_string();
        assert!(code.contains("Vec"));
        assert!(code.contains("Pet"));
    }

    #[test]
    fn map_type_generates_hashmap() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Struct(StructDef {
                name: "Metadata".to_string(),
                description: None,
                fields: vec![FieldDef {
                    name: "tags".to_string(),
                    serde_rename: None,
                    description: None,
                    required: true,
                    field_type: TypeRef::Map(Box::new(TypeRef::Primitive(PrimitiveType::String))),
                }],
                additional_properties: None,
            })],
        };
        let tokens = assemble_model_tokens(&catalog);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
        let code = tokens.to_string();
        assert!(code.contains("HashMap"));
    }

    #[test]
    fn unknown_type_falls_back_to_value() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Struct(StructDef {
                name: "Flexible".to_string(),
                description: None,
                fields: vec![FieldDef {
                    name: "data".to_string(),
                    serde_rename: None,
                    description: None,
                    required: true,
                    field_type: TypeRef::Unknown,
                }],
                additional_properties: None,
            })],
        };
        let tokens = assemble_model_tokens(&catalog);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
        let code = tokens.to_string();
        assert!(code.contains("serde_json"));
        assert!(code.contains("Value"));
    }

    #[test]
    fn serde_rename_on_fields() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Struct(StructDef {
                name: "ApiModel".to_string(),
                description: None,
                fields: vec![FieldDef {
                    name: "created_at".to_string(),
                    serde_rename: Some("createdAt".to_string()),
                    description: None,
                    required: true,
                    field_type: TypeRef::Primitive(PrimitiveType::String),
                }],
                additional_properties: None,
            })],
        };
        let tokens = assemble_model_tokens(&catalog);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
        let code = tokens.to_string();
        assert!(code.contains("createdAt"));
    }

    #[test]
    fn untagged_enum() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Enum(EnumDef {
                name: "AnyAnimal".to_string(),
                description: None,
                variants: vec![
                    EnumVariant {
                        name: "Cat".to_string(),
                        value: None,
                        description: None,
                    },
                    EnumVariant {
                        name: "Dog".to_string(),
                        value: None,
                        description: None,
                    },
                ],
                untagged: true,
            })],
        };
        let tokens = assemble_model_tokens(&catalog);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
        let code = tokens.to_string();
        assert!(code.contains("untagged"));
    }

    #[test]
    fn struct_with_additional_properties() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Struct(StructDef {
                name: "ExtensibleObj".to_string(),
                description: None,
                fields: vec![FieldDef {
                    name: "name".to_string(),
                    serde_rename: None,
                    description: None,
                    required: true,
                    field_type: TypeRef::Primitive(PrimitiveType::String),
                }],
                additional_properties: Some(TypeRef::Primitive(PrimitiveType::Json)),
            })],
        };
        let tokens = assemble_model_tokens(&catalog);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
        let code = tokens.to_string();
        assert!(code.contains("additional_properties"));
        assert!(code.contains("flatten"));
    }

    #[test]
    fn validate_model_names_catches_duplicates() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![
                ModelDef::Struct(StructDef {
                    name: "Dup".to_string(),
                    description: None,
                    fields: vec![],
                    additional_properties: None,
                }),
                ModelDef::Enum(EnumDef {
                    name: "Dup".to_string(),
                    description: None,
                    variants: vec![],
                    untagged: false,
                }),
            ],
        };
        let result = validate_model_names(&catalog);
        assert!(result.is_err());
    }

    #[test]
    fn validate_model_names_passes_for_unique() {
        let catalog = make_simple_catalog();
        let result = validate_model_names(&catalog);
        assert!(result.is_ok());
    }

    #[test]
    fn empty_catalog_generates_valid_code() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![],
        };
        let tokens = assemble_model_tokens(&catalog);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn all_primitive_types_map_correctly() {
        // Verify each primitive maps to the expected Rust type
        let cases: Vec<(PrimitiveType, &str)> = vec![
            (PrimitiveType::String, "String"),
            (PrimitiveType::Integer, "i64"),
            (PrimitiveType::Number, "f64"),
            (PrimitiveType::Boolean, "bool"),
            (PrimitiveType::Bytes, "Vec < u8 >"),
            (PrimitiveType::Json, "serde_json :: Value"),
        ];

        for (prim, expected) in cases {
            let tokens = primitive_to_tokens(&prim);
            assert_eq!(tokens.to_string(), expected, "Failed for {:?}", prim);
        }
    }

    #[test]
    fn generate_models_dry_run() {
        let catalog = make_simple_catalog();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = generate_models(&catalog, temp_dir.path(), true);
        assert!(result.is_ok());
        // Dry run should not create files
        assert!(!temp_dir.path().join("types.rs").exists());
    }

    #[test]
    fn generate_models_writes_file() {
        let catalog = make_simple_catalog();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = generate_models(&catalog, temp_dir.path(), false);
        assert!(result.is_ok());
        assert!(temp_dir.path().join("types.rs").exists());
    }
}
