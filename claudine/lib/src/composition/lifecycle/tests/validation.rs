//! validation lifecycle tests.

use super::*;

#[test]
fn scan_rejects_pre_checks_removed_key() {
    let frontmatter = json!({
        "pre_checks": [{"command": "test"}],
        "start": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "pre_checks");
    assert!(replacement.contains("initialize"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_post_checks_removed_key() {
    let frontmatter = json!({
        "post_checks": [{"command": "test"}],
        "success": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "post_checks");
    assert!(replacement.contains("success"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_handle_removed_key() {
    let frontmatter = json!({
        "handle": "shell('fix')",
        "start": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "handle");
    assert!(replacement.contains("shell"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_deviate_removed_key() {
    let frontmatter = json!({
        "deviate": "shell('fix')",
        "start": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "deviate");
    assert!(replacement.contains("retry"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_handle_timeout_removed_key() {
    let frontmatter = json!({
        "handle_timeout": [{"action": "retry"}],
        "failure": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "handle_timeout");
    assert!(replacement.contains("blocked"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_handle_inline_body_unchanged_removed_key() {
    let frontmatter = json!({
        "handle_inline_body_unchanged": [{"action": "retry"}],
        "failure": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "handle_inline_body_unchanged");
    assert!(replacement.contains("failure"), "replacement: {replacement}");
}

#[test]
fn scan_allows_handle_underscore_without_suffix() {
    // `handle_` with no suffix is not one of the removed keys; only exact
    // `handle` and `handle_<non-empty>` are rejected.
    let frontmatter = json!({
        "handle_": { "message": "ok" }
    });
    assert!(scan_removed_validation_keys(&frontmatter).is_none());
}

#[test]
fn scan_returns_none_for_clean_frontmatter() {
    let frontmatter = json!({
        "start": { "message": "ok" }
    });
    assert!(scan_removed_validation_keys(&frontmatter).is_none());
}

#[test]
fn err_inside_array_literal_when_clause_is_rejected() {
    let fm = json!({
        "start": {
            "stack": [
                {"when": "length([err.msg]) > 0", "action": {"say": "leaked"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&fm, &config, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleErrNotAvailable { .. }),
        "got: {err:?}"
    );
}

#[test]
fn err_inside_object_literal_value_when_clause_is_rejected() {
    let fm = json!({
        "start": {
            "stack": [
                {"when": "{ reason: err.msg }", "action": {"say": "leaked"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&fm, &config, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleErrNotAvailable { .. }),
        "got: {err:?}"
    );
}

#[test]
fn err_span_inside_object_literal_key_is_text_not_a_read() {
    // An object key is authored text that Darkmatter never evaluates, so a
    // `{{ err.msg }}` spelled inside one reads nothing.
    let fm = json!({
        "start": {
            "stack": [
                {"when": "{ \"{{ err.msg }}\": 1 }", "action": {"say": "fine"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&fm, &config, dummy_path()).is_ok());
}

#[test]
fn doc_err_inside_container_literal_is_still_allowed() {
    let fm = json!({
        "start": {
            "stack": [
                {"when": "{ err: doc.err }", "action": {"say": "fine"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&fm, &config, dummy_path()).is_ok());
}

