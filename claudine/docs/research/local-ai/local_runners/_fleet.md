---
sequence: "@claudine/docs/research/_local-runner-tasks.yaml"
initialize:
    - set:
          research_prompt: "@claudine/docs/research/local-ai/local_runners/_research.md"
agent: opencode
model: kimi-for-coding/k2p7
---

# Local Runners Research Fleet

Runs the topic prompt once per roster entry. Each prompt owns its freshness
checks and success validation. An unhandled failure stops the sequence; rerun
the fleet after correcting it to skip research already confirmed as current.
