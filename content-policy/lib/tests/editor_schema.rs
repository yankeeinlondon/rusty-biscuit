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
