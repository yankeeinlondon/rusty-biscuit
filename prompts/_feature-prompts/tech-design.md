### Tech Design

You are responsible for building the technical design for the feature "{{feature}}". You will build that technical design from the file {{base_dir}}/spec.md.

- You will act as an orchestrator for each step
- You must have a subagent execute the following steps:
    - Reviewer
        - review the specification at {{base_dir}}/spec.md
        - create a detailed tech design and save it to {{base_dir}}/tech-design.md
    - Finalization
        - review the tech design file at {{base_dir}}/tech-design.md
        - Ensure the file is idiomatic and well formed Markdown and that all the code block are valid code for the specified language
        - update the document if you see changes that need to be made
    - Summarization
        - provide a summarization of the tech-design at {{base_dir}}/tech-design.md
- you will provide feedback to the caller at each step
- you will append to the log file:
    - the log file is located at: `{{base_dir}}/log.md`
    - start your log entry with the heading `## Tech Design for {{feature}} Complete`
    - add a timestamp
    - write the summary provided by the Summarization subagent
- Update the frontmatter of the log file:
    - use `md set "{{base_dir}}/log.md" tech_design "{{base_dir}}/tech-design.md" --save`
    - use `md set "{{base_dir}}/log.md" last_updated "${YYYY}-${MM}-${DD}" --save`
- communicate to the caller that the tech design is complete

