use super::status::{description_suffix, render_optional_line, render_required_line};
use super::*;
use claudine::composition::{PropertyState, PropertyStatus};
use std::path::PathBuf;

fn missing_with_shape(name: &str, shape: InteractiveShape) -> MissingProperty {
    MissingProperty {
        name: name.to_string(),
        type_label: Some("string".to_string()),
        description: None,
        interactive_shape: Some(shape),
    }
}

#[test]
fn collect_missing_values_rejects_unsupported_shape() {
    let missing = vec![MissingProperty {
        name: "config".to_string(),
        type_label: Some("object".to_string()),
        description: None,
        interactive_shape: None,
    }];
    let err = collect_missing_values(&missing).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    assert!(err.to_string().contains("config"));
}

#[test]
fn parse_number_accepts_integer() {
    let value = parse_number("42", false, None, None).unwrap();
    assert_eq!(value, serde_json::json!(42));
}

#[test]
fn parse_number_accepts_float() {
    let value = parse_number("3.14", false, None, None).unwrap();
    assert!(value.as_f64().is_some());
}

#[test]
fn parse_number_rejects_non_numeric() {
    assert!(parse_number("hello", false, None, None).is_err());
}

#[test]
fn parse_number_integer_mode_rejects_non_integer() {
    assert!(parse_number("3.14", true, None, None).is_err());
}

#[test]
fn parse_number_integer_mode_accepts_whole_float() {
    let value = parse_number("3.0", true, None, None).unwrap();
    assert!(value.as_i64().is_some() || value.as_f64().is_some());
}

#[test]
fn parse_number_rejects_empty() {
    assert!(parse_number("   ", false, None, None).is_err());
}

#[test]
fn parse_number_enforces_minimum() {
    assert!(parse_number("5", false, Some(10.0), None).is_err());
    assert_eq!(parse_number("10", false, Some(10.0), None).unwrap(), serde_json::json!(10));
}

#[test]
fn parse_number_enforces_maximum() {
    assert!(parse_number("15", false, None, Some(10.0)).is_err());
    assert_eq!(parse_number("10", false, None, Some(10.0)).unwrap(), serde_json::json!(10));
}

#[test]
fn format_label_combines_name_type_and_description() {
    let prop = missing_with_shape(
        "tier",
        InteractiveShape::Text {
            format: TextFormat::Plain,
            min_len: None,
            max_len: None,
        },
    );
    let label = format_label(&prop);
    assert!(label.starts_with("tier"));
    assert!(label.contains("string"));
}

#[test]
fn format_label_includes_description_when_present() {
    let prop = MissingProperty {
        name: "tier".to_string(),
        type_label: Some("enum(a|b)".to_string()),
        description: Some("the tier to use".to_string()),
        interactive_shape: Some(InteractiveShape::EnumOne {
            members: vec!["a".into(), "b".into()],
        }),
    };
    let label = format_label(&prop);
    assert!(label.contains("the tier to use"));
}

#[test]
fn render_required_line_includes_property_name_and_type() {
    let status = PropertyStatus {
        name: "title".to_string(),
        type_label: "string".to_string(),
        description: None,
        state: PropertyState::Missing,
    };
    let line = render_required_line(&status);
    assert!(line.contains("title"));
    assert!(line.contains("string"));
    assert!(line.contains("required"));
}

#[test]
fn render_required_line_marks_invalid_with_wrong_type_message() {
    let status = PropertyStatus {
        name: "count".to_string(),
        type_label: "number".to_string(),
        description: None,
        state: PropertyState::Invalid,
    };
    let line = render_required_line(&status);
    assert!(line.contains("wrong type"));
}

#[test]
fn render_optional_line_marks_valid_with_dim_styling() {
    let status = PropertyStatus {
        name: "description".to_string(),
        type_label: "string".to_string(),
        description: None,
        state: PropertyState::Valid,
    };
    let line = render_optional_line(&status);
    assert!(line.contains("<dim>"));
    assert!(line.contains("<green>"));
}

fn seed_spec_tree(root: &Path) {
    use std::fs;
    fs::create_dir_all(root.join(".git")).unwrap();
    let target = root.join("features/2026-06-30-style-everywhere/spec.md");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, "---\ntitle: Everywhere\n---\nbody\n").unwrap();
    let other = root.join("features/2026-06-01-other/spec.md");
    fs::create_dir_all(other.parent().unwrap()).unwrap();
    fs::write(&other, "---\ntitle: Other\n---\nbody\n").unwrap();
}

#[test]
fn provided_partial_candidates_filters_to_single_substring_match() {
    let tmp = tempfile::TempDir::new().unwrap();
    seed_spec_tree(tmp.path());

    let ctx = ScopeContext::discover_from(tmp.path());
    let patterns = vec!["**/*spec*.md".to_string()];
    let got = provided_partial_candidates(&patterns, "everywhere", &ctx);
    assert_eq!(got.len(), 1, "expected exactly one substring match: {got:?}");
    assert!(
        got[0].ends_with("2026-06-30-style-everywhere/spec.md"),
        "expected the style-everywhere spec, got {got:?}",
    );
}

#[test]
fn provided_partial_candidates_is_case_insensitive() {
    let tmp = tempfile::TempDir::new().unwrap();
    seed_spec_tree(tmp.path());

    let ctx = ScopeContext::discover_from(tmp.path());
    let patterns = vec!["**/*spec*.md".to_string()];
    let got = provided_partial_candidates(&patterns, "EVERYWHERE", &ctx);
    assert_eq!(got.len(), 1, "case-insensitive substring match: {got:?}");
}

