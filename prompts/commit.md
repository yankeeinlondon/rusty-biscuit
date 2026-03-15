---
lessons_learned: "@docs/knowledge/commits.md"
---
# Commit Staged Files

## Conventional Commits

[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) provides a convention for how commit messages should be structured. In this monorepo we always use the conventions proposed in the by this standard, which state a commit message following the general structure of:

- `{operation}({package}): {message}`
- or in a few rare cases, as `{operation}: {message}`

> **Note:** there are two cases where it's ok to have no 'scope' (aka, package) associated to a commit message:
>
> 1. If the change appears to have no relationship to any particular package in the monorepo.
> 2. If there are bunch of small changes which are all related to the same underlying event or cause and the changes do not touch any source code

The valid operations we use include: fix, docs, chore, feat, refactor, style, perf, test, ci, style.

> **Note:** the action 'refactor' should be reserved for commits which have at least some source code files.

## Package in this Monorepo

This monorepo has the following packages:

::shell sniff repo packages

Of these packages, the following ones appear to have changes _staged_ for commit:

::shell sniff repo staged-packages

## Orchestration

We will act as an aggregator when we can see opportunities to do so effectively. This will be done to allow for concurrent activity as well as to preserve the context window as much as possible.

## Lessons Learned

We keep a permanent "memory file" of important things we've discovered that wouldn't have been obvious to someone with just `git` skills and knowledge of this monorepo.

The lessons learned are found in {{lessons_learned}}

## Staged Files

The following files have been staged for commit:

::shell sniff git staged -v

## Task

Your task is to:

**IMPORTANT:** you must follow these steps exactly
**IMPORTANT:** remember that you are running in a non-interactive mode so you can not ask the user questions and expect a reply!
**IMPORTANT:** DO NOT push commits to any remote!
**IMPORTANT:** you should not run tests, build any packages, or run a formatter. Your job is to commit what you were given and you should assume that all validations before the commit were already done.
**IMPORTANT:** when acting as an orchestrator you should take every opportunity to communicate progress back to the user as they will not be able to see the subagent's work.

0. If no files are staged for commit then communicate this to the user and exit.
1. read the lessons you've learned while making commits by reading:
   - {{lessons_learned}}
2. evaluate all the _staged_ files in this monorepo,
3. organize the work into **semantic groups**
   - each group will have an "operation" and "scope" in addition to the set of files representing the group
4. act as an orchestrator and concurrently execute a subagent for every semantic group:
   - provide the subagent the grouped files and the delta's in these files
   - provide the subagent the "operation" and "scope" (including no scope if that's the determination)
   - tell the subagent to run `sniff git commits` for examples of real commits in this repo
   - the subagent is then responsible for:
       - reviewing the changes and drafting a useful summary of the change for the rest of the message,
       - and then using `git` to make the commit
       - and finally, to let the orchestrator know of any problems they ran into and how they were able to overcome these issues
       - NOTE: if the subagent is not able to make a commit for any reason then this needs to be communicated back to the orchestrator with details on why they weren't able to commit.
       - the subagent SHOULD NOT push commits to any remote!
       - the subagent SHOULD be reminded that they are running in a non-interactive session so there is no way to get feedback from the user and attempts should be made to achieve the goals without asking for additional context
   - the subagent, if it ran into any problems while trying to commit
5. once all the subagents have completed their tasks, you will run `sniff repo` to provide the user a summary of the state of the repo
6. then you will review the "lessons learned" that the subagents provided to you and determine if these are both:
   1. important and worthy of saving to the lessons learned memory file, and
   2. not already represented in the lessons-learned file

   If both criteria are met then you should add a new entry into the lessons-learned file: {{lessons_learned}}
