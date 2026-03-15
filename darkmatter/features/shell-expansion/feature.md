---
stage: tech-design
design: ''
files_modified: []
feature: ''
package_areas_affected: []
packages_impacted: []
suggestions: ''
spec: ''
package_area: ''
docs_updated: []
---
# Feature

## Context on Feature Building

We are in the process of building out the "" feature. Building a feature consists of the following discrete steps:

1. Create a detailed technical design (stage: `tech-design`)
2. Create a detailed plan (stage: `plan`)
3. Implement the plan (stage: `implement`)
4. Commit the changes to git (stage: `commit-implement`)
5. Review the plan (stage: `review`)
6. Implement the suggestions in the review (stage: `suggestions`)
7. Commit the changes to git (stage: `commit-review`)
8. Review all README.md documents (`update-readmes`)
9. Update skill (`update-readmes`)
10. Final Commit (`final-commit`)

**IMPORTANT:**

- You are running in a **non-interactive session** so you should not expect the user to respond to questions
- Your job is to act as an **Orchestrator** to help preserve the context window and take advantage of concurrency when that is possible without adding risk to the project

## Your Responsibility

- You are NOT responsible for all of the steps specified above!
- You ARE responsible for precisely ONE of these steps!
- To determine which step that is you will need to look at the `stage` frontmatter property
    - this property will match one of the steps described above

## Your Task

### Tech Design

You are responsible for building the technical design for the feature "". You will build that technical design from the file .

- You will act as an orchestrator for each step
- You must have a subagent execute the following steps:
    - Reviewer
        - review the specification at
        - create a detailed tech design and save it to @/features//tech-design.md
    - Finalization
        - review the tech design file at @/features//tech-design.md
        - Ensure the file is idiomatic and well formed Markdown and that all the code block are valid code for the specified language
        - update the document if you see changes that need to be made
    - Summarization
        - provide a summarization of the tech-design at @/features//tech-design.md
- you will provide feedback to the caller at each step
- your final response will be the summarization provided by the Summarization subagent


