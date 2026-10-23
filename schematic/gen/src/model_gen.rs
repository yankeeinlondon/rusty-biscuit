//! Model code generation from `ModelCatalog`.
//!
//! Transforms `ModelCatalog` type definitions into Rust source code with
//! serde derives, doc comments, and correct type mappings.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use schematic_define::SchemaComponent;
use schematic_define::constraints::Constraints;
use schematic_define::models::{
    EnumDef, EnumVariant, FieldDef, ModelCatalog, ModelDef, PrimitiveType, StructDef, TypeAlias,
    TypeRef, UnionDiscriminator,
};
use serde_json::{Map, Value};

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
    generate_models_with(catalog, None, output_dir, dry_run)
}

/// Generates Rust source code from a `ModelCatalog`.
///
/// `json_schema`, when given, adds `schemars::JsonSchema` to every emitted
/// type and rebuilds each struct field's value rules from these imported
/// source components (see [`assemble_model_tokens_with`]).
///
/// ## Errors
///
/// Returns `GeneratorError` if:
/// - Generated code fails syn validation
/// - File writing fails (when not dry-run)
pub fn generate_models_with(
    catalog: &ModelCatalog,
    json_schema: Option<&[SchemaComponent]>,
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    let tokens = assemble_model_tokens_with(catalog, json_schema);
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
    assemble_model_tokens_with(catalog, None)
}

/// Assembles token stream for all model definitions.
///
/// `json_schema`, when given, adds `schemars::JsonSchema` and nominal typed
/// registration. The definitions crate needs it so its definition context can
/// register the types; the standalone client output does not depend on
/// `schemars`.
///
/// The given components are the imported source schemas, one per catalog
/// model under the model's name. A struct field whose property carries value
/// rules gets a `#[schemars(extend(..))]` attribute, so the schema Schemars
/// derives for the field states the same rules as the source property (see
/// [`RuleSource`]). A field without rules gets no attribute.
pub fn assemble_model_tokens_with(catalog: &ModelCatalog, json_schema: Option<&[SchemaComponent]>) -> TokenStream {
    let derive_json_schema = json_schema.is_some();
    let rules = json_schema.map(|components| RuleSource::new(catalog, components));
    let type_defs: Vec<TokenStream> = catalog
        .types
        .iter()
        .map(|model| {
            let tokens = match model {
                ModelDef::Struct(s) => generate_struct_with(s, derive_json_schema, rules.as_ref()),
                ModelDef::Enum(e) => generate_enum_with(e, derive_json_schema),
                ModelDef::Alias(a) => generate_alias_with(a, derive_json_schema),
                _ => TokenStream::new(),
            };
            // Only nominal declarations implement Registration: aliases reuse
            // their underlying implementation and containers stay inline.
            let nominal = match model {
                ModelDef::Struct(s) => Some(&s.name),
                ModelDef::Enum(e) => Some(&e.name),
                ModelDef::Alias(a) if is_nominal_alias(a) => Some(&a.name),
                _ => None,
            };
            if derive_json_schema && let Some(name) = nominal {
                let name = format_ident!("{}", name);
                quote! {
                    #tokens
                    impl schematic_define::Registration for #name {
                        const RUST_IDENTIFIER: Option<&'static str> = Some(stringify!(#name));
                        fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
                            ctx.register_nominal::<Self>()
                        }
                    }
                }
            } else { tokens }
        })
        .collect();

    let schema_import = if derive_json_schema {
        quote! { use schemars::JsonSchema; }
    } else {
        quote! {}
    };

    quote! {
        //! Generated model types.
        //!
        //! These types were generated from an OpenAPI specification.
        //! Do not edit manually.

        #schema_import
        use serde::{Deserialize, Serialize};
        use std::collections::HashMap;

        #(#type_defs)*
    }
}

