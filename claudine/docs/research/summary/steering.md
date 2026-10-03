---
sequence:
- name: draft
- name: iterate
- name: finalize
prompt: |-
  Steering is sending one message to an agent session that is already running, so a person or Claudine can course-correct it without stopping it. Providers differ in whether a running session can be found at all, whether it exposes a writable channel, whether a message joins the active turn or waits for the next one, what a provider's answer proves about delivery, and whether reaching the agent requires interrupting it first.

  ## Task

  Your task is to report on steering support across the Agentic CLI providers Claudine supports.

  - your report should start by outlining why steering matters to a wrapper like Claudine: a person redirecting a session, and Claudine's automatic warning when an agent starts repeating itself before the repetition guard stops it
  - and then shift its focus to how providers differ: discovery of running sessions (Claudine-managed versus sessions started outside Claudine), control channels and launch profiles, operation effects (join the active turn, queue a follow-up, start an idle turn, interrupt then submit), delivery boundaries, receipt strength (accepted, queued, delivered, unknown), target guards against stale or replaced conversations, and known loss or duplication hazards
  - close with a point of view on which provider families (JSON-RPC over stdio, HTTP servers, ACP, provider registries) are the most promising next steering adapters and what evidence each still needs

  Standing correction (keep it satisfied on every regeneration): research outcomes are evidence, not activation. A provider is steerable through Claudine only where a reviewed grant exists in `claudine/docs/providers/steering-activation.yaml`, for an exact provider version, operating system, and launch profile, with a reviewed adapter revision. As of 2026-09-28 the only grants are Codex 0.157.1 on macOS, for Claudine-managed non-interactive runs through `codex app-server` (steer the active turn; start a turn when idle). Pi's managed RPC profile is implemented but blocked by a reviewed policy block, because extensions can switch the session without coordination. Every other provider is unavailable. Treat `claudine/docs/topics/steering-activation.md` and `claudine/docs/topics/steering-routing.md` as authoritative over anything the per-provider research documents imply about what Claudine can do today, and never describe a research `outcome: passed` record as a grant.

  As background material we have steering research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/steering/*.md` (skip `_fleet.md` and `_schema.yaml`).

  Important: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.

  ::block when="state.name == 'draft'"
  - Iterate over the first three research documents to develop a point of view on how to write this document and then produce an initial draft of the document
  ::end-block
  ::block when="state.name == 'iterate'"

  - Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/steering.md` (everything below the frontmatter); read it from there
  - Act as an orchestrator and iterate over each remaining provider's research document:
      - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned
  - Once every remaining provider has been incorporated, your final response is the fully updated draft
  ::end-block

  ::block when="state.name == 'finalize'"

  The document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/steering.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.
  ::end-block
---
