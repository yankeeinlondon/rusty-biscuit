use super::*;
use crate::config::claudine_config::{
    DetailedModelOverride, ModelOverrideMode, ProviderModelOverride,
};

/// OpenCode's `ShellCommand` identity, destructured from the
/// generated catalog so tests stay truthful to the committed data.
fn opencode_command() -> (&'static str, &'static [&'static str]) {
    match provider_info(Provider::OpenCode).model_catalog_source {
        ModelCatalogSource::ShellCommand { program, args } => (program, args),
        other => panic!("opencode must be ShellCommand-sourced, got {other:?}"),
    }
}

#[test]
fn service_validates_baseline_model() {
    let service = ModelCatalogService::new();
    assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
    assert!(service.is_valid(Provider::Claude, "claude-opus-4-8"));
}

/// Aliases from the expected-offering records (`opus`, `flash`) are
/// part of the validation baseline: users author them in frontmatter
/// `model:` hints.
#[test]
fn service_validates_offering_alias() {
    let service = ModelCatalogService::new();
    assert!(service.is_valid(Provider::Claude, "opus"));
    assert!(service.is_valid(Provider::Gemini, "flash"));
}

/// Goose had no compiled model list before the baseline flip; its
/// expected offerings made it validatable.
#[test]
fn service_validates_goose_from_expected_offerings() {
    let service = ModelCatalogService::new();
    assert!(service.is_valid(Provider::Goose, "gpt-5"));
}

/// Ids under a declared offering-source namespace (local runners)
/// pass validation even though their model population cannot be
/// enumerated statically. The `/` separator is mandatory.
#[test]
fn service_accepts_offering_source_prefix() {
    let service = ModelCatalogService::new();
    assert!(service.is_valid(Provider::OpenCode, "ollama/llama3.3"));
    assert!(service.is_valid(Provider::OpenCode, "OLLAMA/llama3.3"));
    assert!(!service.is_valid(Provider::OpenCode, "ollamallama3.3"));
    assert!(!service.is_valid(Provider::OpenCode, "no-such-runner/model"));
}

/// The on-disk listing cache never feeds validation — for any
/// provider, no-source or shell-sourced alike.
#[test]
fn validation_ignores_listing_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    for provider in [Provider::Codex, Provider::OpenCode] {
        let stale = ModelCacheEntry {
            provider,
            models: vec!["cached-only-model".into()],
            fetched_at: chrono::Utc::now(),
        };
        service.cache.write(&stale).unwrap();
        assert!(!service.is_valid(provider, "cached-only-model"));
    }
    assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
}

#[test]
fn service_rejects_unknown_model() {
    let service = ModelCatalogService::new();
    assert!(!service.is_valid(Provider::Codex, "not-a-real-model-xyz"));
    assert!(!service.is_valid(Provider::Claude, "not-a-real-model-xyz"));
}

#[test]
fn service_case_insensitive_validation() {
    let service = ModelCatalogService::new();
    assert!(service.is_valid(Provider::Codex, "GPT-5.5"));
    assert!(service.is_valid(Provider::Claude, "CLAUDE-OPUS-4-8"));
}

#[test]
fn service_with_additive_override() {
    let mut overrides = HashMap::new();
    overrides.insert(
        Provider::Codex,
        ProviderModelOverride::AddList(vec!["custom-codex-model".into()]),
    );
    let service = ModelCatalogService::with_overrides(overrides);

    assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
    assert!(service.is_valid(Provider::Codex, "custom-codex-model"));
}

#[test]
fn service_with_replace_override() {
    let mut overrides = HashMap::new();
    overrides.insert(
        Provider::Codex,
        ProviderModelOverride::Detailed(DetailedModelOverride {
            mode: ModelOverrideMode::Replace,
            values: vec!["only-this-model".into()],
        }),
    );
    let service = ModelCatalogService::with_overrides(overrides);

    assert!(!service.is_valid(Provider::Codex, "gpt-5.5"));
    assert!(service.is_valid(Provider::Codex, "only-this-model"));
}

