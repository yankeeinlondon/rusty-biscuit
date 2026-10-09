//! `--stats`: counts of the selected declarations by kind, visibility and
//! language, and their doc coverage, written as terminal tables or as a
//! versioned JSON envelope.
//!
//! Overall and package counters share one stats shape. The additive report
//! envelope keeps schema version 1; package paths are relative to its
//! `package_root_dir`, independently of the normal scan `root_dir`.

use std::collections::BTreeMap;
use std::io::{self, Write};

use biscuit_terminal::prelude::{
    ColumnType, Prose, WordWrap, Table, TableCellContent, TableColumn, Terminal, TerminalRenderable,
};
use serde::Serialize;
use tree_hugger::{ProgrammingLanguage, SymbolKind, SymbolKindV2, SymbolRecord, Visibility, VisibilityLevel};

use crate::grouping::{Group, PackageIdentity};
use crate::render::{literal, table_terminal};
use crate::report::{REPORT_SCHEMA_VERSION, ReportError};

/// The aggregate of one run's selected declarations.
///
/// Every selected record counts in `by_kind`, `by_visibility` and
/// `by_language`, so each map sums to `--count`. Synthetic prelude exports
/// are counted apart in `synthetic_exports` and left out of `doc_coverage`,
/// which measures declarations.
#[derive(Debug, Default)]
pub(crate) struct Stats {
    by_kind: Counts<SymbolKindV2>,
    by_visibility: Counts<VisibilityLevel>,
    by_language: Counts<ProgrammingLanguage>,
    doc_coverage: DocCoverage,
    synthetic_exports: u64,
}

/// Counts by value, in first-seen order; [`Self::keyed`] orders them by
/// their serialized spelling, as the reports do.
#[derive(Debug)]
struct Counts<K>(Vec<(K, u64)>);

impl<K> Default for Counts<K> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<K: Copy + PartialEq + Serialize> Counts<K> {
    fn add(&mut self, key: K) {
        match self.0.iter_mut().find(|(counted, _)| *counted == key) {
            Some((_, count)) => *count += 1,
            None => self.0.push((key, 1)),
        }
    }

    /// The counts keyed by serialized spelling (`Function`, `Public`,
    /// `Rust`), each with its display label.
    fn keyed(&self, label: impl Fn(K) -> String) -> BTreeMap<String, (String, u64)> {
        self.0.iter().map(|&(key, count)| (serde_key(&key), (label(key), count))).collect()
    }

    fn json(&self) -> BTreeMap<String, u64> {
        self.0.iter().map(|&(key, count)| (serde_key(&key), count)).collect()
    }
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
        self.by_kind.add(record.kind);
        self.by_visibility.add(record.visibility.level);
        self.by_language.add(record.language);

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

    fn as_json(&self) -> StatsJson {
        StatsJson {
            by_kind: self.by_kind.json(),
            by_visibility: self.by_visibility.json(),
            by_language: self.by_language.json(),
            doc_coverage: CoverageJson {
                documented: self.doc_coverage.documented,
                total: self.doc_coverage.total,
                unavailable: self.doc_coverage.unavailable,
                ratio: self.doc_coverage.ratio(),
            },
            synthetic_exports: self.synthetic_exports,
        }
    }

    /// Writes overall and sorted package stats, with `errors` last.
    pub(crate) fn write_json(&self, out: &mut dyn Write, root_dir: &str, package_root_dir: &str, groups: &[Group], errors: &[ReportError]) -> io::Result<()> {
        let envelope = Envelope {
            report_schema_version: REPORT_SCHEMA_VERSION,
            root_dir,
            package_root_dir,
            stats: self.as_json(),
            groups: groups.iter().map(|group| GroupJson {
                package: group.package.as_ref(),
                stats: group.stats.as_json(),
            }).collect(),
            errors,
        };
        serde_json::to_writer(&mut *out, &envelope)?;
        out.write_all(b"\n")?;
        out.flush()
    }

    /// Package headings are literal text; the counters use the same rendering
    /// and width policy as the overall section, including empty groups.
    pub(crate) fn write_grouped_terminal(&self, out: &mut dyn Write, term: &Terminal, plain: bool, groups: &[Group]) -> io::Result<()> {
        write_heading(out, term, plain, "All selected declarations")?;
        self.write_terminal(out, term, plain)?;
        for group in groups {
            writeln!(out)?;
            let heading = group.package.as_ref().map_or_else(|| "Unassigned".to_string(), |package| {
                format!("{} ({})", package.name, package.path)
            });
            write_heading(out, term, plain, &heading)?;
            group.stats.write_terminal(out, term, plain)?;
        }
        out.flush()
    }

