---
sequence: "@claudine/docs/research/_provider-tasks.yaml"
research_prompt: "@claudine/docs/research/non-interactive-sessions/_research.md"
agent: codex
model: gpt-5.6-sol
---

# Non Interactive Sessions Research Fleet

Runs the topic prompt once per roster entry. Each prompt owns its freshness
checks and success validation. An unhandled failure stops the sequence; rerun
the fleet after correcting it to skip research already confirmed as current.
