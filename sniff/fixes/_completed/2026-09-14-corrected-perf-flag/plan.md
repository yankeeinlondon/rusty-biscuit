---
total_phases: 4
created: 2026-09-15
phase: 4
agent: "claude/opus"
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - sniff/fixes/corrected-perf-flag/spec.md
  - sniff/fixes/corrected-perf-flag/plan.md
docs_created_during_phase_1:
  - sniff/fixes/corrected-perf-flag/baseline-before.txt
  - sniff/fixes/corrected-perf-flag/implementation-log.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/cli/src/output/perf_tree.rs
  - sniff/cli/src/output/mod.rs
docs_updated_during_phase_2:
  - sniff/fixes/corrected-perf-flag/plan.md
  - sniff/fixes/corrected-perf-flag/spec.md
  - sniff/fixes/corrected-perf-flag/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/cli/src/output/perf_tree.rs
  - sniff/cli/src/output/render.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_3:
  - sniff/fixes/corrected-perf-flag/plan.md
  - sniff/fixes/corrected-perf-flag/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_4:
  - sniff/fixes/corrected-perf-flag/plan.md
  - sniff/fixes/corrected-perf-flag/spec.md
  - sniff/fixes/corrected-perf-flag/implementation-log.md
docs_created_during_phase_4:
  - sniff/fixes/corrected-perf-flag/baseline-after.txt
skills_files_updated_during_phase_4:
  - .claude/skills/sniff/cli.md
  - .claude/skills/os/build-hosts.md
source_code:
  - sniff/cli/src/output/perf_tree.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/render.rs
  - sniff/cli/tests/cli.rs
documentation:
  - sniff/fixes/corrected-perf-flag/spec.md
  - sniff/fixes/corrected-perf-flag/plan.md
  - sniff/fixes/corrected-perf-flag/implementation-log.md
  - sniff/fixes/corrected-perf-flag/baseline-before.txt
  - sniff/fixes/corrected-perf-flag/baseline-after.txt
  - .claude/skills/sniff/cli.md
  - .claude/skills/os/build-hosts.md
completed_phase: "4"
implemented: true
packages:
  - sniff-cli
human_review: true
human_review_items:
  - >-
    UNCHANGED AND STILL OPEN — the S-1 scope ruling, the one decision this fix
    never obtained. Phase 1 reproduced the underscore/ellipsis corruption in
    `MetricsTree` and verified that the only working fix lives in
    `biscuit-terminal::build_markup` (escape the already-padded label, as
    `metrics_tree.rs:355` already does for the share cell). The plan's own
    remedy — escaping in Sniff's projection — is falsified: it makes the value
    column ragged at every width. Phase 1's full evidence, measurements, and
    three options are under "S-1 · Verdict" in the implementation log.
  - >-
    Phase 3 could not obtain that ruling (non-interactive session), so it took
    the branch the plan defines for a DECLINED ruling: `biscuit-terminal` is
    untouched, no projection-side escaping was added, and the required deferral
    comment now sits at the label-construction site in
    `perf_tree.rs::timing_node`. Phase 4 changed nothing here. This was a
    default, not a decision — the author should still rule. Consequence if it is
    never made: `sniff --perf` human output misaligns its value column and can
    inject italics on terminals narrower than 49 columns; above 48 columns it is
    clean at every width tested (20–140).
  - >-
    NEW in Phase 4, and needs a human: the Linux build host's cross-check lock
    is stale. `~/ci-verification/.cross-check.lock` on `$BUILD_LINUX` has been
    held since 2026-09-14T18:25:30Z by `{"purpose": "nightly-reward-spike",
    "owner": "reward-20260914-c3e60d0", "branch": "feat-nightly-perf"}`. Any
    `just cross-check ... --os all` or `--os linux` now blocks for 30 minutes
    and exits 75. Repo rule is that only the lock's owner removes it by hand, so
    it was left alone. Until someone clears it, native Linux has no local
    pre-push evidence path and CI's `ubuntu-latest` leg is its only proof — that
    is the one OS this fix could not exercise locally (native Windows and WSL2
    archive mode both ran green).
  - >-
    NEW in Phase 4, minor, non-blocking, and NOT fixed here: `scripts/
    cross-check.sh` reports `FAIL` for the `wsl` leg after a run whose own
    `cross-check-exit:` marker is 0 and whose every test passed. Verified on the
    guest: the archive run writes its JUnit report to
    `<clone>/target/nextest/ci/test-results.xml`, which is neither path
    `publish_wsl_receipt` searches, and the same fresh `target` makes the
    restoring `mv target.hold target` nest the warm cache at
    `target/target.hold` instead of restoring it — so every WSL cross-check also
    rebuilds from cold and leaves an orphaned cache copy on a host where disk
    matters. Out of this fix's scope (Rule 3; `scripts/` is not in the Affected
    Code table). Recorded in `.claude/skills/os/build-hosts.md` so it is not
    re-diagnosed. The exact source of the non-zero SSH exit was not pinned.
  - >-
    CARRIED FORWARD from Phase 3, minor: a pre-existing CLI test
    (`cli.rs::repo_aggregate_perf_covers_complete_command`) asserted the retired
    flat format on stderr and was rewritten to assert the same facts segment-wise
    against the tree. In scope per the spec's Affected Code table; flagged only
    because it changed an existing test rather than adding one.
