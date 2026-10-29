---
area: repo
status: draft
created: 2026-09-21
owner: Ken Snyder <ken@ken.net>
origin: darkmatter slow-test investigation on feat/dark-fixes, 2026-09-21
related:
    - 2026-09-20-repo-perf
    - 2026-09-21-lockfile-provenance-cost
packages:
    - dmls
    - darkmatter
    - claudine
---

# Stop host contention from turning into red tests

## Outcome

On the shared macOS development host, a test result means something about the
code. A test that is cheap does not go red because an unrelated process was
busy, a test that is expensive is visibly expensive on an idle host, and the
repository's own tooling is not one of the things making the host busy.

This is a **local-development** spec. It adds no CI run, matrix cell, fixture,
or gate; hosted runners are single-tenant and showed none of this.

## The policy this collides with

Slow tests are treated as failing tests. nextest's default profile marks a test
`SLOW` at a 5 s period (`.config/nextest.toml`, `slow-timeout = { period =
"5s", terminate-after = 6 }`). That period is **wall-clock**. On an idle host,
wall-clock is a fair proxy for cost. On this host it is not, and the policy
turns scheduler noise into failures.

Keep the policy. It caught five genuinely overweight tests in this
investigation. The problem is the measurement under it.

The configuration already knows. Its comment says integration tests "finish in
5–10s isolated but get squeezed past 15s under heavy parallel load", which is
why termination is at 30 s rather than 5 s, and `retries = 0` is deliberate: "A
test that passes only on retry is still a failed test run. Keep retries disabled
so timing and resource-contention defects remain visible locally." This spec
does not reopen either ruling. Contention should stay visible. What is missing
is a way to tell, once it is visible, whether a `SLOW` is the test's cost or the
host's.

## Evidence

One host, one day: `aarch64-apple-darwin`, 16 logical cores (12 performance),
2026-09-21, `feat/dark-fixes`. Load averages are 1-minute figures from
`uptime`. Nothing here was controlled; these are observations to reproduce.

### The host is never idle

| When | 1-min load | What was running besides the measured work |
|---|---|---|
| 09:05 | 9–23 | baseline for the session |
| 09:11 | 29.4 | three `dmls` bursts, another session's `cargo`/`rustc` |
| 12:37 | **62.5** | 13 `cargo`/`rustc`/`nextest` processes from another session |
| 12:42 | 8.0 | settled |
| 12:51 | 33.6 | another session again |

Load above 16 means runnable work is queueing. The session never saw the host
below load 4.

### Contention flips results, in both directions

Same commit, same `just test` in `darkmatter/`, minutes apart:

| Host load at start | Wall time | Result |
|---|---|---|
| ~19, spiking to 62 during the run | 93.1 s | 8,460 passed, **8 slow** |
| ~8 | 54.9 s | 8,460 passed, 0 slow |

The eight slow tests in the first run were not the expensive ones. They
included `seed_state_tests::env_resolves` and
`type_tests::test_compose_context_capture` — tests with no loop and no matrix.
None of them was slow in any other run. A red from that run says nothing about
the code.

The other direction matters as much. Judged by wall-clock under load, five
overweight tests looked like one more flake each. Measured by **CPU time**
(`/usr/bin/time -p`, `user + sys`, the test binary run alone with `--exact`)
they separate cleanly:

| Test | CPU alone | Verdict |
|---|---|---|
| claudine `every_reference_spelling_reaches_the_referenced_document` | 25.6 s (11.0 s wall) | over budget on any host |
| darkmatter `a_warm_cache_root_never_replays_composed_local_output` | 5.9 s | over budget on any host |
| darkmatter `an_existing_cache_root_is_left_byte_identical_by_local_only_work` | 3.0 s | over budget at ~1.7× slowdown |
| darkmatter `dynamic_nested_and_probe_dependent_shapes_fail_before_execution` | 2.5 s | went slow at load ~4 |
| darkmatter `test_compose_cleanup_preserves_nested_lists_inside_blockquotes` | 2.2 s | over budget at ~2.3× slowdown |

