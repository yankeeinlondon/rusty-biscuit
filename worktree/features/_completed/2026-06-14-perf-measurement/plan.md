---
agent: "open_code/zai-coding-plan/glm-5.2"
phases: 4
created: 2026-06-14
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - worktree/lib/Cargo.toml
  - worktree/lib/benches/list_status.rs
  - worktree/justfile
  - just/devops.just
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - worktree/cli/src/args.rs
  - worktree/cli/src/perf.rs
  - worktree/cli/src/main.rs
  - worktree/cli/src/lib.rs
  - worktree/cli/src/commands/list.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .opencode/skill/worktree/SKILL.md
  - .claude/skills/worktree/SKILL.md
source_files_during_phase_3:
  - worktree/cli/src/perf.rs
  - worktree/cli/tests/perf_flag.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - worktree/README.md
  - worktree/docs/performance-testing.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
  - worktree/lib/Cargo.toml
  - worktree/lib/benches/list_status.rs
  - worktree/justfile
  - just/devops.just
  - worktree/cli/src/args.rs
  - worktree/cli/src/perf.rs
  - worktree/cli/src/main.rs
  - worktree/cli/src/lib.rs
  - worktree/cli/src/commands/list.rs
  - worktree/cli/tests/perf_flag.rs
documentation:
  - worktree/README.md
  - worktree/docs/performance-testing.md
packages:
  - worktree
---

# Execution Plan — Performance Measurement (`--perf` + Criterion benches)

Source spec: [`spec.md`](./spec.md) (status: `ready for planning and implementation`).

## Plan Overview

This plan delivers two independent measurement surfaces for the worktree
package area and wires them into existing shared infrastructure:

1. **Dev-time** — Criterion benches for the library-owned `list_worktrees()`
   gather stage, wired through the shared `/just` bench helpers (R1, R2).
2. **Runtime** — a `--perf` global flag on the `wt` CLI emitting a per-stage
   timing report to stderr via `biscuit-terminal`'s `MetricsTree` + `BlockQuote`
   (R3), plus tests (R6) and documentation updates (R4, R5).

**Dependency graph:**

```
Phase 1 (R1, R2) ──┐
                   ├──> Phase 4 (R4, R5)
Phase 2 (R3) ──────┤
        │          │
        └──> Phase 3 (R6)
```

- **Phase 1 and Phase 2 are parallelizable** — they touch disjoint file sets
  (`worktree/lib/*` + `worktree/justfile` vs `worktree/cli/src/*`).
- Phase 3 (tests) depends on Phase 2 (the perf module must exist).
- Phase 4 (docs) depends on Phases 1 and 2 (features must exist to document).

**Reference implementations used as templates:**
- Criterion bench: `biscuit-hash/lib/benches/hashing.rs` + `biscuit-hash/lib/Cargo.toml`
- Runtime perf report: `claudine/cli/src/perf.rs:1185` (`render_perf_report`)
- Shared bench recipes: `just/devops.just` (`_bench`, `_bench_save`, `_bench_compare`, `_bench_preflight`, `_bench_id`)

---

## Phase 1 — Criterion benches for the library-owned gather surface (R1, R2)

**Goal:** `just -d worktree bench` runs a Criterion bench against
`list_worktrees()` and writes the HTML report to
`target/criterion/report/index.html`. `bench-save` / `bench-compare` work
end-to-end via the shared baseline infrastructure.

**Files touched:** `worktree/lib/Cargo.toml`, `worktree/lib/benches/list_status.rs` (new), `worktree/justfile`.

**Parallelizable with:** Phase 2 (disjoint file set).

### Tasks

- [x] **1.1** Add Criterion bench configuration to `worktree/lib/Cargo.toml`:
  - Add `[lib]` section with `bench = false` (disables the default bench harness so `cargo bench` only runs Criterion targets — mirrors `biscuit-hash/lib/Cargo.toml:6-10`).
  - Add `criterion = { version = "0.5", features = ["html_reports"] }` to `[dev-dependencies]`.
  - Add `[[bench]] name = "list_status" harness = false`.

