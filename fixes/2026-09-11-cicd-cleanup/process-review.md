---
title: Process review — how six review/fix cycles happened and how to close a spec faster
kind: process-review
created: 2026-09-12
for: fixes/2026-09-11-cicd-cleanup/spec.md
inputs:
  - review-1.md … review-6.md
  - plan.md
  - log.md
  - open-questions-and-blockers.md
  - prompts/plan.md
  - prompts/_reviews/feature-review.md (+ _senior-reviewer.md, _ready.md)
  - prompts/_implement/implement-suggestions.md
  - just/flow.just, just/review.just, just/plan.just
status: proposal
---

# Process review

This fix has consumed six review/fix cycles in thirteen hours and is not
closed. This document reconstructs what the cycles actually did, names the
causes that made them slow, and proposes changes to the prompts, the just
recipes, the conventions, and the skills so that the next spec closes in
fewer cycles. The evidence is the six reviews, the plan, the log, and the
prompts that composed each agent's instructions.

## 1. What happened

| When (09-11/12, local) | Event |
|---|---|
| 13:40 | spec created |
| 14:24 | spec review adds four open questions, each saying implementation of the affected section must not start until Ken rules |
| 14:25 | plan drafted; Phase 1 finds the prerequisite spec (09-10) unimplemented and **stops the plan** on blockers B0 through B5 |
| 14:25 to 17:58 | all nine plan phases executed anyway by the yolo implementer, each phase marking its OQ-gated work "blocked" |
| 18:01 | review 1: 10 high findings, 4 human-review items (the four OQs) |
| 18:31 to 00:05 | cycles 1 through 4: each implementation fixes what it can and defers the same three findings; reviews 2 through 5 re-report them and repeat the same four human items |
| 00:15 to 01:27 | Ken rules OQ1 through OQ4, B0, B5, D1, D2 in one interactive session; scratch-repository experiment run and recorded |
| 01:27 to 02:38 | cycle 5 implements review 5 plus three of the rulings |
| 02:39 | review 6: 7 high findings, several of them rulings the cycle did not implement, plus the absorbed 09-10 work |
| 09:22 | cycle 6 starts |

Spec to review 6: just under thirteen hours, six reviews, five completed
implementations, plus a three-and-a-half-hour plan execution before the
first review.

## 2. Where the time went

**Four cycles produced almost nothing new.** Reviews 3, 4, and 5 each
contain roughly 15 to 20 lines of genuinely new finding content inside 130
to 170 lines. The rest is repeated structure: the same 17-row
requirement-to-verification map (three rows read "Appropriate; passed" in
five consecutive reviews), the same L1/L2/L3 disclaimer, the same "no pushes
were performed" trailer, the same GitNexus paragraph, the same paragraph
about a broken `previous` path, and the same three deferred findings
re-narrated at six lines each.

**Three findings were deferred four times for one reason.** The standalone
verdict job, the accepted-gap publisher, and the constraint-store default
each waited on a ruling (OQ3, OQ4, OQ2) that the spec had said was required
before implementation. The log recorded "the ruling costs one line in
`constraints.default_directory()`" in cycle 1 and again in cycles 2, 3, and
4. The four human-review items were listed, lightly reworded, in reviews 1
through 5. Nothing in the loop surfaced them to Ken as a blocking request;
they were answered six hours after review 1, in a session outside the loop.

**One contract was re-found five reviews in a row.** The pre-push trigger
review (spec §4, AC7, AC17) was fixed in cycle 1 for a dirty tree, re-found
in review 3 for a push carrying more than one ref, fixed for that, re-found
in review 4 for a stacked pull request's base, fixed for that, re-found in
review 5 for a simultaneous head-and-target update, fixed for that, and only
closed in review 6. Each cycle fixed exactly the case the reviewer
reproduced and tested exactly that case.

**A fix was lost between cycles.** Cycle 3's Bash 3.2 harness fix passed in
review 4, but the commit that closed cycle 3 was made from a tree that did
not include it, so cycle 4 restored it.

