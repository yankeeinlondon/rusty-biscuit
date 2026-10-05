//! `--stats`: counts of the selected declarations by kind, visibility and
//! language, and their doc coverage, written as terminal tables or as a
//! versioned JSON envelope.
//!
//! ```json
//! {"report_schema_version":1,"root_dir":"/repo",
//!  "stats":{"by_kind":{"Function":3},"by_visibility":{"Public":3},"by_language":{"Rust":3},
//!           "doc_coverage":{"documented":2,"total":3,"unavailable":0,"ratio":0.6666666666666666},
//!           "synthetic_exports":0},
//!  "errors":[]}
//! ```

use std::collections::BTreeMap;
use std::io::{self, Write};

use biscuit_terminal::prelude::{
    ColumnType, Table, TableCellContent, TableColumn, Terminal, TerminalRenderable,
};
use serde::Serialize;
use tree_hugger::{SymbolKind, SymbolRecord, Visibility, VisibilityLevel};

use crate::report::{REPORT_SCHEMA_VERSION, ReportError};

/// The aggregate of one run's selected declarations.
///
/// Every selected record counts in `by_kind`, `by_visibility` and
/// `by_language`, so each map sums to `--count`. Synthetic prelude exports
/// are counted apart in `synthetic_exports` and left out of `doc_coverage`,
/// which measures declarations.
#[derive(Debug, Default)]
pub(crate) struct Stats {
    by_kind: BTreeMap<String, u64>,
    by_visibility: BTreeMap<String, u64>,
    by_language: BTreeMap<String, u64>,
    doc_coverage: DocCoverage,
    synthetic_exports: u64,
    /// Display names of the keys above, for the terminal tables.
    labels: BTreeMap<String, String>,
}

/// `documented` over `total` (records whose `has_docs` is known); a record
/// whose language has no doc-comment detection counts as `unavailable`.
#[derive(Debug, Default)]
struct DocCoverage {
    documented: u64,
    total: u64,
    unavailable: u64,
}

impl DocCoverage {
    /// `documented / total`, or `None` (JSON `null`) when `total` is 0.
    fn ratio(&self) -> Option<f64> {
        (self.total > 0).then(|| self.documented as f64 / self.total as f64)
    }
}

impl Stats {
    /// Counts one selected record. `record.metrics` must have been computed
    /// (the stats query requests `Capability::Metrics`).
    pub(crate) fn add(&mut self, record: &SymbolRecord) {
        let kind = serde_key(&record.kind);
        self.labels
            .entry(kind.clone())
            .or_insert_with(|| SymbolKind::from(record.kind).to_string());
        *self.by_kind.entry(kind).or_default() += 1;

        let level = serde_key(&record.visibility.level);
        self.labels
            .entry(level.clone())
            .or_insert_with(|| visibility_label(record.visibility.level));
        *self.by_visibility.entry(level).or_default() += 1;

        let language = serde_key(&record.language);
        self.labels
            .entry(language.clone())
            .or_insert_with(|| record.language.to_string());
        *self.by_language.entry(language).or_default() += 1;

        if record.visibility.is_synthetic {
            self.synthetic_exports += 1;
            return;
        }
        match record.metrics.as_ref().and_then(|metrics| metrics.has_docs) {
            Some(documented) => {
                self.doc_coverage.total += 1;
                self.doc_coverage.documented += u64::from(documented);
            }
            None => self.doc_coverage.unavailable += 1,
        }
    }

    /// Writes the stats JSON envelope, `errors` last.
    pub(crate) fn write_json(&self, out: &mut dyn Write, root_dir: &str, errors: &[ReportError]) -> io::Result<()> {
        let envelope = Envelope {
            report_schema_version: REPORT_SCHEMA_VERSION,
            root_dir,
            stats: StatsJson {
                by_kind: &self.by_kind,
                by_visibility: &self.by_visibility,
                by_language: &self.by_language,
                doc_coverage: CoverageJson {
                    documented: self.doc_coverage.documented,
                    total: self.doc_coverage.total,
                    unavailable: self.doc_coverage.unavailable,
                    ratio: self.doc_coverage.ratio(),
                },
                synthetic_exports: self.synthetic_exports,
            },
            errors,
        };
        serde_json::to_writer(&mut *out, &envelope)?;
        out.write_all(b"\n")?;
        out.flush()
    }

    /// Writes the three tables (by kind, visibility and language) and the
    /// doc-coverage line. `term` decides styling and width.
    pub(crate) fn write_terminal(&self, out: &mut dyn Write, term: &Terminal, plain: bool) -> io::Result<()> {
        for (title, counts) in [
            ("Kind", &self.by_kind),
            ("Visibility", &self.by_visibility),
            ("Language", &self.by_language),
        ] {
            let rows: Vec<Vec<TableCellContent>> = counts
                .iter()
                .map(|(key, count)| {
                    let label = self.labels.get(key).cloned().unwrap_or_else(|| key.clone());
                    vec![label.into(), (*count as i64).into()]
                })
                .collect();
            let table = Table::new()
                .with_columns(vec![
                    TableColumn::new(title),
                    TableColumn::new("Count").with_type(ColumnType::Integer),
                ])
                .with_data(rows);
            write_rendered(out, &table.render(term), plain)?;
            writeln!(out)?;
        }
        let coverage = &self.doc_coverage;
        let ratio = match coverage.ratio() {
            Some(ratio) => format!("{:.1}%", ratio * 100.0),
            None => "n/a".to_string(),
        };
        writeln!(
            out,
            "Doc coverage: {} of {} documented ({ratio}); {} unavailable",
            coverage.documented, coverage.total, coverage.unavailable
        )?;
        if self.synthetic_exports > 0 {
            writeln!(out, "Synthetic exports: {}", self.synthetic_exports)?;
        }
        out.flush()
    }
}

