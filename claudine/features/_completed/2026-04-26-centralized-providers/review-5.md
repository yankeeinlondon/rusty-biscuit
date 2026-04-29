---
ready: false
agent: ""
model: ""
---

# Review 5

The implementation has made substantial progress: the canonical `provider` module exists, the lib-side registry and provider facades are mostly catalog-backed, and the targeted provider/CLI tests I ran pass. I do not think this is ready for production yet because several parts of the designed centralized contract are still either no-op, duplicated, or only partially tested.

## Findings

### 1. `ProviderBehavior::detect_from_payload` is part of the designed dynamic surface but is still a no-op

- Severity: High
- References:
    - `claudine/lib/src/provider/behavior.rs:42`
    - `claudine/lib/src/provider/behavior.rs:45`
    - `claudine/lib/src/provider/methods.rs:145`
    - `claudine/lib/src/provider/tests.rs:734`

The spec puts payload detection on `ProviderBehavior`, reached from `provider_info(p).behavior`, but the implemented `ProviderBehavior::detect_from_payload` default always returns `false` and no provider overrides it. Runtime provider detection works only because `Provider::detect_from_payload` bypasses that trait and walks `provider_info(*provider).adapter.detect(raw)` instead.

That leaves a public, central-provider behavior method that advertises the designed operation but gives the wrong answer for every recognizable provider payload. The test named `detect_from_payload_exercise_all_providers` only asserts the method does not panic; it does not assert the representative payloads are recognized, so this gap is currently invisible to CI.

Suggested fix: either move detection fully onto `ProviderBehavior::detect_from_payload` and have it delegate to the adapter where appropriate, or remove that method from `ProviderBehavior` and update the spec/design-facing comments accordingly. Add tests that assert `provider_info(Provider::Claude).behavior.detect_from_payload({"hook_event_name":"Stop"})` and the other representative provider payloads return `true`.

### 2. The string-heavy legacy `AgentCapabilities` model is still serialized as provider catalog data

- Severity: Medium
- References:
    - `claudine/lib/src/agents/model.rs:40`
    - `claudine/lib/src/agents/model.rs:77`
    - `claudine/lib/src/agents/model.rs:88`
    - `claudine/lib/src/agents/model.rs:95`
    - `claudine/lib/src/agents/model.rs:104`
    - `claudine/lib/src/agents/model.rs:120`
    - `claudine/lib/src/provider/claude.rs:130`
    - `claudine/lib/src/provider/claude.rs:481`

The spec calls for replacing structured facts like paths, entrypoints, output formats, system prompt delivery, YOLO, reasoning controls, and known gaps with typed catalog data. The implementation adds typed `ProviderInfo` fields, but `ProviderInfo` also serializes the legacy `AgentCapabilities` under `capabilities`; that legacy model still contains `Vec<&'static str>`, `Vec<PathBuf>`, `yolo_equivalent: Option<&'static str>`, prose output formats, prose entrypoints, and prose gaps. Provider modules still build those values through `LazyLock<AgentCapabilities>`.

This means `claudine providers --describe --format json` exposes two overlapping capability representations: typed top-level catalog fields and the old string/prose capability tree. Those can drift independently, and the current tests mostly prove that both exist, not that the old string fields are derived from the typed source or no longer authoritative.

Suggested fix: either stop including legacy `AgentCapabilities` in the structured provider describe surface, or make the legacy facade derive from the typed catalog so there is only one data source. Add parity tests for every duplicated field until the legacy model is removed.

### 3. The CLI wrapper still carries provider-specific dispatch that the design expected to be catalog-backed

- Severity: Medium
- References:
    - `claudine/cli/src/commands/wrap/mod.rs:610`
    - `claudine/cli/src/commands/wrap/profile.rs:844`
    - `claudine/cli/src/commands/wrap/profile.rs:1061`
    - `claudine/cli/src/commands/wrap/profile.rs:1445`
    - `claudine/cli/src/commands/wrap/catalog_helpers.rs:10`

The design says wrapper defaults should consume typed catalog data for output formats, entrypoints, YOLO, prompt conventions, stream protocol, and resume support, with overrides only for irreducible quirks. The trait now has some catalog-backed defaults, but the wrapper path still has hard-coded provider branches for native output detection and per-provider overrides for entrypoints/output formats that duplicate catalog data. `catalog_helpers.rs` still says those helpers are intentionally unused and that "Phase 6 will swap callers over," which reads as unfinished migration state.

This is not just cleanup. `has_explicit_native_output_request` is outside the wrapper registry invariant and can drift from `ProviderInfo::output_formats`; a new provider or changed output flag can compile without updating that function. The CLI-side tests cover the wrapper registry and selected prompt conventions, but there is no source guard or parity test equivalent to the lib-side `no_unauthorized_match_provider_in_lib`.

Suggested fix: derive explicit native output detection from `provider_info(provider).output_formats`, delete or justify the remaining `apply_entrypoint` / `apply_output_format` overrides, and add a CLI-side drift guard or parity tests for wrapper methods that duplicate catalog fields.

## Test Coverage Notes

The following checks passed during this review:

- `cargo test -p claudine provider::tests`
- `cargo test -p claudine-cli wrapper_registry_covers_every_provider_and_documents_exceptions`
- `cargo test -p claudine-cli prompt_arg_conventions`

Additional tests I would add before production:

- Assert `ProviderBehavior::detect_from_payload` recognizes each provider's representative payload or remove the method.
- Assert legacy `AgentCapabilities` duplicate fields match their typed `ProviderInfo` equivalents while both surfaces exist.
- Add CLI wrapper parity tests for output format flags, entrypoint injection, explicit-native-output detection, and YOLO flag/env behavior against the typed catalog.

## Production Readiness

Not ready. The runtime paths I inspected are mostly functional and the targeted tests pass, but the implementation still exposes a no-op designed behavior method and retains multiple sources of provider truth in the describe and wrapper surfaces.