#[test]
fn override_catalog_id_hit_miss_and_case_insensitive() {
    use crate::config::claudine_config::{ModelOverrideEntry, ModelOverrideValue};

    let mut overrides = HashMap::new();
    overrides.insert(
        Provider::Codex,
        ProviderModelOverride::Detailed(DetailedModelOverride {
            mode: ModelOverrideMode::Add,
            values: vec![
                "plain-model".into(),
                ModelOverrideValue::Entry(ModelOverrideEntry {
                    id: "joined-model".into(),
                    catalog_id: Some("openai/joined-model@1.0".into()),
                }),
            ],
        }),
    );
    overrides.insert(
        Provider::Gemini,
        ProviderModelOverride::AddList(vec!["list-model".into()]),
    );
    let service = ModelCatalogService::with_overrides(overrides);

    // Hit, including case-insensitive id match.
    assert_eq!(
        service.override_catalog_id(Provider::Codex, "joined-model"),
        Some("openai/joined-model@1.0".to_string())
    );
    assert_eq!(
        service.override_catalog_id(Provider::Codex, "JOINED-MODEL"),
        Some("openai/joined-model@1.0".to_string())
    );

    // Miss: plain value, unknown id, bare-list shorthand, no override.
    assert_eq!(
        service.override_catalog_id(Provider::Codex, "plain-model"),
        None
    );
    assert_eq!(
        service.override_catalog_id(Provider::Codex, "not-configured"),
        None
    );
    assert_eq!(
        service.override_catalog_id(Provider::Gemini, "list-model"),
        None
    );
    assert_eq!(
        service.override_catalog_id(Provider::Claude, "anything"),
        None
    );
}

#[test]
fn service_first_valid_finds_match() {
    let service = ModelCatalogService::new();
    let candidates = vec!["not-real".into(), "gpt-5.5".into(), "gpt-5.4".into()];
    assert_eq!(
        service.first_valid(Provider::Codex, &candidates),
        Some("gpt-5.5".into())
    );
}

#[test]
fn service_first_valid_returns_none_when_no_match() {
    let service = ModelCatalogService::new();
    let candidates = vec!["not-real".into(), "also-fake".into()];
    assert_eq!(service.first_valid(Provider::Codex, &candidates), None);
}

#[test]
fn gemini_catalog_is_research_fed() {
    // Expected offerings are research-fed for all providers
    // (previously Gemini was empty/None-sourced).
    let service = ModelCatalogService::new();
    assert!(!service.catalog_for(Provider::Gemini).is_empty());
    assert!(service.is_valid(Provider::Gemini, "gemini-2.5-pro"));
}

#[test]
fn gemini_can_have_override() {
    let mut overrides = HashMap::new();
    overrides.insert(
        Provider::Gemini,
        ProviderModelOverride::AddList(vec!["gemini-2.5-pro".into()]),
    );
    let service = ModelCatalogService::with_overrides(overrides);
    assert!(service.is_valid(Provider::Gemini, "gemini-2.5-pro"));
}

#[test]
fn refresh_blocking_does_not_panic() {
    let service = ModelCatalogService::new();
    service.refresh_blocking(); // should not panic even if network is down
}

#[test]
fn refresh_provider_blocking_no_shell_source_no_subprocess() {
    // Providers without a ShellCommand source must never spawn a
    // subprocess.
    let tmp = tempfile::tempdir().unwrap();
    let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    service.refresh_provider_blocking(Provider::Claude);
    service.refresh_provider_blocking(Provider::Codex);
    service.refresh_provider_blocking(Provider::Gemini);
    service.refresh_provider_blocking(Provider::Goose);
    service.refresh_provider_blocking(Provider::KimiCode);
    service.refresh_provider_blocking(Provider::QwenCode);
    assert_eq!(service.shell_command_fetch_attempts(), 0);
}

/// A `None`-sourced refresh writes an empty listing to the
/// drift-channel cache — there is no dynamic listing to record, and
/// validation is baseline-fed regardless.
#[test]
fn refresh_provider_blocking_none_source_writes_empty_listing() {
    let tmp = tempfile::tempdir().unwrap();
    let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

    service.refresh_provider_blocking(Provider::QwenCode);
    assert_eq!(service.shell_command_fetch_attempts(), 0);

    let listing = service.cached_listing(Provider::QwenCode).unwrap();
    assert!(listing.is_empty(), "expected empty listing, got {listing:?}");
    assert!(service.is_valid(Provider::QwenCode, "qwen3-coder-plus"));
}

