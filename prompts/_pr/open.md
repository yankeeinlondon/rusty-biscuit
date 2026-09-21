---
name: Open Pull Request
description: |-
    Opens (or finds) the pull request for a branch that has already passed local testing and been pushed. Reached from `_pr/push.md` by proxy; opening the pull request is what starts CI/CD.
$schema:
    base: string(required) -> the branch the pull request targets
    branch: string(required) -> the branch being proposed, as the push stage recorded it
    report: file(required) -> the report file shared by every stage of the PR flow
    title: string -> an optional pull request title; derived from the commits when omitted
    about: string -> optional context from the caller about what this branch's work is
body_file: "{{ replace_last(report, '.md', '.pr-body.md') }}"
timeout: 20m
success:
    stack:
        - when: "frontmatter(report, 'status') == 'pr_opened' && frontmatter(report, 'pr_url')"
          action:
              - success: "pull request ready: {{ frontmatter(report, 'pr_url') }}"
              - message: "🚀  pull request for `{{branch}}` is open and CI/CD is running: {{ frontmatter(report, 'pr_url') }}"
              - say: "The pull request is open and CI CD is running."
        - action:
              - message: |-
                    💥  could not open a pull request for `{{branch}}` in {{ ctx.area_description || ctx.repo }}; the branch is pushed, so `gh pr create` can still be run by hand

                    {{ frontmatter(report, 'summary') }}
              - say: "The branch passed local testing, but the pull request could not be opened."
              - error: "no pull request URL was recorded in {{report}}"
failure:
    stack:
        - when: "err.is_transient || err.is_throttled"
          action:
              - warn: "opening the pull request failed with a recoverable error ({{err.code}}); retrying once"
              - retry: 1
        - action:
              - message: "❌  opening the pull request for `{{branch}}` failed ({{ctx.agent}}/{{ctx.model}}: {{err.msg}}); the branch is pushed, so `gh pr create` can still be run by hand"
              - say: "Opening the pull request failed."
---

# Open the Pull Request

## Context

- The `{{branch}}` branch of **{{ctx.repo}}** has passed local testing on **{{ctx.os}}** and is pushed to `origin`.
- Your job is to open its pull request against `{{base}}`. Opening it is what starts CI/CD.
- You are running non-interactively: nobody can answer a question during this session.
::block when="about"
- The caller described this branch's work as:

    > {{about}}
::end-block
- The commits being proposed, as the push stage recorded them:

    {{ as_unordered_list(frontmatter(report, 'commits')) }}

::file ./_report.md

## Task

1. Confirm `git rev-parse origin/{{branch}}` equals the `head` recorded in the report. If it does not, set `status: error` with a `summary` explaining the mismatch and stop; do not push from this stage.
2. Check for an existing pull request with `gh pr list --head {{branch}} --state open --json url`.
    - If one exists, the push already re-triggered its CI. Record its URL as `pr_url`, set `status: pr_opened`, and skip to step 5.
3. Write the pull request description to `{{body_file}}` (never inline it on the command line; descriptions contain backticks and `$`). It should:
    - open with a short paragraph on what the branch does and why
    - summarize the changes grouped by theme, not commit by commit
    - point to any `spec.md` files under `features/` or `fixes/` that the commits implement or close
    - list every commit above, so a reviewer can see work that is unrelated to the main theme
4. Open it with `gh pr create --base {{base}} --head {{branch}} --title <title> --body-file {{body_file}}`.
::block when="title"
    - Use this title: **{{title}}**
::end-block
::block when="!title"
    - Derive a Conventional Commits title from the commits (e.g. `fix(darkmatter): unify array rendering to compact JSON`).
::end-block
5. Set `pr_url` and `status: pr_opened` in the report, and replace `summary` with one sentence describing the pull request.
6. End with the pull request URL.
