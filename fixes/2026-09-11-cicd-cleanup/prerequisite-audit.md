---
title: Prerequisite audit — fixes/2026-09-10-local-affected-scope
kind: audit
created: 2026-09-11
audited_head: 674d0d886
audited_tree: b6e3cce7abb7929b0ba455b19a907d589b310c6a
audits: fixes/2026-09-10-local-affected-scope/spec.md
for: fixes/2026-09-11-cicd-cleanup/plan.md
verdict: not-implemented
---

# Prerequisite audit

## Question

`fixes/2026-09-11-cicd-cleanup/plan.md` declares
`fixes/2026-09-10-local-affected-scope/spec.md` a `depends_on` entry and an
**entry condition**, not work to duplicate. Phase 1 task 1 must establish
whether the dependency has landed, and stop this plan if it has not.

## Verdict

**Not implemented.** The 2026-09-10 specification is still `status: draft`.
Its directory contains only `spec.md` — no plan, no phase record, and no
`_completed` move. The only commits touching its surfaces are the ones that
predate it:

| Commit | Subject | Relationship |
|---|---|---|
| `97f12132c` | planning(repo): schedule local affected-scope and host-result reuse fix | created the spec |
| `73bdd6d53` | fix(ci): match local receipts against the PR merge base | pre-existing PR #75 implementation, not the spec |

Every surface the spec names is still the PR #75 single-environment
implementation described in the 2026-09-11 plan's *Execution constraints*
section: `verified_environment()`, one `--exclude-environment`, and
compile-only reverse-dependent package selection.

## Acceptance-criterion checklist

Each row is the 2026-09-10 spec's numbered acceptance criterion, judged
against this checkout.

| AC | Criterion (abbreviated) | State | Evidence in this checkout |
|---|---|---|---|
| 1 | Scope calculated once locally, published, reused by CI without invoking the calculator | **NO** | No scope-notes ref exists. The hook exports `BISCUIT_CI_SCOPE_OUT` and feeds it to `local_evidence.py record`, but publishes only the *validation* note. `ci.yml:115` always runs `affected_scope.py`. |
| 2 | Source changes select owning packages plus direct reverse dependencies; infra-only selects no package jobs | **YES** | Pre-existing PR #75 behavior. Reproduced below. |
| 3 | Missing/stale/malformed/base-mismatched scope receipt makes CI calculate correct scope | **VACUOUS** | There is no scope receipt, so CI always calculates. Fail-safe by absence, contract unimplemented. |
| 4 | Dirty worktree publishes committed scope but no validation claim | **PARTIAL** | `.githooks/pre-push:94-99` gates the *entire* publication block on `git status --porcelain` being empty. The "publish scope anyway" half is absent. |
| 5 | Passing `strict`/`warn` omits only completed equivalent host cells and contributes visible passing outcomes to the rollup | **NO** | Exclusion is whole-environment (`--exclude-environment`). `scripts/ci-rollup.rs` contains zero occurrences of `excluded`, `local_evidence`, `local-origin`, or `LOCAL` — it has no local-origin cell concept at all. This is the direct cause of the PR #76 regression. |
| 6 | Complete failing `warn` run publishes its outcomes | **NO** | `.githooks/pre-push:76-79` returns from `warn` before reaching the publication block at line 94. |
| 7 | Complete failing `strict` run publishes before blocking | **NO** | Same structure; `strict` exits at line 83. |
| 8 | Interrupted/incomplete run does not suppress unfinished cells; completed siblings still reusable | **NO** | No cell granularity exists. The receipt is one whole-environment document (`l1_packages`, `l2_packages` name lists). |
| 9 | `scope-only` publishes scope, runs no tests, excludes no OS cells, allows the push | **NO** | The mode does not exist. `.githooks/pre-push:23-34` accepts only `off`, `warn`, `strict`. |
| 10 | `off` is a deprecated alias for `scope-only` with a migration message | **NO** | `off` still `exit 0`s immediately at line 25, discarding scope. |
| 11 | First-time `--no-verify` publishes nothing; CI calculates scope and runs everything | **YES** | By construction — Git does not invoke the hook. |
| 12 | A local failure does not short-circuit the other OS jobs | **VACUOUS** | Local failures are never published, so nothing can short-circuit. Contract untested. |
| 13 | Multiple matching OS receipts reused together; conflicts fail safe per cell | **NO** | `verified_environment()` (`local_evidence.py:87-107`) returns the **first** matching environment and stops. |
| 14 | `workflow_dispatch` ignores local evidence | **YES** | `ci.yml:116` guards the verify step on `EVENT_NAME != workflow_dispatch`. |
| 15 | CI summary names scope provenance, fallback reason, reused cells, retained incomplete cells, reduced estimate | **PARTIAL** | `ci.yml:164-183` prints change class, packages, `excluded_environment` as "locally validated environment", and the job estimate. It has no provenance/fallback-reason row, no per-cell reuse list, and no retained-incomplete list, because none of those exist. |
| 16 | Tests cover pass, fail, incomplete, stale, conflicting, `scope-only`, and no-receipt paths | **NO** | `scripts/ci/test_local_evidence.py` is 5 tests / 12 asserts, all against the v1 whole-environment model: exact scope, scope drift, advanced PR base, invalid base. No fail-outcome, incomplete, conflicting-receipt, or mode coverage. |

**Score: 3 implemented (AC2, AC11, AC14), 2 vacuously satisfied (AC3, AC12),
2 partial (AC4, AC15), 9 not implemented.**

## Consequence for this plan

The 2026-09-11 plan's Phase 3 and Phase 4 are written to *modify* contracts
that do not exist:

- Phase 3 removes `excluded_environment` and "accept the verified per-cell
  result set" — there is no per-cell result set to accept.
- Phase 4 says "Replace `verified_environment()` with verification that reads
  every `refs/notes/ci-local/<environment>` note" and "preserve `scope-only`
  and the deprecated `off` alias semantics from the prerequisite" — neither
  `scope-only` nor `off`-as-alias exists to preserve.
- Phase 4 also assumes `strict` and `warn` publish failing cells; today
  neither publishes anything on failure.

Two ways forward, both needing Ken's ruling (see
`open-questions-and-blockers.md`):

1. **Land 2026-09-10 first** as its own fix, then resume this plan at Phase 2.
2. **Absorb it** — rewrite Phases 3 and 4 of this plan to *build* the
   per-cell evidence model, the `scope-only`/`off` modes, and failure-path
   publication rather than to modify them. This is a real scope increase and
   changes what Phase 2's "freeze the shared contracts" means.

Option 2 is the cheaper total path (the 2026-09-11 spec already supersedes
three of the 2026-09-10 decisions: the compile-only reverse dependency, the
one-environment receipt, and `ci-verdict`), but it must be a recorded
decision, not a silent expansion. Per Phase 1 task 1, this plan is **stopped**
until that decision is made.
