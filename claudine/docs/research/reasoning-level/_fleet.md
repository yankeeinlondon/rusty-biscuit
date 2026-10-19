---
sequence: "@claudine/docs/research/_provider-tasks.yaml"
research_prompt: "@claudine/docs/research/reasoning-level/_research.md"
---

# Reasoning Level Research Fleet

Runs the topic prompt once per roster entry. Each prompt owns its freshness
checks and success validation. An unhandled failure stops the sequence; rerun
the fleet after correcting it to skip research already confirmed as current.
