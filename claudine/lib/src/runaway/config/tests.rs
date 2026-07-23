use super::*;
use crate::provider::Provider;
use crate::runaway::patterns::PatternKind;

// -------------------------------------------------------------------------
// ExitExpressionEntry deserialization
// -------------------------------------------------------------------------

#[test]
fn entry_accepts_single_pattern() {
    let entry: ExitExpressionEntry = serde_json::from_value(serde_json::json!({
        "pattern": "STOP."
    }))
    .unwrap();
    assert_eq!(entry.patterns, vec!["STOP.".to_string()]);
    assert_eq!(entry.kind, PatternKind::Literal);
    assert!(!entry.ignore_case);
    assert!(entry.scope.is_none());
}

#[test]
fn entry_accepts_patterns_array_sharing_scope() {
    let entry: ExitExpressionEntry = serde_json::from_value(serde_json::json!({
        "patterns": ["STOP.", "Bye."],
        "kind": "literal",
        "scope": "opencode"
    }))
    .unwrap();
    assert_eq!(entry.patterns, vec!["STOP.", "Bye."]);
    assert_eq!(entry.scope.as_deref(), Some("opencode"));
}

#[test]
fn entry_regex_kind_with_ignore_case_ignored() {
    let entry: ExitExpressionEntry = serde_json::from_value(serde_json::json!({
        "pattern": "(?i)stop",
        "kind": "regex",
        "ignore_case": true
    }))
    .unwrap();
    assert_eq!(entry.kind, PatternKind::Regex);
    // Stored as-is — the detector's compile path ignores
    // `ignore_case` when kind == Regex.
    assert!(entry.ignore_case);
}

#[test]
fn entry_rejects_both_pattern_and_patterns() {
    let result = serde_json::from_value::<ExitExpressionEntry>(serde_json::json!({
        "pattern": "a",
        "patterns": ["b"]
    }));
    assert!(result.is_err());
}

#[test]
fn entry_rejects_neither_pattern_nor_patterns() {
    let result = serde_json::from_value::<ExitExpressionEntry>(serde_json::json!({
        "kind": "literal"
    }));
    assert!(result.is_err());
}

#[test]
fn entry_rejects_unknown_field() {
    let result = serde_json::from_value::<ExitExpressionEntry>(serde_json::json!({
        "pattern": "STOP.",
        "posture": "strict"
    }));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("posture"), "error: {msg}");
}

#[test]
fn entry_to_input_preserves_all_fields() {
    let entry = ExitExpressionEntry {
        patterns: vec!["a".to_string(), "b".to_string()],
        kind: PatternKind::Regex,
        ignore_case: true,
        scope: Some("opencode/k2".to_string()),
    };
    let input = entry.to_input();
    assert_eq!(input.patterns, vec!["a", "b"]);
    assert_eq!(input.kind, PatternKind::Regex);
    assert!(input.ignore_case);
    assert_eq!(input.scope.as_deref(), Some("opencode/k2"));
}

// -------------------------------------------------------------------------
// ExitExpressionsValue — array / object layer deserialization (E1)
// -------------------------------------------------------------------------

#[test]
fn layer_array_shorthand_has_no_explicit_mode() {
    let value: ExitExpressionsValue = serde_json::from_value(serde_json::json!([
        { "pattern": "a" },
        { "pattern": "b" }
    ]))
    .unwrap();
    assert!(value.explicit_mode().is_none());
    assert_eq!(value.rules().len(), 2);
}

#[test]
fn layer_object_form_carries_explicit_mode() {
    let value: ExitExpressionsValue = serde_json::from_value(serde_json::json!({
        "mode": "merge",
        "rules": [{ "pattern": "a" }]
    }))
    .unwrap();
    assert_eq!(value.explicit_mode(), Some(LayerMode::Merge));
    assert_eq!(value.rules().len(), 1);
}

#[test]
fn layer_object_defaults_to_override_mode() {
    let value: ExitExpressionsValue =
        serde_json::from_value(serde_json::json!({ "rules": [] })).unwrap();
    assert_eq!(value.explicit_mode(), Some(LayerMode::Override));
}

#[test]
fn layer_object_rejects_unknown_field() {
    let result = serde_json::from_value::<ExitExpressionsValue>(serde_json::json!({
        "mode": "merge",
        "rules": [],
        "extra": true
    }));
    assert!(result.is_err());
}

// -------------------------------------------------------------------------
// ScopeSelector + parse_scope (E2)
// -------------------------------------------------------------------------

