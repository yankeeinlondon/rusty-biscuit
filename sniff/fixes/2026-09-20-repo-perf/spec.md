---
area: sniff
status: planned
created: 2026-09-20
reviewed: true
reviewed_by: "codex/gpt-6-astra"
reviewed_on: "2026-09-20"
owner: Ken Snyder <ken@ken.net>
origin: darkmatter compose-floor investigation, 2026-09-20
packages:
    - sniff
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
human_review: true
human_review_items:
    - |-
        **Accept or change the parallel walk, given a measured slowdown on small folders**

        **Why this must be decided before Phase 5.** Phase 5 tests the change on
        every operating system and assumes the implementation is final. The plan
        says that if small folders or concurrent use get noticeably slower, the
        work stops and you decide. That condition was met. If you choose a
        different design, Phase 5 would have to be run again on the new code.

        **What was measured on your Mac.** The host was heavily loaded by other
        sessions. The full numbers are in
        `sniff/fixes/2026-09-20-repo-perf/evidence/after/README.md`.

        - On this monorepo, finding the nested projects is about 2.2× faster
          end to end: 79 → 36 ms (release) and 244 → 107 ms (debug). The folder
          scan alone is about 3× faster.
        - On a tiny folder (8 files), every call is about 3 ms slower: 0.5 →
          3.4 ms in release, 5–7× the old cost. Running such calls one after
          another gives about 84% fewer per second.
        - With 14 calls at once (your normal test concurrency), overall
          throughput is unchanged. For large folders, though, the slowest 5% of
          calls take 42% longer, and the process uses 15% more CPU and up to 31%
          more memory.
        - The cause is inside the `ignore` library. Each parallel scan starts
          12 new threads, and idle threads check for more work every 1 ms. That
          gives every scan a floor of about 3 ms, however small the folder.
          With 4 threads the floor is about 1.7 ms. With 1 thread there is no
          floor, but also no speedup.
        - Who pays the 3 ms: every caller that asks only for repository
          structure. That includes `sniff repo` commands, claudine completion and
          composition, darkmatter compose, and sniff recent-commits.

        **Options**

        1. **Accept as implemented** (12 threads, the library default).
           - Pros:
             - biggest gain on real repositories;
             - matches the spec exactly;
             - no more work: Phase 5 proceeds now;
             - the 3 ms is far below what a person notices on a command line,
               and test suites run at 14-way concurrency, where throughput was
               unchanged.
           - Cons:
             - tiny folders are 5–7× slower per call;
             - code that calls this in a tight loop on small folders slows
               noticeably;
             - under heavy concurrency, large-folder tail latency and memory
               use rise.
        2. **Use a fixed, smaller thread count, such as 4.**
           - Pros:
             - keeps most of the gain: the scan takes 23 ms against 19 ms with
               12 threads;
             - roughly halves the small-folder penalty, to about 1.5 ms;
             - starts far fewer threads when many calls run at once (56
               against 168 for 14 calls).
           - Cons:
             - still about 7× slower on a tiny folder;
             - the "4" is tuned on one machine;
             - the spec says to keep the library default, so the spec must be
               amended;
             - the measurements (this phase) must be rerun before Phase 5.
        3. **Scan serially first, and switch to parallel only once the folder
           proves large.**
           - Pros: removes the small-folder penalty and keeps the large-folder
             gain.
           - Cons:
             - the spec explicitly puts "adaptive scheduling" out of scope, so
               this needs a new spec or phase;
             - more code, with a new threshold to choose;
             - the equivalence tests must be extended.
        4. **Revert to the serial scan.**
           - Pros: no risk at all; nothing gets slower.
           - Cons: gives up 44 ms (release) and 137 ms (debug) per call on real
             repositories, which was the whole purpose of the fix.

        **Recommendation: option 1 (accept as implemented).** The 3 ms cost is
        real but small in absolute terms. Throughput at your normal test
        concurrency did not change, and real repositories, where the time
        actually hurt, get a 2× end-to-end gain. Options 2 and 3 each change the
        agreed design on the strength of one noisy machine, and they add
        re-measurement or a new spec. If you are worried specifically about
        many large scans running at once (the tail-latency and memory rise),
        option 2 is the reasonable fallback. Option 3 is better treated as a
        separate future fix.
