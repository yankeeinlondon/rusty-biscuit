//! `inspect_schema_target`: the format verdict, description, and authored
//! definition an editor previews for the file a `$schema` selected.

use std::path::Path;

use simplified_schema::resolve::{SchemaTargetFormat, SchemaTargetInspection, inspect_schema_target};
use simplified_schema::{NotSchemaReason, SchemaError};

fn inspect(name: &str, text: &str) -> SchemaTargetInspection {
    inspect_schema_target(Path::new(name), text.as_bytes())
}

fn lines(name: &str, text: &str) -> Vec<String> {
    let inspection = inspect(name, text);
    assert!(inspection.classification.is_ok(), "{:?}", inspection.classification);
    inspection.lines.expect("an accepted target has lines")
}

#[test]
fn a_block_mapping_previews_its_dedented_entries() {
    let text = "$schema:\n  title: string(required)\n  # kept\n  count: number\n";
    assert_eq!(lines("s.yaml", text), ["title: string(required)", "# kept", "count: number"]);
}

#[test]
fn a_block_sequence_previews_from_its_first_item_without_the_header_comment() {
    // The `feature-review.yaml` shape: a header comment, then items indented
    // four spaces under the key.
    let text = "# Feature review documents.\n$schema:\n    - description: string(required)\n      verdict: string\n    - status: string\n";
    assert_eq!(
        lines("feature-review.yaml", text),
        ["- description: string(required)", "  verdict: string", "- status: string"],
    );
}

#[test]
fn a_flow_mapping_and_scalar_values_start_flush() {
    assert_eq!(lines("s.yaml", "$schema: {title: string}\n"), ["{title: string}"]);
    assert_eq!(lines("s.yaml", "$schema: other.yaml\n"), ["other.yaml"]);
    assert_eq!(lines("s.yaml", "$schema: \"other.yaml\"\n"), ["\"other.yaml\""]);
}

#[test]
fn crlf_endings_are_normalized_and_a_final_newline_is_not_a_line() {
    let text = "$schema:\r\n  a: string\r\n  b: number\r\n";
    assert_eq!(lines("s.yaml", text), ["a: string", "b: number"]);
}

#[test]
fn ten_and_eleven_line_definitions_report_every_line() {
    let body = |count: usize| {
        let mut text = String::from("$schema:\n");
        for index in 0..count {
            text.push_str(&format!("  p{index}: string\n"));
        }
        text
    };
    assert_eq!(lines("s.yaml", &body(10)).len(), 10);
    let eleven = lines("s.yaml", &body(11));
    assert_eq!(eleven.len(), 11);
    assert_eq!(eleven[10], "p10: string");
}

#[test]
fn a_json_target_previews_its_whole_document() {
    let text = "{\r\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\r\n  \"type\": \"object\"\r\n}\r\n";
    let inspection = inspect("schema.JSON", text);
    assert_eq!(inspection.format, SchemaTargetFormat::Json);
    assert!(inspection.classification.is_ok(), "{:?}", inspection.classification);
    assert_eq!(
        inspection.lines.unwrap(),
        ["{", "  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",", "  \"type\": \"object\"", "}"],
    );
}

#[test]
fn rejected_targets_keep_the_loader_error_and_have_no_preview() {
    let types_only = inspect("lib.yaml", "kind: schema\ntypes:\n  Name: string\n");
    assert_eq!(types_only.format, SchemaTargetFormat::Yaml);
    assert!(
        matches!(types_only.classification, Err(SchemaError::NotASchemaDocument { reason: NotSchemaReason::TypesOnly, .. })),
        "{:?}",
        types_only.classification,
    );
    assert!(types_only.lines.is_none() && types_only.description.is_none());

    let no_dialect = inspect("raw.json", "{\"type\": \"object\", \"description\": \"x\"}");
    assert!(matches!(
        no_dialect.classification,
        Err(SchemaError::NotASchemaDocument { reason: NotSchemaReason::JsonMissingDialect, .. })
    ));
    assert!(no_dialect.lines.is_none() && no_dialect.description.is_none());

    let draft7 = inspect("raw.json", "{\"$schema\": \"http://json-schema.org/draft-07/schema#\"}");
    assert!(
        matches!(&draft7.classification, Err(SchemaError::UnsupportedJsonDialect { declared, .. }) if declared == "http://json-schema.org/draft-07/schema#")
    );

    let malformed = inspect("s.yaml", "$schema:\n  a: string\n  a: number\n");
    assert!(matches!(malformed.classification, Err(SchemaError::SchemaDocument { .. })));
    assert!(malformed.lines.is_none());

    let binary = inspect_schema_target(Path::new("s.yaml"), &[0xff, 0xfe, 0x00]);
    assert!(
        matches!(&binary.classification, Err(SchemaError::SchemaDocument { message, .. }) if message.starts_with("not UTF-8"))
    );
}

#[test]
fn description_is_kept_only_when_it_is_a_nonblank_string() {
    let yaml_description = |line: &str| inspect("s.yaml", &format!("{line}$schema:\n  a: string\n")).description;
    assert_eq!(yaml_description(""), None);
    assert_eq!(yaml_description("description: \"\"\n"), None);
    assert_eq!(yaml_description("description: \"   \"\n"), None);
    assert_eq!(yaml_description("description: Reviews of a feature.\n"), Some("Reviews of a feature.".into()));
    assert_eq!(
        inspect("s.yaml", "kind: schema\ndescription: Tagged café.\n$schema:\n  a: string\n").description,
        Some("Tagged café.".into()),
    );
    // A leading comment block is not a description.
    assert_eq!(inspect("s.yaml", "# Not this.\n$schema:\n  a: string\n").description, None);
    // `null` and a list are not strings: the envelope's definition rejects
    // the file.
    for line in ["description: null\n", "description: [a]\n"] {
        let inspection = inspect("s.yaml", &format!("{line}$schema:\n  a: string\n"));
        assert!(matches!(inspection.classification, Err(SchemaError::SchemaDocument { .. })), "{line}");
        assert_eq!(inspection.description, None);
    }
    // A number converts softly, as frontmatter does, and is read as text.
    assert_eq!(yaml_description("description: 3\n"), Some("3".into()));

    let json_description = |value: &str| {
        inspect(
            "s.json",
            &format!("{{\"$schema\": \"https://json-schema.org/draft/2020-12/schema\"{value}}}"),
        )
        .description
    };
    assert_eq!(json_description(""), None);
    assert_eq!(json_description(", \"description\": null"), None);
    assert_eq!(json_description(", \"description\": 3"), None);
    assert_eq!(json_description(", \"description\": \" \""), None);
    assert_eq!(json_description(", \"description\": \"JSON schema.\""), Some("JSON schema.".into()));
}
