# Compose Pipeline Quick-Win Performance Spec

> Status: **draft** · Author: Ken (with Claude Opus 4.7) · Created 2026-05-09
>
> Companion document: [`claudine/docs/pipeline.md`](../../docs/pipeline.md) — flat
> enumeration of every step from "user presses Enter" to process exit.
>
> Prior art: [`2026-05-09-slow-prep`](../2026-05-09-slow-prep/spec.md) reduced
> redundant sniff scans, made model-catalog refresh provider-scoped, and
> consolidated repo-root / selection-config discovery into a single
> `CompositionPrepContext`. This spec layers further wins on top of that
> foundation and is intentionally scoped to the *prep* phase that runs
> before the first user-visible byte of stderr.

## Problem

Even after `2026-05-09-slow-prep`, `claudine compose` and `claudine inline-compose` still feel slow. The dominant felt-latency is the gap between the user pressing Enter and the **first byte of stderr** — the `Composition · …` execution header — appearing. Until that line lands, the user is staring at a blank screen with no signal that Claudine has even received the command.

The wall-clock time spent in this gap is the union of every step in Phases A, B, and C1 of the pipeline (see [`docs/pipeline.md`](../../docs/pipeline.md)). Some of that work is genuinely required before a header can be honest about what is being launched (e.g., provider name). Much of it is not, and several optimisations that *don't* even change wall-clock time can dramatically improve perceived latency by emitting feedback earlier.

The goal of this feature is **at least an order-of-magnitude improvement in time-to-first-feedback** plus measurable wall-clock wins on the longest-pole steps.

## Goals

1. **Time-to-first-feedback ≤ 50 ms** from `main()` entry to the first stderr line, on a representative compose invocation.
2. **Wall-clock prep reduction ≥ 10×** on cold-cache invocations targeting OpenCode or Qwen with frontmatter `model:` validation.
3. **Wall-clock prep reduction ≥ 2×** on warm-cache invocations targeting Claude or Codex from inside a known repo.
4. **Zero behavioural regressions**: every existing flag, frontmatter directive, lifecycle event, and error path must continue to work identically. Cache invalidation must be safe; stale data must never cause silent miscompiles of the prompt or wrong-provider launches.
5. **Diagnosable**: every new optimisation must keep a `RUST_LOG=info,compose_prep=trace` view that lets a contributor see whether a step ran fresh, hit a cache, or was deferred.

## Non-Goals

- Changes to the agent process itself (Phase E in the pipeline doc).
- Changes to live streaming or summary rendering (Phases E.4–F).
- Changes to `claudine sequence` orchestration semantics (the per-step pipeline still benefits transitively).
- Changes to the dispatch / lifecycle / hook configuration model.
- Removing any feature, flag, or config knob.

## Trace Findings (2026-05-09)

A `--perf` run was captured at [`trace.md`](trace.md). Headlines:

```
Performance (elapsed 23.25s)
  arg parsing:        584µs
  config loading:     256µs
  tracing init:       69µs
  environment setup:  3.07s
Composition Report total: 165µs
Agent Execution    total: 9.03s   first response: 7.04s   provider api: 6.72s
```

**Crucial scope correction.** The `--perf` collector starts its `env_setup` timer at the **top of `execute_composition_request_inner`** (`composition/mod.rs:521`) and stops it at `mark_env_setup_complete` (`composition/mod.rs:1120`). That covers **Phase C1-C7 of the pipeline doc, not Phase B**. The `elapsed 23.25s` total is also measured from the same `total_start`, so Phase A and Phase B are *invisible* to the `--perf` report. Reconciling the buckets:

| Bucket | Wall-clock | Phase coverage |
|---|---|---|
| arg parsing + config loading + tracing init | < 1 ms | A4 + A5 |
| environment setup | **3.07 s** | C1-C7 |
| Agent Execution total | 9.03 s | E + F |
| Unaccounted (within 23.25s elapsed) | ≈ 11.15 s | C8 + C9 + D + part of E that isn't in `summary.duration_ms` |
| Phase A (other) + Phase B | unmeasured | A1-A3 + B1-B6 |

The `--perf` data therefore tells us **C1-C7 alone is taking 3+ seconds**, and reaffirms that Darkmatter composition (the original suspect) is essentially free at 165 µs.

**Root cause (high confidence, found by code reading).** Composition pays for the launch-workspace-context discovery **three times**:

1. **B2.3.a — `compose_prep.shared_sniff`** does the canonical scan. Cached as `prep_context.launch_context` and `prep_context.env_context`.
2. **C2.1 — `composition/mod.rs:544`** calls `env::resolve_launch_workspace_context(&launch_cwd, source_repo_root)` directly to build a `LaunchWorkspaceContext` for the header. **This is a fresh scan; the prep-time cache is not consulted.**
3. **C3.3 — `env::build_child_env` (`env.rs:47-60`)** calls `resolve_launch_workspace_context` *again internally*. A `build_child_env_with_launch` variant exists (`env.rs:86`) for callers that already have the context, but the composition path hasn't been migrated. The direct wrapper path was migrated.

