---
sequence:
    - name: Wrike
      url: https://www.wrike.com/
    - name: Monday
      url: https://monday.com
    - name: Trello
      url: https://trello.com
    - name: Asana
      url: https://asana.com/
    - name: Jira
      url: https://www.atlassian.com/software/jira
    - name: Clickup
      url: https://asana.com/
research: "@schematic/docs/research/project-mgmt"
---
# API's for popular Project Management Software

Your task is to research the API support that the SAAS provider {{state.name}}({{state.url}}) provides:

- you will write your research to the file '{{research}}/{{name}}.md'
- the document should start with an H1 header: `# Research into {{name}}'s API Support` and then fill in the following sections:
- `## Overview on Product`
    - give an overview of the product's functional footprint
    - list all key URLs for documentation, APIs, etc.
    - provide an overview of the pricing structure
- `## API Details`
    - list all API's which are supported:
        - REST?
        - Websocket?
        - JSON-RPC?
        - etc.
    - for each API found, provide the following details:
        - Is there a formal schema which defines the API? Something like OpenAPI schema or similar?
        - Does the company provide SDK's and for what languages?
        - What authentication mechanisms are supported?
        - What is the signup process for someone who wants to provide an 
        - Investigate what kind of problems 
- `## Schemas`
    - in this section we want to get a sense of how this vendor thinks about the structure of the following entities:
        - Todo / Action / Task
        - Person / Contact
        - Company / Organization
        - Workflow
        - Status
    - for all of the entities above where we can get a good feeling for how these entities are defined (either as a part of the API or otherwise):
        - create a representative Rust struct that models the entity
        - describe where we are sourcing our schema knowledge from and how confident we are on it
- `## Gotchas`
    - research any "gotchas" or commonly discussed issues that developers working with {{state.name}}'s API's run into
    - identify ways to work around these issues where possible
    - mention any surprising limitations that users report about the API(s) provided by {{state.name}}
