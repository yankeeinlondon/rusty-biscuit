# Isolation Functions

The functions in this section are intended to isolate certain "scopes" of a document. The "action" we provide will determine whether we are returned a vector of results, or a single string (with optional delimiting).

Currently we focus on both Markdown and HTML documents:

- `md_isolate(content, scope, action)`
- `html_isolate(content, scope, action)`

> Note: the "scopes" used here are the same scopes we use in the Interpolation area for context-aware interpolation.
