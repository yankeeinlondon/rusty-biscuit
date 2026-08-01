---
status: draft
created: 2026-07-31
area: claudine
packages:
  - claudine
  - claudine-cli
  - claudine-gen
depends_on:
  - claudine/fixes/_completed/2026-07-29-windows-paths/spec.md
related:
  - biscuit-file/features/2026-07-31-portable-strings/
---

# Claudine on Windows: Failing Tests and Suite Cost

## Summary

Measured on the Windows host (31.9 GB / 12 logical CPUs, repository on an NTFS
volume) from branch `fix/dark-windows`, 2026-07-31, across
`claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli` and
`claudine-gen` at the L1 tier:

| | before | after the perf fix |
|---|---|---|
| wall clock | 496.7s | **285.4s** |
| slow (>5s) | 93 | **8** |
| flaky | 2 | **0** |
| failed | 148 | 148 |
| timed out | 3 | 3 |
| passed / skipped | 5985 / 13 | 5985 / 13 |

One performance defect is already fixed (see below). **The 148 failures and 3
timeouts are untouched and are the subject of this fix.** They are pre-existing
on this host, not a regression from the perf work: the failing-test sets before
and after were diffed by name and are identical.

`fix/dark-windows` is scoped to Darkmatter, so only the perf fix landed there.
Everything else below is carried forward for the next branch.

## Ownership and dependencies

This umbrella fix depends on `2026-07-29-windows-paths`, which owns permission
matching, sensitive-path classification, absolute allow entries, the shared
comparison boundary, and removal of the interim Windows warning. That
security-sensitive tranche lands first so output normalization cannot mask a
matching defect.

This July 31 fix owns every path-to-text boundary and every file-URI correction,
including system-prompt rendering, completion values, reports, links, and
mixed-separator output. It adopts biscuit-file's implemented portable-string
API for visible text and uses URL-aware conversion for file hyperlinks; neither
representation is used as path identity or for permission comparisons.

## Interim state — already landed

`fc47ab983 perf(claudine): make the expression resolution context capture
demand-driven`.

`composition::document_expression_resolution_context` fell back to
`ComposeContext::capture` when given no prepared snapshot, and the lifecycle
executor calls it **once per evaluated expression argument**. That capture runs
Sniff's repo-wide scan (git, repo, file changes, languages, docs, OS, hardware,
GPU), so a lifecycle action with two arguments paid ~3s before executing an
effect that itself takes 2ms.

Attribution from one loop-control test:

| phase | time |
|---|---|
| fixture setup (`Terminal::default`, `materialized`, engine, guard) | ~2ms total |
| event with an **empty** stack | 349ms |
| …with **one literal** action | 3.30s |
| → argument evaluation | 3.34s |
| → the effect-engine call itself | **2ms** |
| → `LayeredLookup::new(…, resolution_context())` | **1.67s + 1.41s** |

The fallback now captures on demand against the document's directory, which is
what the executor's own `early_binding_context` two functions above already did.
The eager capture was an inconsistency between siblings, not a deliberate
choice.

This is cost-only. It fixed no failing test and is not a Windows fix.

## The 148 failures

Counts are exact (distinct test names). Causes are **sampled, not exhaustive** —
each class below is named from representative panics, so treat the class sizes
as indicative until each test is confirmed.

### By binary

| binary | failures |
|---|---|
| `claudine` (lib unit) | 49 |
| `claudine-cli::bin/claudine` (CLI unit) | 32 |
| `claudine-cli::completion_compose` | 17 |
| `claudine-cli::completion_setter` | 15 |
| `claudine-cli::skills_integration` | 9 |
| `claudine-cli::mcp_cli` | 9 |
| `claudine-cli::compose_schema_cli` | 3 |
| `claudine-cli::completion_sequence` | 3 |
| `completion_inline_compose`, `completion_contract` | 2 each |
| 7 further binaries | 1 each |

