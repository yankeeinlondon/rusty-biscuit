---
date: 2026-06-14
agent: "${env.AGENT}"
reviewed: true
status: "ready for planning and implementation"
---

## Problem Statement

The prior performance spec (`worktree/fixes/2026-06-13-perf/spec.md`) optimized
the `wt list` pipeline and added a performance-testing contract
(`worktree/docs/performance-testing.md`). That contract documents the *intended*
Criterion bench surfaces but the benches themselves were explicitly deferred.
Meanwhile, the `bench` recipe in `worktree/justfile` is a documented no-op, and
users have no runtime diagnostic to answer "why is `wt list` slow?" without
running a dev-time test suite.

This spec closes both gaps:

1. **Dev-time measurement**: Criterion benches with a single HTML report for
   the library-owned gather surface. The broader CLI-stage Criterion coverage
   described by the prior performance contract remains intentionally deferred
   because it requires a deliberate benchmarkable boundary for CLI render
   internals.
2. **Runtime measurement**: a `--perf` flag on the `wt` CLI that emits a
   per-stage timing report to stderr after command completion, following the
   pattern established in the `claudine` CLI.

Reader's note: this spec builds on the stage decomposition introduced by the
prior optimization. The pipeline stages (`list_worktrees` gather, graph gather,
table render, graph image render, verbose render) are already isolated as
separate code paths; this spec adds measurement at both the bench and runtime
layers without changing what any stage does. The only intentional narrowing
from `worktree/docs/performance-testing.md` is Criterion scope: this spec
turns on the library-owned bench now and documents CLI-stage Criterion benches
as follow-up work, while `--perf` covers the full runtime pipeline immediately.

## Current-State Analysis

### Criterion benches

- **None exist** for the worktree package area. The `bench` recipe is a no-op
  (`worktree/justfile:118-120`).
- Eight other package areas in the monorepo already use Criterion via the
  shared `/just/devops.just` bench infrastructure (`_bench`, `_bench_save`,
  `_bench_compare`, `_bench_preflight`, `_bench_id`): `biscuit-hash`,
  `biscuit-terminal`, `biscuit-file`, `sniff`, `darkmatter`, `renderable`,
  `tree-hugger`, `claudine`.
- `biscuit-hash` is the cleanest reference: `[lib] bench = false` +
  `[[bench]] name = "hashing" harness = false` + `criterion` dev-dependency +
  a `bench` recipe that calls `just _bench biscuit-hash`.

### Runtime perf diagnostic

- **None exists.** The closest is `perf_subprocess_counts_meet_sla`
  (`worktree/cli/src/commands/list.rs:756`) and
  `perf_full_command_non_image_meets_sla`
  (`worktree/cli/tests/perf_command_sla.rs`), which are dev-time SLA gates —
  not user-facing diagnostics.
- The `claudine` CLI has a mature `--perf` implementation
  (`claudine/cli/src/perf.rs`, ~1300 lines) that emits a reconciling
  performance tree to stderr using the `MetricsTree` component from
  `biscuit-terminal`.

### Claudine `--perf` evaluation

Claudine's implementation has several strengths worth adopting and several
layers of complexity worth skipping for worktree's scale.

**Adopt:**

| Pattern | Why |
| --- | --- |
| Stderr-only output | Perf is metadata, not data. `wt list` already writes the status table and graph to stderr (stdout is reserved for the `cd:` protocol in `wt go`). Consistent. |
| Single reconciling tree (`Performance` root) | A tree whose Structural children + `unattributed` remainder = wall-clock is honest and readable. |
| `MetricsTree` component from `biscuit-terminal` | Don't reinvent rendering. The component already handles unit alignment, share-of-wall-clock column, connectors, and `NO_COLOR` degradation. |
| `BlockQuote` wrapper with colored border | Visually distinct from command output. |
| `--perf` is opt-in, off by default | No stage-timing or report-render overhead when not used; only the top-of-main timestamp is unconditional. |
| Per-stage `Instant` capture threaded from process start | One wall-clock zero point; stages measured as deltas. |
| Help text: "Emit a performance report to stderr after command completion." | Matches claudine's idiom; clear and concise. |

