---
created: 2026-09-14
total_phases: 4
phase: 4
completed_phase: "4"
implemented: true
agent: claude/opus
yolo: true
spec: fixes/2026-09-14-cicd-improvements/spec.md
status: implemented
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - fixes/2026-09-14-cicd-improvements/plan.md
  - fixes/2026-09-14-cicd-improvements/implementation-log.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - just/devops.just
docs_updated_during_phase_2:
  - CLAUDE.md
  - fixes/2026-09-14-cicd-improvements/plan.md
  - fixes/2026-09-14-cicd-improvements/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/rust-testing/SKILL.md
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - docs/testing-strategy.md
  - fixes/2026-09-14-cicd-improvements/plan.md
  - fixes/2026-09-14-cicd-improvements/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - fixes/2026-09-14-cicd-improvements/plan.md
  - fixes/2026-09-14-cicd-improvements/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/rust-testing/SKILL.md
source_code:
  - just/devops.just
documentation:
  - CLAUDE.md
  - docs/testing-strategy.md
  - .claude/skills/rust-testing/SKILL.md
  - fixes/2026-09-14-cicd-improvements/plan.md
  - fixes/2026-09-14-cicd-improvements/implementation-log.md
packages: []
human_review: false
message_to_agent: |-
  Phase 4 is complete and this fix is finished — there is no Phase 5. Terminal
  state is IMPLEMENTATION COMPLETE, READY FOR REVIEW.

  If you are picking this up, the three things most likely to matter:

  1. DO NOT MOVE THIS DIRECTORY INTO `fixes/_completed/` and do not run
     `just complete`. That move is the author's, after the review cycle closes.
     R3 — the rule this fix writes into `CLAUDE.md:152-155` — makes that
     explicit, and this plan is the first artifact that has to obey it.

  2. NOTHING WAS COMMITTED OR STAGED. The change set is six files: the four in
     the plan's Summary table (`.claude/skills/rust-testing/SKILL.md`,
     `CLAUDE.md`, `just/devops.just`, `docs/testing-strategy.md`) plus this
     plan and the implementation log. The working tree also carries unrelated
     modifications under `darkmatter/`, `prompts/`, and untracked
     `claudine/fixes/` that predate this fix — scope any diff command to the
     six paths above, or a bare `git diff` will look alarming.

  3. THE SKILL HASH IS FINAL AND VERIFIED:
     `61d07be7e22c9f45-6f2ea54170be340f`. If you edit the SKILL.md body for any
     reason, re-run `md hash <file>` and rewrite the `hash:` field — and prefer
     `md hash --diff <file>` to check it, which exits 2 on drift. Re-running
     `md hash` twice and comparing proves only determinism; it would have
     passed on the stale committed value this fix repaired.

  One open recommendation is recorded at the end of the Phase 4 log and was
  deliberately NOT actioned: no guard exists anywhere that a skill file's
  committed `hash:` is a fixed point, which is why the drift went unnoticed
  since `d14beb34a`. `md hash --diff` already gives the exit-code contract and
  `tools/test-toolkit/` already hosts passive corpus tests, so it is cheap —
  but adding it would expand scope past both the plan and the spec. It is
  raised as an `_unscheduled` candidate for the author, and it does not block
  review of this fix.

  --- Phase 3's handoff, retained for context ---

  Phase 3 is complete. The drift sweep is closed and the change set is now the
  full four files named in the plan's Summary table. Phase 4 is hash refresh,
  verification, and the AC audit.

  1. YOUR HASH TARGET IS STILL EXACTLY ONE FILE:
     `.claude/skills/rust-testing/SKILL.md`. Phase 3 edited
     `docs/testing-strategy.md`, but that file carries frontmatter WITHOUT a
     `hash:` field, so it is not an AC6 target and AC6's scope has not grown.
     I bumped its `updated:` to 2026-09-15; do not add a hash to it.

  2. NO SKILL FILE WAS TOUCHED IN PHASE 3. The last body edit to SKILL.md was
     Phase 2's, so the hash you compute is final as soon as you compute it —
     nothing in Phase 3 invalidates it. Phase 2's prediction still holds: the
     frontmatter half should read `61d07be7e22c9f45` and the body half will
     differ from BOTH the committed value and the Phase 1 measurement
     (`61d07be7e22c9f45-fb3622c484a86f23`), because the body grew 29 lines.
     Compute fresh; never assert "unchanged from committed".

  3. PHASE 1'S ROW-2 VERDICT WAS OVERRIDDEN — do not re-litigate it in the AC
     audit. `docs/testing-strategy.md:301-307` was edited: one clause noting
     selectors narrow the set, plus a by-name pointer to the `rust-testing`
     skill. Reason (full version in the Phase 3 log): the code block five lines
     ABOVE the prose already documents `just test biscuit-file # one package`,
     so the file contradicted itself inside one section. The `--no-fail-fast`
     half of that sentence was NOT touched and remains true under R-1 = keep,
     so the plan's conditional reconciliation never triggered.

  4. FOR THE AC AUDIT, pointers you can use directly:
     AC1/AC2 → `.claude/skills/rust-testing/SKILL.md:333-360`
     AC3     → `just/devops.just:1204-1212`
     AC4     → `CLAUDE.md:51-64`
     AC5     → `CLAUDE.md:152-155`
     AC6     → yours to produce
     AC7     → R-6 commands; both still empty as of Phase 3.
     Line numbers in CLAUDE.md and SKILL.md are unchanged since Phase 2 —
     Phase 3 edited neither file.

  5. THE PHASE 1 SWEEP WAS SHORT BY 5 ROWS, all not-drift (found by relaxing
     its path filters: `scripts/ci/test_ci_local.py`,
     `scripts/ci/test_affected_scope.py`, `tools/test-toolkit/tests/
     nextest_config_verification.rs`, `scripts/cross-check.sh:151`, and
     `claudine/fixes/_unscheduled/test-suite-residuals/spec.md`). No verdict
     was wrong; the list was merely incomplete. All 22 rows are now resolved in
     the Phase 3 log. You do not need to re-sweep.

  6. Phase 4's `just --summary` check: the ROOT summary is already verified
     byte-identical to the Phase 1 baseline (84 recipes, exit 0) as of Phase 3.
     You still owe the two AREA runs (e.g. `darkmatter`, `sniff`) to prove the
     shared `just/devops.just` import parses everywhere.

  --- Phase 2's handoff, retained for context ---

  Phase 2 is complete. All three rules are written; the R-1 keep decision is
  executed as a comment-only change. Phase 3 is the drift sweep.

  1. THE PHASE 3 CHECKLIST IS ALREADY DONE, in advance. Phase 1's sweep
     recorded all 17 candidates with verdicts in the implementation log under
     "Spike: contradiction sweep". Under the R-1 = keep ruling, exactly two
     were real drift and BOTH are now fixed (`just/devops.just:1204` comment,
     `CLAUDE.md` `_completed` sentence). The other 15 are recorded as not-drift
     with a one-line reason each. Phase 3's job is to re-verify those verdicts
     against the FINAL Phase 2 wording, not to re-derive the checklist.

  2. `docs/testing-strategy.md:301-306` NEEDS A JUDGEMENT CALL FROM YOU — I did
     not make it, because it is Phase 3's task and not mine. The fail-fast half
     of that sentence is still true (the flag was kept), so the conditional
     reconciliation Phase 3 lists is NOT triggered. But Phase 1 recorded this
     row as not-drift on the reasoning "it never claims the set is always every
     package", and re-reading the literal text I do not think that reasoning
     holds: it says the recipe "discovers every workspace package from
     `cargo metadata` ... and hands all of them to a single nextest
     invocation", with no mention of selectors at all. That is accurate for the
     bare `just test` and silently incomplete for `just test <selector>` — the
     exact nuance the skill now spells out at :333.

     My read: this is an omission, not a false statement, and Rule 3 argues
     against rewording a sentence that is correct for the invocation it
     describes. If you disagree, the minimal fix is a clause noting optional
     selectors narrow the set — do not restructure the paragraph. Either way,
     record the verdict and the reason; do not silently inherit Phase 1's.

  3. For Phase 3's "no second copy of any rule" check, the near-duplicate you
     will find is `.claude/skills/rust-testing/SKILL.md:434-440` (was 405-408).
     That is MECHANISM (`_test_workspace` runs one nextest invocation with the
     flag) and predates this fix; what I added at :333 is POLICY (why, and the
     decision). R-2 explicitly rules that keeping the two distinct is correct.
     Do not collapse them.

  4. Phase 4's hash target moved: the skill body grew by 29 lines, so the body
     half of `md hash` WILL differ from both the committed value and the Phase
     1 measurement (`61d07be7e22c9f45-fb3622c484a86f23`). That measurement is
     now only useful as proof the pre-existing drift was real. Compute fresh.
     The frontmatter half should still read `61d07be7e22c9f45` — I changed no
     frontmatter field, and `md hash` excludes `hash`/`last_updated`.

  Carried forward from Phase 1 and still true: AC7 must use the R-6 commands
  (the merge-base form cannot pass on this branch); both are still empty.
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