#[test]
fn provided_partial_candidates_zero_matches_returns_empty() {
    let tmp = tempfile::TempDir::new().unwrap();
    seed_spec_tree(tmp.path());

    let ctx = ScopeContext::discover_from(tmp.path());
    let patterns = vec!["**/*spec*.md".to_string()];
    let got = provided_partial_candidates(&patterns, "no-such-partial", &ctx);
    assert!(got.is_empty(), "expected zero matches: {got:?}");
}

#[test]
fn merge_overrides_combines_base_and_collected() {
    let base = serde_json::json!({ "a": "1" });
    let mut collected = serde_json::Map::new();
    collected.insert("b".to_string(), serde_json::json!("2"));
    let merged = merge_overrides(Some(&base), collected);
    assert_eq!(merged.get("a"), Some(&serde_json::json!("1")));
    assert_eq!(merged.get("b"), Some(&serde_json::json!("2")));
}

#[test]
fn pre_validate_with_interactive_returns_missing_when_not_allowed() {
    use std::fs;
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    fs::write(
        &file,
        "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
    )
    .unwrap();
    let source = claudine::composition::resolve_composition_source(file.to_str().unwrap())
        .unwrap();

    // Build interactive options that DENY (default: all false).
    let interactive = InteractiveSchemaOptions::default();
    assert!(!interactive.allowed());

    let term = Terminal::default();
    let err = pre_validate_with_interactive_collection(&source, None, interactive, &term, None)
        .unwrap_err();
    assert!(
        matches!(err, CompositionError::MissingProperties { .. }),
        "expected MissingProperties when interactive not allowed: {err:?}"
    );
}

#[test]
fn pre_validate_with_interactive_returns_missing_for_file_property_when_not_allowed() {
    use std::fs;
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    fs::write(
        &file,
        "---\n$schema:\n  cover: 'file(required)'\n---\nbody\n",
    )
    .unwrap();
    let source = claudine::composition::resolve_composition_source(file.to_str().unwrap())
        .unwrap();

    // Non-TTY options deny prompting, so a missing `file` property must
    // still surface as MissingProperties rather than trying to drive a
    // chooser.
    let interactive = InteractiveSchemaOptions::default();
    assert!(!interactive.allowed());

    let term = Terminal::default();
    let err = pre_validate_with_interactive_collection(&source, None, interactive, &term, None)
        .unwrap_err();
    assert!(
        matches!(err, CompositionError::MissingProperties { .. }),
        "expected MissingProperties for file property when not interactive: {err:?}"
    );
}

#[test]
fn pre_validate_with_interactive_succeeds_when_overrides_supply_value() {
    use std::fs;
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    fs::write(
        &file,
        "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
    )
    .unwrap();
    let source = claudine::composition::resolve_composition_source(file.to_str().unwrap())
        .unwrap();

    let overrides = serde_json::json!({ "title": "Plan" });
    let term = Terminal::default();
    let pre = pre_validate_with_interactive_collection(
        &source,
        Some(&overrides),
        InteractiveSchemaOptions::default(),
        &term,
        None,
    )
    .unwrap();
    let fm = pre.set_overrides.unwrap();
    assert_eq!(
        fm.get("title").and_then(|v| v.as_str()),
        Some("Plan")
    );
}

#[test]
fn pre_validate_with_interactive_returns_unsupported_for_object_shape() {
    use std::fs;
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    fs::write(
        &file,
        "---\n$schema:\n  config: 'object(required)'\n---\nbody\n",
    )
    .unwrap();
    let source = claudine::composition::resolve_composition_source(file.to_str().unwrap())
        .unwrap();

    // Allow interactive so the helper attempts to enter the loop.
    let interactive = InteractiveSchemaOptions {
        prompt_for_missing: true,
        stdin_is_tty: true,
        stderr_is_tty: true,
        silent: false,
    };
    assert!(interactive.allowed());

    let term = Terminal::default();
    let err = pre_validate_with_interactive_collection(&source, None, interactive, &term, None)
        .unwrap_err();
    assert!(
        matches!(err, CompositionError::UnsupportedInteractiveSchema { .. }),
        "expected UnsupportedInteractiveSchema: {err:?}"
    );
}

#[test]
fn merge_overrides_collected_wins_over_base() {
    let base = serde_json::json!({ "k": "old" });
    let mut collected = serde_json::Map::new();
    collected.insert("k".to_string(), serde_json::json!("new"));
    let merged = merge_overrides(Some(&base), collected);
    assert_eq!(merged.get("k"), Some(&serde_json::json!("new")));
}

#[test]
fn description_suffix_handles_empty() {
    assert!(description_suffix(None).is_empty());
    assert!(description_suffix(Some("")).is_empty());
    assert!(description_suffix(Some("   ")).is_empty());
    assert!(description_suffix(Some("hello")).contains("hello"));
}

#[test]
fn path_label_portably_renders_windows_shaped_segments() {
    let root = PathBuf::from("repo");
    let path = root.join(r"assets\nested\cover.png");
    let ctx = ScopeContext {
        cwd: root.clone(),
        home: None,
        repo_info: None,
        git_root: Some(root),
    };

    assert_eq!(path_label(&path, &ctx), "assets/nested/cover.png");
}

#[test]
fn resolve_file_value_portably_renders_missing_windows_shaped_path() {
    let path = PathBuf::from(r"missing\schema\definitely-not-present.json");

    let error = resolve_file_value(&path).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert_eq!(
        error.to_string(),
        "file not found: missing/schema/definitely-not-present.json"
    );
}
