//! "Quote the slot": an unquoted `{{ … }}` YAML value is reported as such,
//! at the authored value, both when YAML reads it as a flow mapping and when
//! the document does not decode; nothing else is.

use serde_yaml_ng::Value as YamlValue;
use simplified_schema::slots::{decode_error_slot, quote_the_slot_message, unquoted_slot_hint};
use simplified_schema::{SchemaError, parse_schema_declaration_with_source, parse_standalone_schema_document};

const QUOTE_THE_SLOT: &str = "quote the slot";

/// The text of the hint `unquoted_slot_hint(text, error_line)` returns.
fn hint_text(text: &str, error_line: Option<usize>) -> Option<&str> {
    unquoted_slot_hint(text, error_line).map(|span| &text[span])
}

#[test]
fn a_mapping_value_slot_is_found_without_its_trailing_comment() {
    let text = "$schema:\n  status: {{ load_data('./t.yaml') }}  # the statuses\n";
    assert_eq!(hint_text(text, None), Some("{{ load_data('./t.yaml') }}"));
}

#[test]
fn a_list_item_slot_is_found() {
    let text = "$schema:\n  status:\n    - string\n    - {{ x }}[]\n";
    assert_eq!(hint_text(text, None), Some("{{ x }}[]"));
}

#[test]
fn quoted_commented_block_and_literal_openers_are_not_slots() {
    for text in [
        "status: \"{{ x }}\"\n",
        "status: '{{ x }}'\n",
        "status: \"multi\n  line: {{ x }}\"\n",
        "# status: {{ x }}\n",
        "status: string # {{ x }}\n",
        "notes: |\n  status: {{ x }}\n  - {{ y }}\n",
        "notes: >-\n  status: {{ x }}\n",
        "status: {{{ x }}}\n",
        "status: {{!data:abc}}\n",
        "status: string\n",
    ] {
        assert_eq!(unquoted_slot_hint(text, None), None, "{text:?}");
    }
}

#[test]
fn a_slot_after_a_block_scalar_ends_is_found() {
    let text = "notes: |\n  status: {{ x }}\nstatus: {{ y }}\n";
    assert_eq!(hint_text(text, None), Some("{{ y }}"));
}

#[test]
fn crlf_line_endings_keep_the_value_span_and_line() {
    let text = "$schema:\r\n  name: string\r\n  status: {{ x }}(required)\r\n";
    assert_eq!(hint_text(text, Some(3)), Some("{{ x }}(required)"));
}

#[test]
fn an_error_line_matches_its_own_line_and_the_one_after_the_slot() {
    let text = "a: string\nstatus: {{ x }}\nb: string\nc: string\n";
    assert_eq!(hint_text(text, Some(2)), Some("{{ x }}"));
    assert_eq!(hint_text(text, Some(3)), Some("{{ x }}"));
    assert_eq!(hint_text(text, Some(1)), None);
    assert_eq!(hint_text(text, Some(4)), None);
}

#[test]
fn the_message_names_the_key_or_the_list_form() {
    let text = "$schema:\n  status: {{ x }}\n  tags:\n    - {{ y }}\n";
    let status = unquoted_slot_hint(text, Some(2)).expect("status slot");
    assert_eq!(quote_the_slot_message(text, &status), "quote the slot: write `status: \"{{ … }}\"`");
    let item = unquoted_slot_hint(text, Some(4)).expect("list slot");
    assert_eq!(quote_the_slot_message(text, &item), "quote the slot: write `- \"{{ … }}\"`");
}

#[test]
fn a_flow_mapping_slot_is_reported_at_the_authored_value() {
    // A leading line keeps the declaration away from offset zero.
    let document = "title: x\n$schema:\n  name: string\n  status: {{ x }}\n";
    let offset = document.find("  name").expect("declaration starts");
    let source = &document[offset..];
    let value: YamlValue = serde_yaml_ng::from_str(source).expect("YAML reads a flow mapping");

    let failure = parse_schema_declaration_with_source(&value, source, offset).expect_err("non-string key");

    assert!(failure.error.to_string().contains(QUOTE_THE_SLOT), "{}", failure.error);
    assert!(failure.error.to_string().contains("`status: \"{{ … }}\"`"), "{}", failure.error);
    let span = failure.authored_span.expect("located");
    assert_eq!(&document[span], "{{ x }}");
}

