# Faster Sniff tests results

## Completion claims

| Claim | Status | Evidence |
|---|---|---|
| Implemented | **Complete** | Phases 1–8 implemented the shared CLI process fixture, spawn guard, fixture migrations, request/work-count proofs, bounded process and terminal waits, and reconciled test inventory. Phase 9 completed the local pre-push tranche. |
| Verified locally | **Complete** | The committed candidate passes `just sanity`, `just lint`, `just check`, `just doctest`, `just test`, required-tmux `just test-l2`, the Sniff-scoped root leak sweep, tier coverage, audit configuration validation, and inventory reconciliation. Every gate was re-run at the current tree in Phase 10's continuation rather than credited from the earlier ledger. |
| Verified on CI | **Pending, no longer a readiness gate** | The branch is still unpushed, so no CI run contains the candidate. Ken's `DECISION` in [`review-1.md`](review-1.md) removed CI from the production-readiness determination and directed local cross-OS execution instead; that execution is complete and green on all three declared hosts (see [Review-1 implementation outcomes](#review-1-implementation-outcomes)). What CI is still owed is the matched per-family timing sample, not correctness evidence. |
| Faster tests demonstrated | **No** | The clean, pinned, alternating comparison review-1 asked for was run and is **negative**: the full L1 cohort shows no demonstrated change in either direction, and `sanity` is consistently slower. See [Review-1 implementation outcomes](#review-1-implementation-outcomes). |

The implementation is ready for review and is now verified on macOS, Linux,
WSL2, and native Windows. It is **not** ready to archive: the fix's headline
outcome — a faster suite — is not demonstrated by the measurement, and that is
a question for Ken rather than something this cycle can close. The active fix
directory has intentionally not been moved to `_completed/`.

## Scope and coordination

All baseline and candidate measurements remain on the **before** side of the
`production-caching-2026-07-22` boundary. The coordinated
[`2026-07-22-inefficient-calling`](../2026-07-22-inefficient-calling/spec.md)
fix has not landed, and no comparison spans it.

This fix changed test infrastructure and test code only. It did not change a
production parser, schema, shipped template, prompt, configuration or
persistence format, CLI output channel, or terminal rendering contract.
Consequently, no passive shipped-artifact corpus, read/write/read round trip,
or new product end-to-end test was applicable in Phase 10.

## Requirement-to-test mapping

| Changed behavior | Public observable proof |
|---|---|
| Deterministic CLI launches do not inherit checkout, home/config/cache, Git plumbing, software-roster, or rendering state | `cli_process_fixture` runs the real CLI and environment recorder through both command surfaces. It covers the exact hostile values `GIT_DIR=/host/repository.git`, `SNIFF_WAN_IP_ENDPOINTS=http://host.invalid/ip`, `FORCE_COLOR=1`, relocated home/cache/temp roots, missing/present ambient directories, canonical and symlink checkout paths, fake-only and host PATH modes, and post-policy overrides. |
| Future L1 CLI tests cannot bypass the fixture silently | `spawn_site_guard` detects all three raw spawn forms and whitespace variants, ignores prose and unrelated binaries, rejects a deliberately neutered detector, rejects stale/blank allowlist entries, and passes the live source census with zero generic migration exemptions. |
| Repository tests remain correct under inherited Git contamination | `test_git_full_reports_conflicted_files` and `structural_fixtures_match_canonical_git_when_available` use the original hostile `GIT_DIR`/`GIT_WORK_TREE` inputs and assert the dependent conflict report/prediction against fixture-owned state and canonical Git. |
| Focused library tests do not perform incidental full discovery | `test_installable_false_for_os_specific_programs` retains the exact `WindowsTerminal` and `TextMate` inputs through the public category seam. `formatting_workload_keeps_descendant_work_at_zero` asserts zero descendant, Git-discovery, and status-walk work while retaining its output assertion. |
| Seeded execution and projection do not reacquire Git state | `seeded_git_execution_and_projection_do_not_rediscover_the_repository` asserts one acquisition discovery, then zero additional discoveries/opens and exactly two requested status walks, while also asserting dirty state and all four changed-path buckets. |
| Work counters remain trustworthy across concurrency boundaries | Existing scoped-thread, pooled/Rayon-worker, parallel-walker, manifest-walker, and network-hop tests assert propagated counts; parallel accepted work matches an independent serial scan. |
| Terminal and PTY readiness is bounded by observation, not sleeps | `os_subcommand_runs_in_pty` asserts the real OS heading and EOF under an explicit deadline. The two L2 tests poll complete final `CapturedFrame` predicates and assert visible content, style/link fallback, and layout through shipped fixture binaries. |
| Remote/process fixtures own external effects and cleanup | Loopback Wiremock tests assert identity, selection, request counts, pagination bounds, consent, credential scope, and typed failures. Process tests assert complete dual-pipe output, timeout typing, descendant termination, `ECHILD` reaping, and bounded inherited-pipe cleanup; the root leak sweep found no survivors. |

The detailed per-phase mapping and original red/green discoveries are retained
in [`log.md`](log.md). No implementation code changed in Phase 10, so this
phase added no test identity of its own.

## Coverage changes

The matched local L1 population changed from 2,599 to 2,609 tests: 13 were
added and three were removed. Sanity coverage stayed at 1,820 identities.

Added identities:

- Eight fixture-contract tests: command-surface parity, override precedence,
  hostile-input scrubbing, bounded PATH modes, ambient-directory validation,
  canonical checkout containment, symlink containment, and owned-command
  lifetime.
- Four spawn-guard tests: live fixture routing, raw-form recognition, negative
  detector mutation, and stale/unexplained allowlist rejection.
- `seeded_git_execution_and_projection_do_not_rediscover_the_repository`.

Removed identities and replacement coverage:

| Removed identity | Reason | Replacement coverage |
|---|---|---|
| `install_dry_run_emits_announcement_and_success_under_pty` | Its bespoke `SNIFF_INTERACTIVE_PTY` gate was unreachable from every canonical recipe, and its manufactured PTY added no distinct terminal behavior. | Reachable L1 `install_dry_run_plain_emits_announcement_and_success_status` asserts the announcement, success status, and dry-run behavior; `os_subcommand_runs_in_pty` retains real CLI PTY wiring and bounded EOF. |
| `foo::bar` | Placeholder assertion with no product behavior. Its deletion receives no performance credit. | No replacement required; the reconciler records the zero-member family and disposition explicitly. |
| `from_shorthand_tries_github_when_token_set` | Used public provider endpoints, accepted every transport failure, and could not distinguish whether dispatch occurred. | Loopback provider and `GitRemote` dispatch tests assert provider identity, exact requests, typed errors, credentials, and consent without live network. |

Two equality assertions were strengthened without changing identity:
`os::user::equality_is_by_variant_and_value` now compares independently
constructed equal and unequal UID/SID keys, and
`programs::types::test_executable_source_equality` compares independently
deserialized same- and cross-variant values. Phase 9 also replaced the exact
Clippy input `assert!(host.len() >= 1)` with an equivalent non-empty assertion;
the named test and all broader gates passed afterward.

No test was moved to a slower tier, disabled, retried, or given a larger
timeout. The two retained L2 tests remain on the canonical `test-l2` route;
the bespoke environment-gated test and placeholder binary were removed.

## Measurements and budgets

Five alternating warm local runs per revision kept build/setup, runner elapsed,
and summed test duration separate. Full provenance and every raw run are in
[`measurement/local-phase8/`](measurement/local-phase8/).

| Cohort | Baseline build/setup | Candidate build/setup | Baseline runner / summed | Candidate runner / summed | Judgment |
|---|---:|---:|---:|---:|---|
| Full local L1 | 1.2 s median | 1.1 s median | 21.53 / 324.99 s | 44.80 / 684.81 s | All samples passed, but the candidate ran amid load up to 134 and extensive unrelated dirty shared-worktree activity. Attribution only; CI comparison pending. |
| Sanity | 2.6 s median | 2.4 s median | 7.94 / 104.80 s | 17.03 / 254.82 s | The alternating candidate window missed the 15-second budget under the same contention. The Phase 10 acceptance reruns passed 1,820 tests in **11.72 s** and, at the committed candidate, **12.13 s wall clock** — both within budget. |
| Changed CLI integration cohort | — | — | 115.54 s summed | 94.63 s summed | Directional 18.1% reduction, inside drift; no causal claim. |
| Requested-work / representative fixtures | — | — | 23.45 s summed | 4.30 s summed | 81.7% reduction outside both drift brackets. |
| Inherited-Git fixture repairs | — | — | 29.89 s summed | 51.17 s summed | Loaded-candidate increase retained as attribution; no production regression inferred. |
| Loopback remote-provider cohort | — | — | 5.07 s summed | 158.90 s summed | Host/candidate-state confounder: the only target change removed one live-network test. CI comparison pending. |

Each changed timing contract completed ten candidate executions with no
failure, retry, timeout, or leak: CLI PTY 0.304–0.446 seconds, tmux CI/CD final
frame 0.317–0.378 seconds, and tmux Git final frame 0.340–0.374 seconds.

The ratified work-budget classes D0/W1/C0/C1/H1/P1/E1 and their family mapping
remain adjacent to the baseline in [`inventory.md`](inventory.md). Numeric
per-family CI timing targets remain pending because the budget tool correctly
refuses local provenance and fewer than three compatible candidate CI runs.

## Work-count evidence

The matched production `staged_filesystem_full_all_stages` case is unchanged
across all eight configured signal groups: filesystem walk 639, filesystem I/O
17,337, inventory 1,367, repository structure 185, Git 19, and zero process,
remote, and WAN work. This is the unchanged drift bracket, not an optimization
claim.

Eliminated incidental test work is proved separately: acquisition performs one
Git discovery; seeded execution and projection perform zero additional Git
discoveries or opens while preserving dirty state and all dependent path
buckets. Worker-propagation tests establish that the lower counts are complete,
not lost across threads, Rayon/pools, or walkers. The compatible counter report
is [`work-counts.compare.md`](measurement/local-phase8/work-counts.compare.md).

## Final local gates

Every gate below was re-run against the committed candidate at branch head
`a05e3b747`. Unrelated Darkmatter and Claudine commits landed after the Sniff
candidate, so no earlier result was credited by reuse; no Sniff source file
changed between the two ledgers and every result reproduced.

| Gate | Wall / runner | Result |
|---|---|---|
| `just sanity` | 12.13 s wall (9.418 s + 0.506 s runner) | Pass: 1,419 library (23 skipped) + 401 CLI tests; **within the 15-second budget**. A preceding cold invocation took 142.16 s because the changed dependency graph rebuilt; the budget governs the warm subset. |
| `just test` | 86.34 s wall / 23.855 s runner | Pass: 2,609 tests, 24 declared policy skips, 0 failures, no slow marks, retries, or timeouts. |
| `just lint` | 50.70 s | Pass, no diagnostics. |
| `just check` | 42.17 s | Pass. |
| `just doctest` | 11.13 s | Pass: 91 run, 0 failed, 22 documented ignored examples; CLI has zero doctests. |
| `just test-l2` with tmux required | 14.35 s wall / 0.655 s runner | Pass: both affected tests (0.323 s, 0.331 s), 802 correctly skipped. |
| Root `just test-leaks sniff` | 28.50 s wall / 23.477 s runner | Pass: all 2,609 L1 tests; `leak-sweep: no leaked processes detected`. |
| Root `just check-tier-coverage sniff` | — | Pass under installed Bash 5.3: 1 listed, zero stranded. The default macOS Bash 3.2 invocation fails before audit because shared tooling uses `BASHPID`; that portability issue is owned by the repository-wide tooling. |
| Audit `config validate` + `reconcile --markdown` | — | Pass: 2 packages, 6 selections, 4 environments, 83 families, 2,635 runner identities, 2,676 source attributes, 41 declared platform exclusions, `GATE EXIT=0`. |

`just test-real` was not applicable: this fix changes no real-resource product
behavior, and the audited route remains explicit (`network` for the library,
bare CLI). The final `git diff main -- .config/nextest.toml` is 16 insertions
and 41 deletions of unrelated removals and expanded justifications; filtering
that diff for `sniff` or `detect_completes` returns no line, so this fix added,
removed, or weakened no Sniff override, retry, tier, assertion, or timeout. The
only surviving Sniff-specific override is `sniff-windows-l1`, documented in
[`inventory.md`](inventory.md) as retained pending native-Windows concurrency
evidence.

## Failures and skips

The final L1 and L2 gates have no failures, retries, timeouts, slow marks, or
leaks — reproduced at the committed candidate, where `just test` and the root
leak sweep each ran all 2,609 tests green. The 24
L1 skips and 22 ignored doctest examples are declared policy exclusions, not
missing execution evidence. Earlier red tests and invalid command attempts are
recorded in [`log.md`](log.md); none is hidden by a retry or weakened gate.

The Phase 10 default tier-audit invocation is the only new environment failure:
macOS `/bin/bash` 3.2 does not define `BASHPID`. A second invocation was made
only after changing the environment to the already-installed Bash 5.3, and it
passed. This shared-tool portability issue is outside the Sniff package scope.

## Deferred findings and pending evidence

There are **no permitted deferred findings** and no generic fixture-migration
exemptions. The only outstanding items are required CI evidence, not deferrals:

| Pending evidence | Reason | Owner / handoff |
|---|---|---|
| CI run covering the candidate | The candidate is committed (Sniff sources `66892d319`, branch head `a05e3b747`), but the branch is 53 commits ahead of `origin/fix/cli-slow-tests` and unpushed; the newest branch run `34159725015` is against origin head `a9e88c069…`, which does not contain the candidate. Pushing is the human-gated Phase 9 step. | [Phase 9 handoff](plan.md#phase-9--ci-evidence-rb5-second-tranche-ac6-ac8) |
| Three consecutive candidate runs for Ubuntu, macOS, native Windows, and WSL2 | Required for compatible matched-test and per-family budget comparison; baseline run `34008778001` supplies only one baseline sample. | [Inventory CI handoff](inventory.md#phase-9-ci-evidence-handoff) |
| ~~Native Windows compile/runtime proof~~ | **Resolved by review-1 cycle 1.** The area now has a local `just check-windows` mingw recipe, and the suite was executed natively on `build-win-native`: 2,597 run, 2,597 passed. | [Review-1 implementation outcomes](#review-1-implementation-outcomes) |
| Matched per-family CI timing budgets | Ratifying per-family budgets needs three consecutive candidate runs per declared environment, which needs the push. The **local** alternating comparison review-1 required has been run and is recorded; only the CI-sample half is outstanding. | [Deferred performance measurement](deferred-performance-measurement.md) |

The unrelated shared `check-tier-coverage` Bash 3.2 portability finding is
owned by the repository-wide test tooling rather than deferred inside this
Sniff fix; the successful Bash 5.3 invocation provides this phase's required
tier census.

## Review-1 implementation outcomes

Cycle 1 of the review-to-implement loop closed all three findings in
[`review-1.md`](review-1.md). The narrative record is in [`log.md`](log.md).

### Fluent CWD escape (Medium)

`assert_disposable_context` now requires an ambient-context directory to be
inside the canonical system temporary root as well as outside the checkout, so
fixture ownership of the launch directory is structural rather than
conventional. Neither of the two fluent forms will accept a developer
repository or a home subdirectory. The spawn guard gained a PATH-escape
detector requiring a call-site justification comment on every
`fake_only_path()` / `host_path()` site, and the four live sites carry one.
Nine tests were added; each was proved non-vacuous by mutation.

### Cross-OS execution (High)

Ken's `DECISION` replaced the CI clause with local execution on the three
declared build hosts. All runs used a marker-verified protocol — reset, detach,
patch, run, and re-verify HEAD/dirty-count/candidate-marker inside a single ssh
session — because the shared standing clones were concurrently reset by another
session mid-work and the first results had to be discarded.

| Host | Environment | Result | Elapsed |
|---|---|---:|---:|
| `build-linux` | Linux x86_64 | 2,605 run, **2,605 passed**, 15 skipped | 7.430 s |
| `build-win` | WSL2 | 2,605 run, **2,605 passed**, 15 skipped | 13.443 s |
| `build-win-native` | native Windows | 2,597 run, **2,597 passed**, 8 skipped | 68.349 s |
| this host | arm64 macOS 27.0 | 2,618 run, **2,618 passed**, 24 skipped | 26.570 s |

Native Windows found two real, previously unexercised defects in this fix's own
process fixture: `parse_environment` keyed the recorded environment
case-sensitively, but Windows variable names are case-insensitive and
`cmd.exe /D /C set` reports the canonical `Path`, not `PATH`. Two tests panicked
on `Option::unwrap`, and a third — the assert-vs-raw surface equality check —
was silently **vacuous** on Windows, comparing `None` to `None`. All three are
fixed. A matched control at the unpatched base on the same host in the same
session failed 3 tests, so the candidate is strictly better there.

Two `0xc0000409` stack-buffer-overrun aborts in the `sniff` library's native
detectors reproduce at the **unpatched base** and vary between runs. They are a
pre-existing, load-sensitive native-Windows defect unrelated to this fix and are
recorded for separate follow-up rather than deferred here.

### Faster-tests outcome (High) — negative

The clean, pinned, alternating comparison was run: baseline `c2dee9217` in its
preserved detached worktree against a freshly pinned candidate worktree at
`fe83e7481` carrying only this fix's Sniff diff, five alternating warm rounds
per cohort, all twenty measured runs compile-free. Full detail in
[`measurement/review1-alternating/summary.md`](measurement/review1-alternating/summary.md).

| Cohort | Baseline median (range) | Candidate median (range) | Δ | Drift bracket | Verdict |
|---|---:|---:|---:|---:|---|
| full L1 `test`, wall | 23.66 s (20.22–36.63) | 23.36 s (20.24–30.99) | −1.3% | 69.4% | inside drift — no demonstrated change |
| `sanity`, wall | 11.10 s (10.14–15.29) | 12.01 s (11.26–17.77) | +8.2% | 46.4% | magnitude inside drift, but slower in **5 of 5** paired rounds |

**The candidate is not measurably faster on either cohort**, so review-1's
finding stands. The `sanity` cohort is a near-perfect control — it is
`--lib --bins`, so it never builds the changed `cli/tests/` sources, and its
test population is identical in both trees — and it still moved consistently
slower. The attribution is that this fix committed ~9.4 MB across 395 files of
its own measurement artifacts into the repository, and Sniff's repo-walking
tests (`filesystem::docs::tests::integration::*`, `list_worktrees_*`,
`blast_radius::*_in_real_repo`) traverse them. That is an artifact cost rather
than a property of the test-speed work, but it is a cost this branch imposes on
developers, and it means any future between-tree comparison in this area must
control for tracked-content volume.

The `sanity` budget is met at the median in both trees (11.10 s and 12.01 s
against 15 s) but is **not robustly met under load in either**: baseline
exceeded 15 s in 1 of 5 rounds and the candidate in 2 of 5, and the candidate
has strictly less headroom on every measure.

## Acceptance review

| Criterion | Status | Evidence |
|---|---|---|
| AC1 — every family evaluated and routed | **Verified** | Final reconciler: 83 families, 2,635 identities, 41 declared platform exclusions, zero violations; [`inventory.md`](inventory.md). |
| AC2 — deterministic CLI/repository isolation | **Verified** | Fixture hostile-input and real-CLI probes, zero-generic-exemption spawn guard, and contaminated Git regressions pass. Review-1 cycle 1 closed the two remaining gaps: launch-directory ownership is now structural on the fluent escapes, and the Windows command adapter is executed natively rather than assumed. |
| AC3 — request/work-count contracts | **Verified** | Seeded acquisition/execution counters, zero descendant work, dependent output assertions, and thread/Rayon/walker propagation tests pass; compatible eight-signal drift bracket retained. |
| AC4 — bounded effects and cleanup | **Verified locally** | Loopback request-count/error tests, real process termination/reaping tests, bounded PTY/final-frame polling, required-tmux execution proof, and leak sweep all pass. |
| AC5 — generic exemptions eliminated | **Verified** | Spawn allowlist has zero generic migration entries; surviving technical tier exclusions have explicit ownership and guard coverage. |
| AC6 — canonical gates and cross-platform evidence | **Verified** | All applicable local gates pass: `just test` (2,618/0 failed), `just check`, `just lint`, `just doctest` with `remote` preserved, required-tmux `just test-l2` (2/2), `just sanity` at 12.01 s median against the 15-second budget, and the new `just check-windows` mingw compile check. The cross-platform clause is satisfied by execution rather than by `windows-latest`: green runs on Linux, WSL2, and native Windows, per Ken's `DECISION` in `review-1.md`. |
| AC7 — bespoke gates/placeholders resolved | **Verified** | `install_interactive_pty.rs` and `foo.rs` removed; source/listing reconciliation finds no silently unreachable replacement. |
| AC8 — results and clean process evidence complete | **Partially verified** | Local results, coverage, work counts, skips/failures, no-weakened-gate check (`nextest.toml` diff carries no Sniff line), and the clean leak sweep are complete. Review-1 cycle 1 added the clean, pinned, alternating before/after comparison the spec requires — and its result is **negative**, so the ratified timing budgets are not met by a demonstrated speedup. Matched per-family CI budgets stay pending on the push. |
| AC9 — contract documentation drift | **Verified** | The Sniff skill already documents the fixture, L1/L2 feature split, final-frame polling, counters, and audit configuration. No additional area-doc or production-contract change was found in Phase 10. |

Final closure is therefore **implemented and verified for correctness on all
four target environments, but its headline performance claim is not
demonstrated**. Archival must wait on a decision from Ken about the
faster-tests outcome, not on further verification: the correctness evidence the
spec asks for is now complete, and the only mechanical item left is the matched
per-family CI timing sample, which needs the branch pushed.

Evidence location: the raw runs under `baseline/`, `measurement/`, and
`enumeration/`, plus `sources.json`, are kept out of the tree at
`~/.rusty-biscuit/ci-evidence/2026-09-07-faster-sniff-tests/`; the CI
evidence there came from runs 34008778001, 34159725015, and 34173378609.
