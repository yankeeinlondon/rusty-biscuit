---
agent: open_code/zai-coding-plan/glm-5.2
phases: 5
created: 2026-06-14
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - unchained-ai/contract/Cargo.toml
  - unchained-ai/contract/src/lib.rs
  - unchained-ai/contract/justfile
  - unchained-ai/Cargo.toml
  - unchained-ai/justfile
  - Cargo.toml
docs_updated_during_phase_1:
  - reaper/features/2026-06-11-unchained-adapter/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - unchained-ai/lib/Cargo.toml
  - unchained-ai/lib/src/lib.rs
  - unchained-ai/lib/src/execution/mod.rs
  - unchained-ai/lib/src/models/mod.rs
  - unchained-ai/lib/src/models/selection.rs
  - unchained-ai/lib/src/rigging/providers/models/mod.rs
  - unchained-ai/lib/src/rigging/providers/models/ollama.rs
  - unchained-ai/lib/src/rigging/providers/provider_errors.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - unchained-ai/lib/src/execution/mod.rs
  - unchained-ai/lib/src/primitives/atomic/prompt.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - unchained-ai/contract/Cargo.toml
  - unchained-ai/contract/src/lib.rs
  - unchained-ai/contract/src/error.rs
  - unchained-ai/contract/src/profile.rs
  - unchained-ai/contract/src/structured.rs
  - unchained-ai/contract/src/adapter.rs
  - unchained-ai/lib/src/execution/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .opencode/skill/unchained-ai/SKILL.md
source_files_during_phase_5:
  - unchained-ai/contract/tests/real_provider.rs
docs_updated_during_phase_5:
  - reaper/features/2026-06-11-unchained-adapter/plan.md
  - docs/dependencies.md
  - unchained-ai/README.md
  - unchained-ai/lib/README.md
  - .opencode/skill/unchained-ai/SKILL.md
  - .opencode/skill/unchained-ai/pipeline-primitives.md
  - .opencode/skill/biscuit-contract/SKILL.md
docs_created_during_phase_5:
  - unchained-ai/contract/docs/dependencies.md
skills_files_updated_during_phase_5:
  - .opencode/skill/unchained-ai/SKILL.md
  - .opencode/skill/unchained-ai/pipeline-primitives.md
  - .opencode/skill/biscuit-contract/SKILL.md
packages:
  - unchained-ai
  - unchained-ai-contract
source_code:
  - unchained-ai/contract/Cargo.toml
  - unchained-ai/contract/src/lib.rs
  - unchained-ai/contract/justfile
  - unchained-ai/Cargo.toml
  - unchained-ai/justfile
  - Cargo.toml
  - unchained-ai/lib/Cargo.toml
  - unchained-ai/lib/src/lib.rs
  - unchained-ai/lib/src/execution/mod.rs
  - unchained-ai/lib/src/models/mod.rs
  - unchained-ai/lib/src/models/selection.rs
  - unchained-ai/lib/src/rigging/providers/models/mod.rs
  - unchained-ai/lib/src/rigging/providers/models/ollama.rs
  - unchained-ai/lib/src/rigging/providers/provider_errors.rs
  - unchained-ai/lib/src/primitives/atomic/prompt.rs
  - unchained-ai/contract/src/error.rs
  - unchained-ai/contract/src/profile.rs
  - unchained-ai/contract/src/structured.rs
  - unchained-ai/contract/src/adapter.rs
  - unchained-ai/contract/tests/real_provider.rs
documentation:
  - reaper/features/2026-06-11-unchained-adapter/plan.md
  - docs/dependencies.md
  - unchained-ai/README.md
  - unchained-ai/lib/README.md
  - unchained-ai/contract/docs/dependencies.md
  - .opencode/skill/unchained-ai/SKILL.md
  - .opencode/skill/unchained-ai/pipeline-primitives.md
  - .opencode/skill/biscuit-contract/SKILL.md
---

# Execution Plan — Unchained AI Inference Adapter

Source spec: [`spec.md`](./spec.md). Depends on the already-shipped
`biscuit-contract` crate ([`reaper/features/2026-06-03-inference-trait/spec.md`](../2026-06-03-inference-trait/spec.md)),
whose `InferenceAdapter` trait and data types are implemented at
`biscuit-contract/lib/src/inference.rs`.

