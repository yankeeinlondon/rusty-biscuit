//! The `as` contract of `load_data`: preparation and the tabular row rule.
//!
//! A contract is resolved once per document context and written text. The
//! written type is parsed passively, checked for document-local references,
//! wrapped in a synthetic one-property schema so the shared resolver expands
//! its imports from the document's folder and schema roots, and translated to
//! the coercion engine's [`Shape`]. The bare shape is bound: no optional-null
//! wrapper is added, so `string[]` rejects `null` and `string[] | null`
//! admits it.

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use simplified_schema::SchemaError;
use simplified_schema::coercion::Shape;
use simplified_schema::simplified::narrowing::{ContainerAdmission, container_admission, is_null_type};
use simplified_schema::simplified::{
    DefKey, DefinitionTable, PropertyAtom, PropertyDef, SchemaRoot, SchemaShape, SimplifiedType, TypeExpr,
    flattened_alternatives, parse_type_definition_value, serialize_property_atom, serialize_property_def,
    to_json_schema_with_definitions,
};

use super::error::DataError;
use crate::markdown::compose::expression::call_binding::{NoNamedTypes, shape_from_property_def_with_definitions};
use crate::markdown::compose::expression::FileReferenceDiagnostic;

/// The property name of the synthetic schema a contract is resolved in. It
/// appears in schema error messages as the parameter the author wrote.
const CONTRACT_PROPERTY: &str = "as";

/// The suggestion a row-shape diagnostic offers.
pub const ROW_SHAPE_SUGGESTION: &str = "use `{ <string>: string }[]` for rows read by header, or `tuple([…])[]` for rows read by position";

/// A resolved `load_data` contract.
#[derive(Debug, Clone)]
pub struct PreparedContract {
    /// The contract as written.
    pub written: String,
    /// The definition with every named-type reference expanded.
    pub property: PropertyDef,
    /// The stored definitions the expanded property's references point into.
    pub definitions: Arc<DefinitionTable>,
    /// The canonical schema files the contract imported, sorted.
    pub imports: Vec<PathBuf>,
    /// The bare shape the data is bound to.
    pub shape: Shape,
    /// The bind-memo key: the bare declaration, the lowered synthetic schema
    /// with its reachable definitions, and the sorted import identities. Two
    /// contracts with the same key accept and convert every value alike, even
    /// when their written text or folder differs.
    pub key: String,
}

impl PreparedContract {
    /// The row shape this contract selects for CSV or TSV data.
    ///
    /// ## Errors
    ///
    /// [`RowShapeConflict`] when the contract's arms do not agree on one row
    /// shape, or an element cannot describe a row.
    pub fn row_mode(&self) -> Result<RowMode, RowShapeConflict> {
        match classify_rows(&self.property, &self.definitions)? {
            RowClassification::Known(mode) => Ok(mode),
            // An expanded contract holds no unresolved import.
            RowClassification::Provisional => Err(RowShapeConflict::NoRowMode),
        }
    }
}

/// How a CSV or TSV row becomes a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowMode {
    /// Each row is an object keyed by the header.
    Object,
    /// Each row is a list of cells; the header row is skipped.
    List,
}

/// What the passive row rule can say about a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowClassification {
    /// Every row-describing element agrees on this mode.
    Known(RowMode),
    /// No element disagrees, but an unresolved named-type import decides the
    /// mode; the rule is applied again once the contract is resolved.
    Provisional,
}

/// Why a contract cannot describe CSV or TSV rows.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RowShapeConflict {
    /// One arm reads rows as objects and another as lists.
    #[error("`{object}` reads rows as objects but `{list}` reads them as lists")]
    MixedRows {
        /// The first arm with object rows, as written after expansion.
        object: String,
        /// The first arm with list rows.
        list: String,
    },
    /// A collection element is a scalar, `unknown`, nullable, or otherwise not
    /// an object or a list.
    #[error("`{element}` does not describe a row; {ROW_SHAPE_SUGGESTION}")]
    NotARow {
        /// The element type.
        element: String,
    },
    /// A top-level arm is a single object, dictionary, or scalar rather than a
    /// collection of rows.
    #[error("`{arm}` is not a collection of rows; {ROW_SHAPE_SUGGESTION}")]
    NotTabular {
        /// The arm.
        arm: String,
    },
    /// No arm selects a row mode: an empty tuple, or only `null`.
    #[error("the contract selects no row shape; {ROW_SHAPE_SUGGESTION}")]
    NoRowMode,
    /// `header: false` reads rows by position, but the contract's rows are
    /// objects, which need header names.
    #[error(
        "`header: false` reads rows by position, but this contract describes rows as objects; use a `tuple([…])[]` or `string[][]` contract, or read an input that has a header"
    )]
    HeaderlessObjectRows,
}

