---
created: 2026-07-16
phase: 8
total_phases: 8
agent: claude/default
yolo: "true"
packages:
    - claudine-cli
source_files_during_phase_1:
    - claudine/cli/tests/characterization_error_routes.rs
    - .config/nextest.toml
docs_updated_during_phase_1:
    - claudine/features/2026-07-13-error-propogation/plan.md
docs_created_during_phase_1:
    - claudine/features/2026-07-13-error-propogation/decisions.md
    - claudine/features/2026-07-13-error-propogation/inventory.md
skills_files_updated_during_phase_1: []
packages_during_phase_2:
    - claudine
    - claudine-cli
source_files_during_phase_2:
    - claudine/lib/src/diagnostics/discovery.rs
    - claudine/lib/src/diagnostics/discovery/tests.rs
    - claudine/lib/src/diagnostics/mod.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/cli/tests/diagnostic_discovery.rs
docs_updated_during_phase_2:
    - claudine/features/2026-07-13-error-propogation/plan.md
    - claudine/features/2026-07-13-error-propogation/decisions.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_3:
    - claudine
source_files_during_phase_3:
    - claudine/lib/src/diagnostics/snapshot.rs
    - claudine/lib/src/diagnostics/snapshot/tests.rs
    - claudine/lib/src/diagnostics/mod.rs
    - claudine/lib/src/diagnostics/registry.rs
    - claudine/lib/src/diagnostics/discovery.rs
    - claudine/lib/src/diagnostics/discovery/tests.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/harness/runtime.rs
    - claudine/lib/src/harness/mod.rs
docs_updated_during_phase_3:
    - claudine/features/2026-07-13-error-propogation/plan.md
    - claudine/features/2026-07-13-error-propogation/decisions.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_4:
    - claudine
    - claudine-cli
source_files_during_phase_4:
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/mod.rs
    - claudine/lib/src/harness/resolve.rs
    - claudine/lib/src/error.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/diagnostics/discovery/tests.rs
    - claudine/lib/src/diagnostics/snapshot/tests.rs
    - claudine/lib/src/system_prompt/context.rs
    - claudine/lib/src/system_prompt/change_state.rs
    - claudine/lib/src/linking/hashing.rs
    - claudine/lib/src/stream/reporting.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/commands/mcp/show.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/overlay.rs
    - claudine/cli/src/commands/wrap/wrapper_mcp.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/src/commands/wrap/env/mod.rs
    - claudine/cli/src/commands/wrap/sequence/resolve.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/target.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/docs/providers/dispatch-inventory.json
docs_updated_during_phase_4:
    - claudine/features/2026-07-13-error-propogation/plan.md
    - claudine/features/2026-07-13-error-propogation/decisions.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_5:
    - claudine
    - claudine-cli
source_files_during_phase_5:
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/diagnostics/discovery.rs
    - claudine/lib/src/diagnostics/mod.rs
    - claudine/lib/src/error.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/output/error_walker.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - claudine/cli/tests/characterization_error_routes.rs
    - claudine/cli/tests/effective_diagnostic_render.rs
    - claudine/cli/tests/wrap_basics.rs
    - claudine/cli/tests/wrap_compose_validation.rs
docs_updated_during_phase_5:
    - claudine/features/2026-07-13-error-propogation/plan.md
    - claudine/features/2026-07-13-error-propogation/decisions.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_6:
    - claudine-cli
source_files_during_phase_6:
    - claudine/cli/tests/error_guards.rs
    - claudine/cli/tests/error_guards/source_scan.rs
    - claudine/cli/tests/error_guards/transport-allow.toml
    - claudine/cli/Cargo.toml
    - claudine/justfile
    - scripts/check-error-transport.sh
    - scripts/check-error-transport.allow
docs_updated_during_phase_6:
    - claudine/features/2026-07-13-error-propogation/plan.md
    - claudine/features/2026-07-13-error-propogation/decisions.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_7:
    - claudine-cli
source_files_during_phase_7:
    - claudine/cli/tests/level2_typed_error_render_capture.rs
    - claudine/cli/tests/effective_diagnostic_render.rs
    - claudine/cli/tests/level2_lifecycle_control.rs
    - claudine/cli/tests/level2_lifecycle_dispatch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
docs_updated_during_phase_7:
    - claudine/features/2026-07-13-error-propogation/plan.md
    - claudine/features/2026-07-13-error-propogation/decisions.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
packages_during_phase_8:
    - claudine
    - claudine-cli
source_files_during_phase_8:
    - claudine/lib/src/diagnostics/registry.rs
    - claudine/lib/src/diagnostics/mod.rs
    - claudine/lib/src/diagnostics/facets.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/cli/tests/error_guards.rs
    - claudine/cli/tests/error_guards/source_scan.rs
    - claudine/cli/tests/error_guards/boxed-diagnostic-allow.toml
    - claudine/cli/tests/level2_lifecycle_dispatch.rs
    - claudine/justfile
    - scripts/check-lifecycle-doc-facets.sh
docs_updated_during_phase_8:
    - claudine/docs/topics/lifecycle.md
    - claudine/features/2026-07-13-error-propogation/plan.md
    - claudine/features/2026-07-13-error-propogation/decisions.md
    - claudine/features/2026-07-13-file-resolution/spec.md
docs_created_during_phase_8:
    - claudine/docs/topics/error-architecture.md
skills_files_updated_during_phase_8:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/timeline.md
    - .claude/skills/claudine/error-architecture.md
packages:
    - claudine
    - claudine-cli
