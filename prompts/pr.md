---
name: Pull Request
description: |-
    Pushes the current branch and opens a pull request, keeping the caller informed at every checkpoint.

    1. **pr.md** (this prompt) pushes the branch. The repository's pre-push hook runs the local test gates on this host first; a failure blocks the push and the agent diagnoses what failed, why, and whether this branch caused it.
    2. **_pr/open.md** opens the pull request once local testing has passed, which hands the work to CI/CD.
    3. **_pr/triage.md** runs instead when local testing failed: an interactive session presents the diagnosis and asks whether to fix it, proxying to **fix.md** with an `issue` when the answer is yes.

    Stages share state through the `report` file; see `_pr/_report.md` for its contract.
$schema:
    base: string -> the branch the pull request targets
    title: string -> an optional pull request title; derived from the commits when omitted
    about: string -> optional context from the caller about what this branch's work is
    report: string -> the report file shared by every stage of the PR flow
base: main
report: "{{ ctx.repo_root + '/.claudine/tmp/pr-report.md' }}"
# the pre-push hook can run silently for well over the default stream-silence window
step_timeout: 90m
timeout: 4h
initialize:
    stack:
        - when: "ctx.branch == base"
          action:
              - message: "🤨  refusing to open a PR from `{{base}}` onto itself in {{ ctx.area_description || ctx.repo }}"
              - error: "the current branch is `{{base}}`; check out a feature branch before running the PR prompt"
        - when: "length(ctx.dirty_files) > 0"
          action:
              - warn: "the working tree has uncommitted changes; only commits are pushed, but the pre-push hook will test the working tree"
        - action:
              - ensure_dir: "{{ dirname(report) }}"
              - ensure_file: "{{ report }}"
              - set_frontmatter: ["{{ report }}", "status", "pending"]
              - set_frontmatter: ["{{ report }}", "base", "{{ base }}"]
              - delete_frontmatter: ["{{ report }}", "failures"]
              - delete_frontmatter: ["{{ report }}", "issue"]
              - delete_frontmatter: ["{{ report }}", "fix_requested"]
              - delete_frontmatter: ["{{ report }}", "pr_url"]
start:
    message: "starting PR in {{ ctx.area_description || ctx.repo }}; using local host ({{ctx.os}}) first."
success:
    stack:
        - when: "frontmatter(report, 'status') == 'passed'"
          action:
              - success: "local testing passed on {{ctx.os}}; `{{ctx.branch}}` is pushed and ready for CI/CD"
              - message: |-
                    ✅  local testing passed on **{{ctx.os}}** for `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }}; the branch is pushed and ready for CI/CD

                    {{ frontmatter(report, 'summary') }}
              - say: "Local testing passed on {{ctx.os}}. The branch is pushed and ready for CI CD."
              - action: proxy
                target: ./_pr/open.md
                with:
                    base: "{{ base }}"
                    report: "{{ report }}"
                    title: "{{ title }}"
                    about: "{{ about }}"
        - when: "frontmatter(report, 'status') == 'local_failed'"
          action:
              - warn: "local testing failed on {{ctx.os}}; the push was blocked"
              - message: |-
                    ❌  local testing failed on **{{ctx.os}}** for `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }}; the push was blocked

                    {{ frontmatter(report, 'summary') }}
              - say: "Local testing failed on {{ctx.os}} and the push was blocked. {{ frontmatter(report, 'summary') }}"
              - effect: sad-trombone
              - action: proxy
                target: ./_pr/triage.md
                with:
                    base: "{{ base }}"
                    report: "{{ report }}"
        - when: "frontmatter(report, 'status') == 'error'"
          action:
              - message: |-
                    💥  local testing on **{{ctx.os}}** could not produce a verdict for `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }}

                    {{ frontmatter(report, 'summary') }}
              - say: "Local testing on {{ctx.os}} could not produce a verdict. {{ frontmatter(report, 'summary') }}"
              - error: "{{ frontmatter(report, 'summary') }}"
        - action:
              - message: "💥  the PR agent finished without recording an outcome in `{{report}}`"
              - say: "The PR agent finished without recording an outcome."
              - error: "the PR agent did not set `status` in {{report}}"
blocked:
    message: "💥  the PR flow for `{{ctx.branch}}` was **blocked** before the agent started"
failure:
    message: "❌  the PR flow for `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }} failed ({{ctx.agent}}/{{ctx.model}}: {{err.msg}})"
    say: "The pull request flow failed in {{ ctx.area_description || ctx.repo }}."
---

# Push a Branch for a Pull Request

## Context

- You are pushing the `{{ctx.branch}}` branch of the **{{ctx.repo}}** repository so a pull request can be opened against `{{base}}`.
- You are running on a **{{ctx.os}}** host, and local testing happens here before anything reaches CI/CD.
- You are running non-interactively: nobody can answer a question during this session. Record what you find in the report file instead.
::block when="about"
- The caller described this branch's work as:

    > {{about}}
::end-block

::file ./_pr/_report.md

## Task

### 1. Preflight

- Run `git fetch origin {{base}}` and list the commits in `origin/{{base}}..HEAD`.
    - With no commits to propose, set `status: error` with a `summary` saying so and stop.
- Note the output of `git status --short`. Uncommitted changes are not pushed, but a dirty checkout makes the pre-push hook test the working tree rather than the commits.
- Review `just ci-local --plan` so you know which package areas and cells the hook will run and which it will reuse from earlier evidence.
- Write `branch`, `head` (full SHA), `base`, and `commits` into the report frontmatter now.

### 2. Local testing: push the branch

Run `git push -u origin HEAD`.

The repository's pre-push hook runs the local test gates for every affected package on this host and blocks the push when any of them fails. **That hook _is_ the local testing step**; do not run the gates separately first, because only the hook publishes the evidence CI reuses. Never bypass it: no `--no-verify`, and do not set `RUSTY_BISCUIT_PRE_PUSH`.

- The hook routinely takes 15–30 minutes. If your shell tool enforces a shorter time limit, run the push in the background with all output captured to `.claudine/tmp/pr-push.log` and poll that file until the process exits.
- Capture the full output either way; you will need it if anything fails.

### 3a. The push succeeded

- Confirm the remote branch now points at `HEAD` (`git rev-parse @{u}` equals `git rev-parse HEAD`).
- Set `status: passed` and a `summary` naming how many commits were pushed and which package areas the hook tested.
- **Do not open the pull request.** The next stage does that once the caller has been told local testing passed.

### 3b. The push was blocked by failing gates

::file ./_pr/_diagnosis.md

When the diagnosis is complete:

- set `status: local_failed`
- fill `failures` with one entry per failing test
- write `summary` as a short, plain-language count of the distinct problems and their attributions (for example "2 problems: a stale fixture introduced by this branch, and a flaky L2 terminal test")
- write `issue` as described above
- put the detailed write-up, grouped by problem, in the report body

### 3c. Something else went wrong

If the push failed for a reason that is not a failing gate (authentication, a rejected non-fast-forward, the hook crashing before any test ran, a full disk), set `status: error` with a `summary` that names the reason. Do not force-push and do not work around a rejection.

### 4. Close out

End with a short summary for the caller: the outcome, the report file's location, and (for a failure) the distinct problems you found.
