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
use serde_json::Value;

use crate::catalog::{Described, Example, ExampleVerification};
use crate::markdown::compose::expression::call_binding::TypeResolver;
use crate::markdown::schemas::simplified::{
    Constraint, ExampleOutcome, FunctionType, LOCAL_REFERENCE, PropertyAtom, PropertyDef, SimplifiedSchema,
    StandaloneSchemaEnvelope, TypeExpr, parse_standalone_schema_document, serialize_property_def,
};

mod roots;

pub use roots::{ReservedRootDescriptor, RootEvaluation, RootMembers, reserved_root_descriptors};

/// The embedded catalog file's repository path, used in load errors.
const CATALOG_PATH: &str = "darkmatter/schemas/partials/functions.yaml";

const CATALOG_SOURCE: &str = include_str!("../../../../../../schemas/partials/functions.yaml");

/// The file-form example artifacts a catalog entry may reference with
/// `example(./function-examples/<name>.yaml)`, keyed by that reference
/// without its leading `./`. Compiled in so reading the catalog never touches
/// the disk; an entry naming a file missing here is a load error.
const EXAMPLE_FILES: &[(&str, &str)] = &[
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

/// One entry of the catalog's `categories:` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogCategory {
    pub name: Box<str>,
    pub description: Option<Box<str>>,
}

/// One parameter of a function overload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParameterDescriptor {
    pub name: Box<str>,
    /// The declared type in canonical grammar spelling, without descriptions
    /// (`number | null`, `number(integer; min(1))`).
    pub type_text: Box<str>,
    pub optional: bool,
    pub rest: bool,
}

/// An example attached to a function overload.
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionExample {
    /// `example(CALL => RESULT)`; `result` is the authored literal text or
    /// `error`.
    Call {
        call: Box<str>,
        expected: ExampleOutcome,
        result: Box<str>,
        display_only: Option<Box<str>>,
    },
    /// `example(./function-examples/x.yaml)`, read from the embedded file.
    /// `returns` is the file's `returns` text, or its JSON spelling when it is
    /// not a string.
    File {
        path: Box<str>,
        invocation: Box<str>,
        returns: Box<str>,
        description: Option<Box<str>>,
    },
}

/// Descriptor for one overload of an expression function.
///
/// Owns every field. A descriptor borrowed from a static catalog (the
/// built-in or mutation catalog) yields `&'static` field views; one borrowed
/// from a host catalog lives as long as that catalog.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug)]
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
}

/// The embedded catalog, loaded once.
///
/// ## Panics
///
/// When the embedded file does not load; a unit test loads it.
pub fn expression_function_catalog() -> &'static FunctionCatalog {
    static CATALOG: LazyLock<FunctionCatalog> = LazyLock::new(|| {
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
    Schema(#[from] crate::markdown::schemas::SchemaError),
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
    let invalid = |message: String| CatalogError::Invalid {
        path: path.to_string(),
        message,
    };
    let document = parse_standalone_schema_document(source, path)?
        .ok_or_else(|| invalid("not a SimplifiedSchema document; expected `kind: schema`".into()))?;
    if document.envelope != StandaloneSchemaEnvelope::Tagged {
        return Err(invalid("expected the tagged envelope (`kind: schema`, `categories:`, `types:`)".into()));
    }
    let Some(categories) = document.categories.as_ref() else {
        return Err(invalid("missing the `categories:` list".into()));
    };
    let Some(SimplifiedSchema::Single(types)) = document.schema() else {
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
    let object = crate::markdown::schemas::example::validate_example_bytes(reference, text.as_bytes())
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

#[cfg(test)]
mod tests;
