---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/fixes/2026-10-16-unified-logical-or/spec.md"
plan: "darkmatter/fixes/2026-10-16-unified-logical-or/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
decisions_for_review:
    - |-
        The plan's Phase 1 baseline task expects Claudine's prompt corpus
        (`just test-prompts` in claudine/) to be green before any code change.
        It is not: 6 of 87 tests fail on the unmodified tree, and none of the
        failures involve `||`. Three `review_fix_rehearsal` tests expect a
        "follow-up pass" that the live review prompts no longer run, and three
        `shipped_prompt_route_drift` tests report that the live
        `prompts/_implement/` has moved ahead of its frozen fixture (new
        `depends_on` checks, a swapped `phase`/`start_phase` fallback order,
        and two new files). I recorded these six failures by name as the
        baseline and did not change the prompts or the fixture. Fixing them is
        prompt-maintenance work outside this fix, and the repository treats
        `prompts/` as draft space that only opt-in tests read. The other
        options were to refresh the fixture from the live prompts, which would
        hide drift the test exists to report, or to fix the rehearsal tests,
        which needs a decision on how the review prompt should behave. If the
        reviewer wants the corpus green, that is a separate change; nothing in
        this fix depends on it. Later phases should treat only failures beyond
        these six as caused by this change.
message_to_agent: |-
    Phase 1 changed no code. Baseline for AC3 (prompt corpora), taken before any edit:
    - `cargo nextest run -p darkmatter --features prompt-tests --test prompts`: 10/10 pass.
    - `just test-prompts --no-fail-fast` in claudine/: 81/87 pass. These 6 fail on the
      unmodified tree and are NOT caused by this fix:
      review_fix_rehearsal::{a_rerun_on_a_ready_spec_runs_its_open_follow_up_pass,
      a_ready_review_gets_one_follow_up_pass_and_no_further_review,
      a_not_ready_review_is_repaired_and_the_ready_one_gets_its_follow_up_pass} and
      shipped_prompt_route_drift::{fixture_preserves_the_shipped_schema_and_loop_semantics,
      shipped_implement_prompts_have_not_drifted_from_their_fixture,
      fixture_body_matches_the_shipped_body}.
      Treat any failure beyond these six as a regression from this change. Note that
      `just test-prompts` stops at the first failure unless you pass `--no-fail-fast`.
    The R14 safety search found 0 conditions that use a `||` result as a value, so no
    condition was rewritten. Re-run the search right before landing (Phase 7).
---

# Implementation Log for 2026-10-16-unified-logical-or (7 phases)

## Phase 1

- Started 2026-10-30. Phase 1 is read-only: a safety search and a green
  baseline. No source, doc, or skill file was changed.

### Safety search re-run (R14)

- Command (repository root, tracked files):
  `git grep -nE '(when|while|until)[=:"'"'"' ]+.*\|\|' -- '*.md' '*.yaml' '*.yml' '*.rs' ':!**/_completed/**' ':!**/*frozen*/**' ':!darkmatter/fixes/2026-10-16-unified-logical-or/**'`
  gave 141 lines. Keeping only lines with a `when=`/`when:`/`while:`/`until:`
  (and `=`) condition left **99 condition lines** (the planning-time run found
  82; the extra lines are new prompt and test-fixture copies, plus doc
  prose).
- Pattern check for a value-consumed `||` (a `||` inside a function call's
  argument list, or a parenthesized `||` group used as a comparison operand):
  **0 hits**.
- A manual read of all 99 lines (grouped into 58 distinct conditions)
  confirmed that every `||` is a truth test. It is either the outermost
  operator, or it sits inside `&&`/`!`/a parenthesized group whose result is
  only tested for truthiness. Edge case checked:
  `env.CLAUDINE_TEST_MISSING || 'default' == 'default'` (a claudine test)
  parses as `env.X || ('default' == 'default')` because `==` binds tighter,
  so it is still a truth test.
- R14 outcome: **0 found, no rewrite needed.** Some doc and comment lines
  still describe the two-mode split (for example the
  `darkmatter/docs/topics/darkmatter-expressions.md` "Logical OR: returns a
  boolean" row, and the "either dialect" prose). Phase 6 rewrites them.

### Green baseline capture

- `cargo nextest run -p darkmatter --features prompt-tests --test prompts`:
  **10/10 passed** (green).
- `just test-prompts` in `claudine/` (the `claudine-cli` `prompts` binary with
  `prompt-tests,test-fixtures`): the default run stopped after 3 failures, so
  I re-ran it with `--no-fail-fast` to get a complete baseline:
  **87 run, 81 passed, 6 failed.** All 6 fail on the unmodified tree; there
  are no working-tree changes under `prompts/` or `claudine/`:
    - `review_fix_rehearsal::a_rerun_on_a_ready_spec_runs_its_open_follow_up_pass`
    - `review_fix_rehearsal::a_ready_review_gets_one_follow_up_pass_and_no_further_review`
    - `review_fix_rehearsal::a_not_ready_review_is_repaired_and_the_ready_one_gets_its_follow_up_pass`
      — these expect a `repair` follow-up pass on a production-ready review
      that the live review prompts do not run (stub log is `[]`, expected
      `["repair"]`).
    - `shipped_prompt_route_drift::fixture_preserves_the_shipped_schema_and_loop_semantics`
    - `shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture`
    - `shipped_prompt_route_drift::fixture_body_matches_the_shipped_body`
      — the live `prompts/_implement/` has drifted from
      `claudine/cli/tests/fixtures/shipped_implement_route/_implement/`: new
      `depends_on` checks, a swapped `phase`/`start_phase` fallback order, and
      new `_fm.md`/`implement-feature.md` files. None of the new conditions
      use `||`.
- Neither group is about `||`; both are prompt-maintenance drift. They are
  recorded as the baseline (see `decisions_for_review`) and left unchanged.

### Requirement-to-test mapping

- Phase 1 changes no behavior, so it adds no tests. The two baseline runs
  above are the evidence for AC3's corpus starting state.

### Gates

- Scope calculator, run from the repository root after every Phase 1 edit:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gave `packages: [darkmatter, darkmatter-cli, dmls, simplified-schema, test-toolkit]`
  and `change_class: package`. The list is not empty because the working tree
  still holds uncommitted source edits from the earlier
  `2026-10-30-reference-navigation-and-hover` work. Phase 1 itself edited only
  this log and the plan. The gates were run regardless:
    - `just test` (darkmatter): **12874 run, 12874 passed, 10 skipped** (69 s).
    - `just lint` (darkmatter): **pass**, including the zed-dmls
      wasm32-wasip2 check.
- No cross-OS check was needed: Phase 1 changed no code.
- No darkmatter skill update was needed: Phase 1 changed no behavior.