`resolve_launch_workspace_context` (`env.rs:360`) does:

- `detect_repo_root(launch_cwd)` (which calls `detect_git` then `detect_repo`)
- a second `detect_repo_root(launch_cwd)` for `child_cwd`
- `resolve_monorepo_package_context(launch_cwd)` (filesystem walk)

`detect_git` is the expensive call: it reads HEAD, branch, upstream, and commit summary on every invocation. Two-to-three of those per compose run, on top of the canonical scan in B2.3.a, plausibly accounts for the bulk of the 3.07 s — especially in worktree layouts (the captured trace was from the `darkmatter` worktree).

**Implication for this spec.** The original recommendations (W1-W6) are still valid, but the *highest-impact item is new* and is now the dominant priority:

> **W0 — Eliminate redundant launch-workspace-context discovery in C2.1 and C3.3 by threading `prep_context.launch_workspace` through composition.**

W0 is added below as a new hot-spot row and as a new design section. The original W1-W6 are renumbered only in the sequencing table at the bottom.

Other findings worth noting:

- **Phase B is not measured.** Whatever the prep phase costs (Phase A1-B6 on the pipeline doc) is invisible to `--perf` because the timer hasn't started yet. We should extend `StartupTimings` to capture from `main()` entry through the start of `execute_composition_request_inner` so future traces show the full picture. Tracked as a new design item below.
- **Q2 resolved.** Darkmatter compose is 165 µs total. Caching it (proposed earlier as a hypothetical W7) is not worth pursuing.
- **Q4 partially resolved.** The captured run targeted `--claude` (static catalog, O(1) refresh). Even so, `env_setup` is 3 s, confirming model-catalog work is *not* the dominant cost on this kind of run. W3 (background refresh) still matters but only for OpenCode/Qwen runs with frontmatter `model:` — re-rank accordingly.
- **Q1 partially resolved.** The redundant launch-workspace scans are likely the lion's share of the 3 s. After W0 lands, B2.3.a's scan will be the only one — at which point parallelisation (W4) targets a much smaller absolute number, and the cost-benefit may not justify the threading complexity. Defer W4 until post-W0 measurements.

## Hot-Spot Inventory

These are the candidate steps, ranked by effort-to-impact ratio after the trace findings. Each row references the pipeline phase from [`docs/pipeline.md`](../../docs/pipeline.md).

| Pipeline ID | Step | Cost driver | Today's behaviour | Optimisation lever |
|---|---|---|---|---|
| **C2.1 + C3.3** | **Redundant `resolve_launch_workspace_context`** | **Filesystem syscalls (`detect_git`, `detect_repo`, monorepo walk)** | **Composition runs the same scan up to three times per invocation** | **W0 — Reuse `prep_context.launch_context` / `env_context`; migrate compose to `build_child_env_with_launch`** |
| C2.2 | Execution header emission | UX timing | Fires after C1, blocking on B + C1 | W1 — Receipt banner before B2.1 |
| C1.4 | `resolve_binary_path_direct` | Redundant `which::which` | Re-scans PATH for the binary that B2.3.d already found | W2 — Plumb path through `InstalledProviderSnapshot` |
| B3.4 | `compose_prep.model_catalog` refresh | Subprocess spawn | OpenCode / Qwen shell out to `<provider> models` synchronously when frontmatter `model:` exists and no override wins | W3 — Background refresh; proceed with stale cache |
| B2.3.* | `CompositionPrepContext::new` sub-steps | Sequential execution | `shared_sniff`, `selection_config`, `installed_clients` run serially | W4 — `std::thread::scope` parallelism (defer until post-W0 measurement) |
| B2.3.d | `installed_clients` PATH scan | Filesystem syscalls | Full PATH walk for 8 binaries on every invocation | W5 — Disk-cache keyed on PATH hash + entry mtimes |
| various | `crate::log::terminal()` | Terminal capability detection | Called multiple times per invocation | W6 — `LazyLock<Terminal>` per-process |
| measurement | `--perf` granularity | Timer scope + bucketing | Total starts at C1 (Phase A+B invisible); `environment setup` lumps all of C1-C7 into one number | W8 — Granular timing model: cover Phase A+B *and* break env-setup into named sub-stages mapped to pipeline IDs |

## Design

### W0 — Eliminate redundant launch-workspace-context discovery

**This is now the highest-priority change** based on the trace, ahead of every other item below.

Today, on a single compose invocation:

