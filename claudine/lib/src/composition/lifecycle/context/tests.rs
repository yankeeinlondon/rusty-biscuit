use super::*;
use serde_json::json;

/// Hand-build a [`DiagnosticSnapshot`] for a projection test where no typed
/// `Diagnostic` reconstructs the `detail` (e.g. a cap payload the label-only
/// path cannot rebuild). `message` is irrelevant here: `to_value` reads
/// `LifecycleErrorInfo::msg`, not the embedded snapshot's message.
fn snapshot_with(
    code: &str,
    category: &str,
    disposition: &str,
    origin: &str,
    severity: &str,
    detail: Value,
) -> DiagnosticSnapshot {
    DiagnosticSnapshot {
        schema_version: crate::diagnostics::DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        category: category.to_string(),
        code: code.to_string(),
        disposition: disposition.to_string(),
        origin: origin.to_string(),
        severity: severity.to_string(),
        detail,
        message: String::new(),
        cause: None,
    }
}

#[test]
fn error_info_from_claudine_error_records_kind_variant_msg() {
    let err = ClaudineError::Io(std::io::Error::other("disk full"));
    let info = LifecycleErrorInfo::from_claudine_error(&err);
    assert_eq!(info.kind, "ClaudineError");
    assert_eq!(info.variant, "Io");
    assert!(info.msg.contains("disk full"), "got: {}", info.msg);
}

#[test]
fn error_info_from_harness_error_records_kind_variant_msg() {
    let err = HarnessError::ShellCommandDenied {
        command: "rm -rf /".to_string(),
    };
    let info = LifecycleErrorInfo::from_harness_error(&err);
    assert_eq!(info.kind, "HarnessError");
    assert_eq!(info.variant, "ShellCommandDenied");
    assert!(!info.msg.is_empty());
}

#[test]
fn error_info_from_composition_error_records_kind_variant_msg() {
    let err = CompositionError::NoRunnableProviders;
    let info = LifecycleErrorInfo::from_composition_error(&err);
    assert_eq!(info.kind, "CompositionError");
    assert_eq!(info.variant, "NoRunnableProviders");
    assert!(!info.msg.is_empty());
}

#[test]
fn error_info_to_value_has_kind_variant_msg() {
    let info = LifecycleErrorInfo {
        kind: "ClaudineError",
        variant: "Io".to_string(),
        msg: "disk full".to_string(),
        snapshot: None,
    };
    let value = info.to_value();
    assert_eq!(value.get("kind"), Some(&json!("ClaudineError")));
    assert_eq!(value.get("variant"), Some(&json!("Io")));
    assert_eq!(value.get("msg"), Some(&json!("disk full")));
}

#[test]
fn composition_error_projects_diagnostic_facets_with_legacy_aliases() {
    let err = CompositionError::SchemaLoad {
        source_path: std::path::PathBuf::from("p.md"),
        message: "bad".to_string(),
    };
    let value = LifecycleErrorInfo::from_composition_error(&err).to_value();
    // Legacy aliases stay present, but now read as the deprecated spellings
    // of the faceted contract: `kind` mirrors `category`, `variant` mirrors
    // `code` (spec success criteria), not the internal Rust type/arm names.
    assert_eq!(value.get("kind"), Some(&json!("composition")));
    assert_eq!(value.get("variant"), Some(&json!("composition.schema_load")));
    assert!(value.get("msg").is_some());
    // New typed facets project alongside.
    assert_eq!(value.get("code"), Some(&json!("composition.schema_load")));
    assert_eq!(value.get("category"), Some(&json!("composition")));
    assert_eq!(value.get("disposition"), Some(&json!("correctable")));
    assert_eq!(value.get("origin"), Some(&json!("author")));
    assert_eq!(value.get("severity"), Some(&json!("error")));
    assert_eq!(value["detail"]["source_path"], json!("p.md"));
    // The deprecated aliases are exactly the faceted fields they alias.
    assert_eq!(value.get("kind"), value.get("category"));
    assert_eq!(value.get("variant"), value.get("code"));
}