source_code:
    - claudine/lib/src/diagnostics/discovery.rs
    - claudine/lib/src/diagnostics/discovery/tests.rs
    - claudine/lib/src/diagnostics/snapshot.rs
    - claudine/lib/src/diagnostics/snapshot/tests.rs
    - claudine/lib/src/diagnostics/registry.rs
    - claudine/lib/src/diagnostics/facets.rs
    - claudine/lib/src/diagnostics/mod.rs
    - claudine/lib/src/error.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/mod.rs
    - claudine/lib/src/harness/resolve.rs
    - claudine/lib/src/harness/runtime.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/system_prompt/context.rs
    - claudine/lib/src/system_prompt/change_state.rs
    - claudine/lib/src/linking/hashing.rs
    - claudine/lib/src/stream/reporting.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/output/error_walker.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - claudine/cli/src/commands/mcp/show.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/overlay.rs
    - claudine/cli/src/commands/wrap/wrapper_mcp.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/src/commands/wrap/env/mod.rs
    - claudine/cli/src/commands/wrap/sequence/resolve.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/target.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/tests/characterization_error_routes.rs
    - claudine/cli/tests/diagnostic_discovery.rs
    - claudine/cli/tests/effective_diagnostic_render.rs
    - claudine/cli/tests/error_guards.rs
    - claudine/cli/tests/error_guards/source_scan.rs
    - claudine/cli/tests/error_guards/transport-allow.toml
    - claudine/cli/tests/error_guards/boxed-diagnostic-allow.toml
    - claudine/cli/tests/level2_typed_error_render_capture.rs
    - claudine/cli/tests/level2_lifecycle_control.rs
    - claudine/cli/tests/level2_lifecycle_dispatch.rs
    - claudine/cli/tests/wrap_basics.rs
    - claudine/cli/tests/wrap_compose_validation.rs
    - claudine/cli/Cargo.toml
    - claudine/justfile
    - claudine/docs/providers/dispatch-inventory.json
    - .config/nextest.toml
    - scripts/check-lifecycle-doc-facets.sh
documentation:
    - claudine/docs/topics/error-architecture.md
    - claudine/docs/topics/lifecycle.md
    - claudine/features/2026-07-13-error-propogation/plan.md
    - claudine/features/2026-07-13-error-propogation/decisions.md
    - claudine/features/2026-07-13-error-propogation/inventory.md
    - claudine/features/2026-07-13-file-resolution/spec.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/timeline.md
    - .claude/skills/claudine/error-architecture.md
---

# Execution Plan — End-to-End Typed Error Propagation

Derived from [`spec.md`](./spec.md) (reviewed by `codex/default`, 2026-07-16).

## Reconnaissance Findings

Verified against the working tree before planning. These shape the ordering:

- **Three `Diagnostic` impls exist today**: `ClaudineError` (`lib/src/error.rs:254`),
  `CompositionError` (`lib/src/composition/error/render/mod.rs:206`),
  `HarnessError` (`lib/src/harness/error.rs:69`).
- **The motivating bug is confirmed and narrow.** `cli/src/output/error_walker.rs:162`
  `discover_block_error` tries Darkmatter's `as_block_error`, then downcasts
  **only** `CompositionError`. `ClaudineError` and `HarnessError` implement
  `Diagnostic` but are invisible to the walker — so
  `HarnessError::PathResolutionFailed` can never render a block. This is exactly
  spec §"Failure Pattern 2".
- **The walker is `deepest`-wins today** (`deepest_block_error`, line 148). D4
  replaces this with role-based selection (first semantic/owning wins, else
  deepest transparent). This is a **real selection-behavior change**, not a
  refactor, and needs characterization before it moves.
- **A precursor guard already exists**: `scripts/check-error-transport.sh` +
  `check-error-transport.allow`, wired via `just lint-transport`. It is a grep
  heuristic keyed on `map_err(|e| …)` and — critically — **defaults to only
  `claudine/lib/src/composition`**. D8 requires `lib`, `cli`, and `contract`.
  Extend this asset; do not build a parallel one.
- **Lossy-site scale**: ~50 `eyre!("…{e}…")` + ~11 `map_err(… to_string())` in
  production. Densest: `cli/src/commands/wrap/mod.rs` (6),
  `wrapper_mcp.rs` (5), `harness_orch/loop_control.rs` (5),
  `lib/src/composition/lifecycle/executor.rs` (4), `wrapper_stages.rs` (4),
  `wrap/env/mod.rs` (4), `wrap/composition/pipeline.rs` (4), `signals.rs` (4).
- **`from_action_failure` has ~11 call sites**, nearly all in
  `lib/src/composition/lifecycle/executor.rs` (653, 757, 764, 825, 832, 866,
  934, 944, 950, 987, 992) plus `lifecycle/runtime.rs:234`. This is the D7
  legacy boundary and the single densest migration cluster.
- **`just lint` already runs** `lint-transport` and `lint-lifecycle-doc-facets`.

## Decision Gate (resolve in Phase 1)

The spec's Open Question recommends **Option A** (Claudine semantic adapters)
and states plainly that "D2–D10 assume Option A" and that Option B "should be
ratified as a separate cross-package contract before implementation". This plan
proceeds on **Option A** as the spec's own ratified recommendation. Option C is
explicitly excluded by the spec.

If Ken prefers Option B, **stop after Phase 1 Task 1** — Option B invalidates
D2's crate-local registry and turns this into a cross-area public API change in
`biscuit-terminal`, requiring its own spec.

## Success Criteria

Maps to spec §"Acceptance Criteria" 1–9. The plan is done when:

