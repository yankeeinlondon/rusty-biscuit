//! Registry-covers-all-fields guard.
//!
//! `SERIALIZED_PROVIDER_INFO_FIELDS` is the checked-in list of the
//! serialized `--describe` keys, in serialization order. Its twin lives in
//! the claudine lib (`provider::tests::serialized_field_list_matches_catalog`),
//! which asserts the same list against the real `ProviderInfo`
//! serialization — together they bind this guard to reality without the
//! generator linking the lib (bootstrap rule).
//!
//! Every serialized field must have exactly one registry entry OR a
//! documented exclusion (`EXCLUDED_SERIALIZED_FIELDS`); the behavior half
//! of `ProviderInfo` (trait objects, fn-pointer accessors) never appears
//! here because it is `#[serde(skip)]` by construction (field-source
//! matrix, exclusion 1).

use claudine_gen::{EXCLUDED_SERIALIZED_FIELDS, REGISTRY};

/// The serialized `ProviderInfo` field list (43 keys, describe order).
const SERIALIZED_PROVIDER_INFO_FIELDS: &[&str] = &[
    "provider",
    "display_name",
    "slug",
    "short_name",
    "binary",
    "agent_offset",
    "cli_aliases",
    "docs_url",
    "usage_dashboard_url",
    "sniff_binding",
    "supports_skills",
    "stream_protocol",
    "event_mapping",
    "resource_support",
    "session_log_paths",
    "config_paths",
    "memory_files",
    "output_formats",
    "entrypoints",
    "system_prompt",
    "yolo",
    "reasoning",
    "known_gaps",
    "acp",
    "prompt_arg_conventions",
    "expected_offerings",
    "offering_sources",
    "model_catalog_source",
    "model_env_vars",
    "cli_sensitive_axes",
    "repo_home_root_files",
    "resume",
    "model_cli_flag",
    "non_interactive_conflicting_flags",
    "billing_models",
    "cap_policies",
    "allowed_env_keys",
    "display_policy",
    "suppress_structured_stderr_on_success",
    "supports_interactive_inline_closure",
    "model_required_in_non_tty",
    "platform_kind",
    "unmapped_native_events",
];

#[test]
fn every_serialized_field_is_registered_or_excluded() {
    for field in SERIALIZED_PROVIDER_INFO_FIELDS {
        let registered = REGISTRY.iter().filter(|entry| entry.field == *field).count();
        let excluded = EXCLUDED_SERIALIZED_FIELDS
            .iter()
            .filter(|(name, _)| name == field)
            .count();
        assert_eq!(
            registered + excluded,
            1,
            "serialized field `{field}` must have exactly one registry entry or one \
             documented exclusion (found {registered} entries, {excluded} exclusions)"
        );
    }
}

#[test]
fn registry_has_no_entries_outside_the_serialized_field_list() {
    for entry in REGISTRY {
        assert!(
            SERIALIZED_PROVIDER_INFO_FIELDS.contains(&entry.field),
            "registry entry `{}` names no serialized ProviderInfo field — update the \
             checked-in field list (and its lib-side twin) alongside the struct",
            entry.field
        );
    }
}

#[test]
fn registry_order_matches_serialization_order() {
    let registry_order: Vec<&str> = REGISTRY.iter().map(|entry| entry.field).collect();
    let expected: Vec<&str> = SERIALIZED_PROVIDER_INFO_FIELDS
        .iter()
        .filter(|field| REGISTRY.iter().any(|entry| entry.field == **field))
        .copied()
        .collect();
    assert_eq!(
        registry_order, expected,
        "registry entries must follow ProviderInfo serialization order"
    );
}
