---
$schema:
    agent: string(required) -> the agent to be used for this prompt
    model: string -> the model to be used for this prompt
    message: string -> if you want to provide a message that describes what this commit is about it will be fed into the prompt to help the agent name the commits
    plan: file -> if you want to have the "message" be that these commits are related to a plan than you can include a file reference to the plan
    phase: number -> if you've provided a plan to explain these commits then you can specify the specific phase which these commits belong to
    lessons_learned: file
lessons_learned: "@.claudine/memory/commits.md"
timeout: 30m
step_timeout: 12m
show_system_prompt: false
operation: commit
agent: opencode
model: |-
    {{
        agent == "opencode" && !model
            ? "minimax/MiniMax-M3"
            : model
    }} 
resides_in: |-
    {{
        length(ctx.dirty_package_areas) == 1
            ? 'are all part of the ' + as_csv(ctx.dirty_package_areas) + ' package area'
            : 'are spread across ' + length(ctx.dirty_package_areas) + ':\n ' + as_unordered_list(ctx.dirty_package_areas)
    }}
initialize:
    stack:
        # `current`, not `ctx`: this asks what is staged now. `ctx` is the start-of-run
        # snapshot, which predates any staging an earlier stage of the same run did.
        - when: "length(current.staged_files) == 0"
          action:
              - message: "🤨  there were no staged files to commit in {{ctx.repo}}!"
              - stop
start:
    message: |-
        🗳️  starting **git commits** in {{ ctx.repo }} -> **{{ctx.branch}}** 
        {{ ctx.is_monorepo ? '\n  **packages impacted:** _' + as_csv(ctx.dirty_packages) + '_' : '' }}
success:
    message: |-
        🗳️  staged files in {{ctx.area || ctx.repo }}'s **{{ctx.branch}}** branch, have been **committed to git** (_but not pushed_)
failure:
    message: |-
        💥  the staged files in {{ctx.area || ctx.repo }}'s **{{ctx.branch}} branch failed to commit as requested! The error was:

        {{err.msg}}
---

# Commit Staged Files

## Context

You are being asked to commit files to git using the **Conventional Commits** naming convention.
::block when="message"

- The caller has provided some context on the changes that took place and resulted in the staged files becoming _staged_:

    > {{message}}

::end-block
- There are {{ length(ctx.staged_files) }} staged files to commit which {{resides_in}}
- The staged files are:

    {{ as_unordered_list(ctx.staged_files) }}

### Conventional Commits

[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) provides a convention for how commit messages should be structured. In this monorepo we always use the conventions proposed in the by this standard, which state a commit message following the general structure of:

- `{operation}({package}): {message}`
- or in a few rare cases, as `{operation}: {message}`

> **Note:** there are two cases where it's ok to have no 'scope' (aka, package) associated to a commit message:
>
> 1. If the change appears to have no relationship to any particular package in the monorepo.
> 2. If there are bunch of small changes which are all related to the same underlying event or cause and the changes do not touch any source code

The valid operations we use include: fix, docs, chore, feat, refactor, style, perf, test, ci, planning.

- `refactor` is reserved for commits that include source code.
- `feat` is reserved for code that ships; a new spec under `features/` is not a `feat`.

### The `planning` operation

`planning(<area>)` covers every change to a feature or fix's lifecycle artifacts (`spec.md`, `plan.md`, `design.md`, `implementation-log.md`, `review-N.md`, spikes, and the directory itself) when no code ships in the same group. The scope is the spec's frontmatter `area`; a repo-wide spec is `planning(repo)`.

- Subject shapes, in lifecycle order: `schedule <name>` (a new spec in a dated directory, or a move out of `_unscheduled`), `record execution plan for <name>`, `record Phase N close for <name>`, `record review-to-implement iteration N`, `close <name> cycle N, open cycle N+1`, `close <name> cycle N and move to completed`, `supersede <old> with <new>`, `close <name> as invalidated`.
- One lifecycle event is one commit. Artifacts that reference each other land together: a spec with the plan and spike it cites; a superseded spec with its successor; a review that flips `implemented: true` with the review that opens the next cycle. Such commits may have zero source diff; that is normal.
- A planning commit that names files in its frontmatter lists (`source_files_during_phase_N`, `docs_created_during_phase_N`) lands **after** the commits that ship those files, so the claim is true at HEAD.
- Do not paraphrase in a close body; quote what the staged text says.

### Agent Skills
::block when="ctx.current_package_area && has_skill(ctx.current_package_area)"

This git commit operation was called from within the **{{ctx.current_package_area}}** package area. There is an Agent Skill "{{ctx.current_package_area}}" that provides insights into this package area that you should consider using to help understand this package area.

If the packages which have staged files in them are outside of the {{ctx.current_package_area}} package area
you may also want to consider using the Agent Skill's associated with given package areas where the
staged files exist. 

> **Note:** not all package areas have a corresponding Agent Skill but many of them do
::end-block
::block when="!ctx.current_package_area || !has_skill(ctx.current_package_area)"

Some but not all of the package areas in this monorepo have Agent Skill's of the same name as the package area (e.g., "claudine" agent skill for "claudine" package area). You should consider these skills when they are available to the package areas the staged files reside in.

::end-block

## Concurrent Repo Activity

