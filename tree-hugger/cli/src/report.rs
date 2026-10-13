//! The versioned JSON reports of the declaration commands (`--json`,
//! `--json-full`, and the `--jsonl` line envelope).
//!
//! The envelope is written piece by piece so file entries stream as their
//! files finish, and it always ends with the `errors` array: a report whose
//! output was cut short never parses, and a failed file can never look like
//! success.
//!
//! ```json
//! {"report_schema_version":1,"schema_version":{"major":4,"minor":1},
//!  "root_dir":"/repo","files":[{"file":"src/lib.rs", ...}],"errors":[]}
//! ```

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::Serialize;
pub(crate) use tree_hugger::portable_relative;
use tree_hugger::{
    AnalysisPass, Capability, FileSymbolIndex, ProgrammingLanguage, SchemaVersion, SymbolKindV2,
    SymbolRecord, TreeHuggerError,
};

/// Version of the envelope itself; the records inside carry `schema_version`.
pub(crate) const REPORT_SCHEMA_VERSION: u32 = 1;

/// Which array holds the per-file entries.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Entries {
    /// `files`: the selected records of each file with a match (`--json`).
    Files,
    /// `indexes`: every file's complete index (`--json-full`).
    Indexes,
}

impl Entries {
    fn key(self) -> &'static str {
        match self {
            Self::Files => "files",
            Self::Indexes => "indexes",
        }
    }
}

/// One file's entry in a `--json` report.
#[derive(Debug, Serialize)]
pub(crate) struct FileEntry<'a> {
    pub(crate) file: &'a str,
    pub(crate) language: ProgrammingLanguage,
    pub(crate) completed_passes: &'a [AnalysisPass],
    pub(crate) completed_capabilities: &'a [Capability],
    pub(crate) symbols: &'a [SymbolRecord],
    /// `classes` only: the displayed methods and constructors of each
    /// container in `symbols`, keyed by the container's ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) members: Option<&'a BTreeMap<String, Vec<SymbolRecord>>>,
}

/// One `--jsonl` line: a selected record with its file's context, so a
/// saved or piped line is usable alone. `report_schema_version` and
/// `schema_version` mean what they mean in the `--json` envelope.
#[derive(Debug, Serialize)]
pub(crate) struct JsonLine<'a> {
    pub(crate) report_schema_version: u32,
    pub(crate) schema_version: SchemaVersion,
    pub(crate) file: &'a str,
    pub(crate) language: ProgrammingLanguage,
    pub(crate) completed_passes: &'a [AnalysisPass],
    pub(crate) completed_capabilities: &'a [Capability],
    pub(crate) record: &'a SymbolRecord,
    /// `classes` only: the container's displayed methods and constructors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) members: Option<&'a [SymbolRecord]>,
}

impl JsonLine<'_> {
    /// Writes the line and its newline.
    pub(crate) fn write(&self, out: &mut dyn Write) -> io::Result<()> {
        serde_json::to_writer(&mut *out, self)?;
        out.write_all(b"\n")
    }
}

/// `entry` as compact JSON, the bytes [`JsonReport::entry`] appends; a
/// serialization failure is the write error it becomes.
pub(crate) fn serialized<T: Serialize>(entry: &T) -> io::Result<Vec<u8>> {
    Ok(serde_json::to_vec(entry)?)
}

/// A file that could not be analyzed.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReportError {
    /// Portable scan-root-relative path.
    pub(crate) file: String,
    /// The `TreeHuggerError` variant in snake case (`io`, `parse_failed`, ...).
    pub(crate) kind: &'static str,
    /// The error's display text.
    pub(crate) message: String,
}

impl ReportError {
    pub(crate) fn new(file: String, error: &TreeHuggerError) -> Self {
        Self {
            file,
            kind: error_kind(error),
            message: error.to_string(),
        }
    }
}