- The motivating proxy failure renders a `StatusBlock`, never generic `Error:`.
- One Claudine registry is the CLI walker's only Claudine allowlist, and source
  parity proves it complete.
- Rendering, `err.*`, and machine output select the *same* effective diagnostic.
- Exit codes, lifecycle ordering, and retry/resume/proxy decisions are unchanged.
- `just test`, `just test-l2`, `just lint` pass in the `claudine/` area.

---

## Phase 1 — Ratify, Inventory, and Characterize

No production behavior changes. This phase produces the artifacts every later
phase consumes, and the safety net D10 mandates.

- [x] **Record the Option A ruling** in `features/2026-07-13-error-propogation/decisions.md`:
      Claudine owns semantic adapters; every lower-layer (Darkmatter,
      biscuit-file) error crossing into Claudine is retained as `#[source]` by a
      Claudine diagnostic wrapper. Note Option B's exit condition explicitly.
- [x] **Produce the D8 lossy-boundary inventory** at
      `features/2026-07-13-error-propogation/inventory.md`. Scan production Rust
      in `claudine/lib/src`, `claudine/cli/src`, `claudine/contract/src`, plus
      crossings from `claudine-gen` and the rendezvous crates into the core CLI.
      Exclude generated sources, fixtures, and snapshot literals **structurally**
      (path-based), not by substring exception.
- [x] Classify **every** occurrence as one of: (1) typed provenance defect —
      replace; (2) genuinely unstructured external text — retain with a written
      reason; (3) presentation-only after the final render boundary — retain.
      Category 1 rows become Phase 4/5 tasks.
- [x] Confirm the inventory covers the five named anchors — `error_walker.rs`,
      `lifecycle/context.rs` (`from_action_failure`),
      `harness_orch/loop_control.rs`, `harness/error.rs`
      (`PathResolutionFailed { detail: String }`), and `harness_orch/prompt.rs`
      pre-flight. These are starting points, not the allowlist.
- [x] **Enumerate every production `impl Diagnostic for …`** and record it as the
      expected-registry baseline for Phase 2 (today: `ClaudineError`,
      `CompositionError`, `HarnessError` — re-derive rather than trusting this).
- [x] **[D10] Write characterization tests** capturing *pre-migration* behavior
      for each route Phase 4/5 will touch: process exit code, selected lifecycle
      event order, and emission count (exactly-once). These must pass **before
      and after** every later phase, unchanged.
- [x] **[D10] Inventory repo-owned lifecycle examples/tests that consume `err.msg`**
      and record current values, so the documented `err.msg` change to the
      effective diagnostic's concise projection can be verified as still useful
      for TTS/messaging. Cross-check against the known deprecated
      `err.kind`/`err.variant` alias surface (L2 blast radius).

**⛔ Checkpoint 1** — Inventory classifies 100% of matches with no "unclassified"
rows; characterization suite is green on unmodified `HEAD` and is committed as
the behavioral baseline; Option A recorded. **Do not proceed if characterization
is flaky** — a flaky baseline cannot prove D10 neutrality.

---

## Phase 2 — Discovery Registry and Effective-Diagnostic Selection

Library-side contracts. Additive; nothing is rewired to them yet, so this phase
is behavior-neutral by construction.

- [x] **[D4] Add the diagnostic role** to the `Diagnostic` trait in
      `lib/src/diagnostics/mod.rs` as **data or an object-safe method** —
      e.g. `fn role(&self) -> DiagnosticRole` returning `Semantic | Transparent`.
      It must **never** be inferred from enum names or `Display` text. Default
      the method so existing impls compile, then set roles explicitly per variant.
- [x] **[D2] Implement the discovery seam** in `lib/src/diagnostics/` :
      `pub fn as_diagnostic(error: &(dyn Error + 'static)) -> Option<&dyn Diagnostic>`.
      It may be `#[doc(hidden)]` but **must be `pub`** — `claudine-cli` is a
      separate crate. Register every concrete Claudine `Diagnostic` type.
- [x] **[D2] Implement effective-diagnostic selection** as one shared function
      (outer→inner walk): first `Semantic` wins and stops; if only `Transparent`
      diagnostics are found, the deepest wins. It composes with Darkmatter's
      `as_block_error` for lower-layer causes.
- [x] **[D4] Add the traversal guards**: terminate on repeated error-object
      identity and enforce a generous max depth. Reaching either guard is logged
      and **must not** discard the best candidate already selected.
- [x] *(parallelizable)* **[L1]** Test: every `Diagnostic` impl is discoverable
      after erasure to `dyn Error`.
- [x] *(parallelizable)* **[L1]** Test: nested Claudine → Darkmatter →
      biscuit-file chains select the expected diagnostic under both `Semantic`
      and `Transparent` wrapper cases.
- [x] *(parallelizable)* **[L1]** Test: a cyclic and an over-depth `source()`
      chain terminate and preserve the best pre-guard selection.

**⛔ Checkpoint 2** — `just test` green; selection function has explicit test
coverage for both roles; `as_diagnostic` is reachable from `claudine-cli`
(prove with a compile-time use, not by inspection).

---

## Phase 3 — Snapshot Boundary and Catalog Extension

Parallelizable with Phase 2 after the role type lands; both are additive.

- [x] **[D9] Define `DiagnosticSnapshot`** (name may differ) with: schema
      version; category, code, disposition, origin, severity; catalog-shaped
      structured detail; concise notification-safe message; and the one-level
      next registered cause from D4.
- [x] **[D9]** Facet values are **owned strings** at the snapshot boundary even
      though the in-process API uses closed enums — a newer producer must stay
      readable to an older consumer.
