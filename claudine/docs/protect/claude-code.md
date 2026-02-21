---
prompt: |-
    ## Focus

    This document is focused on the topic of how a Agentic CLI can be configured and run in a way that is both permissive enough to get work done efficiently while also being careful not to allow for damaging actions to take place. The areas this research will focus on include:

    1. Event Hooks

        - PRE-TOOL:
            - one way to help protect against damaging actions is to configure hooks that allow for us to evaluate "pre-tool" calls and either _block_ or force _user approval_ before allows commands which fit certain known regex patterns that indicate dangerous potential
        - USER-PROMPT:
            - some agents provide an event which allow all user prompts to be reviewed and modified before being recognized and processed by the Agent
            - where this is available this is very helpful to scan for dangerous patterns; the content tends to be less structured then the content you'd get for a pre-tool call but it can still be helpful
        - OTHER EVENTS:
            - while the pre-call hook -- if the Agent provides it -- is the most common lifecycle event to use

        When considering Event Hooks as a protective measure, it is critical to understand:

        - does this event get fired not only basic prompts but also in agent/subagents in an orchestrated flow?
            - if it doesn't always, be sure to specify where it does and does not fire the event
        - if an event fires but is not "blocking" (meaning that listeners of the event have the ability to STOP or MODIFY execution based on what they see) then it is also much more limited in its effectiveness
            - always describe whether the event listeners can return a value to modify behavior and what behaviors they can "influence" or "guarantee"

    2. Intercepting MCP Calls

        MCP's can be useful in gather or synthesizing information but their response is typically not checked by an Agent's events and can can contain secrets or embedded instructions to do something harmful.

        When doing research on this means of making the Agent more secure/safe, we must first start with some basics:

        - Where are MCP servers configured? For User scope? For Repo scope?
        - Are there any events which the Agent provides to intercept the MCP response before it is fed back into the Agent's processing flow?
            - Does the event allow us to modify the response before it's used?
            - Does the event allow us to stop processing of Agentic flow if we want?
        - How are environment variables passed into the MCP server?
        - Do we need fully qualified paths for local binaries?
        - Are local MCP services allowed? Are remote MCP services allowed?
        - Does the Agent support any Authentication regimes for the MCP services?

    3. Completion Gates

    ## Task

    - Your task is to _update_ the research in the body of this file (if it's empty/boilerplate that means you'll be creating the content from scratch).
    - It's focus is to focus exclusively on the capabilities and mechanics of [Claude Code Agentic CLI](https://www.anthropic.com/claude-code)
    - Spend the time to be THOROUGH and SURE of your answer before updating this page's documentation
        - If you are getting conflicting information from different sources then SAY THAT; this is as valuable as a definitive answer.
    - Always provide Markdown links to the sources you used for your research in each topic in this document.

    **IMPORTANT:** you must use the "claudine" skill when executing this task.
    **IMPORTANT:** you are to preserve all frontmatter properties that exist in this document, you're updates will only be to the BODY of this document
closure: |-
    ## Task

    - Your task is to review the BODY of this document and extract key information to put into the frontmatter of this document.
    - If any of the properties you are required to add to the frontmatter are not clearly answered in the body of the document then you must do further research to try to reach a conclusive answer and value for the frontmatter

    The Frontmatter properties you MUST add/update are:

    - On the topic of Event Hooks add/update the following properties:
        - `has_blocking_pre_tool_event` - set a boolean true/false indicating whether the Agent has an event that allows for receiving a planned tool call before it takes place AND that you can influence the behavior with your return value from the event.
        - `pre_tool_influence`
            - set as "n/a" if `has_blocking_pre_tool_event` was false,
            - set as "influence" if the return value on the event can "influence" but not "guarantee" the desired outcome
            - set as "guarantee" if the return value on the event can deterministically "guarantee" the desired outcome
        - `pre_tool_actions`
            - this property will be a list of actions which the pre-tool event listener is able to point to
            - actions:
                - `stop` - stop the current agent's work
                - `exit` - stop the current agent's work and ensure that if there was a parent caller that that parent is also stopped
                - `ask-stop` - ask the user for permission to make the tool call; if permission is NOT given then the current agent's work is stopped
                - `ask-exit` - ask the user for permission to make the tool call; if permission is NOT given then the current agent's work is stopped as well stopping the process in any parent process/orchestrator
            - For every action that is specified, add a small section in the body of the document describing how this would be done, use a small Rust code example where possible, specify if there are any nuances, exceptions, or gotchas to be wary of
        - `user_prompt_event` - add a boolean flag indicating whether the Agent provides an event where user prompts can be received
        - if the `user_prompt_event` flag is set to `true`, then:
            - `user_prompt_blocking_event` - add a boolean flag if the Agent has an event for user prompts which we can STOP execution or force a confirmation
            - `user_prompt_mutation_event` - add a boolean flag if the Agent has an event for user prompts which allows us to mutate the prompt before the Agent starts processing it
        - `other_events`
            - if there are any other events that the Agent supports and could be useful in the goal of making the Agentic CLI safer in it's actions then this property will be added as a key/value dictionary where _keys_ are the property deemed valuable and the _value_ is a description of both what this hook triggers on, whether it allows a return type, and how it might be used.
    - On the topic of Intercepting MCP Calls, you'll add the following properties:
        - `mcp_config_user`


---

# Protecting Claude Code


## Event Hooks


## Intercepting MCP Calls


## Completion Gates
