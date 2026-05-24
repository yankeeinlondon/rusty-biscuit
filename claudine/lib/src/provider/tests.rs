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
            "capabilities",
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

fn path_templates_raw(paths: &[super::PathTemplate]) -> Vec<&'static str> {
    paths.iter().map(super::PathTemplate::raw).collect()
}

fn path_bufs_raw(paths: &[std::path::PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

fn assert_legacy_paths_in_catalog(
    provider: Provider,
    label: &str,
    legacy_paths: &[std::path::PathBuf],
    catalog_paths: &[super::PathTemplate],
) {
    let catalog_raw = path_templates_raw(catalog_paths);
    for path in path_bufs_raw(legacy_paths) {
        assert!(
            catalog_raw.iter().any(|raw| *raw == path),
            "{provider:?}: legacy {label} path {path:?} missing from typed catalog {catalog_raw:?}"
        );
    }
}

fn delivery_supported(delivery: super::SystemPromptDelivery) -> bool {
    !matches!(delivery, super::SystemPromptDelivery::Unsupported)
}

fn replacement_supported(info: &super::ProviderInfo) -> bool {
    delivery_supported(info.system_prompt.replace.interactive)
        || delivery_supported(info.system_prompt.replace.non_interactive)
}

fn yolo_matches_legacy(legacy: Option<&str>, yolo: super::YoloSupport) -> bool {
    let Some(legacy) = legacy else {
        return matches!(yolo, super::YoloSupport::None);
    };
    let normalized = legacy.replace(' ', "=");
    match yolo {
        super::YoloSupport::None => false,
        super::YoloSupport::DirectFlag { native_flag } => {
            legacy == native_flag || normalized == native_flag
        }
        super::YoloSupport::DirectFlagWithAlias {
            native_flag,
            aliases,
        } => {
            legacy == native_flag
                || normalized == native_flag
                || aliases
                    .iter()
                    .any(|alias| legacy == *alias || normalized == *alias)
        }
        super::YoloSupport::NonInteractiveOnly {
            non_interactive_flag,
        } => legacy == non_interactive_flag || normalized == non_interactive_flag,
        super::YoloSupport::EnvVar { env_var, value } => {
            legacy.contains(env_var) && legacy.contains(value)
        }
    }
}

fn has_catalog_resume_entrypoint(info: &super::ProviderInfo) -> bool {
    info.entrypoints.iter().any(|entrypoint| {
        entrypoint.required_flags.iter().any(|flag| {
            matches!(
                *flag,
                "-c" | "--continue" | "-r" | "--resume" | "resume" | "--resume-session"
            )
        }) || entrypoint.subcommand == Some("resume")
    })
}

#[test]
fn agent_capabilities_identity_and_docs_match_typed_catalog() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let caps = info.agent_capabilities();

        assert_eq!(caps.meta.id, info.provider, "{provider:?}: id mismatch");
        assert!(
            caps.meta.display_name == info.display_name
                || caps.meta.display_name.contains(info.display_name),
            "{provider:?}: legacy display name {:?} does not follow typed display policy {:?}",
            caps.meta.display_name,
            info.display_name
        );
        assert_eq!(
            caps.meta.binary, info.binary,
            "{provider:?}: binary mismatch"
        );
        assert!(
            caps.docs.homepage.is_some()
                || caps.docs.docs.is_some()
                || caps.docs.skills_docs.is_some()
                || caps.docs.slash_docs.is_some()
                || caps.docs.subagents_docs.is_some()
                || caps.docs.scripts_docs.is_some(),
            "{provider:?}: legacy docs surface is empty while typed docs_url is {:?}",
            info.docs_url
        );
    }
}

#[test]
fn agent_capabilities_config_paths_match_typed_catalog() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let caps = info.agent_capabilities();

        assert_legacy_paths_in_catalog(
            provider,
            "user config",
            &caps.config.user_files,
            info.config_paths,
        );
        assert_legacy_paths_in_catalog(
            provider,
            "project config",
            &caps.config.project_files,
            info.config_paths,
        );
        assert_legacy_paths_in_catalog(
            provider,
            "local config",
            &caps.config.local_files,
            info.config_paths,
        );
    }
}

#[test]
fn agent_capabilities_runtime_matches_typed_catalog() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let caps = info.agent_capabilities();
        let non_interactive = &caps.runtime.non_interactive;

        assert_eq!(
            non_interactive.supported,
            !info.entrypoints.is_empty(),
            "{provider:?}: non-interactive support drifted"
        );
        assert_eq!(
            non_interactive.structured_output_supported,
            info.stream_protocol.is_some()
                || info
                    .output_formats
                    .iter()
                    .any(|format| !matches!(format.format, super::OutputFormat::Text)),
            "{provider:?}: structured output support drifted"
        );
        if has_catalog_resume_entrypoint(info) || !non_interactive.resume_supported {
            assert_eq!(
                non_interactive.resume_supported,
                has_catalog_resume_entrypoint(info),
                "{provider:?}: resume support drifted where the typed catalog can represent it"
            );
        }

        for entrypoint in info.entrypoints {
            let mut fragments = vec![info.binary];
            if let Some(subcommand) = entrypoint.subcommand {
                fragments.push(subcommand);
            }
            fragments.extend(entrypoint.required_flags.iter().copied());
            assert!(
                non_interactive
                    .entrypoints
                    .iter()
                    .any(|legacy| { fragments.iter().all(|fragment| legacy.contains(fragment)) }),
                "{provider:?}: typed entrypoint fragments {fragments:?} missing from legacy entrypoints {:?}",
                non_interactive.entrypoints
            );
        }

        for output_format in info.output_formats {
            assert!(
                non_interactive
                    .output_formats
                    .iter()
                    .any(|legacy| legacy.contains(output_format.native_name)),
                "{provider:?}: typed output format {:?} missing from legacy output formats {:?}",
                output_format.native_name,
                non_interactive.output_formats
            );
        }
    }
}