- [x] **R-1 — `_test_workspace`: keep, drop, or condition the flag?**
    - **DECIDED 2026-09-15 — recommendation ACCEPTED: keep the flag,
      unconditionally, and rewrite the comment.** Decided after the
      flag-reversibility spike reported, not before.
    - The spike confirmed fact (3) empirically at the repository root
      (`just test --no-fail-fast` → `No workspace packages matched`, exit 1)
      and confirmed fact (1) (`just test biscuit-hash` selects 2 packages).
    - **It also narrowed the reasoning.** Reversibility is scope-dependent,
      which the plan did not know: root `just test` sends `*args` to
      `_test_workspace *selectors` and they are consumed as selectors, but an
      *area* `just test` (e.g. `darkmatter/justfile:71`) forwards `{{ args }}`
      to `_test_local_all` and onward to nextest, so `just test --no-fail-fast`
      works normally from inside a package area.
    - This strengthens *keep* rather than weakening it: the single recipe whose
      callers cannot re-specify the flag is precisely the one that has it baked
      in, while every area recipe — the one agents actually use daily — keeps
      full control and retains nextest's fail-fast default.
    - **Phase 2 constraint:** the new comment must scope the irreversibility
      claim to the repository root. An unscoped restatement of fact (3) would
      be false about area recipes.
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

