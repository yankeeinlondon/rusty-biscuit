---
kind: implementation-log
---

# Documentation audit: 2026-10-15-diagnostic-node-positions

Updated **3 existing canonical documents**; created **0 documents**. The
completed specification now lists five canonical documents using implicit
repository-relative references. Every referenced document declares
`kind: canonical-documentation`; the specification body is unchanged.

| Canonical document | Requirements covered | Change in this step |
| --- | --- | --- |
| `darkmatter/docs/topics/schemas/definition.md` | Category-specific node starts, original-source positions after replacement, decoded pointer keys, scalar boundaries, fallbacks, public position-map APIs | Added concrete replacement and new-path examples, decoded-key guidance, snapshot reuse and passive lookup contract, and the editor-coordinate link. Removed the inaccurate blanket exclusion of anchors. |
| `darkmatter/docs/inline/schema-validation.md` | CLI output, complete-file line numbers, byte columns, JSON records and exit behavior | Reviewed; already current and canonical. Its location guide links to the full rules. |
| `darkmatter/docs/errors/file-reference-failures.md` | File-reference errors point at the value's start, with the structured failure class retained | Added canonical kind; existing example is current. |
| `darkmatter/dmls/docs/diagnostics.md` | Editor range targets and CLI/editor source-byte parity | Added the difference between one-based CLI byte columns and zero-based negotiated LSP coordinates, including a non-BMP Unicode example. |
| `darkmatter/docs/topics/schemas/dmls-schema-support.md` | Shared validation behavior, node-level editor diagnostics, negotiated encoding | Reviewed; already current and canonical. |

The schema definition page's **Problem Locations** section is the canonical
entry point, with examples and its existing Mermaid fallback flow. No new page
is needed. The README already links directly to that section and remains an
overview.

## Drift maintenance

The source locator supports an anchor without an alias; only an alias prevents
trusted projection. The test agent confirmed this with a new public regression
for `m: &entry` containing a nested wrong value. The docs and skill now avoid
claiming that every anchor forces top-level fallback.

Separately, updated `.claude/skills/darkmatter/schema.md` to match the current
named-union behavior identified in the preceding audit: authored arm defaults
are scalars, while the Rust API can supply actual JSON-array defaults; array
items get no automatic null wrapper but an explicit null-admitting arm can
accept null. Shared numeric normalization governs default equality. Corrected
the corresponding remaining null-item sentence in the definition page. The
skill is not counted as a canonical document.

## Validation

Compared the rules with `PositionMap::project`, `locate_in`, and
`matches_orders` in `schemas/positions.rs`, `positions_for` in
`schemas/mod.rs`, the original-source mutation tests, the CLI assignment
test, and DMLS's encoding-aware parity tests. All five references exist under
a package area's `docs/` folder and have the requested kind. Confirmed the
completed specification's body is byte-for-byte equal to HEAD. Cross-reference
targets and anchors exist; `git diff --check` passes. No feature/fix reference
was added to canonical prose. Tests are owned and run by the concurrent test
agent; this documentation slice ran no test suite.
