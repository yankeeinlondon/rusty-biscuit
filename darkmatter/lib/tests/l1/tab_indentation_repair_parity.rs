//! Biscuit File's `yaml.tab-indentation` repair and Darkmatter's
//! frontmatter tab normalization must read tab-indented frontmatter as the
//! same record, so a repaired document means what Darkmatter already read.

use biscuit_file::{YamlDiagnosticCode, analyze_yaml};
use darkmatter::markdown::Markdown;
use serde_json::Value;

/// Returns the tab-normalized record Darkmatter reads and the record the
/// repaired YAML parses to.
fn records(yaml: &str) -> (Value, Value) {
    let document = format!("---\n{yaml}---\n# Body\n");
    let markdown = Markdown::try_from_content(document).expect("Darkmatter reads the document");
    let darkmatter_record = serde_json::to_value(markdown.frontmatter().as_map()).unwrap();

    let analysis = analyze_yaml(yaml);
    assert!(
        !analysis.is_parseable(),
        "the tab-indented original must not parse"
    );
    let repair_codes: Vec<YamlDiagnosticCode> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| !diagnostic.repairs.is_empty())
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(
        !repair_codes.is_empty()
            && repair_codes
                .iter()
                .all(|code| *code == YamlDiagnosticCode::TabIndentation),
        "only the tab repair may edit: {repair_codes:?}"
    );
    let repaired = analysis.apply().source;
    let repaired_record: Value =
        biscuit_file::serde_yaml_ng::from_str(&repaired).expect("repaired YAML parses");

    (darkmatter_record, repaired_record)
}

#[test]
fn test_tab_repair_matches_darkmatter_for_block_scalar() {
    let (darkmatter_record, repaired_record) =
        records("prompt: |-\n\tLine one\n\t\tLine two\nlast_updated: 2026-02-27\n");

    assert_eq!(repaired_record, darkmatter_record);
    assert_eq!(repaired_record["prompt"], "Line one\n  Line two");
}

#[test]
fn test_tab_repair_matches_darkmatter_for_list_and_nested_mapping() {
    let (darkmatter_record, repaired_record) = records(
        "last_updated: 2026-02-27\nupdate_policy:\n\t- Duration(6mo)\nouter:\n\tinner:\n\t\tleaf: 1\n",
    );

    assert_eq!(repaired_record, darkmatter_record);
    assert_eq!(repaired_record["update_policy"][0], "Duration(6mo)");
}
