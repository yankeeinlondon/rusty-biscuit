//! The versioned JSON report of the declaration commands (`--json`,
//! `--json-full`).
//!
//! The envelope is written piece by piece so file entries stream as their
//! files finish, and it always ends with the `errors` array: a report whose
//! output was cut short never parses, and a failed file can never look like
//! success.
//!
//! ```json
//! {"report_schema_version":1,"schema_version":{"major":4,"minor":0},
//!  "root_dir":"/repo","files":[{"file":"src/lib.rs", ...}],"errors":[]}
//! ```

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use biscuit_file::to_portable_string;
use serde::Serialize;
use tree_hugger::{
    AnalysisPass, Capability, FileSymbolIndex, ProgrammingLanguage, SchemaVersion, SymbolRecord,
    TreeHuggerError,
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
        TreeHuggerError::Ignore(_) => "ignore",
    }
}

/// `path` relative to `root`, `/`-separated. A relative `path` is taken
/// from the working directory; a path outside `root` stays whole.
pub(crate) fn portable_relative(path: &Path, root: &Path) -> String {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    match absolute.strip_prefix(root) {
        Ok(relative) => to_portable_string(relative),
        Err(_) => to_portable_string(path),
    }
}

/// Prepares a record for a default `--json` report: its paths made portable
/// and scan-root-relative, and the blobs no default report requests dropped:
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

/// Rewrites the record's file path, and the path segment of its stable key,
/// scan-root-relative. The ID was derived from the key as analyzed and is
/// left unchanged, so it still matches the cache and other runs over the
/// same checkout.
fn relativize_record(record: &mut SymbolRecord, root: &Path) {
    let analyzed = record.source.file_path.to_string_lossy().replace('\\', "/");
    let relative = portable_relative(&record.source.file_path, root);
    let key_segment = format!("::{}::", analyzed.trim());
    if let Some(at) = record.identity.stable_key.find(&key_segment) {
        record
            .identity
            .stable_key
            .replace_range(at..at + key_segment.len(), &format!("::{relative}::"));
    }
    record.source.file_path = PathBuf::from(relative);
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
        serde_json::to_writer(&mut out, &SchemaVersion::V4_0)?;
        out.write_all(b",\"root_dir\":")?;
        serde_json::to_writer(&mut out, root_dir)?;
        write!(out, ",\"{}\":[", entries.key())?;
        Ok(Self { out, entries: 0 })
    }

    /// Appends one entry.
    pub(crate) fn entry<T: Serialize>(&mut self, entry: &T) -> io::Result<()> {
        if self.entries > 0 {
            self.out.write_all(b",")?;
        }
        serde_json::to_writer(&mut self.out, entry)?;
        self.entries += 1;
        Ok(())
    }

    /// Closes the entries array, writes `errors` and flushes.
    pub(crate) fn finish(mut self, errors: &[ReportError]) -> io::Result<()> {
        self.out.write_all(b"],\"errors\":")?;
        serde_json::to_writer(&mut self.out, errors)?;
        self.out.write_all(b"}\n")?;
        self.out.flush()
    }
}