#[test]
fn scope_absent_is_global() {
    assert_eq!(ScopeSelector::parse("").unwrap(), ScopeSelector::Global);
    assert_eq!(ScopeSelector::parse("   ").unwrap(), ScopeSelector::Global);
}

#[test]
fn scope_agent_only_matches_any_model_under_provider() {
    let sel = ScopeSelector::parse("opencode").unwrap();
    assert_eq!(sel, ScopeSelector::Agent(Provider::OpenCode));
    assert!(sel.matches(Provider::OpenCode, "anything"));
    assert!(sel.matches(Provider::OpenCode, "kimi-for-coding/k2p7"));
    assert!(!sel.matches(Provider::Claude, "anything"));
}

#[test]
fn scope_agent_plus_model_matches_exactly() {
    let sel = ScopeSelector::parse("opencode/kimi-for-coding/k2p7").unwrap();
    assert_eq!(
        sel,
        ScopeSelector::AgentModel(Provider::OpenCode, "kimi-for-coding/k2p7".to_string())
    );
    // Model keeps its inner `/` (split is on the FIRST `/`).
    assert!(sel.matches(Provider::OpenCode, "kimi-for-coding/k2p7"));
    assert!(!sel.matches(Provider::OpenCode, "other-model"));
    assert!(!sel.matches(Provider::Claude, "kimi-for-coding/k2p7"));
}

#[test]
fn scope_accepts_alias_forms_of_provider() {
    // `opencode`, `open_code`, `open-code` all resolve to OpenCode.
    assert_eq!(
        ScopeSelector::parse("opencode").unwrap(),
        ScopeSelector::Agent(Provider::OpenCode)
    );
    assert_eq!(
        ScopeSelector::parse("open_code").unwrap(),
        ScopeSelector::Agent(Provider::OpenCode)
    );
    assert_eq!(
        ScopeSelector::parse("open-code").unwrap(),
        ScopeSelector::Agent(Provider::OpenCode)
    );
}

#[test]
fn scope_trailing_slash_treated_as_agent_only() {
    // `"opencode/"` → Agent(OpenCode); empty model segment means any model.
    let sel = ScopeSelector::parse("opencode/").unwrap();
    assert_eq!(sel, ScopeSelector::Agent(Provider::OpenCode));
}

#[test]
fn scope_unknown_agent_rejected() {
    let err = ScopeSelector::parse("nonsense/k2").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("nonsense"), "error: {msg}");
}

#[test]
fn scope_global_matches_everything() {
    let sel = ScopeSelector::Global;
    for provider in [Provider::Claude, Provider::OpenCode, Provider::KimiCode] {
        assert!(sel.matches(provider, "any-model"));
    }
}

#[test]
fn parse_scope_returns_raw_segments() {
    assert_eq!(parse_scope(""), (None, None));
    assert_eq!(parse_scope("opencode"), (Some("opencode"), None));
    assert_eq!(
        parse_scope("opencode/kimi-for-coding/k2p7"),
        (Some("opencode"), Some("kimi-for-coding/k2p7"))
    );
}

// -------------------------------------------------------------------------
// resolve_exit_expressions — three-layer pipeline (E1)
// -------------------------------------------------------------------------

fn entry(pattern: &str) -> ExitExpressionEntry {
    ExitExpressionEntry {
        patterns: vec![pattern.to_string()],
        kind: PatternKind::Literal,
        ignore_case: false,
        scope: None,
    }
}

fn patterns_of(entries: &[ExitExpressionEntry]) -> Vec<&str> {
    entries
        .iter()
        .flat_map(|e| e.patterns.iter().map(String::as_str))
        .collect()
}

fn array(entries: &[ExitExpressionEntry]) -> ExitExpressionsValue {
    ExitExpressionsValue::Array(entries.to_vec())
}

fn object(mode: LayerMode, entries: &[ExitExpressionEntry]) -> ExitExpressionsValue {
    ExitExpressionsValue::Object(ExitExpressionsLayer {
        mode,
        rules: entries.to_vec(),
    })
}

#[test]
fn resolve_with_only_user_returns_user_rules() {
    let user = array(&[entry("a"), entry("b")]);
    let resolved = resolve_exit_expressions(Some(&user), None, None);
    assert_eq!(patterns_of(&resolved), vec!["a", "b"]);
}

#[test]
fn resolve_repo_default_override_replaces_user() {
    let user = array(&[entry("a"), entry("b")]);
    let repo = array(&[entry("c")]);
    let resolved = resolve_exit_expressions(Some(&user), Some(&repo), None);
    // repo present, array form → default override → effective = repo only.
    assert_eq!(patterns_of(&resolved), vec!["c"]);
}

