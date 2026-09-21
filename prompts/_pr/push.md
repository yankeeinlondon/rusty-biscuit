---
name: Push for a Pull Request
description: |-
    Settles which branch is being proposed and pushes it. Reached from `pr.md` (or from `commit.md`, when uncommitted changes were committed first). The repository's pre-push hook runs the local test gates during the push, so the push *is* the local test run.

    The agent is given the repository facts it would otherwise spend tool calls discovering. Its own work is the part that cannot be scripted: `git push` is on the shell blacklist for lifecycle and template commands, so only an agent can run it. It records a raw outcome and stops; explaining a blocked push belongs to `_pr/diagnose.md`.
$schema:
    base: string(required) -> the branch the pull request targets
    report: string(required) -> the report file shared by every stage of the PR flow
    branch: string -> the name for the proposed branch when one has to be created (the checkout is on the base branch); derived from the commits when omitted
    title: string -> an optional pull request title; derived from the commits when omitted
    about: string -> optional context from the caller about what this work is
    on_failure: enum(ask, fix, stop) -> what happens after a blocked push is diagnosed
    committed: boolean -> whether `commit.md` already ran in this invocation (a proxy chain can visit it only once)
on_failure: ask
committed: false
log: "{{ replace_last(report, '.md', '.push.log') }}"
commits_ahead: "$(git rev-list --count origin/{{base}}..HEAD)::timeout:30"
# the pre-push hook can run silently for well over the default stream-silence window
step_timeout: 90m
timeout: 4h
start:
    stack:
        - when: "number(commits_ahead, 0) == 0"
          action:
              - set_frontmatter: ["{{ report }}", "status", "error"]
              - set_frontmatter: ["{{ report }}", "summary", "There was nothing to propose: HEAD has no commits that origin/{{ base }} lacks."]
              - message: "🤨  nothing to propose from `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }}: `HEAD` has no commits that `origin/{{base}}` lacks"
              - error: "`HEAD` has no commits that `origin/{{base}}` lacks, so there is nothing to open a pull request for"
        - action:
              - info: "proposing {{ commits_ahead }} commit(s) from `{{ctx.branch}}`; the pre-push hook runs the local test gates on this host ({{ctx.os}}) during the push"
              - message: "🚦  proposing {{ commits_ahead }} commit(s) from `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }}; local testing runs on this host ({{ctx.os}}) first"
success:
    stack:
        - when: "frontmatter(report, 'status') == 'passed'"
          action:
              - success: "local testing passed on {{ctx.os}}; `{{ frontmatter(report, 'branch') }}` is pushed and ready for CI/CD"
              - message: |-
                    ✅  local testing passed on **{{ctx.os}}** for `{{ frontmatter(report, 'branch') }}` in {{ ctx.area_description || ctx.repo }}; the branch is pushed and ready for CI/CD

                    {{ frontmatter(report, 'summary') }}
              - say: "Local testing passed on {{ctx.os}}. The branch is pushed and ready for CI CD."
              - action: proxy
                target: ./open.md
                with:
                    base: "{{ base }}"
                    branch: "{{ frontmatter(report, 'branch') }}"
                    report: "{{ report }}"
                    title: "{{ title || '' }}"
                    about: "{{ about || '' }}"
        - when: "frontmatter(report, 'status') == 'local_failed'"
          action:
              - warn: "local testing failed on {{ctx.os}}; the push was blocked, and the failures are being diagnosed"
              - message: |-
                    ❌  local testing failed on **{{ctx.os}}** for `{{ frontmatter(report, 'branch') }}` in {{ ctx.area_description || ctx.repo }}; the push was blocked, and the failures are being diagnosed

                    {{ frontmatter(report, 'summary') }}
              - say: "Local testing failed on {{ctx.os}} and the push was blocked. The failures are being diagnosed."
              - effect: sad-trombone
              - action: proxy
                target: ./diagnose.md
                with:
                    base: "{{ base }}"
                    report: "{{ report }}"
                    on_failure: "{{ on_failure }}"
                    committed: "{{ committed }}"
        - when: "frontmatter(report, 'status') == 'error'"
          action:
              - message: |-
                    💥  local testing on **{{ctx.os}}** could not produce a verdict for `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }}

                    {{ frontmatter(report, 'summary') }}
              - say: "Local testing on {{ctx.os}} could not produce a verdict. {{ frontmatter(report, 'summary') }}"
              - error: "{{ frontmatter(report, 'summary') }}"
        # The agent finished without recording an outcome. It still holds the push
        # output in its session, so ask it once before giving up; `nudged` lives in
        # the report because runtime state does not survive the re-entry.
        - when: "!frontmatter(report, 'nudged')"
          action:
              - set_frontmatter: ["{{ report }}", "nudged", "{{ true }}"]
              - warn: "the push agent finished without recording an outcome; asking it to finish the report"
              - resume: "You ended the session without recording an outcome. Set `status` in the frontmatter of {{ report }} to `passed`, `local_failed`, or `error`, along with the other properties the report contract requires for that status, and then stop."
        - action:
              - message: "💥  the push agent for `{{ctx.branch}}` finished without recording an outcome in `{{report}}`, even after being asked to"
              - say: "The push agent finished without recording an outcome."
              - error: "the push agent did not set `status` in {{report}}"
blocked:
    message: "💥  the push stage for `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }} was **blocked** before the agent started: {{err.msg}}"
    say: "The push stage was blocked before the agent started."