#[test]
fn legacy_aliases_mirror_category_and_code_across_classifiable_sources() {
    // The spec contract: for every classifiable source, `err.kind` reads as
    // `err.category` and `err.variant` reads as `err.code`. Cover all three
    // typed-Diagnostic builders plus the label-only `from_code` path.
    let classifiable = [
        LifecycleErrorInfo::from_composition_error(&CompositionError::NoRunnableProviders),
        LifecycleErrorInfo::from_harness_error(&HarnessError::ShellCommandDenied {
            command: "rm -rf /".to_string(),
        }),
        LifecycleErrorInfo::from_claudine_error(&ClaudineError::ProviderNotAvailable(
            "codex".to_string(),
        )),
        LifecycleErrorInfo::from_action_failure("rate_limit", "slow down"),
    ];
    for info in classifiable {
        let value = info.to_value();
        assert_eq!(
            value.get("kind"),
            value.get("category"),
            "err.kind must alias err.category"
        );
        assert_eq!(
            value.get("variant"),
            value.get("code"),
            "err.variant must alias err.code"
        );
    }
}

#[test]
fn generic_action_verb_projects_only_legacy_aliases() {
    // A generic lifecycle action verb (not an error_kind) is not
    // classifiable, so no facet keys appear — only the legacy aliases.
    // Compatibility decision: with no faceted equivalent to alias,
    // `err.kind`/`err.variant` fall back to the internal source-type/verb
    // labels so the failure stays observable during migration.
    let info = LifecycleErrorInfo::from_action_failure("set_frontmatter", "boom");
    let value = info.to_value();
    assert_eq!(value.get("kind"), Some(&json!("LifecycleAction")));
    assert_eq!(value.get("variant"), Some(&json!("set_frontmatter")));
    assert!(value.get("code").is_none(), "no facets for a generic verb");
    assert!(value.get("category").is_none());
    assert!(value.get("severity").is_none(), "severity is facet-gated");
    assert!(value.get("detail").is_none());
    // The promoted sugar derives from facets, so it is absent too — a
    // facet-less verb projects only the legacy aliases (catalog §2.6).
    assert!(value.get("is_transient").is_none());
    assert!(value.get("is_throttled").is_none());
    assert!(value.get("is_correctable").is_none());
    assert!(value.get("reset_at").is_none());
    assert!(value.get("retry_after_ms").is_none());
}

#[test]
fn harness_error_projects_diagnostic_facets() {
    // The "harness" failure surface: a HarnessError now carries facets.
    let info = LifecycleErrorInfo::from_harness_error(&HarnessError::ShellCommandDenied {
        command: "rm -rf /".to_string(),
    });
    let value = info.to_value();
    // `kind`/`variant` are the deprecated aliases of `category`/`code`.
    assert_eq!(value.get("kind"), Some(&json!("composition")));
    assert_eq!(value.get("variant"), Some(&json!("composition.shell_expansion")));
    assert_eq!(value.get("code"), Some(&json!("composition.shell_expansion")));
    assert_eq!(value.get("category"), Some(&json!("composition")));
    assert_eq!(value.get("disposition"), Some(&json!("correctable")));
    assert_eq!(value.get("origin"), Some(&json!("author")));
    assert_eq!(value.get("severity"), Some(&json!("error")));
    assert_eq!(value["detail"]["command"], json!("rm -rf /"));
}

#[test]
fn claudine_provider_error_projects_provider_facets() {
    // The "provider" + "top-level Claudine" surface, via `from_claudine_error`.
    let info = LifecycleErrorInfo::from_claudine_error(&ClaudineError::ProviderNotAvailable(
        "codex".to_string(),
    ));
    let value = info.to_value();
    // `kind` is the deprecated alias of `category`.
    assert_eq!(value.get("kind"), Some(&json!("provider")));
    assert_eq!(value.get("code"), Some(&json!("provider.unavailable")));
    assert_eq!(value.get("category"), Some(&json!("provider")));
    assert_eq!(value.get("origin"), Some(&json!("environment")));
    assert_eq!(value["detail"]["provider"], json!("codex"));
}