/// Returns the extra derive applied when `schemars` support is requested.
fn json_schema_derive(enabled: bool) -> TokenStream {
    if enabled {
        quote! { #[derive(JsonSchema)] }
    } else {
        quote! {}
    }
}

/// Generates a Rust struct from a `StructDef`, optionally deriving `JsonSchema`.
///
/// `rules`, when given, supplies each field's rebuilt value-rule attribute.
fn generate_struct_with(def: &StructDef, derive_json_schema: bool, rules: Option<&RuleSource>) -> TokenStream {
    let name = format_ident!("{}", def.name);
    let doc = doc_comment(&def.description);

    let fields: Vec<TokenStream> = def
        .fields
        .iter()
        .map(|field| generate_field(field, rules.and_then(|rules| rules.field_attribute(&def.name, field))))
        .collect();

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
    let schema_derive = json_schema_derive(derive_json_schema);

    quote! {
        #doc
        #derives
        #schema_derive
        pub struct #name {
            #(#fields)*
            #extra_field
        }
    }
}

/// Generates a single struct field, with `rules` (a `#[schemars(..)]`
/// attribute) after its serde attributes.
fn generate_field(def: &FieldDef, rules: Option<TokenStream>) -> TokenStream {
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
        #rules
        pub #name: #field_type,
    }
}

/// Alias chains longer than this are treated as cycles and carry no rules.
const MAX_ALIAS_DEPTH: usize = 32;

/// The imported source schemas a definitions-out module rebuilds value rules
/// from.
///
/// Every rule is written through `extend`, never a native attribute such as
/// `length` or `range`: Schemars drops a native rule without a diagnostic when
/// the field's type does not match it, cannot express `exclusiveMinimum`,
/// `multipleOf`, `enum`, `const`, or `uniqueItems` at all, and turns an
/// unsuffixed large literal into an `i32` overflow. `extend` always attaches
/// and takes JSON values, so one spelling covers every keyword and every
/// exact-number stand-in.
///
/// `format` is an annotation, not a rule, and is not rebuilt: Schemars writes
/// its own (`int64`, `double`) for the Rust type.
pub(crate) struct RuleSource<'a> {
    /// Source schema objects by model name.
    components: HashMap<&'a str, &'a Map<String, Value>>,
    /// Targets of the aliases emitted as `pub type`, which Schemars inlines
    /// rather than registering as components.
    aliases: HashMap<&'a str, &'a TypeRef>,
}

impl<'a> RuleSource<'a> {
    /// Indexes `components` by name and the catalog's plain aliases.
    pub(crate) fn new(catalog: &'a ModelCatalog, components: &'a [SchemaComponent]) -> Self {
        let components = components
            .iter()
            .filter_map(|component| Some((component.name.as_str(), component.schema.as_object()?)))
            .collect();
        let aliases = catalog
            .types
            .iter()
            .filter_map(|model| match model {
                ModelDef::Alias(alias) if !is_nominal_alias(alias) => Some((alias.name.as_str(), &alias.target)),
                _ => None,
            })
            .collect();
        Self { components, aliases }
    }