message_to_agent: >-
  Phase 4 is complete and so is the plan. There is no Phase 5; the terminal
  state is "implementation complete, ready for review". Do not move this fix to
  `_completed` and do not run `just complete` — that is the author's call after
  the review cycle closes.

  What Phase 4 added: four L1 CLI contract tests in `sniff/cli/tests/cli.rs`
  (`perf_plain_output_is_ansi_free_and_hierarchical`,
  `perf_on_a_scriptable_text_command_stays_off_stdout`,
  `json_perf_stdout_is_exactly_one_document`,
  `counter_tree_reaches_the_human_report`) plus four shared helpers above them
  (`performance_section`, `metric_row`, `metric_value`, `metric_offset`). No
  production code changed in this phase.

  If you touch `--perf` rendering again, reuse those helpers rather than writing
  new string assertions. They encode the three rules that keep a CLI perf
  assertion portable: locate a row by whole whitespace cell (never substring),
  read its value as the cell AFTER the label (never the last cell — that is the
  share, which folds from an em dash to a hyphen without Unicode), and express
  hierarchy as an offset ordering rather than by naming a connector glyph.

  Gates: `cd sniff && just test` (2660 passed, 24 pre-existing skips) and
  `just lint` (exit 0, zero lints) are green, as is
  `cargo clippy -p sniff-cli --all-targets`. Native Windows and WSL2 archive-mode
  cross-checks both ran 46/46 green on the perf suite. Native Linux could NOT be
  exercised: the `$BUILD_LINUX` cross-check lock is stale since 2026-09-14 and
  only its owner may clear it (see `human_review_items`). CI's `ubuntu-latest`
  leg is that OS's proof.

  Still open and still the author's: the S-1 scope ruling from Phase 1. Nothing
  in the shipped code depends on the answer.
---

# Implementation Plan — Corrected `--perf` Rendering

## Overview

### What this work is

The fix is a **renderer-only change**, confined to the human-readable half of
`sniff --perf`. Collection, worker propagation, counter names, stage
instrumentation, and the `PerformanceReport` serialization schema are all
explicitly frozen (spec R1, Non-Goals). The single production entry point is
`sniff/cli/src/output/render.rs:92` —
`render_performance_section(&sniff::PerformanceReport) -> String` — which today
emits a flat `Total:` line plus two sorted bullet lists (`render.rs:92-128`).

That function must be replaced by a **pure projection** from
`PerformanceReport` into one or two
[`MetricsTree`](../../../biscuit-terminal/lib/src/components/metrics_tree.rs)
instances, rendered through `TerminalRenderable::render(&Terminal::default())`.
Sniff owns the report → tree projection; `biscuit-terminal` keeps ownership of
width math, connectors, unit alignment, glyph folding, color degradation, and
share display rules.

There is exactly **one** caller of the rendered text —
`CliPerf::emit` (`sniff/cli/src/perf.rs:71-82`) — which routes it to
`output::emit_text` (stdout) or `output::emit_stderr` (stderr) with an optional
`strip_escape_codes` pass for `--plain` (`sniff/cli/src/output/mod.rs:228-243`).
Those seams do not change; only the string flowing through them does.

### Shape of the data being projected

```rust
PerformanceReport {
    total_duration_ms: f64,
    stages:   BTreeMap<String, PerformanceStage>,  // calls, total/max/last ms
    counters: BTreeMap<String, u64>,
}
```

Both maps are keyed on dotted names drawn from a live, growing namespace. The
actual literals instrumented in `sniff/lib/src` today include:

| Kind | Examples |
|---|---|
| Root | `detect.total` |
| Domain stages | `detect.os`, `detect.hardware`, `detect.network`, `detect.filesystem` |
| Nested stages | `filesystem.shared_walk`, `filesystem.shared_walk.docs`, `filesystem.file_inventory.scan`, `filesystem.file_inventory.classify.extension`, `hardware.gpu`, `network.wan_ip` |
| Counters | `filesystem.io.bytes_read`, `filesystem.repo.manifest_parses`, `network.wan_ip.cache_hits`, `filesystem.file_inventory.classified_extension` |

Three structural facts matter and drive the design:

