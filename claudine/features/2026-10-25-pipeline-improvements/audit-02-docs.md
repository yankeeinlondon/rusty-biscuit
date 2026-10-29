---
kind: implementation-log
spec: 2026-10-12-shared-type-tables
---

# Canonical Documentation Audit

Existing canonical documents updated: **1**. New canonical documents created:
**0**.

- `darkmatter/docs/topics/schemas/definition.md`, under “Shared Tables and
  Independent Imports,” now describes immutable shared tables, first and later
  lookups, local-reference ownership, declaration order, independent modifiers,
  the root table, and the no-import fast path. A complete two-file example
  demonstrates a required import alongside an optional import of the same type.
- The same section distinguishes the expansion-local namespace cache from the
  bounded process-wide parsed-file cache. The previous note contradicted
  itself: it described reuse across resolutions and then claimed no state
  survived a call. The replacement matches the current implementation, which
  reads file contents before consulting the parsed-file cache and retains only
  successful parses. The original specification snapshot is unchanged.

The document already has `kind: canonical-documentation` from the preceding
audit; that value is preserved. The completed spec now lists the document in
`canonical_docs` using its implicit repository-relative reference. The snapshot
body was not changed. No additional canonical page is needed because the
schema definition page already owns named imports and resolution behavior.

Validation: compared the documentation with `Namespace`, `CachedNamespace`,
`ImportEngine::resolve_namespace`, `expand_imports`, and `load_named_types` in
`darkmatter/lib/src/markdown/schemas/resolve.rs`. Checked the new example against
the existing independent-use resolution tests. `git diff --check` passes.
No feature/fix links or names were added to the canonical document.

Stale source comments claiming expansion-only parsing and nonrecursive type
graphs were reported to the test agent, who owns code edits for this spec.
