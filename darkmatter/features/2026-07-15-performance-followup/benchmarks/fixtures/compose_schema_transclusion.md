---
$schema:
  title: string
  status: enum(draft, ready)
title: Schema and Transclusion
status: draft
---

# {{ title }}

An inline `$schema` validates and coerces this frontmatter, then a transclusion
directive pulls in a child document:

::file ./compose_child.md

Closing prose after the transclusion.
