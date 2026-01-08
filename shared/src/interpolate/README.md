# Interpolation

In this part of the repo you'll find various _interpolation_ functions for doing A --> B string replacements. The functions include:

## Context Unaware

The _context unaware_ interpolations are simple text replacements where the replacement is done with no contextual awareness of the content.

- `interpolate(find, replace)` - string-to-string replacement
- `interpolate_regex(find, replace)` - regex replace

## Context Aware

In the context aware interpolations we bring in the idea of a "scope" which helps us isolate areas of the document we are targeting.

- `md_interpolate(scope, find, replace)`
- `html_interpolate(scope, find, replace)`

In this form of interpolation, the only parts of the document which are considered when doing the interpolation are those sections which are "in scope".

For example, in Markdown we might have scopes like:

- `Frontmatter`
- `Prose`
- `Headers`
- `BlockQuotes`
- etc.

See the `MarkdownScope` enumeration for an up-to-date list.
