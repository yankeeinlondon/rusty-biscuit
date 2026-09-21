---
name: Fix Local Test Failures
description: |-
    Repairs the problems a blocked push was diagnosed with, proves the repair by re-running the tests that failed, and **stages** the result. Reached by proxy from `_pr/triage.md` (the caller said yes) or from `_pr/diagnose.md` (`on_failure=fix`).

    An attempt that does not verify is retried with a fresh agent, which reads what earlier attempts recorded in the report. This uses the lifecycle `retry` action rather than a `loop:` block, because a `proxy` fired from inside a looping document is not performed, and this document has to hand off to `commit.md`.

    It never commits. Every commit goes through `commit.md`, so a verified fix is handed to it by proxy. A proxy chain can visit a document only once, which has two consequences: when `commit.md` already ran in this invocation the fix is left staged for the caller to commit, and the flow cannot return to `_pr/push.md`, so the caller runs the PR prompt again to push the fix.
$schema:
    base: string(required) -> the branch the pull request targets
    report: file(required) -> the report file shared by every stage of the PR flow
    committed: boolean -> whether `commit.md` already ran in this invocation (a proxy chain can visit it only once)
committed: false
branch: "{{ frontmatter(report, 'branch') }}"
step_timeout: 60m
timeout: 3h
max_attempts: 3
start:
    stack:
        # `start` fires again on every retry, so this counts attempts in the one
        # place that outlives them.
        - action:
              - increment_frontmatter: ["{{ report }}", "fix_attempts"]
              - message: "🔧  fixing the local test failures on `{{branch}}` in {{ ctx.area_description || ctx.repo }} (attempt {{ frontmatter(report, 'fix_attempts') }} of {{ max_attempts }})"
success:
    stack:
        - when: "frontmatter(report, 'status') == 'fixed' && committed"
          action:
              - warn: "the fix is staged but not committed: the commit prompt already ran once in this invocation. Run the commit prompt, then run the PR prompt again to push"
              - message: |-
                    🔧  the local test failures on `{{branch}}` are fixed and **staged, but not committed**: the commit prompt already ran once in this invocation and cannot be entered twice. Run the commit prompt, then run the PR prompt again to push.

                    {{ frontmatter(report, 'summary') }}
              - say: "The local test failures are fixed and staged. Run the commit prompt, then run the pull request prompt again."
              - stop
        - when: "frontmatter(report, 'status') == 'fixed'"
          action:
              - success: "the local test failures on `{{branch}}` are fixed and staged; handing them to the commit prompt"
              - message: |-
                    🔧  the local test failures on `{{branch}}` are fixed and staged; handing them to the commit prompt

                    {{ frontmatter(report, 'summary') }}
              - action: proxy
                target: ../commit.md
                with:
                    # `commit.md` lists `ctx.staged_files` in its body, and `ctx` is the
                    # start-of-run snapshot, so that list predates the fix was staged. A body
                    # has no live `current`, so the real list travels in `message`.
                    message: "These staged files repair local test failures that blocked the push of `{{ branch }}`. {{ frontmatter(report, 'summary') }} IMPORTANT: the staged-file list printed in this prompt was captured before the fix was staged, so it is out of date. Run `git diff --cached --name-only` for the real list. At handoff it was: {{ as_csv(frontmatter(report, 'staged')) }}."
                    success:
                        success: "the fix is committed. Run the PR prompt again to push it; the pre-push hook reuses the passing evidence this run already produced"
                        message: "🗳️  the fix for the local test failures on `{{ branch }}` is committed. Run the PR prompt again to push it; the pre-push hook reuses the passing evidence this run already produced"
                        say: "The fix is committed. Run the pull request prompt again to push it."
        - when: "frontmatter(report, 'status') == 'error'"
          action:
              - message: |-
                    ⏸️  the local test failures on `{{branch}}` were not fixed here

                    {{ frontmatter(report, 'summary') }}
              - say: "The local test failures were not fixed. {{ frontmatter(report, 'summary') }}"
              - error: "{{ frontmatter(report, 'summary') }}"
        - when: "frontmatter(report, 'fix_attempts') >= max_attempts"
          action:
              - warn: "the local test failures on `{{branch}}` are still not fixed after {{ max_attempts }} attempts; what each attempt tried is in {{report}}"
              - message: "💥  the local test failures on `{{branch}}` are still not fixed after {{ max_attempts }} attempts; what each attempt tried is in `{{report}}`"
              - say: "The local test failures are still not fixed after {{ max_attempts }} attempts."
              - error: "the fix did not verify within {{ max_attempts }} attempts"
        # Each retry is a fresh session that starts from the `## Fix attempt` notes
        # its predecessors left in the report. The budget matches `max_attempts`.
        - action:
              - warn: "attempt {{ frontmatter(report, 'fix_attempts') }} of {{ max_attempts }} did not produce a verified fix; starting a fresh attempt"
              - retry: 2
failure:
    message: "❌  fixing the local test failures on `{{branch}}` failed ({{ctx.agent}}/{{ctx.model}}: {{err.msg}})"
    say: "Fixing the local test failures failed."
---

# Fix Local Test Failures

## Context

- A push of the `{{branch}}` branch of **{{ctx.repo}}** was blocked because local testing on **{{ctx.os}}** failed. The failures have been diagnosed, and the caller wants them fixed.
- A fix gets up to {{max_attempts}} attempts, and this is attempt {{ frontmatter(report, 'fix_attempts') || 1 }}. Each attempt is a fresh session, so the report file is the only memory shared between them.
- You are running non-interactively: nobody can answer a question during this session. Record what you find in the report file instead.
    - If a tool call or a file read is refused, say so in your closing summary and find another way to the same information.

::file ./_report.md

## Task

1. Read `{{report}}` in full. The `issue` property is your briefing: the problems to repair, their root causes, the commands that reproduce them, and the constraints the repair must respect.
    - If the report body already has `## Fix attempt` sections, earlier attempts did not produce a verified fix. Read every one before you change anything, and do not repeat an approach that is recorded there as failed.
2. Repair only the problems whose `fix_branch` is `{{branch}}`.
    - A problem whose `fix_branch` names another branch is not repaired here. Leave it alone and say so in `summary`.
    - Repair the cause the diagnosis named. Do not weaken, skip, or delete a test to make it pass, and do not refresh a pinned hash or fixture without re-deriving it the way `issue` describes.
    - Touch only what the repair needs.
3. Prove the repair: run the reproducing commands from `issue` and confirm that each one now passes. Run `just lint` in every package area you changed. Do not run the full pre-push gates and do not push; the hook does that on the next run of the PR prompt.
4. Append a `## Fix attempt <n>` section to the report body, numbered after any that are already there, recording what you changed, which commands you ran, and their results. Record an approach that failed just as carefully as one that worked.
5. Record the outcome in the report frontmatter:
    - **Every chosen problem is repaired and verified.** Stage exactly the files you changed with `git add <paths>`, list them in `staged`, replace `summary` with one or two sentences on what was repaired, and set `status: fixed`.
    - **Nothing could be repaired on this branch** (every problem belongs on another branch or to the environment). Set `status: error` with a `summary` saying what the caller has to do instead.
    - **The repair did not verify.** Leave `status` as it is and unstage nothing you did not stage. The next attempt starts from your notes.

**Never commit and never push.** Every commit in this repository goes through the commit prompt, which runs after this session when the fix verified. Never bypass the pre-push hook.