- [x] **[D9]** Deserialization preserves **unknown additive** codes and detail
      fields rather than dropping or erroring on them.
- [x] **[D3] Extend `composition.invalid_file_reference`** in
      `lib/src/diagnostics/registry.rs` **additively**. Keep `reference`, `kind`,
      `base_dir`, `suggestions`, `fallback_dir`; add `source_path`, `property`,
      `event`, `repository_root`, `candidates`, `failure`. Document `base_dir`
      and `fallback_dir` as **compatibility projections**. `failure` uses stable
      snake_case slugs (invalid syntax / missing context / no match /
      permission-I/O / unsupported remote).
- [x] **[D3]** Project the **full object shape always**. Fields the current
      private resolver cannot supply are `null` — **never invented, never parsed
      out of `Display`**. The downstream file-resolution feature replaces those
      nulls; this feature must not reverse-engineer them.
- [x] *(parallelizable)* **[L1]** Test: snapshot round-trips every facet,
      detail, message, and one-level cause; unknown additive code/detail values
      survive a read/write cycle.
- [x] *(parallelizable)* **[L1]** Test: `null_detail_for` and the extended
      catalog agree — every declared field is a *present* key, absent optionals
      are `null`, and no registered code returns top-level `null`.

**⛔ Checkpoint 3** — Round-trip and catalog-parity tests green; a diff of the
catalog shows **only additions** (no removed or renamed field).

---

## Phase 4 — Semantic Wrappers and Typed Transport Migration

The first phase that changes production behavior. Run the Phase 1
characterization suite after **every** task in this phase.

- [x] **[D3] Restructure `HarnessError::PathResolutionFailed`** in
      `lib/src/harness/error.rs` — replace `detail: String` with structured
      fields plus the typed lower-level `#[source]` where one exists. Re-check
      its classification: today it maps to `io.read_failed`, which may report an
      **authoring** failure as an environment failure.
- [x] **[D3] Add the composition semantic wrapper** carrying `SourceContext`,
      lifecycle event, action/property path, raw authored value, typed source,
      and a contract-accurate hint. It owns
      **`composition.invalid_file_reference`** — it must **not** introduce a
      proxy-specific code. Event and property path distinguish the surface in
      structured detail only.
- [x] **[D1] Migrate the lifecycle proxy path** (the motivating incident) to
      return the typed wrapper instead of `eyre!`. Verify the original report
      text is gone.
- [x] **[D1] Migrate `harness_orch/loop_control.rs`** (5 sites) — typed errors
      currently converted via `to_string()`.
- [x] **[D1] Migrate the `harness_orch/prompt.rs` pre-flight wrapper** (2 sites).
- [x] **[D1] Migrate remaining Category-1 inventory rows.** Sequence by cluster
      to keep diffs reviewable — `cli/src/commands/wrap/mod.rs`,
      `wrapper_mcp.rs`, `wrapper_stages.rs`, `wrap/env/mod.rs`,
      `wrap/composition/pipeline.rs`, `signals.rs`,
      `lib/src/composition/lifecycle/executor.rs`. **Each cluster is
      independently parallelizable** once the wrapper types exist.
- [x] **[D1]** Every migration uses a permitted mechanism: concrete typed error,
      `#[from]`, `#[source]` variant, or `wrap_err` **only** where the concrete
      source stays in the chain and no structured context is needed.
- [x] **[D10]** Any routing or retry-policy change discovered mid-audit is
      **split into a separate spec** — do not fix it here.
- [x] *(parallelizable)* **[L1]** Test: every contextual wrapper exposes its
      original concrete error through `Error::source()`.

**⛔ Checkpoint 4** — Characterization suite **byte-identical** to Phase 1
baseline (exit codes, event order, emission counts). Any drift here is a defect
in this phase, not an acceptable change.

---

## Phase 5 — Unify Rendering and `err.*` on One Selection

- [x] **[D2/D5] Rewrite `cli/src/output/error_walker.rs`** to call Claudine's
      `as_diagnostic` + the shared selection function. **Delete the direct
      `CompositionError` downcast** — the walker must not keep a second partial
      type list. This is the fix for the motivating incident.
- [x] **[D4] Point `LifecycleErrorInfo` construction at the same selection
      function**, so classification and rendering cannot diverge.
- [x] **[D7] Migrate typed `from_action_failure` callers** (~11 sites, mostly
      `lifecycle/executor.rs`) to pass the selected diagnostic or a snapshot
      when they already hold typed provider/cap/timeout/runaway/harness data. A
      genuinely prose-only action failure **may remain facet-less** — but must
      not claim a registered code while projecting empty/top-level-null detail.
- [x] **[D4] Project `err.msg`** as the selected diagnostic's concise message
      **after** the existing notification-hygiene pass — not its multiline block,
      not a classifier input. Preserve the ratified `harness::failure_message`
      precedence for provider attempt failures.
- [x] **[D4] Add `err.cause.*`** as a strict **one-level** projection of the next
      registered diagnostic. `err.cause.cause` is **not** exposed in v1.
- [x] **[D6] Verify frontmatter enrichment stays transparent** — it must find the
      meaningful inner diagnostic through arbitrary typed wrappers, append the
      excerpt **once**, and leave control-flow matching unchanged.
- [x] **[D5] Confirm exactly one ordinary render boundary.** The early lifecycle
      -evaluation emission stays (catch events must run after the crash is
      visible), marked already-emitted and covered by exactly-once tests. **No
      new early render boundary** without a separate ruling.
- [x] *(parallelizable)* **[L1]** Test: the same selected diagnostic produces the
      terminal rendering, the `LifecycleErrorInfo`, and the serialized snapshot.
