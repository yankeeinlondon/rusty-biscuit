---
fix: 2026-09-14-cicd-improvements
plan: fixes/2026-09-14-cicd-improvements/plan.md
spec: fixes/2026-09-14-cicd-improvements/spec.md
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
completed_phase: "4"
implemented: true
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
     `CLAUDE.md`, `just/devops.just`, `docs/testing-strategy.md`) plus the plan
     and this implementation log. The working tree also carries unrelated
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

# Implementation Log — CI/CD Improvements

## Phase 1

Phase 1 is a read-only decision phase. No source file, skill, doc, or justfile
outside this fix directory was modified. Every ruling below was made after its
blocking spike reported, per the plan's sequencing constraint.

### Test design mapping

Phase 1 changes no executable behavior, so there is no behavior to map to a
new test. This is not a waived requirement — it is the absence of a trigger:
the phase's entire output is decisions recorded in `plan.md`.

One relevant existing guard was located during the sweep and is worth carrying
forward rather than duplicating: `tools/test-toolkit/tests/ci_workflow_contracts.rs:738`
already asserts that the shared CI L1 workflow passes `--no-fail-fast`, with
the rationale "L1 suites must run --no-fail-fast so one failure cannot hide the
suite's evidence (D7)". The CI half of R1 is therefore already test-enforced.
Phase 2 must not add a second assertion of the same contract, and the skill
prose telling agents "CI already passes this flag, do not add it" is backed by
this test.

### Work-group 1A — Decision spikes

#### Spike: flag reversibility (blocks R-1) — COMPLETE

Run at the repository root:

```
$ just test --no-fail-fast
No workspace packages matched: --no-fail-fast
error: recipe `_test_workspace` failed with exit code 1
error: recipe `test` failed on line 75 with exit code 1
EXIT=1
```

Fact (3) is confirmed empirically: the argument is consumed as a *selector*,
matched against package name and package-area path (`just/devops.just:1134-1148`),
matches nothing, and the recipe exits 1 before any test runs.

Selector narrowing also confirmed:

```
$ just test biscuit-hash
Running Level-1 tests for 2 workspace packages in one invocation...
Testing biscuit-hash biscuit-hash-cli in one local invocation
EXIT=0
```

Two packages, not the workspace — so fact (1) holds: `_test_workspace` is not
always a full-workspace sweep, and the selector-narrowed form still receives
`--no-fail-fast`.

**Correction to the plan — the spike found more than it was asked for.**
Reversibility is *scope-dependent*, which the plan's fact (3) does not say:

| Invocation | `*args` destination | `--no-fail-fast` reversible by caller? |
|---|---|---|
| root `just test` (`justfile:75`) | `_test_workspace *selectors` — consumed as selectors | **No** |
| area `just test` (e.g. `darkmatter/justfile:71`) | `_test_local_all … {{ args }}` → nextest | **Yes** |

Verified by reading `darkmatter/justfile:71-87`, and corroborated by many
repository records of `just test --no-fail-fast` and `just test-l2 --no-fail-fast`
succeeding from inside a package area (e.g.
`darkmatter/features/2026-07-13-meta-schema/log.md:486`, which records the
harness quirk that the flag must be passed bare rather than after `--`).

**Output: removal is NOT reversible by a caller at the repository root.** This
is the deciding input for R-1.

#### Spike: hash fixed-point mechanics — COMPLETE, PLAN PREMISE FALSIFIED

Two findings, both contradicting the plan's fact (6).

**(a) The tracked file is not currently a fixed point.** The working tree is
clean for this path and the HEAD blob hashes identically, so this is committed
drift, not a local edit:

```
$ md hash .claude/skills/rust-testing/SKILL.md
61d07be7e22c9f45-fb3622c484a86f23        # actual
hash: 61d07be7e22c9f45-5e3087a71564a05a  # committed frontmatter
```

Traced across the file's recent history:

| Commit | Committed `hash:` | Actual `md hash` | Fixed point? |
|---|---|---|---|
| `c11df941e` | `7055b0e89017847d-adf6e5f1009bfc55` | `7055b0e89017847d-adf6e5f1009bfc55` | yes |
| `bc087c644` | `7055b0e89017847d-adf6e5f1009bfc55` | `7055b0e89017847d-8800fd3c48f340c6` | **no** |
| `d14beb34a` (HEAD) | `61d07be7e22c9f45-5e3087a71564a05a` | `61d07be7e22c9f45-fb3622c484a86f23` | **no** |

The body half has been stale since `bc087c644`; the frontmatter half was
refreshed in `d14beb34a` but the body half was not. The tool is consistent —
`c11df941e` matches exactly — so this is authoring drift, not a hashing bug.

Consequence for Phase 4: AC6 must be satisfied by *computing and writing* the
hash, and it will incidentally repair pre-existing drift. "Unchanged from the
committed value" is not a valid check.

**(b) The proposed Phase 4 ordering is ceremonial, not required.** On a scratch
copy (tracked file untouched, scratch deleted):

| Mutation | Resulting hash | Moved? |
|---|---|---|
| baseline | `61d07be7e22c9f45-fb3622c484a86f23` | — |
| `last_updated: 2026-09-12` → `2026-09-14` | `61d07be7e22c9f45-fb3622c484a86f23` | no |
| rewrite the `hash:` field itself | `61d07be7e22c9f45-fb3622c484a86f23` | no |
| `name: rust-testing` → `rust-testing-x` | `c2a2f2c28bc705e9-fb3622c484a86f23` | **frontmatter half** |
| append a body line | `61d07be7e22c9f45-5861014924fe0fe7` | **body half** |

`md hash` excludes `hash` and `last_updated` from the frontmatter hash — which
is what makes a self-referential hash field possible at all. The plan predicted
the `last_updated` bump would move the frontmatter half and that the ordering
was therefore load-bearing; it does not, and it is not.

**Output — confirmed Phase 4 procedure.** The only real constraint is that the
hash is computed after the final body edit. Bumping `last_updated` before or
after hashing is immaterial. Step 5 (re-run and confirm a fixed point) is still
worth keeping as a cheap assertion, and given finding (a) it is the step that
would have caught this drift twice.

#### Spike: contradiction sweep (read-only) — COMPLETE

