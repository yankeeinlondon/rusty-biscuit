//! The expression-function catalog: one descriptor per function overload,
//! projected once from the embedded `darkmatter/schemas/partials/functions.yaml`.
//!
//! The file is a tagged SimplifiedSchema document (`kind: schema`,
//! `categories:`, `types:`). Each `types:` entry whose definition is a
//! `function(...)`, or a union of them (an overload set), is a function; every
//! other entry is a named helper type that signatures may reference
//! (`agentic-cli`, `ip-address`), never a function. Reading the catalog
//! performs no host probes, no filesystem I/O, and no context capture: the
//! file and its file-form examples are compiled in.
//!
//! Every function is also registered for dispatch; the registry
//! (`expression::functions`) compiles each function's overloads from these
//! descriptors and checks catalog/registry parity both ways.

use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use simplified_schema::shipped::PreparationMode;

use crate::catalog::{Described, Example, ExampleVerification};
use crate::markdown::compose::expression::call_binding::TypeResolver;
use simplified_schema::simplified::{
    Constraint, ExampleOutcome, FunctionType, LOCAL_REFERENCE, PropertyAtom, PropertyDef, SchemaRoot,
    StandaloneSchemaEnvelope, TypeExpr, parse_standalone_schema_document_with_mode, serialize_property_def,
};

mod roots;

pub use roots::{ReservedRootDescriptor, RootEvaluation, RootMembers, reserved_root_descriptors};

/// The embedded catalog file's repository path, used in load errors.
pub(crate) const CATALOG_PATH: &str = "darkmatter/schemas/partials/functions.yaml";

pub(crate) const CATALOG_SOURCE: &str = include_str!("../../../../../../schemas/partials/functions.yaml");

/// The embedded function catalog's text, which a schema-slot resolution is
/// keyed on: a build whose catalog differs never reuses a cached result.
pub(crate) fn catalog_source() -> &'static str {
    CATALOG_SOURCE
}

/// The file-form example artifacts a catalog entry may reference with
/// `example(./function-examples/<name>.yaml)`, keyed by that reference
/// without its leading `./`. Compiled in so reading the catalog never touches
/// the disk; an entry naming a file missing here is a load error.
pub(crate) const EXAMPLE_FILES: &[(&str, &str)] = &[
    (
        "function-examples/as_line_separated.yaml",
        include_str!("../../../../../../schemas/partials/function-examples/as_line_separated.yaml"),
    ),
    (
        "function-examples/as_ordered_list.yaml",
        include_str!("../../../../../../schemas/partials/function-examples/as_ordered_list.yaml"),
    ),
    (
        "function-examples/as_tsv.yaml",
        include_str!("../../../../../../schemas/partials/function-examples/as_tsv.yaml"),
    ),
    (
        "function-examples/as_unordered_list.yaml",
        include_str!("../../../../../../schemas/partials/function-examples/as_unordered_list.yaml"),
    ),
];

/// The data files the catalog's inline examples read (`load_data("fixture.csv", …)`),
/// keyed by file name. The example runner writes each into the directory it
/// evaluates in, and a generated function page shows the ones its examples
/// name, so the input a reader sees is the input the examples are checked
/// against.
pub(crate) const EXAMPLE_DATA_FILES: &[(&str, &str)] = &[
    ("fixture.yaml", include_str!("example_data/fixture.yaml")),
    ("fixture.json", include_str!("example_data/fixture.json")),
    ("fixture.jsonl", include_str!("example_data/fixture.jsonl")),
    ("fixture.csv", include_str!("example_data/fixture.csv")),
    ("fixture.tsv", include_str!("example_data/fixture.tsv")),
    ("fixture.toml", include_str!("example_data/fixture.toml")),
    ("fixture.json5", include_str!("example_data/fixture.json5")),
    ("fixture-headerless.csv", include_str!("example_data/fixture-headerless.csv")),
    ("fixture-semicolon.txt", include_str!("example_data/fixture-semicolon.txt")),
    ("fixture-export", include_str!("example_data/fixture-export")),
];

/// One entry of the catalog's `categories:` list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogCategory {
    pub name: Box<str>,
    #[serde(deserialize_with = "simplified_schema::shipped::present")]
    pub description: Option<Box<str>>,
}

