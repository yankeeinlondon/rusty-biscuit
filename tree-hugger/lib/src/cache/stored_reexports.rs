//! Checks a read entry's re-export report against its source, its language
//! and its imports. Deserialization already refuses an absent, null,
//! mistyped, repeated or unknown member; these are the well-typed
//! inconsistencies a fresh analysis could not have produced.

use crate::analysis::reexports::is_not_applicable;
use crate::shared::{
    ImportRecord, ProgrammingLanguage, ReexportCoverageState, ReexportEntry, ReexportForm, ReexportReport,
};

use super::stored_evidence::LineStarts;

/// The error names the first inconsistency found.
pub(super) fn validate_reexports(
    lines: &LineStarts<'_>,
    language: ProgrammingLanguage,
    imports: &[ImportRecord],
    report: &ReexportReport,
) -> Result<(), &'static str> {
    let coverage = &report.coverage;
    if (coverage.state == ReexportCoverageState::NotApplicable) != is_not_applicable(language) {
        return Err("a coverage state that disagrees with the language");
    }
    if coverage.state == ReexportCoverageState::NotApplicable && !report.entries.is_empty() {
        return Err("entries in a language with no forwarding construct");
    }
    if (coverage.state == ReexportCoverageState::Limited) == coverage.limits.is_empty() {
        return Err("limits that disagree with the coverage state");
    }
    let mut previous = None;
    for limit in &coverage.limits {
        if !lines.is_region(&limit.span) {
            return Err("a re-export limit span disagrees with the source");
        }
        let position = (limit.span.start_byte, limit.reason);
        if previous.is_some_and(|last| last > position) {
            return Err("re-export limits are out of order");
        }
        previous = Some(position);
    }
    let mut previous_start = 0;
    for entry in &report.entries {
        if entry.span.start_byte < previous_start {
            return Err("re-export entries are out of source order");
        }
        previous_start = entry.span.start_byte;
        validate_entry(lines, imports, entry)?;
    }
    for import in imports {
        let forwarded = report.entries.iter().any(|entry| entry.forwards_import_at(&import.span));
        if import.reexported != forwarded {
            return Err("an import marker that disagrees with the re-export entries");
        }
    }
    Ok(())
}

fn validate_entry(lines: &LineStarts<'_>, imports: &[ImportRecord], entry: &ReexportEntry) -> Result<(), &'static str> {
    if !lines.is_region(&entry.span) || !lines.is_region(&entry.statement_span) {
        return Err("a re-export span disagrees with the source");
    }
    if entry.statement_span.start_byte > entry.span.start_byte || entry.statement_span.end_byte < entry.span.end_byte {
        return Err("a re-export statement span that does not contain its specifier");
    }
    let unnamed = matches!(entry.form, ReexportForm::Wildcard | ReexportForm::Module);
    if entry.exported_name.is_some() == unnamed {
        return Err("an exported name that disagrees with the re-export form");
    }
    let originates = matches!(entry.form, ReexportForm::Named | ReexportForm::ValueCopy);
    if entry.original_name.is_some() != originates {
        return Err("an original name that disagrees with the re-export form");
    }
    let names = [&entry.exported_name, &entry.original_name, &entry.local_name, &entry.source, &entry.scope, &entry.restriction];
    if names.iter().any(|name| name.as_deref() == Some("")) {
        return Err("an empty re-export name or path");
    }
    if !entry.excluded_names.is_empty() && entry.form != ReexportForm::Wildcard {
        return Err("excluded names on a form that is not a wildcard");
    }
    if entry.excluded_names.iter().any(String::is_empty) {
        return Err("an empty excluded name");
    }
    if entry.caveats.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("re-export caveats are unordered or repeated");
    }
    if let Some(import_span) = &entry.import_span
        && !imports.iter().any(|import| &import.span == import_span)
    {
        return Err("a re-export import span that is not an import");
    }
    Ok(())
}