Swept `--no-fail-fast` and `_completed` across `*.md`, `*.just`, `justfile`,
`*.yml`, `*.rs`, and `*.sh`, excluding `**/_completed/**`, `**/reviews/**`, and
dated feature/fix artifacts (`*/features/YYYY-MM-*`, `*/fixes/YYYY-MM-*`) as
historical records per the plan.

Drift candidate checklist carried into Phase 3 — verdicts below are **under the
R-1 = keep ruling**; the two conditional rows flip if that ruling is overridden:

| # | Location | Claim | Verdict |
|---|---|---|---|
| 1 | `just/devops.just:1204-1205` | comment: `--no-fail-fast` "preserves the old workspace loop's promise" | **DRIFT — fix in Phase 2.** The workspace loop it names no longer exists, and the comment is silent on the selector case. This is the R-1 target. |
| 2 | `docs/testing-strategy.md:302-306` | root `just test` hands all packages to one nextest invocation with `--no-fail-fast` | **Not drift under keep.** Accurate, including "one exception to area iteration". Does not contradict the selector nuance — it never claims the set is always every package. Leave alone (Rule 3). *Conditional: becomes false under drop.* |
| 3 | `docs/testing-strategy.md:759-760` | "Level 1 runs with `--no-fail-fast`" | Not drift. CI-side statement, true under either ruling. Plan explicitly says do not touch. |
| 4 | `.claude/skills/rust-testing/SKILL.md:390` | "L1 runs with `--no-fail-fast`, and CI selects the `ci` nextest profile" | Not drift. CI-side, true either way. |
| 5 | `.claude/skills/rust-testing/SKILL.md:405-408` | root `just test` → `_test_workspace`, one invocation, `--no-fail-fast`, selectors allowed | **Not drift — but read before writing Phase 2 prose.** This is *mechanism* and is already correct. The R1 *policy* goes in "Canonical Just Recipes"; do not restate the mechanism there or Phase 3's no-duplicate check fails. *Conditional: becomes false under drop.* |
| 6 | `memory/just.md:52` | `_test_all` selects `_test_local_all` locally, `_run_all _test` under CI | **Not drift (R-5).** Makes no fail-fast claim; unaffected by either ruling. |
| 7 | `memory/just.md:208` | `just _recipe --no-fail-fast --archive-file /x` passes all three through to `*args` | **Not drift (R-5).** True — it describes `just`'s own option parsing, not what a recipe then does with the args. It is adjacent to the root-`just test` trap but does not assert anything false. Leave untouched. |
| 8 | `just/ci-local.just:7,485` | local CI emulation passes `--no-fail-fast` to `_test` | Not drift. CI-shaped; this is R1 being implemented correctly. |
| 9 | `.claude/skills/os/wsl.md:71` | archive-reproduction recipe uses `--no-fail-fast` | Not drift. CI-shaped reproduction of the WSL2 leg. Consistent with R1. |
| 10 | `.claude/agents/feature-tester-rust.md:245` | nextest cheat sheet: `--no-fail-fast  # Continue on failures` | Not drift. A neutral flag reference with no policy claim. Editing it would be scope creep (Rule 3). |
| 11 | `scripts/cross-check.sh:267,294,440` | cross-OS verification runs `--no-fail-fast` | Not drift. Multi-OS, CI-shaped. |
| 12 | `tools/test-toolkit/tests/ci_workflow_contracts.rs:738-739` | test asserting shared CI workflow contains `--no-fail-fast` | Not drift — it is the guard. Do not duplicate in Phase 2. |
| 13 | `docs/topics/ci-cd.md:318,429` | `fail-fast: false` on matrices | Not drift. GitHub Actions matrix `fail-fast`, an unrelated concept from nextest's flag. Flagged here only so Phase 3 does not confuse the two. |
| 14 | `CLAUDE.md:138` | "when a feature/fix is completed it is moved to `_completed`" | **DRIFT — the AC5 target.** Passive voice, no owner. |
| 15 | `.claude/skills/just/SKILL.md:78` and `.claude/agents/just-scripter.md:23` | "`complete` moves a scheduled feature to the `_completed` directory" | **Not drift, but consequential.** Both describe the mechanism of `just complete` (`just/lifecycle.just:108`), which remains accurate. They are, however, where an agent learns the move is one command away. See R-3 note: the CLAUDE.md rule should name `just complete` explicitly. |
| 16 | `claudine/features/_unscheduled/feature-fix-lifecycle/` | proposes automating the `_completed/` move | Not drift (unscheduled proposal, not live guidance). Recorded so that whoever schedules it reconciles it with R3. |
| 17 | `.claudine/memory/commits.md:174` | `planning` commit type covers moves into `_completed` | Not drift. Commit-type taxonomy, not authorization. |

No passive-voice `_completed/` guidance exists anywhere outside `CLAUDE.md:138`
in live (non-historical) files, so AC5 has exactly one edit site.

### Work-group 1B — Evidence baseline

| Baseline | Value |
|---|---|
| `just --summary` (root) | exit **0**, 84 recipes, captured at `/tmp/ph1_just_summary.txt` |
| `md hash .claude/skills/rust-testing/SKILL.md` | `61d07be7e22c9f45-fb3622c484a86f23` (**not** the committed value — see spike (a)) |
| committed `hash:` frontmatter | `61d07be7e22c9f45-5e3087a71564a05a` (stale) |
| `git merge-base HEAD main` | `aad933bdb725259062b446c0baba8f708c4ccbde` |
| branch commits since merge-base | 30 |
| staged changes | **none** |

The 84-recipe summary list is recorded in full in the plan's Phase 1 baseline
so Phase 2 and Phase 4 can diff against it without re-deriving it.

**Working-tree caveat for Phase 2.** Staged changes are clean, but the working
tree carries unrelated modifications (`darkmatter/docs/research/parsing/*`,
`darkmatter/features/*`, `prompts/*`, plus several untracked `claudine/fixes/`
and `darkmatter/` directories). None are in this fix's scope. Phase 2's
checkpoint says "`git diff --stat` lists only the files the executed rulings
authorize" — that will not hold against a bare `git diff --stat`. Scope it to
the authorized paths:

```
git diff --stat -- CLAUDE.md just/devops.just .claude/skills/rust-testing/
```