#[test]
fn claudine_io_error_projects_io_facets() {
    // A top-level Claudine io failure projects an `io.*` code.
    let info = LifecycleErrorInfo::from_claudine_error(&ClaudineError::Io(
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
    ));
    let value = info.to_value();
    assert_eq!(value.get("code"), Some(&json!("io.permission_denied")));
    assert_eq!(value.get("category"), Some(&json!("io")));
}

#[test]
fn action_failure_error_kind_projects_cap_facets() {
    // The "cap" surface: a provider rate-limit reaches the lifecycle `err`
    // as an `error_kind` string carried through `from_action_failure`.
    let info = LifecycleErrorInfo::from_action_failure("rate_limit", "slow down");
    let value = info.to_value();
    assert_eq!(value.get("code"), Some(&json!("cap.rate_limit")));
    assert_eq!(value.get("category"), Some(&json!("cap")));
    assert_eq!(value.get("disposition"), Some(&json!("throttled")));
    assert_eq!(value.get("origin"), Some(&json!("provider")));
}

#[test]
fn action_failure_error_kind_projects_timeout_facets() {
    // The "timeout" surface.
    let info = LifecycleErrorInfo::from_action_failure("step_timeout", "30m of silence");
    let value = info.to_value();
    assert_eq!(value.get("code"), Some(&json!("timeout.step_silence")));
    assert_eq!(value.get("category"), Some(&json!("timeout")));
}

#[test]
fn action_failure_error_kind_projects_runaway_facets() {
    // The "runaway" surface.
    let info = LifecycleErrorInfo::from_action_failure("runaway_volume", "flood");
    let value = info.to_value();
    assert_eq!(value.get("code"), Some(&json!("runaway.volume")));
    assert_eq!(value.get("category"), Some(&json!("runaway")));
    assert_eq!(value.get("disposition"), Some(&json!("unrecoverable")));
}

#[test]
fn action_failure_error_kind_projects_provider_facets() {
    // The "provider" run surface, via an `agent_failure` error_kind.
    let info = LifecycleErrorInfo::from_action_failure("agent_failure", "exit 1");
    let value = info.to_value();
    assert_eq!(value.get("code"), Some(&json!("provider.exited")));
    assert_eq!(value.get("category"), Some(&json!("provider")));
}

#[test]
fn severity_projects_for_classifiable_errors() {
    // Typed-Diagnostic path: a correctable composition error defaults to
    // `error` severity (catalog §1).
    let value =
        LifecycleErrorInfo::from_composition_error(&CompositionError::NoRunnableProviders)
            .to_value();
    assert_eq!(value.get("severity"), Some(&json!("error")));

    // Label-only `from_code` path: a throttled cap defaults to `warning`.
    let value = LifecycleErrorInfo::from_action_failure("rate_limit", "slow down").to_value();
    assert_eq!(value.get("disposition"), Some(&json!("throttled")));
    assert_eq!(value.get("severity"), Some(&json!("warning")));
}

#[test]
fn throttled_cap_promotes_is_throttled_and_null_cap_fields() {
    // A rate-limit cap reaches `err` via `from_action_failure`, so its
    // facets come from the `from_code` path → `detail` is `Null`. The
    // predicate sugar reflects the throttled disposition; the cap fields
    // promote to `null` because the label can't reconstruct them.
    let info = LifecycleErrorInfo::from_action_failure("rate_limit", "slow down");
    let value = info.to_value();
    assert_eq!(value.get("disposition"), Some(&json!("throttled")));
    assert_eq!(value.get("is_throttled"), Some(&json!(true)));
    assert_eq!(value.get("is_transient"), Some(&json!(false)));
    assert_eq!(value.get("is_correctable"), Some(&json!(false)));
    assert_eq!(value.get("reset_at"), Some(&Value::Null));
    assert_eq!(value.get("retry_after_ms"), Some(&Value::Null));
}

#[test]
fn transient_error_promotes_is_transient_only() {
    let info = LifecycleErrorInfo::from_action_failure("step_timeout", "30m of silence");
    let value = info.to_value();
    assert_eq!(value.get("disposition"), Some(&json!("transient")));
    assert_eq!(value.get("is_transient"), Some(&json!(true)));
    assert_eq!(value.get("is_throttled"), Some(&json!(false)));
    assert_eq!(value.get("is_correctable"), Some(&json!(false)));
}