**Rulings reached implementation one cycle late, and partially.** The
rulings landed in the spec after review 5 was written. Cycle 5 read them
and implemented three (OQ2, OQ3, OQ4) but not D1, D2/W14, or OQ1, and left
a test in place that asserts the behavior D1 rejected. Review 6 therefore
opens with "the implementation does not satisfy several explicit September
12 rulings", which is a review cycle spent discovering a spec change that
was already on disk when the implementation started.

**The plan was executed once and never driven again.** All nine phases ran
in one afternoon. After review 1 no review and no log entry cites a plan
task; the plan's status stayed `blocked`, its tallies were never updated,
and its execution-constraint sentence ("no task authorizes a commit, push,
workflow dispatch, ruleset edit, or WSL rerun") was the only place the
authorization boundary was written down, where the loop never read it.

**The automation was not running.** `just flow`, `just
plan-implement-review`, `just review`, and `just implement-feature-review`
compose `prompts/review-feature.md`, `prompts/implement-feature-review-
suggestions.md`, `prompts/implement-phase.md`, `prompts/implement-review.md`,
and `prompts/review-implementation.md`. None of those files exist. The
cycles were driven by hand against `prompts/_reviews/feature-review.md` and
`prompts/_implement/implement-suggestions.md`, so the flow's own safeguards
(the commit at each cycle boundary, the iteration counter, the ready check)
did not apply. That is how rulings sat uncommitted at HEAD while review 6
reviewed "HEAD plus the working tree", and how a working-tree fix was
dropped by a hand-made commit.

**The log's headings did not match its bodies.** Every cycle section is
titled "Successful Completion". Cycles 1 through 4 each deferred three of
their findings under that heading; cycle 4 says "deferred for the third
time" when it was the fourth; cycle 5 says "4 fixed, 0 deferred" and then
lists two owed items that review 6 reopened. Cycles 1, 2, and 6 started
without reading the previous cycle's section.

## 3. Root causes

### RC1. Human decisions were modeled as after-the-fact review, not as gates

`prompts/_ready.md` tells the reviewer that the need for human review must
not affect readiness, because "we will treat human based reviews as being an
external process … that will happen after we've completed the review/fix
cycle". That is the right rule for acceptance testing. It is the wrong rule
for a design decision the spec says must precede implementation. The loop
had no state for "cannot converge until a human rules", so it produced four
not-ready reviews whose blocking findings were unfixable by construction,
and `just flow` would have stopped only at its five-iteration cap.

### RC2. The loop's unit of work is the review, not the spec

`implement-suggestions.md` iterates the review's findings serially and
implements all of them. It does not read the spec's `## Rulings`, does not
diff the spec against the commit the review was written from, and does not
triage findings by whether they can be implemented at all. Two consequences:
a spec change reaches code only after one more review discovers the gap, and
findings blocked on a ruling or an authorization are re-attempted, re-
deferred, and re-found every cycle.

### RC3. The authorization boundary had no representation

Every cycle held findings that needed a push, a workflow trigger, a scratch
repository, or a ruleset write. The implementer is non-interactive and may
not do those. The prompts say "you are non-interactive" but do not say what
to do with a finding that needs an authorized action: mark it, hand it off,
and stop counting it as deferred work. Reviewers likewise never had a
category for "this requires the human to act, not decide", so hosted
verification was reported as a high finding six times.

### RC4. Every review re-audits the whole spec from zero

`feature-review.md` is a first-review prompt run six times. Its only
iteration awareness is a note that "the prior review's suggestions have now
all been implemented", which was false every time. There is no delta mode,
no findings ledger with stable identifiers, and the verification map is
re-emitted rather than maintained. The test-rigor section (`_test-rigor.md`)
is written for terminal UI (WezTerm, keyboard injection) and does not fit a
CI spec, so each review spent a paragraph explaining why L2 and L3 do not
apply and then invented "hosted integration" as the missing boundary.

### RC5. Fixes reproduce the case, not the contract

The implement prompt sends each finding to a subagent to "implement the
suggestion" and add tests for it. Nothing asks the subagent to enumerate the
contract's state space before fixing one state. The trigger-review chain is
the direct result: one ref, several refs, a stacked PR, a simultaneous
update, a deleted target, each fixed one cycle apart.

### RC6. Tests that pin a rule outlive the rule

`pending_workflow_contract` fixtures pass by asserting that requested
behavior is still absent; two hook tests pass by grepping the hook's source
for a message string. Reviewers flagged both patterns in every review. When
D1 reversed the strict-failure rule, the grep pin stayed green through a
whole cycle because nothing connects a ruling to the tests that encode the
rule it replaced.

### RC7. Plan and loop are disconnected, and the prerequisite was found late

The plan prompt produces phases and tasks; the review/fix loop never reads
them. The plan's Phase 1 audit found a dependency spec unimplemented, which
should have preceded planning, and the absorption then happened silently
across cycles until B0 formalized it. The plan's `status: blocked` blocked
nothing.

### RC8. Recipes drifted from prompts

Five prompt paths referenced by the just recipes do not exist. Manual
driving removed the cycle-boundary commit and the iteration bookkeeping, and
the spec never received the `flow_iteration` key the flow expects.

## 4. Recommendations

Ordered by how many cycles each would have saved here.

### R1. Rule before you plan

- Treat a spec's `## Open Questions` as a gate. Add a `prompts/rule.md`
  prompt (interactive, one question at a time, each with the issue, the
  options, and a recommendation, exactly the shape used on 09-12) that
  writes `## Rulings` into the spec and marks each OQ ruled.
- `prompts/plan.md` refuses to plan a section whose OQ is unruled, or plans
  it as an explicitly gated phase with no tasks. Today the plan planned
  everything and then marked most of it blocked.
- Give the review a second flag beside `human_review`: `blocked_on_human`,
  with `blocking_items`. `_ready.md` keeps its rule for acceptance items and
  gains the inverse rule for blocking ones. `just flow` halts on
  `blocked_on_human`, prints the items, and exits with a distinct status
  instead of looping to the cap.

Saved here: reviews 2 through 5 collapse to one cycle after a one-hour
ruling session. The scratch experiment took under an hour once authorized;
it blocked three findings for six hours.

### R2. Make the spec the loop's input

- `implement-suggestions.md` reads `## Rulings` first, then diffs the spec
  against the commit the review names. If the spec changed after the review
  was written, the implementer either runs a delta review first or treats
  each ruling as a finding with priority above the review's own.
- Findings get stable identifiers (`F-11`, `F-12`) and live in a ledger
  (`findings.yaml` in the fix directory) with `status: open | fixed |
  blocked-ruling | blocked-authorization | superseded`, the review that
  opened it, and the cycle that closed it. Reviews append and update the
  ledger; they do not re-narrate it. The implement prompt reads the ledger,
  not the review body.

### R3. Represent the authorization boundary

- A finding may declare `needs: push | dispatch | scratch-repo | ruleset |
  ruling`. The implementer skips those, lists them under a `handoff`
  heading in the log, and does not count them as deferred.
- Add a `prompts/authorized-actions.md` prompt run interactively by Ken
  with an agent: it reads the handoff list and performs each action with
  confirmation, recording results under `fixtures/`. The 09-12 scratch
  experiment is the template.
- Record the scratch-repository recipe in the `rust-devops` skill so
  proving a GitHub semantic is a twenty-minute step: create a public
  scratch repo, reduced workflow, scenario file, one PR per case, read
  `mergeStateStatus` and `statusCheckRollup`, delete. Include the two facts
  it produced: the ruleset `workflows` rule is organization-only, and
  `continue-on-error` turns a job's `needs` result into `success`.

### R4. Review in delta mode after the first review

- For `iteration > 1`, the review prompt: verifies each ledger item marked
  fixed (regression check), reviews only code changed since the previous
  review's commit plus any spec change, and re-emits nothing that has not
  changed. The verification map becomes a maintained file
  (`verification-map.md`) edited in place.
- Replace the terminal-specific rigor rubric with a per-spec declaration.
  The spec's frontmatter names its verification boundaries (for this fix:
  `boundaries: [L1, hosted-integration]`); `_test-rigor.md` is
  parameterized on that list and defines "hosted integration" alongside L1
  through L3.