1. `CompositionPrepContext::new` runs the canonical `sniff::detect_with_plan` and stores `launch_context: LaunchContext` and `env_context: EnvironmentContext` (`prep_context.rs:108-136`).
2. `composition/mod.rs:544` calls `env::resolve_launch_workspace_context(&launch_cwd, source_repo_root)` — a *different* function that does its own `detect_git` + `detect_repo` + monorepo scan, ignoring the cached prep data. The result is used to build the early header `EnvPlan`.
3. `composition/mod.rs:746` calls `env::build_child_env(...)` (the variant *without* `_with_launch`), which internally calls `resolve_launch_workspace_context` *yet again* (`env.rs:60`).

The direct wrapper path (`run_provider_wrapper_inner`) was migrated to `build_child_env_with_launch` and threads its own pre-computed `LaunchWorkspaceContext`. The composition path was missed — the comment at `env.rs:75-84` describes the pattern, but compose never adopted it.

**Change.**

1. Add a `launch_workspace: LaunchWorkspaceContext` field to `CompositionPrepContext`, computed once from the same shared sniff result the existing `launch_context` and `env_context` are built from. Add a fast `LaunchWorkspaceContext::from_sniff_result(...)` constructor (mirror of the existing `LaunchContext::from_sniff_result`) so no additional filesystem walks are required.
2. Thread the `LaunchWorkspaceContext` through `CompositionExecutionRequest` as `prep_launch_workspace: Option<LaunchWorkspaceContext>` (additive, optional for legacy callers).
3. At `composition/mod.rs:544`, reuse `request.prep_launch_workspace` instead of calling `resolve_launch_workspace_context` again. Fall back to a fresh call only when the field is `None` (legacy library callers).
4. Replace the `env::build_child_env(...)` call site at `composition/mod.rs:746` with `env::build_child_env_with_launch(... launch_workspace)`. This is a one-line substitution that requires the threaded context.
5. Verify that `enforce_repo_launch_detection` still triggers the existing `--repo` hard-fail on prep-time sniff failure (already tested by the slow-prep feature; W0 must not regress it).

After W0, compose should make exactly one filesystem walk (B2.3.a `compose_prep.shared_sniff`) for repo / git / package context per invocation. The measured ~3 s should drop dramatically; even a conservative estimate that each redundant `detect_git` costs ~500-800 ms in a worktree implies ~1.5-2.5 s recovered from this one change.

**Effort:** ≈3-4 hr. **Risk:** low (extends the existing slow-prep pattern; no new caching, no new threading). **Wall-clock:** **expected dominant single win** based on trace evidence. **Test:** integration test asserting that a synthetic counter on `detect_git` (or a tracing-driven equivalent) increments at most once per compose invocation across the prep + env-setup window.

### W1 — Receipt banner (C2.2 split)

Emit a short receipt banner immediately after `Cli::parse_from` returns and the compose subcommand dispatcher (`run_compose` / `run_inline_compose`) has the file argument in hand. The banner is intentionally information-poor:

```
→ Composing prompts/foo.md…
```

It uses the file_ref string verbatim (no resolution) and a single-line `Status::from_prose` rendering. **It does not touch `LaunchContext`, `EnvironmentContext`, `InstalledAiClients`, or any subprocess.**

The full execution header (provider, yolo, mode, etc.) continues to fire at C2.2 once provider resolution is complete. The receipt banner is an additive line, not a replacement.

Skip when `--silent`. Suppressed details under `--quiet` follow the same rules as the existing header.

**Effort:** ≈1 hr. **Risk:** trivial. **Wall-clock:** zero. **Perceived latency:** transformative.

### W2 — Kill the redundant `which::which`

Today:

- B2.3.d: `InstalledAiClients::new()` (`sniff/lib/src/programs/category_detector.rs:160`) returns `Option<(PathBuf, ExecutableSource)>` for every supported binary, in parallel.
- `build_installed_snapshot` (`claudine/lib/src/composition/select.rs:21`) discards the path and keeps only `Vec<Provider>`.
- C1.4: `resolve_binary_path_direct` (`claudine/cli/src/commands/wrap/mod.rs:217`) re-runs `which::which()` for the chosen provider's binary.

Change:

1. Extend `InstalledProviderSnapshot` with `binary_paths: BTreeMap<Provider, PathBuf>` (or a parallel `HashMap`), populated by `build_installed_snapshot` from `clients.path_with_source(p.sniff_ai_cli())`.
2. Add `fn binary_path(&self, provider: Provider) -> Option<&Path>` accessor.
3. Replace the body of `resolve_binary_path_direct` with a snapshot lookup; fall back to `which::which` only when the snapshot has no entry (e.g., the rare path where `request.resolved_target` is `Some` but the snapshot was not threaded through).
4. Thread the snapshot from `CompositionPrepContext` into `execute_composition_request` via a new optional field on `CompositionExecutionRequest`.

