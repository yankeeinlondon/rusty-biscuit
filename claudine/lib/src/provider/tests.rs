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

use super::identity::{PROVIDERS_DISPLAY_ORDER, Provider};
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
        assert!(!info.docs_url.is_empty(), "{provider:?}: docs_url is empty");
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
        assert_eq!(info.slug, provider.as_slug(), "{provider:?}: slug mismatch");
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
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let json = serde_json::to_value(info).expect("provider_info serializes");

        // Identity half.
        assert_eq!(
            json["slug"],
            serde_json::json!(info.slug),
            "{provider:?}: slug mismatch"
        );
        assert_eq!(
            json["display_name"],
            serde_json::json!(info.display_name),
            "{provider:?}: display_name mismatch"
        );

        // Typed catalog half — all of these fields must serialize so
        // `claudine providers --describe --format json` round-trips the
        // central catalog without information loss.
        for key in [
            "event_mapping",
            "output_formats",
            "entrypoints",
            "system_prompt",
            "yolo",
            "reasoning",
            "known_gaps",
            "acp",
            "prompt_arg_conventions",
            "session_log_paths",
            "session_locations",
            "config_paths",
            "memory_files",
            "stream_protocol",
        ] {
            assert!(
                json.get(key).is_some(),
                "{provider:?}: serialized JSON is missing field {key:?}"
            );
        }

        // Trait objects and fn-pointer accessors must NOT serialize.
        for key in [
            "behavior",
            "mcp",
            "adapter",
            "configurator",
            "agent_capabilities_fn",
            "resource_support_fn",
        ] {
            assert!(
                json.get(key).is_none(),
                "{provider:?}: serialized JSON unexpectedly contains field {key:?}"
            );
        }
    }
}

/// Every event-mapping row reachable from `info.event_mapping.mappings`
/// must round-trip through JSON without losing rows.
#[test]
fn provider_info_json_does_not_lose_event_rows() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let json = serde_json::to_value(info).expect("provider_info serializes");
        let row_count = info.event_mapping.mappings.len();
        let mappings = json["event_mapping"]["mappings"]
            .as_array()
            .unwrap_or_else(|| panic!("{provider:?}: event_mapping.mappings is not an array"));
        assert_eq!(
            mappings.len(),
            row_count,
            "{provider:?}: event_mapping row count mismatch"
        );
    }
}