## Context for Implementers

This plan delivers two coupled work items in two crates, plus workspace/docs
wiring:

1. A **reusable single-turn execution surface + model resolver** in
   `unchained-ai/lib` that replaces the `Prompt::execute()` "not yet
   implemented" stub.
2. A thin **`unchained-ai-contract` adapter crate** that maps
   `biscuit_contract::inference::InferenceAdapter` onto that surface.

`claudine-contract` is the precedent for the adapter-crate shape (layout,
Cargo deps, justfile, `docs/dependencies.md`, `tests/real_provider.rs`,
structured-output validation via `jsonschema` Draft 2020-12). Mirror it where
the spec does not dictate otherwise.

### Confirmed codebase facts (do not re-derive)

- `biscuit-contract/lib` is implemented and tested; do **not** change its
  public API.
- `unchained-ai/lib` already provides: `ModelCapability` (full tier vocabulary
  incl. `*Thinking`/`*Ultrathink`/`*Cheap` variants), `ProviderModel`
  (`provider()`, `model_id()`, `wire_id()`, `metadata()`, `parse_wire_id()`),
  `Provider` (`config()` → `env_vars`, `auth_method`, `base_url`, `is_local`;
  `is_local()`, `display_name()`), `ProviderModelMetadata`
  (`supported_parameters: Option<Vec<String>>`,
  `default_parameters: Option<ModelDefaultParameters>`),
  `ProviderError` (`MissingApiKey`, `RateLimitExceeded`,
  `AuthenticationFailed`, `Timeout`, `HttpError`, …), OpenAI-compatible client
  adaptors for Z.ai / ZenMux, and `rig-core` v0.31.
- `Prompt::execute()` / `execute_readonly()` at
  `unchained-ai/lib/src/primitives/atomic/prompt.rs:365` return
  `"LLM execution not yet implemented"` — the stub this work replaces.
- Root `justfile` `areas` does **not** include `claudine-contract` or any
  contract sub-crate; `claudine-contract` is wired into `claudine/justfile`
  only. Follow the same convention: wire `unchained-ai-contract` into
  `unchained-ai/justfile`, not into root `areas`.
- `just/lifecycle.just` + `just/util.just` provide the shared `_build`,
  `_sanity`, `_test`, `_lint`, `_coverage`, `_doctest`, `_test_real` helpers.

### Decisions resolving spec ambiguity

- **Dependency docs path.** The spec says "Update
  `unchained-ai/docs/dependencies.md`", but that file does not exist and the
  `claudine-contract` precedent is a per-crate `claudine/contract/docs/dependencies.md`.
  This plan creates `unchained-ai/contract/docs/dependencies.md` (matching
  precedent) and updates root `docs/dependencies.md`. An area-level
  `unchained-ai/docs/dependencies.md` is **not** created unless the area owner
  requests it — surface it in review if the spec author intended otherwise.
- **Object-safety / async bound.** Use the contract's default
  `#[async_trait]` (Send) bound to match `biscuit-contract` exactly.

### Validation commands (used throughout)

| What | Command |
|------|---------|
| Workspace sees the crate | `cargo metadata --no-deps --format-version 1 \| jq -r '.packages[].name' \| grep unchained-ai-contract` |
| Type-check one crate | `cargo check -p unchained-ai-contract` / `cargo check -p unchained-ai` |
| Area lifecycle | `just -f unchained-ai/justfile build` / `test` / `lint` / `doctest` / `sanity` |
| Canonical recipe gate | `just check-canonical unchained-ai` |
| Full PR gate (area) | `just -f unchained-ai/justfile all` |

---

## Phase 1 — Scaffolding & Workspace Wiring

Goal: a compiling, empty `unchained-ai-contract` crate, wired into the
workspace and the area `justfile`, with the canonical recipe set in place. No
behavior yet. Everything downstream depends on this phase.

**Tasks**