**Effort:** ≈2 hr. **Risk:** low. **Wall-clock:** small but free. **Side benefit:** the snapshot is now available everywhere it's needed, simplifying any future caller.

### W3 — Background model-catalog refresh

Today, when frontmatter declares a `model:` and no CLI / env override wins, B3.4 calls `catalog.refresh_provider_blocking(provider)`. For OpenCode and Qwen this is a `<provider> models` subprocess that can take 200–2000 ms.

The catalog already has stale-cache fallback semantics. Add a new entry point — `refresh_provider_async` — that:

1. Returns immediately with the existing on-disk cache (or `None` if no cache exists).
2. Spawns a detached thread that performs the refresh, writes the refreshed cache to disk, and exits.
3. The detached thread holds no `&mut self` borrow of the catalog; it serialises directly to the cache file with a lock-free atomic rename pattern.

Resolution at B3.5 reads the immediate (stale) cache for validation. The freshly refreshed cache is picked up by the *next* invocation. This is acceptable because:

- Frontmatter `model:` validation is best-effort by design (the catalog can already be out-of-date or missing).
- Provider model lists change rarely.
- The user-visible behaviour change is "first-time-after-model-release validation may use the previous list" — which is no worse than the existing stale-cache fallback path.

When **no cache exists** (true cold start, e.g. first invocation ever), fall back to the synchronous blocking refresh so we don't silently emit "model X not in catalog" for a brand-new install. This case is rare and worth the wait.

Add `--no-background-refresh` (or env var `CLAUDINE_BACKGROUND_REFRESH=0`) as an escape hatch for users who explicitly want the old blocking behaviour.

**Effort:** ≈4 hr. **Risk:** medium (concurrent file writes; threaded subprocess management). **Wall-clock:** large for OpenCode/Qwen with frontmatter `model:`.

### W4 — Parallelise `CompositionPrepContext::new`

Today the four sub-steps run sequentially under `compose_prep.prep_context`. Dependency analysis:

```
shared_sniff (B2.3.a) ──► source_repo_root (B2.3.b) ──► selection_config (B2.3.c)
                       └──────────────────────────────► (also feeds C5.2)
installed_clients (B2.3.d) — independent
```

Restructure with `std::thread::scope`:

1. Spawn `shared_sniff` and `installed_clients` in parallel.
2. Join `shared_sniff`, then run `source_repo_root` (which depends on the launch repo root from sniff).
3. Run `selection_config` (depends on source_repo_root).
4. Join `installed_clients`.

In practice this collapses the four-stage critical path into roughly two stages. The implementation must:

- Use `std::thread::scope` so borrows of stack-allocated inputs (`cwd`, `excluded`) are sound.
- Preserve the existing tracing spans inside each thread by using `tracing::Span::current().in_scope` or explicit instrumentation.
- Preserve the existing fallback (`SniffResult::default()` + `launch_detection_error: Some(...)`) on sniff failure.
- Continue to honour the `--repo` hard-fail contract via `enforce_repo_launch_detection`.

**Effort:** ≈3 hr. **Risk:** low–medium (thread lifetime correctness; preserving span attribution in `RUST_LOG` traces). **Wall-clock:** medium; depends on which sub-step dominates today (the trace will tell us).

### W5 — Disk-cache `installed_clients`

Cache `InstalledAiClients` results at `~/.claudine/cache/installed.json`, keyed on:

```
hash(PATH_env)  +  list_of(entry, mtime)  for each entry in PATH
```

Format:

```json
{
  "version": 1,
  "key": "<blake3 of canonicalised PATH and per-entry mtimes>",
  "captured_at": "2026-05-09T12:34:56Z",
  "ttl_secs": 3600,
  "entries": {
    "claude": {"path": "/opt/homebrew/bin/claude", "source": "Path"},
    "codex":  {"path": "/usr/local/bin/codex",     "source": "Path"},
    ...
  }
}
```

On each invocation:

1. Re-compute the cache key. (Cheap: `stat()` per PATH entry, hash.)
2. If the on-disk cache key matches **and** `now - captured_at < ttl_secs`, return the cached snapshot.
3. Otherwise fall through to the existing `InstalledAiClients::new()` and write the result.

Invalidation knobs:

- `--no-cache` (or `CLAUDINE_NO_CACHE=1`) bypasses the cache.
- `claudine cache clear` (new admin subcommand, or extend an existing one) deletes the file.
- TTL configurable via `CLAUDINE_INSTALLED_CACHE_TTL` (default `3600`).

The `~/.claudine/cache/models/<provider>.json` pattern from `2026-04-25-agent-selection` is a good template — reuse the same atomic-write helpers.

**Effort:** ≈5 hr. **Risk:** low–medium (cache invalidation correctness; corrupt cache should fall back to a fresh scan, not crash). **Wall-clock:** medium-large for cold runs; near-zero for warm.

