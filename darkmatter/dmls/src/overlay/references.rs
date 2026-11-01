//! The file each `$schema` reference of a Markdown document, or of a YAML
//! document on its root-`$schema` route, selects.
//!
//! Validation, document links, and go-to-definition must agree on the file a
//! bare-name `$schema` names. The overlay therefore selects it once, beside
//! schema assembly and from the same inputs (the text the schema was
//! assembled from, the registry's schema roots, the document's folder, and its
//! context), through `simplified_schema::resolve::select_schema_file`, the
//! function the validator's resolver uses. Providers read these facts and
//! never repeat the lookup.
//!
//! Each selected file is also read once, on the same cache miss, into a
//! [`SchemaTargetPreview`]: its format verdict, description, and the first
//! lines of its definition, which the `$schema` value hover shows without
//! touching the disk.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::resolve::resolve_yaml_schema_with_roots;
use serde_yaml_ng::Value as YamlValue;
use simplified_schema::SchemaError;
use simplified_schema::resolve::{
    SchemaFileSelection, SchemaTargetFormat, SelectionOutcome, inspect_schema_target, select_schema_file,
};

use crate::work_counters;

use super::{FmEntry, FmEntryRole, FmValueKind, FrontmatterAst};

/// Every `$schema` reference of one document with what it selects.
#[derive(Debug, Default)]
pub struct SchemaReferenceFacts {
    /// In document order: the scalar `/$schema`, or each string item of a
    /// root-union sequence.
    pub entries: Vec<SchemaReferenceFact>,
}

/// One `$schema` string and the file it selects.
#[derive(Debug)]
pub struct SchemaReferenceFact {
    /// The JSON Pointer of the authored value: `/$schema` or `/$schema/{n}`.
    pub pointer: String,
    /// The decoded string value, as the validator reads it.
    pub reference: String,
    /// What the reference selects, or why the schema roots it is looked up in
    /// could not be read (no root is guessed then).
    pub selection: Result<SchemaFileSelection, Arc<SchemaError>>,
    /// What the selected file holds, when the selection found one.
    pub target: Option<SchemaTargetPreview>,
}

/// The most definition lines a [`SchemaTargetPreview`] keeps.
pub const PREVIEW_LINES: usize = 10;

/// What the file a `$schema` reference selected holds, as its value hover
/// shows it.
#[derive(Debug)]
pub struct SchemaTargetPreview {
    /// How the file is read.
    pub format: SchemaTargetFormat,
    /// The file's nonblank top-level `description`.
    pub description: Option<String>,
    /// The first [`PREVIEW_LINES`] lines of the authored definition; empty
    /// when the file is not an accepted schema target.
    pub preview: Vec<String>,
    /// How many lines the whole definition has.
    pub total_lines: usize,
    /// Whether the reference can be used as a schema.
    pub verdict: SchemaTargetVerdict,
}

/// Whether a selected `$schema` file can be used as the document's schema.
#[derive(Debug)]
pub enum SchemaTargetVerdict {
    /// The file and everything it imports resolved.
    Valid,
    /// The error resolving this one reference reports: a format rejection,
    /// an unreadable or malformed file, or a failure in what it imports.
    Rejected(Arc<SchemaError>),
}

impl SchemaReferenceFacts {
    /// Selects every `$schema` reference of `text` in `schema_roots`, or
    /// records the roots' failure on each.
    ///
    /// Only an authored scalar that decodes to a YAML string is a reference:
    /// an inline mapping, `null`, a number, a boolean, or an alias is not.
    ///
    /// Every found file is read and inspected for its preview. `assembled`
    /// says whether the document's schema assembled from these references:
    /// then every target is valid; otherwise each accepted target is resolved
    /// again on its own, so a failure of another reference or of a trigger
    /// does not mislabel it.
    pub fn select(
        text: &str,
        path: &Path,
        schema_roots: Result<&[PathBuf], &Arc<SchemaError>>,
        context: &FileResolutionContext,
        assembled: bool,
    ) -> Self {
        let Some(ast) = FrontmatterAst::parse(text).and_then(|parse| parse.ast) else {
            return Self::default();
        };
        Self::select_in(&ast, text, path, schema_roots, context, assembled)
    }

