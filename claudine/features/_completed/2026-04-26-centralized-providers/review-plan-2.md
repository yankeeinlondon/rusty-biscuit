# Centralized Provider Review — Implementation Plan

**Source:** Review-2 findings for `2026-04-26-centralized-providers`  
**Goal:** Address all critical, functional, and ergonomic gaps to reach production-ready state.  
**Phases:** 3 (compilation/migration → type completeness → ergonomics/tests)

---

## Phase 1: Stop-the-Bleed — Compilation Fix & Missed Dispatch Points

**Objective:** Fix the critical compilation error and eliminate the remaining `match provider` blocks that bypass the centralized catalog.

### 1.1 Fix Compilation Error
- **File:** `claudine/lib/src/provider/methods.rs:145`
- **Change:** Replace `if let Some(_event) = provider_info(provider).adapter.detect(raw)` with `if provider_info(provider).adapter.detect(raw)`.
- **Rationale:** `AdapterBehavior::detect` returns `bool`, not `Option<_>`.
- **Verification:** `cargo check` in `claudine/lib` must pass.

### 1.2 Migrate Missed Dispatch Points
Migrate the three remaining `match provider` blocks to query `ProviderInfo` fields.

#### A. `permissions::query::is_cli_sensitive`
- **File:** `claudine/lib/src/permissions/query.rs`
- **Current:** Manual `match provider` block.
- **Target:** Query `provider_info(provider).cli_sensitive_axes` (or equivalent catalog field).
- **Fallback:** If the catalog field does not yet exist, add `cli_sensitive_axes: &'static [CliSensitiveAxis]` to `ProviderInfo` and populate it per provider.

#### B. `composition::select::provider_env_vars`
- **File:** `claudine/lib/src/composition/select.rs`
- **Current:** Manual `match provider` block.
- **Target:** Query `provider_info(provider).model_env_vars`.
- **Fallback:** If the catalog field does not yet exist, add `model_env_vars: &'static [ModelEnvVar]` to `ProviderInfo` and populate it per provider.

#### C. `model_catalog::provider_sources`
- **Files:** `claudine/lib/src/model_catalog/provider_sources.rs` (both `static_catalog_for_provider` and `fetch_provider_catalog`)
- **Current:** `match provider` blocks returning `Vec<String>`.
- **Target:** 
  - Return `&'static [String]` (or `&'static [&'static str]`) from `ProviderInfo.static_models`.
  - Return `ProviderInfo.dynamic_source` for the fetch path.
- **Data migration:** Move the static model lists and dynamic source URLs into each provider's `ProviderInfo` constant.

### 1.3 Tests
- **Compile test:** `cargo check` and `cargo test` must pass before any other work.
- **Dispatch point tests:** For each migrated module, add an exhaustiveness test that iterates `PROVIDERS_DISPLAY_ORDER` and asserts the new catalog field is non-empty (or matches the old `match` arm behavior via a temporary snapshot).
- **Regression test:** Run existing matrix tests (`claudine providers`, `claudine hooks --support`, etc.) and confirm bit-for-bit output equivalence.

### 1.4 Lint Cleanup
- Run `just lint` (or `cargo clippy --all-targets --all-features` in `claudine/`).
- Fix any new warnings introduced by the migration.

### 1.5 Deliverable
Library compiles, all existing tests pass, zero `match provider` blocks remain in `lib` outside `provider::registry`.

---

## Phase 2: Type System Completeness — EventSupportLevel & AdapterBehavior

**Objective:** Close the functional gaps where the implementation diverged from the spec's typed design.

### 2.1 Expand `EventSupportLevel` to Data-Carrying Variants
- **File:** `claudine/lib/src/provider/event_mapping.rs`
- **Current:**
  ```rust
  pub enum EventSupportLevel {
      NotSupported,
      Hook,
      NonHook,
      Acp,
  }
  ```
- **Target:**
  ```rust
  pub enum EventSupportLevel {
      NotSupported,
      Hook { native_name: &'static str },
      StreamParse { protocol: StreamProtocol, native_name: &'static str },
      WireProxy { mode: WireProxyMode, native_name: &'static str },
      Acp { event: AcpEvent, native_name: &'static str },
      Wrapper { native_name: &'static str },
  }
  ```
- **Migration:** Update every `EventMapping` in every provider file to carry the native name and mechanism data previously stored in parallel `native_event_name` matches.
- **Consumer updates:** Update all call sites that match on `EventSupportLevel` to handle the new payloads (e.g., `events::matrix.rs`, `hooks --support` CLI output).

### 2.2 Add `parse_payload` to `AdapterBehavior`
- **File:** `claudine/lib/src/provider/behavior.rs`
- **Current:**
  ```rust
  pub trait AdapterBehavior: Send + Sync + std::fmt::Debug + 'static {
      fn detect(&self, raw: &serde_json::Value) -> bool;
      fn provider_adapter(&self) -> &'static dyn ProviderAdapter;
  }
  ```
- **Target:**
  ```rust
  pub trait AdapterBehavior: Send + Sync + std::fmt::Debug + 'static {
      fn parse_payload(&self, raw: &serde_json::Value) -> Option<AgenticEvent>;
      fn detect(&self, raw: &serde_json::Value) -> bool;
      fn provider_adapter(&self) -> &'static dyn ProviderAdapter;
  }
  ```
- **Implementation:** Move the body of each `adapters/<provider>.rs` payload parser into the corresponding provider's `AdapterBehavior::parse_payload` impl.
- **Fallback:** Default impl returns `None`.