/// One parameter of a function overload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionParameterDescriptor {
    pub name: Box<str>,
    /// The declared type in canonical grammar spelling, without descriptions
    /// (`number | null`, `number(integer; min(1))`).
    pub type_text: Box<str>,
    pub optional: bool,
    pub rest: bool,
}

/// An example attached to a function overload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FunctionExample {
    /// `example(CALL => RESULT)`; `result` is the authored literal text or
    /// `error`.
    Call {
        call: Box<str>,
        expected: ExampleOutcome,
        result: Box<str>,
        #[serde(deserialize_with = "simplified_schema::shipped::present")]
        display_only: Option<Box<str>>,
    },
    /// `example(./function-examples/x.yaml)`, read from the embedded file.
    /// `returns` is the file's `returns` text, or its JSON spelling when it is
    /// not a string.
    File {
        path: Box<str>,
        invocation: Box<str>,
        returns: Box<str>,
        #[serde(deserialize_with = "simplified_schema::shipped::present")]
        description: Option<Box<str>>,
    },
}

/// Descriptor for one overload of an expression function.
///
/// Owns every field. A descriptor borrowed from a static catalog (the
/// built-in or mutation catalog) yields `&'static` field views; one borrowed
/// from a host catalog lives as long as that catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionFunctionDescriptor {
    /// The canonical name.
    pub name: Box<str>,
    /// The aliases dispatch also accepts for this function.
    pub aliases: Box<[Box<str>]>,
    /// The display key: parameter names only, optional ones bracketed and a
    /// rest parameter as `...` (`round(x, [fallback])`, `and(...)`). Read by
    /// `md schema about`, the generated doc table, DMLS, and lifecycle verb
    /// parsing.
    pub signature: Box<str>,
    pub description: Box<str>,
    pub category: Box<str>,
    /// 1-based position within its category, counting every overload.
    pub order: usize,
    /// Index within the function's overloads, in authored order.
    pub overload: usize,
    pub parameters: Box<[FunctionParameterDescriptor]>,
    /// The return type in canonical grammar spelling.
    pub returns: Box<str>,
    pub fallible: bool,
    pub function_type: FunctionType,
    pub examples: Box<[FunctionExample]>,
}

impl ExpressionFunctionDescriptor {
    /// Whether YAML value operands can represent every parameter. Inline
    /// callbacks are expression syntax and cannot be lifecycle verb operands.
    pub fn accepts_value_arguments(&self) -> bool {
        fn contains_atom(atom: &PropertyAtom) -> bool {
            match &atom.ty {
                TypeExpr::Function(_) => true,
                TypeExpr::InlineObject(shape) => shape.properties.values().any(contains_def),
                TypeExpr::Tuple(tuple) => tuple.any_atom(contains_atom),
                TypeExpr::Union(arms) => arms.iter().any(contains_atom),
                _ => false,
            }
        }
        fn contains_def(def: &PropertyDef) -> bool {
            match def {
                PropertyDef::Single(atom) => contains_atom(atom),
                PropertyDef::Union(arms) => arms.iter().any(contains_atom),
            }
        }
        !self.function_type.parameters.as_slice().iter().any(|parameter| contains_def(&parameter.def))
    }


    /// The fully typed signature, e.g.
    /// `round(x: numberlike, [fallback: number]) -> number` or
    /// `and(...values: unknown | null) -> boolean`. A fallible function's
    /// return ends in ` | error`.
    pub fn typed_signature(&self) -> String {
        let parameters: Vec<String> = self
            .parameters
            .iter()
            .map(|parameter| {
                let FunctionParameterDescriptor { name, type_text, .. } = parameter;
                if parameter.rest {
                    format!("...{name}: {type_text}")
                } else if parameter.optional {
                    format!("[{name}: {type_text}]")
                } else {
                    format!("{name}: {type_text}")
                }
            })
            .collect();
        let error = if self.fallible { " | error" } else { "" };
        format!("{}({}) -> {}{error}", self.name, parameters.join(", "), self.returns)
    }
}

impl Described for ExpressionFunctionDescriptor {
    fn key(&self) -> &str {
        &self.signature
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn category(&self) -> &str {
        &self.category
    }
    fn order(&self) -> usize {
        self.order
    }
    /// The first inline example; a file-form example has no shared view.
    fn example(&self) -> Option<Example<'_>> {
        self.examples.iter().find_map(|example| match example {
            FunctionExample::Call {
                call,
                result,
                display_only,
                ..
            } => Some(Example {
                invocation: call,
                result,
                verification: display_only
                    .as_deref()
                    .map_or(ExampleVerification::Executable, ExampleVerification::DisplayOnly),
            }),
            FunctionExample::File { .. } => None,
        })
    }
}