- [x] **R-2 — Does R1's documentation live in `SKILL.md` or in `nextest.md`?**
    - **DECIDED — recommendation ACCEPTED: `SKILL.md`, "Canonical Just
      Recipes".** Only one file gets a hash refresh; the Topic Pages table
      needs no row update.
    - **Line-number correction:** "Canonical Just Recipes" is at `SKILL.md:312`,
      not 297. Line 297 sits inside the `expect_level!` gating discussion.
    - **Do not restate mechanism.** `SKILL.md:405-408` already describes
      `_test_workspace`'s one-invocation `--no-fail-fast` behavior correctly.
      That is *mechanism*; what Phase 2 adds at line 312 is *policy*. Keeping
      the two distinct is what lets Phase 3's no-duplicate check pass.
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

- [x] **R-3 — Where exactly does R2 go in `CLAUDE.md`?**
    - **DECIDED — recommendation ACCEPTED: option (a),** a new H2
      `## CI/CD Test-scope Discipline` immediately before
      `## Evidence Reuse and Execution Constraints` (verified still at
      `CLAUDE.md:51`).
    - **Addition for the R3/AC5 half of Work-group 2B:** the sweep found that
      `just complete` (`just/lifecycle.just:108`) is a live recipe that
      performs the `_completed/` move, and it is documented to agents in
      `.claude/skills/just/SKILL.md:78` and `.claude/agents/just-scripter.md:23`.
      A prohibition that does not name the command leaves the button
      documented and unguarded. The CLAUDE.md text should name `just complete`
      explicitly. This stays within "one or two lines — a prohibition, not a
      procedure".
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

- [x] **R-4 — Does `rust-devops` get a pointer to R2?**
    - **DECIDED — recommendation ACCEPTED: no.** One home per rule.
    - Checked while sweeping: `.claude/skills/rust-devops/` mentions fail-fast
      exactly once (`ci-cd.md:271`), and that is the GitHub Actions matrix
      `fail-fast: false` — an unrelated concept from nextest's flag. So there
      is no existing near-duplicate to reconcile, and adding one would create
      the first.
    - R2 is a CI/CD rule and `.claude/skills/rust-devops/SKILL.md` is the skill
      CLAUDE.md already tells agents to load "before changing CI scope".
    - **Recommendation: no.** The spec scopes R2's documentation to
      `CLAUDE.md`, which every agent reads unconditionally; a duplicate in a
      loaded-on-demand skill creates two copies to keep in sync. Record the
      decision so a future reviewer does not re-open it.