- [x] Create the crate skeleton at `unchained-ai/contract/`:
  `Cargo.toml` (name `unchained-ai-contract`, lib name `unchained_ai_contract`,
  `edition = "2024"`, `license = "AGPL-3.0-only"`) and `src/lib.rs` with a
  `//!` crate doc line and nothing else. Do **not** add dependencies beyond
  what is needed to compile the stub (the full dep set lands in Phase 4).
- [x] Add `"unchained-ai/contract"` to the `members` array in the root
  `Cargo.toml` (keep the list sorted/grouped with the other `unchained-ai/*`
  entries).
- [x] Create `unchained-ai/contract/justfile` mirroring
  `claudine/contract/justfile`: same shared imports (`../../just/*.just`),
  `LIBRARY := "unchained-ai-contract"`, and the canonical 12-recipe set
  (`build sanity test test-l2 test-l3 test-browser test-real lint bench
  coverage doctest fuzz all`). `test-real` gates on an env var (e.g.
  `UNCHAINED_AI_CONTRACT_REAL=1`) and the `--test real_provider` binary (created
  in Phase 5; for now it can be a placeholder recipe).
- [x] Add `unchained-ai-contract` to the relevant recipes in
  `unchained-ai/justfile` so the area lifecycle covers it: `build`, `sanity`,
  `test`, `lint`, `coverage`, `doctest` (mirror how `claudine/justfile` adds
  `claudine-contract` to each). Leave `test-l2/l3/browser/fuzz/bench` as the
  existing "not applicable" area stubs.
- [x] Run `cargo check -p unchained-ai-contract` and confirm it succeeds.

**Validation checkpoint**

- [x] `cargo metadata --no-deps --format-version 1` lists `unchained-ai-contract`.
- [x] `just check-canonical unchained-ai` passes (all 12 recipes defined for
      the area's justfiles — note `check-canonical` parses each area's
      top-level justfile; confirm whether the contract sub-justfile is covered
      or exempt, matching how `claudine-contract` is treated).
- [x] `just -f unchained-ai/justfile build` builds all area crates including
      the new empty contract crate with no warnings about the stub.

---

## Phase 2 — Reusable Execution Surface & Resolver (`unchained-ai/lib`)

Goal: the foundational layer both `Prompt::execute()` and the adapter build on.
Two independent modules; the two task groups below may be developed in
parallel within this phase.

### 2A — Completion execution surface (`execution/`)

- [x] Add `unchained-ai/lib/src/execution/mod.rs` exposing the v1 surface:
  `CompletionRequest` (model: `ProviderModel`, system_prompt, prompt, schema:
  `Option<serde_json::Value>`, parameters), `CompletionOutput` (text for prose;
  raw `serde_json::Value` for structured — **no** schema validation here, per
  spec), and a `ResolvedParameters` type seeded from
  `ProviderModelMetadata::default_parameters`.
