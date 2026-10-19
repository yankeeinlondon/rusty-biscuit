---
sequence: "@claudine/docs/research/_provider-tasks.yaml"
research_prompt: "@claudine/docs/research/agent-logging/_research.md"
agent: opencode
model: zai-coding-plan/glm-5.2
---

# Agent Logging Research Fleet

Runs the topic prompt once per roster entry. Each prompt owns its freshness
checks and success validation. An unhandled failure stops the sequence; rerun
the fleet after correcting it to skip research already confirmed as current.