/// The rule for `header: false`: rows are read by position, so a contract
/// whose rows are known to be objects cannot apply. A provisional
/// classification (an unresolved import decides the mode) stays open until
/// the contract is resolved at execution.
///
/// ## Errors
///
/// [`RowShapeConflict::HeaderlessObjectRows`] for object rows.
pub fn require_positional_rows(class: RowClassification) -> Result<(), RowShapeConflict> {
    match class {
        RowClassification::Known(RowMode::Object) => Err(RowShapeConflict::HeaderlessObjectRows),
        RowClassification::Known(RowMode::List) | RowClassification::Provisional => Ok(()),
    }
}

/// Resolves `written` as a contract for a document whose context is
/// `document` (its `cwd` is the document's folder).
///
/// ## Errors
///
/// - [`DataError::ContractSchema`] for a type that does not parse, an
///   imported type that is not exported, or a type the coercion engine cannot
///   bind.
/// - [`DataError::ThisImportRejected`] for a document-local named type at any
///   depth; an imported type may still recurse internally.
/// - [`DataError::NotAContainer`] for a scalar, `unknown`, or `null` contract,
///   judged before any import is read when the text alone proves it.
/// - [`DataError::FileReference`] for an import that is missing or unreadable.
pub fn prepare_contract(written: &str, document: &FileResolutionContext) -> Result<PreparedContract, Box<DataError>> {
    let schema_error = |source: SchemaError| {
        Box::new(DataError::ContractSchema { written: written.to_string(), source: Arc::new(source) })
    };
    let parsed = judge_written_contract(written).map_err(Box::new)?;

    let mut shape = SchemaShape::new();
    shape.properties.insert(CONTRACT_PROPERTY.to_string(), parsed);
    let root = SchemaRoot::Single(shape);
    let roots = crate::markdown::schemas::roots::SchemaRoots::for_document(document).map_err(schema_error)?;
    let mut expansion = crate::markdown::schemas::resolve::expand_named_types(
        &root,
        document.cwd(),
        None,
        roots.search_paths(),
        document,
    )
    .map_err(|error| Box::new(import_failure(written, document, error)))?;
    // A contract naming a template resolves its slots only inside a slot's
    // evaluation, through that slot's session; elsewhere lowering below
    // fails with `UnresolvedSlots`.
    if simplified_schema::slots::has_dynamic(&expansion.schema, &expansion.definitions) {
        let slots = simplified_schema::slots::collect_slots(&expansion.schema, &expansion.definitions);
        match crate::markdown::schemas::slots::session::resolve_contract_slots(
            &mut expansion.schema,
            Arc::make_mut(&mut expansion.definitions),
        ) {
            Some(resolved) => resolved.map_err(schema_error)?,
            None => {
                return Err(schema_error(SchemaError::UnresolvedSlots(Box::new(
                    simplified_schema::UnresolvedSlots::new(slots),
                ))));
            }
        }
    }
    let property = match &expansion.schema {
        SchemaRoot::Single(shape) => shape.properties.get(CONTRACT_PROPERTY).cloned(),
        SchemaRoot::Union(_) => None,
    }
    .ok_or_else(|| {
        schema_error(SchemaError::Convert {
            property: CONTRACT_PROPERTY.to_string(),
            message: "the expanded contract lost its property".to_string(),
        })
    })?;
    if !contract_is_container(&property, &expansion.definitions, &mut Vec::new()) {
        return Err(Box::new(DataError::NotAContainer { written: written.to_string() }));
    }
    let bare = shape_from_property_def_with_definitions(&property, &NoNamedTypes, &expansion.definitions).map_err(
        |error| {
            schema_error(SchemaError::Convert { property: CONTRACT_PROPERTY.to_string(), message: error.to_string() })
        },
    )?;
    let lowered = to_json_schema_with_definitions(&expansion.schema, &expansion.definitions).map_err(schema_error)?;
    let key = contract_key(&property, &lowered, &expansion.imports);
    Ok(PreparedContract {
        written: written.to_string(),
        property,
        definitions: expansion.definitions,
        imports: expansion.imports,
        shape: bare,
        key,
    })
}

