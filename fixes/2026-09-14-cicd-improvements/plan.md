---
created: 2026-09-14
total_phases: 4
phase: 1
agent: claude/opus
yolo: true
spec: fixes/2026-09-14-cicd-improvements/spec.md
status: proposed
---

# Plan — Three Conventions Agents Keep Getting Wrong

## Summary of Work and Definition of Done

### What this actually is

Three rules (R1 fail-fast policy, R2 CI/CD test-scope discipline, R3 who moves
work into `_completed/`) are real and enforced socially but written nowhere an
agent reads. This fix writes them down and resolves the one place where code
contradicts R1.

The deliverable is **four files at most**:

| File | Change | Rule |
|---|---|---|
| `.claude/skills/rust-testing/SKILL.md` | New prose + `hash`/`last_updated` refresh | R1 |
| `CLAUDE.md` (`AGENTS.md` is a symlink — no second edit) | New scoped section + one rewritten sentence | R2, R3 |
| `just/devops.just` | Explicit decision on one flag + a comment recording the reasoning | R1 |
| `docs/testing-strategy.md` | Reconcile **only if** the `_test_workspace` flag changes | R1 drift |

No CI workflow file is touched (AC7). No capability is added. No test is
written, because nothing executable changes unless the R1 ruling removes the
flag — and even then the change is one word in a `just` recipe.

### Research already done (do not repeat it)

These facts were verified against the working tree while drafting this plan.
They are the evidence base for Phase 1's rulings.

1. **`_test_workspace` is not always a full-workspace sweep.**
   `just/devops.just:1118` is `_test_workspace *selectors=""`, and the root
   `justfile:75` is `test *args="": @just _test_workspace {{ args }}`. So
   `just test claudine` is a *single-area* run that still gets
   `--no-fail-fast`. The spec's proposed justification for keeping the flag
   ("a full workspace sweep sits on the CI side") therefore does not cover
   every invocation of the recipe. **This is the central fact the R1 ruling
   has to answer.**
2. **`_test_workspace` has exactly one caller:** root `justfile:75`. The only
   other repository hits are historical baselines under
   `claudine/fixes/_completed/` and the spec itself. Regression surface for
   removing the flag is one recipe.
3. **An agent cannot opt into or out of the flag from the command line.**
   Every argument to `just test` is consumed as a package/area *selector*
   (`requested=({{ selectors }})`); nothing is forwarded to nextest. So
   `just test --no-fail-fast` would be read as a selector and fail with
   "No workspace packages matched". Removing the flag therefore removes the
   behavior with no user-facing way to restore it — a material argument for
   the keep option, and something the skill text must state either way.
4. **CI genuinely implements R1 already.** Verified at
   `.github/workflows/_package-ci.yml:438` (with the cost-of-next-run comment),
   `.github/workflows/_wsl-ci.yml:648`, and `just/ci-local.just:462`. Nothing
   in Phase 2 or 3 should touch these.
