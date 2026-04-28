//! Exhaustiveness invariants for the central provider registry.
//!
//! These tests pin the four invariants that prevent provider drift across
//! the centralized catalog scaffolding:
//!
//! 1. Every variant in [`PROVIDERS_DISPLAY_ORDER`] resolves to a
//!    [`ProviderInfo`] whose `provider` field matches the lookup key.
//! 2. Every [`ProviderInfo`] field is non-empty / non-default for cross-
//!    cutting fields that must always be populated.
//! 3. The [`AiCli`](sniff::programs::AiCli) sniff binding round-trips
//!    through the existing `Provider::sniff_ai_cli()` implementation.
//! 4. The behavior trait objects are non-null (i.e. populated to a real
//!    static value, not accidentally aliased).

use sniff::programs::AiCli;

use super::identity::{Provider, PROVIDERS_DISPLAY_ORDER};
use super::registry::{all_providers, provider_info};

#[test]
fn registry_round_trip() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        assert_eq!(
            info.provider, provider,
            "provider_info({provider:?}) returned mismatched provider {:?}",
            info.provider
        );
    }
}

#[test]
fn registry_field_touch_coverage() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        assert!(
            !info.display_name.is_empty(),
            "{provider:?}: display_name is empty"
        );
        assert!(!info.slug.is_empty(), "{provider:?}: slug is empty");
        assert!(!info.binary.is_empty(), "{provider:?}: binary is empty");
        assert!(
            !info.agent_offset.is_empty(),
            "{provider:?}: agent_offset is empty"
        );
        assert!(
            info.agent_offset.starts_with('.'),
            "{provider:?}: agent_offset must begin with '.', got {:?}",
            info.agent_offset
        );
        assert!(
            !info.cli_aliases.is_empty(),
            "{provider:?}: cli_aliases is empty"
        );
        assert!(
            !info.docs_url.is_empty(),
            "{provider:?}: docs_url is empty"
        );
        // Behavior trait objects should be populated (they are required
        // fields with no `Option` wrapping).
        let _ = info.behavior.detect_from_payload(&serde_json::Value::Null);
        let _ = info.mcp.supported();
        let _ = info.adapter.detect(&serde_json::Value::Null);
        let _ = info.configurator.hooks_supported();
    }
}

#[test]
fn registry_sniff_binding_round_trip() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let from_legacy: AiCli = provider.sniff_ai_cli();
        assert_eq!(
            info.sniff_binding, from_legacy,
            "{provider:?}: registry sniff_binding {:?} does not match Provider::sniff_ai_cli() {:?}",
            info.sniff_binding, from_legacy
        );
    }
}

#[test]
fn registry_static_facts_match_legacy() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        assert_eq!(
            info.display_name,
            provider.to_string(),
            "{provider:?}: display_name mismatch"
        );
        assert_eq!(
            info.slug,
            provider.as_slug(),
            "{provider:?}: slug mismatch"
        );
        assert_eq!(
            info.agent_offset,
            provider.agent_offset(),
            "{provider:?}: agent_offset mismatch"
        );
        assert_eq!(
            info.cli_aliases,
            provider.cli_aliases(),
            "{provider:?}: cli_aliases mismatch"
        );
        assert_eq!(
            info.docs_url,
            provider.docs_url(),
            "{provider:?}: docs_url mismatch"
        );
        assert_eq!(
            info.usage_dashboard_url,
            provider.usage_dashboard_url(),
            "{provider:?}: usage_dashboard_url mismatch"
        );
        assert_eq!(
            info.supports_skills,
            provider.supports_skills(),
            "{provider:?}: supports_skills mismatch"
        );
    }
}

#[test]
fn all_providers_iterates_in_display_order() {
    let collected: Vec<Provider> = all_providers().map(|info| info.provider).collect();
    assert_eq!(collected, PROVIDERS_DISPLAY_ORDER.to_vec());
}

#[test]
fn provider_info_serializes_round_trip() {
    let info = provider_info(Provider::Claude);
    let json = serde_json::to_value(info).expect("provider_info serializes");
    assert_eq!(json["provider"], serde_json::json!("claude"));
    assert_eq!(json["slug"], serde_json::json!("claude"));
    assert_eq!(json["display_name"], serde_json::json!("Claude"));
}

