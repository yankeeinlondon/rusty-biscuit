---
ready: false
phases: 4
start_phase: 4
agent: ""
model: ""
review: review-5.md
source_files_during_phase_3:
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/profile.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - claudine-cli
---

# Centralized Providers — Review 5 Closure Plan

This plan addresses every finding and test recommendation in
[`review-5.md`](./review-5.md). The work is split into **four serial
implementation phases**:

| Phase | Closes | Description |
|---|---|---|
| 1 | Finding 1 | Make `ProviderBehavior::detect_from_payload` truthful, route public detection through it, and strengthen representative-payload tests. |
| 2 | Finding 2 | Remove legacy `AgentCapabilities` from the structured provider describe surface and add parity guards while the compatibility facade remains. |
| 3 | Finding 3 | Finish the CLI wrapper catalog migration for output detection, entrypoints, output formats, and YOLO parity. |
| 4 | All | Run the targeted and full verification gates for `claudine` and `claudine-cli`, including lint with warnings denied. |

## Assumptions

- The claudine package area consists of workspace packages `claudine` and
  `claudine-cli`; confirm with `cargo metadata --no-deps --format-version 1`
  before final verification.
- `AgentCapabilities` remains as a compatibility facade for
  `claudine::agents::Agent` and existing table-style provider output, but it
  must no longer be serialized as central catalog data under
  `claudine providers --describe --format json`.
- `resource_support` is not part of Review 5 Finding 2. It is already typed
  enough for the linking facade and can stay in the structured describe
  surface unless implementation uncovers the same duplicated string problem.
- Provider wrappers remain CLI-only. The fix must not introduce a lib ->
  CLI dependency.
- Provider quirks are allowed to stay as wrapper overrides only when the
  quirk cannot be represented in the typed catalog without making the catalog
  misleading.

## Phase 1 — Make `ProviderBehavior::detect_from_payload` authoritative

### Scope

Files:

- `claudine/lib/src/provider/behavior.rs`
- `claudine/lib/src/provider/methods.rs`
- `claudine/lib/src/provider/{claude,codex,gemini,kimi,opencode,goose,qwen,roo}.rs`
- `claudine/lib/src/provider/tests.rs`

### Tasks

1. Change `ProviderBehavior::detect_from_payload` so it is no longer a silent
   public no-op. Use one of these two implementation shapes, in this order of
   preference:
   - make `detect_from_payload` a required trait method and implement it for
     every provider behavior;
   - or keep a default only if the trait gains a provider-neutral way to
     delegate to the same provider's `AdapterBehavior::detect`.
2. For providers that already implement `AdapterBehavior::detect`, implement
   `ProviderBehavior::detect_from_payload` by delegating to the same detection
   rule so there is one payload-shape rule per provider.
3. For providers that intentionally have no raw-payload detection today
   (currently Goose, Qwen, and Roo per the existing representative-payload
   helper), implement an explicit `false` return with a short comment that no
   representative raw hook payload exists yet.
4. Change `Provider::detect_from_payload` in
   `claudine/lib/src/provider/methods.rs` to walk
   `PROVIDERS_DISPLAY_ORDER` and call
   `provider_info(provider).behavior.detect_from_payload(raw)`, not
   `adapter.detect(raw)`.
5. Update or add a source guard so `Provider::detect_from_payload` remains a
   pure registry walk and does not regain provider-specific shape checks.
6. Strengthen `detect_from_payload_exercise_all_providers` in
   `provider/tests.rs`:
   - keep the empty-payload no-panic loop;
   - for every `representative_payload_for(provider)`, assert
     `provider_info(provider).behavior.detect_from_payload(&payload)` is
     `true`;
   - assert `Provider::detect_from_payload(&payload) == Some(provider)` for
     representative payloads.
7. Keep `adapter_detect_exercise_all_providers` as a parity guard while the
   adapter trait remains the parsing surface.

### Tests

Run after the phase:

```bash
cargo test -p claudine provider::tests::detect_from_payload_exercise_all_providers
cargo test -p claudine provider::tests::adapter_detect_exercise_all_providers
cargo test -p claudine provider::tests
```

### Definition of Done

- No provider with a representative payload returns `false` from
  `ProviderBehavior::detect_from_payload`.
- Public provider detection routes through `ProviderBehavior`, matching the
  spec/design surface.
- Existing adapter detection remains covered, but it is no longer the only
  working detection path.

## Phase 2 — Remove legacy capabilities from structured describe and guard facade parity

### Scope

Files:

- `claudine/lib/src/provider/mod.rs`
- `claudine/lib/src/provider/tests.rs`
- `claudine/cli/src/commands/providers.rs`
- Provider catalog files under `claudine/lib/src/provider/*.rs` if parity
  tests expose drift.
- `claudine/lib/src/agents/model.rs` only if helper accessors are needed for
  clearer tests.