#[derive(Serialize)]
struct Envelope<'a> {
    report_schema_version: u32,
    root_dir: &'a str,
    stats: StatsJson<'a>,
    errors: &'a [ReportError],
}

#[derive(Serialize)]
struct StatsJson<'a> {
    by_kind: &'a BTreeMap<String, u64>,
    by_visibility: &'a BTreeMap<String, u64>,
    by_language: &'a BTreeMap<String, u64>,
    doc_coverage: CoverageJson,
    synthetic_exports: u64,
}

#[derive(Serialize)]
struct CoverageJson {
    documented: u64,
    total: u64,
    unavailable: u64,
    ratio: Option<f64>,
}

/// `value`'s serialized spelling, as records spell it (`Function`,
/// `Public`, `Rust`).
fn serde_key<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(key)) => key,
        other => unreachable!("unit enum variants serialize as strings: {other:?}"),
    }
}

fn visibility_label(level: VisibilityLevel) -> String {
    Option::<Visibility>::from(level).map_or_else(|| "unknown".to_string(), |visibility| visibility.to_string())
}

/// Writes rendered table text; plain output strips any escape the table
/// emitted regardless of color depth.
fn write_rendered(out: &mut dyn Write, rendered: &str, plain: bool) -> io::Result<()> {
    let rendered = if plain {
        biscuit_terminal::prelude::strip_escape_codes(rendered.to_string())
    } else {
        rendered.to_string()
    };
    for line in rendered.lines() {
        writeln!(out, "{line}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_hugger::{CodeRange, MetricsFacet, ProgrammingLanguage, SymbolInfo, SymbolKindV2, SymbolScope};

    fn kind_key(kind: SymbolKindV2) -> String {
        serde_key(&kind)
    }

    fn record(kind: tree_hugger::SymbolKind, language: ProgrammingLanguage, has_docs: Option<bool>) -> SymbolRecord {
        let mut record = SymbolRecord::from(SymbolInfo {
            name: "x".into(),
            kind,
            scope: SymbolScope::Item,
            qualified_name: "x".into(),
            is_namespace_rooted: false,
            range: CodeRange {
                start_line: 1,
                start_column: 0,
                end_line: 1,
                end_column: 1,
                start_byte: 0,
                end_byte: 1,
            },
            language,
            file: "x".into(),
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
        record.metrics = Some(MetricsFacet {
            parameters: None,
            has_docs,
        });
        record
    }

    fn text(stats: &Stats) -> String {
        let mut out = Vec::new();
        stats.write_json(&mut out, "/root", &[]).expect("write");
        String::from_utf8(out).expect("utf-8")
    }

    fn json(stats: &Stats) -> serde_json::Value {
        serde_json::from_str(&text(stats)).expect("json")
    }

    #[test]
    fn coverage_counts_known_docs_and_reports_unknown_apart() {
        let mut stats = Stats::default();
        stats.add(&record(tree_hugger::SymbolKind::Function, ProgrammingLanguage::Rust, Some(true)));
        stats.add(&record(tree_hugger::SymbolKind::Function, ProgrammingLanguage::Rust, Some(false)));
        stats.add(&record(tree_hugger::SymbolKind::Function, ProgrammingLanguage::Bash, None));
        let mut synthetic = record(tree_hugger::SymbolKind::Function, ProgrammingLanguage::Rust, Some(false));
        synthetic.visibility.is_synthetic = true;
        stats.add(&synthetic);

        let value = json(&stats);
        assert_eq!(value["report_schema_version"], 1);
        assert_eq!(value["root_dir"], "/root");
        assert_eq!(value["stats"]["by_kind"][kind_key(SymbolKindV2::Function)], 4);
        assert_eq!(value["stats"]["by_language"]["Rust"], 3);
        assert_eq!(value["stats"]["by_language"]["Bash"], 1);
        assert_eq!(value["stats"]["by_visibility"]["Unknown"], 4);
        let coverage = &value["stats"]["doc_coverage"];
        assert_eq!(coverage["documented"], 1);
        assert_eq!(coverage["total"], 2);
        assert_eq!(coverage["unavailable"], 1);
        assert_eq!(coverage["ratio"], 0.5);
        assert_eq!(value["stats"]["synthetic_exports"], 1);
        assert_eq!(value["errors"], serde_json::json!([]));
        let raw = text(&stats);
        assert!(raw.ends_with(",\"errors\":[]}\n"), "errors comes last: {raw}");
        assert!(raw.starts_with("{\"report_schema_version\":1,\"root_dir\":"), "{raw}");
        assert!(!raw.contains("schema_version\":{"), "no record schema version: {raw}");
    }

    #[test]
    fn ratio_is_null_when_no_record_has_known_docs() {
        let empty = json(&Stats::default());
        assert!(empty["stats"]["doc_coverage"]["ratio"].is_null());
        assert_eq!(empty["stats"]["doc_coverage"]["total"], 0);
        assert_eq!(empty["stats"]["by_kind"], serde_json::json!({}));

        let mut unknown = Stats::default();
        unknown.add(&record(tree_hugger::SymbolKind::Function, ProgrammingLanguage::Lua, None));
        let value = json(&unknown);
        assert!(value["stats"]["doc_coverage"]["ratio"].is_null());
        assert_eq!(value["stats"]["doc_coverage"]["unavailable"], 1);
    }
}