#[test]
fn correctable_error_promotes_is_correctable_only() {
    // A composition error is `correctable`; its `detail` carries no cap
    // fields, so `reset_at`/`retry_after_ms` promote to `null`.
    let value =
        LifecycleErrorInfo::from_composition_error(&CompositionError::NoRunnableProviders)
            .to_value();
    assert_eq!(value.get("disposition"), Some(&json!("correctable")));
    assert_eq!(value.get("is_correctable"), Some(&json!(true)));
    assert_eq!(value.get("is_transient"), Some(&json!(false)));
    assert_eq!(value.get("is_throttled"), Some(&json!(false)));
    assert_eq!(value.get("reset_at"), Some(&Value::Null));
    assert_eq!(value.get("retry_after_ms"), Some(&Value::Null));
}

#[test]
fn cap_detail_promotes_reset_at_and_retry_after_ms_to_top_level() {
    // No typed `Diagnostic` reconstructs a cap `detail` (the cap codes reach
    // `err` via the label-only `from_code` path), so hand-build facets whose
    // `detail` carries the cap fields and assert they promote out of detail.
    let info = LifecycleErrorInfo {
        kind: "LifecycleAction",
        variant: "rate_limit".to_string(),
        msg: "slow down".to_string(),
        snapshot: Some(Box::new(snapshot_with(
            "cap.rate_limit",
            "cap",
            "throttled",
            "provider",
            "warning",
            json!({
                "provider": "claude",
                "reset_at": "2026-06-28T17:30:00Z",
                "retry_after_ms": 5_400_000u64,
            }),
        ))),
    };
    let value = info.to_value();
    // Promoted to the top level …
    assert_eq!(value.get("reset_at"), Some(&json!("2026-06-28T17:30:00Z")));
    assert_eq!(value.get("retry_after_ms"), Some(&json!(5_400_000u64)));
    assert_eq!(value.get("is_throttled"), Some(&json!(true)));
    // … while `detail.*` stays canonical.
    assert_eq!(value["detail"]["reset_at"], json!("2026-06-28T17:30:00Z"));
    assert_eq!(value["detail"]["retry_after_ms"], json!(5_400_000u64));
}

#[test]
fn null_detail_field_promotes_to_null() {
    // A facet `detail` object that carries no cap fields yields `null`
    // promoted values — not an error, per the null-sentinel convention.
    let info = LifecycleErrorInfo {
        kind: "LifecycleAction",
        variant: "x".to_string(),
        msg: "m".to_string(),
        snapshot: Some(Box::new(snapshot_with(
            "document.missing_frontmatter",
            "document",
            "correctable",
            "provider",
            "error",
            json!({ "doc": "spec.md", "property": "status" }),
        ))),
    };
    let value = info.to_value();
    assert_eq!(value.get("reset_at"), Some(&Value::Null));
    assert_eq!(value.get("retry_after_ms"), Some(&Value::Null));
    assert_eq!(value.get("is_correctable"), Some(&json!(true)));
}

#[test]
fn timing_to_value_omits_missing_fields() {
    let timing = LifecycleTiming {
        document_ms: Some(42),
        total_ms: None,
        step_ms: None,
    };
    let value = timing.to_value();
    assert_eq!(value.get("document_ms"), Some(&json!(42u64)));
    assert!(value.get("total_ms").is_none());
    assert!(value.get("step_ms").is_none());
}

#[test]
fn current_to_value_has_ctx_and_env() {
    let current = LifecycleCurrent {
        ctx: json!({"agent": "claude"}),
        env: json!({"HOME": "/tmp"}),
    };
    let value = current.to_value();
    assert_eq!(value.get("ctx").unwrap().get("agent"), Some(&json!("claude")));
    assert_eq!(value.get("env").unwrap().get("HOME"), Some(&json!("/tmp")));
}