- [x] Implement `pub async fn complete(request) -> Result<CompletionOutput,
  ProviderError>`: build the rig-core completion client from
  `ProviderModel::provider()` + `Provider::config()` (reuse the existing
  OpenAI-compatible `client_adaptors` where the provider maps to one; use
  rig-core's `from_env()`/provider clients for the rest). Missing credentials
  surface as `ProviderError::MissingApiKey`. Prose path: single completion
  with system + user prompt. Structured path: prompt-and-parse (combine
  adapter/caller schema into a JSON-only instruction, parse the model text into
  a `Value`, return it raw).
- [x] **Inject a client/transport seam** so the logic is unit-testable without
  a network. Model the seam as a trait (or a `CompletionModel`-like handle)
  the public `complete` accepts, with a production constructor that builds the
  real rig client. The seam is what Phase 4 and the L1 tests inject a fake
  into. Verify the seam design against rig-core v0.31's `CompletionModel` /
  `CompletionClient` API before locking it in.
- [x] Apply generation parameters from metadata defaults, overridable by the
      caller (the adapter layers profile-derived parameters on top in Phase 4).

### 2B — Model capability stack resolver (`models/selection.rs`)

- [x] Add `unchained-ai/lib/src/models/selection.rs` with an ordered-stack
  resolver: given a `ModelCapability`, return the first `ProviderModel` whose
  provider is runnable. "Runnable" = `Provider::is_local()` **or** at least one
  of `Provider::config().env_vars` is present and non-empty in an **injected
  environment view** (never `std::env` directly — L1 tests must not mutate the
  process environment).
- [x] Define the canonical stack for each `ModelCapability` variant (the
  `ModelCapability` doc-comments already describe the intended ordering, e.g.
  Fast stacks latest fast models with US providers before Chinese/local;
  `*Cheap` variants put cheaper models first). Encode a defensible default
  stack per variant; document the ordering in `///` on the resolver.
- [x] Return a typed "no runnable model" outcome (the adapter maps it to
  `InferenceErrorKind::Unavailable` in Phase 4).
- [x] Export the resolver from `models/mod.rs` and `lib.rs` so both the
  adapter and `Prompt::execute()` use the same path (prevents drift).

### Phase 2 tests & checkpoint

- [x] L1 tests for `complete()` prose + structured paths through the **injected
  fake seam** (no network): assert text round-trip for prose, and raw-JSON
  return for structured (validation is the caller's job here).
- [x] L1 tests for the resolver using an injected env view: at least one
  credential-backed provider case and one local-provider (e.g. Ollama) case;
  assert stack ordering and the "no runnable model" outcome when nothing is
  configured.
- [x] `just -f unchained-ai/justfile test` passes for `unchained-ai` (lib).
- [x] `just -f unchained-ai/justfile lint` is clean for `unchained-ai`.

---

## Phase 3 — `Prompt::execute()` Synchronous Bridge (`unchained-ai/lib`)

Goal: replace the stub so `Prompt` routes through the Phase 2 surface. Depends
on Phase 2. **Parallelizable with Phase 4** (different crate).

**Tasks**

- [x] Add `complete_blocking` in `unchained-ai/lib` (e.g. in `execution/`): a
  synchronous bridge that runs the async `complete` path on a dedicated
  current-thread runtime / worker thread. It must **not** call
  `Handle::block_on` from inside an already-running async runtime (that panics
  or stalls). Document the runtime discipline in `///`.
- [x] Rewire `Prompt::execute()` and `execute_readonly()` at
  `unchained-ai/lib/src/primitives/atomic/prompt.rs:365` to delegate through
  `complete_blocking` + the Phase 2 resolver instead of returning the
  "not yet implemented" error. Preserve the existing `Runnable` signature and
  `StepError` mapping.
- [x] Resolve the `Prompt`'s `ModelCapability` to a concrete `ProviderModel`
      via the Phase 2 resolver, then run `complete_blocking` with the prompt's
      system prompt / text / structured-response schema.

**Validation checkpoint**

- [x] L1 test: `Prompt::execute()` no longer returns "not yet implemented";
  it calls the execution surface via the injected fake seam and returns text
  without requiring a real provider. Cover both `execute` and
  `execute_readonly`.
- [x] L1 test: calling `Prompt::execute()` from inside an active Tokio runtime
  does not panic (regression guard for the `block_on` footgun).
- [x] `just -f unchained-ai/justfile test unchained-ai` passes; no existing
      `unchained-ai` test regresses.
- [x] `just -f unchained-ai/justfile doctest` still passes (Prompt docstrings
      that reference execution remain accurate; fix drifted `///` per the
      comment-drift rule — assume code is correct, fix the comment).

---

## Phase 4 — Adapter Implementation (`unchained-ai/contract`)

Goal: a complete, object-safe `UnchainedInferenceAdapter` implementing
`InferenceAdapter`, validated by L1 tests through a fake completion seam.
Depends on Phase 2. **Parallelizable with Phase 3**.

**Tasks**

- [x] Finalize `unchained-ai/contract/Cargo.toml` dependencies (mirror
  `claudine/contract/Cargo.toml`): `biscuit-contract` (path
  `../../biscuit-contract/lib`), `unchained-ai` (path `../lib`), `async-trait`,
  `tokio`, `serde_json`, `jsonschema` (`0.42`, the workspace pin,
  `default-features = false`, Draft 2020-12), `thiserror`, `tracing`. Add
  `wiremock`/`serial_test` to `[dev-dependencies]` as needed for L1 seams.
- [x] `src/error.rs`: map `ProviderError` and rig-core failures onto
  `InferenceError`/`InferenceErrorKind` per the spec's error table (MissingApiKey
  / empty creds on an explicit `with_model` → `Unauthorized`; auth rejected →
  `Unauthorized`; 429 → `RateLimited` + `retry_after` when present; 5xx /
  network unreachable → `Unavailable`; deadline → `Timeout`; etc.). Ensure
  `InferenceError::message` never carries API keys, auth headers, or full
  provider payloads — redact into `tracing`.
- [x] `src/profile.rs`: translate `InferenceProfile` → `ModelCapability` →
  `ProviderModel`. Step 1: priority + reasoning → capability tier per the
  spec's table (Cost→`NormalCheap`/`FastCheap`, Latency→`Fast`, Quality→`Smart`,
  Balanced→`Normal`) with `ReasoningEffort` selecting `*Thinking`/`*Ultrathink`
  variants, dropping to the nearest available variant when none exists.
  Step 2: resolve via the Phase 2 stack resolver with an injected env view.
  Step 3: translate `ReasoningEffort` into a provider reasoning parameter only
  when `ProviderModelMetadata::supported_parameters` exposes a known one
  (Anthropic thinking budget / OpenAI reasoning-effort); otherwise omit. Step
  4: `with_model` pins a `ProviderModel` directly (profile still drives
  reasoning/parameters).
