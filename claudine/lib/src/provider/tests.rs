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
            "resource_support",
            "output_formats",
            "entrypoints",
            "system_prompt",
            "yolo",
            "reasoning",
            "known_gaps",
            "acp",
            "prompt_arg_conventions",
            "session_log_paths",
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
            "capabilities",
            "resource_support_fn",
        ] {
            assert!(
                json.get(key).is_none(),
                "{provider:?}: serialized JSON unexpectedly contains field {key:?}"
            );
        }
    }
}

/// Twin of claudine-gen's registry-covers-all-fields guard
/// (`gen/tests/registry_coverage.rs`): the serialized `--describe` key
/// list is checked in on BOTH sides — the generator asserts its mapping
/// registry against the list, and this test binds the list to the real
/// serialization. Adding/removing/reordering a serialized `ProviderInfo`
/// field must update both copies (and the mapping registry).
#[test]
fn serialized_field_list_matches_catalog() {
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
    // serde_json without `preserve_order` sorts map keys, so membership
    // (not order) is asserted here; the gen-side twin pins the order
    // against the mapping registry.
    let mut expected: Vec<&str> = SERIALIZED_PROVIDER_INFO_FIELDS.to_vec();
    expected.sort_unstable();
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let json = serde_json::to_value(info).expect("provider_info serializes");
        let keys: Vec<&str> = json
            .as_object()
            .expect("ProviderInfo serializes as an object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys, expected,
            "{provider:?}: serialized field list drifted — update the checked-in list \
             here AND in gen/tests/registry_coverage.rs, and extend the mapping registry"
        );
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

/// Pins the generated expected-offering shape on kimi, the provider with
/// both offering classes: `kimi-for-coding` is the plan endpoint (absent
/// from the unchained-ai artifact by design, so never joined) and
/// `kimi-k2.7-code` joins the artifact's identity key exactly.
#[test]
fn kimi_expected_offerings_carry_classification_and_artifact_join() {
    use super::offering::OfferingClass;

    let kimi = provider_info(Provider::KimiCode);
    let plan = kimi
        .expected_offerings
        .iter()
        .find(|offering| offering.id == "kimi-for-coding")
        .expect("kimi-for-coding is an expected offering");
    assert_eq!(plan.class, OfferingClass::PlanEndpoint);
    assert_eq!(plan.alias, Some("kimi-code"));
    assert_eq!(plan.catalog_id, None);

    let joined = kimi
        .expected_offerings
        .iter()
        .find(|offering| offering.id == "kimi-k2.7-code")
        .expect("kimi-k2.7-code is an expected offering");
    assert_eq!(joined.class, OfferingClass::VendorApi);
    assert_eq!(joined.catalog_id, Some("moonshotai/kimi-k-code@2.7"));
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
fn resource_support_provider_matches_provider() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let support = provider_info(provider).resource_support();
        assert_eq!(
            support.provider, provider,
            "{provider:?}: resource_support provider field mismatch"
        );
    }
}

/// Strip Rust `//` line comments and `/* ... */` block comments from `src` so
/// commented-out examples don't trip the source-scan guard below. Does NOT
/// attempt to handle Rust strings containing `//` — false positives from
/// string literals are rare and handled at the call site.
///
/// The package-wide `Provider` dispatch guard now lives in
/// `claudine-cli/tests/dispatch_inventory.rs` (Phase I unified both crates into
/// one inventory-based, site-level guard). This helper survives only for the
/// [`detect_from_payload_has_no_provider_specific_branches`] source scan.
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