- Fix the two boilerplate generators: the `previous` path in
  `feature-review.md` resolves relative to the prompt's directory rather
  than the fix directory (five reviews carried a paragraph about it), and
  the GitNexus disclaimer belongs in the prompt's instructions, not in
  every review's body.

### R5. Fix the contract, not the case

- Add to `implement-suggestions.md`, and to the `rust-testing` skill: when a
  finding is one instance of a contract, enumerate the contract's state
  space in a table before fixing the instance, and write the test
  table-driven. For the trigger review the states were: one ref, several
  refs, a stacked target, a simultaneously updated target, a deleted target,
  a target that advanced. One cycle instead of five.

### R6. Tests that pin a rule must name the rule

- Convention: a `pending_workflow_contract` fixture or a source-grep pin
  carries the OQ, D, or W identifier that gates it in its name or a comment.
  `implement-suggestions.md` gains a step: for every ruling newer than the
  review, grep the test suites for the identifier of the rule it supersedes
  and replace those tests in the same cycle. The `rust-testing` skill's
  pending-contracts section states that a pending contract must be
  time-boxed to the ruling that gates it and deleted when the ruling lands.

### R7. Either drive the plan or retire it

- If the plan is to be driven, the implement prompt maps each finding to a
  plan task and re-tallies the phase; the plan's `status: blocked` blocks
  the loop.