#[test]
fn refresh_provider_blocking_opencode_twice_dedupes() {
    // A repeat OpenCode refresh must not re-attempt the subprocess.
    // We seed the dedup cache up front to avoid relying on
    // `opencode` being on PATH.
    let tmp = tempfile::tempdir().unwrap();
    let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let (program, args) = opencode_command();
    service.prime_shell_command_dedup(
        program,
        args,
        Ok(vec!["qwen-coder".into(), "gpt-5.2".into()]),
    );
    service.refresh_provider_blocking(Provider::OpenCode);
    service.refresh_provider_blocking(Provider::OpenCode);
    assert_eq!(
        service.shell_command_fetch_attempts(),
        0,
        "primed dedup must short-circuit subprocess attempts"
    );
}

#[test]
fn refresh_provider_blocking_failure_does_not_affect_validation() {
    // Validation is baseline-fed, so a failed listing fetch must
    // leave it fully intact — for the failing provider itself and
    // for every other provider.
    let tmp = tempfile::tempdir().unwrap();
    let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let (program, args) = opencode_command();
    service.prime_shell_command_dedup(
        program,
        args,
        Err(CatalogFetchError::CliNotFound("opencode".into())),
    );
    service.refresh_provider_blocking(Provider::OpenCode);

    assert!(service.is_valid(Provider::OpenCode, "opencode/claude-opus-4-8"));
    assert!(service.is_valid(Provider::Claude, "claude-opus-4-8"));
    // The failed fetch wrote nothing to the listing cache.
    assert!(service.cached_listing(Provider::OpenCode).is_none());
}

#[test]
fn refresh_all_uses_primed_shell_result() {
    // refresh_all() must not spawn any subprocess when OpenCode's
    // command result is primed; no-source providers refresh for
    // free.
    let tmp = tempfile::tempdir().unwrap();
    let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    let (program, args) = opencode_command();
    service.prime_shell_command_dedup(
        program,
        args,
        Ok(vec![
            "qwen-2.5-coder".into(),
            "gpt-5".into(),
            "claude-sonnet-4".into(),
        ]),
    );

    service.refresh_blocking();

    assert_eq!(
        service.shell_command_fetch_attempts(),
        0,
        "primed dedup must short-circuit subprocess attempts in refresh_all"
    );

    // The primed shell result lands in the drift-channel listing
    // cache only; validation stays on the expected baseline, so the
    // listing-only ids are NOT valid.
    let listing = service.cached_listing(Provider::OpenCode).unwrap();
    assert!(listing.contains(&"qwen-2.5-coder".into()));
    assert!(listing.contains(&"gpt-5".into()));
    let opencode = service.catalog_for(Provider::OpenCode);
    assert!(!opencode.contains(&"qwen-2.5-coder".into()));
    assert!(opencode.contains(&"opencode/claude-opus-4-8".into()));

    // Qwen's baseline is unaffected by the OpenCode output.
    let qwen = service.catalog_for(Provider::QwenCode);
    assert!(qwen.contains(&"qwen3-coder-plus".into()));
    assert!(!qwen.contains(&"gpt-5".into()));
}

