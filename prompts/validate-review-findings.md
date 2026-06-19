---
$schema:
    review: string(required)
name: Validate Review Findings
description: takes a `review` as input and then evaluates the review's findings with a critical eye while also looking for opportunities to add in additional details that may have been glossed over in the initial review.
agent: "{{ doc.model ? ctx.agent + "/" + ctx.model : ctx.agent }}"
---

## Context

The review "{{review}}" has completed and your job is to iterate over it's findings to ensure that all findings are valid and to fill in details that may have been missed.

## Task

Follow these steps exactly:

1. Create a new log file for this operation at "{{dirname(review)}}/validation-{{basename(review)}}"
    - set `created` Frontmatter on this file to "{{ctx.now}"
    - set `review` Frontmatter on this file to "{{review}}"
    - set `agent` Frontmatter on this file to "{{agent}}"
    - Add H1 heading `# Validation of Review Findings` to start the body of the document
    - Add two H2 headings: 
        - `## Notes`
        - `## Risks and/or Concerns`
2. Set the `verified_by` frontmatter of the review -- located at "{{review}}" -- to "{{dirname(review)}}/validation-{{basename(review)}}"
3. Identify the findings in the review
4. Write a 
5. Act as an orchestrator and iterate serially over each finding. For each finding, create a subagent and ask them to:

    - use the '{{ctx.area}}' skill
    - review the specified finding and:
        - identify any gaps