1. **A measured stage can also be a prefix of another measured stage.**
   `filesystem.shared_walk` is measured *and* parents `filesystem.shared_walk.docs`.
   Intermediate nodes are therefore sometimes measured, sometimes synthetic.
2. **Intermediate levels are frequently unmeasured.**
   `filesystem.file_inventory` and `…classify` are pure synthetic groupings.
3. **Stage and counter namespaces overlap but are never merged** — the spec
   keeps them in two separate trees (R3), which sidesteps the
   `classify.extension` (stage) vs `classified_extension` (counter) ambiguity
   entirely.

### Prior art in this monorepo

Two packages already drive `MetricsTree`, and both are informative but
**neither is reusable**:

- `worktree/cli/src/perf.rs:36-127` — closest analogue (`PerfNode` →
  `MetricNode`, share against wall-clock, zero-total guard at line 112). But it
  has flat stage names, no dotted parsing, no HOT marker, no counters, wraps in
  a `BlockQuote`, and renders via `render_optimistic` — which hardcodes
  `unicode = true` (`metrics_tree.rs:574`) and therefore **must not** be copied
  here (see Ruling R-6).
- `claudine/cli/src/perf/tree.rs` — a bespoke reconciling tree over a wholly
  different data model (`CommandPerfReport`), with `Structural`/`Breakdown`
  roles and an `unattributed` remainder. Sniff explicitly does **not**
  reconcile (spec Non-Goals), so this pattern is the wrong one to borrow.

The correct read: write a small, self-contained generic dotted-name tree
builder in Sniff, parameterized so the timing tree and the counter tree share
it.

### Risks identified during planning

Two concrete hazards that the specification does not anticipate. Both are
addressed by Phase 1 spikes rather than assumed away:

- **Underscore-as-italics.** `MetricsTree::build_markup` inserts row labels
  verbatim into Prose markup (`metrics_tree.rs:322-326`), and Prose's markdown
  pass treats `_` as an italics opener (`prose/markdown.rs:397-416`). An
  intra-word flanking rule (`markdown.rs:46-48`) protects `file_inventory`
  normally — but `truncate_to_width` (`metrics_tree.rs:554-565`) can cut a
  label immediately after a `_`, leaving it flanked by `…`, which is not a word
  character. That `_` then becomes a live opener able to pair with a later `_`
  on another row, injecting `<i>` and swallowing characters — corrupting column
  alignment. Sniff is the **first** `MetricsTree` consumer with
  underscore-heavy labels; `worktree` and `claudine` use hyphenated/spaced
  labels and have never exercised this path.
- **`Duration::from_secs_f64` panics** on NaN, infinite, or negative input.
  Every value in the report is an `f64` that must be converted.

### Definition of success

1. `sniff --perf` human output is a hierarchical, unit-aligned timing tree with
   a synthetic `Total` root, correct nesting for both detection-domain and
   generic dotted names, duration-descending sibling order, `×N` call counts,
   and exactly one deterministic `▇ HOT` row.
2. Counters render as a second, separate `Counters` tree using
   `MetricValue::Count`; the timing tree contains no counter data and the
   counter tree is omitted entirely when `report.counters` is empty.
3. `sniff/lib/src/performance.rs` and `sniff/lib/src/lib.rs` are **untouched**,
   and every existing collection-completeness test in
   `sniff/lib/tests/integration.rs` and `performance.rs` passes unmodified.
4. `--json --perf` stdout parses as exactly one JSON document carrying the
   `performance` field; the human tree goes to stderr and never contaminates it.
5. `--plain` output is ANSI-free; Unicode/ASCII glyph fallback is decided by
   `biscuit-terminal` from the detected terminal, with no hand-authored escape
   codes and no duplicated width logic in Sniff.
6. No zero-duration, empty-stage, empty-counter, or deeply-nested report can
   panic, divide by zero, or emit a non-finite share.
7. `cd sniff && just test && just lint` are both green; no
   platform-conditional (`#[cfg]`) rendering code is introduced.

---

## Phase 1 — Rulings, Risk Spikes, and Baseline

Phase 1 resolves every specification ambiguity before code is written, and
retires the two hazards above with cheap, targeted experiments. Nothing in this
phase ships production behavior.

### Necessary Rules

These are rulings on points the specification leaves under-determined or where
it conflicts with observed component behavior. They are binding for Phases 2–4.

- [x] **R-1 — The `Counters` root will render `100%`, not `—`, and that is accepted.**
  R3 asks for `MetricShare::Unknown` on *every* counter row, but
  `collect_rows` unconditionally overrides the root's share with
  `MetricShare::Full` (`metrics_tree.rs:459-463`). Honoring R3 literally would
  require changing `biscuit-terminal`, which is out of scope.
  **Ruling:** set `MetricShare::Unknown` in the projection exactly as specified;
  accept that the component prints `100%` on the `Counters` root row. Tests
  assert `Unknown` on **non-root** counter rows only. Do not "fix"
  `biscuit-terminal` to satisfy this.

