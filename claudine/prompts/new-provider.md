---
$schema: 
    provider: string(required)
    website: string(required)
    file: string(required)
    repo: string
description: a prompt to create a new Provider in the Claudine library (and CLI)    
base: "{{ctx.repo_root}}/claudine/docs/research"

start: 
    message: "Adding a new provider to Claudine: {{provider}}"
    stderr: "Adding a new provider to Claudine: {{provider}} [${url}]"
---

# Create a New Provider for Claudine

## 1. Research

We will research online to get all the metadata we need to complete the information we'll need for **{{provider}}**.

- Gather all required metadata by running the following commands run in parallel in a "fan out" pattern:

    - Basics: `claudine compose @claudine/docs/research/basics/_basics.md sequence="" state.name="{{provider}}" state.website="{{website}} state.repo="{{repo}}" state.file="{{file}}"`
        - When this command completes it will have written the file to '{{base}}/basics/{{file}}.md'
        - It will have a full report in the basics of **{{provider}}** in the body of the Markdown
        - But more importantly the Frontmatter of the document will provide the following information:

            ```json
            {{ ctx.schema({{base}}/basics/{{file}}.md)}}) }}
            ```

    - Agent Definitions: `claudine compose @claudine/docs/research/agent-definitions/_agent_definitions.md sequence="" state.name="{{provider}}" state.website="{{website}} state.repo="{{repo}}" state.file="{{file}}"`

        - When this command completes it will have written the file to '{{base}}/agent-definitions/{{file}}.md'
        - It will have a full report in the basics of **{{provider}}** in the body of the Markdown
        - But more importantly the Frontmatter of the document will provide the following information:

            ```json
            {{ ctx.schema({{base}}/agent-definitions/{{file}}.md)}}) }}
            ```

    - Hooks: `claudine compose @claudine/docs/research/hooks/_hooks.md sequence="" state.name="{{provider}}" state.website="{{website}} state.repo="{{repo}}" state.file="{{file}}"`

        - When this command completes it will have written the file to '{{base}}/hooks/{{file}}.md'
        - It will have a full report in the basics of **{{provider}}** in the body of the Markdown
        - But more importantly the Frontmatter of the document will provide the following information:

            ```json
            {{ ctx.schema({{base}}/hooks/{{file}}.md)}}) }}
            ```

    - CLI Information: `claudine compose @claudine/docs/research/agent-cli/_agent-cli.md sequence="" state.name="{{provider}}" state.website="{{website}} state.repo="{{repo}}" state.file="{{file}}"`

        - When this command completes it will have written the file to '{{base}}/usage/{{file}}.md'
        - It will have a full report in the basics of **{{provider}}** in the body of the Markdown
        - But more importantly the Frontmatter of the document will provide the following information:

            ```json
            {{ ctx.schema({{base}}/agent-cli/{{file}}.md)}}) }}
            ```

    - MCP: `claudine compose @claudine/docs/research/mcp/_mcp.md sequence="" state.name="{{provider}}" state.website="{{website}} state.repo="{{repo}}" state.file="{{file}}"`

        - When this command completes it will have written the file to '{{base}}/usage/{{file}}.md'
        - It will have a full report in the basics of **{{provider}}** in the body of the Markdown
        - But more importantly the Frontmatter of the document will provide the following information:

            ```json
            {{ ctx.schema({{base}}/mcp/{{file}}.md)}}) }}
            ```

    - ACP: `claudine compose @claudine/docs/research/acp/_acp.md sequence="" state.name="{{provider}}" state.website="{{website}} state.repo="{{repo}}" state.file="{{file}}"`

        - When this command completes it will have written the file to '{{base}}/usage/{{file}}.md'
        - It will have a full report in the basics of **{{provider}}** in the body of the Markdown
        - But more importantly the Frontmatter of the document will provide the following information:

            ```json
            {{ ctx.schema({{base}}/acp/{{file}}.md)}}) }}
            ```

    - Usage Metrics: `claudine compose @claudine/docs/research/usage/_usage.md sequence="" state.name="{{provider}}" state.website="{{website}} state.repo="{{repo}}" state.file="{{file}}"`

        - When this command completes it will have written the file to '{{base}}/usage/{{file}}.md'
        - It will have a full report in the basics of **{{provider}}** in the body of the Markdown
        - But more importantly the Frontmatter of the document will provide the following information:

            ```json
            {{ ctx.schema({{base}}/usage/{{file}}.md)}}) }}
            ```

- Your responsibility is now to wait until all of the commands complete
    - if any commands produce an error; do not fail fast but once all commands have completed:
        - provide a summary to the user which commands completed successfully and which ones did not
        - exit and let the user handle this situation
    - if all commands completed successfully then we have all the metadata that we need to bring support for this new provider into Claudine; progress to the `Generate Code` stage

## 2. Generate Code

- review the metadata that each of the file produced during the research stage
