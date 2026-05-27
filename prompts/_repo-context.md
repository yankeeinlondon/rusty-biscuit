---
name: Area Context
description: a snippet which will inject context about the "area" of the monorepo you are in
snippet: true
---

## Repo Context

You are working in the **{{ctx.repo}}** repo.

::block when="ctx.is_monorepo"
- this repo is a **monorepo** using the 
::end-block