/// The loaded catalog: categories in list order and every overload's
/// descriptor in documentation order (category list order, then canonical
/// name, then authored overload order).
///
/// Serializable as shipped-artifact data in that order; a decoded catalog
/// is checked with [`Self::check_record`] before use.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionCatalog {
    pub categories: Box<[CatalogCategory]>,
    pub descriptors: Box<[ExpressionFunctionDescriptor]>,
    /// The non-function `types:` entries, for resolving references in
    /// signatures.
    named_types: IndexMap<String, PropertyDef>,
}

impl FunctionCatalog {
    /// Every overload of the function `name` names: case-insensitive, by
    /// canonical name or registered alias. Empty for an unknown name.
    pub fn descriptors_for(&self, name: &str) -> &[ExpressionFunctionDescriptor] {
        let lower = name.to_ascii_lowercase();
        let canonical = super::functions::alias_canonical(&lower).unwrap_or(lower.as_str());
        let descriptors = &self.descriptors;
        let Some(start) = descriptors.iter().position(|descriptor| &*descriptor.name == canonical) else {
            return &[];
        };
        let len = descriptors[start..].iter().take_while(|descriptor| &*descriptor.name == canonical).count();
        &descriptors[start..start + len]
    }

    /// The catalog's named helper types, as a [`TypeResolver`] for compiling
    /// signatures that reference them.
    pub(crate) fn type_resolver(&self) -> impl TypeResolver + '_ {
        move |name: &str, reference: &str| {
            (reference == LOCAL_REFERENCE).then(|| self.named_types.get(name).cloned()).flatten()
        }
    }

    /// The names of the non-function `types:` entries, in authored order.
    pub fn named_type_names(&self) -> impl Iterator<Item = &str> {
        self.named_types.keys().map(String::as_str)
    }

    /// The definition of a named helper type.
    pub fn named_type(&self, name: &str) -> Option<&PropertyDef> {
        self.named_types.get(name)
    }

    /// Checks a decoded catalog's invariants, which the loader guarantees
    /// and lookups rely on: every descriptor names a declared category, a
    /// function's overloads are adjacent and numbered from 0 in order, and
    /// each category's positions count 1, 2, … in descriptor order.
    ///
    /// ## Errors
    ///
    /// A [`simplified_schema::shipped::RecordError`] whose pointer is
    /// relative to the catalog (`/descriptors/4`).
    pub fn check_record(&self) -> Result<(), simplified_schema::shipped::RecordError> {
        use simplified_schema::shipped::RecordError;
        let mut positions = vec![0usize; self.categories.len()];
        let mut seen: Vec<&str> = Vec::new();
        for (index, descriptor) in self.descriptors.iter().enumerate() {
            let at = format!("/descriptors/{index}");
            let category = self
                .categories
                .iter()
                .position(|category| category.name == descriptor.category)
                .ok_or_else(|| RecordError::new(format!("{at}/category"), format!("`{}` is not a declared category", descriptor.category)))?;
            positions[category] += 1;
            if descriptor.order != positions[category] {
                return Err(RecordError::new(format!("{at}/order"), format!("position {} is recorded where {} belongs", descriptor.order, positions[category])));
            }
            let continues = index > 0 && self.descriptors[index - 1].name == descriptor.name;
            let expected_overload = if continues { self.descriptors[index - 1].overload + 1 } else { 0 };
            if !continues {
                if seen.contains(&&*descriptor.name) {
                    return Err(RecordError::new(format!("{at}/name"), format!("`{}`'s overloads are not adjacent", descriptor.name)));
                }
                seen.push(&descriptor.name);
            }
            if descriptor.overload != expected_overload {
                return Err(RecordError::new(format!("{at}/overload"), format!("overload {} is recorded where {expected_overload} belongs", descriptor.overload)));
            }
        }
        Ok(())
    }
}

