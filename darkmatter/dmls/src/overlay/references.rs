//! The file each `$schema` reference of a Markdown document selects.
//!
//! Validation, document links, and go-to-definition must agree on the file a
//! bare-name `$schema` names. The overlay therefore selects it once, beside
//! schema assembly and from the same inputs (the text the schema was
//! assembled from, the registry's schema roots, the document's folder, and its
//! context), through `simplified_schema::resolve::select_schema_file`, the
//! function the validator's resolver uses. Providers read these facts and
//! never repeat the lookup.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use simplified_schema::SchemaError;
use simplified_schema::resolve::{SchemaFileSelection, SelectionOutcome, select_schema_file};

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
}

impl SchemaReferenceFacts {
    /// Selects every `$schema` reference of `text` in `schema_roots`, or
    /// records the roots' failure on each.
    ///
    /// Only an authored scalar that decodes to a YAML string is a reference:
    /// an inline mapping, `null`, a number, a boolean, or an alias is not.
    pub fn select(
        text: &str,
        path: &Path,
        schema_roots: Result<&[PathBuf], &Arc<SchemaError>>,
        context: &FileResolutionContext,
    ) -> Self {
        let Some(ast) = FrontmatterAst::parse(text).and_then(|parse| parse.ast) else {
            return Self::default();
        };
        let base_dir = path.parent().unwrap_or(path);
        let entries = references(&ast, text)
            .map(|(pointer, reference)| {
                let selection = match schema_roots {
                    Ok(roots) => Ok(select_schema_file(&reference, base_dir, roots, context)),
                    Err(error) => Err(Arc::clone(error)),
                };
                SchemaReferenceFact { pointer, reference, selection }
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

/// The `(pointer, decoded string)` of each `$schema` reference in `ast`.
fn references<'a>(ast: &'a FrontmatterAst, text: &'a str) -> impl Iterator<Item = (String, String)> + 'a {
    let entries = ast.entries();
    let root = entries.iter().position(|entry| entry.pointer == "/$schema");
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
fn decoded_string(entry: &FmEntry, text: &str) -> Option<String> {
    if entry.kind != FmValueKind::Scalar {
        return None;
    }
    let authored = text.get(entry.value_span.clone())?;
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(authored).ok()? {
        serde_yaml_ng::Value::String(value) => Some(value),
        _ => None,
    }
}