/// Round-trip a couple of well-known providers and check specific nested
/// values to lock in the JSON shape.
#[test]
fn provider_info_json_round_trips_well_known_keys() {
    use super::acp::AcpServerMode;

    // Goose advertises native ACP server mode.
    let goose = provider_info(Provider::Goose);
    let goose_json = serde_json::to_value(goose).expect("goose serializes");
    assert!(
        matches!(goose.acp.server_mode, AcpServerMode::Native),
        "Goose acp.server_mode should be Native (sanity check)"
    );
    assert_eq!(
        goose_json["acp"]["server_mode"],
        serde_json::json!("native"),
        "Goose acp.server_mode should serialize as \"native\""
    );

    // Claude has the richest typed catalog. Spot-check a few nested keys.
    let claude = provider_info(Provider::Claude);
    let claude_json = serde_json::to_value(claude).expect("claude serializes");
    assert_eq!(
        claude_json["slug"],
        serde_json::json!("claude"),
        "Claude slug round-trip"
    );
    assert!(
        claude_json["entrypoints"].is_array(),
        "Claude entrypoints should serialize as an array"
    );
    assert!(
        claude_json["output_formats"].is_array(),
        "Claude output_formats should serialize as an array"
    );
    assert!(
        claude_json["system_prompt"].is_object(),
        "Claude system_prompt should serialize as an object"
    );
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

/// Drift guard: the only authoritative dispatch sites for [`Provider`] in
/// the lib crate are the central registry and identity helpers. Every other
/// per-domain dispatch must route through `ProviderInfo` behavior traits.
///
/// The scan walks `claudine/lib/src/**/*.rs` and flags files whose source
/// contains any of the following patterns (after Rust line comments and
/// `/* ... */` block comments are stripped):
///
/// 1. `match <ident> { ... Provider::<Variant> => ... }` — a `match`
///    expression with at least one `Provider::<Variant> =>` arm,
///    regardless of binding name (`provider`, `p`, `self`, `self.provider`,
///    `*self`, `&*self`, etc.).
/// 2. Standalone `Provider::<Variant> => ` arms (catches single-arm matches
///    or `if let` ladders that drift back into per-variant dispatch).
/// 3. `[(Provider::<Variant>, ...)]` provider tuple arrays — the exact
///    duplicated-fact pattern Phase 2 removed from `discover_agents_full`.
///
/// Plain `[Provider::<Variant>, ...]` arrays are *not* flagged: they are
/// commonly used in tests as input fixtures (display-order checks, picker
/// preference lists, etc.) and do not represent provider facts.
///
/// The allow-list is intentionally narrow. Positive invariant tests
/// (catalog round-trip, wrapper registry coverage, agent discovery, etc.)
/// are the primary safety net; this scan is a defense-in-depth backstop.
#[test]
fn no_unauthorized_match_provider_in_lib() {
    use regex::Regex;
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

    /// Strip Rust `//` line comments and `/* ... */` block comments from
    /// `src` so commented-out examples don't trip the scan. Does NOT
    /// attempt to handle Rust strings containing `//` — false positives
    /// from string literals are rare enough to allow-list explicitly.
    fn strip_comments(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        let bytes = src.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
                // Line comment: skip to newline.
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            } else if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                // Block comment: skip to closing */.
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
            } else {
                out.push(bytes[i] as char);
                i += 1;
            }
        }
        out
    }

    let lib_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let allowed: &[&str] = &[
        // Central registry (the one authoritative dispatch site).
        "src/provider/registry.rs",
        // Canonical identity helpers (slug, sniff binding, aliases, display
        // order). These are part of the central registry surface.
        "src/provider/identity.rs",
        // The guard test source code itself contains the literal patterns
        // we are scanning for. Self-allow.
        "src/provider/tests.rs",
        // Test fixture in the adapters module uses a `match provider`
        // expression with `Provider::Claude => json!(...)` arms to
        // synthesize raw payloads. Test-only fixture.
        "src/adapters/mod.rs",
        // `provider/methods.rs` test module uses
        // `[(Provider::X, "expected-slug"), ...]` tuple fixtures to assert
        // canonical serialization/Display output. The "expected-slug"
        // strings are not duplicated provider facts — they pin the
        // canonical surface that downstream code consumes via
        // `Provider::as_slug()` etc.
        "src/provider/methods.rs",
    ];

    let mut files = Vec::new();
    collect_rs_files(&lib_src, &mut files);

    // Pattern 1: `match <ident> { ... Provider::<Variant> => ... }` block
    // — catches all match-form dispatch on Provider regardless of binding.
    // Multiline + dot-matches-newline keeps this practical for real code.
    let match_with_provider_arm = Regex::new(
        r"(?s)match\s+[A-Za-z_][A-Za-z0-9_\.\*&]*\s*\{[^}]*?Provider::[A-Z][A-Za-z]+\s*=>",
    )
    .unwrap();
    // Pattern 2: standalone `Provider::<Variant> => ` arms (catches
    // alternate forms like `if let` ladders or single-arm matches that the
    // block-level scan may not capture).
    let provider_arm = Regex::new(r"Provider::[A-Z][A-Za-z]+\s*=>").unwrap();
    // Pattern 3: provider tuple arrays — `[(Provider::Foo, ...)]`.
    let provider_tuple_array = Regex::new(r"\[\s*\(\s*Provider::[A-Z]").unwrap();

    let mut violators: Vec<(String, &'static str)> = Vec::new();
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
        let stripped = strip_comments(&content);

        if match_with_provider_arm.is_match(&stripped) {
            violators.push((rel.clone(), "match-with-Provider-arm"));
        }
        if provider_arm.is_match(&stripped) {
            // The block-level pattern subsumes most single-arm cases, but
            // record separately so the diagnostic explains which pattern
            // class fired.
            violators.push((rel.clone(), "Provider::Variant-arm"));
        }
        if provider_tuple_array.is_match(&stripped) {
            violators.push((rel, "[(Provider::...)] tuple array"));
        }
    }

    assert!(
        violators.is_empty(),
        "Drift guard: unauthorized per-variant `Provider` dispatch found in lib crate. \
         Route per-domain dispatch through `ProviderInfo` behavior traits, or add the \
         file to the allow-list in `provider::tests::no_unauthorized_match_provider_in_lib` \
         with a comment explaining why. Violators: {violators:?}"
    );
}

// ---------------------------------------------------------------------------
// Phase 5: typed catalog property tests
// ---------------------------------------------------------------------------

