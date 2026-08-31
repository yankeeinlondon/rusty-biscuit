# Phase 5 verification evidence

Host: macOS (`America/Los_Angeles`)  
Evidence base: `5e4dba173109abbcbe81bf8b224c829eb33966b6` plus uncommitted
worktree changes. This is not an immutable acceptance-candidate SHA because
the execution request prohibits staging and committing.

## Requirement-to-test mapping

[`phase5-test-map.md`](./phase5-test-map.md) maps every `MM-S01`–`MM-S12`
behavior to its public result, dependent state, representation variants,
negative behavior, and concrete tests. The mapped L1 and macOS L2 assertions
all passed. The acceptance-ledger rows remain blocked where the required
platform matrix or immutable candidate is unavailable.

Schema/file behavior additionally has both required artifact layers:

- passive corpus: `shipped_prompt_corpus_parses_frontmatter`
- real shipped artifact through the normal CLI path:
  `level2_lifecycle_shipped_implement_route_matches_direct_run`

Persistence boundaries are covered by the diagnostic two-cycle serialization
tests and by repeated proxy retry/resume/loop rereads that assert source and
target bytes remain unchanged.

## Targeted regressions

| Behavior | Test | Result |
|---|---|---|
| Caller eager-file overrides retain the invocation-fixed launch boundary across a target-repository change; quoted and unquoted schema spellings use the exact `spec.md` input | `eager_file_set_override_keeps_launch_area_across_target_repository_boundary` | failed before the fix; passed after |
| Claudine pre-validation defers a launch-resolving caller eager file to canonical preparation | `pre_validate_schema_defers_caller_eager_file_to_canonical_preparation` | failed before the fix; passed after |
| A captured fallback does not swallow the exact unresolved `everywhere` partial; scalar `file`, scalar-to-`file[]`, and array `file[]` representations remain interactive | `pre_validate_schema_keeps_unresolved_caller_file_partials_interactive` | failed before the correction; passed after |
| The three real PTY partial-file flows display confirmation and launch only after acceptance | the three `level2_pty_provided_partial_*_confirms_and_launches` tests | failed in the first full L2 run; 3/3 passed after the correction |
| Parallel proxied failure retains task/channel order, sibling settlement, deterministic state merge, and descendant teardown | `level2_mega_merge_s10_parallel_proxy_failure_task_integrity` | passed targeted and in the final full L2 run |
| Cross-repository route keeps authoring context separate from invocation-fixed child CWD | `level2_lifecycle_equivalence_cross_repo_file_resolution_matches_direct_run` and shipped-route/direct equivalence | passed targeted and in the final full L2 run |
| Initialize/terminal lifecycle expectations match current public event ownership and typed rendering | cycle guard, initialize status-block, set-forwarding, and shipped-route L2 tests | passed targeted and in the final full L2 run |

Malformed/non-resolving file inputs remain negative tests: launch-only
document-authored files fail, zero-match partials preserve the typed error,
and non-string array members remain schema-validation failures.

## Broader gates

| Area / command | Result |
|---|---|
| `biscuit-file: just test` | passed: library 383 passed / 4 skipped; CLI 61 passed |
| `biscuit-file: just test-l2` | passed: no applicable L2 tests |
| `biscuit-file: just lint` | passed |
| `darkmatter: just test --no-fail-fast` | passed across the area; library 5,667 passed and DMLS 566 passed / 3 skipped |
| `darkmatter: just lint` | passed |
| root `just test biscuit-test-harness` | passed: 85 tests |
| root `just _lint biscuit-test-harness` | passed |
| `claudine/rendezvous: just check` | passed |
| `claudine/rendezvous: just test` | passed: core 82, daemon 168, client 21 |
| `claudine/rendezvous: just lint` | passed |
| `claudine: just test --no-fail-fast` | passed: catalog types 21; library 3,954 / 7 skipped; contract 47 / 5 skipped; CLI 2,313 / 247 skipped; generator 152 / 4 skipped |
| `claudine: just test-l2 --no-fail-fast` | passed: 228 / 228; 2,332 non-L2 tests filtered |
| `claudine: just lint` | passed, including 18 diagnostic guard tests and the lifecycle-doc-facets guard |
| `claudine: just check-windows` | passed compile-only; target-specific unused/dead-code warnings remain and are not native runtime evidence |

`claudine-cli::wrap_structured_stream::structured_verbosity_controls_stream_stderr_lines`
was flaky in both full L1 runs and passed on its third attempt. No test failure
was hidden or waived.

## Repository integrity

- `git diff --check`: passed.
- The anchored stale-marker worktree and index scans passed.
- GitNexus `detect_changes(scope: compare, base_ref: main)` reports CRITICAL
  cumulative integration risk: 10,686 changed symbols across 937 files affect
  56 indexed execution flows. The comparison includes the complete uncommitted
  mega-merge worktree rather than only Phase 5. The immutable acceptance
  candidate and required Linux, Windows, native-provider, and interactive
  terminal matrix therefore remain release blockers.

## Required evidence still blocked

- `darkmatter: just test-l2 --no-fail-fast`: library L2 passed 19/19; CLI
  had 58 passes and 11 host/backend failures. Eight panes could not find the
  `md` binary and three Apple Terminal luma probes failed. These are recorded
  as backend failures, not feature passes.
- Attended/native L3 keyboard and process-interruption tests cannot run in
  this non-interactive session.
- Linux L1/lint, the dedicated Linux tmux L2 CI job, Windows CI L1, and native
  Windows runtime evidence require external CI/native hosts and an immutable
  acceptance-candidate SHA.
- The evidence commit is prohibited by the execution request. No files were
  staged or committed.

These blockers prevent Phase 5 exit even though all locally runnable Claudine
tests and lints pass.
