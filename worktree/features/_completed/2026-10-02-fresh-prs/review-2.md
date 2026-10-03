---
$schema: feature-review.yaml
ready: false
findings:
    - title: PR retries leave completed head progress on the spinner
      priority: high
    - title: Stored-input matrices omit decision fields and receipt behavior checks
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-03T09:49:32-07:00
spec: 2026-10-02-fresh-prs/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/features/2026-10-02-fresh-prs/log.md
description: "A **feature** review of `2026-10-02-fresh-prs/spec.md`"
feature: 2026-10-02-fresh-prs/review-2.md
previous: 2026-10-02-fresh-prs/review-1.md
next: 2026-10-02-fresh-prs/review-3.md
---

# Review 2: Fresh pull requests

**Not production ready.** The three findings from review 1 are addressed. This review found a remaining progress-display defect after a PR retry and incomplete malformed-input regression coverage. Neither finding needs a human decision.

Reviewed the specification, plan, both implementation logs, review 1, the current implementation and uncommitted repairs, the worker and listing flow, changed readers, rendering, fixture cleanup, test targets, and current documentation. Temporary probes exercised the production public APIs with copies of existing fixtures; they were removed afterward, restoring both source files byte for byte. This review changes only the review documents and the specification's review counter.

## Previous findings

Review 1 uses `## Findings` rather than separate unblocked and blocked sections. Its three findings were all unblocked; its frontmatter and repair log identify no blocked or deferred findings.

| Review 1 finding | Verification in this review | Result |
| --- | --- | --- |
| Retries discard results already established by the other half | Both retry routes; replacement launch/id failure, missing receipt, stopped or pending replacement, newer finished head, retained credentials diagnosis, and newer PR result. The new regression tests ran under both CLI targets. | Addressed: independently established results survive replacement failures. |
| Retry paths bypass the original deadline and hide timeouts | PR and head contention before, exactly at, and after the forced deadline; ordinary contention boundary; holder still locked; retained publication; shared retry clock. | Addressed: late retries are refused, and unresolved waits report timeout. |
| Several worker fixtures do not guarantee cleanup after an assertion fails | Reviewed `MixedFixture`, `DesignFixture`, remote fixture, shared `WorkerReaper`, held providers/proxies/Git requests, and directly owned children. Four L1 unwind tests and the tmux unwind scene ran. | Addressed: request release precedes fixture cleanup, both locks and repository-scoped workers are checked, and the original panic survives. |

The cleanup fallback that kills a worker after 20 seconds is bounded but has no dedicated regression test; the required ordinary completion and assertion-failure paths are exercised. This alone is not a readiness finding.

## Unblocked Findings

### High: PR retries leave completed head progress on the spinner

**Defect class:** progress-display state survives a replacement operation even after a newer head phase has overwritten the displayed message, so the next transition to PR-only waiting fails to update the spinner.

In the `worktree-cli` package, [Follow::run](../../cli/src/commands/list/wait.rs) coordinates the two remote operations and tells the spinner what is running. Its `pr_only` flag is initialized once at line 197, set at lines 319–322, and never reset. A forced listing whose first head finishes while PRs are contended sets it to true. The PR retry starts a replacement head; [Follow::observe](../../cli/src/commands/list/wait.rs) sends that head's fetch or fallback phase to the spinner. When that head finishes and the replacement PR request is still pending, `!self.pr_only` is false, so the spinner keeps describing a completed head operation.

For example: first head completes; the PR holder fails without publishing; `--refresh` retries; the replacement fetch completes; PRs remain held. The displayed message remains “pulling remote updates” instead of “updating.” The same defect retains either special fallback message. The final head result and caption are correct; the error is the message shown during the remaining wait. It can persist for the rest of the 75-second forced budget.

Reproduction copied the scripted wait fixture, retained its launch/receipt/lock machinery, and varied only the route and replacement head phase. The production public `wait` API ran twelve cases: four routes crossed with fetching, no-key fallback, and rate-limit fallback. Each head completed before PR resolution. All three PR-retry cases failed the expected final progress-phase assertion; the nine sibling controls passed. Fallback replacements finished with `CheckFailed`, and the fetching replacement finished with `Fetched`.