#[test]
fn resolve_repo_explicit_merge_adds_to_user() {
    let user = array(&[entry("a")]);
    let repo = object(LayerMode::Merge, &[entry("b")]);
    let resolved = resolve_exit_expressions(Some(&user), Some(&repo), None);
    assert_eq!(patterns_of(&resolved), vec!["a", "b"]);
}

#[test]
fn resolve_frontmatter_default_merge_adds_on_top_of_repo() {
    let user = array(&[entry("a")]);
    let repo = array(&[entry("b")]); // default override → effective = [b]
    let fm = array(&[entry("c")]); // default merge → effective = [b, c]
    let resolved = resolve_exit_expressions(Some(&user), Some(&repo), Some(&fm));
    assert_eq!(patterns_of(&resolved), vec!["b", "c"]);
}

#[test]
fn resolve_frontmatter_explicit_override_replaces_all() {
    let user = array(&[entry("a"), entry("b")]);
    let repo = object(LayerMode::Merge, &[entry("c")]); // effective = [a, b, c]
    let fm = object(LayerMode::Override, &[entry("d")]); // effective = [d]
    let resolved = resolve_exit_expressions(Some(&user), Some(&repo), Some(&fm));
    assert_eq!(patterns_of(&resolved), vec!["d"]);
}

#[test]
fn resolve_skips_absent_layers() {
    // No user, no repo, no frontmatter → empty set.
    assert!(resolve_exit_expressions(None, None, None).is_empty());

    // No user; repo present → repo becomes the base.
    let repo = array(&[entry("x")]);
    let resolved = resolve_exit_expressions(None, Some(&repo), None);
    assert_eq!(patterns_of(&resolved), vec!["x"]);
}

#[test]
fn resolve_frontmatter_only() {
    let user = array(&[entry("a")]);
    let fm = array(&[entry("z")]); // default merge → effective = [a, z]
    let resolved = resolve_exit_expressions(Some(&user), None, Some(&fm));
    assert_eq!(patterns_of(&resolved), vec!["a", "z"]);
}

// -------------------------------------------------------------------------
// extract_frontmatter_exit_expressions
// -------------------------------------------------------------------------

#[test]
fn frontmatter_extract_returns_none_when_absent() {
    let fm = Map::new();
    assert!(extract_frontmatter_exit_expressions(&fm).unwrap().is_none());
}

#[test]
fn frontmatter_extract_returns_array_form() {
    let mut fm = Map::new();
    fm.insert(
        "exit_expressions".to_string(),
        serde_json::json!([{ "pattern": "STOP." }]),
    );
    let extracted = extract_frontmatter_exit_expressions(&fm).unwrap().unwrap();
    assert!(extracted.explicit_mode().is_none());
    assert_eq!(extracted.rules().len(), 1);
}

#[test]
fn frontmatter_extract_returns_object_form() {
    let mut fm = Map::new();
    fm.insert(
        "exit_expressions".to_string(),
        serde_json::json!({ "mode": "override", "rules": [{ "pattern": "x" }] }),
    );
    let extracted = extract_frontmatter_exit_expressions(&fm).unwrap().unwrap();
    assert_eq!(extracted.explicit_mode(), Some(LayerMode::Override));
}

#[test]
fn frontmatter_extract_errors_on_invalid_shape() {
    let mut fm = Map::new();
    fm.insert(
        "exit_expressions".to_string(),
        serde_json::json!({ "mode": "tuesday", "rules": [] }),
    );
    assert!(extract_frontmatter_exit_expressions(&fm).is_err());
}

// -------------------------------------------------------------------------
// GuardSettings — defaults + last-writer resolution
// -------------------------------------------------------------------------

#[test]
fn guard_settings_default_matches_spec_constants() {
    let gs = GuardSettings::default();
    assert!(gs.repetition.enabled);
    assert_eq!(gs.repetition.max_repeats, MAX_REPETITION_ALLOWED);
    assert_eq!(gs.repetition.max_cycle_length, MAX_CYCLE_LENGTH);
    assert!(gs.volume.enabled);
    assert_eq!(gs.volume.max_lines, VOLUME_LINES);
    assert_eq!(gs.volume.max_bytes, VOLUME_BYTES);
}

#[test]
fn guard_settings_round_trip() {
    let gs = GuardSettings {
        repetition: RepetitionGuardSettings {
            enabled: false,
            max_repeats: 12,
            max_cycle_length: 8,
        },
        volume: VolumeGuardSettings {
            enabled: true,
            max_lines: 1000,
            max_bytes: 1024 * 1024,
        },
    };
    let json = serde_json::to_value(&gs).unwrap();
    let back: GuardSettings = serde_json::from_value(json).unwrap();
    assert_eq!(back, gs);
}