All five were restructured on `feat/dark-fixes`. The lesson for this spec is the
method: **CPU time, run alone, is the measurement that distinguishes "this test
is expensive" from "this host is busy."** Wall-clock under `just test` cannot.

A completed fix's implementation log already records the first of these timing
out at load ~35 and being waved through after a `-j 2` rerun
(`2026-08-02-silent-empty-ctx-values`). It was over budget the whole time. The
host's noise is what hid it.

### Contention blocked a push

The strict pre-push hook for this branch failed once on a single cell, `L2
darkmatter-cli (tmux,wezterm)`:
`level2_frontmatter_tables::level2_style_frontmatter_table_right_alignment_pushes_row_to_right_edge`
reached the 30 s termination with no output, at a 1-minute load near 44 caused
by other sessions. The same test on the same commit had passed in the hook 25
minutes earlier, then passed 3 of 3 run alone in 3.0–3.3 s while the load was
still 35–47, and passed in the hook on retry once the load fell to ~14. Nothing
about the code changed between those runs.

The cost was bounded only because the hook publishes per-cell receipts: the
retry reused 14 passing cells and took 3 m 44 s instead of 12 m 43 s. A stall
with no output is a different failure shape from a slow test — it looks like
the WezTerm harness waiting on a starved terminal, the family `95b0d75d3`
("give the WezTerm reachability probe room for a loaded host") already touched
once. It was not diagnosed here.

### The repository's own tooling is part of the load

Three `dmls` processes, children of one Zed instance, one per open worktree
(`feat-better-static-analysis`, `feat-unifi`, `feat-dark-fixes`), all started
together, 59 minutes old when sampled:

| PID | CPU time consumed | Share of one core |
|---|---|---|
| 42339 | 19 m 23 s | 33% |
| 42343 | 15 m 58 s | 27% |
| 42349 | 17 m 25 s | 29% |

About 0.9 cores on average, delivered as **synchronized bursts**: all three sat
near 0% CPU, then all three at ~100% for roughly 7–8 CPU-seconds each, while
this session was running nothing. Between bursts every thread was parked in
`recv`. `sample` during a burst:

```text
Router::dispatch_notification
  → WorkspaceIndex::set_document
    → graph::substrate::index_document_timed
      → extract_all_references / parse_mdast / scan_wiki_links / FrontmatterAst::parse
```

So a burst is a notification-driven re-index of workspace Markdown. Two facts
from the source make the cost plausible:

- `Router::apply_watched_changes` coalesces events **within one notification
  only** (`coalesce_changes`), applies each changed path, then calls
  `refresh_all_diagnostics()` — every open document, once per notification.
  There is no debounce across notifications; `diagnostics.debounce_ms` governs
  edit-driven diagnostics, not watched-file events.
- A worktree of this repository holds 4,751 tracked Markdown files, and agent
  sessions write Markdown constantly (specs, logs, journals).

The binary sampled was the installed `~/.cargo/bin/dmls`, built 2026-09-09. It
is not `HEAD`. Whether `HEAD` behaves the same is unverified.

> **Not established:** what triggers the bursts, or why three servers watching
> three different worktrees burst in lockstep. A shared trigger is implied. The
> worktrees share one `.git` directory and one Zed process, and either could
> fan one event out to all three; neither was tested. Finding the trigger is
> this spec's first task, not an assumption it may build on.

