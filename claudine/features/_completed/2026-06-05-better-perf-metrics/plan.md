---
phases: 8
created: 2026-06-05
start_phase: 1
packages:
  - claudine
  - claudine-cli
milestones:
  A: "claudine-only (spec Phase 1) — phases 1-6; resolves RC-1..RC-4, ships standalone"
  B: "darkmatter enrichment (spec Phase 2) — phase 7; gated on OQ-1/OQ-2/OQ-3"
  C: "biscuit-terminal promotion (spec Phase 3) — phase 8; gated on OQ-4"
source_files_during_phase_1: []
docs_created_during_phase_1:
  - phase-1-inventory.md
docs_updated_during_phase_1: []
skills_files_updated_during_phase_1: []
packages_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/src/main.rs
  - claudine/cli/src/perf.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/sequence.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_2:
  - claudine-cli
source_files_during_phase_3:
  - claudine/cli/src/perf.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - claudine-cli
source_files_during_phase_4:
  - claudine/cli/src/perf.rs
  - claudine/cli/src/commands/compose.rs
  - claudine/cli/src/main.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_4:
  - claudine-cli
source_files_during_phase_5:
  - claudine/cli/src/perf.rs
  - claudine/cli/tests/wrap_commands.rs
  - claudine/cli/tests/sequence_perf.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - claudine-cli
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - claudine/docs/topics/composition.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/claudine/cli-reference.md
packages_during_phase_6:
  - claudine-cli
source_files_during_phase_7:
  - darkmatter/lib/src/markdown/compose/types.rs
  - darkmatter/lib/src/markdown/compose/perf.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/cli/src/commands.rs
  - claudine/cli/src/perf.rs
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
packages_during_phase_7:
  - claudine-cli
  - darkmatter
  - darkmatter-cli
source_files_during_phase_8:
  - biscuit-terminal/lib/src/components/metrics_tree.rs
  - biscuit-terminal/lib/src/components/mod.rs
  - biscuit-terminal/lib/src/prelude.rs
  - claudine/cli/src/perf.rs
docs_updated_during_phase_8: []
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
packages_during_phase_8:
  - biscuit-terminal
  - claudine-cli
---

# Implementation Plan — Better `--perf` metrics

Converts [`spec.md`](spec.md) into an ordered execution plan. The core success condition (spec
[Success criteria](spec.md) 1) is that the headline `Performance NNN` equals the sum of the top-level
Structural buckets — the `78.6ms`-headline-vs-`1.57s`-body class of contradiction becomes impossible,
enforced by test (TR-4). Phases 1–6 are **Milestone A** (claudine-only, spec Phase 1) and resolve every
defect in the motivating example with zero upstream dependency. Phases 7–8 are **Milestones B and C** and
must not start until their gating open questions (OQ-1…OQ-4) are decided.

Spec traceability: every task cites the spec ID it satisfies (`RC-*` root cause, `TM-*` model, `TR-*`
reconciliation, `P-*` presentation, `DM-*`/`BT-*` upstream, `OQ-*` open question).

---

## Phase 1 — Grounding and inventory (read-only)

Goal: confirm the spec's file:line anchors against the live tree and capture a real baseline capture before
any change, so regressions are visible.

- [ ] Confirm the six headline-emit sites enumerated in TM-1 / the affected-code map still match the tree:
      composition `mod.rs:848,1693,1900,2000`, wrapper `wrap/mod.rs:334`, sequence `wrap/sequence.rs:442,757`.
      Record what distinguishes the four composition sites (dry-run-unresolved vs. resolved vs. loop vs.
      structured/inline) so Phase 2 can route them through one helper.
- [ ] Confirm `StartupTimings` (`perf.rs:37`) construction sites in `main.rs` (~`263-273`, ~`301-311`) and
      that `process_start` (`main.rs:191`) is in scope at both — the value Phase 2 threads.
- [ ] Confirm the two collector types and their env-setup timers: `CommandPerfCollector::new` /
      `new_with_composition` (env timer starts at construction) and `SequencePerfAccumulator::new`; confirm
      `mark_env_setup_complete` call sites (wrapper `wrap/mod.rs:905`, composition `mod.rs:1524`).