/// The snake-case name of `error`'s variant.
pub(crate) fn error_kind(error: &TreeHuggerError) -> &'static str {
    match error {
        TreeHuggerError::Io { .. } => "io",
        TreeHuggerError::UnsupportedLanguage { .. } => "unsupported_language",
        TreeHuggerError::ParseFailed { .. } => "parse_failed",
        TreeHuggerError::MissingQuery { .. } => "missing_query",
        TreeHuggerError::MissingVendorQuery { .. } => "missing_vendor_query",
        TreeHuggerError::QueryError { .. } => "query_error",
        TreeHuggerError::QueryCachePoisoned => "query_cache_poisoned",
        TreeHuggerError::GitRootNotFound { .. } => "git_root_not_found",
        TreeHuggerError::NoSourceFiles { .. } => "no_source_files",
        TreeHuggerError::TooManyGenericParameters { .. } => "too_many_generic_parameters",
        TreeHuggerError::Ignore(_) => "ignore",
    }
}

/// Prepares a selected record for `--json`, `--jsonl` or `--format table`:
/// its paths made portable and scan-root-relative, and the blobs none of
/// them requests dropped:
/// source text (`declaration_text`, body and source text) and the attached
/// comments of the docs pass (the raw doc comment stays).
pub(crate) fn trim_record(record: &mut SymbolRecord, root: &Path) {
    relativize_record(record, root);
    record.source.declaration_text = None;
    record.source.source_text = None;
    record.source.body_text = None;
    record.docs.comments.clear();
}

/// Makes every path in a `--json-full` index portable and
/// scan-root-relative; nothing else changes.
pub(crate) fn relativize_index(index: &mut FileSymbolIndex, root: &Path) {
    index.file = PathBuf::from(portable_relative(&index.file, root));
    for record in &mut index.symbols {
        relativize_record(record, root);
    }
}

/// Rewrites the record's file path, and the file segment of its stable key,
/// scan-root-relative. The two are projected separately because they can name
/// different files: a resolved prelude alias is located at its target's
/// declaration but keyed by the prelude file. The ID was derived from the key
/// as analyzed and is left unchanged, so it still matches the cache and other
/// runs over the same checkout.
fn relativize_record(record: &mut SymbolRecord, root: &Path) {
    if let Some(segment) = key_file_segment(&record.identity.stable_key, record.kind) {
        let relative = portable_relative(Path::new(&record.identity.stable_key[segment.clone()]), root);
        record.identity.stable_key.replace_range(segment, &relative);
    }
    record.source.file_path = PathBuf::from(portable_relative(&record.source.file_path, root));
}

/// The byte range of the file in a stable key
/// (`{language}::{file}::{kind}::{name}::{ordinal}`, see
/// `tree_hugger::stable_symbol_key`), or `None` when `key` has another shape.
/// The file ends at the first `::{kind}::` after the language, since a
/// qualified name may itself contain `::`.
fn key_file_segment(key: &str, kind: SymbolKindV2) -> Option<Range<usize>> {
    let start = key.find("::")? + 2;
    let end = start + key[start..].find(&format!("::{kind:?}::"))?;
    (end > start).then_some(start..end)
}

/// A report being written to `out`.
pub(crate) struct JsonReport<W: Write> {
    out: W,
    entries: usize,
}

impl<W: Write> JsonReport<W> {
    /// Writes the envelope up to the opening of the entries array.
    pub(crate) fn begin(mut out: W, root_dir: &str, entries: Entries) -> io::Result<Self> {
        write!(out, "{{\"report_schema_version\":{REPORT_SCHEMA_VERSION},\"schema_version\":")?;
        serde_json::to_writer(&mut out, &SchemaVersion::CURRENT)?;
        out.write_all(b",\"root_dir\":")?;
        serde_json::to_writer(&mut out, root_dir)?;
        write!(out, ",\"{}\":[", entries.key())?;
        Ok(Self { out, entries: 0 })
    }

    /// Appends one entry, already serialized (see [`serialized`]).
    pub(crate) fn entry(&mut self, entry: &[u8]) -> io::Result<()> {
        if self.entries > 0 {
            self.out.write_all(b",")?;
        }
        self.out.write_all(entry)?;
        self.entries += 1;
        Ok(())
    }

