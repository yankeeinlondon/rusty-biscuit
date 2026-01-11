---
title: Document with Frontmatter
author: Test Author
date: 2026-01-01
tags:
  - markdown
  - testing
  - rust
version: 1.0
published: true
metadata:
  category: documentation
  priority: high
---

# Document with Frontmatter

This document demonstrates YAML frontmatter parsing.

## Frontmatter Fields

The frontmatter above contains:

- **title**: String field
- **author**: String field
- **date**: Date field (as string)
- **tags**: Array of strings
- **version**: Numeric field
- **published**: Boolean field
- **metadata**: Nested object with key-value pairs

## Content Body

This is the main content body of the document. The frontmatter should be
parsed separately from the markdown content.

### Code Example

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Frontmatter {
    title: String,
    author: String,
    date: String,
    tags: Vec<String>,
    version: f64,
    published: bool,
}
```

## Notes

- Frontmatter must be at the start of the file
- Delimited by `---` before and after
- Uses YAML syntax
- Should not interfere with markdown content parsing
