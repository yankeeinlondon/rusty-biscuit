//! Shared helpers for the per-language re-export tests: report a source
//! snippet through the public `TreeFile::reexports` and `query` paths.

use tempfile::TempDir;
use tree_hugger::{Capability, ReexportReport, SymbolQuery, TreeFile};

/// A source file written to a temp dir under `file_name` (the extension picks
/// the language) and parsed.
pub struct Snippet {
    _dir: TempDir,
    pub file: TreeFile,
    pub source: String,
}

pub fn snippet(file_name: &str, source: &str) -> Snippet {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join(file_name);
    std::fs::write(&path, source).expect("write snippet");
    let file = TreeFile::new(&path).expect("parse snippet");
    Snippet { _dir: dir, file, source: source.to_owned() }
}

impl Snippet {
    /// The report through the direct API.
    pub fn report(&self) -> ReexportReport {
        self.file.reexports().expect("reexports")
    }

    /// The report through the planner, with the import markers.
    pub fn indexed(&self) -> tree_hugger::FileSymbolIndex {
        self.file
            .query(&SymbolQuery::new().kinds([]).capabilities([Capability::Reexports]))
            .expect("query")
            .index
    }

    /// The text a span covers.
    pub fn text(&self, span: &tree_hugger::TextSpan) -> &str {
        &self.source[span.start_byte as usize..span.end_byte as usize]
    }
}
