---
stage: commit-implement
feature: shell-expansion
base_dir: /Volumes/coding/personal/rusty-biscuit/darkmatter/features/shell-expansion
---
# Feature

## Context on Feature Building

We are in the process of building out the "shell-expansion" feature. Building a feature consists of the following discrete steps:

1. Create a detailed technical design (stage: `tech-design`)
2. Create a detailed plan (stage: `plan`)
3. Implement the plan (stage: `implement`)
4. Commit the changes to git (stage: `commit-implement`)
5. Review the plan (stage: `review`)
6. Implement the suggestions in the review (stage: `suggestions`)
7. Commit the changes to git (stage: `commit-review`)
8. Update Documents (`update-documents`)

**IMPORTANT:**

- You are running in a **non-interactive session** so you should not expect the user to respond to questions
- Your job is to act as an **Orchestrator** to help preserve the context window and take advantage of concurrency when that is possible without adding risk to the project

## Your Responsibility

- You are NOT responsible for all of the steps specified above!
- You ARE responsible for precisely ONE of these steps!
- To determine which step that is you will need to look at the `stage` frontmatter property
    - this property will match one of the steps described above

## Your Task

### Commit files from Implementation and Design

This section describes how to perform the actions necessary for the `commit-implement` stage.

- capture the current repo status before making any changes
- before the implementation was executed, we made sure that the current package area was clean (no dirty files)
- now that we've finished the implementation of the plan for the "shell-expansion" feature we need to stage and commit the files which we have modified in the implementation
- we use commit message which follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) standard
- in this case the `action` for the commit will be **feat**
- the `scope` for the commit will almost surely be current **package area** (`sniff package-area`)
- this means that the first line of the commit message will be something like: `feat($(sniff package-area)): implemented the "{{feature}}" feature`
- the remaining lines of the commit should summarize the functionality provided in the feature in a number of bullet points.
- Append to the log file:
    - the log file is located at `{{base_dir}}/log.md`
    - Start your log entry with the heading `## Committed Implementation Changes`
    - Add the initial git status prior to our commit and put that information in `### Before Commit`
    - Add the git status after our commit and put that information in `### After Commit`
- set the `last_updated` frontmatter property on the log file
    - use the command `md set "{{base_dir}}/log.md" last_updated "${YYYY}-${MM}-${DD}" --save`

You are done when:

- you have staged all the files mutated during the implementation
- you have committed these same files with an appropriate Conventional Commit based message
- the log file has been appended to

DO NOT PUSH ANY COMMITS to a REMOTE!