| Site / sibling route | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Forced PR retry → replacement fetch completes → PR-only wait | First head finished; PR contention clears at 1 s without publication; replacement head finishes at 1.1 s; replacement receipt arrives at 2 s | Last spinner phase is `Fetching` | Last spinner phase is `Checking`, whose text is “updating” |
| Forced PR retry → replacement no-key fallback completes → PR-only wait | Same timings, replacement finishes its fallback with a failed check | Last spinner phase is no-key fallback | Generic “updating” while waiting only for PRs |
| Forced PR retry → replacement rate-limit fallback completes → PR-only wait | Same timings, replacement finishes its fallback with a failed check | Last spinner phase is rate-limit fallback | Generic “updating” while waiting only for PRs |
| Ordinary launch → head completes → PR-only wait | Each of the same three phases; receipt delayed | Generic progress restored — clean | Generic progress |
| Adopted matching head → head completes → contended PR-only wait | Each of the same three phases; PR holder remains locked until 2 s | Generic progress restored — clean | Generic progress |
| Forced head retry → replacement head completes → PR-only wait | Holder for another branch releases at 800 ms; each replacement phase finishes before its receipt | Generic progress restored — clean | Generic progress |
| `phase_text` projection | All phase variants inspected | `NotVisible` shares the no-key message; rejected/other fallback and ordinary checking already map to “updating” | Preserve these mappings; the defect is the missed transition |
| `Progress::show` and listing teardown | Existing spinner tests and terminal captures | Renderer accepts the requested text and clears the line on completion — clean | Correct phase supplied before clearing |

Make the PR-only display state describe the currently followed operation, resetting it when a replacement head starts or a new head phase takes over. Preserve the retained head and PR results introduced by the repair. Add an L1 regression for the PR retry and the three distinct stale messages, with the sibling routes as controls.

**Verification-level gap:** specification section 5 requires the spinner to return to generic “updating” when only PRs remain. Existing L1 coverage proves this transition for an ordinary wait. The tmux stale-PR scene captures “updating,” but its head starts in ordinary checking; it cannot detect failure to replace a fetch/fallback message. The other tmux spinner scenes cover fallback-to-fetch and final clearing, not completed-head-to-PR-only waiting after a retry. Add a Level 2 pane capture that observes the replacement fetch/fallback, releases that head operation while keeping PRs held, then verifies one generic spinner line before final clearing. The high priority includes this missing real-terminal verification of a required display transition.

### Medium: Stored-input matrices omit decision fields and receipt behavior checks

**Defect class:** malformed-input tests describe themselves as complete format matrices but omit nested fields that control badges or credentials messages, and receipt mutations are mostly asserted at the file-reader boundary rather than through the listing's public wait result.

In the `worktree` package, [the PR store matrix](../../lib/src/pull_requests.rs:612) starts with a real writer-produced file and checks the public cached-answer and publication results. Its envelope fields and answer-list shapes are covered, but the nested PR object receives only a few individual edits. In particular, `target_branch` gets one wrong-type test; its missing, null, and duplicate cases are absent. The nested `source_repo` gets only a missing-field test. These fields determine whether a PR belongs to a local branch and where its badge is placed, so they need the same strictness checks as the envelope.

The `worktree` package's [receipt matrix](../../lib/src/remote_head.rs:1398) mutates a credentials-rejected receipt and asserts `load_receipt`. It does not walk the rate-limit `authenticated` field or malformed insufficient-permission variant. In `worktree-cli`, [the real-file wait tests](../../cli/src/commands/list/wait/tests.rs:1294) exercise origin/branch/id/timestamp mismatches and one truncated document, but do not carry the full malformed-field matrix through `wait`. A missing receipt must produce a generic PR failure while preserving a finished head; merely proving that a parser returns `None` does not prove that behavior.