- [x] **R-5 — Are the git-tracked `memory/*.md` files in scope for drift?**
    - `memory/just.md` is tracked in the repository and discusses
      `_test_local_all` and `--no-fail-fast` argument forwarding. It is *not*
      the disabled per-project agent memory that the spec rules out of scope.
    - **DECIDED — recommendation ACCEPTED, and the read-only check is already
      done (Phase 1 sweep). Outcome: not drift; Phase 3 has nothing to fix.**
    - `memory/just.md:52` describes `_test_all` selecting `_test_local_all`
      locally versus `_run_all _test` under CI. It makes no fail-fast claim and
      is unaffected by R-1 either way.
    - `memory/just.md:208` states that `just _recipe --no-fail-fast
      --archive-file /x` passes all three through to a `*args` parameter. This
      is **true** — it describes `just`'s own option parsing, not what a recipe
      subsequently does with the arguments. It sits adjacent to the root-`just
      test` trap without asserting anything false, so under Rule 3 it is left
      untouched.
    - The files' status is not unclear: they are tracked, live, and their
      content is accurate. No report-and-leave case arose.

- [x] **R-6 — How is AC7 actually verified on this branch?** *(new; raised by
      the Phase 1 baseline, not present in the original plan)*
    - AC7's command `git diff --name-only <merge-base>...HEAD | grep
      '^\.github/workflows/'` **cannot return empty on this branch.** Three
      commits that predate this fix — `0a750f407`, `1fd610a7b`, `2ed6b0f04` —
      already touch `.github/workflows/`, so the merge-base diff lists five
      workflow files before this fix changes anything.
    - AC7's *intent* is unambiguous and unchanged: this fix touches no workflow
      file. Only the measurement was wrong, because it measures the whole
      branch rather than this fix's contribution.
    - **DECIDED — verify against this fix's own changes:**

      ```sh
      git status --porcelain .github/workflows/          # must be empty
      git diff --name-only HEAD -- .github/workflows/    # must be empty
      ```

      Both are empty as of the Phase 1 baseline, which is the condition Phases
      2-4 must preserve. If this fix is committed before the AC audit runs,
      substitute the range from the first commit of this fix to `HEAD`.
    - No scope change: nothing about which files this fix may touch has moved.

### Work-group 1A — Decision spikes (all three run concurrently)

- [x] **Spike: flag reversibility**
    - Confirm fact (3) empirically: run `just test --no-fail-fast` at the repo
      root and capture that the argument is consumed as a selector and the
      recipe exits non-zero with "No workspace packages matched".
    - Also confirm `just test <a-small-area>` selects only that area's
      packages (read the "Running Level-1 tests for N workspace packages"
      line; interrupting before the suite finishes is fine and expected).
    - Output: a yes/no on whether removing the flag is reversible by a caller.
      This is the deciding input for R-1. **Blocks R-1.**
    - **RESULT — No, not at the repository root.** `just test --no-fail-fast`
      printed `No workspace packages matched: --no-fail-fast` and exited 1;
      `just test biscuit-hash` selected 2 packages and exited 0. The spike
      additionally established that reversibility *is* available at area scope
      (`darkmatter/justfile:71` forwards `{{ args }}` to nextest), which R-1
      folds into its reasoning. Full transcript in the implementation log.

- [x] **Spike: hash fixed-point mechanics**
    - On a scratch copy of `.claude/skills/rust-testing/SKILL.md`, change only
      `last_updated` and re-run `md hash`. Confirm the frontmatter half of the
      hash moves, proving the Phase 4 ordering (edit everything → bump
      `last_updated` → hash → write → re-hash to confirm) is required rather
      than ceremonial.
    - Do not modify the tracked file. Delete the scratch copy.
    - Output: the confirmed hash-refresh procedure, written into Phase 4's task.
    - **RESULT — the premise is falsified twice; see Phase 4's amended task.**
      (a) The tracked file is **not** currently a fixed point: `md hash`
      returns `61d07be7e22c9f45-fb3622c484a86f23` while the committed field
      reads `...-5e3087a71564a05a`. The working tree is clean and the HEAD blob
      hashes identically, so this is committed drift dating to `d14beb34a`.
      (b) Bumping `last_updated` does **not** move the hash, and neither does
      rewriting the `hash:` field — `md hash` excludes both. Changing `name`
      moves the frontmatter half; appending a body line moves the body half.
      The ordering is therefore ceremonial; the only real constraint is
      "hash after the final body edit". Scratch copy deleted; tracked file
      untouched.

- [x] **Spike: contradiction sweep (read-only)**
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
    - **RESULT — 17-entry checklist recorded in the implementation log's
      Phase 1 section**, including both known entries. Two entries are actual
      drift: `just/devops.just:1204-1205` (the R-1 comment, fixed in Phase 2)
      and `CLAUDE.md:138` (the AC5 target). Two more are *conditional* on R-1
      and stay correct under the keep ruling: `docs/testing-strategy.md:302-306`
      and `.claude/skills/rust-testing/SKILL.md:405-408`. The remaining 13 are
      recorded as not-drift with a one-line reason each, which satisfies
      Phase 3's first checkpoint item in advance.
    - Two findings worth carrying beyond the checklist: `docs/topics/ci-cd.md`
      and `rust-devops/ci-cd.md` use `fail-fast` only in the GitHub Actions
      matrix sense (an unrelated concept — do not "reconcile" them), and no
      passive-voice `_completed/` guidance exists in any live file other than
      `CLAUDE.md:138`, so AC5 has exactly one edit site.

### Work-group 1B — Evidence baseline (concurrent with 1A)

- [x] **Capture pre-change baselines**
    - `just --summary` at the repo root, exit code and output recorded
      (it currently exits 0). Phase 4 compares against this.
    - `md hash .claude/skills/rust-testing/SKILL.md` recorded (currently
      `61d07be7e22c9f45-5e3087a71564a05a`, matching the committed frontmatter).
    - `git merge-base HEAD main` recorded, so AC7 can be checked with
      `git diff --name-only <base>...HEAD`.
    - Confirm the working tree has no unrelated staged changes that would
      pollute the AC7 check.
    - **RESULT.** `just --summary` at the root: exit **0**, 84 recipes.
      `md hash .claude/skills/rust-testing/SKILL.md`:
      `61d07be7e22c9f45-fb3622c484a86f23` — **not** the value this plan
      predicted, see the hash spike. `git merge-base HEAD main`:
      `aad933bdb725259062b446c0baba8f708c4ccbde` (30 commits behind HEAD).
      **Staged changes: none.**
    - Two caveats the baseline exposed, both carried into later phases:
      **(i)** the AC7 check is invalid as written — see R-6; **(ii)** the
      working tree carries unrelated *unstaged* modifications
      (`darkmatter/`, `prompts/`, untracked `claudine/fixes/`), so Phase 2's
      "`git diff --stat` lists only authorized files" checkpoint must be path
      scoped: `git diff --stat -- CLAUDE.md just/devops.just
      .claude/skills/rust-testing/`.

### Phase 1 Validation Checkpoint

- [x] Every ruling R-1 … R-5 has a recorded answer (accepted recommendation or
      an explicit override) written into this plan document. *(All five
      accepted their recommendation; R-1, R-2, R-3, and R-5 carry recorded
      refinements. A sixth ruling, R-6, was added for the invalid AC7 check.)*
- [x] R-1 was decided *after* the flag-reversibility spike reported, not before.
      *(And the spike changed its reasoning — scope-dependent reversibility —
      rather than rubber-stamping the code read.)*
- [x] The drift candidate checklist exists and includes the two known entries.
      *(17 entries; known entries are rows 2, 6, and 7.)*
- [x] Baselines captured. No file outside `fixes/2026-09-14-cicd-improvements/`
      has been modified. *(Verified with `git status --porcelain`; the only
      changes are this plan and the implementation log.)*

---

## Phase 2 — Apply the Three Rules

Two independent work-groups touching disjoint files. They can be executed
concurrently by two agents, or serially by one; there is no ordering
dependency between them and no shared file.

### Work-group 2A — R1 (`.claude/skills/rust-testing/SKILL.md`, `just/devops.just`)

- [x] **Write the fail-fast policy** *(satisfies AC1)* — `SKILL.md:333`, new H3
      "Fail-fast is an environment policy, not a flag preference" inside
      "Canonical Just Recipes". Cites the three CI files by name only. The
      irreversibility claim is scoped to the repository root per R-1.
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

- [x] **Write the non-vacuous-proof consequence** *(satisfies AC2)* —
      `SKILL.md:356`, the final paragraph of the same H3, opening with
      "**Consequence for non-vacuous proofs.**" so it cannot read as a
      standalone always-use-this-flag rule.
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

- [x] **Execute the R-1 decision at `just/devops.just:1206`** *(satisfies AC3)*
      — KEEP branch taken. Flag untouched; the two-line comment became eight
      lines at `just/devops.just:1204-1211`. `git diff` for this file shows
      comment lines only (verified). Smoke: `just test biscuit-hash` → 70/70
      passed.
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

- [x] **Add the CI/CD test-scope rule** *(satisfies AC4)* — `CLAUDE.md:51`, new
      H2 `## CI/CD Test-scope Discipline` immediately before "Evidence Reuse and
      Execution Constraints" (option (a)). The scope sentence is the **first**
      bullet, not a footnote.
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