The GitNexus watcher is the repository's tooling too: `gitnexus analyze
--watch`, started by `just gitnexus`, was a three-day-old `node` process at
68–100% of a core on two of the three occasions it was sampled and 0% on the
third. It was not profiled. Whether that is steady-state cost or a reaction to
the same file churn that drives `dmls` is unknown, and it is one watcher per
worktree by design.

Also running, not the repository's doing, listed so nobody rediscovers them: a
macOS `openAndSavePanelService` at ~94% CPU with 15 hours of uptime (cause not
investigated), `fseventsd` at 40%, and Spotlight (`mds`/`mds_stores`) at ~33%
combined. `fseventsd` and Spotlight scale with file churn under
`/Volumes/coding`, which builds in several worktrees produce.

## Scope and design decisions

Three parts, ordered by how much evidence supports acting now.

### 1. Make "expensive test" a CPU-time fact, not a wall-clock impression

Add a developer recipe — not a gate — that answers "is this test over budget,
or was the host busy?" for named tests: run each test binary alone with
`--exact`, report `user + sys` and wall, and record the 1-minute load average
beside them. This is exactly the manual procedure above, made repeatable.

It belongs in the shared `just/` recipes so every package area gets it. Load the
`rust-testing` and `nextest` skills first; the binary-discovery step must use
nextest's own listing, not a glob over `target/debug/deps` (this investigation's
glob picked up `.d` files and an aliased `ls`).

Add to the `rust-testing` skill, in the same change, the rule the evidence
supports: **before treating a `SLOW` as a regression or as a flake, measure its
CPU time alone.** Over ~2 s of CPU in an L1 test is a finding regardless of the
wall-clock result, because a 2.5× slowdown is ordinary on this host. The two
recurring shapes found were a case matrix inside one `#[test]` where every case
composes, and a full `ComposeContext::capture()` inside a loop.

Open question — **should the suite fail on CPU budget instead of wall-clock?**
nextest has no CPU-time timeout, so this would be new tooling. Not recommended
now: the recipe plus the skill rule is the cheap answer to the same question,
and the per-compose cost that makes these tests heavy is being removed at its
source (`2026-09-20-repo-perf`, `2026-09-21-lockfile-provenance-cost`). Revisit
only if overweight tests keep arriving after those land.

### 2. Find and fix the `dmls` re-index storms

Investigation first; the fix follows from what it finds.

1. Reproduce on `HEAD`'s `dmls`, not the 2026-09-09 install. If `HEAD` does not
   burst, the fix is "reinstall", and the remaining work is making a stale
   install noticeable.
2. Identify the trigger. Log, per `didChangeWatchedFiles` notification: event
   count, distinct paths, and the paths themselves at `debug`. Correlate with
   activity in the *other* worktrees and in the shared `.git` directory. Establish
   whether the client sends events for paths outside the server's workspace
   root, and whether `dmls` acts on them.
3. Measure what one burst does: documents re-indexed and diagnostics refreshed
   per notification. `index_document_timed` already times indexing; use it.

Likely remedies, to be chosen by that evidence and not before it:

- Debounce watched-file notifications across a short window, so a burst of N
  notifications costs one re-index pass and one `refresh_all_diagnostics()`.
- Drop events for paths outside the workspace root or outside the configured
  include globs before any work happens.
- Skip re-indexing when a path's content hash is unchanged. DMLS already has a
  rescan-and-rehash fallback (`WatchMode::ServerRescan`), so the hashing exists.
- Refresh diagnostics only for open documents whose link targets intersect the
  changed paths, instead of all of them.

Constraints from the `darkmatter` skill that any remedy must keep: DMLS
validation is passive — no I/O beyond reading the changed document, no
expression execution, no remote fetch — and an open buffer stays authoritative
over disk. Load the `darkmatter` and `lsp` skills before designing this.

This part is Darkmatter package-area work. If the investigation shows it is
larger than a debounce, split it into its own spec under `darkmatter/fixes` and
leave a pointer here.

### 3. Stop concurrent sessions from starving each other

The largest swings (load 33, load 62) came from other agent sessions building
and testing in other worktrees, each assuming it owns 16 cores: nextest at full
test-threads, `cargo` at full jobs. Two such sessions oversubscribe the host 2×
before anything else runs.