#[test]
fn agent_capabilities_facade_matches_catalog() {
    use crate::agents::agent_for;

    for provider in PROVIDERS_DISPLAY_ORDER {
        let from_facade = agent_for(provider).capabilities();
        let from_catalog = provider_info(provider).agent_capabilities();
        assert_eq!(
            from_facade, from_catalog,
            "{provider:?}: agent_for facade does not match provider_info catalog"
        );
    }
}

#[test]
fn resource_support_facade_matches_catalog() {
    use crate::linking::capabilities::capabilities_for;

    for provider in PROVIDERS_DISPLAY_ORDER {
        let from_facade = capabilities_for(provider);
        let from_catalog = provider_info(provider).resource_support();
        assert_eq!(
            from_facade.provider, from_catalog.provider,
            "{provider:?}: resource_support provider mismatch"
        );
        assert_eq!(
            from_facade.skills.level, from_catalog.skills.level,
            "{provider:?}: skills support level mismatch"
        );
        assert_eq!(
            from_facade.commands.level, from_catalog.commands.level,
            "{provider:?}: commands support level mismatch"
        );
        assert_eq!(
            from_facade.agents.level, from_catalog.agents.level,
            "{provider:?}: agents support level mismatch"
        );
        assert_eq!(
            from_facade.scripts.level, from_catalog.scripts.level,
            "{provider:?}: scripts support level mismatch"
        );
    }
}

#[test]
fn agent_capabilities_id_matches_provider() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let caps = provider_info(provider).agent_capabilities();
        assert_eq!(
            caps.meta.id, provider,
            "{provider:?}: agent_capabilities meta.id mismatch"
        );
    }
}

#[test]
fn resource_support_provider_matches_provider() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let support = provider_info(provider).resource_support();
        assert_eq!(
            support.provider, provider,
            "{provider:?}: resource_support provider field mismatch"
        );
    }
}

/// Phase 4 guard: the only authoritative `match Provider` site in the lib
/// crate is the central registry in [`crate::provider::registry`]. Every
/// other per-domain dispatch must route through provider behavior traits.
///
/// The scan walks `claudine/lib/src/**/*.rs` and flags any file whose source
/// contains the literal `match provider` (case-insensitive) outside the
/// allowed audit-list. Allowed sites are the registry itself, the
/// `provider/identity.rs` helpers that own the canonical mapping (string
/// slugs, sniff bindings, display order), and a small set of compatibility
/// shims that still need direct `match` dispatch on the canonical enum.
#[test]
fn no_unauthorized_match_provider_in_lib() {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let lib_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let allowed: &[&str] = &[
        // Central registry (the one authoritative dispatch site).
        "src/provider/registry.rs",
        // Canonical identity helpers (slug, sniff binding, aliases, display
        // order). These are part of the central registry surface.
        "src/provider/identity.rs",
        // The guard test source code itself contains the literal pattern
        // `match provider {` as a string. Allow.
        "src/provider/tests.rs",
        // Event matrix and event provider helpers still own canonical
        // event-shape pattern matching (Phase 3 retained these as the
        // matrix surface). Compatibility shim.
        "src/events/provider.rs",
        "src/events/matrix.rs",
        // Test fixture in the adapters module uses `match provider` to
        // synthesize raw payloads. Test-only.
        "src/adapters/mod.rs",
        // `agents::registry::agent_for` returns a `&'static dyn Agent`
        // (not the same as `provider_info(provider).agent_capabilities`).
        // Phase 8 cleanup will remove this legacy facade once consumers
        // migrate. Compatibility shim.
        "src/agents/registry.rs",
        // Phase 5 migrates these stringly-typed capability surfaces into
        // typed ProviderInfo fields (env vars, CLI override sensitivity,
        // model catalog sources). Compatibility shims for now.
        "src/composition/select.rs",
        "src/permissions/query.rs",
        "src/model_catalog/provider_sources.rs",
    ];

    let mut files = Vec::new();
    collect_rs_files(&lib_src, &mut files);

    let mut violators: Vec<String> = Vec::new();
    for file in &files {
        let rel = file
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        if allowed.iter().any(|allow| rel.ends_with(allow)) {
            continue;
        }
        let Ok(content) = fs::read_to_string(file) else {
            continue;
        };
        // Strip line comments so doc/comment references don't trip the scan.
        let stripped: String = content
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        let lower = stripped.to_ascii_lowercase();
        // Look for `match <ident>` where <ident> is a binding expected to
        // be a Provider; conservatively flag any literal `match provider`.
        if lower.contains("match provider {") || lower.contains("match provider\n") {
            violators.push(rel);
        }
    }

    assert!(
        violators.is_empty(),
        "Phase 4 guard: unauthorized `match provider` dispatch found in lib crate. \
         Route per-domain dispatch through ProviderInfo behavior traits or add \
         the file to the allow-list in `provider::tests::no_unauthorized_match_provider_in_lib`. \
         Violators: {violators:?}"
    );
}