- [x] **Name the owner of the `_completed/` move** *(satisfies AC5)* —
      `CLAUDE.md:152-155`. Two bullets, active voice, names the author as the only
      mover and names `just complete` explicitly per R-3. `AGENTS.md` confirmed
      a symlink to `CLAUDE.md`; not edited.
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

- [x] `git diff --stat` lists only the files the executed rulings authorize —
      at most `SKILL.md`, `CLAUDE.md`, `just/devops.just`. **Scope the command
      to those paths;** the working tree carries unrelated unstaged changes
      (Phase 1 baseline), so a bare `git diff --stat` will not be clean:
      `git diff --stat -- CLAUDE.md just/devops.just .claude/skills/rust-testing/`
      *(Exactly those three files, +55/-3.)*
- [x] Nothing under `.github/workflows/` appears in the diff (AC7 interim),
      checked with `git status --porcelain .github/workflows/` per R-6.
      *(Both R-6 commands empty.)*
- [x] If R-1 kept the flag: `git diff just/devops.just` shows comment lines
      only. *(Verified — the only non-comment line in the hunk is the unchanged
      context line `just _test_local_all "${specs}" --no-fail-fast`.)*
- [x] `just --summary` at the repo root still exits 0 and its recipe list
      matches the Phase 1 baseline. *(Exit 0, 84 recipes, byte-identical to the
      Phase 1 capture.)*

