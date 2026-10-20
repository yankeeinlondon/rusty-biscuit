//! The exported editor schema is the shipped schema file, and declares the
//! types callers reference by name.

use content_policy::EDITOR_SCHEMA;

#[test]
fn the_exported_schema_is_the_schema_file() {
    assert_eq!(EDITOR_SCHEMA, include_str!("../../schemas/content-policy.yaml"));
}

#[test]
fn the_exported_schema_declares_the_policy_types() {
    let lines: Vec<&str> = EDITOR_SCHEMA.lines().collect();
    assert!(lines.contains(&"kind: schema"));
    for declaration in ["  short_form:", "  long_form:", "  policy:"] {
        assert!(lines.contains(&declaration), "missing {declaration}");
    }
}

#[test]
fn the_exported_schema_requires_rule_and_names_its_own_types_bare() {
    let lines: Vec<&str> = EDITOR_SCHEMA.lines().collect();
    assert!(lines.contains(&"    rule: \"short_form(required)\""));
    assert!(lines.contains(&"    - short_form"));
    assert!(lines.contains(&"    - long_form"));
    assert!(
        !EDITOR_SCHEMA.contains("@./content-policy.yaml"),
        "same-file references are written as bare local names"
    );
    assert!(
        !EDITOR_SCHEMA.contains("is not `(required)`"),
        "the header no longer notes a missing-rule limitation"
    );
}