/// The passive verdict on a written contract, judged on its text alone with
/// no I/O: the parsed definition, or why the text cannot be a contract.
///
/// The passive call check, the binder's refinement of a failed `as`
/// argument, and [`prepare_contract`] all start here, so a literal and a
/// computed contract fail with the same error. A named import is not
/// expanded, so a contract it decides is admitted provisionally.
///
/// ## Errors
///
/// - [`DataError::ContractSchema`] for text that is not a type definition.
/// - [`DataError::ThisImportRejected`] for a document-local named type at
///   any depth.
/// - [`DataError::NotAContainer`] for a scalar, `unknown`, or `null`-only
///   contract.
pub fn judge_written_contract(written: &str) -> Result<PropertyDef, DataError> {
    let parsed = parse_type_definition_value(CONTRACT_PROPERTY, &serde_yaml_ng::Value::String(written.to_string()))
        .map_err(|source| DataError::ContractSchema { written: written.to_string(), source: Arc::new(source) })?;
    if let Some(name) = local_reference(&parsed) {
        return Err(DataError::ThisImportRejected { written: written.to_string(), name: name.to_string() });
    }
    if container_admission(&parsed) == ContainerAdmission::NotContainer {
        return Err(DataError::NotAContainer { written: written.to_string() });
    }
    Ok(parsed)
}

/// The contract text an `as` value carries: a string as written, or the
/// canonical text of a list or mapping that parses as a type definition (a
/// list is the union of its items), so every carrier reaches
/// [`judge_written_contract`] and [`prepare_contract`] the same way. `None`
/// for any other value or a list or mapping that is not a definition; the
/// binder has already rejected those.
pub fn written_contract(carrier: &serde_json::Value) -> Option<Cow<'_, str>> {
    match carrier {
        serde_json::Value::String(text) => Some(Cow::Borrowed(text)),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            let yaml = serde_yaml_ng::to_value(carrier).ok()?;
            let parsed = parse_type_definition_value(CONTRACT_PROPERTY, &yaml).ok()?;
            Some(Cow::Owned(serialize_property_def(&parsed)))
        }
        _ => None,
    }
}

/// The exact serialized identity of a resolved contract. The lowered schema
/// keeps union and property order, so it is serialized as built, without
/// key sorting; NUL cannot occur in any part, so the parts cannot run
/// together.
fn contract_key(property: &PropertyDef, lowered: &serde_json::Value, imports: &[PathBuf]) -> String {
    let mut key = serialize_property_def(property);
    key.push('\0');
    key.push_str(&lowered.to_string());
    for import in imports {
        key.push('\0');
        key.push_str(&import.to_string_lossy());
    }
    key
}

/// Keeps a missing or unreadable import in the file-reference family, naming
/// the import reference; every other expansion failure is a schema failure.
fn import_failure(written: &str, document: &FileResolutionContext, error: SchemaError) -> DataError {
    match FileReferenceDiagnostic::from_schema_read("load_data", document.cwd().to_path_buf(), error) {
        Ok(diagnostic) => DataError::FileReference(diagnostic),
        Err(other) => DataError::ContractSchema { written: written.to_string(), source: Arc::new(other) },
    }
}

/// The first document-local named type (`Name` or `Name@this`) in `def`, at
/// any depth.
fn local_reference(def: &PropertyDef) -> Option<&str> {
    match def {
        PropertyDef::Single(atom) => local_reference_in(atom),
        PropertyDef::Union(atoms) => atoms.iter().find_map(local_reference_in),
    }
}

fn local_reference_in(atom: &PropertyAtom) -> Option<&str> {
    match &atom.ty {
        TypeExpr::Imported { name, .. } if atom.ty.is_local_reference() => Some(name),
        TypeExpr::Imported { .. } | TypeExpr::Primitive(_) | TypeExpr::Ref(_) => None,
        TypeExpr::InlineObject(shape) => shape
            .properties
            .values()
            .chain(shape.pattern_keys.iter().map(|pattern| &pattern.def))
            .find_map(local_reference),
        TypeExpr::Union(arms) => arms.iter().find_map(local_reference_in),
        TypeExpr::Tuple(tuple) => tuple
            .elements
            .iter()
            .find_map(|element| local_reference(&element.def))
            .or_else(|| tuple.spread.as_deref().and_then(local_reference_in)),
        TypeExpr::Function(function) => function
            .parameters
            .as_slice()
            .iter()
            .find_map(|parameter| local_reference(&parameter.def))
            .or_else(|| local_reference(&function.returns)),
    }
}

