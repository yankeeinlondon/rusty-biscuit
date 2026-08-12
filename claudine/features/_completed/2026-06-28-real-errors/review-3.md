---
ready: false
implemented: true
agent: "codex/default"
created: "2026-06-29T02:55:27"
---

# Review 3

## Findings

### High - The flagship invalid-file-reference report is still not verified through `claudine compose` at Level 2

The spec requires the reference failure to render identically in `md compose` and `claudine compose`, including the root-cause `invalid file path` headline, receiving key, OSC8-linked prompt file, focused `$schema` / `spec` / `iteration` excerpt, and likely-file suggestions (`spec.md`:170-173; `integrated-design.md`:509-512).

The current implementation has a Darkmatter Level 2 capture for `md compose` (`darkmatter/cli/tests/level2_errors.rs`:320), plus unit tests for the Darkmatter interpolation block (`darkmatter/lib/src/markdown/errors/blocks.rs`:620 and `:687`). I did not find an equivalent `claudine compose` Level 2 invalid-file-reference capture. The only new Claudine Level 2 real-terminal coverage I found is for schema parsing (`claudine/cli/tests/level2_schema_parse_capture.rs`:128), not the reference failure. `rg` also finds no Claudine CLI test exercising `frontmatter(spec, 'review_iterations')` or `composition.invalid_file_reference` through `claudine compose`.

There is also a narrower Level 2 hole on suggestions: the `md compose` L2 fixture uses `does-not-exist-spec.md` without creating a near sibling, and the test asserts headline/key/excerpt/link but not `Did you mean?` output (`darkmatter/cli/tests/level2_errors.rs`:342-365). The suggestion path is currently verified at Level 1/unit only (`darkmatter/lib/src/markdown/errors/blocks.rs`:620-644).

Verification level mismatch: this is terminal-rendered, user-observable behavior. Per the review rubric, `claudine compose` needs Level 2 capture for the exact reference report, and suggestions need Level 2 capture with a real sibling candidate before this requirement can be considered production-ready.

### High - The lifecycle `err.*` projection is missing the promoted handleability fields promised by the catalog

The ratified handleability contract promotes high-traffic values to top-level `err.*`: `err.reset_at`, `err.retry_after_ms`, `err.is_transient`, `err.is_throttled`, and `err.is_correctable` (`error-catalog.md`:174-180). That is part of the author-facing lifecycle surface, not just documentation.

`LifecycleErrorInfo::to_value()` currently projects the legacy aliases plus `code`, `category`, `disposition`, `origin`, and `detail` only (`claudine/lib/src/composition/lifecycle_context.rs`:203-220). There is no top-level insertion for `reset_at`, `retry_after_ms`, or the predicate sugar. A code search finds those names in provider/rate-limit internals and registry rows, but not in the lifecycle `err` object construction.

Impact: handlers written against the ratified public contract cannot use the documented terse forms, and throttled recovery rules cannot simply key off `err.reset_at` / `err.retry_after_ms`. They must reach into detail when it happens to exist, while the catalog explicitly says the promoted fields should be available and null/false when not applicable.

Verification level: Level 1 is appropriate here. Add focused lifecycle projection tests that build throttled, transient, correctable, and non-matching errors and assert the promoted fields and predicates on `LifecycleErrorInfo::to_value()`.

### Medium - The `claudine errors` discoverability surface is not implemented

The catalog says the error contract is discoverable through a `claudine errors` introspection surface that lists every code and its detail schema (`error-catalog.md`:195-202). The CLI command enum has no `Errors` command; it ends with `Context` after the composition commands (`claudine/cli/src/args.rs`:55-112). I also found no command module that renders `diagnostics::CODES`.

Impact: the code registry exists, but prompt authors cannot discover the stable `err.code` and `err.detail.*` contract from the CLI as designed. That makes the handleability feature incomplete even though several `Diagnostic` implementations are now present.

Verification level: Level 1 CLI tests are sufficient for the command shape and rendered rows; Level 2 is not required unless the output layout/styling becomes part of the user-facing contract.

### Medium - The DM↔Claudine boundary guard is still not enforced, and two named lossy sites remain

The integrated design requires `MarkdownError` / `BlockError` values crossing into Claudine to travel by `#[from]` / `#[source]`, and calls out a grep-style boundary lint to prevent reintroducing `Variant(String)` and `map_err(|e| e.to_string())` flattening (`integrated-design.md`:393-412). It also names specific sites to convert.

Two of those named sites still flatten errors:

- `claudine/lib/src/composition/closure.rs`:146-147 still maps atomic-write failure to `CompositionError::AtomicWriteFailed(e.to_string())`.
- `claudine/lib/src/composition/lifecycle_control.rs`:244-245 still maps `resolve_harness_path` failure to `String`.

I also did not find a repo/package test enforcing the boundary lint. Some legacy `String` variants may be acceptable during migration, but the design specifically required these boundary smells to become executable guardrails.

Impact: this does not break the flagship interpolation render path, but it leaves the structural anti-regression protection incomplete and continues to lose typed error detail on lifecycle/closure failures.

Verification level: Level 1 is appropriate. Add a focused lint/guard test or script in the package test path, then either convert the named sites or explicitly exempt non-boundary cases.

## Summary

This iteration fixed several review-2 blockers: Darkmatter now composes interpolation blocks from `SourceRef`, schema parse excerpts have Claudine Level 2 coverage, fatal file-reference behavior is documented as the ratified exception, and the diagnostic facet implementations are much broader than before.

The feature is still not production-ready. The exact reference report is not Level 2-verified through `claudine compose`, the lifecycle `err.*` surface is missing promised promoted fields, `claudine errors` is absent, and the boundary lint/conversion work is incomplete.

Production ready: **no**.
