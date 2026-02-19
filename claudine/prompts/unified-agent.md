# Unified Agent

**IMPORTANT:** this is part of a general redesign of the current library code, do NOT use library code as a reference as we are expecting a major refactoring of this code.


## Functional Outcome

A detailed design for each of the individual CLI Agent's capabilities structs:

- each CLI Agent that we support will have a similarly structured Rust `struct` which defines all the capabilities for that agent.
- in order to drive consistency across each of these structs we will have defined a `Agent` trait that each of the individual structs will have to implement

## Task

To do this in a time sensitive as well as context-window sensitive manner you should setup as an orchestrator and you will kickoff the following agents:

- claude-code
- codex
- gemini-cli
- goose
- kimi-code
- opencode
- qwen-cli
- roo-code

Each agent will be tasked with creating detailed design specs for a particular Agentic CLI:

- the first step these agents need are to familiarize themselves with the research:
    - read @claudine/docs/cross-referencing/{agent}.md for information on this given agent's cross-referencing information
    - read @claudine/docs/agent-cli/{agent}.md for information on the mechanics and optionality that the CLI client itself provides
    - NOTE: replace `{agent}` with the agent's name in the two filepaths above
- now the subagent must design:
    - Define a `struct` you recommend for the focused Agentic CLI
        - this struct needs to capture all metadata that the we have been able to capture in our research into a structure that we might reasonably expect other Agentic CLI's to record their needs in as well
        - your primary goal is to create a structure that works for _your_ Agentic CLI but where it could be easily made to accommodate some other
        - Things the struct will need to be able to capture:
            - Does this Agentic CLI support Agent Skills?
                - How does it support skills?
                - Where are these skills located?
                - What is the link for documentation for agent skill for this Agentic CLI?
                - etc.
            - Does this Agentic CLI support slash commands?
                - How does it support slash command?
                - Where are slash commands saved? Do they support sub-directories? Does it also look in Claude Code directories for this (many do so that they can benefit from Claude Code's roll as a feature leader in the space)?
                - What is the URL for documentation for slash commands for this Agentic CLI?
            -
    - Define a Rust trait which you think might be used to ensure all Agentic CLI's can either share the same properties or at least a set of functions which can be used for all Agentic CLI's.
- the subagent will save their intermediate design information in as @claudine/docs/agent-designs/{agent}.md where `{agent}` is the name of the agent
- the subagent will return to the orchestrator a summary of their findings as well as the filepath to it's design document.
- when all subagents have returned, the orchestrator will kick off an "consolidation" agent:
    - the consolidation agent will be pointed to the @claudine/docs/agent-designs directory and told to consolidate these documents into a comprehensive design document
    - it is expected and a deliberate design decision to have each individual document do their design blinded to the other's design
    - this variety will provide a rich tapestry of design choices which the consolidation agent will be responsible for determining which of these is the best fit or taking elements from multiple individual designs if that's more appropriate.
    - the end goal is to arrive at the stated **Functional Outcome** defined above
        - this design document will be saved to @claudine/docs/unified-agent.md

