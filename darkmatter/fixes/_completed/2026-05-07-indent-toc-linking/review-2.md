---
ready: true
agent: codex
model: ""
---

# Review: `::toc-linking` Indentation Preservation

## Verdict

Ready for production.

I did not find any remaining blocking gaps against the spec. The two issues from the prior review have been addressed: the production compose path now caches indentation-independent TOC output and applies caller-local indentation after cache lookup, and the column-1 lazy-continuation case is now tested with a true flush-left directive.

## Findings

No blocking findings.

## Requirement Coverage

- Requirement 1, indent emitted list lines to the directive/container context: covered by Level 1 renderer tests and `process_toc_linking` fixtures for nested list indentation.
- Requirement 2, preserve the directive's own indentation as the minimum output indentation: covered by Level 1 parser and renderer tests for explicit spaces and tabs, plus multi-directive indentation fixtures.
- Requirement 3, column-1 directive inside an indented list continuation: covered by a Level 1 fixture using `- Item\n::toc-linking ./api.md\n`, with a `pulldown-cmark` round-trip assertion that TOC links are nested under the outer list item.
- Requirement 4, do not change link generation: existing Level 1 tests still assert link text, slug, file path, filtering, and cleanup behavior while the indentation change is isolated to line prefixes.
- Requirement 5, verify `::file` and `::code`: the implementation plan and compose code document that those directives share the broader indentation issue but are not trivially co-located with this fix. That satisfies the spec's flag/defer allowance.

## Test Rigor

This is Markdown composition correctness, not terminal rendering or terminal input behavior. Level 1 in-process tests and CommonMark parser round-trip assertions are the appropriate verification level; Level 2 and Level 3 are not required for this spec.

Strongest verification present is Level 1. That matches the user-observable requirements here because the observable artifact is generated Markdown text and its CommonMark structure.

Command run:

```sh
cargo test --color=never -p darkmatter toc_linking --lib
```

Result: passed, 66 tests passed.

## Notes

The cache fix takes the right shape: cached TOC operation output is rendered with no indentation, then `indent_text` is applied per directive after cache lookup. That avoids making a caller-local formatting concern part of the persistent operation cache key.

One small follow-up worth keeping in mind, but not blocking this feature: the broader `::file`/`::code` indentation issue should get its own focused spec because those directive types need indentation captured earlier in the transclusion pipeline.