The 49 lib failures cluster as: `protect::path::tests` 8,
`stream::path_link::tests` 6, `protect::service::tests` 5,
`permissions::engine::tests` 4, `render::prompt::system::tests` 3,
`config::claude::tests` 3, then a tail of 1–2 across `system_prompt::resolve`,
`permissions::providers::{codex,claude}`, `composition::sequence::task`,
`composition::lifecycle::executor`, `permissions::query`,
`model_catalog::provider_sources`, `messaging::resolve`.

### Class A — path separator (`\` vs `/`), the dominant theme

Evidence:

```
protect::path::tests::absolute_allow_respects_boundary
  assertion failed: is_path_allowed("/var/tmp/file.txt", &allow)

stream::path_link::tests::cwd_preferred_over_home_when_both_could_match
  assertion failed: rendered.contains(">src/main.rs<")

completion_compose::compose_empty_partial_renders_repo_claudine_scope
  repo .claudine prompt must render with .claudine prefix: [".claudine\\prompts\\plan.md"]

completion_setter::setter_at_sigil_triggers_file_completion
  expected spec='docs/spec.md' in ["spec='docs\\spec.md'"]
```

Two different defects hide in this class and must not be conflated:

1. **Rendering** — a path reaches user-facing text or a completion candidate
   with `\` where the product's contract is `/`. The repo already has the remedy:
   `biscuit_file::to_portable_string`. **Darkmatter has adopted it at 56 call
   sites; Claudine has zero.** Adopting it at Claudine's path→text boundaries is
   the bulk of this work.
2. **Matching** — a comparison hardcodes `/` as the segment boundary and returns
   a negative answer on Windows. This is *already specced* in
   [`2026-07-29-windows-paths`](../2026-07-29-windows-paths/spec.md), which
   documents that two of these fail **open** (deny rules and sensitive-path
   protection). `protect::path::tests` (8) and `permissions::engine::tests` (4)
   belong to that spec, not this one.

Sequencing: land the matching fix first (it is a security defect), then the
rendering pass. A rendering change that normalizes `\`→`/` before a matcher
would mask the matching defect rather than fix it.

### Class B — Windows file semantics

```
config::atomic::tests::concurrent_writers_produce_intact_payload
  atomic_write must succeed: Os { code: 5, kind: PermissionDenied,
                                  message: "Access is denied." }
```

One test, 8 panicking threads. `ERROR_ACCESS_DENIED` from concurrent
`atomic_write` is the classic Windows rename-over-open-file / sharing-violation
behavior, which has no Unix analogue. This is a **real product defect on
Windows**, not a test-expectation problem: any concurrent config writer hits it.
Needs a retry-on-sharing-violation or an exclusive-share open, and is the one
item here that should not be deferred behind cosmetics.

### Class C — unclassified

```
mcp_cli::effective_defaults_repo_replaces_user
  called `Option::unwrap()` on a `None` value    (mcp_cli.rs:585:69)
