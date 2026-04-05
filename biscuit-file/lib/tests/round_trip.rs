//! Round-trip conversion tests verifying data fidelity across formats.

use biscuit_file::{Json5, Toml, Yaml};

/// Helper to parse TOML, convert to JSON, convert back to TOML, and compare values.
fn toml_json_round_trip(input: &str) {
    let toml = Toml::from_str(input).expect("parse TOML");
    let json_value = toml.as_json_value().expect("TOML -> JSON");
    let json_str = serde_json::to_string_pretty(&json_value).expect("serialize JSON");

    // Parse the JSON back into a serde_json::Value and convert to TOML
    let roundtrip_json: serde_json::Value =
        serde_json::from_str(&json_str).expect("parse roundtrip JSON");
    let roundtrip_toml: toml::Value =
        serde_json::from_value(roundtrip_json).expect("JSON -> TOML value");
    let original_toml = toml.value().clone();

    assert_eq!(
        original_toml, roundtrip_toml,
        "TOML -> JSON -> TOML round trip should preserve values"
    );
}

#[test]
fn toml_json_round_trip_basic() {
    toml_json_round_trip(
        r#"
[package]
name = "test"
version = "1.0.0"
keywords = ["rust", "test"]

[dependencies]
serde = "1.0"
"#,
    );
}

#[test]
fn toml_json_round_trip_nested() {
    toml_json_round_trip(
        r#"
[server]
host = "localhost"
port = 8080

[server.tls]
enabled = true
cert = "/path/to/cert"
"#,
    );
}

#[test]
fn toml_json_round_trip_numbers() {
    toml_json_round_trip(
        r#"
integer = 42
negative = -17
float = 3.14
"#,
    );
}

#[test]
fn yaml_json_round_trip() {
    let input = "name: test\nversion: '1.0'\nitems:\n  - one\n  - two\n";
    let yaml = Yaml::from_str(input).expect("parse YAML");
    let json_value = yaml.as_json().expect("YAML -> JSON");
    let json_str = serde_json::to_string_pretty(&json_value).expect("serialize JSON");

    let roundtrip_json: serde_json::Value =
        serde_json::from_str(&json_str).expect("parse roundtrip JSON");
    let roundtrip_yaml = serde_yaml_ng::to_value(&roundtrip_json).expect("JSON -> YAML value");
    let original_yaml = yaml.value().clone();

    assert_eq!(
        original_yaml, roundtrip_yaml,
        "YAML -> JSON -> YAML round trip should preserve values"
    );
}

#[test]
fn json_yaml_json_round_trip() {
    let input = r#"{"name": "test", "count": 42, "nested": {"a": 1}}"#;
    let value: serde_json::Value = serde_json::from_str(input).expect("parse JSON");

    let yaml_str = serde_yaml_ng::to_string(&value).expect("JSON -> YAML string");
    let roundtrip: serde_json::Value =
        serde_yaml_ng::from_str(&yaml_str).expect("YAML string -> JSON");

    assert_eq!(value, roundtrip, "JSON -> YAML -> JSON round trip should preserve values");
}

#[test]
fn json5_json_round_trip() {
    let input = r#"{
        // comment
        name: 'test',
        count: 42,
        nested: {a: 1},
    }"#;
    let json5 = Json5::from_str(input).expect("parse JSON5");
    let json_value = json5.as_json_value().clone();
    let json_str = serde_json::to_string_pretty(&json_value).expect("serialize JSON");

    let roundtrip: serde_json::Value =
        serde_json::from_str(&json_str).expect("parse roundtrip JSON");

    assert_eq!(json_value, roundtrip, "JSON5 -> JSON -> JSON round trip should preserve values");
}