**AC7 baseline is already red — see R-6.** `.github/workflows/` shows 5 files
changed between the merge base and HEAD, from three prior commits on this
branch. The working tree itself has no workflow changes
(`git status --porcelain .github/workflows/` is empty), which is the condition
this fix actually has to preserve.

### Phase 1 Validation Checkpoint

- Every ruling R-1 … R-5 has a recorded answer in `plan.md`, plus a new R-6
  recording the AC7 verification correction.
- R-1 was decided after the flag-reversibility spike reported, and the spike
  changed its reasoning (scope-dependent reversibility), so the sequencing
  constraint did real work rather than rubber-stamping the code read.
- The drift checklist exists with 17 entries and includes both known entries
  (`docs/testing-strategy.md:304` as row 2, `memory/just.md:52,208` as rows
  6-7).
- Baselines captured. No file outside `fixes/2026-09-14-cicd-improvements/`
  was modified; verified with `git status --porcelain`.

### Gates run

No lint or test gate applies to this phase — nothing compilable changed. The
executable checks that were run are the spikes themselves: `just test
--no-fail-fast` (exit 1, as predicted), `just test biscuit-hash` (exit 0, 2
packages), `just --summary` (exit 0), and `md hash` across four file revisions.

No pre-existing failures were encountered, and nothing was skipped.

## Phase 2

Two work-groups, disjoint files, executed serially by one agent. Every ruling
carried in from Phase 1 was executed as recorded; no ruling was reopened.

### Test design mapping

Phase 2 changes no executable behavior, so no test is triggered. This is a
reasoned absence, not a waiver:

| Changed file | Nature of change | Test consequence |
|---|---|---|
| `.claude/skills/rust-testing/SKILL.md` | prose only | none — skill text is not executed |
| `CLAUDE.md` | prose only, no frontmatter | none |
| `just/devops.just` | **comment lines only** — verified against `git diff` | none; behavior byte-identical |

The R-1 keep ruling is what makes this true. Had the flag been dropped, the
`_test_workspace` invocation would have changed and a guard would have been
required. It was not dropped.

Two existing guards cover this area and were deliberately **not** duplicated,
per the plan's explicit instruction:

- `tools/test-toolkit/tests/ci_workflow_contracts.rs:738` already asserts the
  shared CI L1 workflow passes `--no-fail-fast`. The skill prose now added
  ("CI already passes it, do not add it") is therefore test-backed rather than
  merely asserted.
- `just check-test-interrupts` already covers the Ctrl+C/exit-130 contract of
  the recipe whose comment changed.

Rather than write a new test for unchanged behavior, the touched recipe was
exercised end-to-end through its normal invocation path (below).

### Work-group 2A — R1

**`.claude/skills/rust-testing/SKILL.md:333`** — new H3, "Fail-fast is an
environment policy, not a flag preference", placed inside "Canonical Just
Recipes" (R-2's chosen home, at its corrected line 312) directly after the
`Delegate to shared recipes` sentence. H3 subsections are the file's existing
idiom (9 already present), so this matches altitude rather than introducing a
new structure. Content, in the order AC1 requires: local fail-fast is nextest's
default and correct; CI runs to completion; the reason is the cost of the next
run, which is what makes it environmental; a truncated Windows/WSL2 report
costs a round-trip in hours.

Three CI files are cited **by name only** — `_package-ci.yml`, `_wsl-ci.yml`,
`just/ci-local.just` — with no line numbers, per CLAUDE.md Code Comment Quality
item E. The spec's own line citations (`:438`, `:648`, `:462`) were deliberately
dropped in the skill text for exactly that reason.

The R-1 selector nuance is recorded with the irreversibility claim **scoped to
the repository root**, as Phase 1 required. The paragraph states the opposite
case explicitly — inside a package area the arguments reach nextest, so the
fail-fast default survives and is overridable — so a reader cannot generalize
the root behavior to the recipe they actually use daily.

**`SKILL.md:356`** — the AC2 paragraph, opening literally with "**Consequence
for non-vacuous proofs.**". The framing was the spec's stated failure mode (an
earlier draft's "always run the neutered pass with `--no-fail-fast`" is what
prompted this fix), so the paragraph both carries the CI-shaped/multi-package
requirement *and* closes with "against a single package locally, fail-fast is
correct and faster" — the clause that prevents it being read as a blanket rule.