- [ ] Confirm the substage checkpoint chain (`record_substage`, `mod.rs:804-814`; `mcp composition` zero-row
      at `:1230`) and whether it shares a clock with the env-setup window (RC-3 / TR-2 audit input).
- [ ] **P-5a audit:** walk the `compose_entry` → `execute_composition_request_inner` window in `compose.rs`
      (`:328`→`:670` direct; `:681`→`:1077` inline) and list each material, non-overlapping unit of work
      (schema validation, shell-approval preflight, file/frontmatter load, dry-run metadata prep). Mark which
      become named Structural children of `prep phase` in Phase 4 vs. which fall to `prep → unattributed`.
- [ ] Capture a real `--perf` baseline: run `claudine compose <fixture> --perf --dry-run` and a wrapper +
      sequence variant; save the raw output to `phase-1-inventory.md`. This is the before-image the Phase 6
      snapshot supersedes.
- [ ] Validation checkpoint: `phase-1-inventory.md` lists the six emit sites with their distinguishing
      context, the prep-window work units with their Phase-4 disposition, and the captured baseline output.

## Phase 2 — Wall-clock baseline + centralized emit (RC-1, TM-1, G-8)

Goal: the headline becomes true wall-clock from a single threaded baseline, computed in one place, killing
the six mid-flight timers. After this phase the headline is correct even though the body is still the old
flat layout.

- [ ] Add a baseline carrier to `StartupTimings` (`perf.rs`): a `process_start: std::time::Instant` field
      (TM-1). Populate it from `main.rs`'s `process_start` at both construction sites.
- [ ] Move wall-clock computation into the collector: have `CommandPerfCollector` and
      `SequencePerfAccumulator` hold the baseline and expose `into_report()` that computes
      `process_start.elapsed()` internally (drop the `total_elapsed: Duration` parameter, or keep it only for
      a test seam). The headline is sampled at report-build, per TM-1's invariant.
- [ ] Add one shared emit helper (e.g. `perf::emit_report(collector)` or `emit_report(report)`) that builds
      the report from the baseline and writes via `render_perf_report` to **stderr** (G-8). Route all six
      sites through it; delete `total_start` (`mod.rs:790,802`), `wrapper_start` (`wrap/mod.rs:313`), and
      `sequence_start.elapsed()` usages as the headline source.
- [ ] Keep the substage checkpoint chain's local `Instant`s (they measure sub-windows, not the headline) —
      only the *headline* timers are removed in this phase.
- [ ] Validation checkpoint: a temporary assertion / manual run shows the dry-run headline now reads ≈ real
      elapsed (`~1.6s` for the motivating fixture, P-5), not `78.6ms`. Existing perf tests still compile
      (they will be rewritten in Phase 6). `cargo build -p claudine-cli` clean.

## Phase 3 — `PerfNode` tree model + reconciliation invariant (RC-2, RC-4, TM-2, TR-1, TR-3)

Goal: replace the flat `CliOverheadReport` + separate-sections model with one reconciling tree. Presentation
is unchanged-shaped output for now (Phase 5 does glyphs); this phase is the data model and the invariant.

- [ ] Define `PerfNode { label: String, total: Duration, role: NodeRole, marker: Option<Marker>, children:
      Vec<PerfNode> }` and `enum NodeRole { Structural, Breakdown, Unattributed }` (TM-2).
- [ ] Build the tree in `into_report`: root `Performance` (wall-clock) → `pre-dispatch` (Structural; children
      `arg parsing`/`tracing init`/`config loading` as Breakdown), `prep phase` (Structural; child
      `composition` carrying `compose_perf.total`, with compose `metrics` as Breakdown leaves — RC-2 nesting),
      `environment setup` (Structural; substages — clock fix in Phase 4), `agent execution` (Structural; or a
      `—` dry-run leaf per P-5), and a synthetic `unattributed` child per reconciling node (TR-3).