- [x] `src/structured.rs`: schema validation owned by this crate. Mirror the
  `claudine/contract/src/structured.rs` approach (`compile_schema`,
  `augment_prompt`, `extract_json` with the single-value / balanced-span /
  reject-multiple-values rule, `validate_instance`). Reject invalid schema as
  `InvalidRequest` before any provider call; reject invalid JSON / schema
  mismatch / prose-when-structure-requested as `InvalidResponse`.
- [x] `src/adapter.rs`: `UnchainedInferenceAdapter` with `new()` (profile-driven),
  `with_model(ProviderModel)` (pins, bypasses selection), and `build() -> 
  Arc<dyn InferenceAdapter>`. The `#[async_trait]` `infer` impl: validate
  request (empty prompt, malformed schema, unparseable override →
  `InvalidRequest`), resolve model, run prose/structured via the Phase 2
  `complete` surface, validate structured output against the schema, and
  populate `InferenceMetadata` (`provider` = display name, `model` =
  `wire_id()`/`model_id()`, `agent` = `None`). Inject the completion seam used
  in Phase 2 so L1 tests run without a network.
- [x] `src/lib.rs`: wire modules, re-export `UnchainedInferenceAdapter`.

**Validation checkpoint**

- [x] L1: every `InferencePriority` × `ReasoningEffort` combination resolves
  to the expected `ModelCapability` and, under an injected env view, the
  expected `ProviderModel` (include a credential-backed case and a
  local-provider case).
- [x] L1: structured success against a schema; `InvalidResponse` for invalid
  JSON, schema violation, prose-when-structure-requested, and multi-value
  output — all driven through the fake completion seam.
- [x] L1: every `InferenceErrorKind` produced from the corresponding simulated
  provider/transport outcome (use the fake seam to inject each failure mode).
- [x] L1: object-safety — store and call through `Arc<dyn InferenceAdapter>`.
- [x] L1: `with_model` bypasses selection; missing/empty credentials on an
  explicit model → `Unauthorized`; nothing runnable in the stack →
  `Unavailable`.
- [x] `just -f unchained-ai/justfile test unchained-ai-contract` passes; `lint`
  and `doctest` clean.

---

## Phase 5 — Real-Provider Tier, Docs & Skills

Goal: close out the spec's success criteria. Depends on Phases 3 and 4.

**Tasks**

- [x] Add `unchained-ai/contract/tests/real_provider.rs` (gated, opt-in):
  against a real provider with credentials in the environment, prove one prose
  and one structured request complete end-to-end and that structured output
  validates against the bundled JSON Schema engine. Skip cleanly when no
  provider credentials are present. Wire it to the `test-real` recipe's env
  gate (`UNCHAINED_AI_CONTRACT_REAL=1`).
