---
created: 2026-07-16
phase: 1
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

- [ ] **[D4] Add the diagnostic role** to the `Diagnostic` trait in
      `lib/src/diagnostics/mod.rs` as **data or an object-safe method** —
      e.g. `fn role(&self) -> DiagnosticRole` returning `Semantic | Transparent`.
      It must **never** be inferred from enum names or `Display` text. Default
      the method so existing impls compile, then set roles explicitly per variant.
- [ ] **[D2] Implement the discovery seam** in `lib/src/diagnostics/` :
      `pub fn as_diagnostic(error: &(dyn Error + 'static)) -> Option<&dyn Diagnostic>`.
      It may be `#[doc(hidden)]` but **must be `pub`** — `claudine-cli` is a
      separate crate. Register every concrete Claudine `Diagnostic` type.
- [ ] **[D2] Implement effective-diagnostic selection** as one shared function
      (outer→inner walk): first `Semantic` wins and stops; if only `Transparent`
      diagnostics are found, the deepest wins. It composes with Darkmatter's
      `as_block_error` for lower-layer causes.
- [ ] **[D4] Add the traversal guards**: terminate on repeated error-object
      identity and enforce a generous max depth. Reaching either guard is logged
      and **must not** discard the best candidate already selected.
- [ ] *(parallelizable)* **[L1]** Test: every `Diagnostic` impl is discoverable
      after erasure to `dyn Error`.
- [ ] *(parallelizable)* **[L1]** Test: nested Claudine → Darkmatter →
      biscuit-file chains select the expected diagnostic under both `Semantic`
      and `Transparent` wrapper cases.
- [ ] *(parallelizable)* **[L1]** Test: a cyclic and an over-depth `source()`
      chain terminate and preserve the best pre-guard selection.

**⛔ Checkpoint 2** — `just test` green; selection function has explicit test
coverage for both roles; `as_diagnostic` is reachable from `claudine-cli`
(prove with a compile-time use, not by inspection).

---

## Phase 3 — Snapshot Boundary and Catalog Extension

Parallelizable with Phase 2 after the role type lands; both are additive.

- [ ] **[D9] Define `DiagnosticSnapshot`** (name may differ) with: schema
      version; category, code, disposition, origin, severity; catalog-shaped
      structured detail; concise notification-safe message; and the one-level
      next registered cause from D4.
- [ ] **[D9]** Facet values are **owned strings** at the snapshot boundary even
      though the in-process API uses closed enums — a newer producer must stay
      readable to an older consumer.
- [ ] **[D9]** Deserialization preserves **unknown additive** codes and detail
      fields rather than dropping or erroring on them.
- [ ] **[D3] Extend `composition.invalid_file_reference`** in
      `lib/src/diagnostics/registry.rs` **additively**. Keep `reference`, `kind`,
      `base_dir`, `suggestions`, `fallback_dir`; add `source_path`, `property`,
      `event`, `repository_root`, `candidates`, `failure`. Document `base_dir`
      and `fallback_dir` as **compatibility projections**. `failure` uses stable
      snake_case slugs (invalid syntax / missing context / no match /
      permission-I/O / unsupported remote).
- [ ] **[D3]** Project the **full object shape always**. Fields the current
      private resolver cannot supply are `null` — **never invented, never parsed
      out of `Display`**. The downstream file-resolution feature replaces those
      nulls; this feature must not reverse-engineer them.
- [ ] *(parallelizable)* **[L1]** Test: snapshot round-trips every facet,
      detail, message, and one-level cause; unknown additive code/detail values
      survive a read/write cycle.
- [ ] *(parallelizable)* **[L1]** Test: `null_detail_for` and the extended
      catalog agree — every declared field is a *present* key, absent optionals
      are `null`, and no registered code returns top-level `null`.

**⛔ Checkpoint 3** — Round-trip and catalog-parity tests green; a diff of the
catalog shows **only additions** (no removed or renamed field).

---

## Phase 4 — Semantic Wrappers and Typed Transport Migration

The first phase that changes production behavior. Run the Phase 1
characterization suite after **every** task in this phase.

- [ ] **[D3] Restructure `HarnessError::PathResolutionFailed`** in
      `lib/src/harness/error.rs` — replace `detail: String` with structured
      fields plus the typed lower-level `#[source]` where one exists. Re-check
      its classification: today it maps to `io.read_failed`, which may report an
      **authoring** failure as an environment failure.
- [ ] **[D3] Add the composition semantic wrapper** carrying `SourceContext`,
      lifecycle event, action/property path, raw authored value, typed source,
      and a contract-accurate hint. It owns
      **`composition.invalid_file_reference`** — it must **not** introduce a
      proxy-specific code. Event and property path distinguish the surface in
      structured detail only.
