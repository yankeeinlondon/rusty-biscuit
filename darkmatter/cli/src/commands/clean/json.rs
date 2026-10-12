//! The version-1 `md clean --json` STDOUT envelope.
//!
//! The library reports diagnostics and accepted repairs in authored document
//! coordinates; this module adds the source description and 1-based line and
//! byte-column positions, and is the only place the wire shape is defined.

use biscuit_file::{SourceSpan, YamlCertainty, YamlRepair};
use darkmatter::markdown::cleanup::diagnostic::{CleanCode, CleanDiagnostic};
use darkmatter::markdown::cleanup::{CleanOutcome, CleanResult};
use darkmatter::markdown::span::line_col_of_offset;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct CleanJsonReport {
    pub version: u8,
    pub source: CleanJsonSource,
    pub frontmatter: CleanJsonFrontmatter,
    pub diagnostics: Vec<CleanJsonDiagnostic>,
    pub applied: Vec<CleanJsonRepair>,
    /// Whether the emitted document differs from the source (outcome
    /// `Changed`). A declined or invalid run is always `false`.
    pub changed: bool,
}

#[derive(Debug, Serialize)]
pub struct CleanJsonSource {
    pub kind: CleanJsonSourceKind,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanJsonSourceKind {
    File,
    Stdin,
}

#[derive(Debug, Serialize)]
pub struct CleanJsonFrontmatter {
    pub present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<CleanJsonSpan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanJsonSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Serialize)]
pub struct CleanJsonPositionedSpan {
    pub start: usize,
    pub end: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Serialize)]
pub struct CleanJsonDiagnostic {
    pub code: CleanCode,
    pub classification: YamlCertainty,
    pub message: String,
    pub span: CleanJsonPositionedSpan,
    pub repairs: Vec<CleanJsonRepair>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanJsonRepair {
    pub span: CleanJsonSpan,
    pub replacement: String,
    pub explanation: String,
}

/// Builds the `--json` envelope for a clean result over `document_source`,
/// the document exactly as it was read.
pub fn json_report(
    result: &CleanResult,
    path: Option<PathBuf>,
    document_source: &str,
) -> CleanJsonReport {
    let source = CleanJsonSource {
        kind: if path.is_some() {
            CleanJsonSourceKind::File
        } else {
            CleanJsonSourceKind::Stdin
        },
        path,
    };
    let frontmatter = CleanJsonFrontmatter {
        present: result.frontmatter_span.is_some(),
        span: result.frontmatter_span.as_ref().map(json_span),
    };

    CleanJsonReport {
        version: 1,
        source,
        frontmatter,
        diagnostics: result
            .diagnostics
            .iter()
            .map(|entry| json_diagnostic(entry, document_source))
            .collect(),
        applied: result.applied.iter().map(json_repair).collect(),
        changed: result.outcome == CleanOutcome::Changed,
    }
}

fn json_diagnostic(entry: &CleanDiagnostic, document_source: &str) -> CleanJsonDiagnostic {
    let (start_line, start_column) = line_col_of_offset(document_source, entry.span.start);
    let (end_line, end_column) = line_col_of_offset(document_source, entry.span.end);

    CleanJsonDiagnostic {
        code: entry.code,
        classification: entry.classification,
        message: entry.message.clone(),
        span: CleanJsonPositionedSpan {
            start: entry.span.start,
            end: entry.span.end,
            start_line,
            start_column,
            end_line,
            end_column,
        },
        repairs: entry.repairs.iter().map(json_repair).collect(),
    }
}

fn json_repair(repair: &YamlRepair) -> CleanJsonRepair {
    CleanJsonRepair {
        span: json_span(&repair.span),
        replacement: repair.replacement.clone(),
        explanation: repair.explanation.clone(),
    }
}

fn json_span(span: &SourceSpan) -> CleanJsonSpan {
    CleanJsonSpan {
        start: span.start,
        end: span.end,
    }
}