- [x] *(parallelizable)* **[L1]** Test: every `err.msg` is escape-free,
      single-line, non-empty, within the ~240-char cap; provider cascade
      precedence unchanged.

**⛔ Checkpoint 5** — Characterization suite still baseline-identical. The
motivating failure renders a `StatusBlock` when run by hand. No route classifies
one cause while rendering another.

---

## Phase 6 — Regression Enforcement

Parallelizable with Phase 7.

- [x] **[D8] Widen and harden the lossy-boundary guard.** Extend the existing
      `scripts/check-error-transport.sh` (currently `map_err(|e| …)`-keyed and
      scoped to `lib/src/composition`) into a **Rust-aware** scan — parse with
      `syn` rather than grep — covering `lib/src`, `cli/src`, `contract/src`.
      Detect all six D8 shapes: formatted-report construction, `to_string()`
      map_err, error-bearing `reason`/`message: String` fields, pre-return
      `format!` context, log-then-return-another-error, and `Diagnostic`/
      `BlockError` impls absent from discovery.
      *Landed as `cli/tests/error_guards.rs` + `error_guards/source_scan.rs`; the
      grep script and its allowlist are deleted. The scan keys on binding
      **provenance** (a `map_err`/`or_else`/`unwrap_or_else` closure parameter or
      an `Err(e)` pattern) and on **retention** — a body that keeps the typed
      value (`Foo { message: e.to_string(), source: e }`) is not a defect, which
      is the distinction a grep cannot draw and the reason all 13 Category-0 rows
      of the Phase 1 inventory are correctly ignored.*
- [x] **[D8] Rework the allowlist** so each exception is tied to an **enclosing
      symbol** (not just a trimmed line, which currently matches by text across
      files) and carries a written reason why no typed source exists.
      *`cli/tests/error_guards/transport-allow.toml`, keyed `(shape, file,
      symbol)` and carrying a `tag` + `reason`. See §D-6 below on the 77-entry
      grandfather.*
- [x] **[D2] Add the Rust-aware source-parity test**: scan production sources for
      every `impl Diagnostic for …` and fail when one is missing from the
      registry. Rust has no reflection that makes a hand-authored downcast list
      exhaustive — this test is what makes D2 enforceable.
      *`registry_lists_every_diagnostic_impl` compares the `impl Diagnostic for …`
      set against the `downcast_ref::<T>()` arms parsed out of `as_diagnostic`.
      Both sides are derived from source, so neither can be trusted into
      agreement. Fails in **both** directions (unregistered impl, phantom
      downcast).*
- [x] **[D7] Add catalog-parity enforcement**: fail on a missing declared detail
      key, an undeclared ad hoc key, or a registered code whose detail is
      top-level `null`.
      *Four tests, static and runtime: `detail_projections_write_only_declared_keys`
      (syn — every `base["k"] = …` / `json!({"k": …})` key must be declared by one
      of the impl's own codes, exhaustive over all ~70 variants without
      constructing them), `a_diagnostic_claiming_a_registered_code_projects_a_detail`
      (inheriting the `Value::Null` default while claiming a code),
      `from_code_projects_a_catalog_shaped_detail_for_every_registered_code` (all
      43 codes through the synthesized-label path — the exact regression D7
      names), and `every_diagnostic_in_the_corpus_projects_its_catalog_key_set`
      (key-set equality on constructed values), with
      `the_corpus_covers_every_code_a_diagnostic_can_return` keeping the corpus
      complete against the source-derived claimed-code set.*
- [x] Keep `just lint-transport` wired into `just lint`; update its recipe
      comment, which currently describes the narrow Phase-6 composition scope.
      *Now `just _test claudine-cli --test error_guards` (1.4s). The recipe's
      "pure grep heuristic — no cargo" claim is gone: `syn` needs the toolchain,
      which is the deliberate trade for provenance-awareness.*

**⛔ Checkpoint 6** — Each guard **fails on a deliberately introduced violation**
(prove it, then revert). A guard that cannot fail is not a guard. `just lint`
green.

**Checkpoint 6 met.** Every guard was proven against a real injected fault, and
every injection reverted (`git diff` over `lib/` and `cli/src` empty afterwards):

| Guard | Injected fault | Result |
|---|---|---|
| `no_unallowlisted_typed_error_collapses` | a new `map_err(\|e\| format!("could not read: {e}"))` in `harness/resolve.rs` | FAILED, naming the shape, file, line, and new symbol |
| `registry_lists_every_diagnostic_impl` | deleted the `HarnessError` arm from `as_diagnostic` — **the motivating incident, restaged** | FAILED: "`["HarnessError"]` implement `Diagnostic` but `as_diagnostic` cannot see them" |
| `detail_projections_write_only_declared_keys` | `base["retry_after"] = …` (catalog declares `retry_after_ms`) | FAILED (static) |
| `every_diagnostic_in_the_corpus_projects_its_catalog_key_set` | same fault | FAILED (runtime), so both halves are live |
| `from_code_projects_…_for_every_registered_code` | `from_code` reverted to `detail: Value::Null` | FAILED: "`auth.invalid` projects a top-level non-object detail: Null" |
| `every_allowlist_entry_still_matches_a_live_site` | an entry naming a nonexistent symbol | FAILED as stale |

`just lint` green. `just test` green: 5,568 passed across the area (one known
spurious nextest `LKFAIL` on `argv_normalization`, which retried and passed —
pre-existing, see the leak-timeout note in the testing skill).

---

## Phase 7 — L2 Real-CLI Rendering