5. **`docs/testing-strategy.md:304-309` already documents the root `just test`
   `--no-fail-fast` behavior** ("so one scheduler sees every test binary and
   one failure cannot hide the rest"). The spec does not mention this file.
   Per CLAUDE.md's Drift Maintenance rule this doc is in scope *conditionally*:
   it must be reconciled if and only if the flag is removed.
6. **The skill-hash procedure has an ordering trap.** `md hash` computes
   `{frontmatter}-{body}`; today
   `md hash .claude/skills/rust-testing/SKILL.md` returns exactly the committed
   `61d07be7e22c9f45-5e3087a71564a05a`, i.e. the file is currently a fixed
   point. Because `last_updated` lives in the frontmatter, bumping it *changes*
   the frontmatter half. The hash must be computed **after** every other edit
   including the `last_updated` bump, and then re-run once to confirm it is a
   fixed point.
7. **CLAUDE.md carries no frontmatter**, so it takes no hash. AC6 applies to
   skill files only.
8. **The non-vacuous-proof procedure is not currently documented in any
   skill.** The only repository mention is a passing one in
   `.claude/skills/os/wsl.md:54`. AC2 is therefore *new* prose, not a
   correction of existing prose — there is no wrong sentence to hunt down.
9. **`memory/just.md` and `memory/implement-new-message-platforms.md` are
   git-tracked repository files** (distinct from the disabled per-project agent
   memory). `memory/just.md:52` and `:208` discuss `_test_local_all` and
   `--no-fail-fast` forwarding. Their status needs a ruling (R-5) before
   deciding whether they are drift targets.

### What success looks like

Concrete, observable, and checkable by someone who did not write the change:

- **AC1** — A reader of `.claude/skills/rust-testing/SKILL.md` alone can answer:
  what fail-fast setting applies locally, what applies in CI, *why* (the
  cost of the next run, not a flag preference), and that CI already passes
  `--no-fail-fast` so they must not add it.
- **AC2** — The same skill states that a non-vacuous guard proof needs the
  complete failure list at CI-shaped or multi-package scope, and that fail-fast
  is correct and faster for a single package locally.
- **AC3** — `just/devops.just` line ~1206 reflects a *decided* position, and
  the comment at that line gives reasoning that survives fact (1) above —
  i.e. it does not claim the recipe is always a full-workspace sweep. If the
  flag is removed, `just test` and `just test <selector>` both still work.
- **AC4** — `CLAUDE.md` carries the test-scope rule with "this is a CI/CD
  rule, it does not extend to local testing" stated in the rule itself, not
  inferable only from context, and sits adjacent to "Evidence Reuse and
  Execution Constraints".
- **AC5** — `CLAUDE.md`'s "Features and Fixes" section names the author as the
  only party who moves work into `_completed/` and states the agent's terminal
  state is "implementation complete, ready for review".
- **AC6** — `md hash` on every edited skill file equals its committed `hash:`
  frontmatter, verified by a second run producing no change.
- **AC7** — `git diff --name-only` against the merge base lists no path under
  `.github/workflows/`.
- **Drift** — no surviving sentence anywhere in the repository contradicts the
  newly written rules.

### Explicit non-goals

Carried from the spec's Scope section; listed here so no phase quietly expands.

- No change to how CI invokes tests.
- None of the other 2026-09-13 audit promotion candidates (git safety
  discipline, status-writing conventions, exit-code masking when piping,
  stale binaries after `just lint`, `cargo install --locked`).
- No change to the memory system.
- No new argument-forwarding capability for `just test`, even though fact (3)
  makes it tempting. If the team wants it, it is a separate fix.

### Risk profile

Low. The largest risks are (a) making the R1 ruling on a false premise —
mitigated by fact (1) being surfaced *before* the decision in Phase 1 — and
(b) shipping prose that contradicts `docs/testing-strategy.md`, mitigated by
the Phase 3 drift sweep.

---

## Phase 1 — Rulings, Spikes, and Evidence Baseline

Nothing is edited in this phase. Its output is a set of decisions recorded in
this document and a small evidence baseline that Phase 4 compares against.

### Necessary Rulings

These must be answered before Phase 2 starts. Each carries a recommendation;
the recommendation is the default if the author does not rule otherwise.

- [ ] **R-1 — `_test_workspace`: keep, drop, or condition the flag?**
    - The spec offers two options (keep and say the rule keys on
      cost-of-next-run; or drop and say the rule is strictly environmental).
      Fact (1) exposes a third: the recipe covers both a full sweep *and*
      `just test <one-area>`, so neither framing is clean.
    - **Recommendation: keep the flag, unconditionally, and rewrite the
      comment.** Reasons: fact (3) means removal is irreversible from the
      command line; fact (2) means keeping costs nothing; a conditional
      (`--no-fail-fast` only when no selectors are given) adds branching logic
      to a shared recipe for a marginal gain and would violate Rule 2.
    - The new comment must not repeat the current claim that this is the "old
      workspace loop's promise" — it must say the recipe is the repository's
      broadest local scope, that re-running it is expensive enough to sit on
      the CI side of R1's cost-of-next-run test, and that selector-narrowed
      invocations inherit the flag by design rather than by oversight.
    - **Rejecting this recommendation is a supported outcome** — see Phase 2's
      conditional task and Phase 3's conditional drift task.

- [ ] **R-2 — Does R1's documentation live in `SKILL.md` or in `nextest.md`?**
    - `.claude/skills/rust-testing/nextest.md` is the skill's nextest topic
      page and is the mechanically obvious home for a flag discussion.
    - **Recommendation: `SKILL.md`, in/adjacent to "Canonical Just Recipes"
      (line 297).** The spec names that section explicitly, and R1 is a policy
      about *when to pay for completeness*, not a nextest feature. Keeping it
      in the always-loaded entry point is the point of the fix — a rule in a
      topic page is a rule most agents will not read.
    - Consequence of accepting: only one file gets a hash refresh (AC6 is
      cheap). Consequence of rejecting: two files need hash refreshes and the
      Topic Pages table (line 749) needs a row update.

- [ ] **R-3 — Where exactly does R2 go in `CLAUDE.md`?**
    - Candidates: (a) a new H2 immediately *before* "Evidence Reuse and
      Execution Constraints" (currently line 51); (b) a new H2 immediately
      after it; (c) new bullets inside it.
    - **Recommendation: (a), a new H2 `## CI/CD Test-scope Discipline`.** The
      spec calls R2 "the upstream question: whether the cell should exist at
      all", so it reads in logical order before the reuse rule. A separate
      heading also keeps R2's CI/CD scope statement from being read as a
      qualifier on the evidence-reuse bullets.
    - Rejecting in favor of (c) risks exactly the failure the spec warns
      about: an unscoped reading that discourages local testing.

- [ ] **R-4 — Does `rust-devops` get a pointer to R2?**
    - R2 is a CI/CD rule and `.claude/skills/rust-devops/SKILL.md` is the skill
      CLAUDE.md already tells agents to load "before changing CI scope".
    - **Recommendation: no.** The spec scopes R2's documentation to
      `CLAUDE.md`, which every agent reads unconditionally; a duplicate in a
      loaded-on-demand skill creates two copies to keep in sync. Record the
      decision so a future reviewer does not re-open it.

- [ ] **R-5 — Are the git-tracked `memory/*.md` files in scope for drift?**
    - `memory/just.md` is tracked in the repository and discusses
      `_test_local_all` and `--no-fail-fast` argument forwarding. It is *not*
      the disabled per-project agent memory that the spec rules out of scope.
    - **Recommendation: read-only check in Phase 3.** If a sentence there is
      falsified by the R-1 outcome, fix that sentence and nothing else; if the
      file's status is itself unclear, report it and leave it untouched.

### Work-group 1A — Decision spikes (all three run concurrently)

- [ ] **Spike: flag reversibility**
    - Confirm fact (3) empirically: run `just test --no-fail-fast` at the repo
      root and capture that the argument is consumed as a selector and the
      recipe exits non-zero with "No workspace packages matched".
    - Also confirm `just test <a-small-area>` selects only that area's
      packages (read the "Running Level-1 tests for N workspace packages"
      line; interrupting before the suite finishes is fine and expected).
    - Output: a yes/no on whether removing the flag is reversible by a caller.
      This is the deciding input for R-1. **Blocks R-1.**

- [ ] **Spike: hash fixed-point mechanics**
    - On a scratch copy of `.claude/skills/rust-testing/SKILL.md`, change only
      `last_updated` and re-run `md hash`. Confirm the frontmatter half of the
      hash moves, proving the Phase 4 ordering (edit everything → bump
      `last_updated` → hash → write → re-hash to confirm) is required rather
      than ceremonial.
    - Do not modify the tracked file. Delete the scratch copy.
    - Output: the confirmed hash-refresh procedure, written into Phase 4's task.

- [ ] **Spike: contradiction sweep (read-only)**
    - Search the repository for live statements that either R-1 outcome would
      falsify, and for passive-voice `_completed/` language beyond
      `CLAUDE.md:138`.
    - Suggested starting points: `docs/testing-strategy.md`,
      `docs/topics/ci-cd.md`, `.claude/skills/rust-testing/`,
      `.claude/skills/rust-devops/`, `.claude/skills/os/`, `memory/`,
      root and area `justfile`s, `just/*.just`.
    - Exclude `**/_completed/**` and `**/reviews/**` — historical records are
      not drift.
    - Output: a checklist of file:line drift candidates, carried into Phase 3.
      **Note:** `docs/testing-strategy.md:304-309` and `memory/just.md:52,208`
      are already known and should be on the list before the sweep starts.

### Work-group 1B — Evidence baseline (concurrent with 1A)

- [ ] **Capture pre-change baselines**
    - `just --summary` at the repo root, exit code and output recorded
      (it currently exits 0). Phase 4 compares against this.
    - `md hash .claude/skills/rust-testing/SKILL.md` recorded (currently
      `61d07be7e22c9f45-5e3087a71564a05a`, matching the committed frontmatter).
    - `git merge-base HEAD main` recorded, so AC7 can be checked with
      `git diff --name-only <base>...HEAD`.
    - Confirm the working tree has no unrelated staged changes that would
      pollute the AC7 check.

### Phase 1 Validation Checkpoint

- [ ] Every ruling R-1 … R-5 has a recorded answer (accepted recommendation or
      an explicit override) written into this plan document.
- [ ] R-1 was decided *after* the flag-reversibility spike reported, not before.
- [ ] The drift candidate checklist exists and includes the two known entries.
- [ ] Baselines captured. No file outside `fixes/2026-09-14-cicd-improvements/`
      has been modified.

---

## Phase 2 — Apply the Three Rules

Two independent work-groups touching disjoint files. They can be executed
concurrently by two agents, or serially by one; there is no ordering
dependency between them and no shared file.

### Work-group 2A — R1 (`.claude/skills/rust-testing/SKILL.md`, `just/devops.just`)

- [ ] **Write the fail-fast policy** *(satisfies AC1)*
    - Add the rule to the section chosen in R-2 (default: "Canonical Just
      Recipes", `SKILL.md:297`).
    - Must state, in this order: locally fail fast (nextest's default, and the
      right one); in CI run to completion; **the reason is the cost of the
      next run**, which is what makes it an environment policy rather than a
      flag preference; a truncated Windows or WSL2 report costs a full
      round-trip measured in hours.
    - Must state that CI already passes `--no-fail-fast` and an agent must not
      add it, citing `.github/workflows/_package-ci.yml`,
      `.github/workflows/_wsl-ci.yml`, and `just/ci-local.just` **by name
      only, not by line number** — line numbers in a skill are a drift
      treadmill (CLAUDE.md Code Comment Quality, item E).
    - Must record the R-1 decision for root `just test` / `_test_workspace`,
      including the selector nuance from fact (1).
    - Keep it proportionate: this is a rule, not an essay. The existing
      section is a table plus a sentence; match that altitude.

- [ ] **Write the non-vacuous-proof consequence** *(satisfies AC2)*
    - Place it adjacent to the fail-fast rule, framed explicitly as a
      *consequence* of R1 — not as a standalone "always use this flag" rule.
      The spec is emphatic on this: the earlier "always run the neutered pass
      with `--no-fail-fast`" phrasing is what prompted the fix.
    - Must say: at CI-shaped or multi-package scope the complete failure list
      is required, **because a truncated list looks exactly like a narrow
      blast radius** — the opposite of what the proof is for; against a single
      package locally, fail-fast is correct and faster.
    - Per fact (8) there is no existing wrong sentence to delete; this is
      additive.

- [ ] **Execute the R-1 decision at `just/devops.just:1206`** *(satisfies AC3)*
    - *If keep (default):* leave the flag, replace the two-line comment above
      it. The replacement must not claim the recipe is always a full-workspace
      sweep, and must explain the selector case. Comment-only change — confirm
      `git diff` for this file shows no non-comment line, per CLAUDE.md's
      Scope discipline rule.
    - *If drop:* remove `--no-fail-fast` from the `just _test_local_all` call,
      replace the comment with the environmental-rule reasoning, then verify
      root `just test` and `just test <one-area>` both still run (fact (2):
      one caller, so this is a bounded check), and hand the Phase 3 drift task
      the now-mandatory `docs/testing-strategy.md:304-309` reconciliation.
    - Either way the comment earns its length under CLAUDE.md criterion B —
      *why* a counter-intuitive choice was made — at the surprising line.

### Work-group 2B — R2 and R3 (`CLAUDE.md`)

- [ ] **Add the CI/CD test-scope rule** *(satisfies AC4)*
    - Insert at the position chosen in R-3 (default: new H2
      `## CI/CD Test-scope Discipline` immediately before "Evidence Reuse and
      Execution Constraints", currently line 51).
    - Content: before adding a CI run, matrix cell, fixture, or gate, ask what
      question it answers and whether something cheaper answers it; never
      trigger a full-scope run to learn UI behavior or ruleset semantics that
      could be read or reasoned out; CI turnaround here runs to hours, so a
      speculative cell is not a small cost.
    - **The scope sentence is not optional and not a footnote:** this is a
      CI/CD rule and does not extend to local testing, where runs are cheap
      and catch things early. Over-testing locally has not been a problem here.
    - One sentence tying it to the neighbouring section: R2 is the upstream
      question (should the cell exist), evidence reuse is the downstream one
      (must it re-run).
    - Match the file's existing bullet style and line width.

- [ ] **Name the owner of the `_completed/` move** *(satisfies AC5)*
    - Replace `CLAUDE.md:138` — "when a feature/fix is completed it is moved to
      `_completed`" — with active-voice text naming the **author** as the only
      party who makes that move, after the review cycle closes.
    - Add the agent-side half: an agent's terminal state is "implementation
      complete, ready for review"; an agent never moves a directory into
      `_completed/`.
    - One or two lines. It is a prohibition, not a procedure — resist
      expanding it into a lifecycle description.
    - Do **not** edit `AGENTS.md`: it is a symlink to `CLAUDE.md` (verified).

### Phase 2 Validation Checkpoint

- [ ] `git diff --stat` lists only the files the executed rulings authorize —
      at most `SKILL.md`, `CLAUDE.md`, `just/devops.just`.
- [ ] Nothing under `.github/workflows/` appears in the diff (AC7 interim).
- [ ] If R-1 kept the flag: `git diff just/devops.just` shows comment lines
      only.
- [ ] `just --summary` at the repo root still exits 0 and its recipe list
      matches the Phase 1 baseline.

---

## Phase 3 — Drift Reconciliation

Depends on Phase 2 being written and on the R-1 outcome. Sequential after
Phase 2 because the drift check compares against the *final* wording.

- [ ] **Reconcile `docs/testing-strategy.md`** *(conditional on R-1)*
    - Lines 304-309 describe root `just test` handing every package to one
      nextest invocation "with `--no-fail-fast`, so one scheduler sees every
      test binary and one failure cannot hide the rest."
    - *If R-1 kept the flag:* the sentence stays true. Check only whether it
      now contradicts the skill's selector nuance; leave it alone if not.
      **Do not gratuitously reword a correct sentence** (Rule 3).
    - *If R-1 dropped the flag:* this sentence is now false and **must** be
      corrected in the same change. Also re-read lines 745-770 — the CI-side
      `--no-fail-fast` statements there remain true either way and must not be
      touched.

- [ ] **Work the Phase 1 drift checklist**
    - Walk every file:line candidate the Phase 1 sweep produced.
    - Apply CLAUDE.md's drift rule: **assume the code is correct and the
      comment/doc is wrong**, fix accordingly, and report in the closing
      summary that drift was detected and how it was resolved.
    - Include the R-5 outcome for `memory/just.md` here.
    - Leave `**/_completed/**`, `**/reviews/**`, and prior `plan.md`/`log.md`
      artifacts alone — they are dated records, not live guidance.

- [ ] **Confirm no second copy of any rule was created**
    - Grep the final text of each of the three rules for near-duplicates
      elsewhere in `.claude/skills/` and `docs/`. A rule stated twice is a
      rule that will drift. Per R-4 the deliberate decision is one home per
      rule.

### Phase 3 Validation Checkpoint

- [ ] Every drift candidate is either fixed or explicitly recorded as
      not-drift with a one-line reason.
- [ ] No rule text exists in two places.
- [ ] Still nothing under `.github/workflows/` in the diff.

---

## Phase 4 — Hash Refresh, Verification, and Handoff

Must run last: the hash is a function of the final bytes, so any Phase 3 edit
to a skill file invalidates a hash computed earlier.

- [ ] **Refresh skill hashes** *(satisfies AC6)*
    - For each edited skill file, in this exact order (per the Phase 1 spike):
      1. confirm all body edits are final;
      2. bump `last_updated` to `2026-09-14`;
      3. run `md hash <file>`;
      4. write the result into the `hash:` frontmatter field;
      5. re-run `md hash <file>` and confirm the value is unchanged — a fixed
         point, as the file was before this change.
    - Default scope is one file, `.claude/skills/rust-testing/SKILL.md`. If
      R-2 was overridden, `nextest.md` is added and its Topic Pages row
      (`SKILL.md:749`) updated *before* step 1 above.
    - `CLAUDE.md` takes no hash (fact (7)). Do not add frontmatter to it.

- [ ] **Run the spec's verification list**
    - `just --summary` in every package area whose justfile changed. Only
      `just/devops.just` is in scope, which is imported by the root justfile
      and every area justfile — so run it at the root **and** in at least two
      areas (e.g. `darkmatter`, `sniff`) to prove the shared import still
      parses everywhere.
    - *If `_test_workspace` changed:* one local run showing the intended
      behavior, plus confirmation that its single caller (root `just test`)
      and a selector-narrowed invocation both still work.
    - `md hash` output matches every committed hash.
    - No CI evidence is required and none should be gathered — nothing here
      changes what CI runs.

- [ ] **Acceptance-criteria audit**
    - Walk AC1 … AC7 from the spec one at a time against the final diff, and
      record a file:line pointer for each. AC7 is
      `git diff --name-only <merge-base>...HEAD | grep '^\.github/workflows/'`
      returning nothing.
    - Re-read the two rules the spec warns are easy to get subtly wrong:
      AC2's framing (a consequence of R1, not a standalone always-use-this
      rule) and AC4's scope sentence (CI/CD only, not a blanket discouragement
      of local testing). If either reads wrong to a fresh reader, fix it here.

- [ ] **Closing summary**
    - Record: the R-1 … R-5 decisions and their reasoning; any drift found and
      how it was resolved; the final hash values; and the verification output.
    - **Terminal state is "implementation complete, ready for review."**
      Do not move this fix directory into `fixes/_completed/` — that is R3,
      and this plan is the first thing that must obey it.
    - Do not commit unless separately asked (CLAUDE.md).

### Phase 4 Validation Checkpoint — Done Means

- [ ] AC1 through AC7 each have a recorded file:line pointer.
- [ ] `md hash` is a fixed point on every edited skill file.
- [ ] `just --summary` exits 0 at the root and in two areas.
- [ ] The diff touches no workflow file and no file outside the four named in
      the Summary table (plus this plan).
- [ ] The fix directory is still at
      `fixes/2026-09-14-cicd-improvements/`, unmoved.

---

## Dependency Graph

```
Phase 1  ─┬─ WG 1A  spike: flag reversibility ──┐
          │  spike: hash mechanics              ├─→ rulings R-1…R-5
          │  spike: contradiction sweep ────────┘        │
          └─ WG 1B  baselines (independent)              │
                                                         ▼
Phase 2  ─┬─ WG 2A  R1: SKILL.md prose + devops.just flag   ┐
          └─ WG 2B  R2 + R3: CLAUDE.md                      │ concurrent,
                                                            │ disjoint files
                                                            ▼
Phase 3     drift reconciliation (needs final Phase 2 wording, needs R-1)
                                                            ▼
Phase 4     hash refresh → verification → AC audit → handoff
```

**Concurrency summary**

| Work-group | Phase | Concurrent with | Why it is safe |
|---|---|---|---|
| 1A | 1 | 1B | Read-only; different outputs |
| 1B | 1 | 1A | Read-only baselines |
| 2A | 2 | 2B | `SKILL.md` + `just/devops.just` vs `CLAUDE.md` — disjoint |
| 2B | 2 | 2A | Same |
| Phase 3 | 3 | — | Must see final Phase 2 wording |
| Phase 4 | 4 | — | Hash is a function of the final bytes |

## Complexity and Risk Notes

- **Phase 1's flag-reversibility spike blocks R-1.** It is the only sequencing
  constraint inside Phase 1 and the only thing standing between this plan and
  a decision made on the spec's incomplete premise. Do not skip it because the
  code read already suggests the answer.
- **The `just/devops.just` change is comment-only under the default ruling.**
  CLAUDE.md requires comment-only commits to contain no behavior changes; if
  the diff shows a non-comment line, split it.
- **Prose altitude is the most likely quality failure.** All three rules are
  short. The temptation is to write a section; the spec asks for a rule. For
  R3 in particular, "one line is enough; it is a prohibition, not a procedure."
- **Skill line-number citations are drift bait.** Cite files by name in the
  skill text, not `file:line` — this plan cites lines because it is a dated
  artifact, the skill is not.
