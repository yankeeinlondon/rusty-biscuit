---
ready: true
agent: codex
model: ""
---

# Review: `::toc-linking` Indentation Preservation

## Verdict

Not ready for production.

The direct renderer and test-only expansion helper now preserve leading whitespace, and the current Level 1 tests pass. However, the production compose path can still emit incorrectly indented output when the operation cache is reused, and one acceptance criterion is not actually tested or fully implemented.

## Findings

### High: cached TOC output includes caller-specific indentation but the cache key excludes indentation

Requirement 1 and requirement 2 require each directive expansion to use that directive's container/directive indentation. In the production compose path, the cache key is computed only from the target path and `TocLinkingOptions`:

- `darkmatter/lib/src/markdown/compose/mod.rs:1739`
- `darkmatter/lib/src/markdown/compose/mod.rs:1742`
- `darkmatter/lib/src/markdown/compose/cache/operation.rs:179`
- `darkmatter/lib/src/markdown/compose/cache/operation.rs:223`

But the cached content is rendered with `directive.indent` and `directive.inferred_indent` inside the `get_or_compute_operation` closure:

- `darkmatter/lib/src/markdown/compose/mod.rs:1767`
- `darkmatter/lib/src/markdown/compose/mod.rs:1772`

On a cache hit, the code returns `cached.content.clone()` directly:

- `darkmatter/lib/src/markdown/compose/mod.rs:1786`

That means two `::toc-linking` directives pointing at the same file with the same options but different indentation can reuse the first directive's rendered whitespace. This is especially likely in exactly the documented scenario: one root-level TOC and one nested TOC for the same target, or two nested TOCs at different depths.

The fix should cache an indentation-independent rendered link block and apply indentation after cache lookup, or include indentation in the cache key. The former is more consistent with the cache operation model because indentation is caller-local post-processing, not source/option-derived content.

Verification needed: Level 1 integration test through `Markdown::compose_with`, with cache enabled, containing at least two same-target/same-options directives at different indentation levels. Assert both rendered locations have the correct indentation.

### High: the "column 1 inside a list item continuation" acceptance criterion is not actually covered

Acceptance criterion 2 asks for a fixture where `::toc-linking` is at column 1 inside a list item continuation. The current test named `indented_toc_linking_at_column_one_in_list` does not exercise that case:

- `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs:411`
- `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs:422`

The directive in that test is written with two leading spaces:

```md
- Item
  ::toc-linking ./api.md
```

So the test only verifies ordinary directive indentation capture. It does not verify the parser's inferred-indent path in a list continuation. The parser's fallback only copies leading whitespace from the previous non-empty line:

- `darkmatter/lib/src/markdown/compose/toc_linking/parser.rs:35`
- `darkmatter/lib/src/markdown/compose/toc_linking/parser.rs:59`

For a true lazy-continuation case such as:

```md
- Item
::toc-linking ./api.md
```

the previous non-empty line has no leading whitespace, so `inferred_indent` is `None` and the generated bullets remain at column 1. That breaks requirement 3 and recreates the sibling-list corruption described in the spec.

Verification needed: Level 1 compose fixture with a true flush-left directive in a CommonMark lazy-continuation list item, followed by a pulldown-cmark roundtrip assertion that the generated TOC entries are children of the outer item, not siblings.

## Test Rigor

This feature is a Markdown composition/output correctness change, not terminal input, terminal rendering, or OS keyboard behavior. Level 1 in-process integration tests and CommonMark parser roundtrip assertions are the appropriate minimum verification level. No Level 2 or Level 3 coverage is required for this spec.

Current strongest coverage is Level 1. The Level 1 suite includes renderer tests, parser tests, the test-only `process_toc_linking` pipeline helper, and a CommonMark roundtrip assertion. The gaps are not test level mismatches; they are missing production-path and true column-1 fixtures.

Command run:

```sh
cargo test -p darkmatter toc_linking --color=never
```

Result: passed. The run produced existing deprecation warnings from `darkmatter/lib/tests/layout_snapshots.rs`.

## Notes

The implementation notes that `::file` and `::code` share the broader indentation issue but are not trivially co-located with this fix. That satisfies requirement 5's "verify/flag" scope for this iteration.