failure:
    stack:
        - when: "err.category == 'timeout'"
          action:
              - warn: "the push agent was stopped by a timeout ({{err.code}}); resuming its session"
              - resume: "You were stopped by a timeout. If the push is still running in the background, keep polling {{ log }}; otherwise continue from where you left off, record the outcome in {{ report }}, and stop."
        - when: "err.is_transient || err.is_throttled"
          action:
              - warn: "the push agent failed with a recoverable error ({{err.code}}); retrying once. The hook reuses the evidence it already published, so the retry is cheap"
              - retry: 1
        - action:
              - message: "❌  the push stage for `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }} failed ({{ctx.agent}}/{{ctx.model}}: {{err.msg}})"
              - say: "The push stage of the pull request flow failed in {{ ctx.area_description || ctx.repo }}."
---

# Push a Branch for a Pull Request

## Context

- You are proposing work from the **{{ctx.repo}}** repository as a pull request against `{{base}}`. Local testing happens on this host before anything reaches CI/CD.
- You are running non-interactively: nobody can answer a question during this session. Record what you find in the report file instead.
    - If a tool call or a file read is refused, say so in your closing summary and find another way to the same information.
::block when="about"
- The caller described this work as:

    > {{about}}
::end-block

<!-- Two constraints shape this section.
     1. The shell blocks are inline on purpose. A shell command in a transcluded partial that
        interpolates a value delivered by `proxy ... with:` (here `base`) is not pre-approved on
        the proxy path, so moving them into a shared `::file` partial breaks the facts.
     2. Git state comes from shell blocks, not `ctx.*`. `ctx` is the start-of-run snapshot, so
        `ctx.dirty_files` would still list files that `commit.md` committed on the way here, and
        the live `current` is available to lifecycle handlers but not to a document body. -->
## What Is Already Known

These facts were gathered immediately before this session started. Use them as given rather than re-running the commands; re-check a fact only when something you did could have changed it.

- The checkout is on the `{{ctx.branch}}` branch of **{{ctx.repo}}**, on a **{{ctx.os}}** host.

### Uncommitted changes

The output of `git status --short`. Nothing between this paragraph and the next heading means the working tree is clean. Anything listed makes the pre-push hook replan from the working tree and withhold the evidence CI would otherwise reuse.

::shell-block timeout=30 when_error="(could not be determined on this host)"
git status --short
::end-block

### Is local `{{base}}` current with `origin/{{base}}`?

`origin/{{base}}` was fetched first. The two numbers are commits only on local `{{base}}`, then commits only on `origin/{{base}}`; `0 0` means they match.

::shell-block timeout=30 when_error="(could not be determined on this host)"
git fetch --quiet origin {{base}}
git rev-list --left-right --count {{base}}...origin/{{base}}
::end-block