Uses the package's existing L2 process harness; target-gated or portable across
macOS, Windows, and Linux. Cases are **mutually parallelizable**.

All cases land in `cli/tests/level2_typed_error_render_capture.rs` (tmux, `#![cfg(unix)]`).

- [x] **[L2]** Lifecycle proxy resolution from `initialize` (the motivating case).
- [x] **[L2]** Proxy resolution from a terminal/recovery event.
      *This route was **broken** and Phase 7 found it: it reached the pane as a
      generic `Error: failed to load Markdown: …`. See §D-13 — a live instance of
      D-7's `Box` un-downcastability, entering via `Report::from(Box<…>)` rather
      than a `#[source]` field. Fixed at the site; proven to fail on revert.*
- [x] **[L2]** Composition source lookup.
      *Its contract is genuinely terminal-dependent — piped reports autocomplete
      *unavailable*; a real TTY runs autocomplete and reports **no matches**. The
      TTY path had no coverage at all before this.*
- [x] **[L2]** Schema / file-reference failure.
      *Schema half here; the file-reference half keeps its existing dedicated
      suite (`level2_invalid_file_reference_capture.rs`). The fixture supplies a
      **wrong-typed present** value, not a required-missing one: missing values
      open the biscuit-tui prompt loop on a real TTY, which hangs the capture
      instead of rendering.*
- [x] **[L2]** Darkmatter transclusion failure.
      *The D-3 hazard detector: asserts Darkmatter's own path/hint survive the
      `Semantic` Claudine wrapper rather than degrading to a flat `Display` line.*
- [x] **[L2]** Harness pre-flight failure.
- [x] **[L2]** A deliberately unstructured fallback error — this one **must**
      still hit the generic path; that path stays valid for truly unstructured
      errors.
      *Uses argument-shape rejection per §D-10, **not** `--timeout`.*
- [x] Each typed case asserts a rendered `StatusBlock`, **never** the generic
      `Error:` line.
- [x] Cover TTY, `NO_COLOR`, `FORCE_COLOR`, and piped-stderr variants **where
      their output contracts differ**. Assert plain/piped output carries the same
      information with no ANSI/OSC 8 bytes.
      *Styled-TTY and `NO_COLOR`-TTY are asserted as a **pair** over the same
      route, so the absence assertions cannot pass vacuously: the styled case
      proves the red and the OSC 8 link are reachable, which is what makes their
      absence under `NO_COLOR` a suppression rather than a block that never
      rendered. Piped-stderr stays headless in `effective_diagnostic_render.rs`.*
- [x] **Cross-route parity**: the two proxy routes assert **identical** code,
      headline, hint, and available typed resolution detail. Assert
      event/property context **separately**, so intentional route-specific detail
      is not mistaken for drift.
      *⚠️ **Not satisfiable as written** — the D-2 exit condition has fired. See
      §D-12. Both routes are now typed and both render a block, but they fail at
      different stages against different resolvers, so code/headline/hint cannot
      agree without a routing change D10 forbids here. Phase 7 asserts parity on
      what typing delivers and **pins the divergence**, so the file-resolution
      feature's convergence trips the test rather than landing silently.*
- [x] Each route re-asserts its pre-migration exit code, lifecycle event order,
      and exactly-once emission count.
      *The exit code is read back **through the pane** (`; echo claudine_rc:$?`),
      not from a child handle — the point is to observe what the interactive
      surface did. The pane is resized to 200 rows first: `capture()` reads only
      the visible region, and route 2 renders a full lifecycle ahead of its
      block, so a default 40-row pane would scroll the earlier emission away and
      let `emission_count` undercount a real duplicate-emission regression into a
      pass.*
- [x] Snapshots assert **actionable content**, not `to_string()` substrings.
      Beware the known L2 trap: a broad `38;2;` match falsely hits OSC 8 links —
      assert red SGR in both semicolon and ITU colon forms.
      *Keyed on the block's specific red triple (`251;44;54`) in semicolon and
      both ITU colon forms, never a bare `38;2;` prefix.*

**⛔ Checkpoint 7** — `just test-l2` green. Run via `just` recipes, **not raw
nextest** — a raw `-p` invocation drops the `!level2_` filterset and produces
false failures.

**Checkpoint 7 met.** `just test-l2` green (85 tests). The new suite's guards
were proven against the real fault rather than assumed:

| Guard | Injected fault | Result |
|---|---|---|
| `terminal_proxy_resolution_failure_renders_a_status_block` (headless) | reverted the `Report::from(*error)` unbox | FAILED with the exact reported symptom (`Error: failed to load Markdown: …`) |
| `level2_terminal_proxy_renders_status_block_in_tmux` | same | FAILED |
| `level2_proxy_routes_…_diverge_on_identity_in_tmux` | same | FAILED |

The remaining L2 cases passed under the injection, which is correct — they
exercise routes the defect never touched, and is why the terminal-proxy route
needed its own case rather than trusting route 1's coverage.

### Three pre-existing L2 failures fixed (Phase 5's err.* blast radius)

`just test-l2` was **red on HEAD** before this phase. Phase 5's D7 migration
flipped `err.kind`/`err.variant` (deprecated aliases of `err.category`/`err.code`)
at the `from_action_failure` sites, but Checkpoint 5 gates only `just test` — and
the entire err.* blast radius lands at L2. Phase 7 owns the L2 gate, so it owns
these:

