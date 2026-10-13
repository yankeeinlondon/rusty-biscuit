//! The re-export extraction seam: one entry point the planner calls once per
//! analysis that requires `Capability::Reexports`, and the marker that links
//! forwarding entries back to the file's import records.
//!
//! User contract: `tree-hugger/docs/reexports.md`.

use crate::analysis::counters::count_reexport_extraction;
use crate::file::tree_file::TreeFile;
use crate::analysis::import_record;
use crate::shared::{ImportRecord, ImportSymbol, ProgrammingLanguage, ReexportReport};

mod common;
mod cpp;
mod csharp;
mod ecma;
mod go;
mod lua;
mod perl;
mod python;
mod rust;
mod scala;
mod swift;

/// The file's re-export report, read from its own syntax and its import
/// records only: nothing outside the file is opened.
///
/// A language with no forwarding construct reports `not_applicable`. Every
/// other language has an extractor in its own module; the arms name every
/// language so a new one cannot be skipped.
pub(crate) fn extract(tree_file: &TreeFile, imports: &[ImportRecord]) -> ReexportReport {
    count_reexport_extraction();
    match tree_file.language {
        ProgrammingLanguage::Java
        | ProgrammingLanguage::C
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh
        | ProgrammingLanguage::Php => ReexportReport::not_applicable(),
        ProgrammingLanguage::Rust => rust::extract(tree_file, imports),
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => ecma::extract(tree_file, imports),
        ProgrammingLanguage::Go => go::extract(tree_file, imports),
        ProgrammingLanguage::Python => python::extract(tree_file, imports),
        ProgrammingLanguage::Perl => perl::extract(tree_file, imports),
        ProgrammingLanguage::Cpp => cpp::extract(tree_file, imports),
        ProgrammingLanguage::CSharp => csharp::extract(tree_file, imports),
        ProgrammingLanguage::Swift => swift::extract(tree_file, imports),
        ProgrammingLanguage::Scala => scala::extract(tree_file, imports),
        ProgrammingLanguage::Lua => lua::extract(tree_file, imports),
    }
}

/// Sets `reexported` on each import that a `syntax` or `declared` entry of
/// `report` forwards, matched by exact import span (never by name), and
/// clears it on every other import.
pub(crate) fn mark_forwarded_imports(imports: &mut [ImportRecord], report: &ReexportReport) {
    for import in imports {
        import.reexported = report.entries.iter().any(|entry| entry.forwards_import_at(&import.span));
    }
}

/// Extracts the file's report and sets `is_reexport` on each import it
/// forwards. The one place the flag is decided, so the planner,
/// `imported_symbols()` and the diagnostics agree.
pub(crate) fn flag_forwarded_imports(tree_file: &TreeFile, imports: &mut [ImportSymbol]) -> ReexportReport {
    let mut records: Vec<ImportRecord> = imports.iter().map(import_record).collect();
    let report = extract(tree_file, &records);
    mark_forwarded_imports(&mut records, &report);
    for (import, record) in imports.iter_mut().zip(&records) {
        import.is_reexport = record.reexported;
    }
    report
}

/// Whether the language has no forwarding construct, which decides the
/// `not_applicable` state by language alone. Must list the languages
/// [`extract`] reports as `not_applicable`.
pub(crate) fn is_not_applicable(language: ProgrammingLanguage) -> bool {
    matches!(
        language,
        ProgrammingLanguage::Java
            | ProgrammingLanguage::C
            | ProgrammingLanguage::Bash
            | ProgrammingLanguage::Zsh
            | ProgrammingLanguage::Php
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{
        ReexportCoverage, ReexportCoverageState, ReexportEntry, ReexportEvidence, ReexportForm,
        SymbolRef, TextSpan,
    };

    fn span(start: u32, end: u32) -> TextSpan {
        TextSpan { start_byte: start, end_byte: end, ..TextSpan::default() }
    }

    fn import(name: &str, at: TextSpan) -> ImportRecord {
        ImportRecord {
            name: name.into(),
            alias: None,
            source: None,
            span: at,
            statement_span: None,
            symbol: SymbolRef { source_span: None, id: None, name: name.into(), qualified_name: None },
            reexported: true,
        }
    }

    fn entry(evidence: ReexportEvidence, import_span: Option<TextSpan>) -> ReexportEntry {
        ReexportEntry {
            form: ReexportForm::Named,
            evidence,
            exported_name: Some("x".into()),
            original_name: Some("x".into()),
            local_name: None,
            source: None,
            excluded_names: Vec::new(),
            scope: None,
            restriction: None,
            type_only: false,
            conditional: false,
            caveats: Vec::new(),
            import_span,
            span: span(0, 1),
            statement_span: span(0, 2),
        }
    }

    fn report(entries: Vec<ReexportEntry>) -> ReexportReport {
        ReexportReport {
            coverage: ReexportCoverage { state: ReexportCoverageState::Complete, limits: Vec::new() },
            entries,
        }
    }

    #[test]
    fn only_syntax_or_declared_entries_mark_the_import_at_their_exact_span() {
        let mut imports = vec![
            import("a", span(10, 11)),
            import("b", span(20, 21)),
            import("c", span(30, 31)),
            import("d", span(40, 41)),
        ];
        let report = report(vec![
            entry(ReexportEvidence::Syntax, Some(span(10, 11))),
            entry(ReexportEvidence::Declared, Some(span(20, 21))),
            entry(ReexportEvidence::Exposed, Some(span(30, 31))),
            entry(ReexportEvidence::Syntax, None),
        ]);
        mark_forwarded_imports(&mut imports, &report);
        let marked: Vec<_> = imports.iter().map(|import| import.reexported).collect();
        assert_eq!(marked, [true, true, false, false]);
    }

    #[test]
    fn an_empty_report_clears_every_marker() {
        let mut imports = vec![import("a", span(1, 2))];
        mark_forwarded_imports(&mut imports, &report(Vec::new()));
        assert!(!imports[0].reexported);
    }
}
