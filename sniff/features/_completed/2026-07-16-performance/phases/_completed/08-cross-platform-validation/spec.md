---
sub-spec: true
depends-on:
  - ../01-work-accounting/spec.md
  - ../02-reuse-and-scope/spec.md
  - ../03-observation-index/spec.md
  - ../04-package-enrichment-and-ownership/spec.md
  - ../05-git-observation/spec.md
  - ../06-remote-network-and-subprocess/spec.md
  - ../07-profile-guided-cleanup/spec.md
  - ../../../spec.md
phase: 8
status: complete
date: 2026-07-17
---

# Phase 8 — Documentation, cross-platform CI, and completion

Close the umbrella feature: document what the previous seven phases actually built, correct the
drift the reviews found, give the work counters ongoing cross-platform visibility, and record the
final evidence against Phase 1.

This phase writes **no production behavior**. Its diff is documentation, comments, and CI. That is
deliberate: a behavior change smuggled into the completion phase would have no phase-level review of
its own, and the umbrella spec's own rule is that "any additional behavior or schema change requires
a separate review and must not be smuggled in as a performance optimization."

## Scope

| In | Out |
|---|---|
| Architecture doc, library/CLI READMEs, rustdoc, benchmark manifest, `.claude/skills/sniff/` | Any production code change |
| The six named drift corrections | Drift outside the named list (recorded below, not fixed) |
| Cross-platform work-count CI | A wall-clock regression gate |
| Final counter table vs. Phase 1 | Re-running Criterion for a verdict (see "Timing evidence") |
| Sub-spec lifecycle moves | Moving the umbrella feature (see "Completion boundary") |

## Drift corrections

The umbrella spec names six. **Two were already corrected by the phase that owned the behavior** —
which is the documentation-maintenance rule working as intended, not an omission here.

| Drift | State on entry | Action |
|---|---|---|
| `.editorconfig` lookup semantics | **Live.** `detect_formatting`'s docblock claimed it "traverses parent directories until a root configuration is found or the filesystem root is reached"; `find_editorconfig` checks the given directory and nothing else. | Fixed. Code is authoritative. |
| The unsupported general "10–50×" claim | **Live** in 6 places. | Fixed. See below. |
| NTP timeout/platform wording | **Live.** `os/time.rs` claimed a 5-second timeout on all three platforms; the architecture doc and skill claimed "up to 10s (Linux `timedatectl`)". Actual bound is `process::timeouts::NTP` = **3s**, and Linux `timedatectl` reports local daemon state without a network round trip — the 10s figure described a cost that never existed on that path. | Fixed. |
| `filter_inventory` copy/sharing language | **Already correct.** Phase 2 rewrote it: the doc states the no-narrowing case shares through the `Arc` and a narrowing filter copies. | Verified, no change. |
| Capped inventory determinism | **Already correct.** Phase 2 rewrote `file_types/model.rs` to state ordering is deterministic while a truncated subset is not. | Verified, no change. |
| 375-package benchmark name/count | **Already correct.** Phase 1 renamed the `huge_500_packages` IDs to `huge_375_packages`. | Verified; added the "never compare against an archived `huge_500` result" note to the bench manifest. |

### Historical Phase 8 structure/full evidence

The umbrella spec asked for the claim to be "replaced with qualified evidence" on the strength of a
2.4× measurement. The counters say the honest number is closer to **1×** on the fixture in question:

| Counter, 375-package fixture | `structure()` | `full()` |
|---|---:|---:|
| `filesystem.io.metadata_probes` | 13,274 | 13,275 |
| `filesystem.io.read_dirs` | 702 | 701 |
| `filesystem.repo.manifest_parses` | 454 | 454 |
| `filesystem.repo.package_enrichments` | 300 | 300 |
| `filesystem.file_inventory.files_accepted` | — | 3,755 |

At the Phase 8 capture, structure mode recorded the *same discovery work* as full mode. Full's only
material addition was classifying the inventory. Two reasons explained that historical result:

1. **R5.6 was blocked.** Structure mode was supposed to skip enrichment; it did not, and Phase 4
   stopped for review rather than ship a change that silently empties `sniff repo test-runner`,
   `dependencies`, and `package-manager`. Until that lands, structure's only saving is inventory.
2. **Structure pays a walk full mode does not.** It skips the manifest index and therefore falls back
   to `walk_for_nested_markers` (`filesystem.repo.nested_marker_walks: 1`), while full reuses the
   observation index.

So the claim was not merely imprecise — it pointed the wrong way about where the cost was at that
phase boundary. It was replaced
in all six sites with the counter comparison plus the workload dependence (the ratio grows with
files-per-package and collapses toward 1 on package-dense, file-sparse trees; `_huge` is 375
packages × ~10 files, which is exactly the converging shape).