#[tokio::test]
async fn concurrent_opencode_refreshes_run_fetcher_once() {
    // Drive two OpenCode refreshes concurrently against an
    // injectable fake source that blocks until released. The dedup
    // contract requires the fetcher to run exactly once even when
    // both callers observe the command's OnceCell as uninitialized
    // at the start of their await.
    use std::time::Duration;
    use tokio::sync::Notify;

    let tmp = tempfile::tempdir().unwrap();
    let mut service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

    let fetch_count = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());

    let fetch_count_for_fetcher = fetch_count.clone();
    let started_for_fetcher = started.clone();
    let release_for_fetcher = release.clone();
    service.set_shell_command_fetcher(Arc::new(move |_program, _args| {
        let fetch_count = fetch_count_for_fetcher.clone();
        let started = started_for_fetcher.clone();
        let release = release_for_fetcher.clone();
        Box::pin(async move {
            fetch_count.fetch_add(1, Ordering::SeqCst);
            started.notify_waiters();
            release.notified().await;
            Ok(vec![
                "qwen-2.5-coder".to_string(),
                "gpt-5".to_string(),
                "claude-sonnet-4".to_string(),
            ])
        })
    }));

    let s1 = service.clone();
    let s2 = service.clone();
    let first_handle =
        tokio::spawn(async move { s1.refresh_provider(Provider::OpenCode).await });
    let second_handle =
        tokio::spawn(async move { s2.refresh_provider(Provider::OpenCode).await });

    // Wait until the first (and only) fetcher invocation has begun
    // and is parked on `release.notified()`. Then give the second
    // task room to schedule and observe the in-flight OnceCell.
    started.notified().await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Release the in-flight fetch so both callers can complete.
    release.notify_waiters();

    let first_result = first_handle.await.unwrap().unwrap();
    let second_result = second_handle.await.unwrap().unwrap();

    assert_eq!(
        fetch_count.load(Ordering::SeqCst),
        1,
        "shell-command fetcher must run exactly once across concurrent refreshes"
    );
    assert_eq!(
        service.shell_command_fetch_attempts(),
        1,
        "OnceCell init closure must run exactly once"
    );

    assert!(first_result.contains(&"gpt-5".to_string()));
    assert_eq!(first_result, second_result);
}

/// W3: when a cache file already exists for a dynamic-source
/// provider, `refresh_provider_async` must return promptly without
/// blocking on the fetcher closure. We prove this by installing a
/// fetcher that parks indefinitely; if the call were blocking, the
/// test would exceed its time budget.
///
/// `serial` because the env-var sibling test mutates
/// `CLAUDINE_BACKGROUND_REFRESH` for the whole process.
#[test]
#[serial_test::serial]
fn refresh_provider_async_returns_immediately_when_cache_exists() {
    use std::sync::mpsc;
    use std::time::Duration;
    use tokio::sync::Notify;

    let tmp = tempfile::tempdir().unwrap();
    let mut service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

    // Seed an on-disk cache so the provider has a non-empty entry.
    let entry = ModelCacheEntry {
        provider: Provider::OpenCode,
        models: vec!["seeded-model".into()],
        fetched_at: chrono::Utc::now(),
    };
    service.cache.write(&entry).unwrap();

    // Install a fetcher that would block forever if called. The async
    // refresh should never await on it from the caller's thread.
    let parked = Arc::new(Notify::new());
    let parked_for_fetcher = parked.clone();
    service.set_shell_command_fetcher(Arc::new(move |_program, _args| {
        let parked = parked_for_fetcher.clone();
        Box::pin(async move {
            parked.notified().await;
            Ok(Vec::new())
        })
    }));

    // Run the call on a worker thread and require it to return well
    // under the wall-clock budget the parked fetcher would impose.
    let svc = service.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        svc.refresh_provider_async(Provider::OpenCode);
        let _ = tx.send(());
    });
    rx.recv_timeout(Duration::from_secs(5))
        .expect("refresh_provider_async must not block when cache exists");

    // The seeded cache must still be readable immediately after the
    // call returns — the current invocation always sees the existing
    // entry, never the in-flight refresh.
    let read = service.cache.read(Provider::OpenCode).unwrap();
    assert_eq!(read.models, vec!["seeded-model"]);

    // Release the parked fetcher so the detached background thread
    // can wind down without leaking the runtime.
    parked.notify_waiters();
}

