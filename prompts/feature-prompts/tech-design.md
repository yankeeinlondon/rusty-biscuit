### Tech Design

You are responsible for building the technical design for the feature "{{feature}}". You will build that technical design from the file {{spec}}.


- You will act as an orchestrator for each step
- You must have a subagent execute the following steps:
    - Reviewer
        - review the specification at {{spec}}
        - create a detailed tech design and save it to {{base_dir}}/tech-design.md
    - Finalization
        - review the tech design file at {{base_dir}}/tech-design.md
        - Ensure the file is idiomatic and well formed Markdown and that all the code block are valid code for the specified language
        - update the document if you see changes that need to be made
    - Summarization
        - provide a summarization of the tech-design at {{base_dir}}/tech-design.md
- you will provide feedback to the caller at each step
- your final response will be the summarization provided by the Summarization subagent