```

`skills_integration` (9) reports empty result sets ("No skills matching: a,
-alpha"), which is consistent with path-driven discovery finding nothing — i.e.
probably Class A downstream — but that is inference, not measurement. Confirm
before folding it in.

## The 3 timeouts

`claudine-gen::drift` — `committed_catalog_matches_regenerated_inputs`,
`committed_data_matches_regenerated_inputs`,
`committed_families_match_regenerated_inputs`.

All three hit nextest's 30s termination ceiling on **all four** attempts
(`retries = 3`), so each burns ~120s of the suite. They regenerate every
provider's committed artifacts and compare byte-for-byte, so they are
genuinely heavy — but whether 30s is inherent to the workload on this host, or
another uncached repo scan of the kind the perf fix removed, **has not been
profiled**. Profile before deciding between "make it faster" and "give it a
`slow-timeout` override in `.config/nextest.toml`".

Note `committed_vocabulary_matches_regenerated_inputs` and
`committed_signals_match_regenerated_inputs` in the same binary pass at ~9s, so
the ceiling is close for the whole family.

## The 8 remaining slow tests

| test | marker |
|---|---|
| `claudine-gen::drift` × 3 (the timeouts above) | >25s |
| `claudine-cli::inline_compose_hash inline_compose_writes_hash_that_passes_md_diff` | >20s |
| `claudine-gen::vocabulary build_vocabulary_is_deterministic_and_well_formed` | >15s |
| `claudine-cli::wrap_ctrl_c_windows ctrl_c_terminates_wrapped_child_on_windows` | >15s |
| `claudine-gen::agent_errors_check all_archived_seeds_match_…` | >5s |
| `claudine-gen::vocabulary every_research_vocabulary_projects_to_runtime_strings` | >5s |
| `claudine composition::preflight::tests::*` × 3 | >5s |

`wrap_ctrl_c_windows` is Windows-specific by construction and may be legitimately
slow (it waits on real process termination); check before optimizing.

## Working notes for the next branch

**Build parallelism.** `cargo build -p claudine-cli --tests` at the default
`-j 12` exhausts this host's RAM plus its 6.6 GB pagefile — ~2000 tests in one
binary linking against large rlibs. rustc/link.exe then die in ways Cargo
reports as *compile errors in your source*: `E0463: can't find crate for
claudine`, `link.exe failed: exit code 0xc000012d` (STATUS_COMMITMENT_LIMIT),
and, when a build is killed mid-flight, nonsense like "no global memory
allocator found". **Always `cargo build -p <pkg> --tests -j 4` first.**
`cargo nextest run -j N` sets *test-execution* concurrency only — its build step
still uses the default `-j`, which is how this trap gets sprung twice in a row.

**Measurement method that worked.** Reading the code did not find the perf
defect; four rounds of instrument-and-bisect did, in this order: time the test in
isolation (10.4s single-threaded ⇒ fixed cost, not contention) → time a trivial
test in the same binary (0.3s ⇒ the cost is in the test body, not process
startup) → empty stack vs one literal action (349ms vs 3.30s ⇒ per-action) →
`eprintln!` timers inside the executor (⇒ `resolution_context()`). Each step
halved the search space and cost one `-j 4` rebuild.

**Do not trust `tail` on a suite run.** Two analyses in the source session were
wrong because the run was piped through `tail -400`, which keeps only the last
binary's output. Redirect the whole run to a file, then query it.

## Success criteria

1. `just test` for the claudine area is green on Windows. A remaining test may
   be `#[cfg(unix)]`-gated only when it exercises a genuinely Unix-only
   facility, with that facility named in the reason; Windows product-path
   failures must not be gated, ignored, or moved out of L1.
2. No test exceeds nextest's 30s termination ceiling on this host; the three
   `drift` tests either run under it or carry a documented override.
3. `protect::path` and `permissions::engine` matching defects are fixed per
   `2026-07-29-windows-paths`, and its interim
   `warn_windows_path_matching_is_broken` instrumentation is removed.
4. `config::atomic` survives concurrent writers on Windows.
5. Claudine's path→text boundaries render through
   `biscuit_file::to_portable_string`, matching Darkmatter's adoption.
6. Host-independent matcher and renderer contracts remain ungated, and the
   constrained build plus full L1 gates pass on native
   `x86_64-pc-windows-msvc` Windows. Linux, macOS, xwin, and GNU-target checks
   are deferred portability follow-ups rather than completion authorities.

`rendezvous-daemon` and its DuckDB session-log pipeline remain required on
every platform. Windows target exclusions are not an acceptable portability
shortcut. Native Windows live-daemon tests pass 5/5, and the
`x86_64-pc-windows-gnu` Cargo graph contains
`rendezvous-daemon -> duckdb -> libduckdb-sys`; this graph evidence does not
claim that the unavailable GNU compiler completed a build.

## Out of scope

- The remaining Darkmatter slow tests and the uncached
  `sniff::detect_repo_structure` scan (~5.4s per schema-file resolution, no
  process-level reuse). That is a Darkmatter/Sniff decision with a staleness
  trade-off for the long-running DMLS server, deliberately left open.
- L2/L3/browser/real tiers — this spec measured L1 only.