/// Guard against `provider/<slug>/legacy.rs` files ever returning.
///
/// The module split (design/module-split.md) parked each provider's legacy
/// `AgentCapabilities` builders in a TEMPORARY `legacy.rs`; the retirement
/// (workstream 2) deleted the tree and every `legacy.rs` with it. The set
/// is now empty and must stay empty: any `legacy.rs` appearing under
/// `src/provider/` fails this guard.
#[test]
fn provider_legacy_files_only_shrink() {
    use std::fs;
    use std::path::Path;

    let provider_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/provider");
    let mut unexpected = Vec::new();
    for entry in fs::read_dir(&provider_dir).expect("src/provider must be readable") {
        let path = entry.expect("readable dir entry").path();
        if !path.is_dir() || !path.join("legacy.rs").is_file() {
            continue;
        }
        let slug = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        unexpected.push(slug);
    }

    assert!(
        unexpected.is_empty(),
        "Provider legacy.rs file(s) found for {unexpected:?}. The legacy \
         `AgentCapabilities` tree was retired (design/module-split.md): \
         providers must not have a `legacy.rs`; put typed catalog data in \
         `data.rs` and trait impls in `behavior.rs` instead."
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
/// must report a provider that actually speaks ACP, plus the events it
/// captures through it.
#[test]
fn acp_events_imply_acp_support() {
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
                info.acp.is_supported(),
                "{provider:?}: declares EventSupportLevel::Acp rows but acp.server_mode is {:?}",
                info.acp.server_mode
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
    assert!(matches!(info.acp.server_mode, AcpServerMode::Native));
    assert!(info.acp.events_via_acp.contains(&AcpEvent::ApprovalRequest));
}

/// Providers without any ACP rows capture no events via ACP. `server_mode`
/// is deliberately NOT constrained here: it records the provider's own ACP
/// posture (research-fed), independent of Claudine's event wiring.
#[test]
fn non_acp_providers_capture_no_acp_events() {
    use crate::events::AgenticEvent;
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let has_acp_event = AgenticEvent::ALL
            .iter()
            .any(|event| info.event_mapping.support_level(*event).is_acp());
        if !has_acp_event {
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
///
/// Nextest runs each test in its own process, so the `OnceLock` starts
/// uninitialized — we must trigger lazy initialization via
/// [`provider_info`] before reading `REGISTRY.get()`.
#[test]
fn registry_array_length_matches_variant_count() {
    let _ = provider_info(PROVIDERS_DISPLAY_ORDER[0]);
    let registry = super::registry::REGISTRY
        .get()
        .expect("registry initialized");
    assert_eq!(registry.len(), super::PROVIDER_COUNT);
}

/// Returns a representative payload that the named provider's adapter
/// must recognize. Used by the strengthened detection tests below.
fn representative_payload_for(provider: Provider) -> Option<serde_json::Value> {
    Some(match provider {
        Provider::Claude => serde_json::json!({"hook_event_name": "Stop"}),
        Provider::Codex => {
            serde_json::json!({"type": "turn.completed", "thread_id": "t-1"})
        }
        Provider::Gemini => serde_json::json!({"hook_event_name": "BeforeAgent"}),
        Provider::OpenCode => serde_json::json!({"event_type": "session.idle"}),
        Provider::KimiCode => serde_json::json!({"method": "notification"}),
        // Goose and Qwen do not detect via raw payload shape today. Kilo
        // shares OpenCode's payload shape, so it cannot be uniquely detected
        // from a raw payload — the wrapper path knows the provider instead. Pi
        // has no native hooks, so it never delivers a raw hook payload at all;
        // its `--mode json` stdout stream is parsed by PiSemanticStreamParser.
        // Antigravity likewise delivers no raw hook payload; its
        // `--output-format json` envelope is parsed by
        // AntigravitySemanticStreamParser.
        Provider::Goose
        | Provider::QwenCode
        | Provider::Kilo
        | Provider::Pi
        | Provider::Antigravity => return None,
    })
}

/// Every provider's [`ProviderBehavior::detect_from_payload`] can be
/// exercised with an empty payload without panicking, and providers that
/// expose payload-shape detection on this surface recognize their
/// representative payloads. Public provider detection must route through
/// the same behavior surface and return the provider for those payloads.
#[test]
fn detect_from_payload_exercise_all_providers() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let empty = serde_json::Value::Object(Default::default());
        let _ = info.behavior.detect_from_payload(&empty);

        if let Some(payload) = representative_payload_for(provider) {
            assert!(
                info.behavior.detect_from_payload(&payload),
                "{provider:?}: behavior.detect_from_payload must recognize representative payload {payload}"
            );
            assert_eq!(
                Provider::detect_from_payload(&payload),
                Some(provider),
                "{provider:?}: public detection must return the representative payload's provider"
            );
        }
    }
}

/// Every provider's [`AdapterBehavior::detect`] can be exercised with an
/// empty payload without panicking, and providers that have a payload
/// shape rule recognize their representative payload — exercising the
/// per-provider detection move that closed Finding 2.
#[test]
fn adapter_detect_exercise_all_providers() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let empty = serde_json::Value::Object(Default::default());
        let _ = info.adapter.detect(&empty);

        if let Some(payload) = representative_payload_for(provider) {
            assert!(
                info.adapter.detect(&payload),
                "{provider:?}: adapter.detect must recognize representative payload {payload}"
            );
        }
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

// ---------------------------------------------------------------------------
// Review 4 / Phase 1: serializable catalog half (Finding 1)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Phase 2: system-prompt delivery spec invariants
// ---------------------------------------------------------------------------

/// The affected providers (Gemini, Codex, Qwen) must use the new
/// spec-driven delivery variants so the wrap layer can dispatch without
/// HOME redirect.
#[test]
fn gemini_system_prompt_uses_env_var_file() {
    let info = provider_info(Provider::Gemini);
    assert!(
        matches!(
            info.system_prompt.append.interactive,
            super::SystemPromptDelivery::EnvVarFile {
                env_var: "GEMINI_SYSTEM_MD"
            }
        ),
        "Gemini append interactive must be EnvVarFile(GEMINI_SYSTEM_MD)"
    );
    assert!(
        matches!(
            info.system_prompt.append.non_interactive,
            super::SystemPromptDelivery::EnvVarFile {
                env_var: "GEMINI_SYSTEM_MD"
            }
        ),
        "Gemini append non-interactive must be EnvVarFile(GEMINI_SYSTEM_MD)"
    );
    assert!(
        matches!(
            info.system_prompt.replace.interactive,
            super::SystemPromptDelivery::EnvVarFile {
                env_var: "GEMINI_SYSTEM_MD"
            }
        ),
        "Gemini replace interactive must be EnvVarFile(GEMINI_SYSTEM_MD)"
    );
    assert!(
        matches!(
            info.system_prompt.replace.non_interactive,
            super::SystemPromptDelivery::EnvVarFile {
                env_var: "GEMINI_SYSTEM_MD"
            }
        ),
        "Gemini replace non-interactive must be EnvVarFile(GEMINI_SYSTEM_MD)"
    );
}

#[test]
fn qwen_system_prompt_uses_inline_flags() {
    let info = provider_info(Provider::QwenCode);
    assert!(
        matches!(
            info.system_prompt.append.interactive,
            super::SystemPromptDelivery::InlineFlag {
                flag: "--append-system-prompt"
            }
        ),
        "Qwen append interactive must be InlineFlag(--append-system-prompt)"
    );
    assert!(
        matches!(
            info.system_prompt.append.non_interactive,
            super::SystemPromptDelivery::InlineFlag {
                flag: "--append-system-prompt"
            }
        ),
        "Qwen append non-interactive must be InlineFlag(--append-system-prompt)"
    );
    assert!(
        matches!(
            info.system_prompt.replace.interactive,
            super::SystemPromptDelivery::InlineFlag {
                flag: "--system-prompt"
            }
        ),
        "Qwen replace interactive must be InlineFlag(--system-prompt)"
    );
    assert!(
        matches!(
            info.system_prompt.replace.non_interactive,
            super::SystemPromptDelivery::InlineFlag {
                flag: "--system-prompt"
            }
        ),
        "Qwen replace non-interactive must be InlineFlag(--system-prompt)"
    );
}

#[test]
fn codex_system_prompt_uses_config_key_variants() {
    let info = provider_info(Provider::Codex);
    assert!(
        matches!(
            info.system_prompt.append.interactive,
            super::SystemPromptDelivery::ConfigKeyInline {
                flag: "-c",
                key: "developer_instructions"
            }
        ),
        "Codex append interactive must be ConfigKeyInline(-c, developer_instructions)"
    );
    assert!(
        matches!(
            info.system_prompt.append.non_interactive,
            super::SystemPromptDelivery::ConfigKeyInline {
                flag: "-c",
                key: "developer_instructions"
            }
        ),
        "Codex append non-interactive must be ConfigKeyInline(-c, developer_instructions)"
    );
    assert!(
        matches!(
            info.system_prompt.replace.interactive,
            super::SystemPromptDelivery::ConfigKeyFile {
                flag: "-c",
                key: "model_instructions_file"
            }
        ),
        "Codex replace interactive must be ConfigKeyFile(-c, model_instructions_file)"
    );
    assert!(
        matches!(
            info.system_prompt.replace.non_interactive,
            super::SystemPromptDelivery::ConfigKeyFile {
                flag: "-c",
                key: "model_instructions_file"
            }
        ),
        "Codex replace non-interactive must be ConfigKeyFile(-c, model_instructions_file)"
    );
}

/// Every provider's JSON describe surface includes typed resource
/// portability data and excludes the retired legacy `AgentCapabilities`
/// tree.
#[test]
fn provider_info_json_includes_resource_support_not_capabilities() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let json = serde_json::to_value(info).expect("provider_info serializes");
        assert!(
            json.get("capabilities").is_none(),
            "{provider:?}: serialized JSON must not expose legacy `capabilities`"
        );
        assert!(
            json.get("resource_support").is_some(),
            "{provider:?}: serialized JSON missing `resource_support`"
        );
        assert!(
            json["resource_support"].is_object(),
            "{provider:?}: `resource_support` must be a JSON object, got {:?}",
            json["resource_support"]
        );
    }
}

