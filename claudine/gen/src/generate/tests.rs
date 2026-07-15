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
