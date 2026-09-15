---
created: 2026-09-14
status: proposed
implemented: false
reviewed: false
area: repository-ci
related:
    - fixes/2026-09-13-cicd-redundancies/spec.md
---

# Three Conventions Agents Keep Getting Wrong Because Nobody Wrote Them Down

## Objective

Three rules govern how agents work with this repository's tests, CI scope, and
feature lifecycle. All three are real, two are already half-implemented in
code, and none is written anywhere an agent reads. Each has cost work at least
once. Write them down, and fix the one place where the code contradicts the
rule.

This fix adds no capability. It closes the gap between what the repository
actually does and what its instructions say.

## Why Now

These surfaced while auditing 233 agent-memory files before deleting them on
2026-09-13. Memory loaded into an orchestrator's context but not into its
subagents', so the two acted on different rules. The durable rules among them
belong in skills and `CLAUDE.md`, which every agent reads. These three are the
ones that survived the relevance test and are not covered anywhere today.

## R1. Fail-fast Is an Environment Policy, Not a Flag Preference

### The rule

**Locally, fail fast.** This is nextest's default and it is the right one.
Cycle time is short; the first failure is usually enough to act on, and if
more than one test is broken, fixing the first surfaces the next. A complete
failure list is not worth the wait when the next run is a minute away.

**In CI, run to completion.** Here the trade inverts. CI is desperate to be
fast, but completeness wins — especially for the legs that do not run on the
author's host. A truncated Windows or WSL report costs a full round-trip,
measured in hours, to learn what the second failure was. Paying for the whole
report once beats discovering failures one per run.

The distinction is not "which flag is better." It is **how expensive the next
run is.** That is what makes it an environment policy.

### What is already true

CI implements this. `--no-fail-fast` is passed explicitly at:

- `.github/workflows/_package-ci.yml:438`, with a comment giving this exact
  reasoning;
- `.github/workflows/_wsl-ci.yml:648`;
- `just/ci-local.just:462`, so local CI simulation matches hosted CI.

Nothing needs to change in any of those. The policy is correct; it is
undocumented.

### What contradicts it

`_test_workspace` (`just/devops.just:1206`) hardcodes `--no-fail-fast` for a
**local** workspace-wide sweep. Its comment says the flag "preserves the old
workspace loop's promise that one failure does not hide the rest."

That is a defensible reading — a full-workspace sweep is CI-shaped in
character, and re-running it is not cheap. But it is currently an unexplained
exception to a rule nobody has stated, which is how exceptions become
confusion. **Decide it explicitly and record the reasoning next to the flag**,
whichever way it goes. Either:

- keep it, and say the rule keys on cost-of-next-run rather than on
  local-versus-CI, so a full workspace sweep sits on the CI side; or
- remove it, and say the rule is strictly environmental.

The first is closer to the underlying principle. The plan should confirm
whether any caller depends on the current behavior before changing it.

### Where it gets documented

`.claude/skills/rust-testing/SKILL.md`, in the section covering the canonical
`just` recipes. State the rule, the reason (cost of the next run), the fact
that CI already passes the flag so an agent does not need to add it, and the
`_test_workspace` decision.

### Interaction with proving a fix non-vacuous

The procedure for proving a guard fix non-vacuous — neuter the guard, confirm
the new tests go red, restore — needs the complete failure list when it runs
against a CI-shaped or multi-package scope, because a truncated list looks
exactly like a narrow blast radius, which is the opposite of what the proof is
for. Against a single package locally, fail-fast is fine and faster.

Document that as a consequence of R1 rather than as a separate always-use-this
rule. An earlier draft of this guidance said "always run the neutered pass with
`--no-fail-fast`", which is wrong locally and is what prompted this fix.

## R2. Test-scope Discipline Applies to CI/CD, Not to Local Work

### The rule

Before adding a CI run, a matrix cell, a fixture, or a gate, ask what question
it answers and whether something cheaper answers the same question. Never
trigger a full-scope run to learn something about UI behavior or ruleset
semantics that could be read or reasoned out.