- [x] **1.2** Create `worktree/lib/benches/list_status.rs`:
  - Module-level doc comment stating the bench runs in the ambient `rusty-biscuit` checkout and baseline numbers drift as the repo grows; note the host-state preflight (`_bench_preflight`) and host-derived baseline (`_bench_id`) mitigate cross-host comparison.
  - One benchmark group (`list_status`) with a single benchmark ID `list_status/warm`.
  - Use `iter_batched_ref` so the gather result is not cloned per iteration.
  - Use `Throughput::Elements(1)` so the report shows calls/sec.
  - **Robustness gate:** call `list_worktrees()` once outside the benchmark closure; if it errors (ambient dir is not a git repo, linked-worktree metadata outside CWD, etc.), skip the group entirely. Do not check only for a `.git` directory — linked worktrees often use a `.git` *file*.
  - Use `criterion::{Criterion, criterion_group, criterion_main}` + `std::hint::black_box` (follow the `biscuit-hash` bench structure).

- [x] **1.3** Replace the no-op `bench` recipe in `worktree/justfile` (currently `worktree/justfile:118-120`) with a call to the shared `_bench` helper, and add `bench-save` / `bench-compare`:
  ```just
  # Criterion benches for worktree gather performance.
  bench *args="":
      @just _bench {{ LIBRARY }} {{ args }}

  # Save the current bench run as this host's baseline (run before a change).
  bench-save *args="":
      @just _bench_save {{ LIBRARY }} {{ args }}

  # Compare the current bench run against this host's saved baseline.
  bench-compare *args="":
      @just _bench_compare {{ LIBRARY }} {{ args }}
  ```
  - `LIBRARY` is already `lowercase(PACKAGE)` = `worktree` (`worktree/justfile:26`).
  - The shared `_bench` recipe already runs `_bench_preflight` (battery/memory/load) and `_bench_id` (host-derived baseline) — no additional gating needed.

### Validation Checkpoint (Phase 1)

- [x] **V1.1** `just -d worktree bench` exits 0 and prints Criterion output for `list_status/warm`.
- [x] **V1.2** `target/criterion/report/index.html` exists after the run.
- [x] **V1.3** `just -d worktree bench-save` then `just -d worktree bench-compare` round-trip without error (baseline is found).
- [x] **V1.4** Running the bench from a non-git directory skips the group cleanly (no panic).

---

## Phase 2 — Runtime `--perf` flag on the `wt` CLI (R3)

**Goal:** `wt list --perf` emits a per-stage timing report to stderr using
`MetricsTree` inside a `BlockQuote`, with a reconciling tree (stages +
`unattributed` = wall-clock). `wt list` (no flag) performs no stage timing
and no report rendering; the only unconditional cost is a single
top-of-main `Instant::now()`.

**Files touched:** `worktree/cli/src/args.rs`, `worktree/cli/src/perf.rs` (new), `worktree/cli/src/main.rs`, `worktree/cli/src/commands/list.rs`, `worktree/cli/src/commands/mod.rs` (call-site signature only).

**Parallelizable with:** Phase 1 (disjoint file set).

### Tasks

- [x] **2.1** Add the `--perf` global flag to `Cli` in `worktree/cli/src/args.rs`:
  ```rust
  /// Emit a performance report to stderr after command completion.
  #[arg(long, global = true)]
  pub perf: bool,
  ```
  - Place it alongside `--verbose` and `--width` (already global). No short form.