**`just/devops.just:1204-1211`** — R-1's keep branch. The flag is untouched; the
comment above it grew from two lines to eight. The removed claim ("preserves the
old workspace loop's promise") named a loop that no longer exists — the drift
Phase 1 flagged as row 1. The replacement earns its length under CLAUDE.md
criterion B (why a counter-intuitive choice was made, at the surprising line):
it says why the broadest local scope sits on the CI side of the cost test, that
the root caller cannot re-specify the flag because arguments are consumed as
selectors *above in the same recipe*, and that area recipes keep the default.

Scope discipline confirmed — `git diff -- just/devops.just` shows one hunk whose
only non-comment line is the unchanged context line
`just _test_local_all "${specs}" --no-fail-fast`. No split commit is needed.

### Work-group 2B — R2 and R3

**`CLAUDE.md:51`** — new H2 `## CI/CD Test-scope Discipline`, immediately before
"Evidence Reuse and Execution Constraints" (R-3 option (a), which was still at
line 51 as Phase 1 recorded). Bullet style and line width match the
neighbouring sections.

The scope sentence is the **first** bullet and is bolded. The spec is emphatic
that an unscoped framing would discourage exactly the local testing that should
be encouraged, so placing it first means a skimming reader cannot reach the
restrictive bullets before the limiting one. A closing bullet ties the section
to its neighbour: upstream question (should the cell exist) versus downstream
(must it re-run).

**`CLAUDE.md:152-155`** — the AC5 replacement. The passive "when a feature/fix
is completed it is moved to `_completed`" becomes two bullets naming the author
as the only mover, conditioned on the review cycle closing, plus the agent-side
prohibition and its terminal state. Per R-3's addition it names `just complete`
explicitly — the recipe exists at `just/lifecycle.just:108` and is documented to
agents in two places, so a prohibition that did not name it would leave the
button documented and unguarded.

`AGENTS.md` verified a symlink (`AGENTS.md -> ./CLAUDE.md`) and not edited.

### Gates run

| Check | Result |
|---|---|
| `git diff --stat` (path-scoped per Phase 1 caveat) | exactly 3 authorized files, +55/-3 |
| `git status --porcelain .github/workflows/` (R-6) | empty |
| `git diff --name-only HEAD -- .github/workflows/` (R-6) | empty |
| `git diff -- just/devops.just` | comment lines only |
| `just --summary` at root | exit 0, 84 recipes, **byte-identical** to the Phase 1 capture |
| `just test biscuit-hash` (smoke of the touched recipe) | 70 tests run, 70 passed, 0 skipped |

The `just test biscuit-hash` run is the end-to-end exercise of the only
executable file touched, through its normal invocation path and with a selector
— the branch whose behavior the new comment describes. `just --summary` alone
proves the justfile parses but would not have caught a broken recipe body.

No lint gate applies: no Rust source, manifest, or config file changed, so
clippy has nothing to report that it did not report before. Running a
workspace-scoped lint to prove a prose diff is precisely the speculative cost
the R2 rule added in this very phase tells agents not to pay.

No tests were skipped and no pre-existing failures were encountered.

### Deviations and corrections

- **One Phase 1 verdict is not being inherited silently.** Phase 1 recorded
  `docs/testing-strategy.md:302-306` as not-drift because "it never claims the
  set is always every package". Re-reading the literal sentence while writing
  the skill prose, that reasoning does not hold — the text says the recipe
  "discovers every workspace package ... and hands all of them to a single
  nextest invocation" and never mentions selectors. The fail-fast half remains
  true under the keep ruling, so Phase 3's *conditional* reconciliation is not
  triggered, and reconciling it is Phase 3's task rather than this phase's. The
  disagreement is recorded in `message_to_agent` so Phase 3 decides it on the
  text rather than on an inherited verdict. No edit was made here.
- No other deviation. R-1 through R-6 were executed exactly as recorded.

## Phase 3

Drift reconciliation. One file edited (`docs/testing-strategy.md`), one Phase 1
verdict overridden on the text rather than inherited, and the no-duplicate
check run empirically rather than asserted.

### Test design mapping

Phase 3 changes no executable behavior — the only edit is prose in a
`docs/` file — so no new test is triggered. As in Phases 1 and 2 this is a
reasoned absence, not a waiver. What is different here is that the changed
artifact **is** already covered by a passive corpus test, so there was a real
gate to run rather than an argument to make:

| Changed file              | Nature of change | Covering test                                                                              |
|---------------------------|------------------|--------------------------------------------------------------------------------------------|
| `docs/testing-strategy.md` | prose + `updated:` frontmatter bump | `tools/test-toolkit/tests/ci_workflow_contracts.rs:797` `active_ci_authority_matches_the_retirement_contract` reads this file by path into a corpus and asserts no retired CI entity is named in it; `no_reader_facing_document_or_recipe_names_a_retired_ci_entity` covers the same class repo-wide. |

That is the "passive corpus test covering all shipped artifacts" the design
requirements ask for, and it already exists — writing a second one would be the
duplicate this very phase is chartered to prevent. It was run (below), not
merely cited.

No new test was added because no behavior changed. Had Phase 3 needed to edit
`just/devops.just` — the drop branch of R-1 — the `just check-test-interrupts`
and `the_l1_suite_runs_no_fail_fast` guards would have been the targeted gates.
Both ran green anyway as part of the suite.

### `docs/testing-strategy.md` — the judgement call Phase 2 handed over

**Verdict: Phase 1's row-2 not-drift verdict is overridden. A one-clause fix
was applied.** Phase 2 explicitly declined to decide this and asked Phase 3 to
decide it on the text; recorded here so the reasoning is auditable.

The conditional reconciliation the plan describes was **not** triggered — R-1
kept the flag, so `--no-fail-fast` in that sentence is still true and was not
touched. The separate question is the *omission* Phase 2 flagged.

Phase 1 recorded the row as not-drift on the reasoning "it never claims the set
is always every package". Phase 2 doubted that. Re-reading the section as a
whole rather than the sentence alone settles it, and produces a third reason
neither phase had:

```text
docs/testing-strategy.md:293-294   just test              # every workspace package
                                   just test biscuit-file # one package, or every
                                                          #   package under an area path
docs/testing-strategy.md:301-304   "It discovers every workspace package ... and hands
                                    all of them to a single nextest invocation"
```

The code block five lines above the prose documents the selector form
explicitly. So the file contradicts *itself* within one section: the example
says "one package", the prose immediately below says "all of them". This is not
a stylistic preference about a correct sentence, which is what Rule 3 protects —
it is an internal inconsistency, and CLAUDE.md's drift rule (assume the code is
correct, the doc is wrong) resolves it in favor of the code.

Two further corroborations, both of which postdate Phase 1's verdict:

- The recipe's own docblock already says it (`just/devops.just:1116-1117`:
    "Optional selectors may be package names or package-area paths").
- The repository's own skill text already says **"every selected package"**
    (`.claude/skills/rust-testing/SKILL.md:434-436`). The doc was the only live
    place still saying "all of them".

**The fix is one clause and one pointer; the paragraph is not restructured**,
per the plan's constraint:

```diff
-workspace package from `cargo metadata`, resolves each package's declared
-features, and hands all of them to a single nextest invocation with
-`--no-fail-fast`, so one scheduler sees every test binary and one failure
-cannot hide the rest. Ctrl+C stops it immediately with exit code 130, and
+workspace package from `cargo metadata`, narrows that set when selectors are
+given, resolves each selected package's declared features, and hands them to a
+single nextest invocation with `--no-fail-fast`, so one scheduler sees every
+selected test binary and one failure cannot hide the rest. Selector-narrowed
+runs inherit the flag by design; see the `rust-testing` skill for why.
+Ctrl+C stops it immediately with exit code 130, and
```

"every selected test binary" now matches the wording of the `just/devops.just`
comment Phase 2 wrote and of the skill's mechanism paragraph, so the three
places that describe this recipe are finally consistent with each other.

The pointer is deliberately a **by-name citation with no reasoning**
(CLAUDE.md Code Comment Quality item E). Restating *why* the flag is baked in
would have created the second copy this phase exists to prevent; sending the
reader to the one home instead is the opposite.