### W8 — Granular `--perf` timing model

The current `--perf` model is too coarse to drive further decisions:

- The collector starts its timer at the top of `execute_composition_request_inner` (C1), so **Phase A and Phase B are invisible**.
- `environment setup` is a single 3.07 s number that lumps **all of C1-C7 into one bucket**. We can identify the dominant cost by reading code, but a contributor running `--perf` after W0 lands has no way to confirm whether the residual time is in `build_child_env`, system-prompt resolution, MCP work, or somewhere else without re-reading the source.

W8 extends the timing model on both axes: **earlier coverage** (Phase A + B) and **finer granularity inside env_setup** (sub-stages mapped to pipeline IDs).

#### W8a — Extend `StartupTimings` to cover Phase A + Phase B

1. Capture a `process_start: Instant` in `main()` *before* anything else.
2. Extend `StartupTimings` with `pre_dispatch: Duration` (process_start → subcommand entry, i.e. through A6) and `prep_phase: Duration` (subcommand entry → top of `execute_composition_request_inner`, i.e. all of Phase B).
3. Have `run_compose_inner` / `run_inline_compose_inner` capture an `Instant` at entry and set `prep_phase` when `execute_composition_request` is called.

#### W8b — Break `environment setup` into named sub-stages

Add a `Vec<SubstageTiming>` to `CommandPerfCollector` and a small `mark_substage(name)` API. Each call snapshots elapsed-since-last and records `(name, duration)`. Sub-stage names map 1:1 to pipeline-doc IDs so the report and the doc cross-reference cleanly:

| Sub-stage label | Pipeline IDs | Boundary |
|---|---|---|
| `target resolution` | C1.1-C1.4 | from `total_start` until just before C2.1 |
| `header env plan` | C2.1-C2.2 | the `resolve_launch_workspace_context` + header emission window |
| `child env build` | C3.1-C3.5 | `env::build_child_env` (or `_with_launch` post-W0) returns |
| `mcp composition` | C3.6-C3.11 | only when `--mcp` / `--use` is set; otherwise zero-duration entry |
| `argv assembly` | C4.1-C4.7 | yolo + entrypoint + non-interactive + model + output flags |
| `system prompt` | C5.1-C5.6 | through `apply_system_prompt` and `apply_sandbox` |
| `stream + prompt delivery` | C6-C7 | through `mark_env_setup_complete` |

Each sub-stage label should be stable so traces from different commits are diff-able.

#### W8c — Render the new structure

`render_perf_report` shows the sub-stages indented under `environment setup`, plus the new Phase A/B lines:

```
CLI Overhead
  pre-dispatch:        Xms     ← Phase A
  prep phase:          Xms     ← Phase B
  environment setup:   Xms     ← Phase C1-C7 total (sum of sub-stages below)
    target resolution:        Xms
    header env plan:          Xms
    child env build:          Xms
    mcp composition:          Xms
    argv assembly:            Xms
    system prompt:            Xms
    stream + prompt delivery: Xms
```

Sub-stage rendering is gated to `--perf` only — the regular CLI surface continues to render the single `environment setup` line. Sub-stages with zero duration (e.g. `mcp composition` on a non-MCP run) are still rendered with `0µs` so their absence is unambiguous.

#### Why this matters

After W0 lands, the residual env-setup time should drop substantially. Without W8b, we won't know whether the remaining time lives in `system prompt` (suggesting a system-prompt-discovery cache is the next move), `child env build` (suggesting `sanitize_process_env` or shadow-HOME setup is hot), or somewhere else. With W8b, the next `--perf` capture is self-explanatory.

W8b also lets us validate W0: post-W0, `header env plan` should drop from "seconds" to "microseconds" because the redundant scan is gone. If it doesn't, we know the wrong scan was eliminated.

**Effort:** W8a ≈1 hr; W8b ≈2 hr; W8c ≈1 hr. Total ≈4 hr for the full W8. **Risk:** trivial. **Wall-clock:** zero — instrumentation only. **Diagnostic value:** essential.

### W9 — `--perf` report formatting fixes

