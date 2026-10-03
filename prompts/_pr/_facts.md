---
description: |-
    The repository facts a PR stage's agent would otherwise spend tool calls discovering, gathered as the stage is composed. Transclude it under a heading that says the facts are current; it reads `base` from the stage that includes it.

    A command that cannot run here (a missing program, a timeout) fails the including stage exactly as it would written inline, so a stage never launches with its facts silently missing. A command that runs and fails prints its `when_error` text instead.
$schema:
    base: string(required) -> the branch the pull request targets
---

- The checkout is on the `{{ctx.branch}}` branch of **{{ctx.repo}}**, on a **{{ctx.os}}** host.

### Uncommitted changes

::block when="length(ctx.dirty_files) == 0"
The working tree is clean.
::end-block
::block when="length(ctx.dirty_files) > 0"
These paths have uncommitted changes (staged, unstaged, or untracked): {{ as_csv(ctx.dirty_files) }}. They make the pre-push hook replan from the working tree and withhold the evidence CI would otherwise reuse.
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