This is a persistent regression-coverage gap, **not a reproduced permissive-reader bug**. Temporary probes copied writer-generated fixtures and applied one edit per cell. The PR public-result probe passed 66 field cells, mixed/all-invalid list-element cases, and invalid tails. The receipt probe passed 170 field cells across rejected credentials, insufficient permissions, and rate limiting through the production public `wait` API, plus positive writer controls and invalid tails. Invalid receipts preserved the finished head, returned generic PR failure, and did not time out. Valid null key fields retained the specific failure without a variable name.

| Site / fields swept | Shapes tested in review | Observed result and permanent coverage | Expected result |
| --- | --- | --- | --- |
| PR store envelope: `format_version`, `publication`, `origin_digest`, `fetched_at` | Absent, null, wrong whole type, empty containers, duplicate, invalid/trailing content | Miss; principal malformed cases have permanent coverage — clean reader | Miss; no usable publication |
| PR store envelope: `source_repo` | Same shapes, with valid null control | Null remains an answer with unknown repository; other malformed shapes miss; principal cases covered | Preserve explicit unknown repository; reject missing/wrong/duplicate field |
| PR answer list: `pull_requests` | Absent, null, wrong whole type, one invalid element among valid ones, all invalid elements, empty, duplicate, invalid tail | Invalid list misses; empty is a valid answer; permanent cases covered — clean | No partial or silently empty answer |
| PR element: `number` | Absent, null, wrong type, empty containers, duplicate | Miss; permanent tests cover principal cases — clean reader | Miss |
| PR element: `url` | Same shapes, valid null control | Null is valid; malformed shapes miss; duplicate coverage absent | Preserve explicit no-URL; reject malformed field |
| PR element: `source_repo` | Same shapes, valid null control | Null is valid; malformed shapes miss; permanent matrix tests only absence | Preserve explicit unknown source; reject malformed field |
| PR element: `source_branch` | Absent, null, wrong type, empty containers, duplicate | Malformed shapes miss; permanent matrix covers absence/null, omits wrong whole type and duplicate | Miss |
| PR element: `target_branch` | Same shapes | Malformed shapes miss; permanent matrix tests only wrong type | Miss |
| Receipt envelope: `format_version`, `attempt_id`, `origin_digest`, `branch`, `finished_at`, `head`, `prs` | Absent, null, wrong type, empty containers, duplicate, invalid tail | Generic failure with finished head retained; permanent loader tests cover envelope, public wait has only selected cases | Same result through `wait`, within the original budget |
| Receipt nested fields: `prs.kind`, `prs.failure`, `prs.failure.kind` | Same shapes, across all three field-bearing failure variants | Invalid receipt becomes generic failure; permanent matrix covers selected mutations of rejected credentials only | Generic failure; no invented credentials diagnosis |
| Receipt failure `key`: rejected credentials, insufficient permissions, rate limited | Absent, null, wrong type, empty containers, duplicate | Null retains the specific failure with no variable name; malformed shapes become generic failure; permanent malformed coverage is concentrated on rejected credentials | Preserve valid null; reject missing/wrong/duplicate key in every variant |
| Receipt rate-limit failure: `authenticated` | Absent, null, wrong type, empty containers, duplicate | Generic failure; permanent malformed matrix omits this field | Reject invalid field, rather than inventing authenticated/unauthenticated status |

The complete field inventories are the six PR envelope fields plus five element fields, and the seven receipt envelope fields plus the nested discriminator/failure fields, variable-name fields, and rate-limit authentication flag listed above. Array-element shapes apply only to `pull_requests`; they do not apply to scalar receipt fields. Empty strings on optional PR metadata and branch-name strings remain distinct string values under the current format, not missing fields; they should have explicit matrix outcomes. Empty identifiers/bindings and invalid variable names are already rejected where validity checks require it.

Extend the existing data-driven tests, rather than adding a separate one-off test per omitted cell. Give the plan's matrix explicit columns for the nested fields, preserving its snapshot of the agreed contract. Keep a positive control for each receipt variant and exercise each receipt edit through real files and the public `wait` result. Verify valid explicit null, valid empty PR answers, unknown fields, and stale/future/misbound publications alongside rejection cases. No production-reader change is requested unless those tests expose a defect.