- [x] **R-2 — A measured node that is also a prefix keeps its own measured value.**
  `filesystem.shared_walk` is both measured and a parent. Per R2.1.6 the
  measured `total_duration_ms` wins; children still nest beneath it and are
  *not* summed into it. Synthetic-sum semantics apply only to nodes with no
  matching stage key. The same rule resolves any collision produced by domain
  re-parenting.

- [x] **R-3 — Domain re-parenting requires an existing `detect.<domain>`.**
  R2.1.4 re-parents `<domain>.<rest>` below `detect.<domain>` only when that
  stage is present. When it is absent (focused commands), `<domain>.<rest>`
  forms an ordinary generic branch directly under the synthetic root. Never
  fabricate a synthetic `detect` node. The alias set is exactly
  `{os, hardware, network, filesystem}` and nothing else.
  *Verified in Phase 1.* The domain test is a plain `matches!` on the **first
  dotted segment**. `detect` is not a member of the alias set, so `detect.*`
  keys can never re-parent and no additional `!starts_with("detect.")` guard is
  needed — do not implement a condition that cannot fire. No live stage is
  named `detect.<domain>.<rest>`, so re-parenting collides with nothing.

- [x] **R-4 — HOT candidates are rendered, measured, non-root nodes only.**
  `detect.total` is suppressed by R2.1.2 and is therefore never a candidate.
  Synthetic intermediates are never candidates. Ties break by **full dotted
  stage name ascending** (not by display segment) so selection is total and
  deterministic. With zero eligible candidates, render the `Total` root alone
  and attach no marker.
  *Verified in Phase 1, with a caveat the plan did not anticipate.* The live
  winner on a full run is `filesystem.shared_walk.docs` — a **depth-4 leaf**
  whose 1483.78 ms accumulated total **exceeds** the 722.71 ms wall clock (4286
  calls). HOT does not land on a detection domain; no test may assume it does.
  This also confirms that shares legitimately exceed 1.0 and must not be
  clamped in the projection (`MetricShare::Of` caps display at 99%).