**A timing ratio was deliberately not published.** Three sequential runs put structure *slower* than
full (223/144, 138/112, 124/100 ms). That is not evidence structure is slower: `work_counts` runs
structure first over the same fixture, so full reads a page cache structure just warmed, and the
host sat at load 87/16 cores. The confound is recorded in the bench manifest so the next reader does
not "discover" it as a finding.

### Drift found but not fixed

`benches/README.md` and the architecture doc say `ProgramsInfo::detect()` fans out over **8**
categories; the skill and the type say **9** (test runners were added later). Real drift, but it
belongs to the phase that owns program detection, not to a performance-completion phase whose diff
is supposed to contain no behavior. Recorded here so it is not lost.

## Historical Phase 8 counters vs. Phase 1

`cargo run -p sniff --release --example work_counts`, same host and fixtures as Phases 1–7.

These tables preserve the Phase 8 measurement record. They predate the later shallow-structure,
manifest-store, and ownership-index work and must not be used as current structure-mode bounds.
Full-mode and Git rows remain the final Phase 8 baseline for the work they describe.

**`staged_filesystem_full_all_stages`** (`FilesystemRequest::new()`):

| counter | Phase 1 | Phase 8 | Δ |
|---|---:|---:|---:|
| `filesystem.walk.entries_visited` | 638 | 638 | — |
| `filesystem.file_inventory.files_accepted` | 395 | 395 | — |
| `filesystem.docs.documents_parsed` | 182 | 182 | — |
| `filesystem.io.metadata_probes` | 7,885 | **4,075** | **−48%** |
| `filesystem.io.read_dirs` | 422 | **211** | **−50%** |
| `filesystem.io.file_opens` | 370 | 279 | −25% ⚠ |
| `filesystem.io.canonicalizations` | 271 | 271 | — |
| `filesystem.repo.manifest_parses` | 214 | **124** | **−42%** |
| `filesystem.repo.package_enrichments` | 180 | **90** | **−50%** |

**`repo_full_huge_375_packages`** (`RepoRequest::full()`):

| counter | Phase 1 | Phase 8 | Δ |
|---|---:|---:|---:|
| `filesystem.walk.entries_visited` | 4,685 | 4,685 | — |
| `filesystem.walk.walks_started` | 1 | 1 | — |
| `filesystem.file_inventory.files_accepted` | 3,755 | 3,755 | — |
| `filesystem.io.metadata_probes` | 25,975 | **13,275** | **−49%** |
| `filesystem.io.read_dirs` | 1,403 | **701** | **−50%** |
| `filesystem.io.file_opens` | 758 | 708 | −7% ⚠ |
| `filesystem.io.bytes_read` | 47,708 | 50,548 | +6% ⚠ |
| `filesystem.io.canonicalizations` | 600 | 600 | — |
| `filesystem.repo.manifest_parses` | 754 | **454** | **−40%** |
| `filesystem.repo.package_enrichments` | 600 | **300** | **−50%** |

**Git status, 100 dirty files:**

| counter | P1 `full()` | P8 `full()` | P1 `deep()` | P8 `deep()` |
|---|---:|---:|---:|---:|
| `git.status_walks` | 1 | 1 | 1 | 1 |
| `git.blob_loads` | 200 | 200 | 400 | **200** |
| `git.file_diffs` | 100 | 100 | 100 | 100 ⚠ |
| `git.ref_walks` | 2 | 2 | 3 | 3 |
| `git.repository_discoveries` | 1 | 1 | 1 | 1 |

**Historical unchanged cases (drift brackets):** `staged_filesystem_summary_git_plus_repo` and
`repo_structure_huge_375_packages` are **byte-identical to Phase 1 on every counter**. Both are
structure-mode, and R5.6 — the only requirement that would have moved them — was blocked. Their
stability is what makes the deltas above readable as real rather than as harness drift.

### ⚠ Counters that cannot be compared naively

- **`file_opens`/`bytes_read`**: Phase 1 **under-reports** both for every case that built a manifest
  index, because `ManifestIndex::build`'s `build_parallel` workers carried no `WorkerCollector` and
  silently recorded zero. Phase 3 fixed the instrumentation. So `bytes_read` "rising" 6% is the
  counter getting *honest*, not work being added, and the true `file_opens` reduction is larger than
  −7%. Compare against the Phase 3/4 tables, not these.
- **`git.file_diffs`**: under-reports before Phase 5 — `text_hunks` ran a full
  `diff_with_slider_heuristics` per patch side without incrementing anything, so every
  `include_diffs` case ran twice the diffs the counter admitted. Phase 5 collapsed stats+patch onto
  one diff per side, so the counter **does not move while the real work halves**. `git.blob_loads`
  (deep: 400 → 200) is the counter that shows R8.

### Counter classes with no row here