/// Build an [`EffectiveState`] over `fm` with a cheap (sniff-free) context.
fn state(fm: Value) -> darkmatter::markdown::compose::EffectiveState {
    use darkmatter::markdown::compose::{ComposeContext, EffectiveStateBuilder};
    let fm: std::collections::HashMap<String, Value> = fm
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .with_context(ComposeContext::capture_for_content(std::path::Path::new("."), ""))
        .build()
        .unwrap()
}

#[test]
fn injected_globals_attaches_err_timing_current() {
    let info = LifecycleErrorInfo {
        kind: "ClaudineError",
        variant: "Io".to_string(),
        msg: "disk full".to_string(),
        snapshot: None,
    };
    let timing = LifecycleTiming {
        document_ms: Some(100),
        total_ms: None,
        step_ms: None,
    };
    let current = LifecycleCurrent {
        ctx: json!({"agent": "codex"}),
        env: json!({"DEBUG": "1"}),
    };
    let globals = lifecycle_injected_globals(Some(&info), Some(&timing), Some(&current));
    assert!(globals.contains_key("err"));
    assert!(globals.contains_key("timing"));
    assert!(globals.contains_key("current"));
}

#[test]
fn injected_globals_omits_unattached() {
    let globals = lifecycle_injected_globals(None, None, None);
    assert!(globals.is_empty());
}

#[test]
fn err_global_resolves_through_dm2_subtree() {
    use darkmatter::markdown::compose::subtree::SubtreeCompose;
    let info = LifecycleErrorInfo {
        kind: "ClaudineError",
        variant: "Io".to_string(),
        msg: "disk full".to_string(),
        snapshot: None,
    };
    let globals = lifecycle_injected_globals(Some(&info), None, None);
    let state = state(json!({}));
    let resolved = SubtreeCompose::new(&json!("{{err.msg}}"), &state)
        .with_globals(globals)
        .compose()
        .unwrap();
    assert_eq!(resolved, json!("disk full"));
}

#[test]
fn timing_global_resolves_through_dm2_subtree() {
    use darkmatter::markdown::compose::subtree::SubtreeCompose;
    let timing = LifecycleTiming {
        document_ms: Some(100),
        total_ms: None,
        step_ms: None,
    };
    let globals = lifecycle_injected_globals(None, Some(&timing), None);
    let state = state(json!({}));
    let resolved = SubtreeCompose::new(&json!("took {{timing.document_ms}}ms"), &state)
        .with_globals(globals)
        .compose()
        .unwrap();
    assert_eq!(resolved, json!("took 100ms"));
}

#[test]
fn doc_namespace_reaches_literal_err_property_through_dm2() {
    // `doc.err` reaches a literal frontmatter `err` property, not the
    // lifecycle `err` global — `doc` is never an injected global root.
    use darkmatter::markdown::compose::subtree::SubtreeCompose;
    let info = LifecycleErrorInfo {
        kind: "ClaudineError",
        variant: "Io".to_string(),
        msg: "disk full".to_string(),
        snapshot: None,
    };
    let globals = lifecycle_injected_globals(Some(&info), None, None);
    let state = state(json!({"err": "literal-value"}));
    let resolved = SubtreeCompose::new(&json!("{{doc.err}} / {{err.msg}}"), &state)
        .with_globals(globals)
        .compose()
        .unwrap();
    assert_eq!(resolved, json!("literal-value / disk full"));
}

#[test]
#[serial_test::serial(env_lifecycle_current)]
fn capture_env_reflects_live_process_environment() {
    let key = "CLAUDINE_TEST_CAPTURE_ENV_LIVE";
    // SAFETY: serialized via #[serial]; no other thread reads this var.
    unsafe { std::env::set_var(key, "live-value") };
    let env = LifecycleCurrent::capture_env();
    unsafe { std::env::remove_var(key) };
    assert_eq!(env.get(key), Some(&json!("live-value")));
}

#[test]
fn capture_env_only_leaves_ctx_empty() {
    let current = LifecycleCurrent::capture_env_only();
    assert_eq!(current.ctx, json!({}));
    assert!(current.env.is_object(), "env is a JSON object snapshot");
}

