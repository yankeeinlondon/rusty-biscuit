//! Shared record and cell handling for machine-readable exports.

use std::collections::HashSet;

use tree_hugger::SymbolRecord;

use crate::order::SortOrder;
use crate::render::neutralized;

/// Returns every selected record of one file, including displayed class
/// members, in one `--sort` order.
///
/// Class containers and their members are gathered first and sorted once
/// as a single sequence: a flat export has no container sections, so a
/// member sorts among every other record of the file. A member reached
/// through several containers is returned once.
pub(crate) fn flatten_selected<'a>(
    records: &'a [SymbolRecord],
    members: Option<&'a [Vec<SymbolRecord>]>,
    sort: SortOrder,
    include_synthetic: bool,
) -> Vec<&'a SymbolRecord> {
    let mut emitted = HashSet::new();
    let mut flattened: Vec<&SymbolRecord> = records
        .iter()
        .chain(members.into_iter().flatten().flatten())
        .filter(|record| include_synthetic || !record.visibility.is_synthetic)
        .filter(|record| emitted.insert(&record.id))
        .collect();
    flattened.sort_by(|left, right| crate::order::compare(sort, ("", left), ("", right)));
    flattened
}

/// Encodes one TSV cell. Escaped TSV characters are handled before the
/// shared control neutralizer so a tab remains `\\t`, not `\\u{9}`.
#[allow(dead_code)]
pub(crate) fn tsv_cell(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => encoded.push_str("\\\\"),
            '\t' => encoded.push_str("\\t"),
            '\r' => encoded.push_str("\\r"),
            '\n' => encoded.push_str("\\n"),
            control if control.is_control() => encoded.push_str(&neutralized(&control.to_string())),
            other => encoded.push(other),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classes::ClassSummary;
    use tree_hugger::{Capability, SourceFile, SymbolKindData, SymbolKindV2, SymbolQuery};
    use crate::order::SortOrder;

    fn records() -> (tempfile::TempDir, Vec<SymbolRecord>, Vec<Vec<SymbolRecord>>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("classes.java");
        std::fs::write(&path, "class Zebra { int field; void shared() {} }\nclass Alpha { void beta() {} }\n")
            .expect("write fixture");
        let query = SymbolQuery::new()
            .kinds(crate::classes::CONTAINER_KINDS.into_iter().chain(crate::classes::MEMBER_KINDS))
            .capabilities([Capability::Containment]);
        let records = SourceFile::read(&path).expect("read").query(&query).expect("query").index.symbols;
        let selection = crate::classes::ClassSelection::new(&crate::selection::Selection::default(), crate::classes::MemberFilter::default());
        let summaries = crate::classes::summarize(&records, &selection);
        let containers = summaries.iter().map(|summary| summary.container.clone()).collect();
        let members = summaries.iter().map(ClassSummary::displayed_members).collect();
        (dir, containers, members)
    }

    #[test]
    fn class_flattening_deduplicates_members_omits_summary_fields_and_sorts_the_whole_file() {
        let (_dir, records, mut members) = records();
        let has_summary_field = records.iter().any(|record| match &record.kind_data {
            SymbolKindData::Type(data) => !data.fields.is_empty(),
            _ => false,
        });
        assert!(has_summary_field, "the class fixture includes summary-only field metadata");
        let duplicate = members[0][0].clone();
        members[1].push(duplicate);

        let flattened = flatten_selected(&records, Some(&members), SortOrder::Name, true);
        let names: Vec<_> = flattened.iter().map(|record| record.identity.name.as_str()).collect();
        // One case-sensitive name order over containers and members alike.
        assert_eq!(names, ["Alpha", "Zebra", "beta", "shared"]);
        let by_kind = flatten_selected(&records, Some(&members), SortOrder::Kind, true);
        let names: Vec<_> = by_kind.iter().map(|record| record.identity.name.as_str()).collect();
        assert_eq!(names, ["Zebra", "Alpha", "shared", "beta"]);
        assert_eq!(flattened.iter().filter(|record| record.identity.name == "shared").count(), 1);
        assert!(!flattened.iter().any(|record| record.kind == SymbolKindV2::Field));
    }

    #[test]
    fn tsv_escape_precedes_control_neutralization() {
        assert_eq!(tsv_cell("a\tb"), "a\\tb");
        assert_eq!(tsv_cell("\\n\n"), "\\\\n\\n");
    }

    #[test]
    fn tsv_cell_round_trips_controls_and_escape_looking_text_in_one_pass() {
        fn decode(input: &str) -> String {
            let mut chars = input.chars();
            let mut output = String::new();
            while let Some(ch) = chars.next() {
                if ch != '\\' { output.push(ch); continue; }
                match chars.next().expect("escape suffix") {
                    '\\' => output.push('\\'), 't' => output.push('\t'), 'r' => output.push('\r'),
                    'n' => output.push('\n'), 'u' => {
                        assert_eq!(chars.next(), Some('{'));
                        let digits: String = chars.by_ref().take_while(|ch| *ch != '}').collect();
                        output.push(char::from_u32(u32::from_str_radix(&digits, 16).unwrap()).unwrap());
                    }
                    other => panic!("unknown escape {other:?}"),
                }
            }
            output
        }
        let corpus = ["tab\there", "line\nfeed", "carriage\rreturn", "slash\\n", "C1\u{0085}", "\u{1b}[31mansi\u{1b}[0m", "Unicode λ ' \" and spaces"];
        for original in corpus {
            assert_eq!(decode(&tsv_cell(original)), original);
        }
        assert_eq!(tsv_cell("\\n\n"), "\\\\n\\n");
    }

    #[test]
    fn outside_root_projection_keeps_fallback_and_record_identity() {
        let (_dir, records, _) = records();
        let record = &records[0];
        let identity = record.id.clone();
        let root = std::env::temp_dir().join("hug-export-root-that-does-not-exist");
        let outside = std::env::temp_dir().join("hug-export-vendor/other.rs");
        let projected = crate::report::portable_relative(&outside, &root);
        assert_eq!(projected, "../hug-export-vendor/other.rs");
        let mut projected_record = record.clone();
        crate::report::trim_record(&mut projected_record, &root);
        assert_eq!(projected_record.id, identity);
    }

    #[test]
    fn ctags_flattening_can_drop_synthetic_records_while_tsv_keeps_them() {
        let (_dir, mut records, members) = records();
        records[0].visibility.is_synthetic = true;
        assert!(flatten_selected(&records, Some(&members), SortOrder::File, true)
            .iter()
            .any(|record| record.visibility.is_synthetic));
        assert!(!flatten_selected(&records, Some(&members), SortOrder::File, false)
            .iter()
            .any(|record| record.visibility.is_synthetic));
    }
}