---

## Phase 3 — Drift Reconciliation

Depends on Phase 2 being written and on the R-1 outcome. Sequential after
Phase 2 because the drift check compares against the *final* wording.

- [x] **Reconcile `docs/testing-strategy.md`** *(conditional on R-1)* —
      **VERDICT: minimal clause added, overriding Phase 1's not-drift row 2.**
      `docs/testing-strategy.md:301-307`. The fail-fast half is untouched and
      remains true (keep ruling), so the conditional reconciliation below was
      not triggered; this is the separate omission question Phase 2 handed
      over. Reasoning recorded in the implementation log — the paragraph's
      "hands **all of them**" is contradicted by the code block five lines
      above it in the same section (`just test biscuit-file # one package`),
      and by the repository's own skill text, which already says "every
      **selected** package". That is an internal contradiction inside one
      section, not a stylistic preference, so Rule 3 does not protect it.
      Fix is one clause plus a by-name pointer; the paragraph is not
      restructured. `updated:` frontmatter bumped to 2026-09-15; this file
      carries no `hash:` field, so it is not an AC6 target.
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

- [x] **Work the Phase 1 drift checklist** — all 17 rows re-verified against
      the *final* Phase 2 wording, plus 5 rows the Phase 1 sweep did not
      surface (found by re-running the sweep without its path filters). Two
      rows were drift and were fixed in Phase 2; row 2 is overridden above;
      the remaining 19 are recorded as not-drift with a one-line reason each in
      the implementation log's Phase 3 section. R-5 (`memory/just.md:52,208`)
      re-confirmed: no fail-fast policy claim, nothing to fix.
    - Walk every file:line candidate the Phase 1 sweep produced.
    - Apply CLAUDE.md's drift rule: **assume the code is correct and the
      comment/doc is wrong**, fix accordingly, and report in the closing
      summary that drift was detected and how it was resolved.
    - Include the R-5 outcome for `memory/just.md` here.
    - Leave `**/_completed/**`, `**/reviews/**`, and prior `plan.md`/`log.md`
      artifacts alone — they are dated records, not live guidance.