### How `HEAD` relates to `origin/{{base}}`

Commits only on `HEAD` (the work to propose), then commits only on `origin/{{base}}` (what this branch is behind by):

::shell-block timeout=30 when_error="(could not be determined on this host)"
git rev-list --left-right --count HEAD...origin/{{base}}
::end-block

### Does `{{ctx.branch}}` have a remote?

The last commit on `origin/{{ctx.branch}}`, then the number of local commits not yet pushed to it:

::shell-block timeout=30 when_error="(there is no remote branch yet: this branch has never been pushed)"
git log -1 --format=reference origin/{{ctx.branch}}
git rev-list --count origin/{{ctx.branch}}..HEAD
::end-block

### The commits to propose (`origin/{{base}}..HEAD`)

::shell-block timeout=30 when_error="(could not be listed on this host)"
git log --oneline origin/{{base}}..HEAD
::end-block

### Packages with source changes, and what the hook will run

This is the output of `just ci-local --plan`: the packages whose source changed relative to `origin/{{base}}`, the cells the pre-push hook will run for them on this host, and the cells it will reuse from earlier passing evidence.

::shell-block timeout=120 when_error="(the plan preview failed; the hook prints its own plan when it runs)"
just ci-local --plan
::end-block

::file ./_report.md

## Task

### 1. Settle the branch

::block when="ctx.branch != base"
The work is on `{{ctx.branch}}`, which is not the base branch, so that is the branch to propose. Write `branch: {{ctx.branch}}` into the report frontmatter.
::end-block
::block when="ctx.branch == base"
The checkout is on `{{base}}`, the branch the pull request targets, so the {{commits_ahead}} unpushed commit(s) listed above need a branch of their own. **Never push `{{base}}` itself.**

::block when="branch"
- Create the branch the caller named and move to it: `git switch -c {{branch}}`.
::end-block
::block when="!branch"
- Derive a branch name from the commits, following the repository's `<type>/<short-topic>` convention (for example `fix/ci-worker-budget`), then create it and move to it with `git switch -c <name>`.
::end-block
- If that name already exists locally or on `origin`, choose another rather than reusing it.
- Leave local `{{base}}` alone: do not reset or rewind it. Mention in your closing summary that it still carries the commits.
- Write the new branch's name as `branch` into the report frontmatter.
::end-block

Also write `head` (the full SHA of `HEAD`), `base`, and `commits` (one `<short sha> <subject>` per commit to propose, from the list above) into the report frontmatter now.

### 2. Local testing: push the branch

Run `git push -u origin HEAD`.

The repository's pre-push hook runs the local test gates for every affected package on this host and blocks the push when any of them fails. **That hook _is_ the local testing step**; do not run the gates separately first, because only the hook publishes the evidence CI reuses. Never bypass it: no `--no-verify`, and do not set `RUSTY_BISCUIT_PRE_PUSH`.

- The hook routinely takes 15–30 minutes. If your shell tool enforces a shorter time limit, run the push in the background and poll until the process exits.
- Either way, capture the push's complete output (stdout and stderr) to `{{log}}`. The next stage reads that file instead of re-running anything.

### 3. Record the outcome, then stop

Record exactly one of these outcomes.

- **The push succeeded.** Confirm the remote branch now points at `HEAD` (`git rev-parse @{u}` equals `git rev-parse HEAD`). Set `status: passed` and a `summary` naming how many commits were pushed and which package areas the hook tested. **Do not open the pull request**; the next stage does that.
- **The hook blocked the push because gates failed.** Set `status: local_failed`, set `log` to `{{log}}`, and write a `summary` that says only which gates failed, as the hook printed them. **Do not investigate, reproduce, or fix anything**: the diagnosis stage starts from your log with a full budget of its own.
- **Something else went wrong** (authentication, a rejected non-fast-forward, the hook crashing before any test ran, a full disk). Set `status: error` with a `summary` that names the reason. Do not force-push and do not work around a rejection.

End with a short summary for the caller: the branch, the outcome, and the report file's location.