message_to_agent: |-
    Phase 4 (claude/default) is done (measurement only; no source files changed, uncommitted docs/evidence
    in the fix directory). It triggered plan ruling R7: tiny-tree single-request latency regressed
    ~5-7x (+~3 ms per detect_repo_structure, caused by ignore 0.4.25's 1 ms idle-worker sleep poll plus
    12 fresh threads per run; see evidence/after/worker-diagnostics.md). The spec's human_review_items
    asks the author to choose: (1) accept as implemented, (2) fixed smaller worker cap, (3) adaptive
    serial->parallel, (4) revert. Do NOT start Phase 5 until the author has answered. If they pick 1,
    Phase 5 proceeds unchanged. If they pick 2 or 3, the production code changes: rerun the Phase 3
    parity suite and the Phase 4 campaign (reuse evidence/after/bracket.sh.txt and
    probes/probes.sh.txt; build each side in a fresh fixed-path worktree and measure against the
    fix-sniff checkout as the corpus) before Phase 5.
    What Phase 5 still needs from earlier phases:
    (a) Run the OS legs with `just cross-check sniff --os <os> nested::tests`, using plain substring
    filters (the remote shell breaks -E quoting) and --no-capture to see SKIP lines. Native Windows
    (20/20) and Linux (22/22) passed in Phase 3. WSL2 has never been run.
    (b) Lint with `--features remote,bench-internals`, because `--features network` alone has
    pre-existing dead-code errors.
    (c) Phase 4 gates: just test 2874 passed / 32 skipped; just lint and strict clippy are clean.
---

# Parallelize the nested-marker walk

## Outcome

Reduce the latency of fallback nested-workspace discovery by replacing the
serial walker in `sniff/lib/src/filesystem/repo/nested.rs` with `ignore`'s
parallel walker. Preserve candidate contents and ordering, request-scoped
observation reuse, and work accounting on macOS, Linux, native Windows, and
WSL2. No public API, serialized output, CLI, or observation-timing change is
intended.

The original investigation suggests a roughly 3–6× improvement in the **walk
component** on this monorepo. That is a measurement to reproduce, not a promised
speedup for repository detection, compose, or other machines.

## Problem and existing evidence

`walk_for_nested_markers` currently uses `WalkBuilder::build()`, collects all
successful non-directory entries into a `Vec<PathBuf>`, then passes them to
`candidates_from_marker_paths`. Only marker paths contribute candidates.

The originating investigation reported these results on this repository
(`aarch64-apple-darwin`, 16 cores), with the same ignore settings, directory
pruning, and marker predicate:

| Strategy | Debug | Release |
|---|---|---|
| Serial, collect all then filter (current implementation) | 172 ms | 61 ms |
| Serial, filter during the walk | 170 ms | 60 ms |
| Parallel, filter during the walk | 30 ms | 18 ms |

All three reportedly found the same 92 marker files among 11,290 paths. These
are historical observations, not fixture expectations. The draft did not
include the benchmark harness, sample distribution, cache state, or actual
worker count; capture those with implementation evidence before treating the
figures as a reproducible baseline. This review has not rerun the measurements.

These measurements suggest traversal dominates this workload and filtering
alone gives little latency improvement. They do not establish that allocation
is free or that parallel execution reduces CPU time or filesystem work.

### Why this matters beyond Sniff

Darkmatter fixes an ambient repository observation once per request. The tests
`the_observation_is_fixed_at_request_creation` and
`a_child_reads_the_observation_fixed_at_request_creation` in
`darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs` pin that behavior.
Older constructors have a separate root-pipeline-entry fallback contract,
pinned by `an_older_constructor_request_is_fixed_at_the_root_entry_not_by_the_child`.
Keep all these boundaries intact.

Ambient compose requests that reach this fallback on this checkout pay its
walk cost. This does not apply to every compose test: caller-supplied fixed
contexts and supplied marker evidence can avoid ambient discovery or this
walk. The investigation's 959 tests taking at least 200 ms and 68% suite-cost
figure are motivation, not proof that all that time belongs to this function;
summed test elapsed time is not automatically CPU time.

> **Review note:** Preserve eager observation, but do not infer that parallel
> walking is the only possible optimization. This fix targets the measured
> fallback bottleneck without redesigning observation ownership or timing.

## Scope and design decisions

Production changes belong in `sniff/lib/src/filesystem/repo/nested.rs`, with
focused regression tests and performance evidence. `ignore` is already a
**direct** dependency of `sniff`; no dependency or feature change is needed.

1. Use `build_parallel()` for the fallback traversal. Preserve the settings
   `hidden(false)`, `git_ignore(true)`, `git_global(true)`, `git_exclude(true)`,
   and the existing `filter_entry` directory prune. Preserve remaining defaults,
   including `.ignore` and parent-ignore handling, Git-repository requirements
   for Git ignore rules, and symlink traversal behavior. Equal builder settings
   are necessary but do not replace serial/parallel parity tests.