/// The embedded catalog: the accepted shipped artifact's
/// ([`crate::markdown::schemas::shipped`]), or, when that artifact or its
/// section was refused, loaded once from the embedded file.
///
/// ## Panics
///
/// When the artifact was refused and the embedded file does not load; a
/// unit test loads it.
pub fn expression_function_catalog() -> &'static FunctionCatalog {
    crate::markdown::schemas::shipped::registry().function_catalog()
}

/// [`expression_function_catalog`] loaded from the embedded file, once per
/// process: the fallback when the shipped artifact was refused.
pub(crate) fn expression_function_catalog_from_sources() -> &'static FunctionCatalog {
    static CATALOG: LazyLock<FunctionCatalog> = LazyLock::new(|| {
        crate::note_work!(ShippedRuntimePreparation);
        load_catalog(CATALOG_SOURCE, CATALOG_PATH, EXAMPLE_FILES)
            .unwrap_or_else(|error| panic!("the embedded expression-function catalog does not load: {error}"))
    });
    &CATALOG
}

/// Every overload's descriptor, in documentation order.
pub fn expression_function_descriptors() -> &'static [ExpressionFunctionDescriptor] {
    &expression_function_catalog().descriptors
}

/// Every overload of the function `name` names, resolving aliases and case
/// like dispatch.
pub fn function_descriptors(name: &str) -> &'static [ExpressionFunctionDescriptor] {
    expression_function_catalog().descriptors_for(name)
}

/// Why a catalog document does not load.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CatalogError {
    #[error(transparent)]
    Schema(#[from] simplified_schema::SchemaError),
    #[error("{path}: {message}")]
    Invalid { path: String, message: String },
}

/// Loads a catalog document. Strict: the tagged envelope with `categories:`,
/// each function entry a `function` or a union of `function`s, one category
/// per function, no entry written as null, and every file-form example present in `example_files` and
/// valid under the example envelope.
///
/// The result owns every name, type, and example it projects and borrows
/// nothing from `source` or `example_files`; nothing is built on an error
/// path that outlives the call.
pub(crate) fn load_catalog(
    source: &str,
    path: &str,
    example_files: &[(&str, &str)],
) -> Result<FunctionCatalog, CatalogError> {
    load_catalog_with_mode(source, path, example_files, PreparationMode::Runtime)
}

