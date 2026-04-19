### Plan

- Create a detailed plan for the {{feature}} feature:

    - Functional Specification: "{{base_dir}}/spec.md"
    - Technical Design Document: "{{base_dir}}/tech-design.md"

- Save the plan to "{{base_dir}}/plan.md"
- Summarize the plan and communicate to summary to the caller
- Append to the log file the summary:
    - The log file is located at: `{{base_dir}}/log.md`
    - Start your log entry with the heading `## Plan for {{feature}}`
- set the `plan` frontmatter property on the log file
    - use the command `md set "{{base_dir}}/log.md" plan "{{base_dir}}/plan.md" --save`
- set the `last_updated` frontmatter property on the log file
    - use the command `md set "{{base_dir}}/log.md" last_updated "${YYYY}-${MM}-${DD}" --save`