2. In the callback, continue past walk errors, exclude entries whose available
   file type is a directory, and use the existing `is_nested_marker_path`
   predicate before allocating an owned path. Do not replace the existing
   non-directory condition with `is_file()`: that would narrow symlink and
   unknown-file-type behavior. Do not add per-entry metadata or content reads.
3. Accumulate marker paths in a per-worker `Vec<PathBuf>` and merge each batch
   once when that visitor finishes, using the existing `ManifestIndex` worker
   pattern as a guide. A shared mutex must not serialize every visited entry.
   Join all workers and finish all merges before projecting candidates. Retain
   all matching markers; no inventory cap, early exit, or truncation applies.
4. Create a `performance::WorkerCollector::inherit()` on the spawning thread
   for each visitor, activate it before work in that visitor's callback, and
   let it drop on the executing thread to flush counters. Follow the existing
   pattern in `filesystem/repo/manifest_index.rs`. The visitor used for initial
   root/error handling must also finish cleanly.
5. Keep `FS_READ_DIRS` and `REPO_NESTED_MARKER_WALKS` increments at the caller's
   walk chokepoint, once per invocation, including an empty or missing root.
   Their current meaning is a logical walk invocation, not physical directory
   reads. Add no counter merely to make the optimization look like less work.
6. Use the locked `ignore` default worker policy. In `ignore` 0.4.25 it is
   available parallelism capped at 12, falling back to one. Do not equate a
   16-core host with 16 walker workers. This follows existing parallel walkers
   without adding a public tuning knob or a new pool. Verify small trees and
   concurrent callers because this is a per-walk cap, not a process-wide cap.
7. Call `candidates_from_marker_paths` once after collection. Keep grouping,
   standards matching/deduplication, detector dispatch, and manifest parsing
   unchanged and outside the parallel callbacks.

The `RepoEvidence::nested_markers` path remains unchanged: `Some(markers)`,
including `Some(&[])`, reuses supplied evidence and starts no fallback walk.
No fresh discovery, persistent cache, Git subprocess, network call, or
cross-request state is introduced. Callers, request tiers, and the shared
observation walker are outside this fix's production scope.

> **Review note:** Parallelism trades thread startup and potentially greater
> instantaneous resource use for latency. Local buffers avoid per-entry lock
> contention; the existing capped worker policy avoids an unbounded pool. If
> representative small-tree or concurrent-request benchmarks regress materially,
> revisit this decision before implementation approval rather than silently
> widening scope to a global pool or adaptive scheduler.

## Compatibility and determinism

Preserve the current best-effort behavior: discard individual walk errors and
continue with reachable entries; a missing root yields no candidates. No new
error return or diagnostics are introduced. An immutable fixture is required
for parity assertions; neither serial nor parallel traversal provides an atomic
snapshot of a concurrently changing filesystem.

Preserve these details explicitly:

- Root-level markers do not create nested candidates.
- Gitignored markers remain excluded where Git ignore rules apply; untracked
  but unignored markers remain discoverable. Non-Git directories still work.
- Hidden directories remain eligible, while the existing named-directory prune
  remains authoritative. Marker-named directories do not themselves constitute
  evidence, but their permitted descendants can contain markers.
- Preserve the existing non-directory symlink admission and link-following
  defaults; do not follow nested directory links or canonicalize collected
  paths as part of this optimization. Include a symlinked starting root in
  serial/parallel comparison where supported, since root handling can differ
  from descendant handling.
- Fixed marker names remain ASCII case-insensitive on native Windows and
  byte-exact on Unix targets, including macOS and WSL2. `.sln` and `.slnx`
  suffixes remain case-sensitive everywhere. Reuse the predicate rather than
  duplicating this platform logic. Non-Unicode basenames are not markers.

`candidates_from_marker_paths` groups by parent root, deduplicates standards,
sorts standards by `spec().id`, and sorts candidates by their `PathBuf` roots.
No additional sort of collected marker paths is needed. Assert equal ordered
`(root, matched_standards)` values, not “byte-identical `Vec<Candidate>`”:
`Candidate` has no serialization contract, and allocation addresses are
irrelevant. Ordering equivalence is for the same input and target platform;
it does not promise identical native path ordering across operating systems.

The ignored-file and marker-directory differences described in the current
function's doc comment are already established behavior relative to an older
probe loop. They are **not new intentional changes** in this fix. Review the
edited function's docs and comments for drift, including the ambiguous link to
“the spec”; preserve useful contract history without implying this spec
introduces those deltas.

## Acceptance criteria

1. Add a test-only serial reference that retains the pre-change walker settings
   and non-directory filtering, using the unchanged candidate projection.
   Compare complete ordered candidate fields with the parallel result, and
   assert independently expected candidates so a shared projection bug cannot
   make both sides pass. Keep existing root-marker and supplied-evidence tests.
