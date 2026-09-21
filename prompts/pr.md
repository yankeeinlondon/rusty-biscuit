---
name: Pull Request
description: |-
    Proposes the current work as a pull request, testing locally first and keeping the caller informed at every checkpoint.

    This document is a **router**: it launches no agent of its own. It checks what can be checked without one, creates the run's report file, and hands off to the first stage that has real work to do.

    1. `_pr/dirty.md` runs only when the working tree has uncommitted changes. It asks whether to commit them; a "yes" stages them and hands off to `commit.md`, which continues to the push stage when it succeeds.
    2. `_pr/push.md` settles which branch is being proposed and pushes it. The repository's pre-push hook runs the local test gates on this host first, so **the push is the local test run**.
    3. `_pr/open.md` opens the pull request once local testing has passed, which hands the work to CI/CD.
    4. `_pr/diagnose.md` runs instead when the hook blocked the push: it explains what failed, why, and whether this branch caused it.
    5. `_pr/triage.md` presents that diagnosis in an interactive session and asks whether to fix it (`on_failure=ask`, the default).
    6. `_pr/fix.md` repairs the chosen problems, proves the repair, stages it, and hands off to `commit.md`.

    A proxy chain may visit a document only once, and every commit goes through `commit.md`. A run that needed a fix therefore ends with the fix committed, and the caller runs this prompt again to push it. That second run is cheap: the hook reuses the passing evidence the first run produced.

    Stages share state through the `report` file; see `_pr/_report.md` for its contract.
$schema:
    base: string -> the branch the pull request targets
    branch: string -> the name for the proposed branch when the push stage has to create one (you are on the base branch); derived from the commits when omitted
    title: string -> an optional pull request title; derived from the commits when omitted
    about: string -> optional context from the caller about what this work is
    on_failure: enum(ask, fix, stop) -> what happens after a blocked push is diagnosed; `ask` opens an interactive triage session, `fix` goes straight to the fix stage, `stop` ends with the diagnosis
    report: string -> the report file shared by every stage of the PR flow; created fresh for each run
base: main
on_failure: ask
report: "{{ ctx.repo_root + '/.claudine/tmp/pr/' + kebab_case(ctx.branch) + '-' + ctx.timestamp + '.md' }}"
initialize:
    stack:
        - when: "!ctx.branch"
          action:
              - error: "the checkout is not on a branch (detached HEAD?); check out the branch you want to propose before running the PR prompt"
        - when: "!has_command('gh')"
          action:
              - error: "the GitHub CLI (`gh`) is not on PATH; the PR flow needs it to open the pull request, and checking now is cheaper than finding out after the pre-push hook has run"
        - when: "remote_vendor() != 'github'"
          action:
              - error: "the PR flow opens pull requests with `gh`, but this repository's remote is hosted on `{{ remote_vendor() || 'an unrecognized provider' }}`"
        - action:
              - ensure_dir: "{{ dirname(report) }}"
              - ensure_file: "{{ report }}"
              - set_frontmatter: ["{{ report }}", "status", "pending"]
              - set_frontmatter: ["{{ report }}", "base", "{{ base }}"]
        - when: "ctx.dirty_files"
          action:
              - warn: |-
                    the working tree has {{ length(ctx.dirty_files) }} uncommitted file(s); a dirty checkout makes the pre-push hook withhold the evidence CI reuses, so they are dealt with before anything is pushed
              - action: proxy
                target: ./_pr/dirty.md
                with:
                    base: "{{ base }}"
                    branch: "{{ branch || '' }}"
                    title: "{{ title || '' }}"
                    about: "{{ about || '' }}"
                    on_failure: "{{ on_failure }}"
                    report: "{{ report }}"
        - action:
              - action: proxy
                target: ./_pr/push.md
                with:
                    base: "{{ base }}"
                    branch: "{{ branch || '' }}"
                    title: "{{ title || '' }}"
                    about: "{{ about || '' }}"
                    on_failure: "{{ on_failure }}"
                    report: "{{ report }}"
blocked:
    message: "💥  the PR flow for `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }} was **blocked** before any stage started: {{err.msg}}"
    say: "The pull request flow was blocked before it started."
failure:
    message: "❌  the PR flow for `{{ctx.branch}}` in {{ ctx.area_description || ctx.repo }} failed before any stage started: {{err.msg}}"
    say: "The pull request flow failed before it started."
---

# Pull Request Router

This document routes to the stages under `_pr/` from its `initialize` stack and is never meant to reach an agent.

If you are an agent reading this, routing did not happen. Do not push, commit, or open a pull request. Report that `prompts/pr.md` reached its body, which means none of its `initialize` handoffs fired, and stop.