// ---------------------------------------------------------------------------
// Phase 5: typed catalog property tests
// ---------------------------------------------------------------------------

/// Every supported event mapping has a non-empty native name.
#[test]
fn supported_events_have_non_empty_native_names() {
    use crate::events::{AgenticEvent, EventSupportLevel};

    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        for event in AgenticEvent::ALL {
            let Some(mapping) = info.event_mapping.lookup(event) else {
                continue;
            };
            if matches!(mapping.support_level, EventSupportLevel::NotSupported) {
                continue;
            }
            assert!(
                !mapping.native_name.is_empty(),
                "{provider:?} {event:?}: support_level {:?} but native_name is empty",
                mapping.support_level
            );
        }
    }
}

/// Hook-level events imply the provider's configurator advertises hook
/// support. (The inverse does not hold — providers can advertise hook
/// support without specifically marking every event as a Hook.)
#[test]
fn hook_events_imply_configurator_hooks_supported() {
    use crate::events::{AgenticEvent, EventSupportLevel};

    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let has_hook_event = AgenticEvent::ALL.iter().any(|event| {
            matches!(
                info.event_mapping.support_level(*event),
                EventSupportLevel::Hook
            )
        });
        if has_hook_event {
            assert!(
                info.configurator.hooks_supported(),
                "{provider:?}: declares Hook events but configurator.hooks_supported() == false"
            );
        }
    }
}

/// Stream-protocol providers must have at least one supported event with a
/// non-empty native name (they need *something* to parse).
#[test]
fn stream_providers_expose_at_least_one_event() {
    use crate::events::{AgenticEvent, EventSupportLevel};

    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        if info.stream_protocol.is_none() {
            continue;
        }
        let any_supported = AgenticEvent::ALL.iter().any(|event| {
            !matches!(
                info.event_mapping.support_level(*event),
                EventSupportLevel::NotSupported
            )
        });
        assert!(
            any_supported,
            "{provider:?}: declares stream_protocol={:?} but no events are supported",
            info.stream_protocol
        );
    }
}

/// Every PathTemplate in the typed catalog data round-trips its raw form.
#[test]
fn typed_path_templates_have_non_empty_raw() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let bundles: &[(&str, &[crate::provider::PathTemplate])] = &[
            ("session_log_paths", info.session_log_paths),
            ("session_locations", info.session_locations),
            ("config_paths", info.config_paths),
            ("memory_files", info.memory_files),
            ("system_prompt.memory_files", info.system_prompt.memory_files),
        ];
        for (label, templates) in bundles {
            for template in *templates {
                assert!(
                    !template.raw().is_empty(),
                    "{provider:?} {label}: PathTemplate has empty raw form"
                );
            }
        }
    }
}

/// Every output-format support entry has a non-empty native name and the
/// canonical lowercase identifier exposed by `OutputFormat::as_str` is one
/// of the documented values (sanity check that we didn't add new variants
/// without updating `as_str`).
#[test]
fn output_format_support_entries_are_well_formed() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        for support in info.output_formats {
            assert!(
                !support.native_name.is_empty(),
                "{provider:?}: output format {:?} has empty native_name",
                support.format
            );
            assert!(
                matches!(support.format.as_str(), "text" | "json" | "stream"),
                "{provider:?}: output format identifier {:?} is unrecognized",
                support.format.as_str()
            );
        }
    }
}

/// Provider-info round-trip: the registry returns an entry whose
/// `provider` field matches the lookup key for every variant. This is
/// already tested above; the additive check here verifies it under the
/// "Phase 5 invariants" umbrella so test failure isolation is clearer.
#[test]
fn provider_field_matches_registry_key() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        assert_eq!(provider_info(provider).provider, provider);
    }
}