`updated:` frontmatter bumped `2026-09-12` → `2026-09-15`. This file carries no
`hash:` field, so **it is not an AC6 target** and Phase 4's hash work does not
expand to it.

### Drift checklist — re-verification against the final Phase 2 wording

Phase 1's 17 rows were re-verified against the text as it now stands, not as it
was predicted to stand. The sweep was also re-run with its path filters relaxed,
which surfaced **5 rows Phase 1 missed**. Twenty-two rows total; all resolved.

Phase 1 rows — changes from the Phase 1 verdict only:

| \# | Location                                    | Re-verified verdict                                                                                                                             |
|----|---------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|
| 1  | `just/devops.just:1204-1211`                | **Was drift; fixed in Phase 2.** Re-read: the new comment scopes irreversibility to the root and names the area case. Correct against final wording. |
| 2  | `docs/testing-strategy.md:301-307`          | **VERDICT CHANGED — drift; fixed in this phase.** See above.                                                                                       |
| 5  | `.claude/skills/rust-testing/SKILL.md:434-440` (was 405-408) | Not drift, and **not collapsed** into the new policy H3 per R-2. Overlap is in subject, not content.                            |
| 14 | `CLAUDE.md:152-155` (was 138)               | **Was drift; fixed in Phase 2.** Re-read: active voice, names the author, names `just complete`. Correct.                                           |
| 15 | `.claude/skills/just/SKILL.md:78`, `.claude/agents/just-scripter.md:23` | Not drift, **re-confirmed against the final CLAUDE.md wording.** They describe what `just complete` does (still accurate — `just/lifecycle.just:164` performs the move) and assert nothing about who may run it. Adding the prohibition here would be the second copy R-4 forbids; naming `just complete` in CLAUDE.md was the agreed resolution and it is in place. |

Rows 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 16, 17 re-verified unchanged. R-5 in
particular (`memory/just.md:52` and `:208`) re-read in full: neither makes a
fail-fast policy claim, `:208` correctly describes `just`'s own option parsing
rather than what a recipe does with the arguments, so **R-5's "nothing to fix"
outcome stands and no edit was made to `memory/`**.

Five rows the Phase 1 sweep did not surface, all not-drift:

| \#  | Location                                                    | Claim                                                             | Verdict                                                                                                                                                          |
|-----|-------------------------------------------------------------|-------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 18  | `tools/test-toolkit/tests/nextest_config_verification.rs:111` | a self-invoked `cargo nextest run ... --no-fail-fast -E test(=…)`  | Not drift. A single-test probe of the slow-timeout config; no policy claim, and the flag is irrelevant to a one-test filter.                                         |
| 19  | `scripts/ci/test_ci_local.py:1023,1041,1043,1057`            | fixtures asserting `_test <pkg> --no-fail-fast --features …`       | Not drift. These pin the **CI-local emulation** contract (row 8's implementation). R1 requires exactly this.                                                          |
| 20  | `scripts/ci/test_affected_scope.py:3818`                     | pins the `_package-ci.yml` L1 command string verbatim              | Not drift — a second guard on the CI side of R1, alongside row 12.                                                                                                   |
| 21  | `scripts/cross-check.sh:151`                                 | comment: forwards `--no-fail-fast` etc. to `nextest run`           | Not drift. Argument-routing description, no policy claim.                                                                                                            |
| 22  | `claudine/fixes/_unscheduled/test-suite-residuals/spec.md:151-152` | records a past local `just _test claudine --no-fail-fast` run | Not drift. An *unscheduled* spec recording evidence from a diagnostic run that needed a complete failure list across a large package — the case AC2 says the flag is for. It prescribes nothing. |

`.claude/skills/nextest/` and `.claude/skills/rust-testing/nextest.md` were also
re-read because the sweep hit them heavily. They document the **nextest tool's**
`fail-fast` profile key, not repository policy, and their illustrative config
(`[profile.default] fail-fast = true` / `[profile.ci] fail-fast = false`) is
independently *consistent* with R1. `rust-testing/nextest.md:282`
(`cargo nextest run -p my_crate --fail-fast`) is consistent with AC2's closing
clause. No edit.

Confirmed again, as Phase 1 noted: `docs/topics/ci-cd.md:318,429`,
`docs/testing-strategy.md:755`, and `.claude/skills/rust-devops/ci-cd.md:271`
use `fail-fast` in the **GitHub Actions matrix** sense. That is an unrelated
concept from nextest's flag and was deliberately not "reconciled".

### No second copy of any rule

Run against the final Phase 2 text, searching `.claude/skills/` and `docs/`:

| Rule | Distinctive phrases searched                                                       | Hits outside its home | Conclusion                                                      |
|------|--------------------------------------------------------------------------------------|-----------------------|-------------------------------------------------------------------|
| R1   | `cost of the next run`, `cost-of-next-run`, `environment policy`                     | **0**                 | One home: `SKILL.md:333-360`.                                     |
| R2   | `speculative cell`, `what question it answers`, `before adding a CI run`, `test-scope` | **0**                 | One home: `CLAUDE.md:51-64`. **R-4 confirmed empirically.**       |
| R3   | `ready for review`, `implementation complete`, `_completed`                          | 2 unrelated¹          | One home: `CLAUDE.md:152-155`.                                    |

¹ `.claude/agents/rust-designer.md:359` and `rust-developer.md:360` both carry a
`## Rust Implementation Complete` **report-section heading**. Neither mentions
`_completed/` nor the lifecycle move (verified — they do not appear in the
`_completed` sweep of `.claude/`). Not a duplicate of R3.

The one genuine near-duplicate is the R1 policy/mechanism pair inside
`SKILL.md`. R-2 ruled the split deliberate and Phase 2 flagged it for this
check. Re-read side by side: `:333-360` is the *decision and its reasoning*
(when to pay for completeness, why the root recipe bakes the flag in, what a
non-vacuous proof needs); `:434-440` is the *mechanism* (what `_test_workspace`
does with Cargo metadata, selectors, and features). Neither states the other's
content. **Not collapsed**, per the ruling.

### Gates run

| Check                                                                        | Result                                            |
|------------------------------------------------------------------------------|---------------------------------------------------|
| `just test test-toolkit` — the suite owning the corpus test over the edited doc | **183 run, 183 passed, 2 skipped**                |
| `just _lint test-toolkit`                                                    | clean; no warnings                                |
| `just --summary` at root                                                     | exit 0, 84 recipes, **byte-identical** to Phase 1 |
| `git status --porcelain .github/workflows/` (R-6)                            | empty                                             |
| `git diff --name-only HEAD -- .github/workflows/` (R-6)                      | empty                                             |
| scoped `git diff --stat`                                                     | 4 files, +62/-8 — exactly the four in the plan's Summary table |

The 2 skipped tests are pre-existing environment-gated skips in `test-toolkit`,
unrelated to this fix and skipped identically before the change. No test was
skipped by choice and **no pre-existing failure was encountered**.

The lint run is deliberately package-scoped rather than the root `just lint`,
which orchestrates every curated area. Nothing Rust-side changed in this phase;
a workspace lint would answer no question. That reasoning is the R2 rule this
fix added — noting, per its own scope sentence, that the local bar is different
and a bounded local lint is cheap enough to run anyway, which is why it was.

### Deviations and corrections

- **Phase 1's row-2 verdict was overridden, not inherited.** Recorded in full
    above with the reason (an intra-section contradiction, not a wording
    preference). Phase 2 explicitly declined to make this call and routed it
    here; that hand-off worked as intended.
- **The Phase 1 sweep was incomplete by 5 rows.** Its path exclusions
    (`*/features/YYYY-MM-*`, `*/fixes/YYYY-MM-*`) were correct in intent but
    also filtered `scripts/ci/*.py` fixtures and one `_unscheduled/` spec out of
    view. Re-running the sweep unfiltered found them; all five are not-drift, so
    **no earlier verdict was wrong** — the checklist was merely short. Noted so
    a future sweep does not trust the 17 as exhaustive.
- **One file was added to the change set beyond Phase 2's three**:
    `docs/testing-strategy.md`. It is the fourth and last file in the plan's own
    Summary table, so this is the planned scope, not an expansion.
- No other deviation.

## Phase 4

Hash refresh, verification, and the acceptance-criteria audit. One file edited
(`.claude/skills/rust-testing/SKILL.md`, frontmatter only). This is the
terminal phase; the closing summary is at the end of this section.

### Test design mapping

Phase 4 changes two frontmatter fields (`hash`, `last_updated`) in a skill
document. No executable behavior changes, so no new test is triggered. As in
Phases 1-3 this is a reasoned absence rather than a waiver — but unlike those
phases there *is* a shipped-artifact check with a real exit code, so it was run
rather than argued:

| Changed file                           | Nature of change     | Gate                                                                              |
|----------------------------------------|----------------------|-----------------------------------------------------------------------------------|
| `.claude/skills/rust-testing/SKILL.md` | frontmatter fields   | `md hash --diff <file>` — Darkmatter's own stored-hash comparison, exits 2 on drift |

`md hash --diff` is the end-to-end case through the real shipped artifact on
its normal invocation path: it reads the committed frontmatter, recomputes from
the file's actual bytes, and compares. It is strictly stronger than the plan's
step 5 ("re-run `md hash` and confirm unchanged"), which only proves the hash
function is deterministic, not that the stored field agrees with it. Both were
run.

The round trip the design requirements ask for is present and was exercised in
both directions:

```text
$ md hash .claude/skills/rust-testing/SKILL.md      # read  → compute
61d07be7e22c9f45-6f2ea54170be340f
                                                    # write → hash: field
$ md hash .claude/skills/rust-testing/SKILL.md      # read  → recompute
61d07be7e22c9f45-6f2ea54170be340f                   #   identical
$ md hash --diff .claude/skills/rust-testing/SKILL.md
No semantic changes detected
EXIT=0
```

The negative case was also produced, from the HEAD blob rather than by
corrupting the working tree:

```text
$ git show HEAD:.claude/skills/rust-testing/SKILL.md > /tmp/ph4_head_skill.md
$ md hash --diff /tmp/ph4_head_skill.md
Frontmatter remains unchanged, but body has changed
EXIT=2
```

That is the pre-existing committed drift the Phase 1 spike found, reproduced
through the tool's own comparison path and exiting non-zero, next to the same
command exiting zero on the repaired file. The check therefore distinguishes
the broken state from the fixed one — it is not a self-comparison.

**No repository-wide corpus test over skill hashes was added, and that is a
deliberate scope decision, not an oversight.** See the recommendation at the
end of this section.

### Hash refresh (AC6)

The hash was computed after the final body edit. Phase 3 touched no skill file,
so Phase 2's edit was the last one and the value was final the moment it was
computed.

| Measurement                                      | Value                                 |
|--------------------------------------------------|---------------------------------------|
| committed `hash:` before this phase              | `61d07be7e22c9f45-5e3087a71564a05a`   |
| Phase 1 baseline `md hash` (pre-Phase-2 body)    | `61d07be7e22c9f45-fb3622c484a86f23`   |
| **Phase 4 `md hash` (final)**                    | **`61d07be7e22c9f45-6f2ea54170be340f`** |
| second run, for the fixed-point check            | `61d07be7e22c9f45-6f2ea54170be340f`   |

Both handoff predictions held exactly: the frontmatter half is unchanged at
`61d07be7e22c9f45` (Phase 2 changed no frontmatter field, and `md hash`
excludes `hash`/`last_updated` anyway), and the body half differs from *both*
the committed value and the Phase 1 measurement — from the committed value
because of the pre-existing drift, and from the Phase 1 measurement because the
body grew by 29 lines in Phase 2.

`last_updated` bumped `2026-09-12` → `2026-09-15`. Per the Phase 1 spike this
does not affect the hash, so the plan's original strict ordering was ceremonial
and was not followed as ritual; the one real constraint — hash after the final
body edit — was.

**Cross-OS check on the committed hash.** A `hash:` field committed from macOS
is read back on Windows and Linux, so it is worth knowing whether a Windows
checkout with `core.autocrlf=true` would see AC6 fail. It would not — `md hash`
normalizes whitespace by default (that is what `--strict` turns off), and the
value is byte-identical across line endings:

```text
LF  : 61d07be7e22c9f45-6f2ea54170be340f
CRLF: 61d07be7e22c9f45-6f2ea54170be340f
```

Verified on a scratch copy, which was deleted; the tracked file was not
modified. No CI evidence was needed to establish this.

AC6 was satisfied by **computing and writing**, never by asserting the value
was unchanged. `CLAUDE.md` takes no hash (it carries no frontmatter) and
`docs/testing-strategy.md` carries frontmatter without a `hash:` field, so
neither is an AC6 target and none was added to either.

### Verification

| Check                                                          | Result                                                          |
|----------------------------------------------------------------|-----------------------------------------------------------------|
| `just --summary` (root)                                        | exit **0**, 84 recipes, **byte-identical** to the Phase 1 baseline |
| `just --summary` (`darkmatter`)                                | exit **0**, 62 recipes                                          |
| `just --summary` (`sniff`)                                     | exit **0**, 55 recipes                                          |
| `md hash` ×2 on the edited skill                               | identical — fixed point                                         |
| `md hash --diff` on the edited skill                           | exit **0**, "No semantic changes detected"                      |
| `git status --porcelain .github/workflows/` (R-6)              | empty                                                           |
| `git diff --name-only HEAD -- .github/workflows/` (R-6)        | empty                                                           |
| scoped `git diff --stat` (the four Summary-table files)        | 4 files, +64/-10                                                |

The two area runs are not decorative. All three justfiles import the changed
file directly, so a parse failure in `just/devops.just` would surface in each:

```text
justfile:22            import "./just/devops.just"
darkmatter/justfile:14 import "../just/devops.just"
sniff/justfile:16      import "../just/devops.just"
```

**The conditional local-run branch of the verification list does not apply.**
`_test_workspace` did not change — Phase 2 took R-1's keep branch and its diff
for that file is comment-only — so there is no changed behavior to demonstrate.
Phase 2's `just test biscuit-hash` (70 run, 70 passed) already exercised the
recipe end to end through a selector-narrowed invocation, which is the branch
the new comment describes.

Per the spec, no CI evidence was gathered and none is required: nothing in this
fix changes what CI runs. Collecting a CI run here would be precisely the
speculative cell the R2 rule this fix adds tells agents not to pay for.

No lint or test gate is *triggered* by this phase — the only change is two
frontmatter fields in a Markdown file, and no Rust source, manifest, or config
changed in any phase of this fix except comment lines in `just/devops.just`.
The owning suite was nevertheless re-run against the **final** state rather
than citing Phase 3's result, because local runs are cheap and the closing
summary asserts the whole fix is green, not just the state as of Phase 3:

| Gate                     | Result                                          |
|--------------------------|-------------------------------------------------|
| `just test test-toolkit` | **183 run, 183 passed, 2 skipped** (1.17 s)     |
| `just _lint test-toolkit`| clean, no warnings                              |

The 2 skips are the same pre-existing environment-gated skips present before
this fix. The lint is package-scoped rather than the root `just lint`, which
orchestrates every curated area: nothing Rust-side changed, so a workspace lint
would answer no question — which is the R2 rule this fix adds, applied to
itself, while still paying for the bounded local run its scope sentence
encourages.

**No test was skipped by choice and no pre-existing failure was encountered in
any phase of this fix.**

### Acceptance-criteria audit

Walked one at a time against the final diff. Criteria are quoted from
`spec.md:180-195`, not from the plan's restatement.

| AC  | Criterion                                                                    | Pointer                                              | Verdict                                                                                                                                                                                                                                                                       |
|-----|------------------------------------------------------------------------------|------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| AC1 | skill states the fail-fast policy, the cost-of-next-run reasoning, and that CI already passes the flag | `.claude/skills/rust-testing/SKILL.md:333-354`       | **PASS.** Local default at `:335`, CI at `:337`, the reason — "keys on **how expensive the next run is**" — at `:337-341`, and "**CI already passes `--no-fail-fast`** … Do not add it" at `:343-344`, citing the three files by name only.                                    |
| AC2 | non-vacuous-proof consequence; complete list at CI-shaped/multi-package scope; fail-fast correct for one package locally | `.claude/skills/rust-testing/SKILL.md:356-360`       | **PASS.** Opens "**Consequence for non-vacuous proofs.**"; states the truncated list "looks exactly like a narrow blast radius"; closes "Against a single package locally, fail-fast is correct and faster."                                                                  |
| AC3 | `_test_workspace` keeps or drops the flag by explicit decision, with reasoning in a comment; no caller regresses | `just/devops.just:1204-1212`                         | **PASS.** Keep, by the R-1 ruling dated in the skill text. The comment says "every **selected** test binary" and "selector-narrowed runs inherit it by design" — it never claims a full-workspace sweep, so it survives the plan's fact (1). Flag unchanged; no caller regressed. |
| AC4 | CI/CD test-scope rule with scope explicitly limited to CI/CD, adjacent to evidence reuse | `CLAUDE.md:51-64`                                    | **PASS.** `## CI/CD Test-scope Discipline` at `:51`, immediately before `## Evidence Reuse and Execution Constraints`. Scope sentence is the **first** bullet and bolded (`:53`).                                                                                              |
| AC5 | author named as the only party who moves work into `_completed/`; agent stops at "ready for review" | `CLAUDE.md:152-155`                                  | **PASS.** "the **author** moves a feature/fix into `_completed`, and only after the review cycle closes" (`:152`); "an agent never makes that move and never runs `just complete`; an agent's terminal state is 'implementation complete, ready for review'" (`:155`).         |
| AC6 | skill hashes refreshed with `md hash` for every skill file changed            | `.claude/skills/rust-testing/SKILL.md:8-9`           | **PASS.** One skill file changed across the whole fix (R-2). `hash: 61d07be7e22c9f45-6f2ea54170be340f`; `md hash --diff` exits 0.                                                                                                                                             |
| AC7 | no CI workflow file is modified                                              | R-6 commands                                         | **PASS.** `git status --porcelain .github/workflows/` and `git diff --name-only HEAD -- .github/workflows/` both empty. The merge-base form was **not** used — per R-6 it cannot pass on this branch, because three commits predating this fix already touch `.github/workflows/`. |

**The two rules the spec warns are easy to get subtly wrong were re-read as a
fresh reader, not just pattern-matched.**

- *AC2's framing.* The failure mode the spec names is a paragraph that reads as
    a standalone "always run the neutered pass with `--no-fail-fast`" rule. The
    text is bounded at both ends against that reading: it opens by naming itself
    a consequence, and its final sentence gives the opposite instruction for the
    single-package case. A reader who skims only the bold lead-in gets
    "consequence", not "always". **No fix needed.**

- *AC4's scope sentence.* The failure mode is an unscoped reading that
    discourages cheap local testing. The limiting sentence is the first bullet
    and bolded, so a skimming reader cannot reach the three restrictive bullets
    before meeting it, and it states the positive case affirmatively ("runs are
    cheap and catch things early") rather than merely exempting local work.
    **No fix needed.**

Phase 1's row-2 verdict was **not** re-litigated in this audit, per the Phase 3
handoff. `docs/testing-strategy.md:301-307` is recorded as decided in Phase 3.

### Closing summary

**Rulings and reasoning.**

| Ruling | Outcome                                                                                   | Reasoning that decided it                                                                                                                                                             |
|--------|-------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| R-1    | **Keep** `--no-fail-fast` in `_test_workspace`, unconditionally; rewrite the comment      | Decided after the reversibility spike, which changed the reasoning: reversibility is *scope-dependent*. The one recipe whose callers cannot re-specify the flag is the one that bakes it in; area recipes keep nextest's default and full control. |
| R-2    | R1's policy lives in `SKILL.md`, "Canonical Just Recipes"                                 | It is a policy about when to pay for completeness, not a nextest feature. Keeps the rule in the always-loaded entry point; also keeps AC6 to one file.                                  |
| R-3    | New H2 `## CI/CD Test-scope Discipline` immediately before "Evidence Reuse"; name `just complete` in the R3 prohibition | Reads in logical order (upstream before downstream) and keeps R2's CI/CD scope from being read as a qualifier on the reuse bullets. A prohibition that did not name the live recipe would leave the button documented and unguarded. |
| R-4    | **No** pointer to R2 from `rust-devops`                                                   | One home per rule. Confirmed empirically in Phase 3: zero near-duplicates of R2's distinctive phrases anywhere in `.claude/skills/` or `docs/`.                                        |
| R-5    | `memory/just.md` is in scope but is **not** drift — nothing to fix                        | `:52` makes no fail-fast claim; `:208` correctly describes `just`'s own option parsing, not what a recipe does with the arguments. Left untouched under Rule 3.                        |
| R-6    | AC7 verified against this fix's own changes, not the merge base                           | The merge-base form cannot return empty on this branch: three commits predating this fix already touch `.github/workflows/`. Only the measurement was wrong; AC7's intent is unchanged. |

**Drift found and how it was resolved.** Twenty-two candidates were swept
(17 in Phase 1, plus 5 that its path filters hid, surfaced in Phase 3). Three
were real drift; all three are fixed, and each was resolved the way CLAUDE.md's
drift rule requires — treating the code as correct and the prose as wrong:

1. `just/devops.just:1204-1205` — the comment claimed the flag "preserves the
   old workspace loop's promise", naming a loop that no longer exists and
   saying nothing about selectors. Replaced with reasoning that survives the
   selector case. **Fixed in Phase 2.**
2. `CLAUDE.md:138` — passive-voice "when a feature/fix is completed it is moved
   to `_completed`", with no owner. Replaced with active voice naming the
   author, the review-cycle condition, and the agent-side prohibition.
   **Fixed in Phase 2.**
3. `docs/testing-strategy.md:301-307` — said the recipe "hands **all of them**"
   to nextest, contradicted by the code block five lines above it in the same
   section (`just test biscuit-file # one package`). An intra-section
   contradiction, not a wording preference, so Rule 3 did not protect it. One
   clause plus a by-name pointer; the paragraph was not restructured.
   **Fixed in Phase 3**, overriding Phase 1's not-drift verdict on the text.

A fourth piece of drift was repaired incidentally rather than swept: the
committed `hash:` on the edited skill had been stale since `d14beb34a`, which
is why AC6 is satisfied by computing and writing rather than by comparison.

**Final hash.** `.claude/skills/rust-testing/SKILL.md` →
`61d07be7e22c9f45-6f2ea54170be340f`, a verified fixed point.

**Final change set — six files, exactly the plan's Summary table plus its own
artifacts.** `git status --porcelain` over `CLAUDE.md AGENTS.md just/ .claude/
docs/ fixes/ .github/` returns:

```text
 M .claude/skills/rust-testing/SKILL.md
 M CLAUDE.md
 M docs/testing-strategy.md
 M fixes/2026-09-14-cicd-improvements/implementation-log.md
 M fixes/2026-09-14-cicd-improvements/plan.md
 M just/devops.just
```

No workflow file. No new capability. `AGENTS.md` is a symlink to `CLAUDE.md`
and was not edited separately. Nothing was committed or staged.

**Terminal state: implementation complete, ready for review.** The fix
directory remains at `fixes/2026-09-14-cicd-improvements/`. It was not moved
into `fixes/_completed/` and `just complete` was not run — R3 is the rule this
fix adds, and this plan is the first thing that must obey it.

### Recommendation for the author (not actioned — out of scope)

The Phase 1 spike found that the skill's committed `hash:` had been stale since
`d14beb34a` and that nothing detected it. `md hash --diff` already provides the
exit-code contract a guard would need (0 clean, 2 drifted), and
`tools/test-toolkit/` already hosts passive corpus tests over shipped
artifacts, so a test asserting that every `.claude/skills/**/*.md` carrying a
`hash:` field is a fixed point would be cheap and would have caught this.

It was **not** added. The plan's Summary is explicit that the deliverable is
four files and that no test is written because nothing executable changes;
adding a repository-wide corpus test would expand scope past both the plan and
the spec, which is the author's call and not an agent's. Raised here as a
candidate for `fixes/_unscheduled/`. This does not block review of this fix.

### Deviations and corrections

- **One plan step was superseded by a stronger check, not skipped.** The plan's
    step 5 is "re-run `md hash` and confirm unchanged". That was run. It was
    also supplemented with `md hash --diff`, which compares the stored field
    against a fresh computation rather than comparing the tool against itself —
    the difference matters, because step 5 alone would have passed on the
    drifted HEAD file too.

- No other deviation. R-1 through R-6 were executed exactly as recorded, and
    no earlier phase's verdict was reopened.