#[test]
fn capture_at_event_populates_ctx_and_env() {
    let current = LifecycleCurrent::capture_at_event(std::path::Path::new("."));
    // `ctx.today` is always captured (date/time group, zero I/O).
    assert!(
        current.ctx.get("today").and_then(|v| v.as_str()).is_some(),
        "ctx snapshot carries today: {:?}",
        current.ctx
    );
    assert!(current.env.is_object());
}

#[test]
fn timing_from_instants_populates_document_ms_and_total_ms() {
    let start = std::time::Instant::now();
    let run_start = start;
    // Spin until at least 1ms elapsed so the conversion is observably > 0.
    let at = loop {
        let now = std::time::Instant::now();
        if now.duration_since(start).as_millis() >= 1 {
            break now;
        }
    };
    let timing = LifecycleTiming::from_instants(start, Some(run_start), at);
    let doc = timing.document_ms.expect("document_ms is populated");
    let total = timing.total_ms.expect("total_ms populated with run_start");
    assert!(doc >= 1, "document_ms is monotonic non-decreasing: {doc}");
    assert_eq!(doc, total, "document and run start coincide here");
    assert!(timing.step_ms.is_none(), "step_ms stays None outside a sequence");
}

#[test]
fn timing_from_instants_omits_total_ms_without_run_start() {
    let start = std::time::Instant::now();
    let timing = LifecycleTiming::from_instants(start, None, std::time::Instant::now());
    assert!(timing.document_ms.is_some());
    assert!(timing.total_ms.is_none());
}

/// Late-binding contract: a stack `when:` clause reacts to an environment
/// value present in the `current.env` snapshot the production builders
/// attach at **event time**, distinct from a value snapshotted at "prepare"
/// time. Resolved through DM2's layered lookup over the injected `current`
/// global, proving the binding is late.
#[test]
#[serial_test::serial(env_lifecycle_current)]
fn when_clause_reacts_to_env_changed_after_prepare() {
    use darkmatter::markdown::compose::expression::{evaluate, is_truthy, parse};
    use darkmatter::markdown::compose::subtree::LayeredLookup;

    let key = "CLAUDINE_TEST_LATE_BINDING_MYVAR";
    // SAFETY: serialized via #[serial]; no other thread reads this var.

    // "Prepare time": the variable holds an old value. A `current.env`
    // snapshot captured now would carry `old`.
    unsafe { std::env::set_var(key, "old") };
    let prepare_snapshot = LifecycleCurrent::capture_env_only();

    // A side effect / external change happens AFTER prepare.
    unsafe { std::env::set_var(key, "x") };
    // The production builders capture `current` at EVENT time, so they see
    // the post-change value.
    let event_snapshot = LifecycleCurrent::capture_env_only();
    unsafe { std::env::remove_var(key) };

    let base = state(json!({}));
    let expr = parse(&format!("current.env.{key} == 'x'")).expect("parses");

    // Against the event-time snapshot, the guard fires (late binding).
    let event_globals =
        lifecycle_injected_globals(None, None, Some(&event_snapshot));
    let event_lookup = LayeredLookup::new(&base, &event_globals, None);
    let event_value = evaluate(&expr, &event_lookup).expect("evaluates");
    assert!(
        is_truthy(&event_value),
        "when clause fires on the value present at event time"
    );

    // Against the prepare-time snapshot, the same guard does NOT fire,
    // proving the reaction is to the late-bound value, not the prepare one.
    let prepare_globals =
        lifecycle_injected_globals(None, None, Some(&prepare_snapshot));
    let prepare_lookup = LayeredLookup::new(&base, &prepare_globals, None);
    let prepare_value = evaluate(&expr, &prepare_lookup).expect("evaluates");
    assert!(
        !is_truthy(&prepare_value),
        "the prepare-time snapshot still holds the old value"
    );
}

// --- Phase 5: one selection, hygienic `err.msg`, one-level `err.cause` ------

