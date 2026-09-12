---
title: Absorption audit — fixes/2026-09-10-local-affected-scope into this fix
kind: audit
created: 2026-09-12
for: fixes/2026-09-11-cicd-cleanup/spec.md
absorbs: fixes/2026-09-10-local-affected-scope/spec.md
supersedes: fixes/2026-09-11-cicd-cleanup/prerequisite-audit.md
status: complete
---

# Absorption audit

`prerequisite-audit.md` (2026-09-11) found the 2026-09-10 specification not
implemented: 3 of 16 criteria met. Review cycles 2 through 4 of this fix then
built most of that specification's mechanisms while fixing review findings,
without anyone recording that the scope had moved. Ken ruled on 2026-09-12
that the absorption is recorded, on the condition that the mechanisms are
audited against the 09-10 objectives as a deliberate implementation would have
been, not merely against the tests that happen to pass.

Three read-only auditors covered the scope-evidence half (R1, R2, R3, R5, R8,
R9), the validation-evidence half (R4, R6, R7), and documentation, tests, and
the verification plan (R10, AC16). The consequential claims below were
spot-checked against the source before being recorded. Working tree at
`ab90299c0` plus the uncommitted review-4 implementation.

## Verdict

**The absorption is substantive, not incidental.** Both halves of the 09-10
evidence model exist as real implementations: a closed scope-receipt schema
with coded miss reasons and canonical bytes; an R3-ordered verifier; an
evidence overlay that reads nothing from the checkout; a non-vacuous proof that
the planner never runs on a receipt hit; per-cell validation receipts carrying
outcome, exit code, duration, completion, bounded failing-test identities, and
host provenance; verification that combines every environment ref per cell;
and a rollup in which a reused cell is a first-class cell with its own origin,
where a reused failure fails its area through the same rule as a hosted
failure. That core would survive a formal implementation largely unchanged.

Three of the four wasted-work problems the 09-10 spec named are eliminated.
The fourth, pass-only evidence, is eliminated for `warn` and deliberately not
for `strict` (see D1). The weak seam is the hook and its test suite, where
three behaviors are true by control flow or by shell default rather than by
test or by design, and where one test pins a deviation from the spec by
grepping the hook's own message text.

## Requirement map

States: MET, PARTIAL, ABSENT, SUPERSEDED (by this spec, deliberately),
VACUOUS (true by construction, untestable).

