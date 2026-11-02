---
name: Fix Slow Tests
description: |-
    Fixes every test recorded as slow in the **current state of the working tree**. A slow test is a failing test.

    Every canonical test recipe (`just test`, `just sanity`, `just test-l2`, …) records its run under the working tree's state: `HEAD` plus the content of every changed file (`sniff repo state`). `target/slow-tests/current.md` gathers every run recorded in that state, one section per tier and directory, so `just test` followed by `just sanity` in an unchanged tree reports both tiers' slow tests together. This prompt reads that summary:

    - no recorded run: the prompt stops and says to run tests first
    - no slow tests: the prompt reports that and launches no agent
    - a summary of an earlier state (the tree changed after the tests ran): the prompt stops, says which files and commits changed and whether they touch the directories the tests covered, and gives the commands that re-run them; `force=true` proceeds with the earlier summary instead
    - otherwise an agent fixes each slow test's cause and re-runs every recorded scope

    The `success` hook does not trust the agent's word. It runs `just slow-tests verify`, which passes only when the working tree has changed since the baseline, every baseline scope (tier, directory, packages) was re-run in the tree's current state, and none of those runs has a slow test; the agent's report must also say `fixed`. It asks once more when that proof is missing, then fails.
# The agent must build, time, and re-run tests and write its report; without
# the provider's bypass a non-interactive session is denied all of it.
yolo: true
$schema:
    report: string -> the outcome file this run's hooks read; created fresh for each run
    force: boolean -> fix the slow tests of the last recorded run even when the working tree changed after it; the proof still re-runs every scope in the current tree
summary: "{{ ctx.repo_root + '/target/slow-tests/current.md' }}"
report: "{{ ctx.repo_root + '/.claudine/tmp/slow/' + ctx.timestamp + '.md' }}"
has_summary: "{{ file_exists(summary) }}"
baseline_state: "{{ has_summary ? frontmatter(summary, 'state') : 'none' }}"
initialize:
    - when: "!has_summary"
      then:
          - warn: "no recorded test run in this checkout ({{ summary }} is missing); run a test recipe such as `just test` in a package area, then run this prompt again"
          - error: "there is no recorded test run to read slow tests from"
    - when: "frontmatter(summary, 'slow') == 0"
      then:
          - success: "🐢 no slow tests in the {{ frontmatter(summary, 'tiers') }} runs of this working tree ({{ frontmatter(summary, 'tests') }} tests); nothing to fix"
          - message: "🐢 no slow tests in the recorded runs of {{ ctx.area_description || ctx.repo }}; nothing to fix"
          - skip
    - ensure_dir: "{{ dirname(report) }}"
    - ensure_file: "{{ report }}"
    - set_frontmatter: ["{{ report }}", "status", "pending"]
    - set_frontmatter: ["{{ report }}", "baseline_state", "{{ baseline_state }}"]
    - set_frontmatter: ["{{ report }}", "tiers", "{{ frontmatter(summary, 'tiers') }}"]
start:
    # A `resume` re-enters at `start`, after the agent has changed the tree on
    # purpose; the freshness gate applies only to the first entry.
    - when: "!frontmatter(report, 'nudged')"
      then:
          - set: { freshness: "$(just slow-tests fresh)::result::timeout:900" }
          - when: "!freshness.ok && force"
            then:
                - warn: "{{ freshness.stdout }}"
                - warn: "force=true: fixing the slow tests of that earlier run anyway"
          - when: "!freshness.ok && !force"
            then:
                - warn: "{{ freshness.stdout }}"
                - error: "the recorded test run is older than the working tree; re-run the tests, or run this prompt with force=true to fix the slow tests it recorded"
    - info: "🐌 fixing {{ frontmatter(summary, 'slow') }} slow test(s) from the {{ frontmatter(summary, 'tiers') }} runs of this working tree; report: {{ report }}"
    - message: "🐌 fixing {{ frontmatter(summary, 'slow') }} slow test(s) in {{ ctx.area_description || ctx.repo }} ({{ ctx.agent }}/{{ ctx.model }})"
    - say: "Fixing slow tests."
success:
    - set: { proof: "$(just slow-tests verify --baseline {{ baseline_state }})::result::timeout:900" }
    - when: "proof.ok && frontmatter(report, 'status') == 'fixed'"
      then:
          - success: "✅ no slow tests remain: every scope of the baseline re-ran in the current working tree with none at or above the threshold; details in {{ report }}"
          - message: "✅ slow tests fixed in {{ ctx.area_description || ctx.repo }}: every recorded scope re-ran with none left"
          - say: "The slow tests are fixed."
          # A `when` branch does not end its event; without this the checks
          # below run too and nudge an agent that already proved its work.
          - stop
    - when: "frontmatter(report, 'status') == 'blocked'"
      then:
          - warn: "⛔ the agent could not fix every slow test; see {{ report }}"
          - message: "⛔ slow tests remain in {{ ctx.area_description || ctx.repo }}: the agent reported it was blocked; see {{ report }}"
          - say: "Some slow tests could not be fixed."
          - error: "slow tests remain unfixed (status: blocked)"
    - when: "!frontmatter(report, 'nudged')"
      then:
          - set_frontmatter: ["{{ report }}", "nudged", true]
          - warn: "the agent ended without proof that no slow tests remain; asking it once more"
          - resume: |-
                You ended without the proof this task requires. `just slow-tests verify --baseline {{ baseline_state }}` must pass, and {{ report }} must say `status: fixed`. The report's status is `{{ frontmatter(report, 'status') }}`, and the check says:

                {{ proof.stdout }}

                Finish fixing the remaining slow tests, then re-run every scope it names, after your last edit. If you cannot, set `status: blocked` and explain why in the report.
    - warn: "❌ no proof that the slow tests were fixed: {{ proof.stdout }}"
    - message: "❌ fixing slow tests in {{ ctx.area_description || ctx.repo }} ended without proof that none remain"
    - say: "The slow tests were not proven fixed."
    - error: "the agent did not prove that no slow tests remain"
