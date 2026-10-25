//! Model code generation from `ModelCatalog`.
//!
//! Transforms `ModelCatalog` type definitions into Rust source code with
//! serde derives, doc comments, and correct type mappings.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use schematic_define::SchemaComponent;
use schematic_define::constraints::{Constraints, admits_only_null};
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
/// - A source rule cannot be rebuilt (see [`assemble_model_tokens_with`])
/// - Generated code fails syn validation
/// - File writing fails (when not dry-run)
pub fn generate_models_with(
    catalog: &ModelCatalog,
    json_schema: Option<&[SchemaComponent]>,
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    let tokens = assemble_model_tokens_with(catalog, json_schema)?;
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
    assemble_models(catalog, None)
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
/// rules, and a model whose own schema root or extra-properties schema does,
/// gets a `#[schemars(extend(..))]` attribute, so the schema Schemars derives
/// states the same rules as the source (see [`RuleSource`]); a field whose
/// free-form value references a model gets a `schema_with` function instead
/// (see [`write_rules`]). A field or model without rules gets no attribute.
///
/// ## Errors
///
/// Returns `GeneratorError::CodeGenError` naming each source rule the
/// emitted Rust types cannot carry (see [`RuleSource::finish`]) rather than
/// emitting models without it.
pub fn assemble_model_tokens_with(catalog: &ModelCatalog, json_schema: Option<&[SchemaComponent]>) -> Result<TokenStream, GeneratorError> {
    let rules = json_schema.map(|components| RuleSource::new(catalog, components));
    let tokens = assemble_models(catalog, rules.as_ref());
    rules.as_ref().map_or(Ok(()), RuleSource::finish)?;
    Ok(tokens)
}

/// The models of `catalog`, with `schemars` support and rebuilt rules when
/// `rules` is given.
fn assemble_models(catalog: &ModelCatalog, rules: Option<&RuleSource>) -> TokenStream {
    let derive_json_schema = rules.is_some();
    let type_defs: Vec<TokenStream> = catalog
        .types
        .iter()
        .map(|model| {
            let tokens = match model {
                ModelDef::Struct(s) => generate_struct_with(s, derive_json_schema, rules),
                ModelDef::Enum(e) => generate_enum_with(e, derive_json_schema, model_rules(rules, model)),
                ModelDef::Alias(a) => generate_alias_with(a, derive_json_schema, rules),
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

/// The container `#[schemars(extend(..))]` attribute of `model`, when a rule
/// source is given.
fn model_rules(rules: Option<&RuleSource>, model: &ModelDef) -> Option<TokenStream> {
    rules.and_then(|rules| rules.model_attribute(model))
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
/// `rules`, when given, supplies each field's rebuilt value rules, the model
/// root's, and those of the flattened extra-properties values (see
/// [`write_rules`] for how each is written).
fn generate_struct_with(def: &StructDef, derive_json_schema: bool, rules: Option<&RuleSource>) -> TokenStream {
    let name = format_ident!("{}", def.name);
    let doc = doc_comment(&def.description);
    let mut helpers = Vec::new();

    let fields: Vec<TokenStream> = def
        .fields
        .iter()
        .map(|field| {
            let attributes = rules.and_then(|rules| {
                let function = format_ident!("field_rules_schema_{}", field.name.trim_start_matches("r#"));
                let rust_type = field_type_tokens(field);
                let optional = !field.required || matches!(field.field_type, TypeRef::Optional(_));
                write_rules(&name, &function, rust_type, optional, &rules.field_rules(&def.name, field), &mut helpers)
            });
            generate_field(field, attributes)
        })
        .collect();

    let (mut container, extra) = rules.map(|rules| rules.struct_rules(def)).unwrap_or_default();
    // If the struct has additional_properties, add a flattened HashMap field
    let extra_field = def.additional_properties.as_ref().map(|type_ref| {
        let value_type = type_ref_to_tokens(type_ref);
        // Extra values that reach a model through a free-form value are
        // written by the flattened field's own schema; Schemars merges it
        // into the struct's.
        let attributes = match extra {
            Some(extra) if contains_model_reference(&extra) => {
                let rules = Map::from_iter([("additionalProperties".to_owned(), Value::Object(extra))]);
                let function = format_ident!("extra_values_rules_schema");
                write_rules(&name, &function, quote! { HashMap<String, #value_type> }, false, &rules, &mut helpers)
            }
            Some(extra) => {
                container.insert("additionalProperties".to_owned(), Value::Object(extra));
                None
            }
            None => None,
        };
        quote! {
            /// Additional properties not captured by named fields.
            #[serde(flatten)]
            #attributes
            pub additional_properties: HashMap<String, #value_type>,
        }
    });
    let container_rules = extend_attribute(&container);

    // Determine if we can derive Default
    // We can if all required fields have types that impl Default (primitives, Option, Vec)
    // For simplicity, derive Default for all structs - users can manually implement if needed
    let derives = quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize)] };
    let schema_derive = json_schema_derive(derive_json_schema);
    let helpers = (!helpers.is_empty()).then(|| quote! { impl #name { #(#helpers)* } });

    quote! {
        #doc
        #derives
        #schema_derive
        #container_rules
        pub struct #name {
            #(#fields)*
            #extra_field
        }
        #helpers
    }
}

/// The Rust type of `def`'s field: its type, in an `Option` when optional.
fn field_type_tokens(def: &FieldDef) -> TokenStream {
    let field_type = type_ref_to_tokens(&def.field_type);
    if def.required { field_type } else { quote! { Option<#field_type> } }
}

/// Generates a single struct field, with `rules` (its `#[schemars(..)]`
/// attributes) after its serde attributes.
fn generate_field(def: &FieldDef, rules: Option<TokenStream>) -> TokenStream {
    let name = format_ident!("{}", def.name);
    let doc = doc_comment(&def.description);
    let field_type = field_type_tokens(def);

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

/// The attributes writing the rebuilt `rules` of a value of `rust_type`
/// (`optional` when it is an `Option`) on a field of the struct `model`;
/// `None` when there are none.
///
/// Rules that reach no model through a free-form value are an `extend`
/// attribute. Otherwise they are a `schema_with` attribute naming
/// `model::function`, an associated function pushed onto `helpers` that
/// starts from the schema Schemars derives for `rust_type` and writes each
/// rule keyword over it, with each referenced model as
/// `generator.subschema_for::<Model>()`. A free-form value's Rust type does
/// not name the models its schema references, so only the generator call
/// adds each one to the schema's definitions wherever the field's type is
/// reached, nested or not; Schemars resolves recursion through them.
///
/// `schema_with` hides the field's `Option` from Schemars, which would then
/// list the field as required, so an `Option` field also gets
/// `#[serde(default)]`: Serde already reads a missing `Option` as `None`,
/// and Schemars treats a defaulted field as optional, as it does an `Option`.
fn write_rules(
    model: &proc_macro2::Ident,
    function: &proc_macro2::Ident,
    rust_type: TokenStream,
    optional: bool,
    rules: &Map<String, Value>,
    helpers: &mut Vec<TokenStream>,
) -> Option<TokenStream> {
    if !contains_model_reference(rules) {
        return extend_attribute(rules);
    }
    let mut bindings = Vec::new();
    let inserts: Vec<TokenStream> = rules
        .iter()
        .filter(|(keyword, _)| keyword.as_str() != MODEL_REFERENCE)
        .map(|(keyword, value)| {
            let value = schema_value_tokens(value, &mut bindings);
            quote! { schema.insert(#keyword.to_owned(), serde_json::json!(#value)); }
        })
        .collect();
    // A value that is itself a reference takes the generator's `$ref`.
    let reference = rules.get(MODEL_REFERENCE).and_then(Value::as_str).map(|model| {
        let model = format_ident!("{}", model);
        quote! {
            if let Some(reference) = generator.subschema_for::<#model>().as_object() {
                for (keyword, value) in reference {
                    schema.insert(keyword.clone(), value.clone());
                }
            }
        }
    });
    helpers.push(quote! {
        fn #function(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
            #(#bindings)*
            let mut schema = generator.subschema_for::<#rust_type>();
            #reference
            #(#inserts)*
            schema
        }
    });
    let path = format!("{model}::{function}");
    let default = optional.then(|| quote! { #[serde(default)] });
    Some(quote! { #default #[schemars(schema_with = #path)] })
}

/// `value` in `serde_json::json!` syntax, as [`json_tokens`] writes it, except
/// that each [`MODEL_REFERENCE`] object is a binding pushed onto `bindings`:
/// the generator's schema for the model, with the object's other keywords
/// written over it. A nested reference's binding is pushed first.
fn schema_value_tokens(value: &Value, bindings: &mut Vec<TokenStream>) -> TokenStream {
    match value {
        Value::Object(entries) => match entries.get(MODEL_REFERENCE).and_then(Value::as_str) {
            Some(model) => {
                let siblings: Vec<(&String, TokenStream)> = entries
                    .iter()
                    .filter(|(keyword, _)| keyword.as_str() != MODEL_REFERENCE)
                    .map(|(keyword, value)| (keyword, schema_value_tokens(value, bindings)))
                    .collect();
                let binding = format_ident!("reference_{}", bindings.len());
                let model = format_ident!("{}", model);
                bindings.push(if siblings.is_empty() {
                    quote! { let #binding = generator.subschema_for::<#model>(); }
                } else {
                    let inserts = siblings.iter().map(|(keyword, value)| quote! { #binding.insert(#keyword.to_owned(), serde_json::json!(#value)); });
                    quote! { let mut #binding = generator.subschema_for::<#model>(); #(#inserts)* }
                });
                quote! { #binding }
            }
            None => {
                let entries: Vec<TokenStream> = entries
                    .iter()
                    .map(|(keyword, value)| {
                        let value = schema_value_tokens(value, bindings);
                        quote! { #keyword: #value }
                    })
                    .collect();
                quote! { { #(#entries),* } }
            }
        },
        Value::Array(values) => {
            let values: Vec<TokenStream> = values.iter().map(|value| schema_value_tokens(value, bindings)).collect();
            quote! { [#(#values),*] }
        }
        scalar => json_tokens(scalar),
    }
}

/// Whether `rules` hold a [`MODEL_REFERENCE`] anywhere.
fn contains_model_reference(rules: &Map<String, Value>) -> bool {
    fn within(value: &Value) -> bool {
        match value {
            Value::Object(entries) => contains_model_reference(entries),
            Value::Array(values) => values.iter().any(within),
            _ => false,
        }
    }
    rules.contains_key(MODEL_REFERENCE) || rules.values().any(within)
}

/// Alias chains longer than this are treated as cycles; a rule they would
/// carry is a problem [`RuleSource::finish`] reports.
const MAX_ALIAS_DEPTH: usize = 32;

/// The key of a rebuilt free-form schema object that references the catalog
/// model it names, beside the object's other keywords. It never reaches the
/// emitted schema: [`write_rules`] writes such an object as the generator's
/// schema for the model with the other keywords written over it.
const MODEL_REFERENCE: &str = "$schematicModel";

/// The imported source schemas a definitions-out module rebuilds value rules
/// from.
///
/// A rule is rebuilt wherever the emitted Rust type places the schema it
/// sits on: a field, the items of an array, the values of a map or of a
/// struct's flattened extra properties, a model's own root, and the nested
/// schemas of a free-form JSON value. The Rust type decides only where a
/// rule is written, never whether it is kept; a rule beside a reference to
/// another model is written beside that occurrence, so the shared component
/// is unchanged.
///
/// A free-form value keeps a component reference whose target holds a rule
/// (directly or through another reference), so the rebuilt definition
/// reaches that rule as the direct import does, and keeps every composition
/// with a rule beneath it, members in source order: the two generated plans
/// pass through (a nullable alternative and a one-member `allOf`) and any
/// other `allOf`, `anyOf`, `oneOf`, or `not`, which they do not enter. So
/// validation of the rebuilt definition refuses, or with the API's
/// unchecked-composition opt-in discloses, each rule beneath such a
/// composition as it does for the direct import. A nominal union's variants
/// and a flattened `allOf` struct's source `allOf` are rebuilt the same way
/// (see [`Self::variant_rules`] and [`Self::alias_attribute`]).
///
/// Two plain-alias shapes reach rules the rebuilt Rust form has nowhere to
/// put, and [`Self::finish`] reports them instead of dropping the rules: a
/// plain alias reached again through a free-form value while its own rules
/// are being rebuilt (see [`Self::free_form_rules`]), and a JSON body typed
/// as a plain alias whose rules [`Constraints`] cannot hold (see
/// [`Self::body_constraints`]).
///
/// Every rule is written through `extend`, never a native attribute such as
/// `length` or `range`: Schemars drops a native rule without a diagnostic when
/// the field's type does not match it, cannot express `exclusiveMinimum`,
/// `multipleOf`, `enum`, `const`, or `uniqueItems` at all, and turns an
/// unsuffixed large literal into an `i32` overflow. `extend` always attaches
/// and takes JSON values, so one spelling covers every keyword and every
/// exact-number stand-in. `format` is rebuilt too, replacing the one Schemars
/// writes for the Rust type; it stays an annotation. The exception is a rule
/// set that references a model from inside a free-form value, which
/// [`write_rules`] writes through `schema_with` instead.
pub(crate) struct RuleSource<'a> {
    /// Source schema objects by model name.
    components: HashMap<&'a str, &'a Map<String, Value>>,
    /// Targets of the aliases emitted as `pub type`, which Schemars inlines
    /// rather than registering as components.
    aliases: HashMap<&'a str, &'a TypeRef>,
    /// The components whose source schema, or a component it references,
    /// holds a validation rule (`format` is an annotation, not a rule).
    /// Rules and references beneath a composition count too: the rebuild
    /// keeps a composition with a rule beneath it, and a nominal union's
    /// variants reach their models as Rust types.
    ruled: HashSet<&'a str>,
    /// The plain aliases whose rules are being rebuilt for a free-form
    /// reference, so a reference back to one ends the expansion.
    expanding: RefCell<Vec<&'a str>>,
    /// Source rules found that the rebuilt definition cannot carry, in the
    /// order found; reported by [`Self::finish`].
    problems: RefCell<Vec<String>>,
}

impl<'a> RuleSource<'a> {
    /// Indexes `components` by name and the catalog's plain aliases.
    pub(crate) fn new(catalog: &'a ModelCatalog, components: &'a [SchemaComponent]) -> Self {
        let components: HashMap<&'a str, &'a Map<String, Value>> = components
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
        let ruled = ruled_components(&components);
        Self { components, aliases, ruled, expanding: RefCell::new(Vec::new()), problems: RefCell::new(Vec::new()) }
    }

    /// Fails when any rebuild so far met a source rule the rebuilt
    /// definition cannot carry. Call it once every rule has been read.
    ///
    /// ## Errors
    ///
    /// `GeneratorError::CodeGenError` listing each such rule with its
    /// component, location, and how to make it rebuildable.
    pub(crate) fn finish(&self) -> Result<(), GeneratorError> {
        let problems = self.problems.borrow();
        if problems.is_empty() {
            return Ok(());
        }
        Err(GeneratorError::CodeGenError(format!(
            "--definitions-out cannot rebuild every imported value rule:\n- {}",
            problems.join("\n- ")
        )))
    }

    /// Records `problem` once.
    fn problem(&self, problem: String) {
        let mut problems = self.problems.borrow_mut();
        if !problems.contains(&problem) {
            problems.push(problem);
        }
    }

    /// The rebuilt rules of `field`'s property in `model`'s source schema,
    /// found by wire name; empty when the property has none.
    ///
    /// An `enum` or `const` is copied as the source wrote it: Schemars' `Option`
    /// admits the null kind, but null is an allowed value only where the
    /// source lists it, and an absent optional field is never checked.
    pub(crate) fn field_rules(&self, model: &str, field: &FieldDef) -> Map<String, Value> {
        let wire_name = field.serde_rename.as_deref().unwrap_or(&field.name);
        let source = self
            .components
            .get(model)
            .and_then(|schema| schema.get("properties")?.get(wire_name)?.as_object());
        self.rules(&field.field_type, source.cloned().unwrap_or_default(), true, 0)
    }

    /// The rebuilt rules of a struct's source root, and, for a struct with
    /// extra properties, the `additionalProperties` schema carrying the
    /// value rules, which replaces the one Schemars derives from the
    /// flattened map (`None` when the values have no rules).
    pub(crate) fn struct_rules(&self, def: &StructDef) -> (Map<String, Value>, Option<Map<String, Value>>) {
        let Some(source) = self.components.get(def.name.as_str()) else { return Default::default() };
        let extra = def
            .additional_properties
            .as_ref()
            .and_then(|extra| self.slot(extra, source.get("additionalProperties"), true, 0));
        (own_rules(source), extra)
    }

    /// The container `#[schemars(extend(..))]` attribute of a unit enum
    /// `model`: the rules of its source root, except `enum`, which is its
    /// variants and which Schemars writes. `None` for any other model (a
    /// struct's are [`Self::struct_rules`], a nominal alias' are
    /// [`Self::alias_attribute`]) or one without rules.
    pub(crate) fn model_attribute(&self, model: &ModelDef) -> Option<TokenStream> {
        let ModelDef::Enum(def) = model else { return None };
        let mut rules = own_rules(self.components.get(def.name.as_str())?);
        rules.remove("enum");
        extend_attribute(&rules)
    }

    /// The container `#[schemars(extend(..))]` attribute of the nominal
    /// alias `def`: the rules of its source root and, for an `allOf`
    /// emitted as a struct that flattens its members, the source `allOf`
    /// itself whenever a rule sits beneath it (see
    /// [`Self::composed_members`]). Schemars merges the flattened members'
    /// properties into the struct, which no longer says that they came from
    /// a composition, so the kept `allOf` gives validation of the rebuilt
    /// definition the composition the direct import has. The merged copies
    /// stay, so a member's rule can be reported twice (beneath the kept
    /// `allOf` and at its merged property); validation of a rebuilt API is
    /// a superset of the direct import's, never a subset. A model a member
    /// references is written as a `$ref` to its component, which every
    /// nominal model registers. `None` for a plain alias or one without
    /// rules.
    pub(crate) fn alias_attribute(&self, def: &TypeAlias) -> Option<TokenStream> {
        if !is_nominal_alias(def) {
            return None;
        }
        let source = self.components.get(def.name.as_str())?;
        let mut rules = own_rules(source);
        if matches!(def.target, TypeRef::AllOf(_))
            && let Some(Value::Array(members)) = source.get("allOf")
            && let Some(mut members) = self.composed_members(members, false)
        {
            members.iter_mut().for_each(|member| self.literal_references(member));
            rules.insert("allOf".to_owned(), Value::Array(members));
        }
        extend_attribute(&rules)
    }

    /// The rebuilt rules of each variant of the nominal union `def`, whose
    /// variants are `variants`, from the members of its source `oneOf` or
    /// `anyOf`, which import maps one to one and in order. The variant's
    /// Rust type places them as it places a field's (see [`Self::rules`]).
    /// Empty when no variant has rules or the source members do not line up
    /// with the variants.
    ///
    /// Beside a variant with rules, a member that admits only null is
    /// written as the source wrote it: its variant's Rust type is a
    /// free-form value, whose schema admits anything, and without the
    /// member's `type` the union would no longer read as an `Option`'s
    /// nullable alternative, which generated checks pass through, but as a
    /// composition they refuse.
    pub(crate) fn variant_rules(&self, def: &TypeAlias, variants: &[TypeRef]) -> Vec<Map<String, Value>> {
        let keyword = match def.target {
            TypeRef::OneOf(_) => "oneOf",
            _ => "anyOf",
        };
        let Some(Value::Array(members)) = self.components.get(def.name.as_str()).and_then(|source| source.get(keyword)) else {
            return Vec::new();
        };
        if members.len() != variants.len() {
            return Vec::new();
        }
        let rules: Vec<Map<String, Value>> = variants
            .iter()
            .zip(members)
            .map(|(variant, member)| match member.as_object() {
                Some(_) if admits_only_null(member) => Map::new(),
                member => self.rules(variant, member.cloned().unwrap_or_default(), true, 0),
            })
            .collect();
        if rules.iter().all(Map::is_empty) {
            return Vec::new();
        }
        rules
            .into_iter()
            .zip(members)
            .map(|(rules, member)| match member.as_object() {
                Some(null) if admits_only_null(member) => null.clone(),
                _ => rules,
            })
            .collect()
    }

    /// The rules of `endpoint`'s JSON request body whose type is
    /// `reference`, when it names a plain alias: the generated alias is not
    /// a component once assembled, so its rules belong on the endpoint.
    /// `None` otherwise or when the alias has no rules.
    ///
    /// [`Constraints`] holds the body's own keywords and its `items` rules
    /// only. A rule beneath anything else (`properties`,
    /// `additionalProperties`, a kept composition, a reference to a model
    /// with rules) is recorded as a problem for [`Self::finish`].
    pub(crate) fn body_constraints(&self, endpoint: &str, reference: &TypeRef) -> Option<Constraints> {
        let TypeRef::Named(name) = reference else { return None };
        if !self.aliases.contains_key(name.as_str()) {
            return None;
        }
        let rules = self.rules(reference, Map::new(), false, 0);
        // `own_rules` wrote these keywords, so every one reads.
        let constraints = Constraints::from_schema(&rules).expect("rule keywords are well formed");
        if let Some(pointer) = uncarried_rule(&rules, &constraints.to_schema(), "") {
            self.problem(format!(
                "endpoint `{endpoint}` has a JSON body typed as the plain alias `{name}`, whose rule at \
                 `#/components/schemas/{name}{pointer}` cannot be rebuilt: a plain alias is inline once assembled, \
                 so its rules move to the endpoint's request constraints, which hold only the body's own keywords \
                 and its `items` rules. Give the schema holding the rule a component name (a `$ref` to a named \
                 schema) so the rule stays on that model"
            ));
        }
        (!rules.is_empty()).then_some(constraints)
    }

    /// The rules of the source schema `source` for a value of `type_ref`'s
    /// Rust type, merged over the rules of every plain alias `type_ref`
    /// passes through (an outer rule wins).
    ///
    /// The value's own rules are kept whatever its type. Rules of its parts
    /// follow the Rust type's shape: an array's `items`, a map's
    /// `additionalProperties`, and for a free-form JSON value (also the type
    /// emitted for an inline composition) what [`Self::free_form_rules`]
    /// keeps. With `with_base`, an `items` or `additionalProperties` object
    /// also carries the schema Schemars would have written there, because
    /// `extend` replaces it.
    fn rules(&self, type_ref: &TypeRef, source: Map<String, Value>, with_base: bool, depth: usize) -> Map<String, Value> {
        match type_ref {
            TypeRef::Optional(inner) => self.rules(inner, source, with_base, depth),
            TypeRef::Named(name) => match self.aliases.get(name.as_str()) {
                Some(target) if depth < MAX_ALIAS_DEPTH => {
                    let mut merged = self.components.get(name.as_str()).map(|schema| (*schema).clone()).unwrap_or_default();
                    // The occurrence's `$ref` names this alias, whose schema
                    // is the one being merged; it is not an edge of the target.
                    let mut source = source;
                    if source.get("$ref").and_then(Value::as_str).and_then(component_name).as_deref() == Some(name.as_str()) {
                        source.remove("$ref");
                    }
                    merged.extend(source);
                    self.rules(target, merged, with_base, depth + 1)
                }
                Some(_) => {
                    if self.ruled.contains(name.as_str()) || !own_rules(&source).is_empty() {
                        self.problem(format!(
                            "the plain alias `{name}` is more than {MAX_ALIAS_DEPTH} aliases deep, so its rules are \
                             not rebuilt; give one alias in the chain a component name that is not a plain alias"
                        ));
                    }
                    Map::new()
                }
                // A nominal model's own rules live in its component; these
                // are the occurrence's.
                None => own_rules(&source),
            },
            TypeRef::Primitive(PrimitiveType::Bytes) => Map::new(),
            TypeRef::Primitive(PrimitiveType::Json) | TypeRef::Unknown | TypeRef::OneOf(_) | TypeRef::AnyOf(_) | TypeRef::AllOf(_) => {
                self.free_form_rules(&source)
            }
            TypeRef::Array(item) => {
                let mut rules = own_rules(&source);
                if let Some(child) = self.slot(item, source.get("items"), with_base, depth) {
                    rules.insert("items".to_owned(), Value::Object(child));
                }
                rules
            }
            TypeRef::Map(value) | TypeRef::KeyedMap { value, .. } => {
                let mut rules = own_rules(&source);
                if let Some(child) = self.slot(value, source.get("additionalProperties"), with_base, depth) {
                    rules.insert("additionalProperties".to_owned(), Value::Object(child));
                }
                rules
            }
            _ => own_rules(&source),
        }
    }

    /// The rules `source` (an `items` or `additionalProperties` schema)
    /// holds for a value of `type_ref`, over its base schema with
    /// `with_base`. `None` when there are no rules.
    fn slot(&self, type_ref: &TypeRef, source: Option<&Value>, with_base: bool, depth: usize) -> Option<Map<String, Value>> {
        let source = source.and_then(Value::as_object).cloned().unwrap_or_default();
        let mut child = self.rules(type_ref, source, with_base, depth);
        if child.is_empty() {
            return None;
        }
        if with_base {
            let mut base = self.base_schema(type_ref, depth)?;
            base.append(&mut child);
            child = base;
        }
        Some(child)
    }

    /// The rules of a free-form JSON value's source schema: its own, and
    /// those beneath it, without `type`. JSON Schema applies each rule only
    /// to a value of its kind, so a free-form value keeps its meaning (a
    /// `maxLength` checks a string value) without the schema asserting a
    /// type the Rust value does not.
    ///
    /// Beneath it are every `properties` member, `items` object, and
    /// `additionalProperties` object, and the edges that reach a rule:
    ///
    /// - A `$ref` to a model whose component holds a rule (see
    ///   [`Self::ruled`]) becomes a [`MODEL_REFERENCE`] to it, beside the
    ///   object's own rules and its `nullable: true`, which lets null skip
    ///   the reference. A plain alias is not registered once assembled, so a
    ///   reference to one also carries the alias' rules, under the
    ///   occurrence's own. A reference back to an alias already being
    ///   rebuilt is a problem for [`Self::finish`]: the rebuilt definition
    ///   has no component for it to point back to, so that occurrence would
    ///   lose the alias' rules. A reference that names no model is kept as
    ///   written when rules sit beside it, as the direct import keeps it.
    ///
    /// - Each `anyOf`, `oneOf`, `allOf`, and `not` with a rule beneath it
    ///   is kept, members in source order (see [`Self::composed_members`]).
    ///   The two the generator's plans pass through keep their source shape:
    ///   an `anyOf` of two members exactly one of which admits only null (an
    ///   `Option`'s nullable alternative), and a one-member `allOf` without a
    ///   `$ref` beside it, which the plans ignore for the `$ref`. Any other
    ///   also keeps each member's structure, so validation sees the
    ///   composition the direct import has.
    ///
    /// While an `additionalProperties` rule is kept, every declared property
    /// name is kept too, as `{}` when it has no rules: a name missing from
    /// `properties` would make its value an extra one that the extra-value rule
    /// then checks. Without such a rule, a property without rules is left out.
    fn free_form_rules(&self, schema: &Map<String, Value>) -> Map<String, Value> {
        let rebuilt = |value: &Value| value.as_object().map(|value| self.free_form_rules(value)).unwrap_or_default();
        let mut rules = own_rules(schema);
        let extra = match schema.get("additionalProperties") {
            Some(extra @ Value::Object(_)) => Some(rebuilt(extra)).filter(|extra| !extra.is_empty()),
            _ => None,
        };
        if let Some(Value::Object(properties)) = schema.get("properties") {
            let properties: Map<String, Value> = properties
                .iter()
                .filter_map(|(name, property)| {
                    let rules = rebuilt(property);
                    (extra.is_some() || !rules.is_empty()).then(|| (name.clone(), Value::Object(rules)))
                })
                .collect();
            if !properties.is_empty() {
                rules.insert("properties".to_owned(), Value::Object(properties));
            }
        }
        if let Some(items @ Value::Object(_)) = schema.get("items") {
            let items = rebuilt(items);
            if !items.is_empty() {
                rules.insert("items".to_owned(), Value::Object(items));
            }
        }
        if let Some(extra) = extra {
            rules.insert("additionalProperties".to_owned(), Value::Object(extra));
        }
        for keyword in ["anyOf", "oneOf", "allOf"] {
            let Some(Value::Array(members)) = schema.get(keyword) else { continue };
            let wrapper = match (keyword, members.as_slice()) {
                ("anyOf", [first, second]) => admits_only_null(first) != admits_only_null(second),
                ("allOf", [_]) => true,
                _ => false,
            };
            // The generator follows a `$ref` rather than a one-member `allOf`
            // beside it.
            if keyword == "allOf" && wrapper && schema.contains_key("$ref") {
                continue;
            }
            if let Some(members) = self.composed_members(members, wrapper) {
                rules.insert(keyword.to_owned(), Value::Array(members));
            }
        }
        if let Some(Value::Object(negated)) = schema.get("not") {
            let mut member = self.free_form_rules(negated);
            if !member.is_empty() {
                self.composition_structure(negated, &mut member);
                rules.insert("not".to_owned(), Value::Object(member));
            }
        }
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            self.free_form_reference(reference, &mut rules);
            if rules.contains_key(MODEL_REFERENCE) && schema.get("nullable") == Some(&Value::Bool(true)) {
                rules.insert("nullable".to_owned(), Value::Bool(true));
            }
        }
        rules
    }

    /// The rebuilt members of a composition whose source members are
    /// `members`, in source order and count, so a rule's pointer
    /// (`/anyOf/1/maxLength`) is the one the direct import has. A member
    /// that admits only null is copied as written; any other is rebuilt as
    /// a free-form value is. `None` when the composition is not kept.
    ///
    /// A `wrapper` (an `Option`'s nullable alternative or a one-member
    /// `allOf`, which generated checks pass through) is kept when its
    /// member has rules, in its source shape. Any other composition is kept
    /// when any member has rules (a reference to a model with rules
    /// counts), and each of its members also keeps its structure (see
    /// [`Self::composition_structure`]); generated checks do not enter it,
    /// so validation of the rebuilt definition refuses or discloses each
    /// rule beneath it as it does for the direct import.
    fn composed_members(&self, members: &[Value], wrapper: bool) -> Option<Vec<Value>> {
        let mut rebuilt: Vec<Value> = members
            .iter()
            .map(|member| match member.as_object() {
                Some(object) if !admits_only_null(member) => Value::Object(self.free_form_rules(object)),
                _ => member.clone(),
            })
            .collect();
        let mut ruled = members
            .iter()
            .zip(&rebuilt)
            .filter(|(source, _)| !admits_only_null(source))
            .map(|(_, member)| member.as_object().is_some_and(|member| !member.is_empty()));
        let keep = if wrapper { ruled.all(|ruled| ruled) } else { ruled.any(|ruled| ruled) };
        if !keep {
            return None;
        }
        if !wrapper {
            for (source, member) in members.iter().zip(&mut rebuilt) {
                if !admits_only_null(source)
                    && let (Some(source), Value::Object(member)) = (source.as_object(), member)
                {
                    self.composition_structure(source, member);
                }
            }
        }
        Some(rebuilt)
    }

    /// Rewrites each [`MODEL_REFERENCE`] in `value` for a container
    /// attribute, which has no schema generator to call: a nominal model
    /// becomes a `$ref` to its component, which registration resolves
    /// because every nominal model is registered, and a plain alias, which
    /// registers no component, becomes the schema Schemars writes for its
    /// target beneath the rules already rebuilt for it.
    fn literal_references(&self, value: &mut Value) {
        match value {
            Value::Object(object) => {
                if let Some(Value::String(name)) = object.remove(MODEL_REFERENCE) {
                    if self.aliases.contains_key(name.as_str()) {
                        for (keyword, base) in self.base_schema(&TypeRef::Named(name), 0).unwrap_or_default() {
                            object.entry(keyword).or_insert(base);
                        }
                    } else {
                        object.insert("$ref".to_owned(), Value::String(format!("#/components/schemas/{name}")));
                    }
                }
                object.values_mut().for_each(|value| self.literal_references(value));
            }
            Value::Array(values) => values.iter_mut().for_each(|value| self.literal_references(value)),
            _ => {}
        }
    }

    /// Adds to `member`, the rebuilt rules of one member of a kept generic
    /// composition, the structure that gives the member its meaning beside
    /// the rules: the source's `type` and `nullable`, and a reference to the
    /// model its `$ref` names even when that model holds no rule. Without
    /// them a member such as `{type: integer}` would rebuild to `{}`, which
    /// every value matches.
    fn composition_structure(&self, source: &Map<String, Value>, member: &mut Map<String, Value>) {
        for keyword in ["type", "nullable"] {
            if let Some(value) = source.get(keyword) {
                member.entry(keyword).or_insert_with(|| value.clone());
            }
        }
        if !member.contains_key(MODEL_REFERENCE)
            && let Some(name) = source.get("$ref").and_then(Value::as_str).and_then(component_name)
            && let Some((name, _)) = self.components.get_key_value(name.as_str())
        {
            member.insert(MODEL_REFERENCE.to_owned(), Value::String((*name).to_owned()));
        }
    }

    /// Adds to `rules`, the rebuilt rules of a free-form schema object, the
    /// edge its `$ref` `reference` needs (see [`Self::free_form_rules`]).
    fn free_form_reference(&self, reference: &str, rules: &mut Map<String, Value>) {
        let Some(name) = component_name(reference).and_then(|name| self.components.get_key_value(name.as_str()).map(|(name, _)| *name)) else {
            if !rules.is_empty() {
                rules.insert("$ref".to_owned(), Value::String(reference.to_owned()));
            }
            return;
        };
        if !self.ruled.contains(name) {
            return;
        }
        if self.aliases.contains_key(name) {
            let rebuilding = {
                let expanding = self.expanding.borrow();
                expanding.contains(&name).then(|| expanding.last().copied()).flatten()
            };
            if let Some(within) = rebuilding {
                self.problem(format!(
                    "the plain alias `{name}` holds value rules and is reached again, through `$ref: {reference}` in a \
                     free-form value while the rules of `{within}` are rebuilt; a plain alias is emitted as `pub type` \
                     and registers no component, so that inner occurrence would lose `{name}`'s rules. Give the \
                     alias' item or object schema a component name (a `$ref` to a named schema) so the recursion \
                     passes through a model"
                ));
            } else {
                self.expanding.borrow_mut().push(name);
                let alias_rules = self.rules(&TypeRef::Named(name.to_owned()), Map::new(), true, 0);
                self.expanding.borrow_mut().pop();
                for (keyword, value) in alias_rules {
                    rules.entry(keyword).or_insert(value);
                }
            }
        }
        rules.insert(MODEL_REFERENCE.to_owned(), Value::String(name.to_owned()));
    }

    /// The schema Schemars derives (in OpenAPI 3.0 form) for `type_ref`'s
    /// Rust type when it sits in an `items` or `additionalProperties` slot.
    /// `None` for the types whose slot schema is not restated: bytes and an
    /// `Option`, neither of which the importer places there.
    ///
    /// A model is written as Schemars writes a reference to it,
    /// `#/components/schemas/{name}`; registration resolves that pointer,
    /// since the Rust type itself reaches the model.
    fn base_schema(&self, type_ref: &TypeRef, depth: usize) -> Option<Map<String, Value>> {
        let schema = match type_ref {
            TypeRef::Primitive(PrimitiveType::String) => serde_json::json!({"type": "string"}),
            TypeRef::Primitive(PrimitiveType::Integer) => serde_json::json!({"type": "integer", "format": "int64"}),
            TypeRef::Primitive(PrimitiveType::Number) => serde_json::json!({"type": "number", "format": "double"}),
            TypeRef::Primitive(PrimitiveType::Boolean) => serde_json::json!({"type": "boolean"}),
            TypeRef::Primitive(PrimitiveType::Json) | TypeRef::Unknown | TypeRef::OneOf(_) | TypeRef::AnyOf(_) | TypeRef::AllOf(_) => {
                serde_json::json!({})
            }
            TypeRef::Array(item) => serde_json::json!({"type": "array", "items": self.base_schema(item, depth)?}),
            TypeRef::Map(value) | TypeRef::KeyedMap { value, .. } => {
                serde_json::json!({"type": "object", "additionalProperties": self.base_schema(value, depth)?})
            }
            TypeRef::Named(name) => match self.aliases.get(name.as_str()) {
                Some(target) if depth < MAX_ALIAS_DEPTH => return self.base_schema(target, depth + 1),
                Some(_) => return None,
                None => serde_json::json!({"$ref": format!("#/components/schemas/{name}")}),
            },
            _ => return None,
        };
        match schema {
            Value::Object(schema) => Some(schema),
            _ => None,
        }
    }
}

/// The JSON pointer, below `pointer`, to the first rule of `rules` that
/// `carried` does not hold; `None` when it holds them all. A
/// [`MODEL_REFERENCE`] is shown as the `$ref` it stands for.
fn uncarried_rule(rules: &Map<String, Value>, carried: &Map<String, Value>, pointer: &str) -> Option<String> {
    rules.iter().find_map(|(keyword, value)| {
        let kept = carried.get(keyword);
        if kept == Some(value) {
            return None;
        }
        let token = if keyword == MODEL_REFERENCE { "$ref".to_owned() } else { keyword.replace('~', "~0").replace('/', "~1") };
        let at = format!("{pointer}/{token}");
        match value {
            Value::Object(inner) => {
                let empty = Map::new();
                uncarried_rule(inner, kept.and_then(Value::as_object).unwrap_or(&empty), &at)
            }
            _ => Some(at),
        }
    })
}

/// The component named by the local reference `reference`
/// (`#/components/schemas/{name}`, which import points at final model
/// names); `None` for any other reference.
fn component_name(reference: &str) -> Option<String> {
    let token = reference.strip_prefix("#/components/schemas/")?;
    (!token.is_empty() && !token.contains('/')).then(|| token.replace("~1", "/").replace("~0", "~"))
}

/// The components of `components` that hold a validation rule anywhere in
/// their schema or reach one through component references, resolved to a
/// fixed point so recursive references terminate.
fn ruled_components<'a>(components: &HashMap<&'a str, &'a Map<String, Value>>) -> HashSet<&'a str> {
    /// Whether `object` holds a rule, collecting the components it references.
    fn walk(object: &Map<String, Value>, references: &mut Vec<String>) -> bool {
        if let Some(name) = object.get("$ref").and_then(Value::as_str).and_then(component_name) {
            references.push(name);
        }
        let mut rules = Constraints::from_schema(object).unwrap_or_default();
        rules.format = None;
        rules.items = None;
        let mut found = rules != Constraints::default();
        // Every value is walked, so every reference is collected.
        for value in object.values() {
            found |= within(value, references);
        }
        found
    }
    fn within(value: &Value, references: &mut Vec<String>) -> bool {
        match value {
            Value::Object(object) => walk(object, references),
            Value::Array(values) => {
                let mut found = false;
                for value in values {
                    found |= within(value, references);
                }
                found
            }
            _ => false,
        }
    }
    let mut ruled = HashSet::new();
    let mut edges: Vec<(&'a str, Vec<String>)> = Vec::new();
    for (name, schema) in components {
        let mut references = Vec::new();
        if walk(schema, &mut references) {
            ruled.insert(*name);
        }
        edges.push((name, references));
    }
    loop {
        let before = ruled.len();
        for (name, references) in &edges {
            if !ruled.contains(name) && references.iter().any(|target| ruled.contains(target.as_str())) {
                ruled.insert(*name);
            }
        }
        if ruled.len() == before {
            return ruled;
        }
    }
}

/// Whether `alias` is emitted as a real enum or struct (a non-empty
/// combinator), and so registered under its own name.
fn is_nominal_alias(alias: &TypeAlias) -> bool {
    matches!(&alias.target, TypeRef::OneOf(v) | TypeRef::AnyOf(v) | TypeRef::AllOf(v) if !v.is_empty())
}

/// The `#[schemars(extend(..))]` attribute writing `rules`; `None` when
/// there are none.
fn extend_attribute(rules: &Map<String, Value>) -> Option<TokenStream> {
    if rules.is_empty() {
        return None;
    }
    let entries = rules.iter().map(|(keyword, value)| {
        let value = json_tokens(value);
        quote! { #keyword = #value }
    });
    Some(quote! { #[schemars(extend(#(#entries),*))] })
}

/// The value rules written on `schema` itself as JSON Schema keywords
/// ([`Constraints::to_schema`]): numeric exclusive bounds, exact numbers as
/// JSON numbers or stand-ins with their `x-schematic-exact` text, and
/// `format`. Item rules are not included; the caller places them.
fn own_rules(schema: &Map<String, Value>) -> Map<String, Value> {
    // Import refuses a component whose rules do not read
    // (`schema_rule_problems`), so none reaches the rebuild.
    let mut rules = Constraints::from_schema(schema).unwrap_or_default();
    rules.items = None;
    rules.to_schema()
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
///
/// `rules` is the container attribute rebuilding the source root's rules.
fn generate_enum_with(def: &EnumDef, derive_json_schema: bool, rules: Option<TokenStream>) -> TokenStream {
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
        #rules
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
/// flattens its members. `rules`, when given, rebuilds the source root's
/// rules on those (see [`RuleSource::alias_attribute`]) and each union
/// variant's (see [`RuleSource::variant_rules`]); a plain alias carries its
/// rules where it is used.
fn generate_alias_with(def: &TypeAlias, derive_json_schema: bool, rules: Option<&RuleSource>) -> TokenStream {
    let container = rules.and_then(|rules| rules.alias_attribute(def));
    match &def.target {
        TypeRef::OneOf(variants) | TypeRef::AnyOf(variants) if !variants.is_empty() => {
            let variant_rules = rules.map(|rules| rules.variant_rules(def, variants)).unwrap_or_default();
            generate_union_enum(def, variants, derive_json_schema, container, &variant_rules)
        }
        TypeRef::AllOf(members) if !members.is_empty() => {
            generate_composed_struct(def, members, derive_json_schema, container)
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
///
/// `variant_rules`, when not empty, holds each variant's rebuilt rules, in
/// variant order, written on the variant as [`write_rules`] writes a field's.
fn generate_union_enum(
    def: &TypeAlias,
    variants: &[TypeRef],
    derive_json_schema: bool,
    rules: Option<TokenStream>,
    variant_rules: &[Map<String, Value>],
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
    let mut helpers = Vec::new();
    let variant_attributes: Vec<Option<TokenStream>> = variant_types
        .iter()
        .enumerate()
        .map(|(index, rust_type)| {
            let rules = variant_rules.get(index)?;
            let function = format_ident!("variant_rules_schema_{}", index);
            write_rules(&name, &function, rust_type.clone(), false, rules, &mut helpers)
        })
        .collect();
    let helpers = (!helpers.is_empty()).then(|| quote! { impl #name { #(#helpers)* } });

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
        #rules
        #[serde(untagged)]
        pub enum #name {
            #(#variant_attributes #variant_idents(#variant_types),)*
        }

        #helpers

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
    rules: Option<TokenStream>,
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
        #rules
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
        let code = format_code(&validate_code(&assemble_model_tokens_with(catalog, Some(components)).expect("every rule rebuilds")).expect("emitted models parse"));
        let start = code.find(&format!("pub struct {model} ")).expect("model is emitted");
        let end = code[start..].find("\n}").map_or(code.len(), |end| start + end);
        code[start..end].split_whitespace().collect()
    }

    #[test]
    fn definitions_out_fields_rebuild_every_source_rule_through_extend() {
        let (catalog, components) = constrained_catalog();

        let widget = compact_model_source(&catalog, &components, "Widget");

        for expected in [
            r#"#[schemars(extend("format"="slug","maxLength"=64u64,"minLength"=1u64,"pattern"="^[a-z]+$"))]pubname:String"#,
            r#"#[schemars(extend("format"="int32","maximum"=100u64,"minimum"=-5i64,"multipleOf"=5u64))]pubcount:i64"#,
            r#"#[schemars(extend("exclusiveMaximum"=1.5f64,"exclusiveMinimum"=0u64))]pubratio:f64"#,
            r#"#[schemars(extend("items"={"maxLength":5u64,"type":"string"},"maxItems"=10u64,"minItems"=1u64,"uniqueItems"=true))]pubtags:Vec<String>"#,
            r#"#[serde(rename="displayName")]#[schemars(extend("minLength"=2u64))]pubdisplay_name:String"#,
            r#"#[schemars(extend("enum"=["fast","slow"]))]pubmode:Option<String>"#,
            r#"#[schemars(extend("const"="v1"))]pubkind:Option<String>"#,
            r#"#[schemars(extend("maxLength"=8u64,"pattern"="^[a-z-]+$"))]pubslug:Slug"#,
            r#"#[schemars(extend("items"={"minLength":3u64,"type":"string"}))]pubcodes:Vec<Code>"#,
            r#"#[schemars(extend("maximum"=18446744073709552000f64,"x-schematic-exact"={"maximum":"18446744073709551616"}))]pubbig:i64"#,
            "pubowner:Plain,pubplain:String,",
        ] {
            assert!(widget.contains(expected), "missing {expected}\n{widget}");
        }
    }

    /// One field or model per position whose Rust type is not a primitive.
    fn shaped_catalog() -> (ModelCatalog, Vec<SchemaComponent>) {
        let json = || primitive(PrimitiveType::Json);
        let named = |name: &str| TypeRef::Named(name.into());
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![
                ModelDef::Struct(StructDef {
                    name: "Holder".into(),
                    description: None,
                    fields: vec![
                        field("free", None, true, json()),
                        field("attributes", None, true, json()),
                        field("labels", None, true, TypeRef::Map(Box::new(primitive(PrimitiveType::String)))),
                        field("owners_by_id", None, true, TypeRef::Map(Box::new(named("Owner")))),
                        field("palette", None, true, TypeRef::Map(Box::new(primitive(PrimitiveType::String)))),
                        field("owner", None, true, named("Owner")),
                        field("backup", None, false, named("Owner")),
                        field("owners", None, true, TypeRef::Array(Box::new(named("Owner")))),
                        field("entries", None, true, TypeRef::Array(Box::new(json()))),
                        field("plain_owner", None, true, named("Owner")),
                    ],
                    additional_properties: Some(primitive(PrimitiveType::String)),
                }),
                ModelDef::Struct(StructDef {
                    name: "Owner".into(),
                    description: None,
                    fields: vec![field("name", None, false, primitive(PrimitiveType::String))],
                    additional_properties: None,
                }),
                ModelDef::Enum(EnumDef {
                    name: "Color".into(),
                    description: None,
                    variants: vec![EnumVariant { name: "Red".into(), value: Some("red".into()), description: None }],
                    untagged: false,
                }),
            ],
        };
        let component = |name: &str, schema: Value| SchemaComponent { name: name.into(), rust_path: Some(name.into()), schema };
        let components = vec![
            component("Holder", serde_json::json!({"type": "object", "const": {"free": 1}, "properties": {
                "free": {"maxLength": 3, "format": "slug"},
                "attributes": {"type": "object", "properties": {"code": {"type": "string", "minLength": 2}, "bare": {"type": "string"}},
                    "additionalProperties": {"type": "string", "maxLength": 3}},
                "labels": {"type": "object", "additionalProperties": {"type": "string", "format": "uuid", "maxLength": 36}},
                "owners_by_id": {"type": "object", "additionalProperties": {"$ref": "#/components/schemas/Owner", "const": {"name": "a"}}},
                "palette": {"type": "object", "additionalProperties": {"type": "string"}, "enum": [{"primary": "red"}]},
                "owner": {"$ref": "#/components/schemas/Owner", "const": {"name": "boss"}},
                "backup": {"$ref": "#/components/schemas/Owner", "enum": [{"name": "boss"}]},
                "owners": {"type": "array", "items": {"$ref": "#/components/schemas/Owner", "const": {"name": "boss"}}},
                "entries": {"type": "array", "items": {"type": "object", "properties": {"code": {"type": "string"}}, "const": {"code": "x"}}},
                "plain_owner": {"$ref": "#/components/schemas/Owner"}
            }, "additionalProperties": {"type": "string", "maxLength": 3}})),
            component("Owner", serde_json::json!({"type": "object", "properties": {"name": {"type": "string", "maxLength": 10}}})),
            component("Color", serde_json::json!({"type": "string", "enum": ["red"], "maxLength": 5})),
        ];
        (catalog, components)
    }

    #[test]
    fn definitions_out_rebuilds_rules_wherever_the_rust_type_places_them() {
        let (catalog, components) = shaped_catalog();

        let holder = compact_model_source(&catalog, &components, "Holder");
        let code = format_code(&validate_code(&assemble_model_tokens_with(&catalog, Some(&components)).unwrap()).unwrap());
        let code: String = code.split_whitespace().collect();

        for expected in [
            // The model root and its flattened extra-property values.
            r#"#[schemars(extend("additionalProperties"={"maxLength":3u64,"type":"string"},"const"={"free":1u64}))]pubstructHolder"#,
            // A free-form value keeps every nested rule, untyped.
            r#"#[schemars(extend("format"="slug","maxLength"=3u64))]pubfree:serde_json::Value"#,
            r#"#[schemars(extend("additionalProperties"={"maxLength":3u64},"properties"={"bare":{},"code":{"minLength":2u64}}))]pubattributes:serde_json::Value"#,
            // A map's values, over the schema Schemars writes for them.
            r#"#[schemars(extend("additionalProperties"={"format":"uuid","maxLength":36u64,"type":"string"}))]publabels:HashMap<String,String>"#,
            r##"#[schemars(extend("additionalProperties"={"$ref":"#/components/schemas/Owner","const":{"name":"a"}}))]pubowners_by_id:HashMap<String,Owner>"##,
            // A map's and a model reference's whole value.
            r#"#[schemars(extend("enum"=[{"primary":"red"}]))]pubpalette:HashMap<String,String>"#,
            r#"#[schemars(extend("const"={"name":"boss"}))]pubowner:Owner"#,
            r#"#[schemars(extend("enum"=[{"name":"boss"}]))]pubbackup:Option<Owner>"#,
            // Items that are models or free-form values.
            r##"#[schemars(extend("items"={"$ref":"#/components/schemas/Owner","const":{"name":"boss"}}))]pubowners:Vec<Owner>"##,
            r#"#[schemars(extend("items"={"const":{"code":"x"}}))]pubentries:Vec<serde_json::Value>"#,
            "pubplain_owner:Owner,",
        ] {
            assert!(holder.contains(expected) || code.contains(expected), "missing {expected}\n{code}");
        }
        // A unit enum's `enum` is its variants; its other root rules stay.
        assert!(code.contains(r#"#[schemars(extend("maxLength"=5u64))]pubenumColor"#), "{code}");
        // The shared component carries no occurrence's rule.
        assert!(!compact_model_source(&catalog, &components, "Owner").contains("const"));
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
    fn rebuilt_enum_and_const_keep_the_source_null_membership() {
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![ModelDef::Struct(StructDef {
                name: "Choice".into(),
                description: None,
                fields: vec![
                    field("excluded", None, true, TypeRef::Optional(Box::new(primitive(PrimitiveType::String)))),
                    field("allowed", None, true, TypeRef::Optional(Box::new(primitive(PrimitiveType::String)))),
                    field("legacy", None, true, TypeRef::Optional(Box::new(primitive(PrimitiveType::String)))),
                    field("fixed", None, true, TypeRef::Optional(Box::new(primitive(PrimitiveType::String)))),
                    field("absent", None, false, primitive(PrimitiveType::String)),
                ],
                additional_properties: None,
            })],
        };
        let components = vec![SchemaComponent {
            name: "Choice".into(),
            rust_path: Some("Choice".into()),
            schema: serde_json::json!({"type": "object", "properties": {
                "excluded": {"type": ["string", "null"], "enum": ["a"]},
                "allowed": {"type": ["string", "null"], "enum": ["a", null]},
                // OpenAPI 3.0.3: `nullable` does not add null to an `enum`.
                "legacy": {"type": "string", "nullable": true, "enum": ["a"]},
                "fixed": {"type": ["string", "null"], "const": "a"},
                "absent": {"type": "string", "enum": ["a"]}
            }}),
        }];

        let choice = compact_model_source(&catalog, &components, "Choice");

        for expected in [
            r#"#[schemars(extend("enum"=["a"]))]pubexcluded:Option<String>"#,
            r#"#[schemars(extend("enum"=["a",null]))]puballowed:Option<String>"#,
            r#"#[schemars(extend("enum"=["a"]))]publegacy:Option<String>"#,
            r#"#[schemars(extend("const"="a"))]pubfixed:Option<String>"#,
            r#"#[serde(skip_serializing_if="Option::is_none")]#[schemars(extend("enum"=["a"]))]pubabsent:Option<String>"#,
        ] {
            assert!(choice.contains(expected), "missing {expected}\n{choice}");
        }
    }

    /// An object whose extra values are checked beside a declared property
    /// without rules (`label`) and one with a rule (`code`).
    fn mixed_object() -> Value {
        serde_json::json!({
            "properties": {"label": {"type": "string"}, "code": {"type": "string", "maxLength": 2}},
            "additionalProperties": {"type": "string", "maxLength": 3}
        })
    }

    /// The rebuilt rules of [`mixed_object`]: `label` stays a declared name.
    const MIXED_RULES: &str = r#"{"additionalProperties":{"maxLength":3u64},"properties":{"code":{"maxLength":2u64},"label":{}}}"#;

    #[test]
    fn a_kept_extra_value_rule_keeps_every_declared_name_at_each_free_form_position() {
        let json = || primitive(PrimitiveType::Json);
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![
                ModelDef::Struct(StructDef {
                    name: "Item".into(),
                    description: None,
                    fields: vec![
                        field("free_value", None, true, json()),
                        field("free_map", None, true, json()),
                        field("attributes", None, true, json()),
                        field("entries", None, true, TypeRef::Array(Box::new(json()))),
                    ],
                    additional_properties: None,
                }),
                ModelDef::Struct(StructDef {
                    name: "NestedExtras".into(),
                    description: None,
                    fields: vec![field("id", None, true, primitive(PrimitiveType::String))],
                    additional_properties: Some(json()),
                }),
            ],
        };
        let mut free_value = mixed_object();
        free_value["properties"]["inner"] = mixed_object();
        free_value["items"] = mixed_object();
        let component = |name: &str, schema: Value| SchemaComponent { name: name.into(), rust_path: Some(name.into()), schema };
        let components = vec![
            component("Item", serde_json::json!({"type": "object", "properties": {
                "free_value": free_value,
                "free_map": {"additionalProperties": mixed_object()},
                "attributes": {"type": "object", "additionalProperties": mixed_object()},
                "entries": {"type": "array", "items": mixed_object()}
            }})),
            component("NestedExtras", serde_json::json!({"type": "object", "properties": {"id": {"type": "string"}},
                "additionalProperties": mixed_object()})),
        ];

        let item = compact_model_source(&catalog, &components, "Item");
        let code: String = format_code(&validate_code(&assemble_model_tokens_with(&catalog, Some(&components)).unwrap()).unwrap())
            .split_whitespace()
            .collect();

        for (position, expected) in [
            // R2-F1.1–R2-F1.3: the free-form root, a nested property, items.
            (
                "free-form root, property, and items",
                format!(r#"#[schemars(extend("additionalProperties"={{"maxLength":3u64}},"items"={MIXED_RULES},"properties"={{"code":{{"maxLength":2u64}},"inner":{MIXED_RULES},"label":{{}}}}))]pubfree_value:serde_json::Value"#),
            ),
            // R2-F1.4: a free-form value's extra values.
            ("free-form extra values", format!(r#"#[schemars(extend("additionalProperties"={MIXED_RULES}))]pubfree_map:serde_json::Value"#)),
            // R2-F1.5: a map-valued field, imported as a free-form value.
            ("map-valued field", format!(r#"#[schemars(extend("additionalProperties"={MIXED_RULES}))]pubattributes:serde_json::Value"#)),
            // R2-F1.6: inline object items, over the base `{}`.
            ("inline object items", format!(r#"#[schemars(extend("items"={MIXED_RULES}))]pubentries:Vec<serde_json::Value>"#)),
        ] {
            assert!(item.contains(&expected), "{position}: missing {expected}\n{item}");
        }
        // R2-F1.7 and R2-F1.9: a struct's flattened extra value keeps its
        // declared names; the struct's own `id` is a Rust field Schemars
        // declares, so nothing restates it.
        let nested = format!(r#"#[schemars(extend("additionalProperties"={MIXED_RULES}))]pubstructNestedExtras"#);
        assert!(code.contains(&nested), "missing {nested}\n{code}");
    }

    /// A catalog whose free-form values reference models at every position:
    /// `Owner` holds a rule, `Plain` holds none, `Tag` is a plain alias with
    /// a rule, and `Holder` a plain alias of a free-form value that
    /// references `Owner`.
    fn referencing_catalog() -> (ModelCatalog, Vec<SchemaComponent>) {
        let json = || primitive(PrimitiveType::Json);
        let named = |name: &str| TypeRef::Named(name.into());
        let owner = || serde_json::json!({"$ref": "#/components/schemas/Owner"});
        let holding_owner = || serde_json::json!({"properties": {"owner": owner()}});
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![
                ModelDef::Struct(StructDef {
                    name: "Item".into(),
                    description: None,
                    fields: vec![
                        field("free_value", None, true, json()),
                        field("optional_value", None, false, json()),
                        field("nullable_value", None, true, TypeRef::Optional(Box::new(json()))),
                        field("attributes", None, true, TypeRef::Map(Box::new(json()))),
                        field("entries", None, true, TypeRef::Array(Box::new(json()))),
                        field("unruled", None, true, json()),
                        field("owner", None, true, named("Owner")),
                        field("holder", None, false, named("Holder")),
                        field("referenced", None, true, json()),
                    ],
                    additional_properties: None,
                }),
                ModelDef::Struct(StructDef {
                    name: "Extras".into(),
                    description: None,
                    fields: vec![field("id", None, true, primitive(PrimitiveType::String))],
                    additional_properties: Some(json()),
                }),
                ModelDef::Struct(StructDef {
                    name: "Owner".into(),
                    description: None,
                    fields: vec![field("name", None, false, primitive(PrimitiveType::String))],
                    additional_properties: None,
                }),
                ModelDef::Struct(StructDef { name: "Plain".into(), description: None, fields: vec![field("note", None, false, primitive(PrimitiveType::String))], additional_properties: None }),
                ModelDef::Alias(TypeAlias { name: "Tag".into(), description: None, target: primitive(PrimitiveType::String), discriminator: None }),
                ModelDef::Alias(TypeAlias { name: "Holder".into(), description: None, target: json(), discriminator: None }),
            ],
        };
        let component = |name: &str, schema: Value| SchemaComponent { name: name.into(), rust_path: Some(name.into()), schema };
        let components = vec![
            component("Item", serde_json::json!({"type": "object", "properties": {
                "free_value": {"properties": {
                    "owner": owner(),
                    "inner": holding_owner(),
                    "plain": {"$ref": "#/components/schemas/Plain"},
                    "pinned": {"$ref": "#/components/schemas/Owner", "const": {"name": "boss"}, "nullable": true},
                    "tag": {"$ref": "#/components/schemas/Tag", "minLength": 1},
                    "missing": {"$ref": "#/components/schemas/Missing", "maxLength": 2},
                    "bare_missing": {"$ref": "#/components/schemas/Missing"}
                }, "items": owner()},
                "optional_value": {"additionalProperties": owner()},
                "nullable_value": holding_owner(),
                "attributes": {"type": "object", "additionalProperties": holding_owner()},
                "entries": {"type": "array", "items": holding_owner()},
                "unruled": {"properties": {"plain": {"$ref": "#/components/schemas/Plain"}}},
                "owner": {"$ref": "#/components/schemas/Owner", "const": {"name": "boss"}},
                "holder": {"$ref": "#/components/schemas/Holder"},
                "referenced": {"$ref": "#/components/schemas/Owner", "description": "typed as free-form"}
            }})),
            component("Extras", serde_json::json!({"type": "object", "properties": {"id": {"type": "string"}}, "additionalProperties": holding_owner()})),
            component("Owner", serde_json::json!({"type": "object", "properties": {"name": {"type": "string", "maxLength": 10}}})),
            component("Plain", serde_json::json!({"type": "object", "properties": {"note": {"type": "string"}}})),
            component("Tag", serde_json::json!({"type": "string", "maxLength": 8})),
            component("Holder", holding_owner()),
        ];
        (catalog, components)
    }

    #[test]
    fn a_free_form_reference_to_a_model_with_rules_is_written_through_the_generator_at_each_position() {
        let (catalog, components) = referencing_catalog();

        let code: String = format_code(&validate_code(&assemble_model_tokens_with(&catalog, Some(&components)).unwrap()).unwrap())
            .split_whitespace()
            .collect::<String>()
            .replace(",)", ")");

        for (position, expected) in [
            // R2-F2.1–R2-F2.3: a free-form root's property, a nested
            // property, items; a `$ref` with a sibling rule and `nullable`.
            ("free-form field", r#"#[schemars(schema_with="Item::field_rules_schema_free_value")]pubfree_value:serde_json::Value,"#),
            (
                "free-form helper",
                concat!(
                    r#"fnfield_rules_schema_free_value(generator:&mutschemars::SchemaGenerator)->schemars::Schema{"#,
                    r#"letreference_0=generator.subschema_for::<Owner>();letreference_1=generator.subschema_for::<Owner>();letreference_2=generator.subschema_for::<Owner>();"#,
                    r#"letmutreference_3=generator.subschema_for::<Owner>();reference_3.insert("const".to_owned(),serde_json::json!({"name":"boss"}));reference_3.insert("nullable".to_owned(),serde_json::json!(true));"#,
                    // A plain alias carries its own rules under the occurrence's.
                    r#"letmutreference_4=generator.subschema_for::<Tag>();reference_4.insert("maxLength".to_owned(),serde_json::json!(8u64));reference_4.insert("minLength".to_owned(),serde_json::json!(1u64));"#,
                    r#"letmutschema=generator.subschema_for::<serde_json::Value>();schema.insert("items".to_owned(),serde_json::json!(reference_0));"#,
                    r##"schema.insert("properties".to_owned(),serde_json::json!({"inner":{"properties":{"owner":reference_1}},"missing":{"$ref":"#/components/schemas/Missing","maxLength":2u64},"owner":reference_2,"pinned":reference_3,"tag":reference_4}));schema}"##,
                ),
            ),
            // R2-F2.4: a free-form value's extra values, on an optional field
            // that `schema_with` would otherwise make required.
            (
                "optional free-form extra values",
                r#"#[serde(skip_serializing_if="Option::is_none")]#[serde(default)]#[schemars(schema_with="Item::field_rules_schema_optional_value")]puboptional_value:Option<serde_json::Value>,"#,
            ),
            ("optional helper", r#"letmutschema=generator.subschema_for::<Option<serde_json::Value>>();schema.insert("additionalProperties".to_owned(),serde_json::json!(reference_0));"#),
            ("nullable required field", r#"#[serde(default)]#[schemars(schema_with="Item::field_rules_schema_nullable_value")]pubnullable_value:Option<serde_json::Value>,"#),
            // R2-F2.5–R2-F2.6: a map-valued field and inline object items.
            ("map-valued field", r#"#[schemars(schema_with="Item::field_rules_schema_attributes")]pubattributes:HashMap<String,serde_json::Value>,"#),
            (
                "map-valued helper",
                r#"letmutschema=generator.subschema_for::<HashMap<String,serde_json::Value>>();schema.insert("additionalProperties".to_owned(),serde_json::json!({"properties":{"owner":reference_0}}));"#,
            ),
            ("inline object items", r#"#[schemars(schema_with="Item::field_rules_schema_entries")]pubentries:Vec<serde_json::Value>,"#),
            // R2-F2.7: a struct's flattened extra values, whose container
            // keeps no `additionalProperties` of its own.
            (
                "flattened extra values",
                r#"#[derive(JsonSchema)]pubstructExtras{pubid:String,///Additionalpropertiesnotcapturedbynamedfields.#[serde(flatten)]#[schemars(schema_with="Extras::extra_values_rules_schema")]pubadditional_properties:HashMap<String,serde_json::Value>,}"#,
            ),
            // A reference to a model without rules adds nothing.
            ("reference without rules", "pubunruled:serde_json::Value,"),
            // R2-F2.8: a nominal reference keeps its occurrence rule in `extend`.
            ("nominal reference", r#"#[schemars(extend("const"={"name":"boss"}))]pubowner:Owner,"#),
            // A plain alias of a free-form value carries its rules, and the
            // alias' own `$ref` is not an edge.
            ("aliased free-form value", r#"#[serde(default)]#[schemars(schema_with="Item::field_rules_schema_holder")]pubholder:Option<Holder>,"#),
            (
                "aliased helper",
                r#"letreference_0=generator.subschema_for::<Owner>();letmutschema=generator.subschema_for::<Option<Holder>>();schema.insert("properties".to_owned(),serde_json::json!({"owner":reference_0}));schema}"#,
            ),
            // A free-form value that is itself a reference.
            (
                "referenced value",
                r#"letmutschema=generator.subschema_for::<serde_json::Value>();ifletSome(reference)=generator.subschema_for::<Owner>().as_object(){for(keyword,value)inreference{schema.insert(keyword.clone(),value.clone());}}schema}"#,
            ),
        ] {
            assert!(code.contains(expected), "{position}: missing {expected}\n{code}");
        }
        assert!(!code.contains("Plain>"), "a model without rules is not referenced: {code}");
        assert_eq!(code.matches("Missing").count(), 1, "an unresolved reference stays only beside its rule: {code}");
        assert!(!code.contains(MODEL_REFERENCE), "{code}");
    }

    /// `rules` as Schemars writes them once compiled: each
    /// [`MODEL_REFERENCE`] is the generator's `$ref` to the model, beside the
    /// object's other keywords.
    fn as_compiled(value: &Value) -> Value {
        match value {
            Value::Object(entries) => Value::Object(
                entries
                    .iter()
                    .map(|(keyword, value)| match (keyword.as_str(), value) {
                        (MODEL_REFERENCE, Value::String(model)) => ("$ref".to_owned(), Value::String(format!("#/components/schemas/{model}"))),
                        _ => (keyword.clone(), as_compiled(value)),
                    })
                    .collect(),
            ),
            Value::Array(values) => Value::Array(values.iter().map(as_compiled).collect()),
            scalar => scalar.clone(),
        }
    }

    #[test]
    fn a_rebuilt_free_form_reference_gives_the_direct_import_s_findings_and_plans() {
        use crate::codegen::value_rules::api_findings;
        use crate::output::assemble::assemble_api_module;
        use schematic_define::{ApiRequest, DefinitionContext, Endpoint, RestApi};

        let owner = || serde_json::json!({"$ref": "#/components/schemas/Owner"});
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![
                ModelDef::Struct(StructDef { name: "Item".into(), description: None, fields: vec![field("value", None, true, primitive(PrimitiveType::Json))], additional_properties: None }),
                ModelDef::Struct(StructDef { name: "Owner".into(), description: None, fields: vec![field("name", None, false, primitive(PrimitiveType::String))], additional_properties: None }),
            ],
        };
        let api = |value: Value| {
            let mut api = RestApi::builder(&DefinitionContext::types_in("::fixture"), "Rebuilt", "https://example.test").describes("Rebuilt").no_auth().build().unwrap();
            let mut create = Endpoint::post("CreateItem", "/items").describes("Create").returns_empty().build().unwrap();
            create.request = Some(ApiRequest::json_type("Item"));
            api.endpoints = vec![create];
            let component = |name: &str, schema: Value| SchemaComponent { name: name.into(), rust_path: Some(name.into()), schema };
            api.schemas = vec![
                component("Item", serde_json::json!({"type": "object", "required": ["value"], "properties": {"value": value}})),
                component("Owner", serde_json::json!({"type": "object", "properties": {"name": {"type": "string", "maxLength": 10}}})),
            ];
            api
        };
        let findings = |api: &RestApi| -> Vec<String> { api_findings(api).iter().map(|(_, finding)| format!("{:?} {}", finding.severity, finding.message())).collect() };
        let plans = |api: &RestApi| -> String {
            let code: String = format_code(&validate_code(&assemble_api_module(api)).unwrap()).split_whitespace().collect();
            code.find("modrebuilt_value_rules{").map(|start| code[start..].to_owned()).unwrap_or_default()
        };

        for (case, value) in [
            ("reference", serde_json::json!({"properties": {"owner": owner()}})),
            ("nullable reference", serde_json::json!({"properties": {"owner": {"$ref": "#/components/schemas/Owner", "nullable": true}}})),
            ("reference beside a rule", serde_json::json!({"items": {"properties": {"owner": {"$ref": "#/components/schemas/Owner", "const": {"name": "boss"}}}}})),
            ("nullable alternative", serde_json::json!({"properties": {"owner": {"anyOf": [owner(), null_member()]}}})),
            ("nullable alternative as the value", serde_json::json!({"anyOf": [null_member(), {"properties": {"owner": owner()}}]})),
            ("one-member allOf", serde_json::json!({"items": {"properties": {"owner": {"allOf": [owner()], "const": {"name": "boss"}}}}})),
            ("nested wrappers", serde_json::json!({"properties": {"owner": {"anyOf": [{"allOf": [owner()]}, {"type": "null"}]}}})),
        ] {
            let direct = api(value);
            let source = RuleSource::new(&catalog, &direct.schemas);
            let StructDef { fields, .. } = match &catalog.types[0] { ModelDef::Struct(def) => def, _ => unreachable!() };
            let rebuilt_value = as_compiled(&Value::Object(source.field_rules("Item", &fields[0])));
            let rebuilt = api(rebuilt_value.clone());

            assert_eq!(findings(&rebuilt), findings(&direct), "{case}: {rebuilt_value}");
            let direct_plans = plans(&direct);
            assert_eq!(plans(&rebuilt), direct_plans, "{case}: {rebuilt_value}");
            assert!(direct_plans.contains(r#"Property::new("owner",&Plan::"#) && direct_plans.contains("COMPONENT_"), "{case}: {direct_plans}");
        }
        // Compositions the plans do not pass through are kept with their
        // members, in count and order, so the rebuilt value gives the direct
        // import's refusal without the opt-in and its warnings with it.
        let long = || serde_json::json!({"type": "string", "maxLength": 11});
        for (case, value, keyword) in [
            ("generic anyOf", serde_json::json!({"properties": {"owner": {"anyOf": [owner(), long()]}}}), "anyOf"),
            ("anyOf with an integer", serde_json::json!({"properties": {"code": {"anyOf": [long(), {"type": "integer"}]}}}), "anyOf"),
            ("oneOf", serde_json::json!({"properties": {"owner": {"oneOf": [owner(), {"type": "null"}]}}}), "oneOf"),
            ("oneOf with an integer", serde_json::json!({"items": {"properties": {"code": {"oneOf": [long(), {"type": "integer"}]}}}}), "oneOf"),
            ("two-member allOf", serde_json::json!({"properties": {"owner": {"allOf": [owner(), long()]}}}), "allOf"),
            ("not", serde_json::json!({"additionalProperties": {"not": long()}}), "not"),
        ] {
            let mut direct = api(value);
            let source = RuleSource::new(&catalog, &direct.schemas);
            let StructDef { fields, .. } = match &catalog.types[0] { ModelDef::Struct(def) => def, _ => unreachable!() };
            let rebuilt_value = as_compiled(&Value::Object(source.field_rules("Item", &fields[0])));
            assert!(rebuilt_value.to_string().contains(&format!("\"{keyword}\"")), "{case}: {rebuilt_value}");
            let mut rebuilt = api(rebuilt_value.clone());

            let refused = findings(&direct);
            assert!(!refused.is_empty() && refused.iter().all(|finding| finding.starts_with("Error")), "{case}: {refused:?}");
            assert_eq!(findings(&rebuilt), refused, "{case}: {rebuilt_value}");
            direct.unchecked_compositions = true;
            rebuilt.unchecked_compositions = true;
            let accepted = findings(&direct);
            assert!(accepted.iter().all(|finding| finding.starts_with("Warning")), "{case}: {accepted:?}");
            assert_eq!(findings(&rebuilt), accepted, "{case}: {rebuilt_value}");
            assert_eq!(plans(&rebuilt), plans(&direct), "{case}: {rebuilt_value}");
        }
    }

    /// A null-only member as an OpenAPI 3.0 document spells it.
    fn null_member() -> Value {
        serde_json::json!({"type": "string", "nullable": true, "enum": [null]})
    }

    /// An object holding `Owner` behind a nullable alternative and behind a
    /// one-member `allOf`.
    fn wrapped_owner() -> Value {
        let owner = serde_json::json!({"$ref": "#/components/schemas/Owner"});
        serde_json::json!({"properties": {
            "nullable_probe": {"anyOf": [owner, null_member()]},
            "allof_probe": {"allOf": [owner]}
        }})
    }

    /// The rebuilt rules of [`wrapped_owner`]: both wrappers keep their
    /// shape, the null member as written.
    fn wrapped_owner_rules() -> Value {
        let owner = serde_json::json!({MODEL_REFERENCE: "Owner"});
        serde_json::json!({"properties": {
            "nullable_probe": {"anyOf": [owner, null_member()]},
            "allof_probe": {"allOf": [owner]}
        }})
    }

    #[test]
    fn wrappers_and_generic_compositions_are_rebuilt_at_each_free_form_position() {
        let json = || primitive(PrimitiveType::Json);
        let owner = || serde_json::json!({"$ref": "#/components/schemas/Owner"});
        let plain = || serde_json::json!({"$ref": "#/components/schemas/Plain"});
        let long = || serde_json::json!({"type": "string", "maxLength": 11});
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![
                ModelDef::Struct(StructDef {
                    name: "Item".into(),
                    description: None,
                    fields: vec![
                        field("free_value", None, true, json()),
                        field("free_map", None, true, json()),
                        field("attributes", None, true, TypeRef::Map(Box::new(json()))),
                        field("entries", None, true, TypeRef::Array(Box::new(json()))),
                        field("wrapped", None, false, TypeRef::AnyOf(vec![TypeRef::Named("Owner".into()), primitive(PrimitiveType::String)])),
                        field("shapes", None, true, json()),
                        field("controls", None, true, json()),
                    ],
                    additional_properties: None,
                }),
                ModelDef::Struct(StructDef {
                    name: "Extras".into(),
                    description: None,
                    fields: vec![field("id", None, true, primitive(PrimitiveType::String))],
                    additional_properties: Some(json()),
                }),
                ModelDef::Struct(StructDef { name: "Owner".into(), description: None, fields: vec![field("name", None, false, primitive(PrimitiveType::String))], additional_properties: None }),
                ModelDef::Struct(StructDef { name: "Plain".into(), description: None, fields: vec![field("note", None, false, primitive(PrimitiveType::String))], additional_properties: None }),
            ],
        };
        let mut free_value = wrapped_owner();
        free_value["properties"]["inner"] = wrapped_owner();
        free_value["items"] = wrapped_owner();
        let component = |name: &str, schema: Value| SchemaComponent { name: name.into(), rust_path: Some(name.into()), schema };
        let components = vec![
            component("Item", serde_json::json!({"type": "object", "properties": {
                "free_value": free_value,
                "free_map": {"additionalProperties": wrapped_owner()},
                "attributes": {"type": "object", "additionalProperties": wrapped_owner()},
                "entries": {"type": "array", "items": wrapped_owner()},
                "wrapped": {"anyOf": [owner(), null_member()]},
                "shapes": {"properties": {
                    "null_first": {"anyOf": [{"type": "null"}, owner()]},
                    "nested": {"anyOf": [{"allOf": [owner()]}, null_member()]},
                    "inline": {"anyOf": [long(), {"type": "null"}]},
                    "beside": {"allOf": [owner()], "const": {"name": "boss"}}
                }},
                "controls": {"properties": {
                    "any_of": {"anyOf": [long(), {"type": "integer"}]},
                    "both_null": {"anyOf": [null_member(), {"type": "null"}]},
                    "three": {"anyOf": [owner(), long(), {"type": "null"}]},
                    "one_of": {"oneOf": [owner(), {"type": "null"}]},
                    "all_of": {"allOf": [owner(), long()]},
                    "not": {"not": long()},
                    "unruled": {"anyOf": [plain(), {"type": "null"}]},
                    "allof_unruled": {"allOf": [plain()]},
                    "allof_beside_ref": {"$ref": "#/components/schemas/Plain", "allOf": [owner()]},
                    "generic_unruled": {"anyOf": [plain(), {"type": "integer"}]},
                    "unruled_member": {"oneOf": [plain(), long()]}
                }}
            }})),
            component("Extras", serde_json::json!({"type": "object", "properties": {"id": {"type": "string"}}, "additionalProperties": wrapped_owner()})),
            component("Owner", serde_json::json!({"type": "object", "properties": {"name": {"type": "string", "maxLength": 10}}})),
            component("Plain", serde_json::json!({"type": "object", "properties": {"note": {"type": "string"}}})),
        ];
        let source = RuleSource::new(&catalog, &components);
        let ModelDef::Struct(item) = &catalog.types[0] else { unreachable!() };
        let rules = |name: &str| {
            let field = item.fields.iter().find(|field| field.name == name).unwrap();
            Value::Object(source.field_rules("Item", field))
        };
        let reference = serde_json::json!({MODEL_REFERENCE: "Owner"});

        // R3-F1.1–R3-F1.3: a free-form root, a nested property, and items.
        let mut expected = wrapped_owner_rules();
        expected["properties"]["inner"] = wrapped_owner_rules();
        expected["items"] = wrapped_owner_rules();
        assert_eq!(rules("free_value"), expected);
        // R3-F1.4: a free-form value's extra values.
        assert_eq!(rules("free_map"), serde_json::json!({"additionalProperties": wrapped_owner_rules()}));
        // R3-F1.5–R3-F1.6: a map-valued field and inline object items, over
        // the free-form base schema `{}`.
        assert_eq!(rules("attributes"), serde_json::json!({"additionalProperties": wrapped_owner_rules()}));
        assert_eq!(rules("entries"), serde_json::json!({"items": wrapped_owner_rules()}));
        // R3-F1.7: a struct's flattened extra values.
        let ModelDef::Struct(extras) = &catalog.types[1] else { unreachable!() };
        assert_eq!(source.struct_rules(extras).1.map(Value::Object), Some(wrapped_owner_rules()));
        // A property whose own schema is the nullable alternative, which
        // import types as a free-form value.
        assert_eq!(rules("wrapped"), serde_json::json!({"anyOf": [reference, null_member()]}));
        // Member order, nesting, inline rules, and a rule beside the wrapper.
        assert_eq!(
            rules("shapes"),
            serde_json::json!({"properties": {
                "null_first": {"anyOf": [{"type": "null"}, reference]},
                "nested": {"anyOf": [{"allOf": [reference]}, null_member()]},
                "inline": {"anyOf": [{"maxLength": 11}, {"type": "null"}]},
                "beside": {"allOf": [reference], "const": {"name": "boss"}}
            }})
        );
        // Generic compositions are kept with every member, in source order,
        // each member with its `type` and a reference to its model even
        // when the model holds no rule. A composition with no rule beneath
        // it, a wrapper of a model without rules, and a one-member `allOf`
        // beside the `$ref` the plans follow instead are left out.
        let rebuilt_long = serde_json::json!({"type": "string", "maxLength": 11});
        let plain_reference = serde_json::json!({MODEL_REFERENCE: "Plain"});
        assert_eq!(
            rules("controls"),
            serde_json::json!({"properties": {
                "any_of": {"anyOf": [rebuilt_long, {"type": "integer"}]},
                "three": {"anyOf": [reference, rebuilt_long, {"type": "null"}]},
                "one_of": {"oneOf": [reference, {"type": "null"}]},
                "all_of": {"allOf": [reference, rebuilt_long]},
                "not": {"not": rebuilt_long},
                "unruled_member": {"oneOf": [plain_reference, rebuilt_long]}
            }})
        );

        let code: String = format_code(&validate_code(&assemble_model_tokens_with(&catalog, Some(&components)).unwrap()).unwrap())
            .split_whitespace()
            .collect::<String>()
            .replace(",)", ")");
        for expected in [
            r#"#[serde(skip_serializing_if="Option::is_none")]#[serde(default)]#[schemars(schema_with="Item::field_rules_schema_wrapped")]pubwrapped:Option<serde_json::Value>,"#,
            r#"letreference_0=generator.subschema_for::<Owner>();letmutschema=generator.subschema_for::<Option<serde_json::Value>>();schema.insert("anyOf".to_owned(),serde_json::json!([reference_0,{"enum":[null],"nullable":true,"type":"string"}]));schema}"#,
            r#"#[schemars(schema_with="Extras::extra_values_rules_schema")]"#,
        ] {
            assert!(code.contains(expected), "missing {expected}\n{code}");
        }
        assert!(!code.contains(MODEL_REFERENCE), "{code}");
    }

    #[test]
    fn a_declared_name_without_rules_is_left_out_when_no_extra_value_rule_is_kept() {
        let catalog = ModelCatalog { module_path: None, types: vec![] };
        let source = RuleSource::new(&catalog, &[]);
        let rules = |schema: Value| source.free_form_rules(schema.as_object().unwrap());

        // No `additionalProperties` at all.
        assert_eq!(
            rules(serde_json::json!({"properties": {"label": {"type": "string"}, "code": {"maxLength": 2}}})),
            serde_json::json!({"properties": {"code": {"maxLength": 2}}}).as_object().cloned().unwrap()
        );
        // An extra-value schema without rules, and the Boolean forms, which
        // are not rules here.
        for extra in [serde_json::json!({"type": "string"}), Value::Bool(false), Value::Bool(true)] {
            let schema = serde_json::json!({"properties": {"label": {"type": "string"}, "code": {"maxLength": 2}}, "additionalProperties": extra});
            assert_eq!(rules(schema), serde_json::json!({"properties": {"code": {"maxLength": 2}}}).as_object().cloned().unwrap(), "{extra}");
        }
        // Nothing at all is emitted for an object without rules.
        assert!(rules(serde_json::json!({"properties": {"label": {"type": "string"}}, "additionalProperties": {}})).is_empty());
        // A Boolean property schema is still a declared name.
        assert_eq!(
            rules(serde_json::json!({"properties": {"flag": true}, "additionalProperties": {"maxLength": 3}})),
            serde_json::json!({"properties": {"flag": {}}, "additionalProperties": {"maxLength": 3}}).as_object().cloned().unwrap()
        );
    }

    /// R3-F1.B: each generic composition keeps its keyword, every member in
    /// source order (a null-only member as written), and each member's
    /// `type`; one with no rule beneath it is left out, as before.
    #[test]
    fn free_form_rules_keep_each_generic_composition_with_a_rule_beneath_it() {
        let catalog = ModelCatalog { module_path: None, types: vec![] };
        let source = RuleSource::new(&catalog, &[]);
        let rules = |schema: Value| Value::Object(source.free_form_rules(schema.as_object().unwrap()));
        let long = || serde_json::json!({"type": "string", "maxLength": 11});

        for (keyword, members) in [
            ("anyOf", serde_json::json!([long(), {"type": "integer"}])),
            ("oneOf", serde_json::json!([long(), {"type": "integer"}])),
            ("allOf", serde_json::json!([long(), {"type": "string"}])),
        ] {
            assert_eq!(rules(serde_json::json!({keyword: members})), serde_json::json!({keyword: members}), "{keyword}");
        }
        assert_eq!(rules(serde_json::json!({"not": long()})), serde_json::json!({"not": long()}));
        // Nested rules of a member are rebuilt as a free-form value's are.
        assert_eq!(
            rules(serde_json::json!({"oneOf": [
                {"type": "object", "properties": {"label": {"type": "string"}, "code": {"type": "string", "maxLength": 2}}},
                {"type": "null"}
            ]})),
            serde_json::json!({"oneOf": [{"type": "object", "properties": {"code": {"maxLength": 2}}}, {"type": "null"}]})
        );
        // A composition with no rule beneath it adds nothing to check, so
        // it is left out: the free-form value's schema stays `{}`.
        for unruled in [
            serde_json::json!({"anyOf": [{"type": "string"}, {"type": "integer"}]}),
            serde_json::json!({"oneOf": [{"type": "string"}, {"type": "null"}]}),
            serde_json::json!({"allOf": [{"type": "string"}, {"type": "string"}]}),
            serde_json::json!({"not": {"type": "string"}}),
        ] {
            assert_eq!(rules(unruled.clone()), serde_json::json!({}), "{unruled}");
        }
    }

    /// R3-F1.11: a nominal union's variants carry their member's rules, a
    /// null-only member is written as the source spells it, and a variant
    /// reaching a model through a free-form value is written through the
    /// generator. R3-F1.12: a struct flattening an `allOf` keeps the source
    /// `allOf` with each member rebuilt, a model as a `$ref` to its
    /// component.
    #[test]
    fn nominal_compositions_rebuild_their_members_rules() {
        let union = |name: &str, target: TypeRef| ModelDef::Alias(TypeAlias { name: name.into(), description: None, target, discriminator: None });
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![
                union("Stop", TypeRef::OneOf(vec![primitive(PrimitiveType::String), TypeRef::Array(Box::new(primitive(PrimitiveType::String)))])),
                union("Partial", TypeRef::AnyOf(vec![primitive(PrimitiveType::Integer), primitive(PrimitiveType::Json)])),
                union("Linked", TypeRef::OneOf(vec![primitive(PrimitiveType::Json), primitive(PrimitiveType::Integer)])),
                union("Combined", TypeRef::AllOf(vec![TypeRef::Named("Owner".into()), primitive(PrimitiveType::Json)])),
                union("Unruled", TypeRef::AllOf(vec![TypeRef::Named("Plain".into()), primitive(PrimitiveType::Json)])),
                union("Bare", TypeRef::AnyOf(vec![primitive(PrimitiveType::Json), primitive(PrimitiveType::Json)])),
                ModelDef::Struct(StructDef { name: "Owner".into(), description: None, fields: vec![field("name", None, false, primitive(PrimitiveType::String))], additional_properties: None }),
                ModelDef::Struct(StructDef { name: "Plain".into(), description: None, fields: vec![field("note", None, false, primitive(PrimitiveType::String))], additional_properties: None }),
            ],
        };
        let component = |name: &str, schema: Value| SchemaComponent { name: name.into(), rust_path: Some(name.into()), schema };
        let components = vec![
            component("Stop", serde_json::json!({"oneOf": [{"type": "string"}, {"type": "array", "minItems": 1, "maxItems": 4, "items": {"type": "string"}}]})),
            component("Partial", serde_json::json!({"anyOf": [{"type": "integer", "minimum": 0, "maximum": 3}, {"type": "null"}]})),
            component("Linked", serde_json::json!({"oneOf": [{"properties": {"owner": {"$ref": "#/components/schemas/Owner"}}}, {"type": "integer"}]})),
            component("Combined", serde_json::json!({"allOf": [
                {"$ref": "#/components/schemas/Owner"},
                {"type": "object", "properties": {"code": {"type": "string", "maxLength": 11}}}
            ]})),
            component("Unruled", serde_json::json!({"allOf": [{"$ref": "#/components/schemas/Plain"}, {"type": "object", "properties": {"label": {"type": "string"}}}]})),
            component("Bare", serde_json::json!({"anyOf": [{"type": "object"}, {"type": "null"}]})),
            component("Owner", serde_json::json!({"type": "object", "properties": {"name": {"type": "string", "maxLength": 10}}})),
            component("Plain", serde_json::json!({"type": "object", "properties": {"note": {"type": "string"}}})),
        ];
        let source = RuleSource::new(&catalog, &components);
        let variants = |index: usize| match &catalog.types[index] {
            ModelDef::Alias(def @ TypeAlias { target: TypeRef::OneOf(variants) | TypeRef::AnyOf(variants), .. }) => {
                source.variant_rules(def, variants).into_iter().map(Value::Object).collect::<Vec<_>>()
            }
            _ => unreachable!(),
        };
        assert_eq!(variants(0), vec![serde_json::json!({}), serde_json::json!({"minItems": 1, "maxItems": 4})]);
        assert_eq!(variants(1), vec![serde_json::json!({"minimum": 0, "maximum": 3}), serde_json::json!({"type": "null"})]);
        assert_eq!(variants(2), vec![serde_json::json!({"properties": {"owner": {MODEL_REFERENCE: "Owner"}}}), serde_json::json!({})]);
        // A union without rules is written as Schemars derives it.
        assert_eq!(variants(5), Vec::<Value>::new());

        let code: String = format_code(&validate_code(&assemble_model_tokens_with(&catalog, Some(&components)).unwrap()).unwrap())
            .split_whitespace()
            .collect::<String>()
            .replace(",)", ")");
        for expected in [
            r#"#[schemars(extend("maxItems"=4u64,"minItems"=1u64))]TextList(Vec<String>)"#,
            r#"#[schemars(extend("maximum"=3u64,"minimum"=0u64))]Integer(i64)"#,
            r#"#[schemars(extend("type"="null"))]Json(serde_json::Value)"#,
            r#"#[schemars(schema_with="Linked::variant_rules_schema_0")]Json(serde_json::Value)"#,
            r##"#[schemars(extend("allOf"=[{"$ref":"#/components/schemas/Owner"},{"properties":{"code":{"maxLength":11u64}},"type":"object"}]))]pubstructCombined"##,
        ] {
            assert!(code.contains(expected), "missing {expected}\n{code}");
        }
        // No rule beneath the `allOf`: Schemars' flattened struct stands.
        assert!(code.contains("#[derive(JsonSchema)]pubstructUnruled"), "{code}");
        assert!(!code.contains(MODEL_REFERENCE), "{code}");
    }

    #[test]
    fn plain_alias_body_rules_become_constraints_but_nominal_bodies_do_not() {
        let (catalog, components) = constrained_catalog();
        let rules = RuleSource::new(&catalog, &components);

        assert_eq!(
            rules.body_constraints("Rename", &TypeRef::Named("Slug".into())),
            Some(Constraints::new().max_length(8).pattern("^[a-z-]+$"))
        );
        assert_eq!(rules.body_constraints("Create", &TypeRef::Named("Plain".into())), None);
        assert_eq!(rules.body_constraints("Echo", &primitive(PrimitiveType::String)), None);
        rules.finish().expect("every body rule is carried");
    }

    /// `Forest`, a plain array alias over inline objects with a constrained
    /// `label`; `Labels`, a plain array alias over constrained strings;
    /// `RecursiveForest`, whose inline objects also hold `children`, a
    /// reference back to it; `Grove`, a struct whose free-form `trees` holds
    /// that reference; and `Owner`, a model with a rule.
    fn forest_catalog() -> (ModelCatalog, Vec<SchemaComponent>) {
        let json = || primitive(PrimitiveType::Json);
        let array_of_json = || TypeRef::Array(Box::new(json()));
        let alias = |name: &str, target: TypeRef| ModelDef::Alias(TypeAlias { name: name.into(), description: None, target, discriminator: None });
        let catalog = ModelCatalog {
            module_path: None,
            types: vec![
                alias("Forest", array_of_json()),
                alias("Labels", TypeRef::Array(Box::new(primitive(PrimitiveType::String)))),
                alias("RecursiveForest", array_of_json()),
                alias("Owners", array_of_json()),
                ModelDef::Struct(StructDef { name: "Grove".into(), description: None, fields: vec![field("trees", None, true, json())], additional_properties: None }),
                ModelDef::Struct(StructDef { name: "Owner".into(), description: None, fields: vec![field("name", None, false, primitive(PrimitiveType::String))], additional_properties: None }),
            ],
        };
        let label = || serde_json::json!({"type": "string", "maxLength": 11});
        let component = |name: &str, schema: Value| SchemaComponent { name: name.into(), rust_path: Some(name.into()), schema };
        let components = vec![
            component("Forest", serde_json::json!({"type": "array", "maxItems": 3, "items": {"type": "object", "properties": {"label": label()}}})),
            component("Labels", serde_json::json!({"type": "array", "maxItems": 3, "items": {"type": "string", "maxLength": 5}})),
            component("RecursiveForest", serde_json::json!({"type": "array", "items": {"type": "object", "properties": {
                "label": label(),
                "children": {"$ref": "#/components/schemas/RecursiveForest"}
            }}})),
            component("Owners", serde_json::json!({"type": "array", "items": {"$ref": "#/components/schemas/Owner"}})),
            component("Grove", serde_json::json!({"type": "object", "properties": {"trees": {"$ref": "#/components/schemas/RecursiveForest"}}})),
            component("Owner", serde_json::json!({"type": "object", "properties": {"name": {"type": "string", "maxLength": 10}}})),
        ];
        (catalog, components)
    }

    #[test]
    fn a_plain_alias_body_with_a_rule_beneath_its_items_object_fails_at_the_rule() {
        let (catalog, components) = forest_catalog();
        let rules = RuleSource::new(&catalog, &components);

        rules.body_constraints("PlantForest", &TypeRef::Named("Forest".into()));
        let error = rules.finish().expect_err("the label rule cannot ride on request constraints").to_string();

        for expected in ["endpoint `PlantForest`", "plain alias `Forest`", "`#/components/schemas/Forest/items/properties/label/maxLength`", "component name"] {
            assert!(error.contains(expected), "missing {expected}: {error}");
        }
    }

    #[test]
    fn a_plain_alias_body_reaching_a_model_with_rules_through_a_free_form_item_fails_at_the_reference() {
        let (catalog, components) = forest_catalog();
        let rules = RuleSource::new(&catalog, &components);

        rules.body_constraints("SetOwners", &TypeRef::Named("Owners".into()));
        let error = rules.finish().expect_err("the reference cannot ride on request constraints").to_string();

        assert!(error.contains("`#/components/schemas/Owners/items/$ref`"), "{error}");
    }

    #[test]
    fn a_plain_alias_body_of_constrained_scalars_keeps_its_item_rules_as_constraints() {
        let (catalog, components) = forest_catalog();
        let rules = RuleSource::new(&catalog, &components);

        let constraints = rules.body_constraints("SetLabels", &TypeRef::Named("Labels".into()));

        assert_eq!(constraints, Some(Constraints::new().max_items(3).items(Constraints::new().max_length(5))));
        rules.finish().expect("item rules are carried");
    }

    #[test]
    fn a_ruled_plain_alias_reached_again_through_a_free_form_value_fails_the_rebuild() {
        let (catalog, components) = forest_catalog();

        let error = assemble_model_tokens_with(&catalog, Some(&components)).expect_err("the inner occurrence would lose its rules").to_string();

        for expected in ["plain alias `RecursiveForest`", "`$ref: #/components/schemas/RecursiveForest`", "component name"] {
            assert!(error.contains(expected), "missing {expected}: {error}");
        }

        // Without a rule, the recursion loses nothing and rebuilds.
        let mut unruled = components.clone();
        unruled.retain(|component| component.name != "Forest" && component.name != "Owners");
        let recursive = unruled.iter_mut().find(|component| component.name == "RecursiveForest").unwrap();
        recursive.schema["items"]["properties"]["label"] = serde_json::json!({"type": "string"});
        assemble_model_tokens_with(&catalog, Some(&unruled)).expect("a recursive alias without rules rebuilds");
    }
}