/// [`load_catalog`] in `mode`; shipped-artifact generation passes
/// [`PreparationMode::Uncached`].
pub(crate) fn load_catalog_with_mode(
    source: &str,
    path: &str,
    example_files: &[(&str, &str)],
    mode: PreparationMode<'_>,
) -> Result<FunctionCatalog, CatalogError> {
    let invalid = |message: String| CatalogError::Invalid {
        path: path.to_string(),
        message,
    };
    let document = parse_standalone_schema_document_with_mode(source, std::path::Path::new(path), mode)?
        .ok_or_else(|| invalid("not a SimplifiedSchema document; expected `kind: schema`".into()))?;
    if document.envelope != StandaloneSchemaEnvelope::Tagged {
        return Err(invalid("expected the tagged envelope (`kind: schema`, `categories:`, `types:`)".into()));
    }
    let Some(categories) = document.categories.as_ref() else {
        return Err(invalid("missing the `categories:` list".into()));
    };
    let Some(SchemaRoot::Single(types)) = document.schema() else {
        return Err(invalid("`types:` must be a mapping of named types".into()));
    };
    let raw_entries = raw_type_entries(source);
    if let Some(name) = null_type_entry(source) {
        // A general schema file reads `types.x: null` as the `null` type; in
        // the catalog that can only be a function signature left empty.
        return Err(invalid(format!("`types.{name}` is null, not a function signature")));
    }

    let mut named_types = IndexMap::new();
    let mut functions: Vec<CatalogFunction<'_>> = Vec::new();
    for (name, def) in &types.properties {
        let atoms = overload_atoms(def);
        let function_count = atoms.iter().filter(|atom| is_function_atom(atom)).count();
        if function_count == 0 {
            named_types.insert(name.clone(), def.clone());
            continue;
        }
        if function_count != atoms.len() {
            return Err(invalid(format!(
                "`types.{name}` mixes `function(...)` options with other types; a function entry is a \
                 `function` or a union of `function`s"
            )));
        }
        let mut category = None;
        for atom in &atoms {
            if let Some(found) = category_of(atom) {
                match category {
                    None => category = Some(found),
                    Some(first) if first != found => {
                        return Err(invalid(format!(
                            "`types.{name}`: overloads name different categories (`{first}`, `{found}`)"
                        )));
                    }
                    Some(_) => {}
                }
            }
        }
        let category = category.ok_or_else(|| invalid(format!("`types.{name}` has no `category(...)`")))?;
        let declarations: Vec<_> = atoms
            .iter()
            .filter_map(|atom| match &atom.ty {
                TypeExpr::Function(function_type) => Some((**function_type).clone()),
                _ => None,
            })
            .collect();
        super::call_binding::compile_declared_fact(name, &declarations)
            .map_err(|error| invalid(format!("`types.{name}`: {error}")))?;
        let category_index = categories
            .iter()
            .position(|declared| declared.name == category)
            .ok_or_else(|| invalid(format!("`types.{name}`: category `{category}` is not in `categories:`")))?;
        let raw = raw_entries.get(name.as_str()).cloned().unwrap_or_default();
        let raw = if raw.len() == atoms.len() { raw } else { vec![None; atoms.len()] };
        functions.push(CatalogFunction {
            category_index,
            name,
            atoms,
            raw,
        });
    }
    functions.sort_by(|left, right| (left.category_index, left.name).cmp(&(right.category_index, right.name)));

    let catalog_categories: Box<[CatalogCategory]> = categories
        .iter()
        .map(|category| CatalogCategory {
            name: category.name.as_str().into(),
            description: category.description.as_deref().map(Box::from),
        })
        .collect();

    let mut descriptors = Vec::new();
    let mut position_in_category = vec![0usize; catalog_categories.len()];
    for CatalogFunction {
        category_index,
        name,
        atoms,
        raw,
    } in functions
    {
        let aliases: Box<[Box<str>]> =
            super::functions::registered_aliases(name).iter().map(|alias| Box::from(*alias)).collect();
        for (overload, (atom, raw)) in atoms.into_iter().zip(raw).enumerate() {
            let TypeExpr::Function(function_type) = &atom.ty else {
                unreachable!("every overload atom is a function");
            };
            position_in_category[category_index] += 1;
            let examples = project_examples(name, atom, function_type, raw.as_deref(), example_files)
                .map_err(&invalid)?;
            let parameters: Box<[FunctionParameterDescriptor]> = function_type
                .parameters
                .as_slice()
                .iter()
                .map(|parameter| FunctionParameterDescriptor {
                    name: parameter.name.as_str().into(),
                    type_text: type_text(&parameter.def).into(),
                    optional: parameter.optional,
                    rest: parameter.rest,
                })
                .collect();
            let display: Vec<String> = parameters
                .iter()
                .map(|parameter| {
                    if parameter.rest {
                        "...".to_string()
                    } else if parameter.optional {
                        format!("[{}]", parameter.name)
                    } else {
                        parameter.name.to_string()
                    }
                })
                .collect();
            descriptors.push(ExpressionFunctionDescriptor {
                name: name.as_str().into(),
                aliases: aliases.clone(),
                signature: format!("{name}({})", display.join(", ")).into(),
                description: atom.description.clone().unwrap_or_default().into(),
                category: catalog_categories[category_index].name.clone(),
                order: position_in_category[category_index],
                overload,
                parameters,
                returns: type_text(&function_type.returns).into(),
                fallible: function_type.fallible,
                function_type: (**function_type).clone(),
                examples: examples.into(),
            });
        }
    }

    Ok(FunctionCatalog {
        categories: catalog_categories,
        descriptors: descriptors.into(),
        named_types,
    })
}

/// One function entry while loading.
struct CatalogFunction<'a> {
    category_index: usize,
    name: &'a String,
    /// One `function(...)` atom per overload, in authored order.
    atoms: Vec<&'a PropertyAtom>,
    /// Each overload's authored text, when it could be recovered.
    raw: Vec<Option<String>>,
}

/// The options of a `types:` entry: the arms of a union (list form or `|`
/// form), or the single atom.
fn overload_atoms(def: &PropertyDef) -> Vec<&PropertyAtom> {
    match def {
        PropertyDef::Single(atom) => match &atom.ty {
            TypeExpr::Union(arms) if !atom.is_array => arms.iter().collect(),
            _ => vec![atom],
        },
        PropertyDef::Union(arms) => arms.iter().collect(),
    }
}

