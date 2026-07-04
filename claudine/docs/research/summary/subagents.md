---
sequence:
    - name: draft
    - name: iterate
    - name: finalize
prompt: |-
    Subagent definitions — named, specialized agents that a session can delegate work to, often with their own prompt, model, and tool restrictions — vary widely across agentic CLIs, from first-class definition files to nothing at all. Claudine links agent definitions across providers and also has to reason about subagent observability during wrapped runs.

    ## Task

    Your task is to report on agent/subagent definition support across the Agentic CLI providers Claudine supports.

    - your report should start by outlining why subagent definitions matter to agentic processes (delegation, context management, specialization)
    - and then shift its focus to how providers differ: definition format and metadata, user/repo scopes, model and tool restriction support, invocation mechanics, and what a wrapper can observe while a subagent runs
    - close with a point of view on the implications for Claudine's linking strategy and for wrapped-run observability

    As background material we have subagents research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/subagents/*.md`.

    Important: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.

    ::block when="state.name == 'draft'"
    - Iterate over the first three research documents to develop a point of view on how to write this document and then produce an initial draft of the document
    ::end-block
    ::block when="state.name == 'iterate'"

    - Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/subagents.md` (everything below the frontmatter); read it from there
    - Act as an orchestrator and iterate over each remaining provider's research document:
        - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned
    - Once every remaining provider has been incorporated, your final response is the fully updated draft
    ::end-block

    ::block when="state.name == 'finalize'"

    The document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/subagents.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.
    ::end-block
---
