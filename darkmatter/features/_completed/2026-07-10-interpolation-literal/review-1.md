---
ready: false
implemented: true
agent: codex/default
created: 2026-07-10T17:12:28
---

# Review 1: Interpolation Literal

## Verdict

Not ready for production. The scanner and compose rewrite cover the core syntax well, but the required DMLS behavior is not verified through the public provider entry points, and literal hover rendering does not safely represent all content that the scanner accepts.

## Findings

### High — Required DMLS behavior is tested only below the provider boundary

The specification requires three user-observable DMLS outcomes: no `dm.expression.*` diagnostic, a literal hover with the composed output, and no interpolation node or `uses_variable` edge. The new tests do not exercise those outcomes end to end:

- `literal_span_not_seen_by_expression_diagnostics` calls `expressions::interpolations` rather than `diagnostics`, so it cannot detect provider routing, source-map, range, or diagnostic-code regressions.
- The hover tests call `literal_hover_markdown` directly rather than `hover`, so they do not verify cursor lookup, body/frontmatter boundaries, the returned hover range, or LSP markup lowering.
- The substrate test checks only `variable_uses`; it does not build/query the workspace graph and assert that no `NodeKind::Interpolation` node and no `uses_variable` edge exist.

These are all Level 1 requirements: they are deterministic LSP/graph operations and do not depend on a real terminal. Add provider-level L1 tests using a real `DocumentContext` and source map, including UTF-16 positions, plus a workspace-graph assertion for both the node and edge. Until then, acceptance criterion 13 is not demonstrated at the appropriate boundary.

Relevant code: `darkmatter/dmls/src/providers/dsl.rs:1426`, `darkmatter/dmls/src/graph/substrate.rs:725`.

### Medium — Hover Markdown breaks for valid literal content containing backticks or line breaks

Literal content is defined as arbitrary preserved bytes up to the first `}}}`, but `literal_hover_markdown` inserts the composed output into a single-backtick Markdown code span without escaping or choosing a safe fence. For example, `{{{ use `x` }}}` produces nested backticks, and multiline content produces an invalid inline code span. The hover therefore may not show the required composed output accurately for valid literals.

Render the output with a fence length greater than the longest backtick run (or a fenced code block for multiline content), and add provider-level tests for embedded backticks, multiline content, and Unicode.

Relevant code: `darkmatter/dmls/src/providers/dsl.rs:304`.

### Medium — Full compose-pipeline coverage is missing for the most load-bearing frontmatter ordering

The implementation has strong helper-level L1 tests, but the two-pass frontmatter test manually invokes `interpolate_frontmatter`, mutates the shell value itself, and invokes the helper again. It does not run the actual compose pipeline that brackets shell expansion, whose second pass is conditionally dispatched. Consequently, acceptance criterion 10 is not protected against orchestration regressions in operation ordering or pass dispatch.

Add a Level 1 compose integration test using an approved deterministic frontmatter shell command and assert the final typed string, replacement counts, and absence of warnings. No Level 2 or Level 3 test is needed because neither terminal rendering nor OS input is involved.

Relevant code: `darkmatter/lib/src/markdown/compose/pipeline/mod.rs:263`, `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:1760`.

## Requirement Verification Matrix

| Requirements | Strongest verification found | Assessment |
|---|---:|---|
| Scanner recognition, maximal munch, unclosed fallback, adjacency, nesting, empty forms, code-region behavior (AC 3–9) | L1 scanner/rewrite unit tests | Appropriate level; helper coverage is good. |
| Body conversion, replacement-introduced literals, replacement count, `fail_fast` (AC 1, 2, 11, 14) | L1 rewrite unit tests | Appropriate level, though a CLI/library compose integration smoke test would better protect wiring. |
| Frontmatter text typing and two-pass shell ordering (AC 10) | L1 helper tests with manually simulated shell expansion | Level is appropriate, but the tested boundary is insufficient; finding above. |
| Context capture and remote discovery inertness (AC 12) | L1 consumer unit tests | Appropriate level. |
| DMLS diagnostics and hover (AC 2, 13) | L1 scanner/hover-string helper tests | Appropriate level, wrong boundary; high-severity gap. |
| DMLS graph node and edge inertness (AC 13) | L1 substrate fact test | Appropriate level, incomplete assertion; high-severity gap. |
| Terminal-dependent behavior | None specified | L2/L3 are not applicable to this feature. |

## Positive Notes

The shared scanner exposes literals as a distinct product, checks triple braces before expressions, preserves the legacy fallback, and converts literals only after body rescanning. The implementation also routes context capture, remote discovery, and DMLS indexing through the shared recognition behavior rather than duplicating the grammar. Required user documentation was updated.

## Validation

`just test` passed from `darkmatter/` using the prescribed nextest workflow: 5,294 library tests, 545 CLI tests, and 399 DMLS tests passed (the library and CLI runs skipped their configured non-L1 tests). L2/L3 were not run because no requirement depends on a real terminal or OS input. The green L1 suite does not close the boundary and hover-content findings above.