- [ ] **[D1] Migrate the lifecycle proxy path** (the motivating incident) to
      return the typed wrapper instead of `eyre!`. Verify the original report
      text is gone.
- [ ] **[D1] Migrate `harness_orch/loop_control.rs`** (5 sites) — typed errors
      currently converted via `to_string()`.
- [ ] **[D1] Migrate the `harness_orch/prompt.rs` pre-flight wrapper** (2 sites).
- [ ] **[D1] Migrate remaining Category-1 inventory rows.** Sequence by cluster
      to keep diffs reviewable — `cli/src/commands/wrap/mod.rs`,
      `wrapper_mcp.rs`, `wrapper_stages.rs`, `wrap/env/mod.rs`,
      `wrap/composition/pipeline.rs`, `signals.rs`,
      `lib/src/composition/lifecycle/executor.rs`. **Each cluster is
      independently parallelizable** once the wrapper types exist.
- [ ] **[D1]** Every migration uses a permitted mechanism: concrete typed error,
      `#[from]`, `#[source]` variant, or `wrap_err` **only** where the concrete
      source stays in the chain and no structured context is needed.
- [ ] **[D10]** Any routing or retry-policy change discovered mid-audit is
      **split into a separate spec** — do not fix it here.
- [ ] *(parallelizable)* **[L1]** Test: every contextual wrapper exposes its
      original concrete error through `Error::source()`.

**⛔ Checkpoint 4** — Characterization suite **byte-identical** to Phase 1
baseline (exit codes, event order, emission counts). Any drift here is a defect
in this phase, not an acceptable change.

---

## Phase 5 — Unify Rendering and `err.*` on One Selection

- [ ] **[D2/D5] Rewrite `cli/src/output/error_walker.rs`** to call Claudine's
      `as_diagnostic` + the shared selection function. **Delete the direct
      `CompositionError` downcast** — the walker must not keep a second partial
      type list. This is the fix for the motivating incident.
- [ ] **[D4] Point `LifecycleErrorInfo` construction at the same selection
      function**, so classification and rendering cannot diverge.
- [ ] **[D7] Migrate typed `from_action_failure` callers** (~11 sites, mostly
      `lifecycle/executor.rs`) to pass the selected diagnostic or a snapshot
      when they already hold typed provider/cap/timeout/runaway/harness data. A
      genuinely prose-only action failure **may remain facet-less** — but must
      not claim a registered code while projecting empty/top-level-null detail.
- [ ] **[D4] Project `err.msg`** as the selected diagnostic's concise message
      **after** the existing notification-hygiene pass — not its multiline block,
      not a classifier input. Preserve the ratified `harness::failure_message`
      precedence for provider attempt failures.
- [ ] **[D4] Add `err.cause.*`** as a strict **one-level** projection of the next
      registered diagnostic. `err.cause.cause` is **not** exposed in v1.
- [ ] **[D6] Verify frontmatter enrichment stays transparent** — it must find the
      meaningful inner diagnostic through arbitrary typed wrappers, append the
      excerpt **once**, and leave control-flow matching unchanged.
- [ ] **[D5] Confirm exactly one ordinary render boundary.** The early lifecycle
      -evaluation emission stays (catch events must run after the crash is
      visible), marked already-emitted and covered by exactly-once tests. **No
      new early render boundary** without a separate ruling.
- [ ] *(parallelizable)* **[L1]** Test: the same selected diagnostic produces the
      terminal rendering, the `LifecycleErrorInfo`, and the serialized snapshot.
- [ ] *(parallelizable)* **[L1]** Test: every `err.msg` is escape-free,
      single-line, non-empty, within the ~240-char cap; provider cascade
      precedence unchanged.

**⛔ Checkpoint 5** — Characterization suite still baseline-identical. The
motivating failure renders a `StatusBlock` when run by hand. No route classifies
one cause while rendering another.

---

## Phase 6 — Regression Enforcement

Parallelizable with Phase 7.

- [ ] **[D8] Widen and harden the lossy-boundary guard.** Extend the existing
      `scripts/check-error-transport.sh` (currently `map_err(|e| …)`-keyed and
      scoped to `lib/src/composition`) into a **Rust-aware** scan — parse with
      `syn` rather than grep — covering `lib/src`, `cli/src`, `contract/src`.
      Detect all six D8 shapes: formatted-report construction, `to_string()`
      map_err, error-bearing `reason`/`message: String` fields, pre-return
      `format!` context, log-then-return-another-error, and `Diagnostic`/
      `BlockError` impls absent from discovery.