- [ ] Implement DM-1: carry `ComposePerfMetric.calls` onto the Breakdown nodes for compose stages so the
      renderer can show `calls` where `> 1` (claudine-side only, no darkmatter change).
- [ ] Implement TR-3 remainder: `unattributed.total = max(0, parent.total − Σ Structural children)` for every
      reconciling node, including the root (`wall − Σ top-level Structural`).
- [ ] Implement the TR-1 walker `fn reconciles(node) -> bool` and add `debug_assert!` in `into_report` (or the
      emit helper) over the assembled tree (TR-4 runtime half). Release builds skip it.
- [ ] Port `SequencePerfAccumulator::into_report` (`perf.rs:177-`) to emit the same tree: a `steps` Structural
      node with per-step subtrees; preserve the existing `compose_perf` merge as a Breakdown subtree (TM-3).
      Honor the TM-3 decision — headline is whole-sequence wall-clock; orchestration shows as a named
      `sequence orchestration` Structural child when measured, else sequence-level `unattributed`.
- [ ] Validation checkpoint: unit tests assert the tree assembles with correct roles and that TR-1 holds at
      every reconciling node for representative wrapper / compose / dry-run / sequence shapes (full TR-4 test
      lands in Phase 6, but the walker + one reconciliation test land here to lock the invariant early).

## Phase 4 — Same-clock env-setup + prep named children (RC-3, TR-2, P-5a)

Goal: make `environment setup` sub-stages a true Structural carve of their parent, and lift material prep
work into named nodes so `prep phase` is not a catch-all.

- [ ] TR-2 (prefer option a): derive the `environment setup` window and its sub-stage checkpoints from **one**
      timer. Make `mark_env_setup_complete` the close of the same checkpoint chain that records the sub-stages
      (`mod.rs:804-814,1524`), so substages sum to the parent. If a one-timer refactor proves too invasive,
      fall back to option b — keep them on separate clocks, mark substages `Breakdown`, and let
      `environment setup → unattributed` absorb the gap. Record which option was taken in the PR note.
- [ ] Promote the substage nodes to `Structural` under option a (so they reconcile), keeping `mcp composition`
      zero-rows representable.
- [ ] P-5a: for each material prep work unit identified in Phase 1, emit a named Structural child under
      `prep phase` (candidates: `schema validation`, `shell approval`, `frontmatter load`, `dry-run prep`).
      Anything small or not cleanly separable stays in `prep → unattributed`; note any expected remainder
      above the TR-3 display threshold in the PR note.
- [ ] Validation checkpoint: reconciliation test extended so `environment setup` (option a) and `prep phase`
      reconcile within the TR-4 tolerance; the prep remainder is below threshold or explicitly documented.

## Phase 5 — Tree renderer (P-1…P-6)

Goal: render the `PerfNode` tree as the hierarchical, percentage-bearing, aligned report. Inline in
`perf.rs` (the Phase-3-of-spec extraction to biscuit-terminal is Milestone C / Phase 8).

- [ ] P-1: walk the tree generically, emitting box-drawing connectors (`├─ │ └─`) from depth; replace the
      fixed `indent: 2/4` scheme and per-section `render_section` (`perf.rs:498-`).
- [ ] P-4: compute column widths once for the whole tree; align the duration column at the unit boundary
      (mantissa right, unit suffix in a fixed column). Reuse `fmt_duration` (`perf.rs:376`) unchanged (P-6).
- [ ] P-2: render a share-of-wall-clock percent column (`97%`, `<1%` for sub-1%, root `100%`),
      right-aligned in its own fixed width.
- [ ] P-3: flag the single largest leaf across the whole tree with one `HOT`/bar/emphasis marker (OQ-4 of the
      spec leaves glyph choice to implementation); suppress when no leaf clears the materiality floor.
- [ ] DM-1 display: show `calls` on Breakdown rows where `> 1`.
- [ ] P-5: dry-run renders `agent execution` as a `—` leaf; partial sequence keeps its note; keep
      `Prose`/`BlockQuote` and the yellow `▌ ` frame (no raw escapes — G-8).