This part is **a decision for the owner, not a design**, because every option
trades throughput for predictability and the right trade depends on how the
host is used:

- **A. Cap per-session parallelism.** A lower default `test-threads` and
  `cargo` job count for local runs. The default profile is `test-threads = -2`
  today (all cores but two), and the canonical recipes already export
  `NEXTEST_TEST_THREADS` for the CI ≤4-core exception, so the mechanism exists.
  Simple, always on, and slower for the common case of one session. The `os` skill records hosted-runner sizes and
  the rule against tuning a thread cap without evidence — load it first.
- **B. Adapt to load.** The `just test` recipes read the load average and choose
  `test-threads` from the headroom. Cheap to build, no coordination needed,
  but load average lags by design and two sessions starting together both see
  an idle host.
- **C. Coordinate.** A host-wide jobserver or lock shared across worktrees.
  Correct, and the most machinery; cross-platform behavior (macOS, Linux,
  Windows, WSL2) has to be designed, not assumed.
- **D. Do nothing to scheduling; refuse to trust a loaded result.** The test
  recipes print the load average at start and end, and warn that `SLOW` results
  are unreliable above a threshold. No behavior change, just honesty.

Recommendation: **D now, then B if D proves insufficient.** D is nearly free and
directly addresses the harm in the evidence — a red nobody could interpret. It
also has precedent: the `darkmatter` skill already tells agents to check
`uptime` before treating an HTTP-client timeout cluster as a regression. C is
not justified by one day's observations.

### Out of scope

- Why composes are expensive. That is `2026-09-20-repo-perf` and
  `2026-09-21-lockfile-provenance-cost`; this spec is about measurement and
  load, and would still be needed if composes were free.
- The macOS processes listed under Evidence. They are host hygiene. Record the
  stuck-dialog and Spotlight-exclusion advice in the `os` skill's macOS notes if
  it is not already there; do not build tooling around them.
- CI. Hosted runners are single-tenant.
- Diagnosing the WezTerm L2 stall recorded under "Contention blocked a push".
  It is evidence that load reaches the real-terminal harness, and it belongs to
  the `biscuit-test-harness` area if it recurs; part 3's option D would have
  labeled that red as unreliable, which is this spec's contribution to it.
- Raising the 5 s slow period. It would hide the five overweight tests this
  investigation found along with the noise.

## Acceptance criteria

1. The CPU-time recipe exists in the shared `just/` recipes, is documented in
   the `rust-testing` skill, and reproduces the table under "Contention flips
   results" to within run-to-run variation for any of those tests checked out
   at their pre-fix commits. It works on macOS and Linux; on native Windows it
   either works or says plainly that it does not and why.
2. The `rust-testing` skill states the CPU-before-verdict rule and names the two
   overweight shapes. The `darkmatter` skill's existing "check `uptime`" note
   points at it rather than restating it.
3. The `dmls` trigger is identified and recorded in the implementation log with
   the notification evidence. "Could not reproduce on `HEAD`" is an acceptable
   finding if shown.
4. If a `dmls` remedy ships: an L1 test drives N watched-file notifications for
   one path through the router and asserts one re-index and one diagnostics
   refresh; another asserts that an event for a path outside the workspace root
   does no indexing work. Both assert counts, not timing. `just test` and
   `just lint` pass in `darkmatter/`.
5. After a `dmls` remedy, three idle `dmls` servers on three worktrees of this
   repository, with agent sessions active in each, consume a recorded and
   materially lower CPU share over an hour than the ~0.9 cores measured here.
   Record the number; do not gate on it.
6. Part 3's option is ruled by the owner and the ruling recorded here before any
   scheduling change is implemented. If D: the test recipes print start and end
   load and the warning, on all four environments, without failing the run.
7. No CI matrix cell, run, fixture, or gate is added by any part of this spec.