| 09-10 item | State | Evidence | Strategic note |
|---|---|---|---|
| Scope evidence model | MET | `schema.py:66,134-141,278-312,357-363`; byte stability `test_local_evidence.py:246` | Receipt carries `plan` and the legacy `scope` projection, cross-checked on package names only (`schema.py:725-732`); CI re-projects on every hit anyway (`ci.yml:178-184`). Duplication with a drift surface (W6). |
| Validation evidence model | MET | `schema.py:250-276,658-668`; `local_evidence.py:649-703`; failing identities `571-576` → `ci-rollup.rs:1140-1148` | `browser` is recordable and reusable although 09-10 lists browser reuse out of scope (W9). `interrupted` completion has no producer (W8). |
| R1 committed scope before tests | MET, base rule SUPERSEDED | `pre-push:294-300,414-430`; `test-pre-push.sh:933-955` | Base is the PR target tip or remote `main` per this spec §4, not merge-base with `origin/main`. Receipt withheld when the target advanced (W2). |
| R2 publish scope independently | PARTIAL, SUPERSEDED in part | `pre-push:505-507` precedes gates at `527` | Independent of gates, host, dirtiness. Not of the constraint review: a calculation failure blocks the push (`465`) per this spec §4, which R2 did not require. Recorded, no ruling needed. |
| R3 matching scope authoritative | MET | `ci.yml:137-157`; `local_evidence.py:519-539`; `test_ci_local.py:966-972,974-1028` | Notes fetch uses the checkout's `origin` (`ci.yml:84-86`); correct for fork PRs only by `actions/checkout` convention, stated nowhere. |
| R4 publish every complete outcome | PARTIAL | `pre-push:527-596`; per-cell `local_evidence.py:706-765` | Warn-fail publishes. Strict-fail records locally and withholds the push (`586-589`), pinned by a source-grep test (`test-pre-push.sh:733-738`). Needs a ruling (D1). Failing guard clauses print nothing (W10). |
| R5 hook modes | MET | `pre-push:85-103,509`; `test-pre-push.sh:359-391` | Unknown-mode message still says "Expected: off, warn, or strict" (`pre-push:99`), pinned by `test-pre-push.sh:386` (W11). |
| R6 exclude cells not environments | MET; equivalence by key, not policy | `local_evidence.py:276-277,316,372-388`; `schema.py:769-783` | Lint cells are `reusable: true` by default (`affected_scope.py:1433,1470-1478`) unlike `check` (`1508`); only `RECORDABLE_GATES` (`local_evidence.py:62`) keeps lint out of receipts (W7). Cross-note conflict resolves newest-wins (`local_evidence.py:233-238`) where the spec says schedule the cell. Needs a ruling (D2). |
| R7 local failures join the verdict | MET, transitional | `ci-rollup.rs:264-287,1930-1936,3060-3078,3156-3165`; `ci_workflow_contracts.rs:2371-2395` | Labeled "reused local evidence" in rollup and area step summary; absent from the scope job summary (W5). `ci-verdict` remains a second verdict path until the OQ3 rollout. `reuse_contested` (`ci-rollup.rs:1907-1916`) lets an executed result override the plan's reuse decision. |
| R8 fail-safe | MET, one deviation | `local_evidence.py:508-539`; `ci.yml:85,142-151` | A crash in `verify --cells` or `--apply-to` fails the scope job under `set -euo pipefail` (`ci.yml:96,165-184`), skipping the fan-out. Fail-closed, not add-work (W4). |
| R9 cheap observable bootstrap | PARTIAL | `ci.yml:81-88,142-184,224-245` | `rustup show` runs before `scope-verify` on every run (W3). Summary lacks matching environments, reused cells, and retained-incomplete cells; `evidence_rejections` ride in the plan and nothing prints them in CI (W5). |
| R10 docs together | PARTIAL | drift list below | Two authorities the contract test does not guard are stale: root `README.md` and `docs/testing-strategy.md`. |
| AC1 once locally, CI reuses | PARTIAL | `pre-push:464,578-581`; `justfile:103-104`; `just/ci-local.just:77,154-156` | Not literally once: the trigger review plans per context, then `just pre-push` replans from the working tree against merge-base `origin/main`; the validation receipt is recorded against the second plan, the scope receipt against the first. Harmless only because `verify_cells` never compares `scope_identity` (`local_evidence.py:391-405,797`) (W1). |
| AC2 selection policy | MET, one bullet SUPERSEDED | `affected_scope.py:763-788,1750-1757`; `GLOBAL_PATHS_BY_GATE:58` | Reverse-dependency compile check superseded by OQ1; the ruled in-job step does not exist yet. |
| AC3 bad scope receipt → calculate | MET | `test_local_evidence.py:264-321`; `test_ci_local.py:974-1028` | |
| AC4 dirty tree: scope yes, validation no | MET | `test-pre-push.sh:933-955`; `test_evidence_reuse.py:1222` | |
| AC5 passing run reused, visible | MET | `test-pre-push.sh:777-812`; `test_evidence_reuse.py:931`; `ci-rollup-tests.rs:2978,3581` | |
| AC6 failing warn run | PARTIAL | `test_evidence_reuse.py:987,433`; `ci-rollup-tests.rs:3128,3178` | Proven piecewise. No hook test drives a failing complete run through publication; every publishing fixture uses a passing fake `just` (`test-pre-push.sh:761`) (T1). |
| AC7 failing strict publishes, `--no-verify` reuses | ABSENT | `pre-push:586-589` | See D1. |
| AC8 incomplete cells stay scheduled | MET at verify layer | `schema.py:776-783`; `test_evidence_reuse.py:445,460,685,1008,1129` | Hook interruption leaves nothing behind by shell default: no `trap INT` (`pre-push:161,573`) (W8). |
| AC9 scope-only | MET | `test-pre-push.sh:359-368,871-893` | |
| AC10 off alias | MET | `test-pre-push.sh:370-379` | Message drift (W11). |
| AC11 first `--no-verify` | VACUOUS | `test_ci_local.py:994-997`; `test_evidence_reuse.py:842-851` | As the 09-10 spec itself concedes. |
| AC12 local failure does not short-circuit | MET, unit level | as R7 | No workflow-level fixture combines a reused failed cell with executing environments; hosted proof belongs to B5. |
| AC13 multiple receipts combine | MET; conflict rule differs | `test_evidence_reuse.py:280,813,1245` | See D2. |
| AC14 dispatch ignores evidence | MET | `ci.yml:139-140,157`; `test_ci_local.py:1030-1035` | No step test passes a validation receipt under `workflow_dispatch` (T4). |
| AC15 summary provenance | PARTIAL | `ci.yml:224-245` | As R9 (W5). |
| AC16 test coverage | PARTIAL | matrix below | |