CI turnaround here runs to hours. A speculative cell is not a small cost.

### Scope

**This is a CI/CD rule.** It does not extend to local testing. Over-testing
locally has not been a problem in this repository, and a rule that discourages
local testing would do more harm than good — local runs are cheap and catch
things early. An earlier framing of this guidance was unscoped and would have
discouraged exactly the testing that should be encouraged.

### Relationship to existing guidance

`CLAUDE.md`'s "Evidence Reuse and Execution Constraints" section covers
*reusing* qualifying passing evidence instead of re-running it. R2 is the
upstream question: whether the cell should exist at all. The two are
complementary and should sit near each other.

### Where it gets documented

`CLAUDE.md`, adjacent to "Evidence Reuse and Execution Constraints", with the
CI/CD scope stated explicitly so it cannot be read as a blanket
discouragement of testing.

## R3. Moving a Feature or Fix to `_completed/` Is Not an Agent's Call

### The rule

An agent never moves a feature or fix directory into `_completed/`. That move
is the author's, made after the review cycle closes. An agent's terminal state
is "implementation complete, ready for review."

### Why

`CLAUDE.md`'s "Features and Fixes" section says completed work "is moved to
`_completed`" using the passive voice, and never says by whom. An agent read
that as licence and self-archived `darkmatter/fixes/2026-08-12-optional-params/`
in commit `9adfa92a4`. That broke a review run whose prompt referenced the
original path — the document the review was about had moved out from under it.

The failure mode is worse than a wrong edit. Archiving asserts that a review
cycle concluded, which is a claim about a process the agent cannot observe.

### Where it gets documented

`CLAUDE.md`, in the "Features and Fixes" section, replacing the passive
sentence with one that names the owner. One line is enough; it is a
prohibition, not a procedure.

## Scope

In scope:

- The three documentation changes described in R1, R2 and R3.
- The `_test_workspace` decision in R1, and a comment at that line recording
  the reasoning either way.
- Refreshing skill hashes with `md hash` for any skill file touched.

Out of scope:

- Changing how CI invokes tests. R1 documents behavior that is already
  correct in every CI path.
- The remaining promotion candidates from the same 2026-09-13 audit — git
  safety discipline, status-writing conventions, exit-code masking when
  piping a test run, stale binaries after `just lint`, and the
  `cargo install --locked` mechanism. Those are separate changes; this fix
  covers only the three the author has ruled on.
- Any change to the memory system itself. It is disabled; that decision is
  recorded in the memory directory's own index and needs nothing here.

## Acceptance Criteria

1. `.claude/skills/rust-testing/SKILL.md` states the fail-fast policy, the
   cost-of-next-run reasoning behind it, and that CI already passes
   `--no-fail-fast` so an agent does not add it.
2. The same skill documents the non-vacuous-proof consequence: the complete
   failure list is needed for CI-shaped or multi-package scopes, and fail-fast
   is correct for a single package locally.
3. `_test_workspace` either keeps or drops `--no-fail-fast` by explicit
   decision, and the line carries a comment giving the reasoning. If the flag
   is removed, no caller regresses.
4. `CLAUDE.md` states the CI/CD test-scope rule with its scope explicitly
   limited to CI/CD, positioned adjacent to the evidence-reuse section.
5. `CLAUDE.md`'s "Features and Fixes" section names the author as the only
   party who moves work into `_completed/`, and states that an agent stops at
   "ready for review".
6. Skill hashes refreshed with `md hash` for every skill file changed.
7. No CI workflow file is modified.

## Verification

These are documentation changes plus at most one flag. Verification is:

- `just --summary` parses in any package area whose justfile changed;
- if `_test_workspace` changes, one local run of it showing the intended
  behavior, and confirmation that its callers still pass;
- `md hash` output matching the committed hashes.

No CI evidence is required. Nothing here changes what CI runs.
