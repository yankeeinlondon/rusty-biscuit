---
title: Malformed Frontmatter
author: Test Author
date: 2026-01-01
tags:
  - markdown
  - testing
invalid_yaml: [unclosed array
nested:
  missing: colon here should break
    bad_indent: value
version: "1.0
published: maybe
---

# Document with Bad Frontmatter

This document has malformed YAML frontmatter that should trigger parsing errors:

1. Unclosed array in `invalid_yaml`
2. Malformed nested structure
3. Unclosed string in `version`
4. Invalid boolean value in `published`

The parser should either:

- Reject the frontmatter and treat the entire document as plain markdown
- Return a clear error indicating which line/field failed
- Fall back to treating everything after first `---` as content

## Edge Cases

What happens if:

- Frontmatter has duplicate keys?
- YAML contains tabs instead of spaces?
- The closing `---` is missing entirely?

These scenarios should be handled gracefully.