### 2.3 Remove Redundant Native-Name Matches
- **File:** `claudine/lib/src/events/provider.rs`
- **Action:** Delete or thin `Provider::native_event_name`, `Provider::event_support_level`, and `Provider::registration_native_event_name` to forward into `provider_info(self).event_mapping`.
- **Goal:** `events/provider.rs` shrinks by at least 50% as spec requires.

### 2.4 Tests
- **Unit tests:** For each provider, assert that `parse_payload` round-trips a known-good payload to the expected `AgenticEvent`.
- **Property tests (post-phase):** For every provider × every `AgenticEvent`:
  - If support is not `NotSupported`, `native_name` is non-empty.
  - If support is `Hook { .. }`, configurator has at least one supported format.
  - If support is `StreamParse { .. }`, `stream_protocol` is `Some`.
  - If support is `Acp { .. }`, `AcpSupport` is non-`NotSupported`.
- **Snapshot tests:** `claudine hooks --mapping` and `claudine hooks --support` output must be bit-for-bit identical to Phase 1 (the new typed data is internal only at this stage).

### 2.5 Lint Cleanup
- Run `just lint`.
- Remove any now-unused imports in `events/provider.rs` and `adapters/`.

### 2.6 Deliverable
`EventSupportLevel` carries typed capture mechanism data; `AdapterBehavior` exposes `parse_payload`; `events/provider.rs` is dramatically thinner.

---

## Phase 3: Registry Ergonomics, Wrapper Thinning & Test Hardening

**Objective:** Align the registry implementation with the design, thin wrapper profiles, and harden test coverage.

### 3.1 Convert Registry to Array-Backed `OnceLock`
- **File:** `claudine/lib/src/provider/registry.rs`
- **Current:** Exhaustive `match` block.
- **Target:**
  ```rust
  static REGISTRY: OnceLock<[&'static ProviderInfo; 8]> = OnceLock::new();

  pub fn provider_info(p: Provider) -> &'static ProviderInfo {
      let registry = REGISTRY.get_or_init(|| [
          &CLAUDE_INFO, &CODEX_INFO, &GEMINI_INFO, &GOOSE_INFO,
          &KIMI_INFO, &OPENCODE_INFO, &QWEN_INFO, &ROO_INFO,
      ]);
      registry[p as usize]
  }
  ```
- **Requirements:**
  - `Provider` must derive `#[repr(usize)]` starting at 0 (verify this is already true).
  - Add a compile-time or test-time invariant that `registry.len() == PROVIDER_COUNT`.
  - If `strum::EnumCount` is available, use `Provider::VARIANT_COUNT` for the array length.

### 3.2 Thin WrapperProfile with Catalog Defaults
- **File:** `claudine/cli/src/commands/wrap/profile.rs`
- **Action:** Convert the following `WrapperProfile` methods to default implementations that read from `provider_info(self.provider()).capabilities`:
  - `apply_yolo` → reads `capabilities.permissions.yolo`
  - `has_supported_yolo` → reads same field
  - `reject_direct_yolo` → reads same field
  - `apply_output_format` → reads `capabilities.non_interactive.output_formats`
  - `apply_entrypoint` → reads `capabilities.non_interactive.entrypoints`
  - `apply_model` → reads `capabilities.model.cli_flags`
  - `prompt_arg_conventions` → reads `capabilities.non_interactive`
  - `stream_protocol` → reads `stream_protocol`
  - `supports_structured_stream` / `supports_resume` → reads `capabilities.stream`
- **Per-provider cleanup:** Delete trivial overrides in `ClaudeWrapper`, `KimiWrapper`, etc., that now match the catalog-derived default.
- **Preserved overrides:** Keep only genuinely quirky methods (Kimi's `--agent-file`, OpenCode's mode-conditional YOLO, Codex's `model_instructions_file`).

### 3.3 Add `detect_from_payload` Integration Tests
- **File:** `claudine/lib/src/provider/tests.rs` (or new `tests/integration/detect.rs`)
- **Test matrix:** For every provider, supply a representative JSON payload and assert:
  - `provider_info(p).behavior.detect_from_payload(payload) == true` for that provider's payload.
  - `provider_info(other).behavior.detect_from_payload(payload) == false` for all other providers.
- **Goal:** Prevent the Phase 1 compilation error class from recurring (a test that exercises `detect` on real data would have caught the `bool` vs `Option` mismatch at test time, if not compile time).

### 3.4 Add Registry Exhaustiveness Compile-Time Check
- **File:** `claudine/lib/src/provider/tests.rs`
- **Test:**
  ```rust
  #[test]
  fn registry_array_length_matches_variant_count() {
      const _COUNT_CHECK: [(); 8] = [(); Provider::VARIANT_COUNT];
      let registry = REGISTRY.get().expect("registry initialized");
      assert_eq!(registry.len(), Provider::VARIANT_COUNT);
  }
  ```

### 3.5 Lint Cleanup
- Run `just lint`.
- Run `just doctest`.
- Confirm `cli/src/commands/wrap/profile.rs` LOC has dropped by at least 40%.

### 3.6 Deliverable
Registry is O(1) array-backed; wrapper profiles are thinner; integration tests cover payload detection; all lint/doc tests pass.

---

## Summary

| Phase | Focus | Key Deliverables |
|---|---|---|
| **1** | Critical fixes | Compiles; zero missed dispatch points; all matrix tests pass |
| **2** | Type completeness | Data-carrying `EventSupportLevel`; `AdapterBehavior::parse_payload`; thinner `events/provider.rs` |
| **3** | Ergonomics & tests | Array-backed `OnceLock` registry; thinned `WrapperProfile`; `detect_from_payload` integration tests; 40% wrapper LOC reduction |

**Total estimated PRs:** 3 (one per phase). Each phase is independently shippable and passes `just test | lint | doctest` before merging.