/// The shared container rule over a resolved contract: a stored definition
/// is judged by its body, since the text-only rule cannot see through it.
fn contract_is_container(def: &PropertyDef, definitions: &DefinitionTable, visiting: &mut Vec<DefKey>) -> bool {
    match container_admission(def) {
        ContainerAdmission::Container => true,
        ContainerAdmission::NotContainer => false,
        ContainerAdmission::Provisional => {
            let mut any = false;
            for alternative in flattened_alternatives(def) {
                if alternative.is_array {
                    any = true;
                    continue;
                }
                if is_null_type(alternative.atom) {
                    continue;
                }
                let container = match &alternative.atom.ty {
                    TypeExpr::Ref(key) if !visiting.contains(key) => {
                        visiting.push(key.clone());
                        let body = definitions.get(key).map(|definition| &definition.body);
                        let container = body.is_some_and(|body| contract_is_container(body, definitions, visiting));
                        visiting.pop();
                        container
                    }
                    TypeExpr::Ref(_) | TypeExpr::Imported { .. } => false,
                    _ => container_admission(&PropertyDef::Single(alternative.atom.clone()))
                        == ContainerAdmission::Container,
                };
                if !container {
                    return false;
                }
                any = true;
            }
            any
        }
    }
}

/// Selects the CSV/TSV row mode `def` describes.
///
/// Every non-null top-level arm must be a collection: an array's element, or
/// each position of a tuple (its spread's element included), is a row. A row
/// that is an object or dictionary selects [`RowMode::Object`]; a tuple or
/// array selects [`RowMode::List`]. Union elements are walked arm by arm, and
/// every row-describing element must agree. A bare `object` is a row when it
/// is a collection's element, but not a top-level tabular contract.
///
/// ## Errors
///
/// A [`RowShapeConflict`] for disagreeing arms, an element that is a scalar,
/// `unknown`, or nullable, a top-level object or scalar, or a contract that
/// selects no mode (an empty tuple, or only `null`). A conflict is reported
/// even when an unresolved import could not have resolved it.
pub fn classify_rows(def: &PropertyDef, definitions: &DefinitionTable) -> Result<RowClassification, RowShapeConflict> {
    let mut rows = RowTally::default();
    let atoms: &[PropertyAtom] = match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms,
    };
    for atom in atoms {
        top_level_arm(atom, definitions, &mut rows, &mut Vec::new())?;
    }
    rows.finish()
}

#[derive(Default)]
struct RowTally {
    object: Option<String>,
    list: Option<String>,
    provisional: bool,
}

impl RowTally {
    fn note(&mut self, mode: RowMode, arm: &PropertyAtom) {
        let slot = match mode {
            RowMode::Object => &mut self.object,
            RowMode::List => &mut self.list,
        };
        slot.get_or_insert_with(|| serialize_property_atom(arm));
    }

    fn finish(self) -> Result<RowClassification, RowShapeConflict> {
        match (self.object, self.list) {
            (Some(object), Some(list)) => Err(RowShapeConflict::MixedRows { object, list }),
            _ if self.provisional => Ok(RowClassification::Provisional),
            (Some(_), None) => Ok(RowClassification::Known(RowMode::Object)),
            (None, Some(_)) => Ok(RowClassification::Known(RowMode::List)),
            (None, None) => Err(RowShapeConflict::NoRowMode),
        }
    }
}

/// What one row element describes.
enum RowElement {
    Mode(RowMode),
    Provisional,
}