- If the plan is a one-shot execution script, say so in `plan.md`'s
  frontmatter (`mode: one-shot`) and stop pretending it is a tracker.
- Add a prerequisite step to `prompts/plan.md`: for each `depends-on` spec,
  audit its acceptance criteria against the tree before planning, and
  either require it landed or record an absorption ruling up front. Phase 1
  did this audit; it should have been a planning precondition.

### R8. Repair the recipes and enforce cycle-boundary commits

- Point the five broken prompt references at the files that exist, or
  create the files they name. Add a test that every `claudine compose
  @prompts/...` path in `just/` resolves.
- Every cycle ends with a commit of the whole working tree (the flow's
  `commit_flow_changes`), and each review records the commit it reviewed.
  The implement prompt refuses to start if the review's commit is not the
  current HEAD, which would have caught both the lost Bash 3.2 fix and the
  uncommitted rulings.

### R9. Honest cycle headings and warm starts

- The closing heading is `### Cycle N: X fixed, Y deferred, Z handed off`,
  generated from the ledger. "Successful Completion" is reserved for a
  cycle with nothing deferred and nothing handed off.
- The log section carries an `owed:` list in its frontmatter; the next
  cycle's prompt requires reading the previous section and confirming each
  owed item before starting new work.

## 5. Skills that would have helped

| Skill | Gap it would close |
|---|---|
| **New: `spec-lifecycle`** | The whole loop has no written model: spec → open questions → rulings → plan → implement → review loop, with states (draft, awaiting-ruling, planned, in-cycle, blocked-on-human, handed-off, ready), who acts in each, what stops the loop, and the ledger, rulings, absorption, and verification-boundary conventions from R1 through R9. Every prompt above would load it. |
| **`rust-devops`** (extend) | The scratch-repository recipe and the two GitHub facts it produced (R3); the hosted-integration verification boundary and what it can and cannot prove; the rule that a merge-gate change lands ruleset edit and workflow change together. |
| **`rust-testing`** (extend) | Contract state-space enumeration (R5); "a test that pins a rule names the rule" and pending contracts time-boxed to their ruling (R6); the hosted-integration boundary added to the L1/L2/L3 taxonomy so reviewers stop inventing it. |
| **`claudine`** (extend) | A note that prompt `$schema` file references resolve relative to the prompt's directory first, which is why `previous` broke, and the recommended `dir=` pattern to anchor paths in the fix directory. |

## 6. What the same fix would have looked like

With R1 in place: one ruling session after the spec review, before
planning, settling OQ1 through OQ4 and B0 (the prerequisite audit runs as
part of planning) in about an hour. The scratch experiment runs the same
afternoon under R3. The plan is written against ruled decisions, so no
phase is born blocked.

With R2, R5, and R6: review 1 still finds ten items, but cycle 1 fixes the
trigger-review contract across its state space instead of one case, and the
pending contracts for the merge gate and the publisher are real assertions
because their rulings exist. Review 2 runs in delta mode against a ledger
and is a page long.

The remaining hosted-verification items are handed off, not deferred, and
close in one authorized session. A plausible outcome is two or three cycles
and one ruling session instead of six cycles and counting.