/// `err.msg` feeds TTS, webhooks, and desktop notifications verbatim, so it is
/// hygiened at the *constructor*, not at each producer. The nastiest input a
/// producer can hand over — multi-line, escape-bearing, and far over the cap —
/// must still come out usable.
#[test]
fn err_msg_is_hygiened_for_a_hostile_action_failure() {
    let hostile = format!(
        "\u{1b}[31mboom\u{1b}[0m\nsecond line\r\nthird line\n{}",
        "x".repeat(500)
    );
    let value = LifecycleErrorInfo::from_action_failure("shell", hostile).to_value();
    let msg = value["msg"].as_str().expect("msg is a string");

    assert!(!msg.is_empty());
    assert!(!msg.contains('\u{1b}'), "escape bytes survived: {msg:?}");
    assert!(!msg.contains('\n') && !msg.contains('\r'), "not one line: {msg:?}");
    assert!(msg.chars().count() <= 240, "over cap ({}): {msg:?}", msg.chars().count());
    assert!(msg.contains("boom"), "headline lost: {msg:?}");
}

/// The same hygiene contract, asserted over every constructor that can reach
/// `err.msg` — a typed error of each registered kind plus the facet-less
/// action-failure path.
#[test]
fn every_err_msg_producer_is_single_line_escape_free_and_capped() {
    let cases: Vec<(&str, LifecycleErrorInfo)> = vec![
        (
            "claudine",
            LifecycleErrorInfo::from_claudine_error(&ClaudineError::Io(
                std::io::Error::other("disk full"),
            )),
        ),
        (
            "harness",
            LifecycleErrorInfo::from_harness_error(&HarnessError::ShellCommandDenied {
                command: "rm -rf /".to_string(),
            }),
        ),
        (
            "composition",
            LifecycleErrorInfo::from_composition_error(&CompositionError::NoRunnableProviders),
        ),
        (
            "action",
            LifecycleErrorInfo::from_action_failure("shell", "command `x` exited with code 2"),
        ),
        (
            "error_or_action-typed",
            LifecycleErrorInfo::from_error_or_action(
                "materialize",
                &CompositionError::NoRunnableProviders,
            ),
        ),
        (
            "error_or_action-untyped",
            LifecycleErrorInfo::from_error_or_action("materialize", &std::io::Error::other("io")),
        ),
    ];

    for (name, info) in cases {
        assert!(!info.msg.is_empty(), "`{name}` produced an empty err.msg");
        assert!(!info.msg.contains('\u{1b}'), "`{name}` leaked escapes: {:?}", info.msg);
        assert!(
            !info.msg.contains('\n') && !info.msg.contains('\r'),
            "`{name}` is multi-line: {:?}",
            info.msg
        );
        assert!(
            info.msg.chars().count() <= 240,
            "`{name}` exceeds the cap: {:?}",
            info.msg
        );
    }
}

/// The provider attempt cascade keeps its ratified precedence and its
/// budget-reserved `(attempt N)` suffix: the constructor's hygiene pass is
/// idempotent over an already-hygiened `failure_message`, so re-running it
/// cannot re-truncate or re-order the cascade's choice.
#[test]
fn provider_failure_message_precedence_survives_the_constructor() {
    let outcome = crate::harness::AttemptOutcome {
        attempt: 3,
        session_id: None,
        final_response: String::new(),
        exit_code: 1,
        termination: crate::harness::ProcessTermination::Completed,
        stderr_text: Some("noise\nlast stderr line".to_string()),
        error_kind: Some("agent_failure".to_string()),
        guard_context: None,
        error_message: Some("provider said no".to_string()),
        timeout_secs: None,
    };
    let expected = crate::harness::failure_message(&outcome, 3);
    // The cascade prefers `error_message` over stderr, and keeps `(attempt 3)`.
    assert_eq!(expected, "provider said no (attempt 3)");

    let info = LifecycleErrorInfo::from_action_failure("agent_failure", expected.clone());
    assert_eq!(info.msg, expected, "the constructor perturbed the cascade's message");
}