fn is_function_atom(atom: &PropertyAtom) -> bool {
    matches!(atom.ty, TypeExpr::Function(_)) && !atom.is_array
}

fn category_of(atom: &PropertyAtom) -> Option<&str> {
    atom.constraints.iter().find_map(|constraint| match constraint {
        Constraint::Category(name) => Some(name.as_str()),
        _ => None,
    })
}

/// Each `types:` entry's authored overload texts (one per list item, or the
/// single string), so an inline example's result can be shown as written.
/// Missing or unexpected shapes yield no text; the result then falls back to
/// its JSON spelling.
/// The first `types:` entry written as `null`, `~`, or an empty value.
fn null_type_entry(source: &str) -> Option<String> {
    let document = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source).ok()?;
    let types = document.get("types")?.as_mapping()?;
    types
        .iter()
        .find(|(_, value)| value.is_null())
        .and_then(|(key, _)| key.as_str().map(str::to_string))
}

fn raw_type_entries(source: &str) -> IndexMap<String, Vec<Option<String>>> {
    let Ok(document) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source) else {
        return IndexMap::new();
    };
    let Some(types) = document.get("types").and_then(serde_yaml_ng::Value::as_mapping) else {
        return IndexMap::new();
    };
    types
        .iter()
        .filter_map(|(key, value)| {
            let texts = match value {
                serde_yaml_ng::Value::String(text) => vec![Some(text.clone())],
                serde_yaml_ng::Value::Sequence(items) => {
                    items.iter().map(|item| item.as_str().map(str::to_string)).collect()
                }
                _ => return None,
            };
            Some((key.as_str()?.to_string(), texts))
        })
        .collect()
}

fn project_examples(
    function: &str,
    atom: &PropertyAtom,
    function_type: &FunctionType,
    raw: Option<&str>,
    example_files: &[(&str, &str)],
) -> Result<Vec<FunctionExample>, String> {
    let mut examples = Vec::new();
    for example in &function_type.examples {
        let authored = raw
            .and_then(|text| text.get(example.expected_span.clone()))
            .map(str::trim)
            .filter(|text| !text.is_empty() && !text.contains("=>"));
        let result = match (authored, &example.expected) {
            (Some(text), _) => text.to_string(),
            (None, ExampleOutcome::Error) => "error".to_string(),
            (None, ExampleOutcome::Value(value)) => value.to_string(),
        };
        examples.push(FunctionExample::Call {
            call: example.call.as_str().into(),
            expected: example.expected.clone(),
            result: result.into(),
            display_only: example.display_only.as_deref().map(Box::from),
        });
    }
    for constraint in &atom.constraints {
        let Constraint::Example(references) = constraint else {
            continue;
        };
        for reference in references {
            examples.push(file_example(function, reference, example_files)?);
        }
    }
    Ok(examples)
}

fn file_example(function: &str, reference: &str, example_files: &[(&str, &str)]) -> Result<FunctionExample, String> {
    let key = reference.trim().trim_start_matches("./");
    let (path, text) = example_files
        .iter()
        .find(|(path, _)| *path == key)
        .ok_or_else(|| format!("`types.{function}`: example file `{reference}` is not embedded"))?;
    let object = simplified_schema::example::validate_example_bytes(reference, text.as_bytes())
        .map_err(|error| format!("`types.{function}`: {error}"))?;
    let invocation = object
        .get("invocation")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`types.{function}`: example file `{reference}` needs a call `invocation`"))?;
    let returns = match object.get("returns") {
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    };
    Ok(FunctionExample::File {
        path: (*path).into(),
        invocation: invocation.into(),
        returns: returns.into(),
        description: object.get("description").and_then(Value::as_str).map(Box::from),
    })
}

/// The declared type as written, without descriptions.
fn type_text(def: &PropertyDef) -> String {
    let strip = |atom: &PropertyAtom| PropertyAtom {
        description: None,
        ..atom.clone()
    };
    let bare = match def {
        PropertyDef::Single(atom) => PropertyDef::Single(strip(atom)),
        PropertyDef::Union(arms) => PropertyDef::Union(arms.iter().map(strip).collect()),
    };
    serialize_property_def(&bare)
}

