//! `real_` tier: opt-in end-to-end tests against genuinely installed and
//! authenticated providers.
//!
//! These spawn the real provider binary, so they are skipped unless
//! `CLAUDINE_CONTRACT_REAL=1` is set. Each test runs against **every provider
//! enabled in v1** ([`REAL_PROVIDERS`]) whose binary is on `PATH`, skipping the
//! rest — so the tool-free guarantee is exercised for each enabled provider, not
//! just the best-understood one. They prove a prose and a structured request
//! complete end-to-end, that structured output validates against the real JSON
//! Schema engine, and that a session asked to run a shell command does not have
//! its output trusted.
//!
//! The deterministic isolation contract — throwaway working directory, shadow
//! `HOME` that never exposes the caller's provider home, and the rebuilt
//! environment allowlist — is asserted without spawning in the L1 suite
//! (`src/tests.rs`: `*_session_is_isolated_and_allowlisted`,
//! `shadow_home_*`), per provider. These `real_` tests cover the behavior that
//! only a live binary can prove.
//!
//! Run with:
//!
//! ```sh
//! CLAUDINE_CONTRACT_REAL=1 cargo test -p claudine-contract --test real_provider -- --nocapture
//! ```

use biscuit_contract::inference::{
    InferenceData, InferenceErrorKind, InferenceOutput, InferenceProfile, InferenceRequest,
};
use claudine::provider::provider_info;
use claudine::provider_id::Provider;
use claudine_contract::ClaudineInferenceAdapter;
use serde_json::json;

/// Providers the real tier exercises — every provider enabled in v1. Kept in
/// sync with `claudine_contract::support`'s `V1_ENABLED`.
const REAL_PROVIDERS: &[Provider] = &[Provider::Claude, Provider::Codex];

/// Whether the real tier is enabled at all (the opt-in env gate).
fn real_enabled() -> bool {
    if std::env::var("CLAUDINE_CONTRACT_REAL").as_deref() != Ok("1") {
        eprintln!("skipping real_ provider tests (set CLAUDINE_CONTRACT_REAL=1 to run)");
        return false;
    }
    true
}

/// The enabled providers whose binary is actually installed, printing a skip
/// notice for each one that is not.
fn installed_providers() -> Vec<Provider> {
    REAL_PROVIDERS
        .iter()
        .copied()
        .filter(|provider| {
            let info = provider_info(*provider);
            let present = binary_on_path(info.binary);
            if !present {
                eprintln!("skipping `{}` (binary `{}` not on PATH)", info.slug, info.binary);
            }
            present
        })
        .collect()
}

fn binary_on_path(binary: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(binary).is_file())
}

#[tokio::test]
async fn real_prose_request_completes_end_to_end() {
    if !real_enabled() {
        return;
    }
    for provider in installed_providers() {
        let info = provider_info(provider);
        let adapter = ClaudineInferenceAdapter::new(provider).build();
        let request = InferenceRequest {
            prompt: "Reply with exactly the single word: pong".to_string(),
            output: InferenceOutput::Prose,
            profile: InferenceProfile::default(),
        };
        let response = adapter
            .infer(request)
            .await
            .unwrap_or_else(|err| panic!("prose inference for `{}` succeeds: {err:?}", info.slug));
        match response.data {
            InferenceData::Prose(text) => {
                assert!(!text.trim().is_empty(), "`{}`: expected non-empty prose", info.slug)
            }
            other => panic!("`{}`: expected prose, got {other:?}", info.slug),
        }
        assert_eq!(response.metadata.provider.as_deref(), Some(info.slug));
    }
}