/// The serialized `resource_support` for Claude includes objects for each
/// resource type (skills, commands, agents, scripts) with the canonical
/// `level` key from [`crate::linking::capabilities::ResourceSupport`].
/// Closes Finding 1.
#[test]
fn provider_info_json_resource_support_includes_skills_commands_agents_scripts() {
    let info = provider_info(Provider::Claude);
    let json = serde_json::to_value(info).expect("provider_info serializes");
    let resource_support = &json["resource_support"];

    for key in ["skills", "commands", "agents", "scripts"] {
        let entry = &resource_support[key];
        assert!(
            entry.is_object(),
            "Claude resource_support.{key} must be a JSON object, got {entry:?}"
        );
        assert!(
            entry.get("level").is_some(),
            "Claude resource_support.{key} must expose a `level` field"
        );
        assert!(
            entry["level"].is_string(),
            "Claude resource_support.{key}.level must serialize as a string, got {:?}",
            entry["level"]
        );
    }
}

// ---------------------------------------------------------------------------
// Review 4 / Phase 1: per-provider payload detection (Finding 2)
// ---------------------------------------------------------------------------

/// Claude's adapter recognizes every Claude-native `hook_event_name` value
/// and rejects names that are exclusively Gemini's. Locks the per-provider
/// detection move that closed Finding 2.
#[test]
fn adapter_detects_known_claude_payloads() {
    let claude = provider_info(Provider::Claude);
    for name in [
        "Stop",
        "PreToolUse",
        "UserPromptSubmit",
        "SessionStart",
        "Notification",
    ] {
        let payload = serde_json::json!({"hook_event_name": name});
        assert!(
            claude.adapter.detect(&payload),
            "Claude adapter should recognize hook_event_name={name:?}"
        );
    }

    // BeforeAgent is exclusively Gemini.
    let gemini_only = serde_json::json!({"hook_event_name": "BeforeAgent"});
    assert!(
        !claude.adapter.detect(&gemini_only),
        "Claude adapter must not claim Gemini-only hook_event_name=BeforeAgent"
    );
}