- [x] **Confirm no second copy of any rule was created** — R2 has **zero**
      near-duplicates in `.claude/skills/` or `docs/` (searched for its
      distinctive phrases; no hits), confirming R-4 empirically rather than by
      assertion. R3's prohibition exists only at `CLAUDE.md:152-155`. R1's
      policy at `SKILL.md:333-360` and its mechanism at `SKILL.md:434-440`
      overlap in *subject* but not in *content*; R-2 ruled the split
      deliberate and Phase 2 handed it over explicitly, so they were not
      collapsed. The one new pointer added to `docs/testing-strategy.md` cites
      the skill by name and restates no reasoning.
    - Grep the final text of each of the three rules for near-duplicates
      elsewhere in `.claude/skills/` and `docs/`. A rule stated twice is a
      rule that will drift. Per R-4 the deliberate decision is one home per
      rule.

### Phase 3 Validation Checkpoint

- [x] Every drift candidate is either fixed or explicitly recorded as
      not-drift with a one-line reason. *(22 rows — Phase 1's 17 re-verified
      against the final wording, plus 5 the Phase 1 sweep's path filters hid,
      all not-drift. Three fixed in total: two in Phase 2, plus row 2 here.)*
- [x] No rule text exists in two places. *(R1/R2/R3 each searched by their
      distinctive phrases across `.claude/skills/` and `docs/`: R2 and R1 have
      zero hits outside their home; R3's two hits are unrelated report-section
      headings. The R1 policy/mechanism pair inside `SKILL.md` is the ruled-on
      deliberate split (R-2), not a duplicate.)*
- [x] Still nothing under `.github/workflows/` in the diff. *(Both R-6
      commands empty.)*
- [x] The targeted gate for the edited artifact was run, not just cited:
      `just test test-toolkit` → **183 passed, 2 pre-existing skips**, which
      includes the corpus test reading `docs/testing-strategy.md`
      (`active_ci_authority_matches_the_retirement_contract`).
      `just _lint test-toolkit` clean; root `just --summary` exit 0, 84
      recipes, byte-identical to the Phase 1 baseline.

---

## Phase 4 — Hash Refresh, Verification, and Handoff

Must run last: the hash is a function of the final bytes, so any Phase 3 edit
to a skill file invalidates a hash computed earlier.

- [x] **Refresh skill hashes** *(satisfies AC6)* — `.claude/skills/rust-testing/SKILL.md:8-9`.
      Computed after the final body edit (Phase 2's; Phase 3 touched no skill
      file): `61d07be7e22c9f45-6f2ea54170be340f`. Written into `hash:`,
      `last_updated:` bumped `2026-09-12` → `2026-09-15`, and a second
      `md hash` run returned the identical value — fixed point confirmed. The
      frontmatter half is unchanged from Phase 1 as predicted; the body half
      differs from both the committed value and the Phase 1 measurement,
      repairing the pre-existing drift dating to `d14beb34a`.
    - **Procedure confirmed and amended by the Phase 1 spike.** `md hash`
      excludes `hash` and `last_updated` from the frontmatter half, so the
      strict ordering this plan originally specified is not load-bearing. The
      one real constraint is that the hash is computed **after the final body
      edit**. For each edited skill file:
      1. confirm all body edits are final — this is the only ordering
         requirement;
      2. bump `last_updated` to the completion date (does not affect the hash);
      3. run `md hash <file>`;
      4. write the result into the `hash:` frontmatter field (writing this
         field does not itself change the hash — that is what makes a
         self-referential hash field possible);
      5. re-run `md hash <file>` and confirm it is unchanged. **Keep this
         step.** It is cheap, and per the Phase 1 spike it is exactly the check
         that would have caught the drift described next.
    - **The file does not start as a fixed point.** Contrary to this plan's
      fact (6), the committed `hash:` on
      `.claude/skills/rust-testing/SKILL.md` is stale: it reads
      `61d07be7e22c9f45-5e3087a71564a05a` while the file actually hashes to
      `61d07be7e22c9f45-fb3622c484a86f23`, and has since commit `d14beb34a`
      edited the body without refreshing. So AC6 is satisfied by **computing
      and writing** the value, never by asserting it is unchanged from the
      committed one. Repairing this pre-existing drift is a side effect, not a
      scope expansion.
    - Default scope is one file, `.claude/skills/rust-testing/SKILL.md`. Per
      R-2 this remains the only skill file edited, so `nextest.md` and the
      Topic Pages table are out of scope.
    - `CLAUDE.md` takes no hash (fact (7)). Do not add frontmatter to it.

