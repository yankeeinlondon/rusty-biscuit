---
sequence:
    - name: draft
    - name: iterate
    - name: finalize
prompt: |-
    The MCP protocol is an important standard that all Agentic CLI's support to one degree or another and because it is a real standard the MCP servers themselves can provide their services in a largely Agent neutral manner. However "the last mile" problem always provides small variance in how MCP is configured, packaged, or enabled in each agent.

    ## Task

    Your task is to report on the support for MCP in Claudine, focusing on the variants imposed by the Agentic CLI providers Claudine supports.

    - your report should start by outlining the key benefits that MCP provides to agentic processes
    - and then shift it's focus to how Claudine's supported providers support (or don't support) various aspects of MCP. 
    
    As background material we have MCP research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/mcp/*.md`. 

    ::block when="state.name == 'draft'"
    - Iterate over the first three reasearch documents to develop a point of view on how to write this document and then create an initial draft of the document
    ::end-block
    ::block when="state.name == 'iterate'"

    - Note: the initial draft document has been created (see below)
    - Act as an orchestrator and iterate over each remaining provider's research document:
        - provide the subagent the current draft and ask them to iterate on the draft based on the research document they've been assigned
    - Be sure that as a final step you save the updated draft to the body of this document

    # Draft Document
    ::file @claudine/docs/research/summary/mcp.md
    ::end-block
    
    ::block when="state.name == 'finalize'"

    The document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. Make any adjustments to the current draft (below) and this will be considered the finalized summary document.
    
    # Draft Document
    ::file @claudine/docs/research/summary/mcp.md
    ::end-block
---