| Test | Was | Now |
|---|---|---|
| `level2_lifecycle_blocked_stack_observes_err_payload` | `LifecycleAction` / `shell_approval` | `composition` / `composition.failed` |
| `level2_lifecycle_finalize_stack_observes_err_after_blocked` | `shell_approval` | `composition.failed` |
| `level2_lifecycle_post_start_setup_failure_routes_failure_finalize_with_err` | `LifecycleAction` / `harness_attempt` | `config` / `config.invalid` |

Assertions were updated, not weakened — each still proves the payload reaches
its stack, which is what the test exists for.

**⚠️ One finding for Phase 8.** The two flips are not equal in quality:

- `harness_attempt` → `config.invalid` is an **improvement**. A malformed
  `exit_expressions` regex genuinely is a config-validation failure, and
  `when: err.category == "config"` now matches it; under the synthesized label
  no faceted clause could.
- `shell_approval` → `composition.failed` is a **loss of specificity**.
  `CompositionError::PreFlightFailed(String)` has no arm in `code()`, so it
  inherits the `_ => "composition.failed"` catch-all. An author who wrote
  `when: err.variant == "shell_approval"` loses the distinction with no faceted
  replacement — a shell denial is now indistinguishable from any other
  uncoded composition failure.

Not fixed here (Rule 3 / D10 — it is a catalog change, not a test change). Phase
8 should decide whether `PreFlightFailed` earns its own code (e.g.
`composition.shell_approval`, alongside the existing
`composition.shell_expansion`).

### Three pre-existing L2 failures NOT fixed (out of scope)

`level2_context_capture::level2_context_{default,values,side_effects}_at_140_fills_cap_in_tmux`
fail on clean HEAD, verified by stashing all working-tree changes. They assert
`claudine context` fills 140 columns *minus a 1ch right margin* (138..=139) and
observe 140 — a `biscuit-terminal` `Table` width-fill drift, upstream of claudine
and unrelated to error propagation. Left for a ruling on whether the right margin
is still the contract.

---

## Phase 8 — Documentation and Closure

- [x] Update Claudine **error-architecture documentation**: the central discovery
      seam, effective-diagnostic selection (including the role contract and
      guards), the snapshot boundary, and typed-wrapper rules.
- [x] Document the added `composition.invalid_file_reference` fields in the error
      catalog; document `fallback_dir` (and `base_dir`) as **compatibility
      projections**. Do not silently reuse a code with the wrong origin.
- [x] Update **lifecycle documentation**: the effective `err.msg`, the one-level
      `err.cause.*` projection, and the rule that a registered code always
      carries a catalog-shaped detail object.
- [x] Add the **lossy-boundary audit procedure** to the `claudine` skill and
      contributor guidance; refresh the skill if module structure moved.
- [x] **Comment-drift pass**: every symbol whose behavior changed gets its
      rustdoc/inline comments reviewed for stale rendering/propagation claims —
      notably `diagnostics/mod.rs` (its `## Wired implementations` block
      describes `from_action_failure`'s synthesized-`error_kind` path, which
      Phase 5 changes) and `error_walker.rs` (its module doc says
      "deepest typed match", which Phase 2/5 supersedes).
- [x] Document additive JSON/machine-output fields as intentional. Removing or
      renaming an existing field is out of scope.
- [x] Note in `features/2026-07-13-file-resolution/spec.md` that its typed
      transport dependency has landed and the reserved nulls are ready to fill.
      **Also record the D-12 handoff**: AC5 is confirmed unsatisfiable without
      converging the two proxy resolvers, and file-resolution is its home. The
      pinning test `level2_proxy_routes_share_a_typed_surface_but_diverge_on_identity_in_tmux`
      will fail when that convergence lands — by design; it is the prompt to
      promote the assertions to full parity.
