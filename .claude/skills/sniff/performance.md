# Sniff Work Counters

Use this reference when measuring or optimizing Sniff. Work counters, not wall
time, are the primary evidence.

## Contents

- [Collecting evidence](#collecting-evidence)
- [Instrumentation rules](#instrumentation-rules)
- [Known baseline boundaries](#known-baseline-boundaries)
- [Interpretation traps](#interpretation-traps)

## Collecting evidence

```rust
use sniff::performance::{PerformanceCollector, with_current_collector};

let collector = PerformanceCollector::new_shared();
with_current_collector(Some(collector.clone()), || detect_repo(path))?;
let report = collector.snapshot(elapsed);
```

Stable names live in `sniff/lib/src/performance/counters.rs`. An absent counter
means zero. The `work_counts` example prints standard cases; compare only with a
compatible archived phase, OS, runner class, and request shape.

## Instrumentation rules

- Count one chokepoint per work unit.
- Gate clocks and formatted instrumentation state behind
  `performance::is_collecting()`; use `StageTimer::start`.
- Parallel walker workers own a `WorkerCollector`; activate it in callbacks and
  flush on drop.
- In `build_parallel().run(..)`, activate only after matching `Ok(entry)`.
  `ignore` (0.4.25) builds its first visitor on the *calling* thread and hands
  it root errors, such as a missing root. Activating there clears the caller's
  buffered counters, and dropping it uninstalls the caller's collector, so the
  whole request reads zero. The `nested.rs` fallback walk does this, and
  `a_missing_root_keeps_the_callers_counters` pins it.
- Rayon/spawned workers explicitly inherit or pool the collector.
- Add a collector whenever adding a new parallel execution site.
- To prove every visitor flushed, count each admitted entry from a
  `#[cfg(test)]` hook through `increment_counter`, which is thread-buffered,
  and compare the total with a serial entry count. `increment_counter_dynamic`
  writes to the collector directly, so it cannot detect a lost flush. See
  `nested.rs` `every_visitor_flushes_its_work_into_the_request`.

If a counter drops after code adds work, first suspect missing worker
propagation.

### Seeded observation accounting

`FilesystemObservation::discover` records one `GIT_DISCOVERIES` attempt at the
upward-search chokepoint, including absence and failure. Running
`detect_filesystem_with_observation` or
`detect_with_plan_and_filesystem_observation` with that seed records **zero
additional** Git discoveries. Later `detect_git` and `detect_file_changes`
calls reuse the same handle; they still record the work they actually perform,
such as `GIT_STATUS_WALKS`, blob loads, or diffs.

`FilesystemObservation::for_root` performs no Git discovery or known-path Git
open. Its bounded same-worktree validation is visible through
`FS_CANONICALIZATIONS` and `FS_METADATA_PROBES`, including the ancestor checks
that protect nested repositories. `GIT_OPENS` remains reserved for actual
known-path repository opens, not observation cloning or rebasing.

To measure acquisition and seeded execution as one request, install the same
collector around both operations. If acquisition happened before collection,
the execution report correctly shows zero `GIT_DISCOVERIES`; use the caller's
request-local accounting for the already completed acquisition rather than
inventing a second Sniff increment.

## Known baseline boundaries

- Early filesystem baselines undercount manifest-index file opens and bytes.
  Use the Phase 3 table or later.
- Full-mode and Git cases use the final Phase 8 baseline, but older structure
  rows predate shallow structure semantics.
- Git diff accounting changed when stats and patch collection were collapsed.
  Blob loads, not the unchanged diff counter, expose that improvement.
- Current Git status loads and diffs each dirty file side once. Whole-file
  add/delete stats count lines without running a diff.
- CI artifacts are comparable only within one OS and runner class.

## Interpretation traps

- Timing on a loaded host is not optimization evidence. Keep an unchanged case
  as a drift bracket.
- Sequential case order warms the page cache; do not infer request-cost ratios
  from one ordered run.
- macOS sampling changes absolute throughput substantially. Use profiles for
  composition, not absolute attribution.
- A high counter may represent distinct required probes. Attribute by call site
  and path before optimizing.
- Inventory subsets are nondeterministic when truncated, even though complete
  results are deterministic.
- `ignore`'s serial `build()` walks a symlinked starting root differently from
  `build_parallel()`: the serial walk yields the depth-0 root with the link's
  file type, so a "non-directory" filter admits the root path. The parallel
  walk yields it as a directory. Check root-entry parity when converting a
  walker (`2026-09-20-repo-perf` `evidence/spike.md`). On a tiny tree, the
  default parallel policy may also run every callback on one thread.
- With `git_global(true)`, `ignore` reads the global excludes file through the
  process's `HOME`/`USERPROFILE` and `XDG_CONFIG_HOME`, so a Git-root walk
  fixture inherits the host's global ignores. Assert Git ignore rules in a
  child test process with a disposable home
  (`git_ignore_rules_apply_under_an_isolated_git_configuration`), never by
  mutating this process's environment. Outside a Git repository, `.gitignore`
  and the global excludes do not apply; `.ignore` still does.
- The previously evaluated small hot-path changes were below the project
  threshold. Revisit them only with new counter or profile evidence.