/// Gemini's adapter recognizes every Gemini-native `hook_event_name` value
/// (and the legacy `event_name` shape) without claiming names shared with
/// Claude. Locks the per-provider detection move that closed Finding 2.
#[test]
fn adapter_detects_known_gemini_payloads() {
    let gemini = provider_info(Provider::Gemini);
    for name in [
        "BeforeAgent",
        "AfterAgent",
        "BeforeModel",
        "BeforeTool",
        "AfterTool",
    ] {
        let payload = serde_json::json!({"hook_event_name": name});
        assert!(
            gemini.adapter.detect(&payload),
            "Gemini adapter should recognize hook_event_name={name:?}"
        );
    }

    // Legacy `event_name` shape.
    assert!(
        gemini
            .adapter
            .detect(&serde_json::json!({"event_name": "BeforeAgent"})),
        "Gemini adapter should recognize legacy event_name shape"
    );

    // Stop is shared with Claude — Gemini must not claim it.
    let claude_shared = serde_json::json!({"hook_event_name": "Stop"});
    assert!(
        !gemini.adapter.detect(&claude_shared),
        "Gemini adapter must not claim Claude-shared hook_event_name=Stop"
    );
}

/// Codex's adapter recognizes every Codex-shape payload (top-level
/// `type`, nested `hook_event.event_type`, and thread-id keys). Locks the
/// per-provider detection move that closed Finding 2.
#[test]
fn adapter_detects_known_codex_payloads() {
    let codex = provider_info(Provider::Codex);

    let cases = [
        serde_json::json!({"type": "turn.completed", "thread_id": "t-1"}),
        serde_json::json!({"type": "agent-turn-complete", "thread-id": "t-1"}),
        serde_json::json!({
            "session_id": "ses_123",
            "hook_event": {"event_type": "after_tool_use"}
        }),
    ];

    for payload in cases {
        assert!(
            codex.adapter.detect(&payload),
            "Codex adapter should recognize payload {payload}"
        );
    }
}

