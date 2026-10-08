//! Shared record and cell handling for machine-readable exports.

use std::collections::HashSet;

use tree_hugger::SymbolRecord;

use crate::order::SortOrder;
use crate::render::neutralized;

/// Returns every selected record, including displayed class members.
pub(crate) fn flatten_selected<'a>(
    records: &'a [SymbolRecord],
    members: Option<&'a [Vec<SymbolRecord>]>,
    sort: SortOrder,
    include_synthetic: bool,
) -> Vec<&'a SymbolRecord> {
    let Some(members) = members else {
        let mut ordered: Vec<_> = records
            .iter()
            .filter(|record| include_synthetic || !record.visibility.is_synthetic)
            .collect();
        ordered.sort_by(|left, right| crate::order::compare(sort, ("", left), ("", right)));
        return ordered;
    };
    let mut emitted = HashSet::new();
    let mut flattened = Vec::new();
    let mut positions: Vec<_> = (0..records.len()).collect();
    positions.sort_by(|left, right| {
        crate::order::compare(sort, ("", &records[*left]), ("", &records[*right]))
    });
    for position in positions {
        let container = &records[position];
        let owned_members = &members[position];
        if (include_synthetic || !container.visibility.is_synthetic) && emitted.insert(container.id.clone()) {
            flattened.push(container);
        }
        let mut ordered = owned_members.iter().collect::<Vec<_>>();
        ordered.sort_by(|left, right| crate::order::compare(sort, ("", left), ("", right)));
        for member in ordered {
            if (include_synthetic || !member.visibility.is_synthetic) && emitted.insert(member.id.clone()) {
                flattened.push(member);
            }
        }
    }
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
    fn class_flattening_deduplicates_members_omits_summary_fields_and_sorts() {
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
        assert_eq!(names, ["Alpha", "beta", "shared", "Zebra"]);
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
