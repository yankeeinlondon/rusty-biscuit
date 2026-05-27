---
sequence:
    - name: initial
      agent: codex
    - name: Asana
      agent: gemini
    - name: Clickup
      agent: gemini
    - name: Jira
      agent: gemini
    - name: Monday
      agent: gemini
    - name: Trello
      agent: gemini
    - name: Wrike
      agent: gemini
    - name: closure
      agent: codex
agent: "{{state.agent}}"
prompt: |-

    ## Context

    We have performed research into the API's and Schemas that the following Work SAAS providers provide:

    - Asana (./asana.md)
    - Clickup (./clickup.md)
    - Jira (./jira.md)
    - Monday (./monday.md)
    - Trello (./trello.md)
    - Wrike (./clickup.md)

    You will find a file for each of these providers in @schematic/docs/research/project-mgmt directory.

    The overarching goal is to be able to present a high quality canonical schema for the following:

    - Task
        - a "task" is an important unit of work for all of the project management providers
        - how should a Task / Todo / Action be modelled (in Rust) in such a way that it can be mapped to all providers
        - it should have ergnomic features to allow the easy creation of a Task from basic attributes as well as from each of the major providers platform's proprietary schemas
    - Person
        - people are often an important schema to model well
        - we want to be able to model people who are users of the system but also users who are being assigned tasks
        - there should be a single model for both types of users but a way to distinguish whether they are users of the system(s); all users will be able to be assigned tasks
    - Company
        - a company or organization must be able to express metadata about itself that defines it 
        - it must also be able to have a clear relationship to people
            - the most common relationship to people would be an employee relationship
            - however, it would be useful if the _kind_ of relationahip were allowed to be flexibly defined to include other relationship types too
    

    ## Task

    Your task is to "roll up" the information across these providers to provide canonical representations
    
---