2. Use disposable fixtures covering multiple sibling directories and depths,
   empty/root-only trees, every fixed marker name, both solution suffixes,
   multiple markers mapping to the same standard, and one marker mapping to
   multiple standards. Repeat the wide-tree comparison at least 20 times;
   exercise a one-worker and a multi-worker configuration through private test
   setup without exposing a new public option.
3. Cover Git and non-Git roots, an untracked unignored marker, `.gitignore`
   exclusion and negation, `.ignore`, controlled global excludes and
   `.git/info/exclude`, hidden directories, pruned directories, marker-named
   directories, missing roots, platform case rules, and supported symlink
   cases. Isolate host Git/ignore configuration using repository test utilities;
   do not mutate process environment concurrently. Permission-error tests must
   account for privilege-dependent behavior rather than assume `chmod` works
   identically everywhere. Do not rely on racy deletion for deterministic tests.
4. Under a fresh collector, direct fallback invocation reports exactly one
   `FS_READ_DIRS` and one `REPO_NESTED_MARKER_WALKS`. Supplied-evidence discovery,
   including empty evidence, reports zero fallback walks. Account separately
   for any legitimate detector work instead of asserting all discovery work is
   zero. Verify worker propagation with a test-only callback counter whose
   expected total is known after the walk returns; do not add production
   per-entry counters solely for this test. An absent counter means zero.
5. Compare complete detection/topology output before and after on a controlled
   nested-workspace fixture and on this checkout as a manual performance
   corpus. Checkout path/marker counts are recorded evidence, not portable CI
   assertions. Include the relevant existing request-cost/reuse tests.
6. `just test` and `just lint` pass in `sniff/`, retaining the recipe's `remote`
   feature coverage. Exercise the focused parity tests on macOS, Linux, native
   Windows, and WSL2 through existing test workflows; reuse qualifying evidence
   where available. Cross-compilation alone does not prove runtime parity.
   This fix adds no CI matrix cells or timing gates.
7. `just test` in `darkmatter/` passes, including the three observation-boundary
   tests named above. Report the actual selected and passed tests; the draft's
   historical 8,329 passing count is not a fixed acceptance condition. Do not
   weaken those tests or change observation timing to meet a performance goal.

## Performance verification

Capture before/after evidence using the same checkout contents, root spelling,
lockfile, feature set, build profile, host, ignore configuration, and harness.
Record commits plus any uncommitted source changes, Rust version, OS, available
parallelism, worker policy, cache treatment, and test-runner concurrency. Keep
fixture creation and compilation outside timed regions. Preserve commands and
raw samples alongside the implementation's review evidence.

Measure the isolated fallback walk and public
`detect_repo_structure(<repo root>)` separately in debug and release. Use at
least three warmups and 20 measured samples per case, report median and range,
and alternate/bracket baseline and changed runs to expose drift. Label warmed
filesystem-cache measurements as such; do not claim cold-cache results without
controlling that condition. Verify that the measured structure request actually
enters the fallback rather than reusing supplied evidence.

Compare stable work counters separately from timing, including fallback walks,
manifest reads, and metadata probes. The primary mechanism is concurrent
execution of the same traversal and retention of fewer paths, not fewer logical
walks. Unchanged counters are expected; disappearing worker counts are a defect,
not an optimization. Measure a tiny tree and concurrent detections at the normal
local test-runner concurrency, recording throughput and resource use as well as
single-request latency. Report regressions and uncertainty instead of hiding
them behind the large-checkout average.

The historical targets are approximately 172 → 30 ms debug and 61 → 18 ms
release for the walk alone. The draft attributed about 60 ms of a 232 ms debug
`detect_repo_structure` call to other work; remeasure that attribution. A gain
must exceed observed run-to-run variation on the representative checkout; a
fixed 3–6× ratio is not a portable gate. Do not report the walk's speedup as the
command's speedup or infer a compose-suite speedup without measuring that suite.

## Alternatives and open questions

No major API or observation-design decision remains open for this bounded fix.
Performance validation of the selected worker policy remains required. If it
shows a material regression, bring that evidence back for review before adding
adaptive or shared-pool scheduling.

A Git-index-only fast path is out of scope because it misses untracked,
unignored markers and needs a filesystem fallback for non-Git roots. Combining
index and untracked discovery is possible but adds ignore/parity complexity and
does not eliminate traversal. The draft's `git ls-files --cached --others`
comparison is not a semantic oracle: without matching exclude options and the
Sniff prune policy, it does not guarantee the same paths, and tracked ignored
or missing files further distinguish index enumeration from a live walk. No
additional 3× index-based speedup is established by the supplied evidence.
