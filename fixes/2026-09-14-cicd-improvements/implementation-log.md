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
packages: []
human_review: false
message_to_agent: |-
  Phase 1 was read-only and produced five recorded rulings plus four
  corrections to the plan's own premises. Read these before starting Phase 2.

  1. R-1 = KEEP the flag (recommendation accepted), but the *reason* is
     narrower than the plan states. `--no-fail-fast` is unrecoverable from the
     command line ONLY at the repository root. Area-level `just test` (e.g.
     `darkmatter/justfile:71`) forwards `{{ args }}` straight to nextest, so
     `just test --no-fail-fast` works fine inside a package area — many
     historical logs in the repo do exactly that. The plan's fact (3) is true
     but unscoped; prose that repeats it unscoped would be factually wrong
     about the recipe agents use most. Say "at the repository root".

  2. The plan's fact (6) is WRONG. `.claude/skills/rust-testing/SKILL.md` is
     NOT currently a hash fixed point — its committed `hash:` body half
     (`5e3087a71564a05a`) is stale; the real value is `fb3622c484a86f23`. It
     drifted in commit d14beb34a, which edited the body without refreshing the
     hash. Do not treat "hash unchanged from the committed value" as a Phase 4
     success test; Phase 4 must WRITE the correct value.

  3. The Phase 4 hash ordering is CEREMONIAL, not required. Measured: bumping
     `last_updated` does not move the hash, and writing the `hash:` field does
     not move the hash. `md hash` excludes both fields. Only body edits and
     other frontmatter fields (e.g. `name`) move it. The one real constraint
     is "hash after the last body edit".

  4. AC7's verification command CANNOT PASS as written. Three pre-existing
     commits on `fix/cicd-improvements` (0a750f407, 1fd610a7b, 2ed6b0f04)
     already touch `.github/workflows/`, so `git diff --name-only
     <merge-base>...HEAD` lists 5 workflow files before this fix changes
     anything. AC7's intent — this fix touches no workflow file — must be
     checked against this fix's own changes, not the whole branch. Use the
     corrected command recorded under R-6 in the plan.

  Also note: R-2's cited line number has drifted. "Canonical Just Recipes" is
  `SKILL.md:312`, not 297 (line 297 is inside the `expect_level!` gating
  section). And for R3, `just complete` (`just/lifecycle.just:108`) is a real
  recipe that performs the `_completed/` move — the CLAUDE.md prohibition
  should name it, or an agent will read the prohibition and still press the
  button that the `just` skill documents.
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