    /// The `#[schemars(extend(..))]` attribute rebuilding the rules of
    /// `field`'s property in `model`'s source schema, found by wire name.
    ///
    /// An optional field's `enum` gains `null`, and its `const` becomes a
    /// two-member `enum` with `null`, because Schemars admits `null` for the
    /// `Option` and the rule must not reject what the type accepts. `None`
    /// when the property has no rule this field can carry.
    pub(crate) fn field_attribute(&self, model: &str, field: &FieldDef) -> Option<TokenStream> {
        let wire_name = field.serde_rename.as_deref().unwrap_or(&field.name);
        let own = self
            .components
            .get(model)
            .and_then(|schema| schema.get("properties")?.get(wire_name)?.as_object())
            .map(rule_keywords)
            .unwrap_or_default();
        let mut rules = self.rules(&field.field_type, own, true, 0);
        if rules.is_empty() {
            return None;
        }
        if !field.required || matches!(field.field_type, TypeRef::Optional(_)) {
            admit_null(&mut rules);
        }
        let entries = rules.iter().map(|(keyword, value)| {
            let value = json_tokens(value);
            quote! { #keyword = #value }
        });
        Some(quote! { #[schemars(extend(#(#entries),*))] })
    }

    /// The rules of a JSON request body whose type is `reference`, when it
    /// names a plain alias: the generated alias is not a component once
    /// assembled, so its rules belong on the endpoint. `None` otherwise or
    /// when the alias has no rules.
    pub(crate) fn body_constraints(&self, reference: &TypeRef) -> Option<Constraints> {
        let TypeRef::Named(name) = reference else { return None };
        if !self.aliases.contains_key(name.as_str()) {
            return None;
        }
        let rules = self.rules(reference, Map::new(), false, 0);
        (!rules.is_empty()).then(|| Constraints::from_schema(&rules))
    }

    /// `own` merged beneath the rules of every alias `type_ref` passes
    /// through (an outer rule wins), keeping only what the inlined schema of
    /// `type_ref`'s Rust type can carry.
    ///
    /// Item rules survive only for items of a primitive or array type, the
    /// items whose schema `extend` can restate whole. With `with_base`, the
    /// `items` object also carries the `type` (and `format`) Schemars would
    /// have written, because `extend` replaces Schemars' own `items`. A named
    /// model's rules live in its own component, so a field typed by one
    /// carries none.
    fn rules(&self, type_ref: &TypeRef, mut own: Map<String, Value>, with_base: bool, depth: usize) -> Map<String, Value> {
        match type_ref {
            TypeRef::Optional(inner) => self.rules(inner, own, with_base, depth),
            TypeRef::Named(name) => match self.aliases.get(name.as_str()) {
                Some(target) if depth < MAX_ALIAS_DEPTH => {
                    let mut merged = self.components.get(name.as_str()).map(|schema| rule_keywords(schema)).unwrap_or_default();
                    merged.append(&mut own);
                    self.rules(target, merged, with_base, depth + 1)
                }
                _ => Map::new(),
            },
            TypeRef::Primitive(PrimitiveType::String | PrimitiveType::Integer | PrimitiveType::Number | PrimitiveType::Boolean) => {
                own.remove("items");
                own
            }
            TypeRef::Array(item) => {
                let item_own = match own.remove("items") {
                    Some(Value::Object(item_own)) => item_own,
                    _ => Map::new(),
                };
                let mut item_rules = self.rules(item, item_own, with_base, depth);
                if item_rules.is_empty() {
                    return own;
                }
                if with_base {
                    let Some(mut base) = self.base_schema(item, depth) else { return own };
                    base.append(&mut item_rules);
                    item_rules = base;
                }
                own.insert("items".to_owned(), Value::Object(item_rules));
                own
            }
            _ => Map::new(),
        }
    }

    /// The schema Schemars derives for `type_ref`'s Rust type, for the types
    /// whose schema is inline: primitives other than JSON and bytes, arrays of
    /// them, and plain aliases of either.
    fn base_schema(&self, type_ref: &TypeRef, depth: usize) -> Option<Map<String, Value>> {
        let schema = match type_ref {
            TypeRef::Primitive(PrimitiveType::String) => serde_json::json!({"type": "string"}),
            TypeRef::Primitive(PrimitiveType::Integer) => serde_json::json!({"type": "integer", "format": "int64"}),
            TypeRef::Primitive(PrimitiveType::Number) => serde_json::json!({"type": "number", "format": "double"}),
            TypeRef::Primitive(PrimitiveType::Boolean) => serde_json::json!({"type": "boolean"}),
            TypeRef::Array(item) => serde_json::json!({"type": "array", "items": self.base_schema(item, depth)?}),
            TypeRef::Named(name) if depth < MAX_ALIAS_DEPTH => return self.base_schema(self.aliases.get(name.as_str())?, depth + 1),
            _ => return None,
        };
        match schema {
            Value::Object(schema) => Some(schema),
            _ => None,
        }
    }
}

/// Whether `alias` is emitted as a real enum or struct (a non-empty
/// combinator), and so registered under its own name.
fn is_nominal_alias(alias: &TypeAlias) -> bool {
    matches!(&alias.target, TypeRef::OneOf(v) | TypeRef::AnyOf(v) | TypeRef::AllOf(v) if !v.is_empty())
}

/// The value rules of `schema` as JSON Schema keywords
/// ([`Constraints::to_schema`]): numeric exclusive bounds, exact numbers as
/// JSON numbers or stand-ins with their `x-schematic-exact` text, and an
/// `items` object of item rules. `format` is dropped at every level.
fn rule_keywords(schema: &Map<String, Value>) -> Map<String, Value> {
    fn without_format(rules: &mut Constraints) {
        rules.format = None;
        if let Some(items) = rules.items.as_deref_mut() {
            without_format(items);
        }
        if rules.items.as_deref().is_some_and(Constraints::is_empty) {
            rules.items = None;
        }
    }
    let mut rules = Constraints::from_schema(schema);
    without_format(&mut rules);
    rules.to_schema()
}

/// Makes `rules` admit `null`: a non-null `const` becomes `enum: [const,
/// null]` when there is no `enum`, and an `enum` without `null` gains it.
fn admit_null(rules: &mut Map<String, Value>) {
    if !rules.contains_key("enum")
        && let Some(constant) = rules.get("const").filter(|constant| !constant.is_null()).cloned()
    {
        rules.remove("const");
        rules.insert("enum".to_owned(), Value::Array(vec![constant, Value::Null]));
    }
    if let Some(Value::Array(values)) = rules.get_mut("enum")
        && !values.contains(&Value::Null)
    {
        values.push(Value::Null);
    }
}

/// `value` in `serde_json::json!` syntax. Integers carry a `u64`/`i64`
/// suffix, because `json!` types an unsuffixed literal as `i32`; a float is
/// written with an `f64` suffix in the shortest text that reads back as the
/// same `f64`.
pub(crate) fn json_tokens(value: &Value) -> TokenStream {
    match value {
        Value::Null => quote! { null },
        Value::Bool(flag) => quote! { #flag },
        Value::Number(number) => {
            let literal = if let Some(unsigned) = number.as_u64() {
                Literal::u64_suffixed(unsigned)
            } else if let Some(signed) = number.as_i64() {
                Literal::i64_suffixed(signed)
            } else {
                Literal::f64_suffixed(number.as_f64().expect("a JSON number that is no integer is a finite float"))
            };
            quote! { #literal }
        }
        Value::String(text) => quote! { #text },
        Value::Array(values) => {
            let values = values.iter().map(json_tokens);
            quote! { [#(#values),*] }
        }
        Value::Object(entries) => {
            let entries = entries.iter().map(|(key, value)| {
                let value = json_tokens(value);
                quote! { #key: #value }
            });
            quote! { { #(#entries),* } }
        }
    }
}

/// Generates a Rust enum from an `EnumDef`, optionally deriving `JsonSchema`.
fn generate_enum_with(def: &EnumDef, derive_json_schema: bool) -> TokenStream {
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
    let schema_derive = json_schema_derive(derive_json_schema);

    quote! {
        #doc
        #derives
        #schema_derive
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

/// Generates a type alias, or a real composite type when the target is a
/// schema combinator.
///
/// A bare `pub type X = serde_json::Value` would erase the whole shape, so
/// `oneOf`/`anyOf` become enums (dispatched on their discriminator when the
/// alias carries one, else untagged) and `allOf` becomes a struct that
/// flattens its members.
fn generate_alias_with(def: &TypeAlias, derive_json_schema: bool) -> TokenStream {
    match &def.target {
        TypeRef::OneOf(variants) | TypeRef::AnyOf(variants) if !variants.is_empty() => {
            generate_union_enum(def, variants, derive_json_schema)
        }
        TypeRef::AllOf(members) if !members.is_empty() => {
            generate_composed_struct(def, members, derive_json_schema)
        }
        _ => {
            let name = format_ident!("{}", def.name);
            let doc = doc_comment(&def.description);
            let target = type_ref_to_tokens(&def.target);

            quote! {
                #doc
                pub type #name = #target;
            }
        }
    }
}

/// Emits an enum for a `oneOf` / `anyOf` schema.
///
/// It always serializes untagged: each alternative carries its own
/// discriminator member. With a discriminator whose tag sets align with the
/// alternatives, deserialization reads the property and decodes only an
/// alternative that accepts its value; without one it is serde's untagged
/// first match.
fn generate_union_enum(
    def: &TypeAlias,
    variants: &[TypeRef],
    derive_json_schema: bool,
) -> TokenStream {
    let name = format_ident!("{}", def.name);
    let doc = doc_comment(&def.description);
    let schema_derive = json_schema_derive(derive_json_schema);

    let mut used = HashSet::new();
    let variant_idents: Vec<_> = variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            let mut base = variant_name(variant, index);
            // Two variants can share a base name (two arrays, two strings);
            // suffixing keeps the enum well-formed.
            while !used.insert(base.clone()) {
                base = format!("{base}{index}");
            }
            format_ident!("{}", base)
        })
        .collect();

    let variant_types: Vec<_> = variants.iter().map(type_ref_to_tokens).collect();
    let first_ident = &variant_idents[0];

    let dispatch = def
        .discriminator
        .as_ref()
        .filter(|discriminator| discriminator.tags.len() == variants.len())
        .map(|discriminator| discriminated_deserialize(&name, discriminator, &variant_idents));
    let derives = if dispatch.is_some() {
        quote! { Debug, Clone, Serialize }
    } else {
        quote! { Debug, Clone, Serialize, Deserialize }
    };

    quote! {
        #doc
        // Union variants mirror the spec's shapes and routinely differ in size;
        // boxing them would push an indirection onto every caller for no gain.
        #[allow(clippy::large_enum_variant)]
        #[derive(#derives)]
        #schema_derive
        #[serde(untagged)]
        pub enum #name {
            #(#variant_idents(#variant_types),)*
        }

        #dispatch

        // Hand-written because `#[derive(Default)]` accepts `#[default]` only on
        // unit variants, and structs holding this type as a required field derive
        // `Default`.
        impl Default for #name {
            fn default() -> Self {
                Self::#first_ident(Default::default())
            }
        }
    }
}

/// A `Deserialize` impl that selects the alternative by its discriminator.
///
/// The property is read before any alternative is tried, so a value is never
/// accepted by an alternative whose tag differs. Alternatives sharing a value
/// are tried in declaration order. A missing, non-string, or unlisted value is
/// an error naming the union and the property.
fn discriminated_deserialize(
    name: &proc_macro2::Ident,
    discriminator: &UnionDiscriminator,
    variant_idents: &[proc_macro2::Ident],
) -> TokenStream {
    let property = &discriminator.property;
    // The messages are literals: format strings escape the spec's spelling.
    let escaped = property.replace('{', "{{").replace('}', "}}");
    let not_string = format!("`{name}` discriminator `{property}` is not a string");
    let unknown = format!("unknown `{escaped}` value `{{}}` for `{name}`");
    let failed = format!("`{name}` with `{escaped}` `{{}}`: {{}}");

    // Each value's candidate alternatives, in first-seen value order; values
    // sharing the same candidates share one match arm.
    let mut candidates: Vec<(&str, Vec<usize>)> = Vec::new();
    for (index, values) in discriminator.tags.iter().enumerate() {
        for value in values {
            match candidates.iter_mut().find(|(seen, _)| *seen == value.as_str()) {
                Some((_, indexes)) if !indexes.contains(&index) => indexes.push(index),
                Some(_) => {}
                None => candidates.push((value.as_str(), vec![index])),
            }
        }
    }
    let mut arms: Vec<(Vec<&str>, Vec<usize>)> = Vec::new();
    for (value, indexes) in candidates {
        match arms.iter_mut().find(|(_, seen)| *seen == indexes) {
            Some((values, _)) => values.push(value),
            None => arms.push((vec![value], indexes)),
        }
    }

    let arms = arms.iter().map(|(values, indexes)| {
        let (last, earlier) = indexes.split_last().expect("every arm has a candidate");
        let earlier = earlier.iter().map(|index| &variant_idents[*index]);
        let last = &variant_idents[*last];
        quote! {
            #(#values)|* => {
                #(if let Ok(decoded) = serde_json::from_value(value.clone()) {
                    return Ok(Self::#earlier(decoded));
                })*
                serde_json::from_value(value).map(Self::#last)
            }
        }
    });

    quote! {
        impl<'de> Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::de::Error as _;
                let value = serde_json::Value::deserialize(deserializer)?;
                let tag = match value.get(#property) {
                    Some(serde_json::Value::String(tag)) => tag.clone(),
                    Some(_) => return Err(D::Error::custom(#not_string)),
                    None => return Err(D::Error::missing_field(#property)),
                };
                let decoded = match tag.as_str() {
                    #(#arms)*
                    _ => return Err(D::Error::custom(format_args!(#unknown, tag))),
                };
                decoded.map_err(|error| D::Error::custom(format_args!(#failed, tag, error)))
            }
        }
    }
}

/// Emits a struct that flattens each member of an `allOf` schema.
fn generate_composed_struct(
    def: &TypeAlias,
    members: &[TypeRef],
    derive_json_schema: bool,
) -> TokenStream {
    let name = format_ident!("{}", def.name);
    let doc = doc_comment(&def.description);
    let schema_derive = json_schema_derive(derive_json_schema);

    let mut used = HashSet::new();
    let fields = members.iter().enumerate().map(|(index, member)| {
        let mut base = to_snake_case(&variant_name(member, index));
        while !used.insert(base.clone()) {
            base = format!("{base}_{index}");
        }
        let field = format_ident!("{}", base);
        let ty = type_ref_to_tokens(member);

        quote! {
            #[serde(flatten)]
            pub #field: #ty,
        }
    });

    quote! {
        #doc
        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        #schema_derive
        pub struct #name {
            #(#fields)*
        }
    }
}

/// Derives a PascalCase variant/member name for one arm of a combinator.
fn variant_name(type_ref: &TypeRef, index: usize) -> String {
    match type_ref {
        TypeRef::Named(name) => name.clone(),
        TypeRef::Primitive(PrimitiveType::String) => "Text".to_string(),
        TypeRef::Primitive(PrimitiveType::Integer) => "Integer".to_string(),
        TypeRef::Primitive(PrimitiveType::Number) => "Number".to_string(),
        TypeRef::Primitive(PrimitiveType::Boolean) => "Boolean".to_string(),
        TypeRef::Primitive(PrimitiveType::Bytes) => "Bytes".to_string(),
        TypeRef::Primitive(PrimitiveType::Json) => "Json".to_string(),
        TypeRef::Array(inner) => format!("{}List", variant_name(inner, index)),
        TypeRef::Map(inner) => format!("{}Map", variant_name(inner, index)),
        TypeRef::Optional(inner) => variant_name(inner, index),
        _ => format!("Variant{index}"),
    }
}

/// Converts a PascalCase name to snake_case for use as a field name.
fn to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (index, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if index > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
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
            // Imported names may be paths (`serde_json::Value`), which `format_ident!`
            // rejects; `type_name_to_tokens` accepts both spellings.
            crate::codegen::request_structs::shared::type_name_to_tokens(name)
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
            let text = tag_bare_code_fences(text.trim());
            quote! { #[doc = #text] }
        }
        None => TokenStream::new(),
    }
}

/// Tags untagged Markdown code fences in a description as `text`.
///
/// Spec descriptions carry illustrative snippets in bare ``` fences. Rustdoc
/// treats an untagged fence as Rust and tries to compile it, so OpenAI's
/// JavaScript-ish coordinate example would fail `cargo test --doc`.
fn tag_bare_code_fences(description: &str) -> String {
    let mut out = String::with_capacity(description.len() + 16);
    let mut inside_fence = false;

    for (index, line) in description.lines().enumerate() {
        if index > 0 {
            out.push('\n');
        }

        let trimmed = line.trim_end();
        let is_fence = trimmed.trim_start().starts_with("```");

        if is_fence && !inside_fence {
            inside_fence = true;
            // Only a fence with no info string needs one added.
            if trimmed.trim_start() == "```" {
                out.push_str(&trimmed.replace("```", "```text"));
                continue;
            }
        } else if is_fence {
            inside_fence = false;
        }

        out.push_str(trimmed);
    }

    // An unterminated fence would swallow the rest of the doc block.
    if inside_fence {
        out.push_str("\n```");
    }

    out
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
                discriminator: None,
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

    fn pet_union(discriminator: Option<UnionDiscriminator>) -> ModelCatalog {
        ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Alias(TypeAlias {
                name: "Pet".to_string(),
                description: None,
                target: TypeRef::OneOf(vec![TypeRef::Named("Cat".to_string()), TypeRef::Named("Dog".to_string())]),
                discriminator,
            })],
        }
    }

    #[test]
    fn union_with_discriminator_dispatches_in_a_generated_deserialize() {
        let discriminator = UnionDiscriminator {
            property: "kind".to_string(),
            tags: vec![vec!["cat".to_string()], vec!["dog".to_string(), "hound".to_string()]],
        };
        let tokens = assemble_model_tokens(&pet_union(Some(discriminator)));
        let code = validate_code(&tokens).map(|file| format_code(&file)).expect("valid code");

        assert!(code.contains("#[derive(Debug, Clone, Serialize)]\n#[serde(untagged)]\npub enum Pet"), "{code}");
        assert!(code.contains("impl<'de> Deserialize<'de> for Pet"), "{code}");
        assert!(code.contains("value.get(\"kind\")"), "{code}");
        assert!(code.contains("\"cat\" => serde_json::from_value(value).map(Self::Cat)"), "{code}");
        assert!(code.contains("\"dog\" | \"hound\" => serde_json::from_value(value).map(Self::Dog)"), "{code}");
    }

    #[test]
    fn union_without_or_with_misaligned_discriminator_stays_untagged() {
        let misaligned = UnionDiscriminator { property: "kind".to_string(), tags: vec![vec!["cat".to_string()]] };
        for catalog in [pet_union(None), pet_union(Some(misaligned))] {
            let code = validate_code(&assemble_model_tokens(&catalog)).map(|file| format_code(&file)).expect("valid code");

            assert!(code.contains("#[derive(Debug, Clone, Serialize, Deserialize)]\n#[serde(untagged)]\npub enum Pet"), "{code}");
            assert!(!code.contains("impl<'de> Deserialize<'de>"), "{code}");
        }
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

    fn field(name: &str, rename: Option<&str>, required: bool, field_type: TypeRef) -> FieldDef {
        FieldDef { name: name.into(), serde_rename: rename.map(str::to_string), description: None, required, field_type }
    }

    fn primitive(primitive: PrimitiveType) -> TypeRef {
        TypeRef::Primitive(primitive)
    }

    /// A model with one field per rule shape, an unconstrained model, two
    /// primitive aliases, and their source components.
    fn constrained_catalog() -> (ModelCatalog, Vec<SchemaComponent>) {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![
                ModelDef::Struct(StructDef {
                    name: "Widget".into(),
                    description: None,
                    fields: vec![
                        field("name", None, true, primitive(PrimitiveType::String)),
                        field("count", None, true, primitive(PrimitiveType::Integer)),
                        field("ratio", None, true, primitive(PrimitiveType::Number)),
                        field("tags", None, true, TypeRef::Array(Box::new(primitive(PrimitiveType::String)))),
                        field("display_name", Some("displayName"), true, primitive(PrimitiveType::String)),
                        field("mode", None, false, primitive(PrimitiveType::String)),
                        field("kind", None, false, primitive(PrimitiveType::String)),
                        field("slug", None, true, TypeRef::Named("Slug".into())),
                        field("codes", None, true, TypeRef::Array(Box::new(TypeRef::Named("Code".into())))),
                        field("big", None, true, primitive(PrimitiveType::Integer)),
                        field("owner", None, true, TypeRef::Named("Plain".into())),
                        field("plain", None, true, primitive(PrimitiveType::String)),
                    ],
                    additional_properties: None,
                }),
                ModelDef::Struct(StructDef {
                    name: "Plain".into(),
                    description: None,
                    fields: vec![
                        field("label", None, true, primitive(PrimitiveType::String)),
                        field("size", None, false, primitive(PrimitiveType::Integer)),
                    ],
                    additional_properties: None,
                }),
                ModelDef::Alias(TypeAlias { name: "Slug".into(), description: None, target: primitive(PrimitiveType::String), discriminator: None }),
                ModelDef::Alias(TypeAlias { name: "Code".into(), description: None, target: primitive(PrimitiveType::String), discriminator: None }),
            ],
        };
        let component = |name: &str, schema: Value| SchemaComponent { name: name.into(), rust_path: Some(name.into()), schema };
        let components = vec![
            component("Widget", serde_json::json!({"type": "object", "properties": {
                "name": {"type": "string", "minLength": 1, "maxLength": 64, "pattern": "^[a-z]+$", "format": "slug"},
                "count": {"type": "integer", "format": "int32", "minimum": -5, "maximum": 100, "multipleOf": 5},
                "ratio": {"type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 1.5},
                "tags": {"type": "array", "minItems": 1, "maxItems": 10, "uniqueItems": true, "items": {"type": "string", "maxLength": 5}},
                "displayName": {"type": "string", "minLength": 2},
                "mode": {"type": "string", "enum": ["fast", "slow"]},
                "kind": {"type": "string", "const": "v1"},
                "slug": {"$ref": "#/components/schemas/Slug"},
                "codes": {"type": "array", "items": {"$ref": "#/components/schemas/Code"}},
                "big": {"type": "integer", "maximum": 1.8446744073709552e19, "x-schematic-exact": {"maximum": "18446744073709551616"}},
                "owner": {"$ref": "#/components/schemas/Plain"},
                "plain": {"type": "string", "description": "no rules"}
            }})),
            component("Plain", serde_json::json!({"type": "object", "properties": {"label": {"type": "string"}, "size": {"type": "integer"}}})),
            component("Slug", serde_json::json!({"type": "string", "maxLength": 8, "pattern": "^[a-z-]+$"})),
            component("Code", serde_json::json!({"type": "string", "minLength": 3})),
        ];
        (catalog, components)
    }

    /// The emitted source of `model` without whitespace, so assertions do not
    /// depend on the formatter's line breaks.
    fn compact_model_source(catalog: &ModelCatalog, components: &[SchemaComponent], model: &str) -> String {
        let code = format_code(&validate_code(&assemble_model_tokens_with(catalog, Some(components))).expect("emitted models parse"));
        let start = code.find(&format!("pub struct {model} ")).expect("model is emitted");
        let end = code[start..].find("\n}").map_or(code.len(), |end| start + end);
        code[start..end].split_whitespace().collect()
    }

    #[test]
    fn definitions_out_fields_rebuild_every_source_rule_through_extend() {
        let (catalog, components) = constrained_catalog();

        let widget = compact_model_source(&catalog, &components, "Widget");

        for expected in [
            r#"#[schemars(extend("maxLength"=64u64,"minLength"=1u64,"pattern"="^[a-z]+$"))]pubname:String"#,
            r#"#[schemars(extend("maximum"=100u64,"minimum"=-5i64,"multipleOf"=5u64))]pubcount:i64"#,
            r#"#[schemars(extend("exclusiveMaximum"=1.5f64,"exclusiveMinimum"=0u64))]pubratio:f64"#,
            r#"#[schemars(extend("items"={"maxLength":5u64,"type":"string"},"maxItems"=10u64,"minItems"=1u64,"uniqueItems"=true))]pubtags:Vec<String>"#,
            r#"#[serde(rename="displayName")]#[schemars(extend("minLength"=2u64))]pubdisplay_name:String"#,
            r#"#[schemars(extend("enum"=["fast","slow",null]))]pubmode:Option<String>"#,
            r#"#[schemars(extend("enum"=["v1",null]))]pubkind:Option<String>"#,
            r#"#[schemars(extend("maxLength"=8u64,"pattern"="^[a-z-]+$"))]pubslug:Slug"#,
            r#"#[schemars(extend("items"={"minLength":3u64,"type":"string"}))]pubcodes:Vec<Code>"#,
            r#"#[schemars(extend("maximum"=18446744073709552000f64,"x-schematic-exact"={"maximum":"18446744073709551616"}))]pubbig:i64"#,
            "pubowner:Plain,pubplain:String,",
        ] {
            assert!(widget.contains(expected), "missing {expected}\n{widget}");
        }
        assert!(!widget.contains("format"), "format is an annotation, not a rule: {widget}");
    }

    #[test]
    fn definitions_out_models_without_rules_emit_no_schemars_attribute() {
        let (catalog, components) = constrained_catalog();

        let plain = compact_model_source(&catalog, &components, "Plain");

        assert!(!plain.contains("schemars("), "{plain}");
        let client = format_code(&validate_code(&assemble_model_tokens(&catalog)).unwrap());
        assert!(!client.contains("schemars"), "the client path has no rules: {client}");
    }

    #[test]
    fn a_nullable_source_enum_keeps_its_single_null() {
        let mut rules = serde_json::json!({"enum": ["a", null]}).as_object().unwrap().clone();
        admit_null(&mut rules);
        assert_eq!(Value::Object(rules), serde_json::json!({"enum": ["a", null]}));
    }

    #[test]
    fn plain_alias_body_rules_become_constraints_but_nominal_bodies_do_not() {
        let (catalog, components) = constrained_catalog();
        let rules = RuleSource::new(&catalog, &components);

        assert_eq!(
            rules.body_constraints(&TypeRef::Named("Slug".into())),
            Some(Constraints::new().max_length(8).pattern("^[a-z-]+$"))
        );
        assert_eq!(rules.body_constraints(&TypeRef::Named("Plain".into())), None);
        assert_eq!(rules.body_constraints(&primitive(PrimitiveType::String)), None);
    }
}