- [x] **R-5 — Unicode capability is locale-derived, not TTY-derived.**
  `Terminal::default().supports_unicode` resolves to
  `env_says_utf8().unwrap_or(true)` (`terminal.rs:78`) — it does **not** depend
  on whether stdout is a pipe. CLI integration tests therefore must not assert
  `├─` / `└─`, because a `LANG=C` runner legitimately yields `+- `. Glyph
  assertions belong in component/projection tests using an explicitly built
  `Terminal` (the spec's own Testing section already says this).

- [x] **R-6 — Render via `render(&Terminal::default())`, never `render_optimistic`.**
  R4 requires fallback to follow the *detected* terminal;
  `render_optimistic` hardcodes `unicode = true` (`metrics_tree.rs:568-577`).
  The `worktree/cli/src/perf.rs:43` prior art uses `render_optimistic` — do not
  copy that line.

- [x] **R-7 — Heading stays literal; the section ends with exactly one newline.**
  `## Performance` remains a plain, non-Prose-rendered heading for continuity
  (R4). `MetricsTree` trims its own trailing whitespace
  (`metrics_tree.rs:381`), and `emit_text`/`emit_stderr` use
  `print!`/`eprint!` with no newline of their own
  (`output/mod.rs:228-243`) — so the returned string must terminate in exactly
  one `\n`.

- [x] **R-8 — No `BlockQuote` wrapper and no Sniff-side width arithmetic.**
  Unlike `worktree`, pass the tree straight through. Width is whatever
  `Terminal::default()` reports; `MetricsTree` already caps the label column
  against it (`metrics_tree.rs:311-319`).

- [x] **R-9 — Node labels are the last dotted segment, not the full path.**
  Confirmed by the spec's own test wording: `filesystem.shared_walk.docs`
  "nests below `detect.filesystem` then `shared_walk` then `docs`". Hierarchy
  carries the context; full paths would blow the label budget at depth 4
  (`filesystem.file_inventory.classify.embedded_language_hint` is 52 chars
  before its 9-char connector prefix).

- [x] **R-10 — All `f64 → Duration` conversions are defensive.**
  `Duration::from_secs_f64` panics on NaN, infinity, and negative values. Every
  conversion (report total and each stage total) must pass through a single
  helper that maps non-finite or negative input to `Duration::ZERO`. A
  malformed or hand-constructed report must never abort the CLI.

- [x] **R-11 — The collector seam stays frozen.**
  Per R1, no edit to `sniff/lib/src/performance.rs` or `sniff/lib/src/lib.rs`.
  GitNexus reports 21 direct callers of `with_current_collector` and CRITICAL
  upstream impact across Sniff and Claudine. If implementation surfaces a
  genuine collection defect, **stop and re-scope** — do not repair it inside
  this fix.

- [x] **R-12 — Code location follows the spec, with one escape hatch.**
  Default to `sniff/cli/src/output/render.rs` as the Affected Code table
  directs. If that file exceeds roughly 450 lines, extract the projection to a
  private `sniff/cli/src/output/perf_tree.rs`; either way
  `render_performance_section` remains the sole public seam and
  `output/mod.rs:143` keeps its existing re-export.

### Spikes

- [x] **S-1 · Underscore/ellipsis markup spike** *(highest risk; blocks Phase 3 sign-off)*
  - Write a throwaway test that builds a `MetricsTree` from the **real** deep
    Sniff stage set — including `filesystem.file_inventory.classify.embedded_language_hint`,
    `filesystem.shared_walk.docs`, `network.wan_ip`, `network.local_interfaces` —
    and renders at widths 40, 60, 80, and 120.
  - For each width, strip ANSI and verify: (a) every `_` present in an input
    label is still literally present in the output, (b) the unit column is
    aligned across all rows, (c) no stray `<i>` or italic SGR appears on a row
    that should be plain.
  - Force the truncation case deliberately — a narrow width plus a deep label
    is what makes `truncate_to_width` cut right after a `_`.
  - **VERDICT: REPRODUCED.** Swept every width 20–140 inclusive, both
    `supports_unicode` modes, both trees, using the real S-2 key set — 484
    renders. Corruption at 25 combinations, all in the band **width 27–48**;
    clean above 48. Both the italics injection and a **one-column-per-row
    value-column shift** occur, because the paired underscores are *consumed*
    as delimiters. Two orphaned underscores are needed to form a pair; a single
    orphan is inert.
  - **The contingency this plan proposed is falsified.** Projection-time
    `\_` escaping is NOT width-neutral. The analogy to `metrics_tree.rs:355`
    misleads: there the escape is applied *after* padding, whereas a label
    escaped in the projection is measured (`metrics_tree.rs:299`) and padded
    (`metrics_tree.rs:324`) with the backslash counted as visible, and Prose
    then deletes it (`tokens.rs:383-385`). Re-running the same sweep with
    projection-time escaping: italics gone, but the value column is ragged at
    **every** width 28–140 — strictly worse. **Do not implement it.**
  - **The verified fix is component-side**, and evidence has now been recorded
    as this step required. Escaping the *already-padded* label inside
    `build_markup` — what line 355 already does for the share cell — yields
    zero italics and zero ragged columns at all 121 widths, in both Unicode
    modes, for both trees, with Sniff labels left untouched; all 17 existing
    `metrics_tree` tests stay green. The prototype was reverted. This widens
    scope into `biscuit-terminal` and therefore needs a ruling before Phase 3 —
    see `human_review_items` and the implementation log.

- [x] **S-2 · Baseline capture**
  - Run `cargo run -p sniff-cli -- --perf` and
    `cargo run -p sniff-cli -- --json --perf` on this host; save raw stdout and
    stderr to `sniff/fixes/corrected-perf-flag/baseline-before.txt`.
  - Record: the full set of stage keys actually emitted, the full set of
    counter keys, the maximum dotted depth, the longest single segment, and the
    report's `total_duration_ms`.
  - This is the fixture source for Phase 2's unit tests and the before/after
    evidence for Phase 4. It also confirms whether any live stage name violates
    the assumptions behind R-2 and R-3.

- [x] **S-3 · Count/Unknown column sanity** *(fold into the S-1 harness)*
  - Render a `Count`-valued tree where every non-root share is
    `MetricShare::Unknown`, and confirm the share column degrades cleanly
    (em dash under Unicode, `-` under ASCII, `metrics_tree.rs:98`) and that
    `share_w` computation does not collapse the layout.
  - Confirms R-1's accepted root-`100%` behavior empirically rather than by
    reading.

### Tasks

**Work-Group 1.A** — fully concurrent; no shared files.

- [x] **Record rulings** — Confirm each R-1 … R-12 above against the code
  references cited. Any ruling that fails verification is corrected in this
  document before Phase 2 opens.
- [x] **Run S-1** — Underscore/ellipsis spike, as specified above.
- [x] **Run S-2** — Baseline capture, as specified above.

### Validation Checkpoint

- [x] All twelve rulings verified against the cited `file:line` references.
- [x] S-1 has a written verdict (reproduced / not reproduced) with the widths
  tested; if reproduced, the escaping approach is decided and scoped to Sniff.
- [x] `baseline-before.txt` exists and the live stage/counter key sets are
  recorded.
- [x] No production file has been modified yet.

---

## Phase 2 — Pure Timing-Tree Projection

Build the report → `MetricNode` projection as a pure, testable function with no
I/O and no terminal dependency. The rendering wiring is deliberately deferred to
Phase 3 so this phase can be validated purely by unit tests.

### Tasks

**Work-Group 2.A** — sequential; each task builds on the previous.

- [x] **Duration helper**
  - A private `fn ms_to_duration(ms: f64) -> Duration` that returns
    `Duration::ZERO` for non-finite or negative input and
    `Duration::from_secs_f64(ms / 1000.0)` otherwise (R-10).
  - Unit test: `NaN`, `f64::INFINITY`, `-1.0`, `0.0`, and `1234.56` inputs.

- [x] **Generic dotted-name builder**
  - An intermediate tree type — e.g.
    `struct PathNode { segment: String, measured: Option<M>, children: BTreeMap<String, PathNode> }` —
    generic (or duplicated minimally) over the measured payload so both the
    timing tree (`PerformanceStage`) and the counter tree (`u64`) can use it.
  - `insert(path: &str, payload: M)` splits on `.` and creates missing
    intermediates as unmeasured.
  - A measured node that later gains children keeps its payload (R-2).
  - Empty segments and a name with no `.` are both handled without panic.

- [x] **Detection-domain re-parenting**
  - Before insertion, rewrite any stage key `<domain>.<rest>` to
    `detect.<domain>.<rest>` **only when** `detect.<domain>` exists in
    `report.stages` and `<domain>` is one of `os`, `hardware`, `network`,
    `filesystem` (R-3).
  - Skip `detect.total` entirely — it is never inserted (R2.1.2, R-4).
  - All other keys, including `detect.*` keys themselves, insert verbatim.

- [x] **Value and share resolution**
  - Measured node value = `ms_to_duration(stage.total_duration_ms)` (R-2).
  - Synthetic node value = sum of its **immediate** children's resolved values.
  - Share of every non-root node = `node_duration / wall_clock`, where
    `wall_clock = ms_to_duration(report.total_duration_ms)`.
  - When `wall_clock.is_zero()`, every non-root share is
    `MetricShare::Unknown` — guarding the division (R2.2), mirroring
    `worktree/cli/src/perf.rs:112`.
  - Shares are **not** clamped to ≤ 1.0 in the projection; overlap is real and
    `MetricShare::Of` already caps display at 99% (`metrics_tree.rs:95`).

- [x] **Calls, sorting, and the synthetic root**
  - `.with_calls(n as usize)` on measured stages where `calls > 1`; absent
    otherwise (matching `metrics_tree.rs:362`, which already hides `calls == 1`).
  - Sort siblings at every level by resolved duration **descending**, then by
    display label **ascending**.
  - Root = `MetricNode::branch("Total", Duration(wall_clock), MetricShare::Full, children).emphasized()`.

- [x] **HOT selection**
  - Walk only rendered, measured, non-root nodes; pick maximum
    `total_duration_ms`; break ties by full dotted stage name ascending (R-4).
  - Apply `MetricMarker::Highlight` to exactly that node.
  - Zero candidates ⇒ no marker, root only.
  - Assert in test that at most one `Highlight` exists anywhere in the tree.

- [x] **Overlap note**
  - Attach via `MetricsTree::with_notes` (`metrics_tree.rs:275`) the exact text:
    "Concurrent, nested, and repeated stages may overlap; their durations do
    not sum to wall-clock time."

**Work-Group 2.B** — concurrent with 2.A once the projection signature is
frozen (after the first task above). Tests assert on the returned `MetricNode`
structure, not on rendered text, so they are independent of Phase 3.

- [x] **Hierarchy tests**
  - `detect.total` produces no child of `Total`.
  - `filesystem.shared_walk.docs` lands at `Total → detect.filesystem →
    shared_walk → docs` when `detect.filesystem` is present.
  - The same key with `detect.filesystem` **absent** lands at
    `Total → filesystem → shared_walk → docs` with no synthetic `detect` node
    (R-3).
  - A non-detection name such as `repo.scan.manifests` forms an equivalent
    generic hierarchy.
  - A single-segment stage name becomes a direct child of `Total`.

- [x] **Value and ordering tests**
  - A measured parent (`filesystem.shared_walk`) that also has children retains
    its own measured duration and is not overwritten by the child sum (R-2).
  - A synthetic parent (`filesystem.file_inventory`) equals the sum of its
    immediate children only.
  - Siblings ordered duration-descending; an equal-duration pair orders by
    label ascending.

- [x] **Marker, calls, and edge-case tests**
  - Maximum measured non-root stage is the sole HOT node.
  - A deliberate equal-maximum tie resolves to the lexicographically smaller
    full stage name, asserted explicitly.
  - An empty `stages` map yields root-only with no marker.
  - `calls > 1` sets `calls`; `calls == 1` leaves it `None`.
  - `total_duration_ms == 0.0` yields `MetricShare::Unknown` on all children
    and no `NaN`/`inf` anywhere — assert `is_finite()` across every
    `MetricShare::Of` in the tree.
  - A report built from the S-2 baseline key set projects without panic.

### Validation Checkpoint

- [x] `cargo test -p sniff-cli render::tests` green. *Filter corrected during
  Phase 2:* R-12's escape hatch was taken, so the projection and its tests live
  in `output::perf_tree`, not `output::render`. Run
  `cargo test -p sniff-cli --lib output::perf_tree` (20 passed) alongside the
  unchanged `output::render` (6 passed).
- [x] `sniff/lib/` remains unmodified — verify with `git diff --stat sniff/lib`.
- [x] No test asserts on rendered glyphs yet (that is Phase 3).

---

## Phase 3 — Counter Tree and Renderer Integration

Wire both trees into `render_performance_section` and retire the flat lists.

### Tasks

**Work-Group 3.A** — the counter projection; independent of 3.B.

- [x] **Counter tree projection**
  - Reuse the Phase 2 dotted-name builder with a `u64` payload — no
    detection-domain aliasing, no `detect.total` suppression, no HOT marker.
  - Measured leaves use `MetricValue::Count(n)`; synthetic intermediates use
    the sum of their immediate children.
  - `MetricShare::Unknown` on every node including the root, per R3 — with
    R-1's accepted rendering consequence.
  - Root = `MetricNode::branch("Counters", Count(sum), Unknown, …).emphasized()`.
  - Siblings ordered by count descending, then label ascending.

- [x] **Counter tests**
  - Counts render as counts with no unit suffix (`metrics_tree.rs:60`).
  - Non-root shares are `Unknown`; the root's `100%` is asserted as the
    *expected* component behavior with a comment pointing at R-1 and
    `metrics_tree.rs:459-463`, so a future reader does not file it as a bug.
  - Nested counter names (`filesystem.io.bytes_read`,
    `network.wan_ip.cache_hits`) nest generically.
  - An empty counter map produces no tree at all.

**Work-Group 3.B** — the render seam; independent of 3.A until the final wiring
task.

- [x] **Rewrite `render_performance_section`**
  - Keep the signature `pub fn render_performance_section(&sniff::PerformanceReport) -> String`
    and the `output/mod.rs:143` re-export unchanged.
  - Emit `\n## Performance\n\n` verbatim (R-7), then the timing tree rendered
    via `MetricsTree::new(root).with_notes(…).render(&Terminal::default())`
    (R-6, R-8).
  - Append the counter tree — separated by one blank line — only when
    `report.counters` is non-empty.
  - Normalize the tail to exactly one `\n` (R-7).
  - Delete the flat stage and counter bullet blocks (`render.rs:97-125`)
    entirely; leave no dead helper behind.

- [x] **Apply the S-1 verdict** — *resolved by the task's own declined branch;
  the ruling is still open.* No human ruling was available (non-interactive
  session), so Phase 3 took the only branch that does not widen scope without
  one: `biscuit-terminal` is untouched, no projection-side escaping was added,
  and the required comment now sits at the label-construction site in
  `perf_tree.rs::timing_node`. The `git diff --stat sniff/lib biscuit-terminal`
  checkpoints below therefore stand as written and pass. The ruling remains open
  for the author; if it later permits the component fix, land it as a separate
  commit with a width-36 regression test and delete the deferral comment.
  Original task text follows. **BLOCKED pending a scope ruling; see the
  Phase 1 `human_review_items`.** S-1 reproduced, and the Sniff-side escaping
  this task assumed is falsified: it makes the value column ragged at every
  width. Do not apply it. The verified fix lives in
  `biscuit-terminal`'s `build_markup` (escape the already-padded label). If the
  ruling permits that change, land it as a separate commit with its own
  regression test at width 36 using the minimal repro recorded in the
  implementation log, and amend the two checkpoints below that currently
  require `git diff --stat biscuit-terminal` to be empty. If the ruling
  declines it, add a comment at the label-construction site recording that
  underscore flanking was checked, that corruption is confined to terminals
  narrower than 49 columns, and that the fix was deliberately deferred.

- [x] **Glyph-fallback projection test**
  - Build an explicit non-Unicode terminal
    (`Terminal::builder().supports_unicode(false).width(80).build()`, per
    `metrics_tree.rs:886-919`) and assert ASCII connectors `+- ` and the
    `# HOT` marker fallback.
  - This is a component/projection test and deliberately makes **no** claim
    about what `--plain` does (R-5, and the spec's own Testing note).

- [x] **Comment and doc pass**
  - Update `render_performance_section`'s `///` docs to describe the two-tree
    output and the ownership split (Sniff projects, `biscuit-terminal` renders).
  - Per repo policy, no format-string or glyph narration in prose, and no
    `## Arguments` block duplicating the parameter type.

### Validation Checkpoint

- [x] `cargo test -p sniff-cli` green.
- [x] Manual `cargo run -p sniff-cli -- --perf` renders a readable tree at the
  current terminal width; visually compare against `baseline-before.txt`.
- [x] `git diff --stat sniff/lib biscuit-terminal` is empty — the fix stayed
  inside `sniff/cli`.

---

## Phase 4 — CLI Contract Tests, Verification, and Drift

Pin the end-to-end contracts the spec names, then run the package-area gates.

### Tasks

**Work-Group 4.A** — four independent tests in `sniff/cli/tests/cli.rs`;
write concurrently. Existing neighbors to model on:
`test_repo_package_areas_json_perf_stdout_is_valid_json` (`cli.rs:6516`) and
`repo_aggregate_perf_covers_complete_command` (`cli.rs:347`).

- [x] **`--perf --plain` is ANSI-free and hierarchical**
  - Assert the output contains no `\x1b`.
  - Assert `## Performance` and `Total` are present, plus at least one nested
    row identified by a **label** (e.g. `shared_walk`), never by a connector
    glyph (R-5).

- [x] **Scriptable text command splits streams**
  - A scriptable text command with `--perf` keeps its data on stdout and the
    whole `## Performance` block on stderr.
  - Assert stdout contains neither `## Performance` nor `Counters`.

- [x] **`--json --perf` stdout is exactly one JSON document**
  - Parse stdout with `serde_json` and assert it consumed the entire input
    (one value, no trailing content) and that `performance` is present.
  - Assert stderr carries the human tree, confirming `emit_for_json`'s stderr
    routing (`perf.rs:54-56`) still holds.

- [x] **Counter-tree end-to-end presence**
  - On a full detection run with `--perf`, assert the `Counters` root appears
    in the human output, and that at least one known counter segment
    (e.g. `bytes_read`) is present.

**Work-Group 4.B** — concurrent with 4.A.

- [x] **Collection-contract regression check**
  - Run the existing collector tests in `sniff/lib/tests/integration.rs` and
    `sniff/lib/src/performance.rs` **unmodified** and confirm green (spec
    Success Criterion 4).
  - If any is red, that is the R-11 stop condition: halt and re-scope rather
    than touching `with_current_collector`.

- [x] **Cross-platform assurance**
  - Confirm the diff contains no `#[cfg(windows)]`, `#[cfg(unix)]`, or any
    other platform-conditional code — the projection is arithmetic and string
    handling only.
  - Confirm no test asserts a path separator, a locale-dependent glyph, or a
    terminal width, so the macOS / Linux / native-Windows / WSL2 CI legs all
    see identical expectations. Load the `os` skill before asserting anything
    stronger about a specific OS.

**Work-Group 4.C** — sequential; runs after 4.A and 4.B are complete.

- [x] **Package-area gates**
  ```bash
  cd sniff
  just test
  just lint
  ```
  Both must be green. Fix findings in place; do not suppress lints.

- [x] **After-evidence and drift**
  - Capture `--perf` and `--json --perf` output to
    `sniff/fixes/corrected-perf-flag/baseline-after.txt`.
  - Update `.claude/skills/sniff/` if it documents the `--perf` output shape.
  - Update `sniff/README.md` only if it shows example `--perf` output.
  - No `docs/dependencies.md` change is needed — `sniff-cli` already depends on
    `biscuit-terminal` and `components::metrics_tree` is already public.

- [x] **Pre-commit graph check**
  - Run `detect_changes({scope: "all"})` (or the CLI fallback) per repo policy
    before committing; `partial`/`truncated` is not a clean result — re-run.

### Validation Checkpoint — Definition of Done

- [x] All seven Definition-of-Success items in the Overview are demonstrably met.
- [x] `just test` and `just lint` green in `sniff/`.
- [x] `git diff --stat` touches only `sniff/cli/src/output/`,
  `sniff/cli/tests/cli.rs`, and this fix directory. *Phase 4 addition:*
  `.claude/skills/sniff/cli.md`, which this phase's own drift task authorizes.
  Unrelated pre-existing working-tree changes carried in from before Phase 1
  are itemized in the implementation log.
- [x] Before/after evidence saved and visually compared.
- [x] Terminal state: **implementation complete, ready for review.** The agent
  does not move this fix to `_completed` and does not run `just complete` —
  that is the author's call after the review cycle closes.
