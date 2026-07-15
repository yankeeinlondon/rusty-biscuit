use super::*;

/// The variant-derived accessor preserves the historical wired set and
/// order (== `Provider` discriminant order): the retired `PROVIDER_SLUGS`
/// const's original seven, plus kilo, pi, and antigravity (each graduated
/// to a wired `Provider`).
#[test]
fn provider_slugs_match_the_wired_set_in_order() {
    assert_eq!(
        provider_slugs(),
        [
            "claude", "codex", "gemini", "goose", "kimi", "opencode", "qwen", "kilo", "pi",
            "antigravity"
        ]
    );
}

#[test]
fn env_var_ident_accepts_bare_identifiers_only() {
    assert!(is_env_var_ident("ANTHROPIC_MODEL"));
    assert!(is_env_var_ident("CLAUDE_CODE_SUBAGENT_MODEL"));
    assert!(!is_env_var_ident("A / B"));
    assert!(!is_env_var_ident("lowercase"));
    assert!(!is_env_var_ident(""));
    assert!(!is_env_var_ident("1ABC"));
}

#[test]
fn bare_flag_token_accepts_flags_and_rejects_annotations() {
    assert!(is_bare_flag_token("--model"));
    assert!(is_bare_flag_token("-m"));
    assert!(is_bare_flag_token("--prompt-interactive"));
    assert!(!is_bare_flag_token("--model / -m"));
    assert!(!is_bare_flag_token("--model, -m"));
    assert!(!is_bare_flag_token("--model  (goose run)"));
    assert!(!is_bare_flag_token("--output-format json for live wrapping"));
    assert!(!is_bare_flag_token("GOOSE_MODE=approve"));
    assert!(!is_bare_flag_token("no --json for live parsing"));
    assert!(!is_bare_flag_token(""));
}

#[test]
fn bare_config_path_rejects_annotations_and_placeholders() {
    assert!(is_bare_config_path("~/.claude/settings.json"));
    assert!(is_bare_config_path(".claude/settings.local.json"));
    assert!(is_bare_config_path("~/.qwen/debug/*.txt"));
    assert!(!is_bare_config_path(
        "$CODEX_HOME/config.toml; default /Users/<user>/.codex/config.toml"
    ));
    assert!(!is_bare_config_path("/Users/<name>/.kimi-code/config.toml"));
    assert!(!is_bare_config_path("AGENTS.override.md, AGENTS.md"));
    assert!(!is_bare_config_path("a.md or b.md"));
    assert!(!is_bare_config_path("$CODEX_HOME/auth.json"));
}

#[test]
fn model_catalog_source_variant_maps_snake_members() {
    assert_eq!(
        model_catalog_source_variant("model_catalog_source", "none").unwrap(),
        "None"
    );
    // `static` retired with the ModelCatalogSource::Static variant.
    assert!(matches!(
        model_catalog_source_variant("model_catalog_source", "static"),
        Err(GenError::UnmappableValue { .. })
    ));
    assert!(matches!(
        model_catalog_source_variant("model_catalog_source", "telepathic"),
        Err(GenError::UnmappableValue { .. })
    ));
}

/// `shell_command` as a bare member string must fail loudly: the
/// variant carries `program`/`args` and only the object form can
/// supply them.
#[test]
fn model_catalog_source_variant_rejects_bare_shell_command() {
    assert!(matches!(
        model_catalog_source_variant("model_catalog_source", "shell_command"),
        Err(GenError::UnmappableValue { .. })
    ));
}
