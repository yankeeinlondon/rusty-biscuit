---
name: Triage Local Test Failures
description: |-
    An interactive session reached from **pr.md** when local testing blocked a push. Presents the diagnosis to the caller and asks whether to fix it; a "yes" proxies to **fix.md** with the diagnosed `issue`.
$schema:
    base: string(required) -> the branch the pull request targets
    report: file(required) -> the report file shared by every stage of the PR flow
interactive: true
start:
    stderr: "Local testing blocked the push; the agent will walk you through what failed and ask whether to fix it."
success:
    stack:
        - when: "frontmatter(report, 'fix_requested') == true && frontmatter(report, 'issue')"
          action:
              - message: "🔧  handing the local test failures on `{{ctx.branch}}` to the fix prompt"
              - say: "Handing the local test failures to the fix prompt."
              - action: proxy
                target: ../fix.md
                with:
                    issue: "{{ frontmatter(report, 'issue') }}"
        - action:
              - message: "⏸️  the PR for `{{ctx.branch}}` is paused; local test failures were left unfixed (details in `{{report}}`)"
              - say: "The pull request is paused. The local test failures were not fixed."
              - stop
failure:
    message: "❌  triage of the local test failures on `{{ctx.branch}}` failed ({{ctx.agent}}/{{ctx.model}}: {{err.msg}})"
    say: "Triage of the local test failures failed."
---

# Triage Local Test Failures

## Context

- A push of the `{{ctx.branch}}` branch of **{{ctx.repo}}** was blocked because local testing on **{{ctx.os}}** failed.
- An earlier agent diagnosed the failures and recorded the results in the report file. You are now in an **interactive** session with the caller.

::file ./_report.md

## Task

1. Read `{{report}}` in full, frontmatter and body.
2. Present the diagnosis to the caller. Assume they have not seen the logs. For each distinct problem:
    - **what failed**: the gate and the tests it accounts for
    - **why it failed**: the root cause, in plain language
    - **where it came from**: this branch (and which commit), uncommitted changes, a problem already present on `{{base}}`, or the environment, together with the evidence and the confidence
3. Answer the caller's questions. If they point out something the diagnosis missed or got wrong, investigate it and correct the report (`failures`, `summary`, `issue`, and the body) before continuing.
4. Ask the caller directly whether they want these failures **fixed**. They may choose all of the problems, some, or none.
5. Record the answer in the report frontmatter:
    - `fix_requested: true` when they want any problem fixed; then rewrite `issue` so it covers **only** the problems they chose, adding any constraints or preferences they expressed. It must stay self-contained, since the fixing agent will not see this conversation.
    - `fix_requested: false` otherwise.
6. Tell the caller what happens next, then end the session: on a "yes" the fix prompt starts automatically once this session exits; on a "no" the PR flow stops and the branch stays unpushed.

Do not fix anything in this session, and never bypass the pre-push hook.