/// Cold start (no cache) no longer blocks: validation is
/// baseline-fed, so the subprocess result is never needed by the
/// current run and the refresh always detaches to the background.
/// Proven with a fetcher that parks indefinitely — a blocking
/// fallback would exceed the time budget.
#[test]
#[serial_test::serial]
fn refresh_provider_async_backgrounds_even_without_cache() {
    use std::sync::mpsc;
    use std::time::Duration;
    use tokio::sync::Notify;

    let tmp = tempfile::tempdir().unwrap();
    let mut service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

    // Pre-condition: no on-disk cache (the old contract blocked here).
    assert!(service.cache.read(Provider::OpenCode).is_none());

    let parked = Arc::new(Notify::new());
    let parked_for_fetcher = parked.clone();
    service.set_shell_command_fetcher(Arc::new(move |_program, _args| {
        let parked = parked_for_fetcher.clone();
        Box::pin(async move {
            parked.notified().await;
            Ok(Vec::new())
        })
    }));

    let svc = service.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        svc.refresh_provider_async(Provider::OpenCode);
        let _ = tx.send(());
    });
    rx.recv_timeout(Duration::from_secs(5))
        .expect("refresh_provider_async must not block on a cold cache");

    // Release the parked fetcher so the detached background thread
    // can wind down without leaking the runtime.
    parked.notify_waiters();
}

/// W3: no-source providers always run inline because writing the
/// empty listing is in-process and free.
#[test]
#[serial_test::serial]
fn refresh_provider_async_none_source_runs_inline() {
    let tmp = tempfile::tempdir().unwrap();
    let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

    service.refresh_provider_async(Provider::Claude);
    // The inline refresh wrote a (necessarily empty) listing cache
    // entry before returning.
    let read = service.cache.read(Provider::Claude).unwrap();
    assert!(read.models.is_empty());
    assert_eq!(service.shell_command_fetch_attempts(), 0);
}

/// W3 escape hatch: `CLAUDINE_BACKGROUND_REFRESH=0` forces the
/// caller-blocking path even when a cache exists.
#[test]
#[serial_test::serial]
fn refresh_provider_async_env_var_disables_background() {
    use crate::model_catalog::provider_sources::CatalogFetchError;

    let tmp = tempfile::tempdir().unwrap();
    let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());

    let entry = ModelCacheEntry {
        provider: Provider::OpenCode,
        models: vec!["seeded".into()],
        fetched_at: chrono::Utc::now(),
    };
    service.cache.write(&entry).unwrap();
    let (program, args) = opencode_command();
    service.prime_shell_command_dedup(
        program,
        args,
        Err(CatalogFetchError::CliNotFound("opencode".into())),
    );

    let prior = std::env::var("CLAUDINE_BACKGROUND_REFRESH").ok();
    unsafe {
        std::env::set_var("CLAUDINE_BACKGROUND_REFRESH", "0");
    }
    // With the env var set we go through `refresh_provider_blocking`,
    // which uses the primed dedup cell and returns synchronously.
    service.refresh_provider_async(Provider::OpenCode);
    unsafe {
        match prior {
            Some(v) => std::env::set_var("CLAUDINE_BACKGROUND_REFRESH", v),
            None => std::env::remove_var("CLAUDINE_BACKGROUND_REFRESH"),
        }
    }

    // The original cache survives because the primed fetch returned an error.
    let read = service.cache.read(Provider::OpenCode).unwrap();
    assert_eq!(read.models, vec!["seeded"]);
}

#[test]
fn refresh_all_none_source_providers_no_subprocess() {
    // refresh_all() must not spawn any subprocess for no-source
    // providers (Claude, Codex) and should still write their (empty)
    // listing cache entries.
    let tmp = tempfile::tempdir().unwrap();
    let service = ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
    // Prime the dedup cache so the OpenCode refresh does not reach
    // the subprocess.
    let (program, args) = opencode_command();
    service.prime_shell_command_dedup(
        program,
        args,
        Ok(vec!["qwen-2.5-coder".into(), "gpt-5".into()]),
    );
    service.refresh_blocking();

    assert_eq!(
        service.shell_command_fetch_attempts(),
        0,
        "no-source providers must not trigger a shell-command subprocess"
    );

    // Listing cache entries were written, and baseline validation is
    // unaffected either way.
    assert!(service.cached_listing(Provider::Claude).is_some());
    assert!(service.cached_listing(Provider::Codex).is_some());
    assert!(service.is_valid(Provider::Claude, "claude-opus-4-8"));
    assert!(service.is_valid(Provider::Codex, "gpt-5.5"));
}