## Blocked Findings

None. Both findings can be repaired and verified in this non-interactive workflow.

## Recurrence

No finding repeats a defect class reported in review 1. That review concerned lost remote results, retry deadline enforcement, and worker lifetime during failed tests. Those repairs passed their regression tests here. The new progress defect concerns stale display state, and the matrix finding concerns incomplete regression coverage. Review 1's prose overstated the matrix's completeness; the field-by-field audit above corrects that assessment.

## Requirement verification

| User-facing requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| Every eligible sequential listing asks again, including fresh cached answers | Level 1: shipped binary, loopback provider, request counts | Verified |
| Answers arriving within the wait are shown; empty success clears badges | Level 1: binary and post-wait store tests | Verified |
| Ordinary and forced waits preserve head/PR independence, including contention and failed replacements | Level 1: public wait scripts and binary contention tests | Verified, apart from progress display |
| Ordinary wait ends at 3 s; forced retries share 75 s and never launch at/after deadline | Level 1: boundary scripts; serial performance checks | Verified |
| Stored/no/empty answers display correct failure, age, hint, and ordering | Level 1 presentation snapshot and binary tests; Level 2 tmux style/pane captures | Verified |
| Spinner returns to generic progress once only PRs remain, including retries | Level 1 ordinary transition; Level 2 ordinary checking and final clearing | Incomplete: reproduced retry defect and missing representative Level 2 transition |
| Dim status text and credentials line; spinner cleared before final output | Level 2 tmux captures | Verified |
| Credentials diagnosis and head-warning precedence; ambiguous errors remain generic | Level 1 binary/renderer tests; Level 2 credentials capture | Verified on valid inputs; malformed-field matrix gap above |
| No origin, ignored/unsupported origin, changed origin, fork filtering | Level 1 binary, gathering, and matching tests | Verified on valid inputs; nested-field malformed matrix incomplete |
| PR failure does not prevent permitted fast-forward | Level 1 binary test | Verified |
| Only the worker writes PR answers; every worker attempt writes its own receipt; independent worker halves | Level 1 worker tests, call-site inspection, real-file tests | Verified |
| Malformed stores are misses; malformed receipts preserve independent results | Level 1 existing reader tests and temporary public-result matrices | Reader behavior verified; permanent matrix coverage incomplete |
| Workers/locks cleaned up on assertion failure, without replacing the original failure | Level 1 unwind tests and Level 2 tmux unwind scene | Verified |

No requirement introduces keyboard, paste, IME, or mouse handling, so Level 3 is not needed. No cross-OS proof or additional human review is used as a readiness condition.

Current README, list topic page, performance topic page, and worktree skill describe the every-run query and shared wait. The repair documentation also describes result retention and the deadline. No additional documentation drift was identified outside the matrix coverage claims described above.

## Validation performed

- `just test`: **866 passed**, 30 excluded tests.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2`: **30 passed** in the runner; backend proof records **24 actual tmux executions**, zero tmux skips/panics. The changed PR status scenes and new unwind scene executed. Some unrelated Kitty pixel assertions reported unavailable screenshots internally; their passing runner status does not prove those pixels were checked.
- `just lint`: **passed** for both packages.
- `just check-tier-coverage worktree`: **zero stranded tests**. Terminal targets are declared, require `terminal-tests`, and have a live L2 recipe and CI feature declaration.
- Temporary public-wait progress sweep: **three failing PR-retry cases, nine clean sibling controls**.
- Temporary writer-generated PR matrix: **66 field cells passed**, plus positive control, invalid list-element cases, and invalid tails.
- Temporary writer-generated receipt matrix through public `wait`: **170 field cells passed**, plus three variant controls and three invalid tails.
- `just test-perf`: **30 passed**, run serially after other test suites finished. Held PR samples recorded a 3.00 s remote wait and 3.10–3.11 s total command time.

Passing the existing gates does not close the reproduced display defect or the missing regression coverage. Repair both unblocked findings before the next review.
