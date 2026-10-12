# Cleanup rules and frontmatter repair

Cleanup (`md clean`, `md FILE --save`, DMLS `textDocument/formatting`) is
being rebuilt so it never changes a document's meaning. These surfaces are in
place; the user-facing record is the area's `docs/` tree.

- `markdown::cleanup::rules` is the single validator for `style.markdown`
  (document), DMLS `formatting.markdown` (workspace), and the style parser's
  registration. Read values into `Leaf<T>` (absent, null, value); never
  through `#[serde(default)]` options. `resolve` picks each rule
  independently: CLI, document, workspace, runtime default. The baseline
  schema's booleans are literal unions so schema coercion cannot accept a
  quoted value the validator rejects; `base_schema_end_to_end` pins that
  agreement.
- `markdown::cleanup::frontmatter::repair_frontmatter` is the raw-source repair
  both consumers call. Its schema configuration is a lazy closure, so absent or
  empty frontmatter never builds a context. Clean diagnostics
  (`cleanup::diagnostic`) use document coordinates.
- `markdown::cleanup::clean_body(body, &ResolvedMarkdownRules)` is the guarded
  body engine. It never re-serializes Markdown: rules propose bounded byte
  edits against one parse, and one final comparison accepts the candidate or
  returns the input (`CleanOutcome::Declined` plus a `clean.fidelity-fallback`
  finding; `BodyCleanup::fallback` re-anchors it into a document with
  `Fallback::diagnostic(document, body_offset)`). Modules, in pipeline order:
  `protect` (directive lines, quoted `::shell`, and shell-block regions are
  removed from a parse view; expressions are inline-protected), `structure`
  (the block model in source coordinates), `transform` (nine body rules),
  `wrap` (fixed-width, a second stage), `edits` (apply with overlap and
  protection checks), `compare` (the guard).
- Rule-writing traps the corpus found, each now handled:
  pulldown-cmark strips a lazy continuation line's indentation only up to the
  containers it still satisfies (moving it changes code-span text); items of
  one list can sit at different columns; a quote's bare `>` line must move with
  its quote; `ExpressionFinder` can match an "expression" across many lines,
  so rules leave any structure such a span crosses as authored; a bare `|`
  inside a code span still splits a table cell. A rule that cannot apply
  safely leaves the structure as written; the guard is the backstop, and a
  refusal on valid syntax is a rule bug to fix, not an acceptable outcome.
- The `cleanup_content*` wrappers, `cleanup_to_fixed_width`, and
  `Markdown::cleanup*` still run the older unguarded serializer pipeline
  (`lists`, `emphasis`, `brackets`, `blockquote`, `tables`), and `md clean`
  and DMLS still call them; do not extend that pipeline, route callers to
  `clean_body`. Ambient `PREFER_ITALICS` restyling is gone from both;
  `context_construction_guard` no longer allowlists an environment read in
  `cleanup/emphasis.rs`.
- Comparison keys never use `Debug` of pulldown's `CowStr` (its storage
  variant differs for equal values); `compare::tag_key`/`event_key` spell
  fields out.
- Tests: `cleanup/tests/{fidelity_repro,directive_fixed_points,rules,guard,source_fidelity}.rs`
  plus `parse_count::guarded_body`. `test_hooks::install_broken_pass` (built
  under `cfg(test)` or the `cleanup-test-hooks` feature) corrupts the candidate
  before the guard, so consumers can prove a refusal end to end.