- [ ] Validation checkpoint: manual run on the Phase-1 fixtures shows the tree, percentages, HOT on
      `shell expansion`, unit-aligned columns, and a headline matching the Σ of top-level buckets.

## Phase 6 — Tests, snapshot, cross-command coverage, docs (TR-4, Success criteria)

Goal: lock behavior with tests at the right tier (per the `rust-testing` skill) and update the docs/skill
that describe `--perf` output.

- [ ] TR-4 unit test (required): build representative `CommandPerfReport`s (wrapper, compose, dry-run,
      sequence), walk the tree, assert TR-1 at every reconciling node within tolerance = max(1ms, Σ-children ×
      fmt granularity). Include the exact dry-run shape from the motivating example asserting
      `headline == Σ top-level Structural + remainder` — the `78.6ms`-vs-`1.57s` bug cannot recur.
- [ ] Rewrite the existing snapshot `render_perf_report_snapshot_locks_totals_and_alignment` (`perf.rs:1158`)
      to the tree layout, **preserving** its guarantees (microsecond rows show value; long labels keep a
      gutter; totals exclude overlapping rows; composition total mirrors `compose.total`) re-expressed against
      the nested structure. Do not delete it.
- [ ] Keep/port the existing perf unit tests (collector full report, dry-run, composition merge, sequence
      aggregation, `fmt_duration`) to the new model.
- [ ] L2 styling coverage (optional, per `feedback_claudine_cli_l2_styling_capture`): if asserting SGR/glyphs,
      use the biscuit-test-harness real-terminal path with `FORCE_COLOR=1` and semantic assertions on
      `frame.raw` (both semicolon and ITU colon SGR forms) — never byte-equality. Cover the HOT marker and the
      tree connectors. PTY/L1 prove behavior; L2 proves styling.
- [ ] Coverage across commands (Success criteria 1–4): integration assertions that `compose`,
      `inline-compose`, `sequence`, and a wrapper each emit a reconciling headline to **stderr** and leave
      stdout clean (G-8 / NG-5).
- [ ] Docs: update `claudine/docs/topics/composition.md` (or the perf surface in
      `.claude/skills/claudine/cli-reference.md`) to describe the new tree output; regenerate the skill `hash:`
      with `md hash` if a skill file changes (`feedback_skill_hash_update`).
- [ ] Validation checkpoint: `cargo test -p claudine-cli perf --color=never` green; targeted command
      integration tests green; `cargo build -p claudine-cli` clean. Milestone A complete — every motivating-
      example defect resolved.

## Phase 7 — Milestone B: darkmatter enrichment (DM-2…DM-5) — gated

Do not start until OQ-1 (nested sub-reports: recommend Option B), OQ-2 (shell command-display policy:
recommend Option B — redacted + xxHash via biscuit-hash), and OQ-3 (capture-timings attachment: recommend
Option C — by measured window) are confirmed. All changes additive; composed output byte-identical (NG-1);
gated by `perf_enabled` (NG-5).

- [ ] DM-2: tag each `ComposePerfMetric` with its `ComposePhase` (or expose a grouped view via
      `ComposeOperation::phase()`), keeping the flat `metrics` vec intact. Darkmatter unit tests for the
      grouping. Claudine renders `composition → {InlinePre,Transclusion,InlinePost,Finalization} → stages`.
- [ ] DM-3: emit per-`::shell` directive spans `{ command_display, command_hash, elapsed, cached, exit_status
      }` in the shell-expansion stage (`compose/mod.rs` + `perf.rs`/`types.rs`). `command_display` per OQ-2
      Option B (redacted, whitespace-normalized, length-capped); `command_hash` via biscuit-hash xxHash.
      Darkmatter tests for redaction of common token/URL patterns. Claudine renders the dominant shell command
      as the HOT leaf.
- [ ] DM-4: thread `ComposeContext::capture_timings` (`types.rs:1505,1312`) into `ComposePerfReport` (or
      expose on the report claudine holds); claudine attaches it per OQ-3 Option C (under `composition` if
      inside `compose_perf.total`, else a `prep phase` Structural child).