/// OpenCode's adapter recognizes both the snake_case `event_type` and
/// camelCase `eventType` payload shapes. Locks the per-provider detection
/// move that closed Finding 2.
#[test]
fn adapter_detects_known_opencode_payloads() {
    let opencode = provider_info(Provider::OpenCode);
    assert!(
        opencode
            .adapter
            .detect(&serde_json::json!({"event_type": "session.idle"})),
        "OpenCode adapter should recognize event_type"
    );
    assert!(
        opencode
            .adapter
            .detect(&serde_json::json!({"eventType": "session.idle"})),
        "OpenCode adapter should recognize eventType"
    );
}

/// Kimi's adapter recognizes the JSON-RPC `method` payload shape. Locks
/// the per-provider detection move that closed Finding 2.
#[test]
fn adapter_detects_known_kimi_payloads() {
    let kimi = provider_info(Provider::KimiCode);
    assert!(
        kimi.adapter
            .detect(&serde_json::json!({"method": "notification"})),
        "Kimi adapter should recognize method-bearing payload"
    );
}

/// Source guard: `Provider::detect_from_payload` in
/// `claudine/lib/src/provider/methods.rs` must remain a pure walk over
/// `PROVIDERS_DISPLAY_ORDER` and must not contain any provider-specific
/// dispatch (no `Provider::<Variant>` literals, no payload-shape string
/// literals, no helper-function names from the legacy implementation).
/// Closes Finding 2 with an explicit drift backstop.
#[test]
fn detect_from_payload_has_no_provider_specific_branches() {
    use std::fs;
    use std::path::Path;

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/provider/methods.rs");
    let source = fs::read_to_string(&path).expect("methods.rs is readable");
    let stripped = strip_comments(&source);

    let needle = "pub fn detect_from_payload(";
    let start = stripped
        .find(needle)
        .expect("methods.rs declares pub fn detect_from_payload");

    // Locate the function body braces. A simple brace counter is enough
    // because the body is short and the source is comment-stripped.
    let after_signature = stripped[start..]
        .find('{')
        .expect("detect_from_payload has a body opening brace")
        + start;
    let body_bytes = stripped.as_bytes();
    let mut depth = 0i32;
    let mut end = after_signature;
    for (offset, byte) in body_bytes[after_signature..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = after_signature + offset;
                    break;
                }
            }
            _ => {}
        }
    }
    let body = &stripped[after_signature..=end];

    // The body must reference the canonical registry walk.
    assert!(
        body.contains("PROVIDERS_DISPLAY_ORDER"),
        "detect_from_payload body must walk PROVIDERS_DISPLAY_ORDER, got:\n{body}"
    );
    assert!(
        body.contains(".behavior.detect_from_payload(raw)"),
        "detect_from_payload body must call ProviderBehavior::detect_from_payload, got:\n{body}"
    );
    assert!(
        !body.contains(".adapter.detect(raw)"),
        "detect_from_payload body must not call AdapterBehavior::detect directly; \
         ProviderBehavior is the authoritative detection surface. Body:\n{body}"
    );

    // The body must NOT contain any of these forbidden tokens. Each one
    // would represent a regression to provider-specific dispatch in the
    // central method.
    let forbidden_substrings: &[&str] = &[
        "Provider::Claude",
        "Provider::Gemini",
        "Provider::Codex",
        "Provider::OpenCode",
        "Provider::KimiCode",
        "Provider::Goose",
        "Provider::QwenCode",
        "looks_like_codex_payload",
        "hook_event_name",
        "event_type",
        "eventType",
        "event_name",
    ];
    for needle in forbidden_substrings {
        assert!(
            !body.contains(needle),
            "detect_from_payload body must not contain {needle:?}; per-provider dispatch \
             belongs behind `ProviderBehavior::detect_from_payload`. Body:\n{body}"
        );
    }

    // String-form `method` literal (the legacy Kimi shape rule). Match
    // explicitly with surrounding quotes so we don't trip on the trait
    // `detect` method name itself.
    assert!(
        !body.contains("\"method\""),
        "detect_from_payload body must not contain literal \"method\"; \
         per-provider dispatch belongs behind `ProviderBehavior::detect_from_payload`. Body:\n{body}"
    );
}