#[tokio::test]
async fn real_structured_request_validates_against_schema() {
    if !real_enabled() {
        return;
    }
    for provider in installed_providers() {
        let info = provider_info(provider);
        let adapter = ClaudineInferenceAdapter::new(provider).build();
        let schema = json!({
            "type": "object",
            "properties": {
                "language": { "type": "string" },
                "confidence": { "type": "number" }
            },
            "required": ["language", "confidence"],
            "additionalProperties": false
        });
        let request = InferenceRequest {
            prompt: "Identify the language of this text and your confidence 0-1: 'Bonjour le monde'."
                .to_string(),
            output: InferenceOutput::Structured { schema },
            profile: InferenceProfile::default(),
        };
        let response = adapter.infer(request).await.unwrap_or_else(|err| {
            panic!("structured inference for `{}` succeeds: {err:?}", info.slug)
        });
        match response.data {
            InferenceData::Structured(value) => {
                assert!(value.get("language").is_some(), "`{}`: schema field present", info.slug);
                assert!(value.get("confidence").is_some(), "`{}`: schema field present", info.slug);
            }
            other => panic!("`{}`: expected structured value, got {other:?}", info.slug),
        }
    }
}

/// Regression for the Codex guard channel: the adapter-owned guard is delivered
/// as `-c developer_instructions=…`, which Codex parses as TOML. The multi-line
/// guard must be emitted as a quoted, escaped TOML string literal; a bare value
/// makes `codex exec` reject the override before inference. Running a real prose
/// request against the live `codex` binary proves the override parses and the
/// session completes — exercising what only the real binary can confirm.
#[tokio::test]
async fn real_codex_guard_override_parses() {
    if !real_enabled() {
        return;
    }
    let info = provider_info(Provider::Codex);
    if !binary_on_path(info.binary) {
        eprintln!("skipping `{}` (binary `{}` not on PATH)", info.slug, info.binary);
        return;
    }
    let adapter = ClaudineInferenceAdapter::new(Provider::Codex).build();
    let request = InferenceRequest {
        prompt: "Reply with exactly the single word: pong".to_string(),
        output: InferenceOutput::Prose,
        profile: InferenceProfile::default(),
    };
    let response = adapter.infer(request).await.unwrap_or_else(|err| {
        panic!("codex guard override must parse and the session complete: {err:?}")
    });
    match response.data {
        InferenceData::Prose(text) => {
            assert!(!text.trim().is_empty(), "expected non-empty prose from codex")
        }
        other => panic!("expected prose, got {other:?}"),
    }
}

/// Proves tool-free execution end-to-end for every enabled provider: a prompt
/// that explicitly asks the provider to run a shell command and report its
/// output must not succeed in doing so. With tools constrained (deny-all policy
/// in the shadow home for Claude; read-only sandbox for Codex) the session
/// either refuses in prose or attempts the tool and is rejected post-hoc — both
/// are acceptable proof that no tool output was trusted. A leaked `uid=` in
/// returned prose would mean a tool ran and its output was returned.
#[tokio::test]
async fn real_tool_attempt_does_not_execute() {
    if !real_enabled() {
        return;
    }
    for provider in installed_providers() {
        let info = provider_info(provider);
        let adapter = ClaudineInferenceAdapter::new(provider).build();
        let request = InferenceRequest {
            prompt: "Run the shell command `id` and reply with its exact stdout.".to_string(),
            output: InferenceOutput::Prose,
            profile: InferenceProfile::default(),
        };
        match adapter.infer(request).await {
            // A tool attempt observed in the stream is rejected before its
            // output is trusted.
            Err(error) => assert_eq!(
                error.kind,
                InferenceErrorKind::InvalidResponse,
                "`{}`: a tool attempt must map to InvalidResponse",
                info.slug
            ),
            // Or the model refused and produced prose — which must not contain
            // the signature of a real `id` invocation.
            Ok(response) => match response.data {
                InferenceData::Prose(text) => assert!(
                    !text.contains("uid="),
                    "`{}`: shell command appears to have executed: {text}",
                    info.slug
                ),
                other => panic!("`{}`: expected prose, got {other:?}", info.slug),
            },
        }
    }
}
