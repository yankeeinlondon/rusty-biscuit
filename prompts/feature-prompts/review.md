### Review

We have just implemented the "{{feature}}" feature and all tests are passing but your task is now to review the implementation and make sure that it has faithfully implemented everything described in:

- [The Functional Specification]({{base_dir}}/spec.md)
- [The Technical Design]({{base_dir}}/tech-design.md)

During your review you should:

- look for gaps between the intended functionality versus what was actually changed
- look for ways in which the code could be more idiomatic or ergonomic
- ensure that all functionality has strong test coverage
- ensure that if this package area has both a CLI and Library that these two package have clear separations of concerns:
    - the Library should own all business logic
    - the CLI should be focused on reporting to the terminal and leveraging the library for data and logic
- ensure that all public/exported symbols are adequately annotated with doc based comments

## Closure

- Save your recommendations to: "{{base_dir}}/review.md"
- Append to the log file:
    - the log file is located at `{{base_dir}}/log.md`
    - Start your log entry with the heading `## Review of {{feature}} Completed`
    - Then add a timestamp
    - Then add a short summary of the review findings
- Once the log file has been appended to, set the frontmatter of the log file:
    - use `md set "{{base_dir}}/log.md" review "{{base_dir}}/review.md" --save`
    - use `md set "{{base_dir}}/log.md" last_updated "${YYYY}-${MM}-${DD}" --save`
- Communicate to the caller that the review has been completed