/// Every supported event mapping has a non-empty native name.
#[test]
fn supported_events_have_non_empty_native_names() {
    use crate::events::AgenticEvent;
    use crate::provider::EventSupportLevel;
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        for event in AgenticEvent::ALL {
            let Some(mapping) = info.event_mapping.lookup(event) else {
                continue;
            };
            if matches!(mapping.support_level, EventSupportLevel::NotSupported) {
                continue;
            }
            let name = mapping.support_level.native_name().unwrap_or("");
            assert!(
                !name.is_empty(),
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
    use crate::events::AgenticEvent;
    use crate::provider::EventSupportLevel;
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let has_hook_event = AgenticEvent::ALL.iter().any(|event| {
            matches!(
                info.event_mapping.support_level(*event),
                EventSupportLevel::Hook { .. }
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
    use crate::events::AgenticEvent;
    use crate::provider::EventSupportLevel;
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

// ---------------------------------------------------------------------------
// Phase 7: ACP invariants
// ---------------------------------------------------------------------------

/// Any provider with at least one [`EventSupportLevel::Acp`] mapping row
/// must report a non-`NotSupported` ACP server mode.
#[test]
fn acp_events_imply_acp_support() {
    use super::acp::AcpServerMode;
    use crate::events::AgenticEvent;
    use crate::provider::EventSupportLevel;
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let has_acp_event = AgenticEvent::ALL.iter().any(|event| {
            matches!(
                info.event_mapping.support_level(*event),
                EventSupportLevel::Acp { .. }
            )
        });
        if has_acp_event {
            assert!(
                !matches!(info.acp.server_mode, AcpServerMode::NotSupported),
                "{provider:?}: declares EventSupportLevel::Acp rows but acp.server_mode is NotSupported"
            );
            assert!(
                !info.acp.events_via_acp.is_empty(),
                "{provider:?}: declares EventSupportLevel::Acp rows but acp.events_via_acp is empty"
            );
        }
    }
}

/// Goose maps `request_permission` to ACP capture.
#[test]
fn goose_request_permission_is_acp() {
    use super::acp::{AcpEvent, AcpServerMode};
    use crate::events::AgenticEvent;
    let info = provider_info(Provider::Goose);
    assert!(
        info.event_mapping
            .support_level(AgenticEvent::HumanInTheLoop)
            .is_acp()
    );
    assert_eq!(
        info.event_mapping.native_name(AgenticEvent::HumanInTheLoop),
        Some("request_permission")
    );
    assert!(matches!(info.acp.server_mode, AcpServerMode::Native));
    assert!(
        info.acp
            .events_via_acp
            .contains(&AcpEvent::RequestPermission)
    );
}

/// Kimi maps `ApprovalRequest` to ACP capture.
#[test]
fn kimi_approval_request_is_acp() {
    use super::acp::{AcpEvent, AcpServerMode};
    use crate::events::AgenticEvent;
    let info = provider_info(Provider::KimiCode);
    assert!(
        info.event_mapping
            .support_level(AgenticEvent::PermissionRequest)
            .is_acp()
    );
    assert_eq!(
        info.event_mapping
            .native_name(AgenticEvent::PermissionRequest),
        Some("ApprovalRequest")
    );
    assert!(matches!(
        info.acp.server_mode,
        AcpServerMode::AvailableViaWireProxy
    ));
    assert!(info.acp.events_via_acp.contains(&AcpEvent::ApprovalRequest));
}

/// Providers without any ACP rows report `AcpSupport::NOT_SUPPORTED`.
#[test]
fn non_acp_providers_have_not_supported_acp() {
    use super::acp::AcpServerMode;
    use crate::events::AgenticEvent;
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let has_acp_event = AgenticEvent::ALL.iter().any(|event| {
            info.event_mapping.support_level(*event).is_acp()
        });
        if !has_acp_event {
            assert!(
                matches!(info.acp.server_mode, AcpServerMode::NotSupported),
                "{provider:?}: has no EventSupportLevel::Acp rows but acp.server_mode is {:?}",
                info.acp.server_mode
            );
            assert!(
                info.acp.events_via_acp.is_empty(),
                "{provider:?}: has no EventSupportLevel::Acp rows but acp.events_via_acp is non-empty"
            );
        }
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
            (
                "system_prompt.memory_files",
                info.system_prompt.memory_files,
            ),
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

// ---------------------------------------------------------------------------
// Phase 3: registry ergonomics and test hardening
// ---------------------------------------------------------------------------

/// The OnceLock-backed registry array must have exactly one slot per
/// [`Provider`] variant.
#[test]
fn registry_array_length_matches_variant_count() {
    let registry = super::registry::REGISTRY.get().expect("registry initialized");
    assert_eq!(registry.len(), super::PROVIDER_COUNT);
}

/// Every provider's [`ProviderBehavior::detect_from_payload`] can be
/// exercised with an empty payload without panicking.
#[test]
fn detect_from_payload_exercise_all_providers() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let empty = serde_json::Value::Object(Default::default());
        let _ = info.behavior.detect_from_payload(&empty);
    }
}

/// Every provider's [`AdapterBehavior::detect`] can be exercised with an
/// empty payload without panicking.
#[test]
fn adapter_detect_exercise_all_providers() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let empty = serde_json::Value::Object(Default::default());
        let _ = info.adapter.detect(&empty);
    }
}

/// Every provider declares at least one [`crate::provider::PathTemplate`]
/// in `config_paths`. The first entry is treated as the primary user-level
/// config path by [`crate::config::discover_agents_full`].
#[test]
fn config_paths_have_primary_user_entry() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        assert!(
            !info.config_paths.is_empty(),
            "{provider:?}: config_paths must contain at least one entry"
        );
    }
}