#[test]
fn guard_settings_deserializes_partial_form() {
    // Only `repetition.enabled` set; everything else gets defaults.
    let gs: GuardSettings = serde_json::from_value(serde_json::json!({
        "repetition": { "enabled": false }
    }))
    .unwrap();
    assert!(!gs.repetition.enabled);
    assert_eq!(gs.repetition.max_repeats, MAX_REPETITION_ALLOWED);
    assert!(gs.volume.enabled);
    assert_eq!(gs.volume.max_bytes, VOLUME_BYTES);
}

#[test]
fn guard_settings_rejects_unknown_field() {
    let result = serde_json::from_value::<GuardSettings>(serde_json::json!({
        "posture": "strict"
    }));
    assert!(result.is_err());
}

#[test]
fn resolve_guard_settings_user_only_returns_user() {
    let user = GuardSettings::default();
    let resolved = resolve_guard_settings(&user, None, None);
    assert_eq!(resolved, user);
}

#[test]
fn resolve_guard_settings_repo_overrides_user() {
    let user = GuardSettings::default();
    let repo = GuardSettings {
        repetition: RepetitionGuardSettings {
            enabled: false,
            ..RepetitionGuardSettings::default()
        },
        ..GuardSettings::default()
    };
    let resolved = resolve_guard_settings(&user, Some(&repo), None);
    assert!(!resolved.repetition.enabled);
}

#[test]
fn resolve_guard_settings_frontmatter_overrides_repo() {
    let user = GuardSettings::default();
    let repo = GuardSettings {
        repetition: RepetitionGuardSettings {
            enabled: false,
            ..RepetitionGuardSettings::default()
        },
        ..GuardSettings::default()
    };
    let fm = GuardSettings::default(); // re-enabled at frontmatter
    let resolved = resolve_guard_settings(&user, Some(&repo), Some(&fm));
    assert!(resolved.repetition.enabled);
}

// -------------------------------------------------------------------------
// validate_exit_expressions
// -------------------------------------------------------------------------

#[test]
fn validate_accepts_clean_literal_entry() {
    let entries = vec![ExitExpressionEntry {
        patterns: vec!["STOP.".to_string()],
        kind: PatternKind::Literal,
        ignore_case: false,
        scope: Some("opencode".to_string()),
    }];
    assert!(validate_exit_expressions(&entries).is_ok());
}

#[test]
fn validate_accepts_clean_regex_entry() {
    let entries = vec![ExitExpressionEntry {
        patterns: vec![r"^(STOP|Bye)\.$".to_string()],
        kind: PatternKind::Regex,
        ignore_case: false,
        scope: Some("opencode/kimi-for-coding/k2p7".to_string()),
    }];
    assert!(validate_exit_expressions(&entries).is_ok());
}

#[test]
fn validate_rejects_empty_patterns() {
    let entries = vec![ExitExpressionEntry {
        patterns: vec![],
        kind: PatternKind::Literal,
        ignore_case: false,
        scope: None,
    }];
    let err = validate_exit_expressions(&entries).unwrap_err();
    assert!(err.to_string().contains("empty"), "error: {err}");
}

#[test]
fn validate_rejects_invalid_regex_at_load() {
    let entries = vec![ExitExpressionEntry {
        patterns: vec!["[".to_string()],
        kind: PatternKind::Regex,
        ignore_case: false,
        scope: None,
    }];
    assert!(validate_exit_expressions(&entries).is_err());
}

#[test]
fn validate_rejects_unknown_agent_in_scope() {
    let entries = vec![ExitExpressionEntry {
        patterns: vec!["x".to_string()],
        kind: PatternKind::Literal,
        ignore_case: false,
        scope: Some("nonsense/k2".to_string()),
    }];
    let err = validate_exit_expressions(&entries).unwrap_err();
    assert!(err.to_string().contains("nonsense"), "error: {err}");
}

#[test]
fn validate_accepts_all_three_scope_kinds() {
    let entries = vec![
        ExitExpressionEntry {
            patterns: vec!["global".to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: None,
        },
        ExitExpressionEntry {
            patterns: vec!["agent".to_string()],
            kind: PatternKind::Literal,
            ignore_case: false,
            scope: Some("claude".to_string()),
        },
        ExitExpressionEntry {
            patterns: vec![r"am$".to_string()],
            kind: PatternKind::Regex,
            ignore_case: false,
            scope: Some("claude/sonnet-4".to_string()),
        },
    ];
    assert!(validate_exit_expressions(&entries).is_ok());
}