failure:
    stderr: "❌ fixing slow tests failed ({{ ctx.agent }}/{{ ctx.model }}): {{ err.msg }}"
    message: "❌ fixing slow tests in {{ ctx.area_description || ctx.repo }} failed: {{ err.msg }}"
    say: "Fixing the slow tests failed."
blocked:
    stderr: "💥 fixing slow tests was blocked before the agent started: {{ err.msg }}"
    message: "💥 fixing slow tests in {{ ctx.area_description || ctx.repo }} was blocked before it started: {{ err.msg }}"
    say: "Fixing the slow tests was blocked."
---

# Fix Slow Tests

**Slow tests are failing tests.** Every test listed below must be fixed. A run with a slow test in it is a failed run, exactly as if the test's assertion had failed, and this task is finished only when every scope below has been re-run in your changed working tree with no slow tests at all.

## The recorded runs

::block when="has_summary"

These facts were read from `{{ summary }}` when this prompt was composed; you do not need to re-derive them. It holds every test run recorded while the working tree was in state `{{ baseline_state }}` (`sniff repo state`: `HEAD` plus the content of every changed file), one section per tier and directory, each with the command that re-runs it.

- **Tiers:** {{ frontmatter(summary, 'tiers') }}, in {{ frontmatter(summary, 'scopes') }} scope(s)
- **Tests run:** {{ frontmatter(summary, 'tests') }}, of which **{{ frontmatter(summary, 'slow') }}** took {{ frontmatter(summary, 'threshold_s') }}s or longer
- **Last recorded:** {{ frontmatter(summary, 'recorded') }} on **{{ ctx.os }}**, on the `{{ ctx.branch }}` branch of **{{ ctx.repo }}**

::file {{ summary }}

::end-block

The durations come from full parallel runs. A test that is fast alone but slow under load is still slow: CI runners have 2–4 cores and run a whole package at once.

## What counts as a fix

Find and remove the **cause** of the time. Often the cause is in the code under test rather than the test, and fixing it there speeds up production too.

None of these is a fix, and each will be treated as leaving the test broken:

- renaming a test, or a module around it, with `slow_` or any other tier marker so the run stops selecting it
- raising `slow-timeout` or adding a per-test override in `.config/nextest.toml`
- `#[ignore]`, deleting the test, or weakening what it asserts
- splitting the work so each piece hides under the threshold while the total is unchanged and still serialized
- turning off recording (`BISCUIT_SLOW_TESTS=off`) or editing anything under `target/slow-tests/` or the slow-test log by hand

Shortening a deadline is a fix only through a test seam that keeps the production value, as the anti-patterns below describe.

## How to work

1. Load the `rust-testing` skill (its `test-speed.md` has worked examples) and the `nextest` skill.
2. For each slow test, time it alone first, for example `cargo nextest run -p <package> -E 'test(=<test path>)'`, to tell intrinsic cost from contention. Then find where the time goes before changing anything.
3. Fix the cause. Check the change against the anti-patterns below, and keep the test's coverage at least as strong as it was.
4. When every test is fixed, run **every scope's re-run command** listed above, from the repository root, **after your last edit**. Each run is recorded under the working tree's state at that moment, so editing any file afterwards (a fix, a comment, a doc) starts a new state and the scopes must run again. A narrower run (one test, a `-E` filter) does not count.
5. Check your proof with `just slow-tests verify --baseline {{ baseline_state }}`. It lists every scope that still has to run and every slow test that remains. Repeat until it says `proven`.

::file ./_testing-anti-patterns.md

::file ./_test-scope.md

::file ./_os.md

## Out of bounds

- Do not commit or push. Leave your changes in the working tree; commits in this repository go through `prompts/commit.md`.
- Do not change the slow threshold, the recording recipes, or `scripts/slow-tests.rs` to make this task pass.

## Your report

Write your outcome to `{{ report }}`. It already exists with frontmatter that the lifecycle hooks read; keep the existing keys and set `status`:

- `status: fixed` once `just slow-tests verify --baseline {{ baseline_state }}` says `proven`
- `status: blocked` when you could not fix every slow test

Below the frontmatter, write one section per slow test: its cause, what you changed, and its duration before and after. For a blocked test, say what you tried and what stopped you.

::file ./_finish-before-exit.md
