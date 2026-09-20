---
name: Uncommitted Changes
description: |-
    Reached from `pr.md` by proxy when the working tree has uncommitted changes. It launches no agent. It deliberately leaves the required `uncommitted` property unset, so Claudine asks the caller for it; an unattended caller passes `uncommitted=commit` or `uncommitted=abort` to `pr.md` instead.

    `commit` stages the changes and hands off to `commit.md`, overlaying its `success` event so the flow continues to `_pr/push.md` once the commits exist. `abort` ends the flow.
$schema:
    uncommitted: enum(commit, abort; required) -> The working tree has uncommitted changes. `commit` stages them and runs the commit prompt before anything is pushed; `abort` stops the PR flow so you can deal with them yourself
    base: string(required) -> the branch the pull request targets
    report: string(required) -> the report file shared by every stage of the PR flow
    branch: string -> the name for the proposed branch when the push stage has to create one
    title: string -> an optional pull request title
    about: string -> optional context from the caller about what this work is
    on_failure: enum(ask, fix, stop) -> what happens after a blocked push is diagnosed
on_failure: ask
start:
    stack:
        - when: "uncommitted == 'abort'"
          action:
              - set_frontmatter: ["{{ report }}", "status", "error"]
              - set_frontmatter: ["{{ report }}", "summary", "The PR flow stopped because the working tree had uncommitted changes and the caller chose not to commit them."]
              - message: "⏸️  the PR for `{{ctx.branch}}` is paused; the working tree has uncommitted changes and you chose to deal with them yourself"
              - error: "stopped at the caller's request: the working tree has uncommitted changes"
        - action:
              - info: "staging {{ length(ctx.dirty_files) }} uncommitted file(s) and handing them to the commit prompt"
              - shell: "git add ."
              - action: proxy
                target: ../commit.md
                with:
                    # `commit.md` lists `ctx.staged_files` in its body, and `ctx` is the
                    # start-of-run snapshot, so that list predates the `git add` above. A body
                    # has no live `current`, so the real list travels in `message`.
                    message: "{{ about || 'These are the uncommitted changes that were in the working tree when a pull request was requested.' }} IMPORTANT: the staged-file list printed in this prompt was captured before these files were staged, so it is out of date. Run `git diff --cached --name-only` for the real list. At handoff it was: {{ as_csv(ctx.dirty_files) }}."
                    success:
                        message: "🗳️  the uncommitted changes on `{{ctx.branch}}` are committed; continuing to the push stage"
                        stack:
                            - action:
                                  - action: proxy
                                    target: "{{ ctx.repo_root + '/prompts/_pr/push.md' }}"
                                    with:
                                        base: "{{ base }}"
                                        branch: "{{ branch || '' }}"
                                        title: "{{ title || '' }}"
                                        about: "{{ about || '' }}"
                                        on_failure: "{{ on_failure }}"
                                        report: "{{ report }}"
                                        committed: "{{ true }}"
blocked:
    message: "⏸️  the PR flow for `{{ctx.branch}}` stopped at its uncommitted changes: {{err.msg}}"
failure:
    message: "❌  handling the uncommitted changes on `{{ctx.branch}}` failed: {{err.msg}}"
    say: "Handling the uncommitted changes failed."
---

# Uncommitted Changes

This document decides what happens to uncommitted changes from its `start` stack and is never meant to reach an agent.

If you are an agent reading this, neither handoff fired. Do not stage, commit, or push anything. Report that `prompts/_pr/dirty.md` reached its body, and stop.