    /// Writes the three tables (by kind, visibility and language) and the
    /// doc-coverage line. `term` decides styling and width; a table too wide
    /// for it keeps its natural width (see [`table_terminal`]).
    pub(crate) fn write_terminal(&self, out: &mut dyn Write, term: &Terminal, plain: bool) -> io::Result<()> {
        for (title, counts) in [
            ("Kind", self.by_kind.keyed(|kind| SymbolKind::from(kind).to_string())),
            ("Visibility", self.by_visibility.keyed(visibility_label)),
            ("Language", self.by_language.keyed(|language| language.to_string())),
        ] {
            let rows: Vec<Vec<TableCellContent>> = counts
                .into_values()
                .map(|(label, count)| vec![label.into(), (count as i64).into()])
                .collect();
            let table = Table::new()
                .with_columns(vec![
                    TableColumn::new(title),
                    TableColumn::new("Count").with_type(ColumnType::Integer),
                ])
                .with_data(rows);
            write_rendered(out, &table.render(&table_terminal(&table, term)), plain)?;
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
    package_root_dir: &'a str,
    stats: StatsJson,
    groups: Vec<GroupJson<'a>>,
    errors: &'a [ReportError],
}

#[derive(Serialize)]
struct GroupJson<'a> {
    package: Option<&'a PackageIdentity>,
    stats: StatsJson,
}

#[derive(Serialize)]
struct StatsJson {
    by_kind: BTreeMap<String, u64>,
    by_visibility: BTreeMap<String, u64>,
    by_language: BTreeMap<String, u64>,
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

fn write_heading(out: &mut dyn Write, term: &Terminal, plain: bool, heading: &str) -> io::Result<()> {
    let heading = Prose::new(format!("<b>{}</b>", literal(heading)))
        .with_word_wrap(WordWrap::None);
    write_rendered(out, &heading.render(term), plain)?;
    writeln!(out)
}

/// Plain output strips escapes independently of detected color depth.
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
            lines: None,
            cyclomatic: None,
        });
        record
    }

    fn text(stats: &Stats) -> String {
        let mut out = Vec::new();
        stats.write_json(&mut out, "/root", "/root", &[], &[]).expect("write");
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
    fn grouped_terminal_preserves_synthetic_and_unavailable_coverage() {
        let mut stats = Stats::default();
        let mut synthetic = record(tree_hugger::SymbolKind::Function, ProgrammingLanguage::Rust, Some(false));
        synthetic.visibility.is_synthetic = true;
        stats.add(&synthetic);
        stats.add(&record(tree_hugger::SymbolKind::Function, ProgrammingLanguage::Bash, None));
        let mut group_stats = Stats::default();
        group_stats.add(&synthetic);
        group_stats.add(&record(tree_hugger::SymbolKind::Function, ProgrammingLanguage::Bash, None));
        let groups = [Group {
            package: Some(PackageIdentity { name: "<b>name</b>".into(), path: "dir/<a href='evil'>path</a>\u{1b}\n".into() }),
            stats: group_stats,
        }, Group { package: None, stats: Stats::default() }];
        let mut out = Vec::new();
        stats.write_grouped_terminal(&mut out, &Terminal::new(), true, &groups).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("<b>name</b> (dir/<a href='evil'>path</a>\\u{1b}\\u{a})"), "{text}");
        assert_eq!(text.matches("Synthetic exports: 1").count(), 2);
        assert_eq!(text.matches("Doc coverage: 0 of 0 documented (n/a); 1 unavailable").count(), 2);
        assert_eq!(text.matches("Doc coverage: 0 of 0 documented (n/a); 0 unavailable").count(), 1);
        assert!(!text.contains('\u{1b}'));
        let mut out = Vec::new();
        stats.write_json(&mut out, "/scan", "/catalog", &groups, &[]).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(value["groups"][0]["stats"], value["stats"]);
        assert_eq!(value["groups"][0]["package"]["path"], groups[0].package.as_ref().unwrap().path);
        assert!(value["groups"][0]["stats"]["doc_coverage"]["ratio"].is_null());
    }

    /// Counted by value, the maps keep the serialized spelling as their keys
    /// and its order, and the tables label each row for display.
    #[test]
    fn counts_are_keyed_and_ordered_by_their_serialized_spelling() {
        let mut stats = Stats::default();
        for (kind, language) in [
            (tree_hugger::SymbolKind::Variable, ProgrammingLanguage::Rust),
            (tree_hugger::SymbolKind::Function, ProgrammingLanguage::Cpp),
            (tree_hugger::SymbolKind::Class, ProgrammingLanguage::CSharp),
            (tree_hugger::SymbolKind::Function, ProgrammingLanguage::Cpp),
        ] {
            stats.add(&record(kind, language, None));
        }
        let mut public = record(tree_hugger::SymbolKind::Function, ProgrammingLanguage::Rust, None);
        public.visibility.level = VisibilityLevel::Public;
        stats.add(&public);

        let raw = text(&stats);
        assert!(
            raw.contains(
                "\"by_kind\":{\"Class\":1,\"Function\":3,\"Variable\":1},\
                 \"by_visibility\":{\"Public\":1,\"Unknown\":4},\
                 \"by_language\":{\"CSharp\":1,\"Cpp\":2,\"Rust\":2}"
            ),
            "{raw}"
        );

        let mut out = Vec::new();
        stats.write_terminal(&mut out, &Terminal::new(), true).expect("write");
        let table = String::from_utf8(out).expect("utf-8");
        let position = |label: &str| table.find(label).unwrap_or_else(|| panic!("{label}: {table}"));
        assert!(position("class") < position("function") && position("function") < position("variable"), "{table}");
        assert!(position("C#") < position("C++") && position("C++") < position("Rust"), "{table}");
        assert!(position("public") < position("unknown"), "{table}");
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
