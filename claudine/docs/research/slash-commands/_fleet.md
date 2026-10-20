---
sequence: "@claudine/docs/research/_provider-tasks.yaml"
research_prompt: "@claudine/docs/research/slash-commands/_research.md"
agent: opencode
model: kimi-for-coding/k2p7
---

# Slash Commands Research Fleet

Runs the topic prompt once per roster entry. Each prompt owns its freshness
checks and success validation. An unhandled failure stops the sequence; rerun
the fleet after correcting it to skip research already confirmed as current.
