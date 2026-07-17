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
  - ../../spec.md
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

### The "10–50×" claim was worse than unsupported

The umbrella spec asked for the claim to be "replaced with qualified evidence" on the strength of a
2.4× measurement. The counters say the honest number is closer to **1×** on the fixture in question:

| Counter, 375-package fixture | `structure()` | `full()` |
|---|---:|---:|
| `filesystem.io.metadata_probes` | 13,274 | 13,275 |
| `filesystem.io.read_dirs` | 702 | 701 |
| `filesystem.repo.manifest_parses` | 454 | 454 |
| `filesystem.repo.package_enrichments` | 300 | 300 |
| `filesystem.file_inventory.files_accepted` | — | 3,755 |

Structure mode records the *same discovery work* as full mode. Full's only material addition is
classifying the inventory. Two reasons, both already documented elsewhere and now joined up:

1. **R5.6 is blocked.** Structure mode is supposed to skip enrichment; it does not, and Phase 4
   stopped for review rather than ship a change that silently empties `sniff repo test-runner`,
   `dependencies`, and `package-manager`. Until that lands, structure's only saving is inventory.
2. **Structure pays a walk full mode does not.** It skips the manifest index and therefore falls back
   to `walk_for_nested_markers` (`filesystem.repo.nested_marker_walks: 1`), while full reuses the
   observation index.

So the claim was not merely imprecise — it pointed the wrong way about where the cost is. Replaced
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

## Final counters vs. Phase 1

`cargo run -p sniff --release --example work_counts`, same host and fixtures as Phases 1–7.

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

**Unchanged cases (drift brackets):** `staged_filesystem_summary_git_plus_repo` and
`repo_structure_huge_375_packages` are **byte-identical to Phase 1 on every counter**. Both are
structure-mode, and R5.6 — the only requirement that would have moved them — is blocked. Their
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
Not a regression — no case got worse — but the mechanism was mis-modeled, and canonicalization is
evidently not driven per-enrichment. Recorded rather than quietly dropped; R6.2's "hot ownership
lookups use lexical comparisons rather than repeated canonicalize" is the requirement that would
own it, and the deepest-prefix ownership index it depends on (R6.4) is itself unimplemented.

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

## Completion boundary

Per the umbrella spec, completion requires R1–R13 implemented, acceptance criteria passing, output
parity, and cross-platform correctness. **The umbrella feature does not move to `_completed`**, and
this phase must not be read as declaring it complete. Open at the end of Phase 8:

| Requirement | State |
|---|---|
| R5.5 / R5.6 | `ManifestStore` partial (`LockStore` only); the structure-only migration is **blocked pending owner review** — implementing it literally breaks three shipped CLI commands. |
| R6.4 | The single deepest-prefix ownership index is **not built**; three correct implementations remain. |
| R9.5 / R9.6 | Remote-tracking tip-set reuse and worktree-metadata reuse **not done**. |

Phase sub-specs 1–8 may each move to `_completed` independently; the umbrella stays open on the
above. R14 is explicitly **not** on this list — Phase 7 measured all nine candidates and deferred
them with evidence, which the umbrella spec's completion boundary expressly permits ("unimplemented
R14 candidates do not block completion when evidence shows they are negligible").