/// A registered code must never project a *top-level* `null` detail, even on
/// the label-only path that knows no per-instance values: `err.detail.reset_at`
/// has to read as "declared but unknown", not blow up against a scalar
/// (spec §D7).
#[test]
fn every_error_kind_code_projects_a_catalog_shaped_detail_object() {
    for spec in crate::diagnostics::CODES {
        let Some(snapshot) = DiagnosticSnapshot::from_code(spec.code, String::new()) else {
            continue;
        };
        let detail = &snapshot.detail;
        assert!(
            detail.is_object(),
            "`{}` projects a top-level non-object detail: {detail}",
            spec.code
        );
        for field in spec.detail {
            assert!(
                detail.get(field).is_some(),
                "`{}` omits its declared detail key `{field}`: {detail}",
                spec.code
            );
        }
    }
}

/// `err.cause.*` is a strict one-level projection of the next *registered*
/// diagnostic — unregistered prose in between is walked through.
#[test]
fn err_cause_projects_one_level_of_registered_diagnostic() {
    let inner = CompositionError::FileNotFound("missing.md".to_string());
    let wrapped = CompositionError::LifecycleEvaluationAlreadyEmitted {
        inner: Box::new(inner),
    };
    // The transparent wrapper is walked through, so the *selected* diagnostic
    // is the inner one and it has no registered cause below it.
    let value = LifecycleErrorInfo::from_composition_error(&wrapped).to_value();
    assert_eq!(value["code"], json!("composition.invalid_file_reference"));
    assert_eq!(value["cause"], Value::Null, "a leaf must project a null cause");

    // A semantic wrapper over a *registered* source projects that source.
    let err = CompositionError::InvalidFileReference {
        context: Box::new(crate::composition::FileReferenceContext {
            source_path: std::path::PathBuf::from("route.md"),
            event: Some("initialize".to_string()),
            property: "initialize.stack[0].proxy".to_string(),
            reference: "no/such/target.md".to_string(),
            hint: "name an existing document".to_string(),
        }),
        source: HarnessError::PathResolutionFailed {
            raw: "no/such/target.md".to_string(),
            failure: crate::harness::PathResolutionFailure::TargetMissing,
            source_path: Some(std::path::PathBuf::from("route.md")),
            resolved: Some(std::path::PathBuf::from("/repo/no/such/target.md")),
        },
    };
    let value = LifecycleErrorInfo::from_composition_error(&err).to_value();
    let cause = &value["cause"];
    assert!(cause.is_object(), "expected a projected cause, got {cause}");
    assert_eq!(cause["code"], json!("composition.invalid_file_reference"));
    assert!(cause["message"].as_str().is_some_and(|m| !m.is_empty()));
    // v1 exposes exactly one level: `err.cause.cause` is unrepresentable.
    assert!(
        cause.get("cause").is_none(),
        "`err.cause.cause` must not be exposed in v1: {cause}"
    );
}

/// A facet-less action failure projects no `cause` key at all — a cause is only
/// meaningful below a selected diagnostic.
#[test]
fn facet_less_action_failure_projects_no_cause() {
    let value = LifecycleErrorInfo::from_action_failure("shell", "boom").to_value();
    assert!(value.get("facets").is_none());
    assert!(value.get("cause").is_none(), "got: {value}");
}

/// `from_error_or_action` keeps a typed error's facets where
/// `from_action_failure(verb, err.to_string())` would have flattened them to
/// prose — and still falls back to the verb label for genuinely untyped prose.
#[test]
fn from_error_or_action_prefers_facets_and_falls_back_to_the_verb() {
    let typed = LifecycleErrorInfo::from_error_or_action(
        "materialize",
        &CompositionError::FileNotFound("missing.md".to_string()),
    );
    let value = typed.to_value();
    assert_eq!(value["code"], json!("composition.invalid_file_reference"));
    // The deprecated aliases mirror the facets for a classifiable error.
    assert_eq!(value["variant"], json!("composition.invalid_file_reference"));
    assert_eq!(value["kind"], json!("composition"));

    let untyped =
        LifecycleErrorInfo::from_error_or_action("materialize", &std::io::Error::other("boom"));
    assert!(untyped.snapshot.is_none(), "std::io::Error is not a Claudine diagnostic");
    let value = untyped.to_value();
    assert_eq!(value["variant"], json!("materialize"));
    assert_eq!(value["kind"], json!("LifecycleAction"));
}