fn top_level_arm(
    atom: &PropertyAtom,
    definitions: &DefinitionTable,
    rows: &mut RowTally,
    visiting: &mut Vec<DefKey>,
) -> Result<(), RowShapeConflict> {
    if is_null_type(atom) {
        return Ok(());
    }
    if atom.is_array {
        let element = PropertyAtom { is_array: false, array_constraints: Vec::new(), ..atom.clone() };
        return tally(row_element(&element, definitions, visiting)?, atom, rows);
    }
    match &atom.ty {
        TypeExpr::Tuple(tuple) => {
            if tuple.elements.is_empty() && tuple.spread.is_none() {
                return Err(RowShapeConflict::NoRowMode);
            }
            for element in &tuple.elements {
                for row in def_atoms(&element.def) {
                    tally(row_element(row, definitions, visiting)?, atom, rows)?;
                }
            }
            if let Some(spread) = tuple.spread.as_deref() {
                let element = PropertyAtom { is_array: false, array_constraints: Vec::new(), ..spread.clone() };
                tally(row_element(&element, definitions, visiting)?, atom, rows)?;
            }
            Ok(())
        }
        TypeExpr::Union(arms) => arms.iter().try_for_each(|arm| top_level_arm(arm, definitions, rows, visiting)),
        TypeExpr::Ref(key) if !visiting.contains(key) => {
            let Some(definition) = definitions.get(key) else {
                return Err(RowShapeConflict::NotTabular { arm: serialize_property_atom(atom) });
            };
            visiting.push(key.clone());
            let result = def_atoms(&definition.body)
                .iter()
                .try_for_each(|arm| top_level_arm(arm, definitions, rows, visiting));
            visiting.pop();
            result
        }
        TypeExpr::Imported { .. } => {
            rows.provisional = true;
            Ok(())
        }
        _ => Err(RowShapeConflict::NotTabular { arm: serialize_property_atom(atom) }),
    }
}

fn tally(element: RowElement, arm: &PropertyAtom, rows: &mut RowTally) -> Result<(), RowShapeConflict> {
    match element {
        RowElement::Mode(mode) => rows.note(mode, arm),
        RowElement::Provisional => rows.provisional = true,
    }
    Ok(())
}

/// What a single row element type describes; a union element must agree
/// with itself arm by arm.
fn row_element(
    atom: &PropertyAtom,
    definitions: &DefinitionTable,
    visiting: &mut Vec<DefKey>,
) -> Result<RowElement, RowShapeConflict> {
    let not_a_row = || RowShapeConflict::NotARow { element: serialize_property_atom(atom) };
    if atom.is_array {
        return Ok(RowElement::Mode(RowMode::List));
    }
    match &atom.ty {
        TypeExpr::InlineObject(_) | TypeExpr::Primitive(SimplifiedType::Object) => Ok(RowElement::Mode(RowMode::Object)),
        TypeExpr::Tuple(_) => Ok(RowElement::Mode(RowMode::List)),
        TypeExpr::Imported { .. } => Ok(RowElement::Provisional),
        TypeExpr::Union(arms) => agree(arms, definitions, visiting).map_err(|conflict| match conflict {
            RowShapeConflict::NotARow { .. } => not_a_row(),
            other => other,
        }),
        TypeExpr::Ref(key) if !visiting.contains(key) => {
            let definition = definitions.get(key).ok_or_else(not_a_row)?;
            visiting.push(key.clone());
            let result = agree(def_atoms(&definition.body), definitions, visiting);
            visiting.pop();
            result
        }
        _ => Err(not_a_row()),
    }
}

/// The one mode every arm of a union row element describes.
fn agree(
    arms: &[PropertyAtom],
    definitions: &DefinitionTable,
    visiting: &mut Vec<DefKey>,
) -> Result<RowElement, RowShapeConflict> {
    let mut seen: Option<(RowMode, &PropertyAtom)> = None;
    let mut provisional = false;
    for arm in arms {
        match row_element(arm, definitions, visiting)? {
            RowElement::Provisional => provisional = true,
            RowElement::Mode(mode) => match seen {
                Some((earlier, earlier_arm)) if earlier != mode => {
                    let (object, list) = match earlier {
                        RowMode::Object => (earlier_arm, arm),
                        RowMode::List => (arm, earlier_arm),
                    };
                    return Err(RowShapeConflict::MixedRows {
                        object: serialize_property_atom(object),
                        list: serialize_property_atom(list),
                    });
                }
                Some(_) => {}
                None => seen = Some((mode, arm)),
            },
        }
    }
    match (seen, provisional) {
        (_, true) => Ok(RowElement::Provisional),
        (Some((mode, _)), false) => Ok(RowElement::Mode(mode)),
        (None, false) => Err(RowShapeConflict::NoRowMode),
    }
}