#[test]
fn a_schema_file_flow_mapping_slot_is_located_in_the_file() {
    for (text, slot, message) in [
        ("$schema:\n  name: string\n  status: {{ x }}\n", "{{ x }}", "`status: \"{{ … }}\"`"),
        ("$schema:\r\n  status:\r\n    - string\r\n    - {{ y }}  # c\r\n", "{{ y }}", "`- \"{{ … }}\"`"),
    ] {
        let error = parse_standalone_schema_document(text, "schema.yaml").expect_err("non-string key");
        let SchemaError::SchemaDocument { message: actual, span, .. } = &error else {
            panic!("expected a schema-document error, got {error:?}");
        };
        assert!(actual.contains(QUOTE_THE_SLOT) && actual.contains(message), "{text:?}: {actual}");
        assert_eq!(span.clone().map(|span| &text[span]), Some(slot), "{text:?}");
    }
}

#[test]
fn a_real_non_string_key_keeps_its_error() {
    for source in ["obj: { [a]: string }\n", "obj:\n  ? [x]\n  : string\n"] {
        let value: YamlValue = serde_yaml_ng::from_str(source).expect("valid YAML");
        let failure = parse_schema_declaration_with_source(&value, source, 0).expect_err("non-string key");
        let message = failure.error.to_string();
        assert!(message.contains("property names must be strings"), "{source:?}: {message}");
        assert!(!message.contains(QUOTE_THE_SLOT), "{source:?}: {message}");
    }
}

#[test]
fn a_schema_file_that_does_not_decode_reports_the_slot() {
    for text in ["$schema:\n  status: {{ x }}(required)\n", "$schema:\r\n  status: {{ x }}(required)\r\n"] {
        let error = parse_standalone_schema_document(text, "schema.yaml").expect_err("not YAML");
        let SchemaError::SchemaDocument { message, span, .. } = &error else {
            panic!("expected a schema-document error, got {error:?}");
        };
        assert_eq!(message, "quote the slot: write `status: \"{{ … }}\"`");
        assert_eq!(span.clone().map(|span| &text[span]), Some("{{ x }}(required)"));
    }
}

#[test]
fn an_unrelated_decode_failure_keeps_its_message() {
    for text in [
        "$schema:\n  status: [string\n",
        "$schema:\n  status: \"{{ x }}\"\n  name: : string\n",
        // A slot on the line before the error, which quoting does not fix.
        "$schema:\n  status: {{ x }}\n  name: : string\n",
    ] {
        let error = parse_standalone_schema_document(text, "schema.yaml").expect_err("not YAML");
        let SchemaError::SchemaDocument { message, span, .. } = &error else {
            panic!("expected a schema-document error, got {error:?}");
        };
        assert!(message.starts_with("not valid YAML: "), "{text:?}: {message}");
        assert_eq!(*span, None);
    }
}

#[test]
fn a_decode_error_is_blamed_on_a_slot_only_when_quoting_it_helps() {
    let broken_by_slot = "a: string\nstatus: {{ x }}(required)\nb: string\n";
    assert_eq!(decode_error_slot(broken_by_slot, 2).map(|span| &broken_by_slot[span]), Some("{{ x }}(required)"));

    let broken_below_slot = "a: string\nstatus: {{ x }}\nb: : string\n";
    assert_eq!(decode_error_slot(broken_below_slot, 3), None);
}

#[test]
fn a_schema_file_with_a_real_non_string_key_keeps_its_error() {
    for text in ["$schema:\n  obj: { [a]: string }\n", "$schema:\n  obj: { [a]: string }\n  status: {{ x }}\n"] {
        let error = parse_standalone_schema_document(text, "schema.yaml").expect_err("non-string key");
        let message = error.to_string();
        assert!(message.contains("key must be a string"), "{text:?}: {message}");
        assert!(!message.contains(QUOTE_THE_SLOT), "{text:?}: {message}");
    }
}
