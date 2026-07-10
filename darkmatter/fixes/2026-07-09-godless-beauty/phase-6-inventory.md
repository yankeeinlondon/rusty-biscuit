# Phase 6 expression registration inventory

The pre-migration expression surface has three independently maintained authorities:

- `EXPRESSION_FUNCTION_DESCRIPTORS` in `catalog.rs` describes the public callable catalog.
- `PURE_FUNCTIONS` in `functions.rs` dispatches pure functions.
- `FS_FUNCTIONS` in `functions.rs` dispatches context-aware functions.

`and` and `or` are lazy callables dispatched directly by the evaluator. Runtime registrations
also carry aliases and signature strings, duplicating catalog data. `link`, `frontmatter`, and
`validate_schema` have catalog overloads sharing one context-aware handler.

Workspace consumers are Darkmatter evaluation, suggestions, catalog generation, and generated
documentation; DMLS expression hover/completion/lookup; and Claudine lifecycle-action expression
validation and context-command output. Active Darkmatter and Claudine architecture, engine,
drift, and authoring documentation also names the old constants.