**Skip (over-engineering for worktree's scale):**

| Claudine feature | Why skip |
| --- | --- |
| Pre-clap `scan_perf_bootstrap` | Claudine scans raw argv before clap to time arg parsing and gate perf by subcommand kind. Worktree's arg parsing is negligible (~µs vs 700ms gather) and there is only one perf-relevant command. Just capture `Instant::now()` at the top of `main()`. |
| `PerfCommandKind` enum + per-kind gating | Claudine supports 4 command shapes (wrapper, compose, inline-compose, sequence). Worktree has one shape (`list`). |
| `NodeRole::Breakdown` | Claudine needs Breakdown because composition stages overlap (parallel capture work double-counts). Worktree's stages are strictly sequential — every child is `Structural`. |
| `debug_assert_reconciles` TR-1 walker | Claudine has a debug-only assertion that the tree reconciles at every node. For a simple sequential pipeline with only Structural children, reconciliation is trivially `Σ children + unattributed = parent`. |
| `mark_dominant_leaf` (HOT marker) | Nice touch, but adds a tree-walk pass for marginal value at this scale. Follow-up. |
| `prune_unattributed_noise` | Claudine prunes sub-1ms remainders. At worktree's scale (ms-to-seconds), noise is invisible. |
| `SequencePerfAccumulator` | No sequences in worktree. |
| Composition perf integration (`ComposePerfReport`) | Not applicable. |

### Worktree pipeline stages to measure

The `wt list` pipeline decomposes into strictly sequential stages with no
overlap. Each is a `Structural` child of the root; `unattributed` absorbs the
gap.

| Stage | Where | Owned by | Notes |
| --- | --- | --- | --- |
| pre-dispatch | arg parsing + completion check in `main.rs` / `run()` | worktree-cli | Small (~µs). |
| list gather | `list_worktrees()` (`worktree/lib/src/worktree.rs:161`) | worktree (lib) | Dominant cost on non-image terminals (~700ms in `rusty-biscuit`). |
| graph gather | `gather_extras()` → `gather_data()` (`worktree/cli/src/commands/list.rs:90-177`) | worktree-cli | Image-capable terminals only; skips entirely on non-image. |
| table render | `build_status_table()` + `table.render()` (`list.rs:296-312`) | worktree-cli | Pure CPU; uses `biscuit-terminal` Table. |
| graph image render | `MermaidDiagram::new()` + `diagram.render()` (`list.rs:71-72`) | biscuit-terminal | External package. Mermaid → SVG → PNG → terminal protocol. Include in report but label clearly. |
| verbose render | `render_verbose()` (`list.rs:213-259`) | worktree-cli | `--verbose` only. |

## Requirements

### R1 — Add Criterion benches for the library-owned gather surface

Add a Criterion bench target to `worktree/lib` that benchmarks the
library-owned gather stage (`list_worktrees`) with a single benchmark group.
The bench produces the standard Criterion HTML report at
`target/criterion/report/index.html`.

**Implementation decision — library-only bench scope for the initial setup:**

The full pipeline decomposition (gather + table render + graph image render)
spans both the library (`list_worktrees`) and the CLI (`build_status_table`,
`gather_extras`, `MermaidDiagram::render`). Although `worktree-cli` has an
auto library target because `worktree/cli/src/lib.rs` exists, the render-stage
helpers are private implementation details behind `commands::list(...)`.
Criterion benches can only access public API without weakening module
boundaries or turning render helpers into public test seams. Rather than
expanding the CLI crate's public surface just for benches, the initial
Criterion bench covers the library-owned surface only:

- `list_worktrees()` — the dominant cost center, already fully public.

The CLI-owned stages (table render, graph gather, image render) are measured
at runtime via the `--perf` flag (R3) and covered by the existing `perf_*` SLA
tests. A follow-up spec can decide whether to expose a small CLI benchmarking
API or move render-stage helpers behind a deliberate internal library boundary
for finer-grained Criterion coverage.

**Steps:**

1. In `worktree/lib/Cargo.toml`:
   - Add `bench = false` to `[lib]` to disable the default bench harness.
   - Add `criterion = { version = "0.5", features = ["html_reports"] }` to
     `[dev-dependencies]`.
   - Add `[[bench]] name = "list_status" harness = false`.

2. Create `worktree/lib/benches/list_status.rs`:
   - One benchmark group (`list_status`) with hierarchical benchmark IDs.
   - Use `iter_batched_ref` so the gather result is not cloned per iteration.
   - Benchmark IDs:
     - `list_status/warm` — `list_worktrees()` end-to-end in the ambient repo.
       Uses `Throughput::Elements(1)` so the report shows calls/sec.
   - Include a module-level doc comment stating that the bench runs in the
     ambient `rusty-biscuit` checkout and that baseline numbers drift as the
     repo grows (document the caveat; the host-state preflight and
     host-derived baseline from the shared `/just` infrastructure mitigate
     cross-host comparison issues).

3. The bench must not panic if the ambient directory is not a git repo. Check
   `list_worktrees()` once outside the benchmark closure and skip the group if
   it errors; this is more robust than checking only for a `.git` directory
   because linked worktrees often use a `.git` file and worktree metadata can
   be located outside the current directory.

### R2 — Wire the `bench` recipe through the shared `/just` bench infrastructure

Replace the no-op `bench` recipe in `worktree/justfile` with a call to the
shared `_bench` helper. Add `bench-save` and `bench-compare` recipes mirroring
the shared helpers for the canonical before/after workflow.

**Steps:**

1. Replace `worktree/justfile:118-120` (the no-op `bench`) with:

   ```just
   # Criterion benches for worktree gather performance.
   bench *args="":
       @just _bench {{ LIBRARY }} {{ args }}
   ```

2. Add `bench-save` and `bench-compare` recipes:

   ```just
   # Save the current bench run as this host's baseline (run before a change).
   bench-save *args="":
       @just _bench_save {{ LIBRARY }} {{ args }}

   # Compare the current bench run against this host's saved baseline.
   bench-compare *args="":
       @just _bench_compare {{ LIBRARY }} {{ args }}
   ```

3. The shared `_bench` recipe already runs `_bench_preflight` (battery, memory,
   load check) and `_bench_id` (host-derived baseline name), so no additional
   gating is needed in the worktree justfile.

### R3 — Add a `--perf` flag to the `wt` CLI

Add a `--perf` global flag that, when set, instruments the `wt list` pipeline
and emits a per-stage timing report to stderr after command completion. The
report uses the `MetricsTree` component from `biscuit-terminal`, wrapped in a
`BlockQuote`, following the rendering pattern established in the `claudine`
CLI (`claudine/cli/src/perf.rs`).

**CLI design (per the `cli` skill best practices):**

- **Scope**: Global flag on `Cli` (matching `--verbose` and `--width`, which
  are already global). Only `list` emits a report initially; other subcommands
  accept the flag silently (forward-compatible, no error). This is consistent
  with how `--verbose` works today (global, only honored by `list`).
- **Name**: `--perf`, no short form. Claudine has no short form either;
  `--perf` is a diagnostic, used rarely, so a short form is not warranted.
- **Help text**: "Emit a performance report to stderr after command completion."
  (Matches claudine's idiom.)
- **Output**: Stderr only. The perf report is metadata, not data. This is
  consistent with the existing `wt list` output discipline (table and graph
  already go to stderr; stdout is reserved for the `cd:` protocol).
- **Overhead**: No per-stage timing or report-render overhead when not set.
  Capturing `process_start` at the top of `main()` is a single unconditional
  `Instant::now()` call so the report can include pre-dispatch time; do not
  describe the entire non-perf path as literally zero-overhead.
- **Error behavior**: `wt list --perf` emits the report only after a successful
  `list` run. If the command returns an error, the existing error renderer owns
  stderr and no partial perf report is emitted. This keeps failures uncluttered
  and avoids adding RAII/error-path plumbing in the initial implementation.

**Implementation steps:**

1. In `worktree/cli/src/args.rs`, add the flag to `Cli`:

   ```rust
   /// Emit a performance report to stderr after command completion.
   #[arg(long, global = true)]
   pub perf: bool,
   ```

2. Create `worktree/cli/src/perf.rs`:

   - A `PerfCollector` struct that captures per-stage `Duration`s and the
     process-start `Instant`. Simpler than claudine's `CommandPerfCollector`
     because there are no substages, no composition perf, and no agent perf.

     ```rust
     pub(crate) struct PerfCollector {
         process_start: Instant,
         stages: Vec<(&'static str, Duration)>,
     }
     ```

   - `PerfCollector::new(process_start)` — start with an empty stage list.
   - `PerfCollector::record(&mut self, name: &'static str, elapsed: Duration)`
     — append a stage timing.
   - `PerfCollector::emit(&self)` — build the tree and render to stderr.

   - A `PerfNode` type (simplified from claudine — no `NodeRole` enum needed
     since all children are Structural):

     ```rust
     struct PerfNode {
         label: String,
         total: Duration,
         children: Vec<PerfNode>,
     }
     ```

   - A `build_perf_tree` function that assembles the tree:
     - Root: `Performance` with `total = process_start.elapsed()`.
     - Children: one `Structural` node per recorded stage.
     - Append `unattributed` = `max(0, total - Σ stages)`.

   - A `render_perf_report` function that projects `PerfNode` into
     `biscuit_terminal::components::metrics_tree::MetricNode` and renders via
     `MetricsTree` inside a `BlockQuote` with a colored left border. Use the
     same `BlockQuote` + `MetricsTree` pattern as claudine's
     `render_perf_report` (`claudine/cli/src/perf.rs:1185`).

3. In `worktree/cli/src/main.rs`:
   - Capture `let process_start = std::time::Instant::now();` at the very top
     of `main()`, before anything else (before `CompleteEnv`, before `run()`).
   - Pass `process_start` and `cli.perf` down to the `list` command.
   - Add `mod perf;` so the CLI binary can use `worktree/cli/src/perf.rs`.

4. In `worktree/cli/src/commands/list.rs`:
   - Change `run()` signature to accept `perf: bool` and `process_start:
     Instant`.
   - When `perf` is true, wrap each pipeline stage in a timing capture:

     ```rust
     pub fn run(
         width_spec: Option<&str>,
         verbose: bool,
         perf: bool,
         process_start: Instant,
     ) -> Result<(), WorktreeError> {
         let mut collector = if perf {
             Some(PerfCollector::new(process_start))
         } else {
             None
         };

         record(
             &mut collector,
             "pre-dispatch",
             process_start.elapsed(),
         );

         let t0 = Instant::now();
         let list = list_worktrees()?;
         record(&mut collector, "list gather", t0.elapsed());

         // ... graph gather, table render, image render, verbose render ...

         if let Some(ref c) = collector {
             c.emit();
         }
         Ok(())
     }
     ```

   - The `pre-dispatch` stage is recorded as the elapsed time from
     `process_start` to the first line of `list::run()`. It covers completion
     environment handling, clap parsing, global-option extraction, and command
     dispatch. Because it is recorded before `list_worktrees()`, it does not
     double-count the gather stage.

   - The `emit()` call must be the last thing before `Ok(())`. `emit()` should
     sample `total = process_start.elapsed()` before rendering the
     `MetricsTree`, so the perf report's own render time is excluded from the
     measured total.

   - **Graph image render labeling**: When the graph image render stage is
     present, label it `graph image render (biscuit-terminal)` so the report
     makes clear that stage's cost is owned by an external package, not
     worktree. This matches the exclusion documented in
     `worktree/docs/performance-testing.md`.

5. When `perf` is false, the `PerfCollector` is `None`, stage timers are not
   captured, and the report is not rendered. The only unconditional cost is
   the top-of-main `Instant::now()` required to make `pre-dispatch` honest when
   perf is enabled.

**Non-image terminal behavior**: On a non-image terminal, the graph gather and
graph image render stages are skipped entirely (per the prior optimization's
R1 gating). The `--perf` report simply shows fewer stages — it honestly
reflects what ran. No special handling needed.

### R4 — Update `worktree/README.md` with perf workflow documentation

Add a `## Performance` section to the README documenting:

1. The `--perf` flag: what it shows, where it writes (stderr), and that it is
   available on `wt list`.
2. The `just bench` / `just bench-save` / `just bench-compare` workflow for
   dev-time Criterion measurement.
3. A cross-link to `worktree/docs/performance-testing.md` for the full
   performance contract.

**Steps:**

1. Add a `## Performance` H2 section after the existing `## Tech Stack`
   section. Keep it concise — the README is user-facing, not a deep-dive.

2. Content sketch:

   ~~~markdown
   ## Performance

   `wt list` is optimized for the `rusty-biscuit` monorepo (48 workspace
   members). The pipeline resolves the default branch once, gathers per-worktree
   status in parallel, and skips the graph path entirely on non-image terminals.

   ### Runtime diagnostic

   `wt list --perf` emits a per-stage timing report to stderr showing where
   wall-clock time is spent (pre-dispatch, list gather, graph gather, table
   render, image render). The report uses a reconciling tree so stage timings
   plus unattributed time sum to the total wall-clock.

   ### Dev-time benchmarks

   `just bench` runs Criterion benches against the ambient checkout and writes
   an HTML report to `target/criterion/report/index.html`. For before/after
   comparison:

   ```sh
   just bench-save          # before a change
   # ... make your change ...
   just bench-compare       # criterion renders +/- regressions
   ```

   See `docs/performance-testing.md` for the full performance contract,
   including which surfaces are benchmarked and which are excluded.
   ~~~

3. Do not add benchmark numbers to the README — they drift. The README links
   to the perf docs and tells users how to run the benches themselves.

### R5 — Update `worktree/docs/performance-testing.md`

Update the `## Bench Recipe` section (currently lines 40-42) to reflect that
Criterion benches now exist and the `bench` recipe is wired.

**Steps:**

1. Replace the `## Bench Recipe` section content:

   ~~~markdown
   ## Bench Recipe

   Criterion benches live in `worktree/lib/benches/list_status.rs` and cover
   the `list_worktrees()` gather stage. Run via the shared `/just` bench
   infrastructure:

   ```sh
   just -d worktree bench             # run benches, generate HTML report
   just -d worktree bench -- --open   # auto-open the report in a browser
   just -d worktree bench-save        # save baseline before a change
   just -d worktree bench-compare     # compare after a change
   ```

   The HTML report is at `target/criterion/report/index.html`. The shared
   `_bench_preflight` recipe gates on host state (battery, memory, load) and
   `_bench_id` derives a host-specific baseline name so results from one
   machine cannot be silently compared against another.

   ### Runtime `--perf` flag

   `wt list --perf` emits a per-stage timing report to stderr. It is a runtime
   diagnostic for users, complementing the dev-time Criterion benches. The
   report covers all pipeline stages that actually ran (pre-dispatch, list
   gather, graph gather, table render, graph image render, verbose render) even
   when Criterion only covers the library-owned gather stage.
   ~~~

2. Update the `## Measurement Methodology` section to cross-reference the
   `--perf` flag as an additional measurement surface alongside the `perf_*`
   SLA tests.

3. Because `worktree/docs/performance-testing.md` has a `hash` frontmatter
   property, rehash it after editing with the Darkmatter CLI:

   ```sh
   md hash worktree/docs/performance-testing.md
   ```

### R6 — Add tests for the `--perf` flag

Add Level 1 tests for the `--perf` flag:

1. **Integration test** (`worktree/cli/tests/perf_flag.rs`):
   - `wt list --perf` exits 0 and writes timing output to stderr.
   - Assert stderr contains known stage labels (`"Performance"`,
     `"pre-dispatch"`, `"list gather"`, `"table render"`).
   - Assert stdout is empty (perf output is stderr-only).
   - Assert the report shape is present. Do not assert exact durations or parse
     percentage values from terminal-styled output; those vary by host, width,
     and terminal capabilities.

2. **Unit test** (`worktree/cli/src/perf.rs`):
   - `build_perf_tree` produces a tree whose `Σ Structural children +
     unattributed = root total` within a tolerance.
   - `record` appends stages in order.
   - Empty collector (no stages recorded) produces a tree with only
     `unattributed`.

3. **Non-image regression**: `wt list --perf` on a non-image terminal
   (env vars stripped) produces a report with no `graph gather` or `graph
   image render` stage, while still showing `pre-dispatch`, `list gather`, and
   `table render`.

4. **Error-path regression**: a failing `wt list --perf` invocation, such as
   running outside a git repository, reports the normal error and does not emit
   a partial `Performance` tree.

5. Existing `perf_*` SLA tests must continue to pass unchanged. The `--perf`
   flag is off by default; only the single top-of-main `Instant::now()` is
   unconditional.

## Non-Goals

- **Out of scope:** Adding a `[lib]` target to `worktree-cli` for
  finer-grained Criterion coverage of the CLI-owned render stages. The crate
  already has an auto library target via `src/lib.rs`, but exposing private
  render helpers as public benchmark API is a larger architectural decision and
  is tracked as a follow-up. The initial Criterion bench covers only the
  library-owned `list_worktrees()` gather stage.
- **Out of scope:** Matching claudine's full `PerfNode` complexity (`Breakdown`
  role, `mark_dominant_leaf` HOT marker, `prune_unattributed_noise`,
  `SequencePerfAccumulator`, composition perf integration). Worktree's
  pipeline is strictly sequential with one command shape; the simplified model
  (all Structural children + one unattributed remainder) is sufficient.
- **Out of scope:** Pre-clap argv scanning to time arg parsing separately.
  Arg parsing is negligible (~µs) vs the gather stage (~700ms). The
  `process_start: Instant` captured at the top of `main()` includes arg
  parsing in the `pre-dispatch` window, which is the honest accounting.
- **Out of scope:** Instrumenting `wt create`, `wt go`, or `wt remove` with
  `--perf`. Only `wt list` emits a report initially. The global flag is
  forward-compatible; other commands are silent no-ops.
- **Out of scope:** Changing the biscuit-terminal `MetricsTree` component.
  It already supports the needs of this spec (hierarchical tree, unit-aligned
  duration column, share-of-wall-clock percent column, `NO_COLOR` degradation).

## Acceptance Criteria

1. `just -d worktree bench` runs Criterion benches and generates an HTML report
   at `target/criterion/report/index.html`. The bench covers the
   `list_worktrees()` gather stage.
2. `just -d worktree bench-save` and `just -d worktree bench-compare` work
   end-to-end using the shared `/just` baseline infrastructure.
3. `wt list --perf` emits a per-stage timing report to stderr. The report uses
   `MetricsTree` inside a `BlockQuote` and shows at minimum:
   `Performance` (root), `pre-dispatch`, `list gather`, `table render`, and
   `unattributed`.
4. `wt list` (without `--perf`) performs no stage timing and no report
   rendering. The only unconditional perf-related work is a single
   top-of-main `Instant::now()` call.
5. On a non-image terminal, `wt list --perf` does not show `graph gather` or
   `graph image render` stages (they were skipped).
6. `wt list --perf` writes nothing to stdout (perf is stderr-only).
7. `worktree/README.md` has a `## Performance` section documenting the
   `--perf` flag and the `just bench` / `bench-save` / `bench-compare`
   workflow.
8. `worktree/docs/performance-testing.md` `## Bench Recipe` section is updated
   to reflect that Criterion benches exist and the recipe is wired.
9. All existing tests pass unchanged; new tests cover the `--perf` flag
   (integration + unit), success-only emission behavior, and the perf tree
   reconciliation.
10. The `--perf` flag is declared `global = true` on `Cli` (matching
    `--verbose`), with help text "Emit a performance report to stderr after
    command completion." and no short form.

## Open Questions

No blocking design questions remain after evaluation. The spec makes the
important decisions locally:

- Criterion initial scope is library-only (`list_worktrees`); CLI-stage
  Criterion coverage is a follow-up requiring a deliberate public or internal
  benchmarking boundary in `worktree-cli`.
- `--perf` is global on `Cli` (matching `--verbose`), only `list` emits, other
  commands are silent no-ops.
- The perf model is simplified from claudine's: all children are Structural,
  one `unattributed` remainder, no `Breakdown` role, no HOT marker.
- The process-start `Instant` is captured at the top of `main()` — no pre-clap
  argv scan. This intentionally trades one unconditional `Instant::now()` for
  accurate pre-dispatch accounting.
- Rendering uses the existing `biscuit-terminal` `MetricsTree` + `BlockQuote`
  components, stderr output, matching claudine's output discipline.
- Perf reports emit only on successful `wt list` runs; command errors keep the
  existing error-only stderr behavior.