The umbrella asks the table to cover subprocesses/timeouts and HTTP operations. `work_counts`'s
fixtures are local synthetic trees: they spawn no children and make no requests, so those counters
are legitimately absent (an absent counter means zero). Their bounds are asserted in-process by the
Phase 6 tests instead — batched systemd counts against an argv-logging `systemctl` shim, `expect(1)`
wiremock bounds per provider, and the `expect(0)` second-endpoint WAN assertion. That is stronger
evidence than a fixture row would be, because it fails the build rather than printing a number.

### Prediction that did not hold

Phase 4's sub-spec projected `filesystem.io.canonicalizations` would fall "with the enrichment
halving" (600 → 300). It did not: it is 600, unchanged from Phase 1, while enrichments did halve.
Not a regression — no case got worse — but the mechanism was mis-modeled, and canonicalization was
not driven per-enrichment. The prediction remains part of the historical table rather than being
retroactively rewritten. Current source implements R6.2/R6.4 with a request-scoped,
deepest-component-prefix `PackageOwnershipIndex`; its hot-lookup test requires zero
canonicalizations.

## Timing evidence

No Criterion verdict is published by this phase, and the deferrals Phases 3, 5, and 6 made "to
Phase 8" are **not discharged here**. The host that ran this phase sat at load **87 on 16 cores** —
the exact condition Phase 3 characterized when a byte-identical case reported +330%. Re-running the
suite here would produce another number nobody should act on, and publishing it would launder host
noise into the completion record.

This is not a gap in the evidence, because timings were never this feature's acceptance criteria:
the umbrella spec states performance acceptance "is primarily structural", and every requirement
R2–R13 is discharged by a counter or a test. The scheduled CI matrix added by this phase is the
durable answer — it collects work counts per OS on a runner class that can be compared with itself
over time, which a one-off run on a loaded dev box never could.

### Post-review workload coverage

Review cycle 3 added the specified setup-excluded Criterion workload families for deep/wide
formatting-only detection, package-scoped Git plus inventory, integrated versus standalone nested
observation, mixed ecosystems at 100/500/2,000 packages, inventory saturation, document ownership,
dirty-file sizes, divergent worktrees, remote containment, sparse path history, filesystem case
behavior, and a real provider backed by wiremock. Work-counter tests remain the acceptance bounds;
no timing verdict was collected on the loaded host.

The large synthetic service-listing workload was deferred at review cycle 3 for an API-surface
reason — the public service API observes the host, while parser and injected-command seams are
crate-private — and **review cycle 4 resolved it**. The `bench-internals` feature
(`sniff/lib/Cargo.toml`) gates a doc-hidden synthetic systemd fixture, so the production API is
unchanged when the feature is off. `register_service_shapes`
(`sniff/lib/benches/cases/workload_matrix.rs`) registers deterministic `workloads_service_listing/500`
and `/2000` workloads that drive the production listing parser, running-service selection, 128-unit
chunk builder, runner dispatch, show-block parser, and PID projection; fixture construction and
per-iteration cursor setup sit outside Criterion's timed section.

The acceptance bound is still structural, not a timing verdict:
`large_service_workloads_preserve_cardinality_and_chunk_bounds` pins output cardinality and
`1 + ceil(N / 128)` runner calls for both sizes, and
`pid_enrichment_costs_one_subprocess_per_chunk_not_per_service` maps the same bound to the stable
`process.spawns` counter through the real bounded runner. Benchmark compilation is verified locally;
no wall timing from a loaded implementation host is recorded or claimed for these rows. See
[`sniff/lib/benches/README.md`](../../../../../lib/benches/README.md) for the workload-to-counter-test
mapping.

## CI

| Job | Trigger | Runs | Retention |
|---|---|---|---|
| `sniff-bench` | PR (`sniff/**`, workspace manifests, this workflow) + dispatch | Narrow Criterion subset, **Linux only**, artifact-only | 14 days |
| `sniff-work-counts` | **schedule (nightly 04:00)** + dispatch + PR | `work_counts` table, **macOS + Linux + Windows**, `fail-fast: false` | 90 days |

Design decisions:

- **Same-OS comparison is enforced by construction, not by a threshold.** Artifacts are named per OS
  and the Criterion baseline was renamed `ci` → `ci-linux`, so a macOS run cannot be diffed against a
  Linux one by accident.
- **No wall-clock gate.** A percentage gate needs stable runners or a characterized noise band; we
  have neither, and the umbrella spec forbids "one wall-clock threshold across hosted macOS, Linux,
  and Windows runners". A work-count regression is the actionable signal; a timing swing is a prompt
  to look.
- **Counters outlive reports** — hence 90 days vs. 14.
- **Cross-OS counter deltas are not regressions.** Path, case, and ignore semantics differ
  legitimately by platform. Only a same-OS delta against the same runner class means anything.