- [x] **Decide whether `CompositionError::PreFlightFailed` earns its own code.**
      Phase 7 found it inherits the `composition.failed` catch-all, so a shell
      denial lost the `shell_approval` distinction with no faceted replacement
      (see Phase 7's finding). Compare `composition.shell_expansion`, which has
      one.
- [x] **Widen D-7's requested reachability check** (per D-13): not just
      `#[source] Box<T>` fields, but any `Box<T>` reaching `Report::from`/`.into()`
      where `T` is a registered diagnostic. Phase 7 found that instance by hand
      after every static guard passed on it.
- [ ] Move the feature to `features/_completed/2026-07-13-error-propogation/`.

**⛔ Final checkpoint** — `just test`, `just test-l2`, and `just lint` all green
in the `claudine/` package area; all nine acceptance criteria demonstrably met.

**Final checkpoint met**, with one pre-existing failure and one AC carried
forward by ruling:

| Gate | Result |
|---|---|
| `just lint` | green (includes `lint-transport` → 16 error guards, and `lint-lifecycle-doc-facets`) |
| `just test-l2` | green — 131 passed, 0 failed. The three `level2_context_*_at_140` failures Phase 7 recorded as pre-existing **now pass**; the upstream `biscuit-terminal` `Table` margin drift they tracked is resolved on this branch |
| `just test` | claudine 3485 passed, claudine-cli 1994 passed. **One pre-existing failure**: `claudine-gen::drift committed_generated_artifacts_match_phase_1_byte_baseline`, which reads `reviews/2026-07-14-module-assessment/generated-artifact-baseline.json` — the review was archived to `reviews/_completed/…` and the test's path was not updated (commit `8d7fa2414`). Untouched by this feature; `gen/` has no changes in the entire plan |

**AC5 is carried forward, not met.** Both proxy routes are now typed and both
render a `StatusBlock`, but they fail at different stages against different
resolvers, so identical code/headline/hint is unreachable without a routing
change D10 forbids here (§D-2 → §D-12). The divergence is **pinned** by
`level2_proxy_routes_share_a_typed_surface_but_diverge_on_identity_in_tmux` and
handed to `features/2026-07-13-file-resolution/`, whose spec now records the
dependency and the instruction to promote the test rather than weaken it.

### Phase 8 rulings

Two Phase 7 findings were referred here; both are ruled and implemented.

- **§D-14 — `PreFlightFailed` does not earn a code; the approval *family* does.**
  Coding a prose variant would mean parsing its own `Display` to pick a code —
  the defect the feature removes. So the failures were **typed** first:
  `ShellApprovalUnavailable { command, source_file, line, failure }` with a closed
  `ShellApprovalFailure`, plus the additive `composition.shell_approval` code
  (`CODES` 43 → 44) claimed by it and the already-typed `ShellCommandDenied`.
  `err.detail.reason` (`denied`/`blacklisted`/`no_handler`/`dry_run`) restores the
  distinction the old `shell_approval` label carried, now as a faceted value.
  `PreFlightFailed` keeps `composition.failed`, and its doc says why. Every
  `Display` string is preserved byte-for-byte.
- **§D-15 — the boxed-diagnostic guard scans type positions, not value flow.**
  D-13 asked for "any `Box<T>` reaching `Report::from`/`.into()`", which is a
  value-flow question `syn` cannot answer. Scanning the two type positions that
  *produce* such a value (`#[source]`/`#[from] Box<T>` fields and
  `Result<_, Box<T>>` returns) is decidable and strictly broader. Both halves were
  proven against injected faults and reverted:

| Guard half | Injected fault | Result |
|---|---|---|
| `source_field` | a `#[source] Box<ClaudineError>` probe field in `harness/resolve.rs` | FAILED, naming site, file, line, symbol |
| `result_error` | a `Result<(), Box<HarnessError>>` probe signature | FAILED, likewise |

The two live sites are allowlisted with reasons:
`preflight_proxy_target` (`retained` — its box is forced by
`clippy::result_large_err`; correctness rests on `Report::from(*error)` at the
call site, locked by two tests proven to fail on revert) and
`CompositionError::AtomicWriteFailed` (`error-propagation-followup` — D-7's
load-bearing instance, whose fix is a variant redesign D10 defers).

### Drift resolved

- `lint-lifecycle-doc-facets` classified **`err.msg` as a deprecated alias**. It
  is not, and never was: `err.kind`/`err.variant` mirror `err.category`/`err.code`,
  while `err.msg` mirrors nothing — and Phase 5 (§D4) made it the *effective*
  diagnostic's concise, notification-safe projection, which is the correct thing
  to put in a `say:`. The guard now allows rendering it and instead fails on
  **matching** on it (`when:`/`until:`/`while:`), which is the real hazard. Both
  rules were proven against injections.
- Four rustdoc references pointed at `features/2026-06-28-real-errors/…`, which
  moved to `_completed/` — broken links in `registry.rs`, `facets.rs`,
  `diagnostics/mod.rs`, and `lifecycle.md`. Repointed.
- `diagnostics/mod.rs` still described a "deepest-meaningful-cause walk" and
  `render/mod.rs`'s `compose_failed_code` claimed the code "follows the same
  deepest-meaningful-cause walk as rendering". Phase 2/5 replaced deepest-wins
  with role-based selection; per the repo's drift rule the code is authoritative,
  so both comments were corrected.

---

## Dependency Graph

```
Phase 1 (ratify + inventory + characterize)   ← blocks everything
   ├─→ Phase 2 (registry + selection)  ─┐
   └─→ Phase 3 (snapshot + catalog)    ─┤  2 & 3 parallel after role type lands
                                        ↓
                              Phase 4 (wrappers + transport migration)
                                        ↓
                              Phase 5 (unify render + err.*)
                                        ↓
                        ┌───────────────┴───────────────┐
                   Phase 6 (guards)              Phase 7 (L2)     ← parallel
                        └───────────────┬───────────────┘
                                        ↓
                                 Phase 8 (docs + closure)
```

## Risk Register

| Risk | Phase | Mitigation |
|---|---|---|
| Selection change (`deepest` → role-based) silently alters which error users see | 2, 5 | Phase 1 characterization baseline; explicit L1 tests for both roles |
| Migration perturbs exit codes / event order / emission count | 4, 5 | D10 characterization suite re-run after every task; Checkpoint 4 & 5 gates |
| Hand-authored downcast list drifts as new `Diagnostic` types land | 2, 6 | Rust-aware source-parity test (D2) — the only thing making the list exhaustive |
| `err.msg` change breaks TTS/messaging consumers | 1, 5 | Phase 1 inventories `err.msg` consumers; provider cascade precedence preserved |
| Reverse-engineering resolver detail from `Display` to fill new catalog fields | 3 | Spec-mandated: unavailable fields are `null`; file-resolution feature fills them |
| Scope creep into retry/routing policy | 4 | D10: split any routing/retry discovery into a separate spec |
| Option B preferred after all | 1 | Hard stop at Phase 1 Task 1 — Option B is a cross-area `biscuit-terminal` change needing its own spec |

## Mega-merge Phase 6 audit

Audited candidate: `df13f68dd7ad3ef22ef7e324dbdc213ed75afcd6`.
Fresh macOS L1 and package-area lint gates passed, including the diagnostic
transport/source guards. The full L2 gate is blocked by
`level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection`,
which failed deterministically before its retry comparison. Required Linux,
native Windows, and attended L3 evidence is not attached. This feature remains
active; the candidate does not satisfy mega-merge closeout.
