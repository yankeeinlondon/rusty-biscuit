# Phase 4 cleanup baseline

- Public module path: `darkmatter::markdown::cleanup`.
- Public entry points: cleanup default/compact/loose variants, forced-indent variants,
  incidental-newline stripping, fixed-width cleanup/reflow, and the three public mode/style types.
- Cleanup test inventory before relocation: 149 library tests whose names contain
  `markdown::cleanup::tests::`.
- Event stage order: parse with offsets; capture list markers; preserve emphasis with coupled
  placeholders; add `text` to empty fences; align tables; serialize with cmark.
- String stage order: restore and unescape emphasis; normalize list spacing; repair blockquotes;
  restore list markers; normalize indentation; unescape brackets; normalize trailing newlines.
- DMLS parity test: `providers::formatting::tests::test_format_text_is_byte_equivalent_to_library_cleanup`.

The relocation may change owning test-module prefixes, so inventory comparison uses both total
count and test-function basename sets.