- [x] Create `unchained-ai/contract/docs/dependencies.md` (mirror
  `claudine/contract/docs/dependencies.md`): document that
  `unchained-ai-contract` is the one crate depending on **both**
  `biscuit-contract` and `unchained-ai`, list the additional deps
  (`async-trait`, `tokio`, `serde_json`, `jsonschema 0.42`, `thiserror`,
  `tracing`), and state the forbidden-dependency boundary (no `rig-core` or
  provider-native types leak through the contract).
- [x] Update root `docs/dependencies.md`: add a "Recent Dependency Notes"
  bullet for `unchained-ai/contract` and a Workspace Packages entry for
  `unchained-ai-contract` (mirror the `claudine-contract` entries). Also bump
  the `rig-core` version note if the Phase 2 work confirmed a version other
  than the currently-documented one.
- [x] Update the `unchained-ai` skill (`.opencode/skill/unchained-ai/SKILL.md`
  and/or its detailed references): document the new `execution/` surface, the
  `models/selection.rs` stack resolver, and the fact that `Prompt::execute()`
  is no longer a stub. Move `Prompt::execute()` out of the "Not implemented"
  list in the skill's Implementation Status section.
- [x] Update the `biscuit-contract` skill to note the second
  `InferenceAdapter` implementation (`unchained-ai-contract`) alongside
  `claudine-contract`.
- [x] Drift sweep: confirm no other README/skill/dependencies doc still
  claims `Prompt::execute()` is unimplemented or that `unchained-ai` has no
  contract crate.

**Validation checkpoint (success criteria)**

- [x] `just -f unchained-ai/justfile all` passes (full canonical PR gate for
  the area: sanity → lint → doctest → test → test-l2 → test-browser).
- [x] `just check-canonical unchained-ai` passes.
- [x] `cargo test -p unchained-ai-contract --doc` passes.
- [x] With provider creds set, `UNCHAINED_AI_CONTRACT_REAL=1 cargo test -p
  unchained-ai-contract --test real_provider` passes; without creds it skips
  cleanly.
- [x] `UnchainedInferenceAdapter` is constructible as
  `Arc<dyn InferenceAdapter>` and both a prose and a structured request
  succeed end-to-end through the fake seam in L1.
- [x] `biscuit-contract` public API is unchanged (`cargo test -p
  biscuit-contract` still green).

---

## Dependency Graph & Parallelization

```
Phase 1 (scaffolding)
   │
   ▼
Phase 2 (lib: execution + resolver)   ── two modules, parallelizable within the phase
   │
   ├──────────────┬───────────────────┐
   ▼              ▼                   ▼
Phase 3       Phase 4           (Phase 4 cont.)
(Prompt       (adapter crate)    structured/error
 bridge)                        can start as soon
   │              │              as Phase 2 lands)
   └──────┬───────┘
          ▼
      Phase 5 (real tier + docs + skills + full gate)
```

- **Phase 2** unblocks everything; start here after Phase 1.
- **Phase 3 and Phase 4 are parallelizable** after Phase 2 (different crates,
  no cross-dependency).
- Within **Phase 4**, `error.rs` and `structured.rs` are independent of the
  Phase 2 resolver and can be written first; `profile.rs` and `adapter.rs`
  depend on both Phase 2 and the earlier Phase 4 modules.

## Risk Notes (for implementers, not blockers)

- **rig-core v0.31 API surface.** The exact `CompletionModel` / streaming /
  JSON-mode API should be verified against the 0.31 docs before finalizing the
  execution seam trait shape. v1 uses prompt-and-parse deliberately; native
  rig extractor/JSON-mode is a later optimization, not required here.
- **Synchronous bridge runtime discipline.** The `complete_blocking` bridge is
  the most footgun-prone piece. Test the "called from inside a runtime" case
  explicitly (Phase 3 checkpoint) — `Handle::block_on` in that situation
  panics.
- **Resolver stack ordering is a judgment call.** Encode a defensible default
  per `ModelCapability` variant and document it; the spec allows approximation.
  Do not block on perfectly tuned ordering for v1.
