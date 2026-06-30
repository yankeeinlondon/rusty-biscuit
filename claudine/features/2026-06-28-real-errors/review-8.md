---
ready: false
implemented: true
agent: "codex/default"
created: "2026-06-29T11:27:39"
---

# Review 8

## Findings

### High - Deprecated lifecycle aliases do not alias the new facets

The spec requires the legacy lifecycle fields to remain available during migration, with `err.kind` and `err.variant` treated as deprecated aliases for `err.category` and `err.code` (`claudine/features/2026-06-28-real-errors/spec.md`:183-185). The implementation still exposes the old Rust implementation details instead: `LifecycleErrorInfo::to_value()` writes `kind` from the source error type and `variant` from the Rust enum arm before adding the new facet fields separately (`claudine/lib/src/composition/lifecycle_context.rs`:221-231). The tests lock that old behavior by expecting a `CompositionError::SchemaLoad` to project `err.kind == "CompositionError"` and `err.variant == "SchemaLoad"` while `err.category == "composition"` and `err.code == "composition.schema_load"` (`claudine/lib/src/composition/lifecycle_context.rs`:511-527).

Impact: an author using the migration aliases gets different values from the stable contract. That is the opposite of the required compatibility shape: `err.kind` should read as the deprecated spelling of `err.category`, and `err.variant` as the deprecated spelling of `err.code`, not as Rust internals.

Verification level present: Level 1 unit tests, but they assert the wrong alias semantics.

Required verification level: Level 1 lifecycle-context tests proving `err.kind == err.category` and `err.variant == err.code` for classifiable errors, plus a compatibility decision for facet-less legacy action failures.

### High - Public docs outside `lifecycle.md` still teach `err.msg`

The review-7 lifecycle-doc examples were corrected, and the new guard only scans `claudine/docs/topics/lifecycle.md` (`scripts/check-lifecycle-doc-facets.sh`:41-44). Other public Claudine docs still use `err.msg` in author-facing lifecycle documentation: `frontmatter-properties.md` presents `failure.message: "{{err.msg}}"` as the event-time interpolation example (`claudine/docs/topics/frontmatter-properties.md`:30), and `composition.md` explains dry-run deferred lifecycle keys using raw `{{err.msg}}` (`claudine/docs/topics/composition.md`:367).

The spec says new documentation and examples must use the new faceted names (`claudine/features/2026-06-28-real-errors/spec.md`:183-186). Leaving adjacent public docs on `err.msg` means users can still learn the deprecated stringly surface from the normal documentation path, and the guard will not catch it.

Verification level present: Level 1 grep guard, but only for `lifecycle.md`.

Required verification level: Level 1 doc guard covering every public lifecycle/composition topic that mentions lifecycle `err.*`, with the deprecated aliases allowed only in an explicit deprecated-alias section.

### Medium - `err.severity` is not projected even though the diagnostic contract models severity

The `Diagnostic` trait includes `severity()` and the registry stores an effective severity per code (`claudine/lib/src/diagnostics/mod.rs`:96-101, `claudine/lib/src/diagnostics/registry.rs`:29-43), but `DiagnosticFacets` and `LifecycleErrorInfo::to_value()` only project `code`, `category`, `disposition`, `origin`, and `detail` (`claudine/lib/src/composition/lifecycle_context.rs`:89-104, `claudine/lib/src/composition/lifecycle_context.rs`:226-252). The lifecycle docs likewise omit `err.severity` from the faceted field table (`claudine/docs/topics/lifecycle.md`:279-285).

If severity is intended to be part of the ratified handleability surface, it currently cannot be matched by handlers despite being part of the code registry. If severity is intentionally CLI/operator-only, the design/docs should say that explicitly so prompt authors do not expect it in `err.*`.

Verification level present: Level 1 tests cover the registry severity calculation, but I found no lifecycle projection test for `err.severity`.

Required verification level: Level 1 lifecycle-context projection test, or an explicit contract update documenting that severity is not exposed to lifecycle handlers.

## Notes

The review-7 stale-directory suggestion blocker appears addressed: there are Level 1 algorithm/detail tests for the dated-directory case and Level 2 real-terminal captures in both `md compose` and `claudine compose`.

Checks run:

- `env -u CDPATH scripts/check-error-transport.sh` passed.
- `scripts/check-lifecycle-doc-facets.sh` passed.
- `cd claudine && just lint-transport` passed.
- `cd claudine && just lint-lifecycle-doc-facets` passed.

I did not run the Level 2 terminal suites in this review pass.

Production ready: **no**.