#[test]
fn agent_capabilities_system_prompt_permissions_and_reasoning_match_typed_catalog() {
    use crate::agents::ReasoningStyle;

    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let caps = info.agent_capabilities();

        let legacy_memory = &caps.runtime.system_prompt.memory_files;
        for memory_file in info.memory_files {
            assert!(
                legacy_memory.contains(&memory_file.raw()),
                "{provider:?}: typed memory file {:?} missing from legacy system prompt memory files {:?}",
                memory_file.raw(),
                legacy_memory
            );
        }
        assert_eq!(
            caps.runtime.system_prompt.full_replacement_supported,
            replacement_supported(info),
            "{provider:?}: system prompt replacement support drifted"
        );

        assert!(
            yolo_matches_legacy(caps.runtime.permissions.yolo_equivalent, info.yolo),
            "{provider:?}: legacy YOLO {:?} does not match typed {:?}",
            caps.runtime.permissions.yolo_equivalent,
            info.yolo
        );

        let reasoning = &caps.runtime.reasoning;
        match info.reasoning {
            super::ReasoningSupport::NotSupported | super::ReasoningSupport::NotDocumented => {
                assert!(
                    matches!(reasoning.style, ReasoningStyle::NotDocumented)
                        || reasoning.levels_or_controls.is_empty(),
                    "{provider:?}: legacy reasoning should be unsupported/undocumented, got {:?}",
                    reasoning
                );
            }
            super::ReasoningSupport::NamedLevels { levels, .. } => {
                assert_eq!(
                    reasoning.style,
                    ReasoningStyle::NamedLevels,
                    "{provider:?}: reasoning style drifted"
                );
                for level in levels {
                    assert!(
                        reasoning.levels_or_controls.contains(level),
                        "{provider:?}: typed reasoning level {level:?} missing from legacy {:?}",
                        reasoning.levels_or_controls
                    );
                }
            }
            super::ReasoningSupport::NumericBudget { flag, .. } => {
                assert_eq!(
                    reasoning.style,
                    ReasoningStyle::NumericBudget,
                    "{provider:?}: reasoning style drifted"
                );
                assert!(
                    reasoning.levels_or_controls.contains(&flag),
                    "{provider:?}: typed reasoning flag {flag:?} missing from legacy {:?}",
                    reasoning.levels_or_controls
                );
            }
            super::ReasoningSupport::BinaryToggle { on, off, .. } => {
                assert_eq!(
                    reasoning.style,
                    ReasoningStyle::BinaryToggle,
                    "{provider:?}: reasoning style drifted"
                );
                for control in [on, off] {
                    assert!(
                        reasoning.levels_or_controls.contains(&control),
                        "{provider:?}: typed reasoning control {control:?} missing from legacy {:?}",
                        reasoning.levels_or_controls
                    );
                }
            }
            super::ReasoningSupport::ProviderSpecific(_) => {
                assert!(
                    !matches!(reasoning.style, ReasoningStyle::NotDocumented)
                        && !reasoning.levels_or_controls.is_empty(),
                    "{provider:?}: provider-specific typed reasoning should retain legacy controls, got {:?}",
                    reasoning
                );
            }
        }
    }
}

#[test]
fn agent_capabilities_logging_and_known_gaps_match_typed_catalog() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let caps = info.agent_capabilities();
        let logging = &caps.runtime.logging;

        let session_log_paths = path_templates_raw(info.session_log_paths);
        for legacy in &logging.session_locations {
            assert!(
                session_log_paths.contains(legacy),
                "{provider:?}: legacy session log location {legacy:?} missing from typed session_log_paths {session_log_paths:?}"
            );
        }

        let legacy_log_locations = &logging.log_locations;
        for typed in path_templates_raw(info.session_locations) {
            assert!(
                legacy_log_locations.contains(&typed),
                "{provider:?}: typed session location {typed:?} missing from legacy log_locations {legacy_log_locations:?}"
            );
        }

        for legacy_gap in &caps.confidence.gaps {
            assert!(
                info.known_gaps
                    .iter()
                    .any(|gap| { gap.note == *legacy_gap || gap.tracker == Some(*legacy_gap) }),
                "{provider:?}: legacy confidence gap {legacy_gap:?} missing from typed known_gaps {:?}",
                info.known_gaps
            );
        }
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
/// Strip Rust `//` line comments and `/* ... */` block comments from
/// `src` so commented-out examples don't trip source-scan tests. Does NOT
/// attempt to handle Rust strings containing `//` — false positives from
/// string literals are rare enough to allow-list explicitly.
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
        // `stream/providers/mod.rs` contains the `SemanticParser` factory
        // function that matches on `Provider` to construct the correct
        // provider-specific stream parser. This is an intentional
        // per-provider dispatch site introduced by Phase 2.5 of the
        // Sentrux quality remediation plan.
        "src/stream/providers/mod.rs",
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
        let has_acp_event = AgenticEvent::ALL
            .iter()
            .any(|event| info.event_mapping.support_level(*event).is_acp());
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
        // Goose, Qwen, and Roo do not detect via raw payload shape today.
        Provider::Goose | Provider::QwenCode | Provider::RooCode => return None,
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
/// portability data and excludes the legacy `AgentCapabilities` tree.
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
        "Provider::RooCode",
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