The current report has three small but distracting issues (visible in [`trace.md`](trace.md) and the user's screenshot):

1. **`Composition Report` has `total:` at the top** — every other section in the report lists individual metrics and never a total. The placement is also out of step with the natural reading order ("here are the parts" → "here is the sum").
2. **No section totals at all on `CLI Overhead` and `Agent Execution`** — so the user has to mentally sum buckets to sanity-check against `elapsed 23.25s`.
3. **Label/value alignment breaks when labels exceed 20 characters** — the renderer uses `format!("  {:20}{}", label, value)` (`perf.rs:370` and similar). Labels like `"frontmatter shell expansion:"` (28 chars) overflow the field and abut their value with no whitespace, producing lines like `frontmatter shell expansion:0µs` (no space).

W9 fixes all three.

#### W9a — Move per-section totals to a labeled bottom line

For every section (`CLI Overhead`, `Composition Report`, `Agent Execution`, and any new sub-stage sections from W8), render the total as the **last** line, labeled `TOTAL:` (uppercase to distinguish from data rows). Above it, render a **double-underline separator** matching the column geometry — drawn with the box-drawing double-horizontal `═` character so it reads as a clear "summed below" cue without depending on theme colours.

Example (post-W0, post-W8):

```
CLI Overhead
  pre-dispatch:                  Xms
  prep phase:                    Xms
  arg parsing:                  584µs
  config loading:               256µs
  tracing init:                  69µs
  environment setup:           3.07s
    target resolution:           Xms
    header env plan:             Xms
    child env build:             Xms
    mcp composition:              0µs
    argv assembly:               Xms
    system prompt:               Xms
    stream + prompt delivery:    Xms
  ═════════════════════════════════
  TOTAL:                        Xms

Composition Report
  frontmatter interpolation:     11µs
  frontmatter shell expansion:    0µs
  effective state build:          8µs
  ...
  link normalization:            36µs
  ═════════════════════════════════
  TOTAL:                        165µs

Agent Execution
  launches:                         1
  first response:               7.04s
  total execution:              9.03s
  provider api:                 6.72s
  ═════════════════════════════════
  TOTAL:                        9.03s
```

Notes:

- For `Agent Execution`, `TOTAL:` mirrors `total execution:` rather than summing rows (since `launches` is a count and the durations overlap). Drop the existing `total execution:` row in favour of the bottom `TOTAL:` to avoid two synonymous lines, and rename `provider api:` → `provider api duration:` so the relationship to the total is unambiguous.
- For `Composition Report`, `TOTAL:` replaces the existing `total:` row at the top.

#### W9b — Width-aware label column

Replace `{:20}` with a per-section computed width:

```rust
let label_width = section_rows
    .iter()
    .map(|row| row.label.chars().count())
    .max()
    .unwrap_or(0)
    + 2;   // 2-space gutter between label and value column
```

Render each row as `format!("  {:<label_width$}{:>value_width$}", row.label, row.value, ...)` so labels left-align against the indent and values right-align against a uniform column. The separator line in W9a is then exactly `label_width + value_width` wide so it visually anchors the column.

`value_width` should be the max formatted value width in the section (also computed per-section). For tiny values like `0µs` next to `3.07s` this gives a cleanly right-aligned numeric column.

Sub-stages indented under `environment setup` get an extra two-space indent on the label *before* the width calculation, so they nest visually but their value column still aligns with the parent section's value column.

#### W9c — Bold the section TOTAL

`TOTAL:` should render in bold (`<b>TOTAL:</b>`) — biscuit-terminal Prose markup is already used elsewhere in the renderer, so this is a one-line addition. The double-underline separator stays unbold so the visual hierarchy reads "section header bold → rows plain → separator → bold total".

#### Implementation notes

- All three sub-tasks live in `render_perf_report` in `claudine/cli/src/perf.rs`. Refactor the body around a small `Section { title, rows: Vec<Row>, total: Total }` helper struct so each section is rendered through a single `render_section(...)` function that handles width computation, separator drawing, and total placement uniformly.
- Tests: snapshot test (`insta`-style) of `render_perf_report` on a fixture report covering all three sections plus the new W8 sub-stages, with one row deliberately exceeding the old 20-char limit so the alignment fix is regression-protected.
- Guard against zero-width sections: when a section has no rows (e.g. dry-run agent execution), suppress the entire block including the separator and total.

**Effort:** ≈3 hr. **Risk:** trivial. **Wall-clock:** zero (rendering only). **Diagnostic value:** indirect but real — clearer reports lead to faster decisions on subsequent optimisation work.

### W6 — Memoise `crate::log::terminal()`

`crate::log::terminal()` is called from at least:

- `compose.rs:561` (inline-compose pre-validation)
- `composition/mod.rs` (multiple call sites in the executor)
- `commands/wrap/composition/mod.rs` reports

Each call may re-detect terminal capabilities on TTY paths. Memoising is straightforward:

1. Wrap the existing `terminal()` body in a `LazyLock<Terminal>` (or `OnceLock<Terminal>` if construction can fail).
2. Continue to use the post-2026-05-06 `optimistic_terminal(None)` path for non-interactive callers.
3. Keep a separate accessor for the rare cases that genuinely need a fresh detection (none known today).

Test that environment variables read at construction time (`NO_COLOR`, `CLICOLOR_FORCE`, `TERM`) are still honoured — `LazyLock` initialises on first access, which happens after `argv::normalize`, so this is fine.

**Effort:** ≈30 min. **Risk:** trivial. **Wall-clock:** small. **Side benefit:** consistency — all output paths see the same terminal config, so e.g. width-dependent rendering can't drift between sections.

## Sequencing

Updated post-trace. The recommended order now leads with W0 because the trace identifies it as the dominant cost and the rest of the spec's wins target much smaller absolute numbers in comparison. W8 lands second so every subsequent measurement is honest.

| Order | Win | Why this order |
|---|---|---|
| 1 | **W8 + W9** — Granular `--perf` timing model **and** report formatting fixes | Land together first so the W0 measurement is both honest (W8) *and* legible (W9). Without these, we can't prove W0 eliminated the right scan or read the result cleanly. They share `render_perf_report` as their primary edit surface. |
| 2 | **W0** — Eliminate redundant `resolve_launch_workspace_context` | Trace evidence says C1-C7 is 3.07 s; W0 directly attacks that with a low-risk, code-pattern-already-in-the-tree fix. With W8 + W9 already in place, the post-W0 trace will show exactly which sub-stages dropped. |
| 3 | **W1** — Receipt banner | Free perceived-latency win; lands as soon as W8 lets us measure how much Phase A+B actually costs. |
| 4 | **W6** — Memoise `terminal()` | Trivial. Removes noise in subsequent measurements. |
| 5 | **W2** — Kill redundant `which::which` | Cleanup; small wall-clock benefit; simplifies callers. |
| 6 | **Re-measure with `--perf`.** Decide whether W3 / W4 / W5 are still worth the effort. |
| 7 | **W3** — Background model-catalog refresh | Only matters for OpenCode/Qwen runs with frontmatter `model:`. Defer until trace shows it on the critical path. |
| 8 | **W4** — Parallelise `CompositionPrepContext::new` | Defer until W0 lands and we've re-measured; the absolute number to parallelise is much smaller after W0. |
| 9 | **W5** — Disk-cache installed clients | Defer until W0 / W8 reveal whether B2.3.d is significant after the redundant scans are gone. |

Each item can be its own commit and (where appropriate) its own PR. W0 + W8 + W1 + W6 + W2 are roughly one day's work and should be done in one batch. W3, W4, W5 should be re-evaluated against fresh `--perf` data after that batch lands.

## Validation

### Measurement

Every change must be measured with **the same harness** before-and-after:

```sh
RUST_LOG=info,compose_prep=trace c compose --perf prompts/<canonical>.md \
  -y --claude
```

Capture three independent runs and report median wall-clock and median per-span busy time. Use the existing `claudine/docs/topics/performance-testing.md` workflow.

The canonical test prompt should:

- Live in a real repo with `.git`.
- Have at least one frontmatter `model:` directive (so W3 is exercised).
- Have at least one `$(…)` shell expansion in the body (so B4.4 is exercised).

Track three numbers per change:

1. **Time-to-first-feedback (TTFF)** — wall-clock from the first `tracing` event in the span tree to the first stderr line. Today this is roughly the duration of the entire `composition_prepare` span.
2. **Prep-phase wall-clock** — wall-clock from process start to the `composition_prepare` span close (which is currently the same as TTFF; W1 decouples them).
3. **Per-span busy time** — extracted from the trace.

### Acceptance criteria

A change is accepted when:

- The three timing numbers above all improve (or, for W1, TTFF improves while wall-clock is unchanged within noise).
- All existing tests in `claudine/lib`, `claudine/cli`, and `darkmatter/lib` pass.
- A new test (or extension of an existing one) covers the new behaviour:
  - W1: integration test asserting the receipt banner appears on stderr before the existing header banner, ordered correctly.
  - W2: unit test asserting `resolve_binary_path_direct` consults the snapshot when `request.resolved_target` is set, and that no `which::which` call is made on that path.
  - W3: unit test asserting `refresh_provider_async` returns immediately and the cache file is written within a bounded interval.
  - W4: unit test asserting `CompositionPrepContext::new` produces the same outputs as the serial version on a fixture; tracing-span ordering test ensuring spans still nest correctly under `compose_prep.prep_context`.
  - W5: unit tests for cache hit, cache miss (key mismatch), TTL expiry, corrupt-cache fallback, `--no-cache` bypass.
  - W6: unit test asserting `terminal()` returns the same instance on repeat calls within a process.
- `cargo clippy --workspace -- -D warnings` clean.
- The `pipeline.md` doc is updated to reflect the new step semantics (e.g., remove C1.4 from the "redundant" hot-spot list once W2 lands).

## Open Questions

Updated against the captured trace. Resolved questions are kept with their resolution noted; remaining questions block specific later wins, not W0/W1/W6/W8.

1. **Q1 — partially resolved.** Pre-trace assumption was that `shared_sniff` and `installed_clients` were both expensive. Trace says C1-C7 is 3 s and code review identifies redundant `resolve_launch_workspace_context` as the dominant culprit; **W4 (parallelism) is now de-prioritised** until W0 lands and we re-measure. After W0, B2.3 is a single sniff scan, and even halving it via parallelism may not be worth the threading complexity.
2. **Q2 — RESOLVED.** Composition perf in the captured trace is 165 µs total. Darkmatter compose caching is **not worth pursuing**.
3. **Q3** (open): Does the legacy compose path (`request.resolved_target is None`) still need `resolve_binary_path_direct`? If we can remove the `None` branch entirely, W2 simplifies further. Defer until W2 implementation.
4. **Q4 — partially resolved.** Captured trace targeted `--claude` (static catalog). Even so, env_setup is 3 s, confirming **model catalog work is *not* the dominant cost** on this run. W3 still matters for OpenCode/Qwen runs with frontmatter `model:` — but only after W0 reveals what their baseline looks like.
5. **Q5** (open): Cached `terminal()` correctness when stdin is piped vs stdout/stderr inherited. Trivial to verify during W6 implementation.
6. **Q6** (open): Disk-cache path location for W5 (`~/.claudine/cache/...` vs XDG). Decide during W5 implementation, after W0 has clarified whether W5 is even needed.
7. **Q7 (NEW).** After W0 lands, what is the residual env-setup time? A captured `--perf` immediately after W0 should be the gating data point for whether W3, W4, W5 are still worth the effort. **Add `trace-after-w0.md` next to `trace.md` once W0 is implemented.**
8. **Q8 (NEW).** Does the direct wrapper path (`run_provider_wrapper_inner`) — which already uses `build_child_env_with_launch` — show a similarly tight env-setup window when run with `--perf`? If yes, that confirms W0 is the right fix and the wrapper path is a working reference implementation. If no, there's another issue not yet diagnosed. Capture a wrapper `--perf` trace as a control.

## Risks

- **R1.** Background refresh (W3) introduces concurrent file writes to the model-catalog cache. The atomic-rename pattern must be ironclad; a partial write must not corrupt the cache. **Mitigation:** unit test partial-write fault injection.
- **R2.** Disk-cache (W5) staleness can in principle cause a "binary missing" report when the user just installed a provider. **Mitigation:** TTL is short (1 hr), `--no-cache` exists, and the resolver always falls back to the live `which::which` if the cached path no longer exists.
- **R3.** Parallelising (W4) inside `std::thread::scope` can subtly affect tracing-span ordering, making `RUST_LOG` traces harder to read. **Mitigation:** explicit `Span::current()` capture before each thread spawn, and a tracing-order test in the integration suite.
- **R4.** Receipt banner (W1) may be misleading when the file_ref is invalid (e.g., user typed a typo). The banner says "Composing foo.md…" then we error out. **Mitigation:** the existing error path renders a clear `BlockError` so the contradiction reads as "we tried, here's why we couldn't" — acceptable.

## Out of Scope (Tracked Elsewhere)

- Reducing the cost of provider startup itself (Node/Python warmup) — outside Claudine's control.
- Reducing Darkmatter compose-pass cost — tracked as Q2 above; if hot, file a separate spec.
- Reducing CLI binary startup (linker / dependency loading) — would require workspace-level changes.

## Validation Plan (Updated)

After the captured trace, the validation plan is:

1. **Implement W0 + W8 + W6 + W1 + W2 in one batch.** These are low-risk, mostly mechanical, and complement each other. Total ≈1 day.
2. **Capture `trace-after-w0.md`** with the same harness used for `trace.md`. The expected outcome:
   - `environment setup` drops by 1.5-2.5 s (most of the redundant `detect_git` cost).
   - `pre-dispatch` and `prep phase` lines now appear in the report (W8).
   - First user-visible byte (W1's receipt banner) appears within tens of milliseconds.
3. **Capture a wrapper `--perf` control** (Q8). If the wrapper path's env-setup is already in the hundreds of milliseconds, that's the target compose should match after W0.
4. **Decide on W3 / W4 / W5** based on the new numbers. None of them are obviously worth doing if W0 alone gets us under 500 ms env-setup.

## References

- [`claudine/docs/pipeline.md`](../../docs/pipeline.md) — pipeline step inventory.
- [`claudine/docs/topics/composition.md`](../../docs/topics/composition.md) — narrative composition flow.
- [`claudine/docs/topics/performance-testing.md`](../../docs/topics/performance-testing.md) — measurement harness.
- [`claudine/features/2026-05-09-slow-prep/`](../2026-05-09-slow-prep/) — prior optimisation work this builds on.
- [`claudine/features/2026-04-25-agent-selection/`](../2026-04-25-agent-selection/) — model-catalog and selection-config patterns reused here.