fn def_atoms(def: &PropertyDef) -> &[PropertyAtom] {
    match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms,
    }
}

/// A passively judged contract whose every import names a shipped type.
#[derive(Debug, Clone)]
pub struct EmbeddedContract {
    /// The definition with its `Name@darkmatter:<stem>` imports expanded.
    pub property: PropertyDef,
    /// The stored definitions the expanded property's references point into.
    pub definitions: Arc<DefinitionTable>,
}

/// Expands `parsed` (a contract [`judge_written_contract`] accepted) when
/// every named-type import it holds is a shipped `Name@darkmatter:<stem>`
/// type, from the embedded sources alone, and applies the container rule to
/// the result. No file is opened.
///
/// `None` when the contract holds no import, or holds one that names a file:
/// a file import is judged only when the call executes.
///
/// ## Errors
///
/// - [`DataError::NotAContainer`] when the expanded contract is not a
///   container.
/// - [`DataError::ContractSchema`] when a shipped stem or exported type does
///   not exist.
pub fn expand_embedded_contract(
    parsed: &PropertyDef,
    written: &str,
) -> Option<Result<EmbeddedContract, Box<DataError>>> {
    let mut references = Vec::new();
    collect_imports(parsed, &mut references);
    let shipped = crate::markdown::schemas::embedded::SHIPPED_SOURCES;
    if references.is_empty() || references.iter().any(|reference| shipped.reserved_stem(reference).is_none()) {
        return None;
    }
    let schema_error = |source: SchemaError| {
        Box::new(DataError::ContractSchema { written: written.to_string(), source: Arc::new(source) })
    };
    let mut mapping = serde_yaml_ng::Mapping::new();
    mapping.insert(CONTRACT_PROPERTY.into(), written.into());
    // An empty document list: only the shipped namespace resolves, so a
    // reference this check missed fails instead of probing the disk.
    let expanded = crate::markdown::schemas::resolve::prepare_embedded_inline_schema(
        &serde_yaml_ng::Value::Mapping(mapping),
        "load_data",
        &[],
        "",
    );
    Some(expanded.map_err(schema_error).and_then(|(resolved, _)| {
        let property = match &resolved.simplified {
            Some(SchemaRoot::Single(shape)) => shape.properties.get(CONTRACT_PROPERTY).cloned(),
            _ => None,
        }
        .ok_or_else(|| {
            schema_error(SchemaError::Convert {
                property: CONTRACT_PROPERTY.to_string(),
                message: "the expanded contract lost its property".to_string(),
            })
        })?;
        if !contract_is_container(&property, &resolved.definitions, &mut Vec::new()) {
            return Err(Box::new(DataError::NotAContainer { written: written.to_string() }));
        }
        Ok(EmbeddedContract { property, definitions: resolved.definitions })
    }))
}

/// Every unresolved import reference (right of `@`) in `def`, at any depth.
fn collect_imports<'d>(def: &'d PropertyDef, found: &mut Vec<&'d str>) {
    for atom in def_atoms(def) {
        collect_imports_in(atom, found);
    }
}

