---
feature: ""
spec: ""
package_area: ""
design: ""
suggestions: ""
docs_updated: []
packages_impacted: []
package_areas_affected: []
files_modified: []
stage: "tech-design"
---

# Feature

## Context on Feature Building

We are in the process of building out the "{{feature}}" feature. Building a feature consists of the following discrete steps:

1. Create a detailed technical design (stage: `tech-design`)
2. Create a detailed plan (stage: `plan`)
3. Implement the plan (stage: `implement`)
4. Commit the changes to git (stage: `commit-implement`)
5. Review the plan (stage: `review`)
6. Implement the suggestions in the review (stage: `suggestions`)
7. Commit the changes to git (stage: `commit-review`)
8. Review all README.md documents (`update-readmes`)
9. Update skill (`update-readmes`)

**IMPORTANT:**

- You are running in a **non-interactive session** so you should not expect the user to respond to questions
- Your job is to act as an **Orchestrator** to help preserve the context window and take advantage of concurrency when that is possible without adding risk to the project

## Your Responsibility

- You are NOT responsible for all of the steps specified above!
- You ARE responsible for precisely ONE of these steps!
- To determine which step that is you will need to look at the `stage` frontmatter property
  - this property will match one of the steps described above

## Your Task

::file @prompts/feature/feature-prompts/tech-design.md when="stage==tech-design"
::file @prompts/feature/feature-prompts/plan.md when="stage==plan"
::file @prompts/feature/feature-prompts/implement.md when="stage==implement"
::file @prompts/feature/feature-prompts/commit-implementation.md when="stage==commit-implement"
::file @prompts/feature/feature-prompts/review.md when="stage==review"
::file @prompts/feature/feature-prompts/suggestions.md when="stage==suggestions"
::file @prompts/feature/feature-prompts/update-readmes.md when="stage==update-readmes"
::file @prompts/feature/feature-prompts/update-skill.md when="stage==update-skill"