- [x] **2.2** Create `worktree/cli/src/perf.rs` with the simplified perf model (no `NodeRole` enum, no `Breakdown`, no HOT marker, no noise pruning — all `Structural` children):
  - `PerfCollector` struct: `process_start: Instant`, `stages: Vec<(&'static str, Duration)>`.
  - `PerfCollector::new(process_start: Instant)` — empty stage list.
  - `PerfCollector::record(&mut self, name: &'static str, elapsed: Duration)` — append.
  - `PerfCollector::emit(&self)` — sample `total = process_start.elapsed()` *before* rendering (so render time is excluded from the measured total), build the tree, render to stderr.
  - `PerfNode` struct: `label: String`, `total: Duration`, `children: Vec<PerfNode>`.
  - `build_perf_tree(&self) -> PerfNode` — root `Performance` with `total = process_start.elapsed()`; one `Structural` child per recorded stage; append `unattributed` leaf = `max(zero, total - Σ stages)`.
  - `render_perf_report(&self) -> String` — project `PerfNode` → `biscuit_terminal::components::metrics_tree::{MetricNode, MetricValue, MetricShare}` and render via `MetricsTree` inside a `BlockQuote` with a colored left border. Follow the claudine pattern at `claudine/cli/src/perf.rs:1185`: `BlockQuote::from(rendered).with_left_block_color(Color::Tailwind(Tailwind::Yellow400)).with_border("▌ ")`, rendered at `term_width` with the tree rendered at `term_width - 2` (the border is 2 columns wide).
  - All items `pub(crate)` (the module is private to the CLI binary).
  - A free `record` helper (or inline `Option::map`) so callers don't branch on `Option<PerfCollector>` verbosity at every stage.

- [x] **2.3** Capture `process_start` and thread `perf` in `worktree/cli/src/main.rs`:
  - Add `mod perf;` to the module list.
  - First line of `main()`: `let process_start = std::time::Instant::now();` — *before* `CompleteEnv`, *before* `run()`. This is the single unconditional perf-related cost.
  - Read `cli.perf` in `run()` and pass `perf` + `process_start` into `commands::list(...)`.