### Tasks

1. Stop serializing the legacy `AgentCapabilities` tree as a
   `ProviderInfo` field:
   - remove the `serialize_agent_capabilities` helper;
   - change `agent_capabilities_fn` to `#[serde(skip)]`;
   - keep `ProviderInfo::agent_capabilities()` and the `Agent` impl so
     `agents::registry::agent_for(provider)` remains compatible.
2. Update the docs/comments on `ProviderInfo` so the structured JSON surface
   is described as typed top-level catalog fields plus `resource_support`, not
   the legacy `AgentCapabilities` tree.
3. Update JSON describe tests:
   - in `provider_info_serializes_round_trip`, assert behavior fields and
     `agent_capabilities_fn` are absent;
   - in `provider_info_json_includes_capabilities_and_resource_support`,
     rename or replace the test so it asserts `resource_support` is present
     and `capabilities` is absent;
   - remove
     `provider_info_json_capabilities_includes_nested_runtime_data`;
   - in `cli/src/commands/providers.rs::tests::describe_json_serializes_all_providers`,
     remove `"capabilities"` from the required-key list and keep typed keys
     such as `output_formats`, `entrypoints`, `system_prompt`, `yolo`,
     `reasoning`, `known_gaps`, `prompt_arg_conventions`, and
     `resource_support`.
4. Add provider parity tests for duplicated legacy facade fields while
   `AgentCapabilities` still exists. These tests should compare
   `provider_info(provider)` to `provider_info(provider).agent_capabilities()`
   across every provider for:
   - identity: provider id, display name policy, binary, docs URLs;
   - config paths: at least primary user/project/local path coverage where
     both models expose the same category;
   - runtime non-interactive: supported flag, structured output flag,
     resume support if present, entrypoint strings, and output format names;
   - system prompt: memory files and replacement support;
   - permissions: YOLO flag/env surface and unsupported state;
   - reasoning: supported/unsupported state and control names;
   - logging: session log paths and session locations;
   - known gaps/limitations: legacy limitation strings must correspond to
     typed `KnownGap` entries where both are populated.
5. If a parity test fails because the typed catalog is missing a fact that is
   still true in the legacy facade, update the typed provider catalog first.
   If it fails because the legacy facade is stale, update the legacy builder
   to match the typed catalog.
6. Add a short comment above every per-provider `build_*_agent_capabilities`
   function saying it is a compatibility facade and that typed provider fields
   are authoritative.

### Tests

Run after the phase:

```bash
cargo test -p claudine provider::tests::provider_info_serializes_round_trip
cargo test -p claudine provider::tests::agent_capabilities
cargo test -p claudine provider::tests
cargo test -p claudine-cli providers::tests::describe_json_serializes_all_providers
```

Use the actual test filters added in this phase if their names differ; the
intent is to run the new parity tests plus the full provider test module.

### Definition of Done

- `claudine providers --describe --format json` no longer exposes the
  legacy string-heavy `capabilities` object.
- The legacy `AgentCapabilities` facade still works for compatibility.
- Every duplicated legacy field covered by the review has an explicit parity
  test against typed `ProviderInfo` data.

## Phase 3 — Finish CLI wrapper catalog migration and drift guards

### Scope

Files:

- `claudine/lib/src/provider/output_format.rs`
- `claudine/lib/src/provider/{claude,codex,gemini,kimi,opencode,qwen,goose}.rs`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/profile.rs`
- `claudine/cli/src/commands/wrap/catalog_helpers.rs`

### Tasks

1. Extend the typed output-format catalog enough to derive native output
   detection without hard-coded provider branches. The catalog must represent
   at least:
   - a flag-only selector such as Codex `--json`;
   - a flag-with-value selector such as `--output-format json`,
     `--output-format=stream-json`, and OpenCode `--format json`;
   - a positional/native token selector only if OpenCode still genuinely
     accepts bare `json` as an output request;
   - a way to mark flags like Kimi `--wire` or `--print` as structured-stream
     transport, not user-requested native output, so structured parsing is not
     disabled accidentally.
2. Populate missing or stale typed catalog rows before removing wrapper
   overrides:
   - add OpenCode JSON output support to `OPENCODE_OUTPUT_FORMATS` if
     `--format json` is still supported;
   - change Kimi entrypoint/structured-stream catalog data from legacy
     `--print` to the current `--wire` JSON-RPC path;
   - update Gemini text output metadata if the intended wrapper behavior is
     still to inject `--output-format text`.
3. Replace `has_explicit_native_output_request(provider, args)` in
   `wrap/mod.rs` with a provider-neutral helper that reads
   `provider_info(provider).output_formats` and the selector metadata added
   above.
4. Add unit tests for the helper:
   - Codex `--json` disables internal structured parsing;
   - Claude/Gemini/Qwen `--output-format json` and
     `--output-format=stream-json` disable internal structured parsing;
   - OpenCode `--format json` disables internal structured parsing;
   - Kimi `--wire` by itself does not count as a user native-output request;
   - unknown provider flags do not match.
5. Delete or justify remaining `WrapperProfile::apply_entrypoint` and
   `WrapperProfile::apply_output_format` overrides:
   - remove overrides that are now identical to the catalog-backed default;
   - keep overrides only for irreducible quirks such as prompt delivery,
     system prompt temp-file behavior, resume argv, noise filtering, model
     side effects, and structured-stream setup when it is not expressible as
     output format metadata;
   - for kept overrides, add one comment explaining why the typed catalog is
     insufficient.
6. Remove `catalog_helpers.rs` if all helpers are unused and redundant with
   `WrapperProfile` defaults. If any helper remains, delete the "Phase 6 will
   swap callers over" wording and add tests that exercise the helper.
7. Add CLI wrapper parity tests:
   - `apply_output_format_matches_provider_catalog_for_every_provider`;
   - `apply_entrypoint_matches_provider_catalog_for_every_provider`;
   - `explicit_native_output_detection_matches_provider_catalog`;
   - `apply_yolo_matches_provider_catalog_for_every_provider`;
   - a source guard equivalent to the lib guard that fails if
     `wrap/mod.rs` or `profile.rs` regains a raw `match provider` for output
     detection where catalog data should be used.
8. Keep existing behavior-specific tests such as
   `kimi_non_interactive_uses_wire_protocol_and_wire_rpc_delivery`,
   `gemini_output_format_uses_output_format_flag_and_supports_stream_json`,
   and OpenCode model/Yolo tests. Update expected argv only when the typed
   catalog fix intentionally changes stale metadata to match current runtime
   behavior.

### Tests

Run after the phase:

```bash
cargo test -p claudine-cli wrapper_registry_covers_every_provider_and_documents_exceptions
cargo test -p claudine-cli prompt_arg_conventions
cargo test -p claudine-cli output_format
cargo test -p claudine-cli native_output
cargo test -p claudine-cli yolo
cargo test -p claudine-cli wrap::profile::tests
```

Use exact test filters for the new helper/parity tests once named.

### Definition of Done

- Native output detection is derived from typed provider catalog data.
- Redundant entrypoint/output-format wrapper overrides are gone.
- Any remaining wrapper override is documented as a real runtime quirk.
- CLI parity tests fail if catalog data and wrapper behavior drift again.

## Phase 4 — Final verification gate

### Scope

Packages:

- `claudine`
- `claudine-cli`

### Tasks

1. Confirm the package list and toolchain:

```bash
cargo metadata --no-deps --format-version 1
cargo --version
rustc --version
```

2. Run targeted tests that correspond directly to Review 5:

```bash
cargo test -p claudine provider::tests
cargo test -p claudine-cli providers::tests::describe_json_serializes_all_providers
cargo test -p claudine-cli wrapper_registry_covers_every_provider_and_documents_exceptions
cargo test -p claudine-cli prompt_arg_conventions
cargo test -p claudine-cli output_format
cargo test -p claudine-cli native_output
cargo test -p claudine-cli yolo
```

3. Run full claudine package tests:

```bash
just -f claudine/justfile test
```

Equivalent direct command if `just` is unavailable:

```bash
cargo test -p claudine -p claudine-cli
```

4. Run lint with warnings denied:

```bash
just -f claudine/justfile lint
```

Equivalent direct command if `just` is unavailable:

```bash
cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings
```

5. Run a final source drift check:

```bash
rg "Phase 6 will swap|intentionally unused|has_explicit_native_output_request|match provider" \
  claudine/cli/src/commands/wrap claudine/lib/src/provider
```

Any hit must be either removed or explicitly justified in nearby comments and
tests.

### Definition of Done

- All targeted Review 5 tests pass.
- `just -f claudine/justfile test` passes, or the direct cargo equivalent
  passes for `claudine` and `claudine-cli`.
- `just -f claudine/justfile lint` passes with no warnings/errors, or the
  direct clippy command passes with `-D warnings`.
- No stale migration comments or unauthorized provider dispatch points remain.

## Production Readiness Criteria

The implementation is ready for another review when all four phases are done
and the following are true:

- `ProviderBehavior::detect_from_payload` returns correct results for every
  representative provider payload and is the public detection dispatch path.
- The structured provider describe JSON has one authoritative typed catalog
  surface and does not serialize legacy `AgentCapabilities`.
- Legacy `AgentCapabilities` remains only as a compatibility facade and is
  covered by parity tests for every duplicated field still present.
- Wrapper output detection, entrypoint injection, output-format application,
  and YOLO behavior are either catalog-backed or explicitly documented as
  runtime quirks with tests.
- Tests and lint pass for both claudine package-area crates.