- **`work_counts` runs with no `--features`**, matching the harness the Phase 1 baselines were
  captured with. Changing the feature set would change what the counters mean.

### The compile guard was blind

`test.yml`'s `sniff-cross-platform` job already ran `cargo check --all-targets` plus the nextest
tiers on all three OSes, so the plan's "ensure it continues to" was satisfied on entry — but the
check ran **without `--features`**, and `sniff-lib` is `default = []`. Every
`#[cfg(feature = "remote")]` / `#[cfg(feature = "network")]` target was therefore invisible to the
guard *whose entire stated purpose is catching a Unix-only import in test code that never runs on
this OS*. This is the same defect Phase 6 found in the `just test` recipe, where 190 tests —
including the whole 65-test `remote_providers` suite — silently never ran. Now `--features remote`.

## Acceptance

Commands run from `sniff/`:

| Command | Result |
|---|---|
| `just sanity` | pass |
| `just lint` | pass, zero warnings |
| `just build` | pass |
| `just doctest` | pass |
| `just test` | 1603/1604 — sole failure is the pre-existing `detect_area_errors_when_not_in_repo` temp-dir timeout, a known baseline since Phase 4 and verified on clean `HEAD`. This phase changes no production code and cannot have caused it. |
| `just test-l2` | pass |
| `just bench` | **not run** — see "Timing evidence" |

Output parity: the CLI's default/`--plain`/`--json` goldens and the library result fixtures pass
unmodified. This phase changes no serializer, no render path, and no result type, so parity is
preserved by construction; the goldens are the check on that claim, not the argument for it.

### Post-review verification

The figures in this section are **historical review-cycle-3 evidence**, retained as a phase-boundary
record. They are not the current test counts; each later cycle added tests, and the current state is
tracked in [`log.md`](../../../log.md) and the review files.

After review cycle 3 repaired host-dependent OS-version normalization and selected an explicit
`main` branch for Git fixtures, the canonical macOS run passed all 1,657 sniff-lib tests and all 777
sniff-cli tests; `just lint` also passed. A Windows GNU
`cargo check -p sniff --all-targets --features remote` passed, including the process-tree cleanup
path. Native Linux and Windows Level-1 execution was not available on this macOS host; the Windows
CLI cross-check was stopped while still compiling dependencies under the session timeout rule, and
an MSVC cross-check lacked Windows SDK headers. The three-OS CI jobs remain the authoritative place
for native execution and retained per-OS work-count artifacts; their definitions are future coverage,
not evidence that those legs ran in this local implementation cycle.

## Completion boundary

Per the umbrella spec, completion requires R1–R13 implemented, acceptance criteria passing, output
parity, and cross-platform correctness. At the original Phase 8 boundary the umbrella feature did
not move to `_completed`. The following table is the historical state that review cycles 1–3
superseded:

| Requirement | State |
|---|---|
| R5.5 / R5.6 | At the Phase 8 boundary, `ManifestStore` was partial (`LockStore` only) and the structure-only migration was blocked pending owner review. |
| R6.4 | At the Phase 8 boundary, the single deepest-prefix ownership index was not built. |
| R9.5 / R9.6 | At the Phase 8 boundary, remote-tracking tip-set reuse and worktree-metadata reuse were not done. |

Current source closes all three rows: `ManifestStore` and focused shallow repository details satisfy
R5.5/R5.6; `PackageOwnershipIndex` satisfies R6.4; and the shared `RefSnapshot` plus proxy/admin-HEAD
worktree projection satisfy R9.5/R9.6. Review cycle 3 also closes aggregate projection purity,
linked-worktree status duplication, component-aware area matching, manifest-failure reuse, and
process-tree-bounded subprocess cleanup.

The remaining limitation is evidence scope, not an open R1–R13 implementation:

- **Native Linux and Windows Level-1 execution, and a matched three-OS work-count artifact set, are
  still absent as of review cycle 8.** Every implementation cycle has run on a macOS-only host, and
  the reviewed commit is on no remote, so the `sniff-cross-platform` and `sniff-work-counts` matrices
  cannot have executed it. Cross-compilation, Docker, and WSL do not substitute for a native leg, and
  a workflow definition is not an execution record. The current deferral entry, the exact
  implementation identifier, and the closure procedure live in
  [`deferred-perf-tests.md`](../../../deferred-perf-tests.md).

The synthetic large-service Criterion workload is **no longer a limitation** — review cycle 4 landed
it behind the `bench-internals` feature; see
[Post-review workload coverage](#post-review-workload-coverage).

R14 is explicitly not on this list — Phase 7 measured all nine candidates and deferred them with
evidence, which the umbrella spec's completion boundary expressly permits ("unimplemented R14
candidates do not block completion when evidence shows they are negligible").