- [x] **Run the spec's verification list** — `just --summary`: root exit 0,
      84 recipes, byte-identical to the Phase 1 baseline; `darkmatter` exit 0
      (62 recipes); `sniff` exit 0 (55 recipes). All three `import` the changed
      `just/devops.just` directly (`justfile:22`, `darkmatter/justfile:14`,
      `sniff/justfile:16`), so the two area runs prove the shared import, not
      just that some justfile parses. `_test_workspace` did not change — the
      Phase 2 diff is comment-only — so the conditional local-run branch does
      not apply; Phase 2's `just test biscuit-hash` (70/70, selector-narrowed)
      already exercised the recipe end to end. `md hash --diff` on the edited
      skill: **exit 0, "No semantic changes detected"**.
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

- [x] **Acceptance-criteria audit** — all seven PASS, each with a file:line
      pointer; full table in the implementation log's Phase 4 section. AC7 used
      the R-6 commands (both empty). AC2's framing and AC4's scope sentence were
      re-read as a fresh reader would: AC2 opens "**Consequence for non-vacuous
      proofs.**" and closes with the single-package clause, so it cannot read as
      a standalone always-use-this-flag rule; AC4's scope sentence is the first
      bullet and bolded, so a skimming reader meets it before the restrictive
      bullets. Neither needed a fix.
    - Walk AC1 … AC7 from the spec one at a time against the final diff, and
      record a file:line pointer for each.
    - **AC7 uses the R-6 command, not the one originally written here.** The
      merge-base form cannot pass: three commits predating this fix already
      touch `.github/workflows/`. Verify this fix's own contribution instead —
      `git status --porcelain .github/workflows/` and
      `git diff --name-only HEAD -- .github/workflows/` must both be empty.
    - Re-read the two rules the spec warns are easy to get subtly wrong:
      AC2's framing (a consequence of R1, not a standalone always-use-this
      rule) and AC4's scope sentence (CI/CD only, not a blanket discouragement
      of local testing). If either reads wrong to a fresh reader, fix it here.

- [x] **Closing summary** — recorded in the implementation log's Phase 4
      section. Terminal state is **implementation complete, ready for review**;
      the fix directory is unmoved at `fixes/2026-09-14-cicd-improvements/` and
      `just complete` was not run. Nothing was committed or staged.
    - Record: the R-1 … R-5 decisions and their reasoning; any drift found and
      how it was resolved; the final hash values; and the verification output.
    - **Terminal state is "implementation complete, ready for review."**
      Do not move this fix directory into `fixes/_completed/` — that is R3,
      and this plan is the first thing that must obey it.
    - Do not commit unless separately asked (CLAUDE.md).

### Phase 4 Validation Checkpoint — Done Means

- [x] AC1 through AC7 each have a recorded file:line pointer. *(Audit table in
      the implementation log's Phase 4 section; all seven PASS.)*
- [x] `md hash` is a fixed point on every edited skill file. *(One file.
      `md hash` run twice → identical, and `md hash --diff` exits 0 with
      "No semantic changes detected". The same command on the HEAD blob exits 2
      — "body has changed" — which is the pre-existing drift now repaired.)*
- [x] `just --summary` exits 0 at the root and in two areas. *(root 84 recipes
      byte-identical to baseline, `darkmatter` 62, `sniff` 55 — all three
      import `just/devops.just` directly.)*
- [x] The diff touches no workflow file and no file outside the four named in
      the Summary table (plus this plan). *(`git status --porcelain` over
      `CLAUDE.md AGENTS.md just/ .claude/ docs/ fixes/ .github/` lists exactly
      six paths: the four Summary-table files plus this plan and the
      implementation log. Both R-6 commands empty.)*
- [x] The fix directory is still at
      `fixes/2026-09-14-cicd-improvements/`, unmoved. *(Confirmed by the same
      status output; `just complete` was not run.)*

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
