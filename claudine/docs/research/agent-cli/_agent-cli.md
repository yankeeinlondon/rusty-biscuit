---
sequence: "@claudine/docs/providers.yaml"
---

You are responsible for updating the CLI documentation for the {{state.desc}} agent software:

- the current document resides at "@claudine/docs/research/agent-cli/{{state.file}}"
- review the current state to understand the structure and contents
    - if the document doesn't have a H2 section called `## CLI Switch Summary` then you are to add it
- The `## CLI Switch Summary` should be the FULL list of CLI parameters/switches which the Agent provides
    - every switch must be described and examples given
    - if there is a "default" value for what this switch sets then document the default value too
- All other sections should be updated based your online research; you should emphasize recent/current information as this is an area that is changing rapidly and we want to make sure the content we have is fully up-to-date
- Frontmatter
    - Be sure to set the following Frontmatter properties in "@claudine/docs/research/agent-cli/{{state.file}}"
        - `last_updated` - YYYY-MM-Dd
        - `latest-version` - latest version of the Agent software
        - `homepage` - URL of the Agent software
        - `repo` - URL of the Agent's repo (if exists)
        - `docs` - URL of the Agent's documentation
        - `cli_docs` - URL of the CLI documentation
