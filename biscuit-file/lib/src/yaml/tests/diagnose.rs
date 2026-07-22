//! Tests for `Yaml::analyze` and its `diagnose` / `repair_candidates`
//! shorthands delegating to the source-first analyzer (decision D1):
//! parseable, source-backed values delegate; `from_value` has no authored
//! source and yields no analysis.

use super::super::types::Yaml;
use crate::yaml::YamlDiagnosticCode;

#[test]
fn test_diagnose_delegates_for_source_backed_values() {
    let yaml = Yaml::from_str("key: value  \n").unwrap();
    let diagnostics = yaml.diagnose();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, YamlDiagnosticCode::TrailingWhitespace);
    assert_eq!(diagnostics[0].span, 10..12);
}

#[test]
fn test_diagnose_clean_input_returns_nothing() {
    let yaml = Yaml::from_str("key: value\n").unwrap();
    assert!(yaml.diagnose().is_empty());
    assert!(yaml.repair_candidates().is_empty());
}

#[test]
fn test_repair_candidates_returns_proven_candidates() {
    let yaml = Yaml::from_str("key: [ 80,443 ]").unwrap();
    let candidates = yaml.repair_candidates();
    assert!(!candidates.is_empty());
    assert!(
        candidates
            .iter()
            .all(|repair| !repair.explanation.is_empty())
    );
}

#[test]
fn test_from_value_returns_empty_results() {
    let mut mapping = serde_yaml_ng::Mapping::new();
    mapping.insert(
        serde_yaml_ng::Value::String("key".to_string()),
        serde_yaml_ng::Value::String("value".to_string()),
    );
    let yaml = Yaml::from_value(serde_yaml_ng::Value::Mapping(mapping));
    assert!(yaml.analyze().is_none());
    assert!(yaml.diagnose().is_empty());
    assert!(yaml.repair_candidates().is_empty());
}

#[test]
fn test_single_analysis_matches_both_shorthands() {
    let yaml = Yaml::from_str("\u{FEFF}key :  [ 80,443 ]  \r\n").unwrap();
    let analysis = yaml.analyze().expect("source-backed value analyzes");

    assert_eq!(analysis.diagnostics(), yaml.diagnose().as_slice());
    assert_eq!(
        analysis.repairs().cloned().collect::<Vec<_>>(),
        yaml.repair_candidates()
    );
}

#[test]
fn test_analysis_retains_source_for_apply() {
    let yaml = Yaml::from_str("key: value  \n").unwrap();
    let analysis = yaml.analyze().expect("source-backed value analyzes");

    assert_eq!(analysis.source(), "key: value  \n");
    assert_eq!(analysis.apply().source, "key: value\n");
}

#[test]
fn test_path_backed_values_diagnose_retained_source() {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(file, "key: value  ").unwrap();
    let yaml = Yaml::new(file.path()).unwrap();
    let diagnostics = yaml.diagnose();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, YamlDiagnosticCode::TrailingWhitespace);
}