    /// Flushes when the scan asks, so a failed write stops the scan within
    /// one window; see [`crate::scan::in_file_order`].
    pub(crate) fn flush(&mut self, flush: crate::scan::Flush) -> io::Result<crate::scan::Flushed> {
        flush.apply(&mut self.out)
    }

    /// Closes the entries array, writes `errors` and flushes.
    pub(crate) fn finish(mut self, errors: &[ReportError]) -> io::Result<()> {
        self.out.write_all(b"],\"errors\":")?;
        serde_json::to_writer(&mut self.out, errors)?;
        self.out.write_all(b"}\n")?;
        self.out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_file::canonicalize_simplified;
    use tree_hugger::{
        CodeRange, SymbolInfo, SymbolKind, SymbolScope, stable_symbol_key, symbol_id_from_stable_key,
    };

    /// A canonical directory holding `repo/` (the scan root) and `vendor/`.
    fn layout() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let base = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(base.join("repo/src")).expect("repo dir");
        std::fs::create_dir_all(base.join("vendor")).expect("vendor dir");
        (dir, base)
    }

    fn record_at(file: &Path, key_file: &Path) -> SymbolRecord {
        let mut record = SymbolRecord::from(SymbolInfo {
            name: "Alias".into(),
            kind: SymbolKind::Enum,
            scope: SymbolScope::Item,
            qualified_name: "Alias".into(),
            is_namespace_rooted: false,
            range: CodeRange {
                start_line: 1,
                start_column: 0,
                end_line: 1,
                end_column: 1,
                start_byte: 0,
                end_byte: 1,
            },
            language: ProgrammingLanguage::Rust,
            file: file.to_path_buf(),
            container_name: None,
            container_kind: None,
            doc_comment: None,
            visibility: None,
            is_static: false,
            is_static_storage: false,
            is_async: false,
            signature: None,
            type_metadata: None,
        });
        record.identity.stable_key = stable_symbol_key(
            ProgrammingLanguage::Rust,
            key_file,
            record.kind,
            "prelude-export::Alias",
            0,
        );
        record.id = symbol_id_from_stable_key(&record.identity.stable_key);
        record
    }

    #[test]
    fn the_key_file_is_found_before_the_kind() {
        let kind = SymbolKindV2::Function;
        let key = "rust::/repo/src/lib.rs::Function::outer::inner::Function::0";
        assert_eq!(&key[key_file_segment(key, kind).expect("segment")], "/repo/src/lib.rs");
        let windows = "rust::C:/repo/src/lib.rs::Function::f::0";
        assert_eq!(&windows[key_file_segment(windows, kind).expect("segment")], "C:/repo/src/lib.rs");
        assert_eq!(key_file_segment("rust::/repo/src/lib.rs::Enum::f::0", kind), None, "another kind");
        assert_eq!(key_file_segment("no separators", kind), None);
    }

    /// A prelude alias is located at its target but keyed by the prelude file:
    /// each path is projected from its own file, and the ID is kept.
    #[test]
    fn record_and_key_paths_are_projected_from_their_own_files() {
        let (_dir, base) = layout();
        let root = base.join("repo");
        let mut record = record_at(&root.join("src/alpha.rs"), &root.join("src/prelude.rs"));
        let id = record.id.clone();
        relativize_record(&mut record, &root);
        assert_eq!(record.source.file_path, Path::new("src/alpha.rs"));
        assert_eq!(record.identity.stable_key, "rust::src/prelude.rs::Enum::prelude-export::Alias::0");
        assert_eq!(record.id, id);

        let mut external = record_at(&base.join("vendor/x.rs"), &base.join("vendor/x.rs"));
        relativize_record(&mut external, &root);
        assert_eq!(external.source.file_path, Path::new("../vendor/x.rs"));
        assert_eq!(external.identity.stable_key, "rust::../vendor/x.rs::Enum::prelude-export::Alias::0");
    }
}