/// Generates the Markdown function-reference table for
/// `darkmatter-expressions.md`: `Category`, `Function`, `Description`, and
/// `Example` columns, one row per overload in documentation order. The example
/// cell lists every inline example (display-only ones marked with their
/// reason) and every file-form example.
pub fn generate_expression_function_table() -> String {
    let mut out = String::new();
    out.push_str("| Category | Function | Description | Example |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    for descriptor in expression_function_descriptors() {
        let examples: Vec<String> = descriptor
            .examples
            .iter()
            .map(|example| match example {
                FunctionExample::Call {
                    call,
                    result,
                    display_only: None,
                    ..
                } => format!("`{call}` ⇒ `{result}`"),
                FunctionExample::Call {
                    call,
                    result,
                    display_only: Some(reason),
                    ..
                } => format!("`{call}` ⇒ `{result}` (display only: {reason})"),
                FunctionExample::File {
                    invocation, returns, ..
                } => {
                    let returns = serde_json::to_string(returns).expect("a string serializes");
                    format!("`{invocation}` ⇒ `{returns}` (example file)")
                }
            })
            .collect();
        out.push_str(&format!(
            "| {} | `{}` | {} | {} |\n",
            descriptor.category,
            descriptor.signature,
            descriptor.description.replace('|', "\\|").replace('\n', " "),
            examples.join("<br>").replace('|', "\\|"),
        ));
    }
    out
}

/// A generated per-function reference page: which function it documents,
/// where the page lives, and the related pages its "See also" section links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressionFunctionPage {
    /// The function's canonical catalog name.
    pub function: &'static str,
    /// The page's path relative to the `darkmatter/` package-area directory.
    pub path: &'static str,
    /// `(title, link, summary)` entries, links relative to the page.
    pub see_also: &'static [(&'static str, &'static str, &'static str)],
}

/// Every function that has a generated reference page, written by the
/// `expression_doc_generator` example (`just regen-expr-doc`).
pub const EXPRESSION_FUNCTION_PAGES: &[ExpressionFunctionPage] = &[
    ExpressionFunctionPage {
        function: "load_data",
        path: "docs/functions/load-data.md",
        see_also: &[
            (
                "Loading data",
                "../topics/loading-data.md",
                "each format, the input rules for tabular cells, the optional-file pattern, and typing limits",
            ),
            (
                "Container types",
                "../topics/schemas/definition.md#container-types",
                "which `as` contracts describe a container",
            ),
            (
                "Data contracts",
                "../cli/data-contracts.md",
                "the `dm.data.*` diagnostic codes, `data_location`, and the read set",
            ),
        ],
    },
    ExpressionFunctionPage {
        function: "try",
        path: "docs/functions/try.md",
        see_also: &[
            (
                "Recovering from read failures",
                "../topics/darkmatter-expressions.md#recovering-from-read-failures",
                "which failures `try` recovers, which it propagates, and how it behaves in callbacks",
            ),
            (
                "Loading data",
                "../topics/loading-data.md",
                "the optional-file pattern, the `format` option, and the read set under recovery",
            ),
        ],
    },
];