fn collect_imports_in<'d>(atom: &'d PropertyAtom, found: &mut Vec<&'d str>) {
    match &atom.ty {
        TypeExpr::Imported { reference, .. } => found.push(reference),
        TypeExpr::Primitive(_) | TypeExpr::Ref(_) => {}
        TypeExpr::InlineObject(shape) => {
            for def in shape.properties.values().chain(shape.pattern_keys.iter().map(|pattern| &pattern.def)) {
                collect_imports(def, found);
            }
        }
        TypeExpr::Union(arms) => arms.iter().for_each(|arm| collect_imports_in(arm, found)),
        TypeExpr::Tuple(tuple) => {
            tuple.elements.iter().for_each(|element| collect_imports(&element.def, found));
            if let Some(spread) = tuple.spread.as_deref() {
                collect_imports_in(spread, found);
            }
        }
        TypeExpr::Function(function) => {
            function.parameters.as_slice().iter().for_each(|parameter| collect_imports(&parameter.def, found));
            collect_imports(&function.returns, found);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> PropertyDef {
        parse_type_definition_value("as", &serde_yaml_ng::Value::String(text.to_string()))
            .unwrap_or_else(|error| panic!("{text}: {error}"))
    }

    fn rows(text: &str) -> Result<RowClassification, RowShapeConflict> {
        classify_rows(&parsed(text), &DefinitionTable::default())
    }

    fn document(dir: &std::path::Path) -> FileResolutionContext {
        FileResolutionContext::new(dir)
    }

    #[test]
    fn an_array_of_objects_or_dictionaries_reads_object_rows() {
        for text in ["{ name: string, port: number }[]", "{ <string>: string }[]", "object[]"] {
            assert_eq!(rows(text), Ok(RowClassification::Known(RowMode::Object)), "{text}");
        }
    }

    #[test]
    fn an_array_of_tuples_or_arrays_reads_list_rows() {
        for text in ["tuple([string, number])[]", "(string[] | number[])[]"] {
            assert_eq!(rows(text), Ok(RowClassification::Known(RowMode::List)), "{text}");
        }
    }

    #[test]
    fn a_top_level_tuple_reads_each_position_as_a_row() {
        assert_eq!(rows("tuple([{a: string}, {b: string}])"), Ok(RowClassification::Known(RowMode::Object)));
        assert_eq!(
            rows("tuple([{a: string}, ...{b: string}[]])"),
            Ok(RowClassification::Known(RowMode::Object))
        );
        assert!(matches!(rows("tuple([{a: string}, tuple([string])])"), Err(RowShapeConflict::MixedRows { .. })));
    }

    #[test]
    fn an_empty_tuple_selects_no_row_mode() {
        assert_eq!(rows("tuple([])"), Err(RowShapeConflict::NoRowMode));
    }

    #[test]
    fn disagreeing_arms_name_both() {
        let Err(RowShapeConflict::MixedRows { object, list }) = rows("{a: string}[] | tuple([string])[]") else {
            panic!("expected a conflict");
        };
        assert!(object.contains("a: string"), "{object}");
        assert!(list.contains("tuple"), "{list}");
    }

    #[test]
    fn a_union_element_must_agree_with_itself() {
        assert_eq!(rows("({a: string} | {b: number})[]"), Ok(RowClassification::Known(RowMode::Object)));
        assert!(matches!(rows("({a: string} | tuple([string]))[]"), Err(RowShapeConflict::MixedRows { .. })));
    }

    #[test]
    fn scalar_unknown_and_nullable_elements_are_not_rows() {
        for text in ["string[]", "unknown[]", "({a: string} | null)[]", "number[]"] {
            assert!(matches!(rows(text), Err(RowShapeConflict::NotARow { .. })), "{text}: {:?}", rows(text));
        }
        let Err(conflict) = rows("string[]") else { unreachable!() };
        assert!(conflict.to_string().contains(ROW_SHAPE_SUGGESTION), "{conflict}");
    }

    #[test]
    fn a_top_level_object_or_dictionary_is_not_tabular() {
        for text in ["object", "{ <string>: string }", "{a: string}"] {
            assert!(matches!(rows(text), Err(RowShapeConflict::NotTabular { .. })), "{text}: {:?}", rows(text));
        }
    }

    #[test]
    fn null_top_level_arms_select_no_mode() {
        assert_eq!(rows("{a: string}[] | null"), Ok(RowClassification::Known(RowMode::Object)));
        assert_eq!(rows("null"), Err(RowShapeConflict::NoRowMode));
    }

    #[test]
    fn an_unresolved_import_is_provisional_unless_another_arm_conflicts() {
        assert_eq!(rows("Hosts@./hosts.yaml"), Ok(RowClassification::Provisional));
        assert_eq!(rows("({a: string} | Host@./hosts.yaml)[]"), Ok(RowClassification::Provisional));
        assert_eq!(rows("{a: string}[] | Hosts@./hosts.yaml"), Ok(RowClassification::Provisional));
        assert!(matches!(
            rows("{a: string}[] | tuple([string])[] | Hosts@./hosts.yaml"),
            Err(RowShapeConflict::MixedRows { .. })
        ));
        assert!(matches!(rows("string[] | Hosts@./hosts.yaml"), Err(RowShapeConflict::NotARow { .. })));
    }

    #[test]
    fn preparation_rejects_a_local_reference_at_any_depth() {
        let dir = tempfile::tempdir().unwrap();
        for text in ["Host[]", "Host@this", "{ hosts: Host[] }", "tuple([string, Host])[]", "{ h: Host@this }[]"] {
            let error = prepare_contract(text, &document(dir.path())).unwrap_err();
            assert!(matches!(*error, DataError::ThisImportRejected { ref name, .. } if name == "Host"), "{text}: {error}");
        }
    }

    #[test]
    fn preparation_rejects_a_scalar_before_reading_anything() {
        let dir = tempfile::tempdir().unwrap();
        for text in ["string", "unknown", "null", "number | null"] {
            let error = prepare_contract(text, &document(dir.path())).unwrap_err();
            assert!(matches!(*error, DataError::NotAContainer { .. }), "{text}: {error}");
        }
    }

    #[test]
    fn preparation_reports_a_malformed_type_as_a_schema_failure() {
        let dir = tempfile::tempdir().unwrap();
        let error = prepare_contract("{ name: strng }[", &document(dir.path())).unwrap_err();
        assert!(matches!(*error, DataError::ContractSchema { .. }), "{error}");
    }

    #[test]
    fn preparation_binds_the_bare_shape() {
        use simplified_schema::coercion::{BindOptions, bind_with};
        let dir = tempfile::tempdir().unwrap();
        let contract = prepare_contract("{ name: string, port: number }[]", &document(dir.path())).unwrap();
        let value = serde_json::json!([{ "name": "a", "port": "8080" }]);
        let bound = bind_with(&contract.shape, &value, &BindOptions::default()).expect("binds").value().clone();
        assert_eq!(bound, serde_json::json!([{ "name": "a", "port": 8080 }]));
        assert!(bind_with(&contract.shape, &serde_json::Value::Null, &BindOptions::default()).is_err());
        let nullable = prepare_contract("string[] | null", &document(dir.path())).unwrap();
        assert!(bind_with(&nullable.shape, &serde_json::Value::Null, &BindOptions::default()).is_ok());
        assert_eq!(contract.row_mode(), Ok(RowMode::Object));
    }

    #[test]
    fn the_key_distinguishes_presence_and_ignores_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = document(dir.path());
        let a = prepare_contract("string[]", &ctx).unwrap();
        let spaced = prepare_contract("string []", &ctx);
        let nullable = prepare_contract("string[] | null", &ctx).unwrap();
        let constrained = prepare_contract("string[](min(1))", &ctx).unwrap();
        assert_ne!(a.key, nullable.key);
        assert_ne!(a.key, constrained.key);
        assert_eq!(a.key, spaced.unwrap_or_else(|error| panic!("`string []` prepares: {error}")).key);
    }

    #[test]
    fn an_import_resolves_from_the_document_folder_and_keys_by_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("one");
        let second = dir.path().join("two");
        for (folder, ty) in [(&first, "string"), (&second, "number")] {
            std::fs::create_dir_all(folder.join("schemas")).unwrap();
            std::fs::write(folder.join("schemas/types.yaml"), format!("kind: schema\ntypes:\n  Port: {ty}\n")).unwrap();
        }
        let a = prepare_contract("{ port: Port@./schemas/types.yaml }[]", &document(&first)).unwrap();
        let b = prepare_contract("{ port: Port@./schemas/types.yaml }[]", &document(&second)).unwrap();
        assert_eq!(a.imports.len(), 1);
        assert_ne!(a.key, b.key, "the same text in two folders names two types");
    }

    #[test]
    fn headerless_rows_refuse_known_object_rows_only() {
        assert_eq!(
            require_positional_rows(RowClassification::Known(RowMode::Object)),
            Err(RowShapeConflict::HeaderlessObjectRows)
        );
        assert_eq!(require_positional_rows(RowClassification::Known(RowMode::List)), Ok(()));
        assert_eq!(require_positional_rows(RowClassification::Provisional), Ok(()));
        let message = RowShapeConflict::HeaderlessObjectRows.to_string();
        assert!(message.contains("`header: false`"), "{message}");
        assert!(message.contains("tuple("), "{message}");
    }

    #[test]
    fn a_missing_import_stays_a_file_reference_failure() {
        let dir = tempfile::tempdir().unwrap();
        let error = prepare_contract("{ port: Port@./absent.yaml }[]", &document(dir.path())).unwrap_err();
        assert!(matches!(*error, DataError::FileReference(_)), "{error:?}");
    }
}