- [ ] DM-5 (Option B per OQ-1): retain optional nested child sub-reports for recursive transclusion alongside
      the flat aggregate; claudine may promote `composition` to a reconciling node. Lower priority — land last.
- [ ] Claudine consumes the richer report in `claudine/lib/src/composition/prepare.rs` (`compose_perf` plumb)
      and renders the new nodes; no claudine schema churn beyond reading richer data.
- [ ] Validation checkpoint (Success criteria 6): a slow fixture names the specific `::shell` directive and
      shows a `ctx.*` capture subtree; `dm compose --perf` benefits from the same spans; darkmatter +
      claudine tests green; `just test` for both areas (shared-crate change → broader coverage per Success
      criteria 5).

## Phase 8 — Milestone C: biscuit-terminal `MetricsTree` (BT-1) — gated

Do not start until OQ-4 (component genericity: recommend Option B — generic `Duration | Bytes | Count` value,
but Phase-1 claudine renderer stays `Duration`-only) is confirmed.

- [ ] BT-1: add `biscuit-terminal/lib/src/components/metrics_tree.rs` — a `TerminalRenderable` over a tree of
      `{ label, value, share, marker, children }` with depth-derived connectors, unit-aligned value column,
      right-aligned share column, single highlight marker; capability-aware degradation (NO_COLOR/ASCII) via
      the existing `Terminal` plumbing, consistent with `Prose`/`BlockQuote`. Register in `components/mod.rs`.
- [ ] Component-level tests in biscuit-terminal (alignment, connectors, NO_COLOR degradation, marker).
- [ ] Claudine `perf.rs` consumes `MetricsTree` and drops the inline Phase-5 renderer; behavior/snapshots
      unchanged from the consumer's perspective.
- [ ] Candidate follow-on consumers (out of scope here, noted for the crate): `dm compose --perf`, `sniff`
      group timings, `model-citizen` scan timings.
- [ ] Validation checkpoint (Success criteria 7): claudine renders via the shared component; inline renderer
      removed; biscuit-terminal carries the component's own tests; `just test` for biscuit-terminal and
      claudine green.

## Parallelization notes

- [ ] Phase 1 inventory tasks parallelize across the three areas (emit sites, collector/timers, prep-window
      audit).
- [ ] Phases 2 and 3 are sequential (3 builds on the baseline 2 threads), but within Phase 3 the
      `SequencePerfAccumulator` port can proceed in parallel with the single-shot `CommandPerfCollector` tree
      once `PerfNode`/`NodeRole` exist.
- [ ] Phase 4's TR-2 env-setup refactor and the P-5a prep-children work are independent and parallelizable.
- [ ] Phase 5 renderer can start against the Phase-3 model before Phase 4 lands (it renders whatever tree it
      is given); re-verify alignment once Phase 4 adds nodes.
- [ ] Phase 6 test authoring can begin against the Phase-3 model and tighten as Phases 4–5 land.
- [ ] Within Phase 7, DM-2 / DM-3 / DM-4 are independent darkmatter changes and parallelize; DM-5 lands last
      because it changes the reconciliation story for `composition`.

## Risk and rollback

- [ ] **Highest risk: the env-setup one-timer refactor (TR-2 option a).** It touches the hot
      `execute_composition_request_inner` checkpoint flow. Mitigation: option b fallback is pre-specified and
      keeps the report correct (Structural parent + Breakdown substages + remainder). Choosing b is not a
      failure — it is a documented, reconciling outcome.
- [ ] **Centralized emit (Phase 2) touches all six sites at once.** Mitigation: the six sites differ only in
      *when* they fire; the helper takes the collector and the threaded baseline, so behavior per site is
      identical. Phase 1 records the per-site context to prevent a missed site.
- [ ] **Snapshot churn.** The rewritten snapshot is the intended diff; the TR-4 reconciliation test is the
      real guard. Keep both.
- [ ] Milestones B and C are independently revertable — Milestone A stands alone and ships value without
      either. If an OQ decision stalls, A is the deliverable.