/// Generates the Markdown reference page for the catalog function `name`
/// (canonical name or alias): category, aliases, and for each overload its
/// typed signature, description, parameters, return type, fallibility, and
/// every catalog example. A function listed in [`EXPRESSION_FUNCTION_PAGES`]
/// also gets that entry's "See also" links. `None` for an unknown name.
///
/// The page starts with a comment marking it as generated; the committed
/// copies are checked against this output.
pub fn generate_expression_function_page(name: &str) -> Option<String> {
    let overloads = function_descriptors(name);
    let first = overloads.first()?;
    let mut out = String::new();
    out.push_str(
        "<!-- Generated by `just regen-expr-doc` from darkmatter/schemas/partials/functions.yaml. \
         Do not edit by hand. -->\n\n",
    );
    out.push_str(&format!("# `{}`\n\n", first.name));
    out.push_str(&format!("- **Category:** {}\n", first.category));
    if !first.aliases.is_empty() {
        let aliases: Vec<String> = first.aliases.iter().map(|alias| format!("`{alias}`")).collect();
        out.push_str(&format!("- **Aliases:** {}\n", aliases.join(", ")));
    }
    if overloads.len() > 1 {
        out.push_str(&format!("- **Overloads:** {}\n", overloads.len()));
    }
    out.push('\n');

    let data_files = example_data_section(overloads);
    let single = overloads.len() == 1;
    if !single {
        out.push_str("## Overloads\n\n");
    }
    for descriptor in overloads {
        let sub = if single {
            out.push_str("## Signature\n\n");
            "##"
        } else {
            out.push_str(&format!("### `{}`\n\n", descriptor.signature));
            "####"
        };
        out.push_str(&format!("```text\n{}\n```\n\n", descriptor.typed_signature()));
        if !descriptor.description.is_empty() {
            out.push_str(&format!("{}\n\n", descriptor.description.trim()));
        }

        out.push_str(&format!("{sub} Parameters\n\n"));
        if descriptor.parameters.is_empty() {
            out.push_str("None.\n\n");
        } else {
            out.push_str("| Parameter | Type | Kind |\n| --- | --- | --- |\n");
            for parameter in &descriptor.parameters {
                let kind = if parameter.rest {
                    "rest"
                } else if parameter.optional {
                    "optional"
                } else {
                    "required"
                };
                out.push_str(&format!(
                    "| `{}` | `{}` | {kind} |\n",
                    parameter.name,
                    parameter.type_text.replace('|', "\\|"),
                ));
            }
            out.push('\n');
        }

        out.push_str(&format!("{sub} Returns\n\n"));
        out.push_str(&format!("`{}`\n\n", descriptor.returns));
        if descriptor.fallible {
            out.push_str("Fallible: a call can fail even when its arguments are valid.\n\n");
        } else {
            out.push_str("Not fallible: a call whose arguments are valid always returns a value.\n\n");
        }

        if single {
            out.push_str(&data_files);
        }
        out.push_str(&format!("{sub} Examples\n\n"));
        if descriptor.examples.is_empty() {
            out.push_str("None.\n\n");
        }
        for example in &descriptor.examples {
            match example {
                FunctionExample::Call {
                    call,
                    result,
                    display_only,
                    ..
                } => {
                    out.push_str(&format!("```text\n{call}\n⇒ {result}\n```\n\n"));
                    if let Some(reason) = display_only {
                        out.push_str(&format!("Display only: {reason}.\n\n"));
                    }
                }
                FunctionExample::File {
                    path,
                    invocation,
                    returns,
                    description,
                } => {
                    let returns = serde_json::to_string(returns).expect("a string serializes");
                    out.push_str(&format!("```text\n{invocation}\n⇒ {returns}\n```\n\n"));
                    match description {
                        Some(description) => {
                            out.push_str(&format!("From the example file `{path}`: {}\n\n", description.trim()));
                        }
                        None => out.push_str(&format!("From the example file `{path}`.\n\n")),
                    }
                }
            }
        }
    }

    if !single {
        out.push_str(&data_files);
    }

    let see_also = EXPRESSION_FUNCTION_PAGES
        .iter()
        .find(|page| page.function == &*first.name)
        .map_or(&[][..], |page| page.see_also);
    if !see_also.is_empty() {
        out.push_str("## See also\n\n");
        for (title, link, summary) in see_also {
            out.push_str(&format!("- [{title}]({link}): {summary}\n"));
        }
        out.push('\n');
    }
    Some(format!("{}\n", out.trim_end()))
}

/// The "Example files" section for the [`EXAMPLE_DATA_FILES`] that any of
/// `overloads`' inline examples names as a quoted string; empty when none does.
fn example_data_section(overloads: &[ExpressionFunctionDescriptor]) -> String {
    let named: Vec<(&str, &str)> = EXAMPLE_DATA_FILES
        .iter()
        .copied()
        .filter(|(file, _)| {
            let quoted = format!("\"{file}\"");
            overloads.iter().flat_map(|descriptor| descriptor.examples.iter()).any(|example| {
                matches!(example, FunctionExample::Call { call, .. } if call.contains(&quoted))
            })
        })
        .collect();
    if named.is_empty() {
        return String::new();
    }
    let mut out = String::from("## Example files\n\nThe examples read these files from the document's directory.\n\n");
    for (file, content) in named {
        let language = match file.rsplit_once('.').map(|(_, extension)| extension) {
            Some("yaml" | "yml") => "yaml",
            Some("json") => "json",
            Some("json5") => "json5",
            Some("toml") => "toml",
            Some("csv") => "csv",
            _ => "text",
        };
        out.push_str(&format!("`{file}`:\n\n```{language}\n{}\n```\n\n", content.trim_end()));
    }
    out
}

#[cfg(test)]
mod tests;