    /// [`Self::select`] over `ast`, the tree of `text` (a whole YAML
    /// document's, whose `$schema` names its schema on the fallback route).
    pub(crate) fn select_in(
        ast: &FrontmatterAst,
        text: &str,
        path: &Path,
        schema_roots: Result<&[PathBuf], &Arc<SchemaError>>,
        context: &FileResolutionContext,
        assembled: bool,
    ) -> Self {
        let base_dir = path.parent().unwrap_or(path);
        let entries = references(ast, text)
            .map(|(pointer, reference)| {
                let (selection, target) = match schema_roots {
                    Ok(roots) => {
                        let selection = select_schema_file(&reference, base_dir, roots, context);
                        let target = match &selection.outcome {
                            SelectionOutcome::Found(file) => Some(preview(file, assembled, || {
                                resolve_yaml_schema_with_roots(
                                    &YamlValue::String(reference.clone()),
                                    base_dir,
                                    roots,
                                    context,
                                )
                                .map(drop)
                            })),
                            _ => None,
                        };
                        (Ok(selection), target)
                    }
                    Err(error) => (Err(Arc::clone(error)), None),
                };
                SchemaReferenceFact { pointer, reference, selection, target }
            })
            .collect();
        Self { entries }
    }

    /// The selected path of every reference that found its file.
    pub fn selected(&self) -> impl Iterator<Item = &PathBuf> {
        self.entries.iter().filter_map(|fact| match &fact.selection {
            Ok(SchemaFileSelection { outcome: SelectionOutcome::Found(path), .. }) => Some(path),
            _ => None,
        })
    }

    /// Every location the selections probed, with whether a file existed
    /// there now. A cached schema compares this against the disk so a file
    /// created at, or removed from, a probed location reselects.
    pub fn probes(&self) -> Vec<(PathBuf, bool)> {
        let mut seen = std::collections::HashSet::new();
        self.entries
            .iter()
            .filter_map(|fact| fact.selection.as_ref().ok())
            .flat_map(|selection| selection.attempted.iter())
            .filter(|path| seen.insert(path.as_path()))
            .map(|path| (path.clone(), path.is_file()))
            .collect()
    }
}

/// Reads and inspects the selected `file`. An accepted file is
/// [`SchemaTargetVerdict::Valid`] when the document's schema `assembled`, and
/// otherwise whatever `resolve` (this reference alone) reports.
fn preview(file: &Path, assembled: bool, resolve: impl FnOnce() -> Result<(), SchemaError>) -> SchemaTargetPreview {
    work_counters::record_schema_target_inspection();
    let format = if file.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("json")) {
        SchemaTargetFormat::Json
    } else {
        SchemaTargetFormat::Yaml
    };
    let rejected = |error: SchemaError| SchemaTargetPreview {
        format,
        description: None,
        preview: Vec::new(),
        total_lines: 0,
        verdict: SchemaTargetVerdict::Rejected(Arc::new(error)),
    };
    let bytes = match work_counters::read(file) {
        Ok(bytes) => bytes,
        Err(source) => return rejected(SchemaError::Io { path: file.to_path_buf(), source }),
    };
    let inspection = inspect_schema_target(file, &bytes);
    if let Err(error) = inspection.classification {
        return rejected(error);
    }
    let verdict = match assembled {
        true => SchemaTargetVerdict::Valid,
        false => match resolve() {
            Ok(()) => SchemaTargetVerdict::Valid,
            Err(error) => SchemaTargetVerdict::Rejected(Arc::new(error)),
        },
    };
    let mut lines = inspection.lines.unwrap_or_default();
    let total_lines = lines.len();
    lines.truncate(PREVIEW_LINES);
    SchemaTargetPreview {
        format: inspection.format,
        description: inspection.description,
        preview: lines,
        total_lines,
        verdict,
    }
}

/// The `(pointer, decoded string)` of each `$schema` reference in `ast`
/// (none when its `$schema` is data; see [`FrontmatterAst::schema_entry`]).
pub(crate) fn references<'a>(ast: &'a FrontmatterAst, text: &'a str) -> impl Iterator<Item = (String, String)> + 'a {
    let entries = ast.entries();
    let root = ast.schema_entry().and_then(|entry| ast.index_of(entry));
    let candidates: Vec<&FmEntry> = match root.map(|index| (index, &entries[index])) {
        Some((_, entry)) if entry.kind == FmValueKind::Scalar => vec![entry],
        Some((index, entry)) if entry.kind == FmValueKind::Sequence => entries
            .iter()
            .filter(|item| item.parent == Some(index) && matches!(item.role, FmEntryRole::SequenceItem { .. }))
            .collect(),
        _ => Vec::new(),
    };
    candidates.into_iter().filter_map(move |entry| {
        let reference = decoded_string(entry, text)?;
        Some((entry.pointer.clone(), reference))
    })
}

/// The YAML string an authored scalar decodes to, if it is one.
pub(crate) fn decoded_string(entry: &FmEntry, text: &str) -> Option<String> {
    if entry.kind != FmValueKind::Scalar {
        return None;
    }
    let authored = text.get(entry.value_span.clone())?;
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(authored).ok()? {
        serde_yaml_ng::Value::String(value) => Some(value),
        _ => None,
    }
}
