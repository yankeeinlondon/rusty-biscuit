---
name: Open Pull Request
description: |-
    Opens (or finds) the pull request for a branch that has already passed local testing and been pushed. Reached from **pr.md** by proxy; opening the pull request is what starts CI/CD.
$schema:
    base: string(required) -> the branch the pull request targets
    report: file(required) -> the report file shared by every stage of the PR flow
    title: string -> an optional pull request title; derived from the commits when omitted
    about: string -> optional context from the caller about what this branch's work is
success:
    stack:
        - when: "frontmatter(report, 'status') == 'pr_opened'"
          action:
              - success: "pull request ready: {{ frontmatter(report, 'pr_url') }}"
              - message: "🚀  pull request for `{{ctx.branch}}` is open and CI/CD is running: {{ frontmatter(report, 'pr_url') }}"
              - say: "The pull request is open and CI CD is running."
        - action:
              - message: |-
                    💥  could not open a pull request for `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }}

                    {{ frontmatter(report, 'summary') }}
              - say: "The branch passed local testing, but the pull request could not be opened."
              - error: "no pull request was recorded in {{report}}"
failure:
    message: "❌  opening the pull request for `{{ctx.branch}}` failed ({{ctx.agent}}/{{ctx.model}}: {{err.msg}})"
    say: "Opening the pull request failed."
---

# Open the Pull Request

## Context

- The `{{ctx.branch}}` branch of **{{ctx.repo}}** has passed local testing on **{{ctx.os}}** and is pushed to `origin`.
- Your job is to open its pull request against `{{base}}`. Opening it is what starts CI/CD.
- You are running non-interactively: nobody can answer a question during this session.
::block when="about"
- The caller described this branch's work as:

    > {{about}}
::end-block

::file ./_report.md

## Task

1. Confirm `git rev-parse @{u}` equals `git rev-parse HEAD`. If it does not, set `status: error` with a `summary` explaining the mismatch and stop; do not push from this stage.
2. Check for an existing pull request with `gh pr list --head {{ctx.branch}} --state open --json url`.
    - If one exists, the push already re-triggered its CI. Record its URL as `pr_url`, set `status: pr_opened`, and skip to step 5.
3. Write the pull request description to a file under `.claudine/tmp/` (never inline it on the command line; descriptions contain backticks and `$`). It should:
    - open with a short paragraph on what the branch does and why
    - summarize the changes grouped by theme, not commit by commit
    - point to any `spec.md` files under `features/` or `fixes/` that the commits implement or close
    - list every commit in `commits` from the report, so a reviewer can see work that is unrelated to the main theme
4. Open it with `gh pr create --base {{base}} --head {{ctx.branch}} --title <title> --body-file <file>`.
::block when="title"
    - Use this title: **{{title}}**
::end-block
::block when="!title"
    - Derive a Conventional Commits title from the commits (e.g. `fix(darkmatter): unify array rendering to compact JSON`).
::end-block
5. Set `pr_url` and `status: pr_opened` in the report, and replace `summary` with one sentence describing the pull request.
6. End with the pull request URL.
