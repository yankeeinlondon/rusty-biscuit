---
status: draft
created: 2026-09-02
area: claudine
packages:
    - claudine
---

# Shipped plan-route fixture

A stable specification document for tests that run the repository's shipped
`prompts/plan.md` with a caller-supplied `spec` file parameter. Tests must
point at this fixture rather than at an active fix directory, because fix
directories move into `_completed/` when a cycle closes.
