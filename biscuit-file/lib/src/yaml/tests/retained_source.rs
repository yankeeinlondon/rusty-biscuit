//! Tests for retained source ownership (acceptance C-6, decision D5).
//!
//! Every parsing constructor retains the source text read at construction;
//! path-backed values must serve diagnostics from that retained copy even
//! after the file changes on disk (no TOCTOU reread). `from_value` has no
//! authored source and reports none.

use std::io::Write;

use super::super::types::{Yaml, YamlSource};

#[test]
fn test_path_backed_yaml_retains_source() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    write!(file, "name: retained\ncount: 42").unwrap();

    let yaml = Yaml::new(file.path()).unwrap();
    assert_eq!(yaml.source_text(), Some("name: retained\ncount: 42"));
    assert!(matches!(yaml.source(), YamlSource::Path(_)));
}

#[test]
fn test_path_backed_source_survives_file_change_on_disk() {
    // TOCTOU guard: diagnostics read the copy captured at construction,
    // never a second filesystem read.
    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(file, "name: original").unwrap();
    file.flush().unwrap();

    let yaml = Yaml::new(file.path()).unwrap();

    // Overwrite the file after construction.
    std::fs::write(file.path(), "name: replaced\n").unwrap();

    assert_eq!(yaml.source_text(), Some("name: original\n"));
    assert_eq!(yaml.value()["name"], "original");
}

#[test]
fn test_from_str_retains_input_text() {
    let yaml = Yaml::from_str("name: text\n").unwrap();
    assert_eq!(yaml.source_text(), Some("name: text\n"));
    assert!(matches!(yaml.source(), YamlSource::Text(_)));
}

#[test]
fn test_from_bytes_retains_input_text() {
    let yaml = Yaml::from_bytes(b"name: bytes\r\n").unwrap();
    assert_eq!(yaml.source_text(), Some("name: bytes\r\n"));
    assert!(matches!(yaml.source(), YamlSource::Bytes(_)));
}

#[test]
fn test_from_bytes_retains_multibyte_text() {
    let yaml = Yaml::from_bytes("key: 日本語\n".as_bytes()).unwrap();
    assert_eq!(yaml.source_text(), Some("key: 日本語\n"));
}

#[test]
fn test_from_value_has_no_source() {
    let mut mapping = serde_yaml_ng::Mapping::new();
    mapping.insert(
        serde_yaml_ng::Value::String("key".to_string()),
        serde_yaml_ng::Value::String("value".to_string()),
    );
    let yaml = Yaml::from_value(serde_yaml_ng::Value::Mapping(mapping));

    assert_eq!(yaml.source_text(), None);
    // The public YamlSource view is unchanged (Text with empty content).
    assert!(matches!(yaml.source(), YamlSource::Text(_)));
}

#[test]
fn test_from_value_via_from_trait_has_no_source() {
    let yaml: Yaml = serde_yaml_ng::Value::Null.into();
    assert_eq!(yaml.source_text(), None);
}

#[test]
fn test_retained_source_of_empty_input() {
    let yaml = Yaml::from_str("").unwrap();
    assert_eq!(yaml.source_text(), Some(""));
}

#[test]
fn test_clone_preserves_retained_source() {
    let yaml = Yaml::from_str("name: clone\n").unwrap();
    let cloned = yaml.clone();
    assert_eq!(cloned.source_text(), yaml.source_text());
}
