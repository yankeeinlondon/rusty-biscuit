---
stage: suggestions
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

### Implement Review Suggestions (`suggestions`)

Iterate over the review suggestions in the document "/Volumes/coding/personal/rusty-biscuit/darkmatter/features/shell-expansion/review.md", and for each:

- create a subagent and pass the subagent the suggestion to implement and test
- Append to the log file:
    - the log file is located at: `{{base_dir}}/log.md`
    - start your log entry with the heading `## Review Suggestions Implemented`
    - then add a timestamp
    - then list out the files which were mutated during the review implementation
    - then summarize the changes made
- Now we will update the log file's frontmatter:
    - use `md set "{{base_dir}}/log.md" reviews_files "${files_mutated_during_review}" --save`
    - use `md set "{{base_dir}}/log.md" last_updated "${YYYY}-${MM}-${DD}" --save`
- Communicate to the caller that all review suggestions have been implemented


