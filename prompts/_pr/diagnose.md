---
name: Diagnose a Blocked Push
description: |-
    Reached from `_pr/push.md` by proxy when the pre-push hook blocked the push. Explains every failure (what, why, and where it came from), decides which branch each repair belongs on, and writes the briefing a fixing agent will work from. It repairs nothing.

    `on_failure` chooses what follows: `ask` hands the diagnosis to the interactive `_pr/triage.md`, `fix` goes straight to `_pr/fix.md`, and `stop` ends the flow with the diagnosis.
$schema:
    base: string(required) -> the branch the pull request targets
    report: file(required) -> the report file shared by every stage of the PR flow
    on_failure: enum(ask, fix, stop) -> what happens once the diagnosis is complete
    committed: boolean -> whether `commit.md` already ran in this invocation (a proxy chain can visit it only once)
on_failure: ask
committed: false
branch: "{{ frontmatter(report, 'branch') }}"
log: "{{ frontmatter(report, 'log') }}"
# reproducing on the base and bisecting both rebuild, and a build can be silent for a long time
step_timeout: 60m
timeout: 3h
start:
    message: "🔎  diagnosing the local test failures that blocked the push of `{{branch}}` in {{ ctx.area_description || ctx.repo }}"
success:
    stack:
        # Same single nudge as the push stage: the agent's session still holds the
        # investigation, so ask once before treating a missing verdict as an error.
        - when: "frontmatter(report, 'status') != 'diagnosed' && !frontmatter(report, 'diagnosis_nudged')"
          action:
              - set_frontmatter: ["{{ report }}", "diagnosis_nudged", "{{ true }}"]
              - warn: "the diagnosing agent finished without completing the report; asking it to finish"
              - resume: "You ended the session before the diagnosis was recorded. In the frontmatter of {{ report }}, fill in `failures`, `summary`, and `issue` as the report contract describes, put the detailed write-up in the body, set `status: diagnosed`, and then stop."
        - when: "frontmatter(report, 'status') != 'diagnosed'"
          action:
              - message: "💥  the local test failures on `{{branch}}` could not be diagnosed; the push log is at `{{log}}`"
              - say: "The local test failures could not be diagnosed."
              - error: "the diagnosing agent did not set `status: diagnosed` in {{report}}"
        - action:
              - info: "diagnosis: {{ frontmatter(report, 'summary') }}"
              - message: |-
                    🔎  diagnosis of the blocked push of `{{branch}}` in {{ ctx.area_description || ctx.repo }}:

                    {{ frontmatter(report, 'summary') }}
              - say: "The blocked push has been diagnosed. {{ frontmatter(report, 'summary') }}"
        - when: "on_failure == 'stop'"
          action:
              - info: "the PR for `{{branch}}` is paused with its diagnosis in {{report}}"
              - message: "⏸️  the PR for `{{branch}}` is paused with its diagnosis in `{{report}}`"
              - stop
        - when: "on_failure == 'fix'"
          action:
              - action: proxy
                target: ./fix.md
                with:
                    base: "{{ base }}"
                    report: "{{ report }}"
                    committed: "{{ committed }}"
        - action:
              - action: proxy
                target: ./triage.md
                with:
                    base: "{{ base }}"
                    report: "{{ report }}"
                    committed: "{{ committed }}"
failure:
    stack:
        - when: "err.category == 'timeout'"
          action:
              - warn: "the diagnosing agent was stopped by a timeout ({{err.code}}); resuming its session"
              - resume: "You were stopped by a timeout. Continue the diagnosis from where you left off. If a step cannot finish in the time left, record what you established, lower `confidence` accordingly, and complete the report."
        - action:
              - message: "❌  diagnosing the blocked push of `{{branch}}` failed ({{ctx.agent}}/{{ctx.model}}: {{err.msg}}); the push log is at `{{log}}`"
              - say: "Diagnosing the blocked push failed."
---

# Diagnose a Blocked Push

## Context

- A push of the `{{branch}}` branch of **{{ctx.repo}}** was blocked because the pre-push hook's local test gates failed on this **{{ctx.os}}** host.
- The complete output of that push is in `{{log}}`. Start there; do not push again to regenerate it.
- You are running non-interactively: nobody can answer a question during this session. Record what you find in the report file instead.
    - If a tool call or a file read is refused, say so in your closing summary and find another way to the same information.

::file ./_report.md

::file ./_diagnosis.md

## Task

1. Read `{{report}}` and `{{log}}`.
2. Diagnose every failure as described above.
3. Record the result in the report:
    - fill `failures` with one entry per failing test or gate, each with its `fix_branch` where a repair is needed
    - write `summary` as a short, plain-language count of the distinct problems and their attributions (for example "2 problems: a stale fixture introduced by this branch, and a flaky L2 terminal test")
    - write `issue` as described above
    - put the detailed write-up, grouped by problem, in the report body
    - set `status: diagnosed` last, once everything else is in place
4. End with a short summary for the caller: the distinct problems, where each came from, and the report file's location.
