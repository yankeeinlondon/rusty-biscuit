---
agent: claude
$schema:
  spec: file(required;eager;match(deep/*.md))
---
Review {{ spec }}.