While you are committing the staged files across 1:M commits (based on semantic content) there may be other activity taking place in the same repo and branch. This should not be a surprise to you and it means that it's important not to unstage and then restage content during your task's timeframe. 

Above you'll find the complete list of staged files at the beginning of this operation and you your responsibility is only to those files even if more files become _staged_ during the course of your work.

It is important that the git operations you perform do not interfere with this expected behavior.

## Orchestration

You should act as an orchestrator when you see opportunities to do so effectively. This will be done primarily to:

- allow for concurrent activity 
- preserve the context window as much as possible

> **IMPORTANT:** when you spawn a sub-agent always reinforce that they CAN NOT ask the user for feedback or permissions as they are in a non-interactive session!

## Best Practices and Lessons Learned

The following best practices should always be followed:

- Do not try to group commits together by unstaging and restaging one group at a time! You should simply commit the group files explicitly!
    - this will result in potential corruption
    - you should expect that developers are actively working on this code base while you're working. that means staging and unstaging files can have unexpected consequences!
- you should not run tests, build any packages, or run a formatter. Your job is to commit what you were given and you should assume that all validations before the commit were already done.
- NEVER use commands like `git reset`!
- Never pass the message inline with `-m "…"`. Commit bodies routinely contain backticks (inline code like `` `::end-block` ``), `$`, and other shell meta characters; inside a double-quoted `-m` string the shell evaluates those (backticks are command substitution even within double quotes), which corrupts the message
- when multiple subagents commit in parallel against the same worktree, `git commit` can fail with `fatal: Unable to create '.git/index.lock': File exists.` (or the equivalent `refs/heads/<branch>.lock` variant). This is not corruption — git's locks are fail-fast, not queuing. On such a failure, wait 1–3 seconds and retry the same `git commit --only …` command. Retry up to 5 times with short backoff before giving up and reporting failure to the orchestrator.

In addition to the best practices above, the repository keeps a short journal of git mechanics that agents have found the hard way. Treat it as advice from prior runs, not as rules that override the task above:

::file {{lessons_learned}}

Step 5 says when, and how rarely, an entry may be added.

## Task

Your task is to:

1. evaluate all the _staged_ files in this monorepo,
::block when="message"
    - also remembering that the caller has provided this additional context:

        > {{message}}

    - this context may help us contextualize the changes of some or all of the staged files you are responsible for
::end-block
2. Look for files that have been _moved_ and make sure they are captured into groups first so that we can correctly commit these commits correctly
3. Then organize the remaining staged files into **semantic groups**
    - a group will have one or more _staged files_ associated with it
    - _staged files_ must only belong to a single group
    - each "group" will eventually become an individual commit
    - determine which conventional commit "operation" (e.g., fix, doc, chore, feat, etc.) does this group belong to?
    - what should the top line description of this commit be?
    - under the top line description we should add bullet points that flesh out the characteristics of the changes this group is bringing with it

4. Commit all of the groups to git:

    - use your best judgment but this is likely something that 
        - can be done in parallel 
        - and with you acting as an orchestrator of subagents
    - commit messages must have:
        - a top line description of a sentence or two that describes the commit
        - a set of bullet points underneath that tease out some of the important details of the commit (WHAT changed? WHY did it change? Was this part of a review? A plan's execution?)
    - Example Commit:

        ```
        feat(biscuit-clip): add initial clipboard package structure

        - add README.md documenting package purpose and usage
        - add lib.rs with basic module exports
        - add cli.rs for command-line interface
        ```

        You or your subagents can always run `git log --oneline -20` for real commits in the repo.

    **IMPORTANT:** you and the subagent SHOULD NOT push commits to a remote!

    > When working with subagents:
    > 
    > - if the subagent is not able to make a commit for any reason then this needs to be communicated back to the orchestrator with details on why they weren't able to commit.
    > - the subagent SHOULD be reminded that they are running in a non-interactive session so there is no way to get feedback from the user and attempts should be made to achieve the goals without asking for additional context
    
5. Summarization and Closure

    - once all of the staged files you were responsible for have been committed, report back to the caller in summary form: each commit's hash and subject, anything left staged and why, and any unrelated working-tree changes you noticed and left alone.
    - finally, decide whether this batch produced a journal entry. The journal (`{{lessons_learned}}`) holds only **very novel or surprising git mechanics**, and almost every batch produces none. Novelty is not how it felt; it is a test you can check. Add an entry only when **all** of these hold:
        1. a git command did something different from what its documentation, this prompt, or the journal says it does, and you confirmed the behavior a second time rather than inferring it from one failure;
        2. the difference would bite any agent in any repository, not only this batch, this package, or this spec;
        3. no existing entry covers it, even partially; if one does, you leave the journal alone (you do not extend or "refine" an entry);
        4. you can state it in at most three lines as a general rule with no commit hash, no feature or fix name, and no account of what you were doing when you found it.
    - the following are **never** journal material, whatever the temptation: how to word a commit subject, which files belong together in a batch, how a planning close is shaped, anything about a specific package or spec, and anything you already knew before the batch started
    - if all four tests pass, add the entry under the right heading, keep the file's existing style, and commit it as a one-file commit with the subject `docs(agent-memory): <rule in six words or fewer>`; then tell the caller what you added
    - if you are unsure whether the tests pass, they do not; put the candidate under a **Proposed journal entry** heading in your summary instead and let the owner decide