## Decisions the audit could not make (need Ken)

### D1 — Does a complete failing `strict` run publish its validation note?

09-10 R4 and AC7 say yes: publish before blocking, so a later `--no-verify`
push of the unchanged tree reuses the known failure instead of re-running it
on a hosted runner. The hook says no (`pre-push:586-589`, "not published while
the push is blocked"), on the reasoning that the branch is not going anywhere.
Three documents say strict and warn both publish (`.github/ci/README.md:154`,
`docs/topics/ci-cd.md:75`, `.claude/skills/rust-devops/ci-cd.md:105`, and the
hook header `pre-push:4-8`). Code, docs, and spec disagree three ways, and the
test that pins the code greps the hook's message text
(`test-pre-push.sh:733-738`). No ruling in this spec authorizes the deviation.

**Ruled 2026-09-12: publish.** See the spec's Rulings, D1.

### D2 — Cross-note conflicts for one cell: newest wins, or schedule the cell?

09-10 R6 says duplicate receipts for one cell must agree, and a conflict
schedules the cell in CI. The implementation resolves newest-wins across notes
(`local_evidence.py:233-238`; tests `test_evidence_reuse.py:734,751`): an older
pass never overrides a newer failure, but a newer pass does override an older
failure on the same tree. Within one document a conflict still rejects the
whole receipt (`schema.py:665-670`). The `conflicting-evidence` code never
fires across notes.

**Ruled 2026-09-12: newest wins; W14 becomes required.** See the spec's
Rulings, D2.

## Work items a formal implementation would have included

Ordered by how much they change behavior. None needs a ruling.

- **W1** Bind the validation receipt to the scope receipt. Either record
  validation against the trigger-review plan instead of the `just pre-push`
  replan, or make `verify_cells` compare `scope_identity` rather than
  shape-check it. Today AC1's "once" is false and nothing notices.
- **W2** Drop the ancestor requirement from `record_scope`
  (`local_evidence.py:473`). Scope identity is `{base, head, tree}`; the hook
  already plans CI's exact two-dot diff against an advanced target tip
  (`pre-push:298-300`), then discards the receipt. `test-pre-push.sh:1488-1525`
  locks the waste in and must change with it.
- **W3** Move `rustup show` (`ci.yml:87-88`) below the receipt decision so a
  hit installs no toolchain, and extend the step-level contract test to see it.
- **W4** Wrap `verify --cells` and `--apply-to` in the scope job so a verifier
  crash yields "no accepted cells" plus a summary line, per R8's add-work rule.
- **W5** Add scope-summary rows for matching environments, reused pass/fail
  cells, and retained-incomplete cells from `accepted_evidence` and
  `evidence_rejections`; today the rejections reach the plan artifact only.
- **W6** Drop the legacy `scope` projection from the receipt and project in
  CI, since CI re-projects on every hit and the projection is a pure function
  of the plan (`affected_scope.py:2123-2192`).
- **W7** Set `reusable=False` on the lint cell (`affected_scope.py:1470-1478`),
  mirroring `check`, so the equivalence rule is policy rather than an accident
  of what the producer records.
- **W8** Decide interruption explicitly with a `trap INT` in the hook: either
  abort and publish nothing (today's accidental behavior made deliberate) or
  publish completed siblings marked `interrupted`. Today `interrupted` has no
  producer, and a `just` that exited 130 normally would publish a partial
  manifest as complete.
- **W9** Mark `browser` non-reusable or record why it may be, since 09-10
  lists browser reuse out of scope and this spec is silent.
- **W10** Print a reason for each failing guard clause in the validation
  block (`pre-push:536-541`); a gate that dirties the tree currently withholds
  evidence silently. Replace the literal `origin/main` base at `:541` with the
  trigger review's base.
- **W11** Fix the unknown-mode message (`pre-push:99`) and its pin
  (`test-pre-push.sh:386`) to name `scope-only`, `warn`, `strict`.
- **W12** Delete `verified_environment` (`local_evidence.py:983`) and the four
  `test_today_*` tests that pin it (`test_evidence_reuse.py:307,531,538,852`).
- **W13** Fix the five `SC2086` findings `actionlint` reports in
  `_package-ci.yml` (`:168,259,466,626,792`).
- **W14** (found 2026-09-12 while ruling D2) The hook verifies and applies
  published evidence for the trigger review and the plan render
  (`pre-push:467-476`) but never for its own local gates: `just pre-push`
  runs `ci-local`, whose evidence overlay is confined to `--plan`
  (`just/ci-local.just:161-176`). A follow-up commit that changes only
  documentation therefore reruns every gate of every package changed since
  the pull request's base, although the previous commit's note already covers
  those cells under gate-input equivalence and CI itself would reuse them.
  Apply the accepted set to the local run so the hook skips what CI would
  skip, and record the remaining cells only. This also shrinks D2's conflict
  case to explicit reruns.

## Documentation drift (R10)

- `README.md:117` describes pass-only, whole-environment reuse and the
  retired reverse-dependency compile cell.
- `README.md:129` lists `off` as "skip validation entirely"; no `scope-only`
  row; no mention of `--plan` or constraints anywhere in the root README.
- `docs/testing-strategy.md:601-602,634-645,673-683` describes the retired
  behavior as current and names the 09-10 spec as "in flight".
- `.claude/skills/rust-devops/ci-cd.md:319-320` predates per-cell reuse.
- The "strict and warn both publish" claim in four places (D1).
- The hook's unknown-mode message (W11).

## Test coverage matrix (AC16)

| Path | Behavioral coverage |
|---|---|
| Clean strict pass | hook `:777`; evidence `:931`; ci-local `:964`; rollup `:2867`; contracts `:2371` |
| Warn complete failure | evidence `:987,:433`; rollup `:3128,:3178`; **hook: none end to end** (T1) |
| Strict failure publishes before blocking | **none**; `:733` greps the opposite (D1, T2) |
| Interrupted / incomplete cell | evidence `:445,:460,:685,:1008,:1129`; rollup `:1054`; hook: none (W8) |
| Stale receipt | local_evidence `:274-315`; ci-local `:989-1021`; evidence `:351,:1289`; hook `:1488` |
| Malformed receipt | local_evidence `:305`; evidence `:520,:543,:716`; rollup `:3669`; ci-local step: none (T5) |
| Conflicting receipts | evidence `:494` (one note); cross-note is newest-wins `:734` (D2) |
| Two matching receipts merged | evidence `:280,:813,:1245`; affected_scope `:241`; resolved_plan `:484`; contracts `:2725` |
| scope-only | hook `:359,:871,:602` |
| off alias | hook `:370,:381` |
| No receipt / first `--no-verify` | local_evidence `:264`; ci-local `:995`; evidence `:845` |
| Dirty tree: scope yes, validation no | hook `:933,:700`; evidence `:1222` |
| Area override yields no reusable validation | **none behavioral**; `:716` greps the hook (T3) |
| Dispatch ignores evidence | ci-local `:1030,:941`; reuse_validation `:132`; validation receipt under dispatch: none (T4) |
| Valid receipt invokes no planner or Cargo | ci-local `:964,:1064` non-vacuous; toolchain: none (W3) |

Missing tests: **T1** hook test with a failing fake `just` and a staged
failing report asserting `Published … cell(s)`; **T2** the strict-failure
behavior per D1, replacing the source grep; **T3** a behavioral override
test asserting no validation note; **T4** a validation receipt under
`workflow_dispatch` at the step level; **T5** a malformed scope receipt at the
step level. The 09-10 verification-plan bullet for `just ci-local --dry-run`
fixtures has no test; `test_ci_local.py:569` asserts only that the flag exists.

Pending-contract wrappers cover none of these paths; they wrap only the three
OQ3 and OQ4 contracts.

## Commands run by the auditors

- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`: 400 passed.
- `env -u CDPATH GIT_TERMINAL_PROMPT=0 /bin/bash ./.githooks/tests/test-pre-push.sh </dev/null`: 53 passed, 0 failed.
- `actionlint` on `ci.yml`, `_area-ci.yml`, `_package-ci.yml`: exit 1, five
  `SC2086:info` findings, all in `_package-ci.yml` (W13).