- [x] **2.4** Instrument `list::run()` in `worktree/cli/src/commands/list.rs`:
  - Change signature to `pub fn run(width_spec: Option<&str>, verbose: bool, perf: bool, process_start: Instant) -> Result<(), WorktreeError>`.
  - Construct `let mut collector = if perf { Some(PerfCollector::new(process_start)) } else { None };`.
  - Record `"pre-dispatch"` = `process_start.elapsed()` as the *first* stage (covers `CompleteEnv`, clap parsing, dispatch — recorded before `list_worktrees()` so it does not double-count the gather).
  - Wrap each pipeline stage in `let t0 = Instant::now(); ... record(&mut collector, "<name>", t0.elapsed());`:
    - `"list gather"` — `list_worktrees()`.
    - `"table render"` — `build_status_table()` + `table.render()`.
    - `"graph gather"` — `gather_extras()` (image-capable terminals only).
    - `"graph image render (biscuit-terminal)"` — `MermaidDiagram::new()` + `diagram.render()`. Label clearly: external package ownership (matches `performance-testing.md` exclusion note).
    - `"verbose render"` — `render_verbose()` (`--verbose` only).
  - `emit()` must be the last thing before `Ok(())`.
  - When `perf` is false, `PerfCollector` is `None`, no timers captured, no report rendered.
  - **Error behavior:** if `run()` returns `Err`, no partial perf report is emitted. The existing error renderer in `main.rs` owns stderr. (Don't add RAII/error-path plumbing in the initial implementation.)

- [x] **2.5** Update the existing inline test call site broken by the signature change:
  - `worktree/cli/src/commands/list.rs` test `run_skips_graph_git_calls_when_image_unavailable` currently calls `super::run(None, false)` (around line 520). Update to `super::run(None, false, false, std::time::Instant::now())` so the non-perf path is still exercised. Behavior is unchanged.

### Validation Checkpoint (Phase 2)

- [x] **V2.1** `cargo build -p worktree-cli` succeeds with no warnings.
- [x] **V2.2** `wt list --perf` exits 0 and writes the perf tree to stderr (visually inspect the `MetricsTree` + `BlockQuote` rendering).
- [x] **V2.3** `wt list` (no flag) output is byte-identical to pre-change behavior (no perf artifacts).
- [x] **V2.4** `wt list --perf` writes nothing to stdout (perf is stderr-only — the `cd:` protocol on stdout for `wt go` is unaffected).
- [x] **V2.5** On a non-image terminal (env vars stripped), `wt list --perf` shows no `graph gather` / `graph image render` stages — the report honestly reflects what ran.
- [x] **V2.6** `just -d worktree lint` passes (clippy clean for the new module).

---

## Phase 3 — Tests for the `--perf` flag (R6)

**Goal:** Level-1 tests cover the perf flag end-to-end, the perf-tree
reconciliation invariant, the non-image terminal behavior, and the
error-path (no partial perf report on failure). Existing `perf_*` SLA tests
pass unchanged.

**Files touched:** `worktree/cli/tests/perf_flag.rs` (new), `worktree/cli/src/perf.rs` (inline `#[cfg(test)]` module).

**Depends on:** Phase 2.

### Tasks

- [x] **3.1** Add unit tests in `worktree/cli/src/perf.rs` (`#[cfg(test)] mod tests`):
  - `build_perf_tree` produces a tree where `Σ Structural children + unattributed = root total` within a tolerance.
  - `record` appends stages in order.
  - Empty collector (no stages recorded) produces a tree with only `unattributed` (i.e., `unattributed == total`).

- [x] **3.2** Create `worktree/cli/tests/perf_flag.rs` (integration test, uses `assert_cmd` + `predicates` — already dev-dependencies):
  - `wt list --perf` exits 0.
  - stderr contains known stage labels: `"Performance"`, `"pre-dispatch"`, `"list gather"`, `"table render"`.
  - stdout is empty.
  - Do **not** assert exact durations or parse percentage values from terminal-styled output (they vary by host, width, terminal capabilities).

- [x] **3.3** Add a non-image terminal regression test (in `perf_flag.rs`):
  - Strip image-capable env vars (`TERM_PROGRAM`, `KITTY_WINDOW_ID`) before spawning.
  - Assert the report contains `pre-dispatch`, `list gather`, `table render`.
  - Assert the report does **not** contain `graph gather` or `graph image render`.

- [x] **3.4** Add an error-path regression test (in `perf_flag.rs`):
  - Run `wt list --perf` from a directory that is not a git repository.
  - Assert the command fails (non-zero exit).
  - Assert no `Performance` tree is emitted on stderr (only the normal error renderer). The existing error owns stderr.

- [x] **3.5** Confirm the existing `perf_*` SLA tests pass unchanged:
  - `perf_subprocess_counts_meet_sla` (`worktree/cli/src/commands/list.rs`).
  - `perf_full_command_non_image_meets_sla` (`worktree/cli/tests/perf_command_sla.rs`).
  - The `--perf` flag is off by default; only the single top-of-main `Instant::now()` is unconditional.

### Validation Checkpoint (Phase 3)

- [x] **V3.1** `just -d worktree test` passes (Level-1, includes the new unit + integration tests).
- [x] **V3.2** `just -d worktree test-perf` passes (the SLA tests run serially, unchanged).
- [x] **V3.3** `just -d worktree lint` passes.

---

## Phase 4 — Documentation updates (R4, R5)

**Goal:** The README documents the `--perf` flag and the
`just bench` / `bench-save` / `bench-compare` workflow. The performance
contract doc reflects that Criterion benches now exist and the recipe is
wired, and cross-references the `--perf` flag.

**Files touched:** `worktree/README.md`, `worktree/docs/performance-testing.md`.

**Depends on:** Phases 1 and 2 (features must exist to document).

**Parallelizable with:** Phase 3 (disjoint file set, after Phases 1+2 land).

### Tasks

- [x] **4.1** Add a `## Performance` H2 section to `worktree/README.md` *after* the existing `## Tech Stack` section (currently the final section). Content sketch (per spec R4):
  - `### Runtime diagnostic` — `wt list --perf` emits a per-stage timing report to stderr; reconciling tree so stages + unattributed = wall-clock.
  - `### Dev-time benchmarks` — `just bench` runs Criterion benches, HTML report at `target/criterion/report/index.html`; before/after workflow with `just bench-save` / `just bench-compare`.
  - Cross-link to `docs/performance-testing.md` for the full contract.
  - Do **not** add benchmark numbers to the README (they drift).

- [x] **4.2** Update `worktree/docs/performance-testing.md`:
  - Replace the `## Bench Recipe` section (currently lines 40-42, which state the bench recipe is a no-op / follow-up) with the wired reality: Criterion benches live in `worktree/lib/benches/list_status.rs` and cover `list_worktrees()`. Document the `just -d worktree bench` / `bench-save` / `bench-compare` workflow and the HTML report path. Note the shared `_bench_preflight` and `_bench_id` gating.
  - Add a `### Runtime --perf flag` subsection documenting `wt list --perf` as a runtime diagnostic complementing the dev-time benches; the report covers all pipeline stages that actually ran even though Criterion only covers the library-owned gather stage.
  - Update the `## Measurement Methodology` section to cross-reference `--perf` as an additional measurement surface alongside the `perf_*` SLA tests.

- [x] **4.3** Rehash `worktree/docs/performance-testing.md` after editing (it has a `hash` frontmatter property):
  ```sh
  md hash worktree/docs/performance-testing.md
  ```
  - This uses the Darkmatter CLI's Markdown-aware xxHash (frontmatter vs body segmentation), per the repo hashing convention.

### Validation Checkpoint (Phase 4)

- [x] **V4.1** `worktree/README.md` has a `## Performance` section with the `--perf` flag and the bench workflow documented.
- [x] **V4.2** `worktree/docs/performance-testing.md` `## Bench Recipe` section reflects that benches exist and the recipe is wired (no more "no-op"/"follow-up" language).
- [x] **V4.3** `worktree/docs/performance-testing.md` `hash` frontmatter is updated (verify with `git diff` that the hash value changed).
- [x] **V4.4** No benchmark numbers (durations) were added to the README.

---

## Final Acceptance Checkpoint (all phases)

Mirrors the spec's Acceptance Criteria (spec.md:503-530). Run after all four phases land:

- [ ] **A1** `just -d worktree bench` runs Criterion benches and generates `target/criterion/report/index.html` covering `list_worktrees()`. *(Phase 1)*
- [ ] **A2** `just -d worktree bench-save` and `just -d worktree bench-compare` work end-to-end via the shared baseline infrastructure. *(Phase 1)*
- [x] **A3** `wt list --perf` emits a per-stage timing report to stderr using `MetricsTree` inside a `BlockQuote`, showing at minimum: `Performance` (root), `pre-dispatch`, `list gather`, `table render`, `unattributed`. *(Phase 2)*
- [x] **A4** `wt list` (without `--perf`) performs no stage timing and no report rendering. The only unconditional perf-related work is a single top-of-main `Instant::now()`. *(Phase 2)*
- [x] **A5** On a non-image terminal, `wt list --perf` does not show `graph gather` or `graph image render` stages. *(Phase 2)*
- [x] **A6** `wt list --perf` writes nothing to stdout. *(Phase 2)*
- [x] **A7** `worktree/README.md` has a `## Performance` section documenting `--perf` and the bench workflow. *(Phase 4)*
- [x] **A8** `worktree/docs/performance-testing.md` `## Bench Recipe` section is updated. *(Phase 4)*
- [x] **A9** All existing tests pass unchanged; new tests cover `--perf` (integration + unit), success-only emission, non-image regression, error-path regression, and perf-tree reconciliation. *(Phase 3)*
- [x] **A10** The `--perf` flag is declared `global = true` on `Cli` with help text "Emit a performance report to stderr after command completion." and no short form. *(Phase 2)*

## Notes for the implementer

- **Do not run `cargo fmt`** (repo convention — `AGENTS.md`).
- **Match existing style** in each file; do not refactor adjacent code (Rule 3 — Surgical Changes).
- **Comment discipline** (per `AGENTS.md`): only add comments that carry information the code does not. Remove HOW-narration, tautological docs, and stale comments in any file you touch.
- **Rustdoc convention**: no `# H1` inside `///`; use `## H2` sections (`Examples`, `Returns`, `Errors`, `Panics`, `Notes`).
- **Non-interactive session**: do not run commands that wait on stdin/tty. Use one-shot non-interactive forms.
- The spec's Open Questions section confirms no blocking design questions remain; all decisions are made locally in the spec.