- [ ] **[D8] Rework the allowlist** so each exception is tied to an **enclosing
      symbol** (not just a trimmed line, which currently matches by text across
      files) and carries a written reason why no typed source exists.
- [ ] **[D2] Add the Rust-aware source-parity test**: scan production sources for
      every `impl Diagnostic for …` and fail when one is missing from the
      registry. Rust has no reflection that makes a hand-authored downcast list
      exhaustive — this test is what makes D2 enforceable.
- [ ] **[D7] Add catalog-parity enforcement**: fail on a missing declared detail
      key, an undeclared ad hoc key, or a registered code whose detail is
      top-level `null`.
- [ ] Keep `just lint-transport` wired into `just lint`; update its recipe
      comment, which currently describes the narrow Phase-6 composition scope.

**⛔ Checkpoint 6** — Each guard **fails on a deliberately introduced violation**
(prove it, then revert). A guard that cannot fail is not a guard. `just lint`
green.

---

## Phase 7 — L2 Real-CLI Rendering

Uses the package's existing L2 process harness; target-gated or portable across
macOS, Windows, and Linux. Cases are **mutually parallelizable**.

- [ ] **[L2]** Lifecycle proxy resolution from `initialize` (the motivating case).
- [ ] **[L2]** Proxy resolution from a terminal/recovery event.
- [ ] **[L2]** Composition source lookup.
- [ ] **[L2]** Schema / file-reference failure.
- [ ] **[L2]** Darkmatter transclusion failure.
- [ ] **[L2]** Harness pre-flight failure.
- [ ] **[L2]** A deliberately unstructured fallback error — this one **must**
      still hit the generic path; that path stays valid for truly unstructured
      errors.
- [ ] Each typed case asserts a rendered `StatusBlock`, **never** the generic
      `Error:` line.
- [ ] Cover TTY, `NO_COLOR`, `FORCE_COLOR`, and piped-stderr variants **where
      their output contracts differ**. Assert plain/piped output carries the same
      information with no ANSI/OSC 8 bytes.
- [ ] **Cross-route parity**: the two proxy routes assert **identical** code,
      headline, hint, and available typed resolution detail. Assert
      event/property context **separately**, so intentional route-specific detail
      is not mistaken for drift.
- [ ] Each route re-asserts its pre-migration exit code, lifecycle event order,
      and exactly-once emission count.
- [ ] Snapshots assert **actionable content**, not `to_string()` substrings.
      Beware the known L2 trap: a broad `38;2;` match falsely hits OSC 8 links —
      assert red SGR in both semicolon and ITU colon forms.

**⛔ Checkpoint 7** — `just test-l2` green. Run via `just` recipes, **not raw
nextest** — a raw `-p` invocation drops the `!level2_` filterset and produces
false failures.

---

## Phase 8 — Documentation and Closure

- [ ] Update Claudine **error-architecture documentation**: the central discovery
      seam, effective-diagnostic selection (including the role contract and
      guards), the snapshot boundary, and typed-wrapper rules.
- [ ] Document the added `composition.invalid_file_reference` fields in the error
      catalog; document `fallback_dir` (and `base_dir`) as **compatibility
      projections**. Do not silently reuse a code with the wrong origin.
- [ ] Update **lifecycle documentation**: the effective `err.msg`, the one-level
      `err.cause.*` projection, and the rule that a registered code always
      carries a catalog-shaped detail object.
- [ ] Add the **lossy-boundary audit procedure** to the `claudine` skill and
      contributor guidance; refresh the skill if module structure moved.
- [ ] **Comment-drift pass**: every symbol whose behavior changed gets its
      rustdoc/inline comments reviewed for stale rendering/propagation claims —
      notably `diagnostics/mod.rs` (its `## Wired implementations` block
      describes `from_action_failure`'s synthesized-`error_kind` path, which
      Phase 5 changes) and `error_walker.rs` (its module doc says
      "deepest typed match", which Phase 2/5 supersedes).
- [ ] Document additive JSON/machine-output fields as intentional. Removing or
      renaming an existing field is out of scope.
- [ ] Note in `features/2026-07-13-file-resolution/spec.md` that its typed
      transport dependency has landed and the reserved nulls are ready to fill.
- [ ] Move the feature to `features/_completed/2026-07-13-error-propogation/`.

**⛔ Final checkpoint** — `just test`, `just test-l2`, and `just lint` all green
in the `claudine/` package area; all nine acceptance criteria demonstrably met.

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
