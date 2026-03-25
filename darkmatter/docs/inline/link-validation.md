# Reference Validation

Reference validation checks that all references discovered in a document (and optionally its transcluded children) point to valid targets. Unlike most compose operations, validation does not mutate content -- it produces a diagnostic report.

## Scope

Validation covers all reference types discovered by the reference analysis subsystem:

- **Local paths** -- file existence check relative to each reference's own source location
- **Remote URLs** -- syntax validation (always), HTTP reachability (opt-in), including protocol-relative URLs like `//cdn.example.com/app.js`
- **Fragments** -- `#heading-slug` targets resolved against composed document headings (opt-in)
- **Cross-document fragments** -- `./other.md#section` validated by loading the target and checking its headings

References are extracted via graph traversal, so validation naturally covers the full transclusion tree when the document uses `::file`, `::code`, `::toc-linking`, or frontmatter prologue/epilogue directives.

For `::toc-linking`, validation checks the resolved target file dependency and also sees the generated markdown links that directive injects into the effective composed document.

## Path Resolution

Local path validation uses `biscuit_file::FileReference` for path resolution, supporting:

- Relative paths (`./sibling.md`, `../parent/file.md`)
- Repo-root `@` paths (`@docs/guide.md`)
- Simple path joins as fallback

Each reference is validated relative to its **own origin source**, not the root document. This means a child document's `[link](./local.md)` is resolved from the child's directory, not the root's.

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `graph` | `ReferenceGraphOptions` | default | Controls graph traversal depth, cache settings |
| `validate_remote` | `bool` | `false` | Check remote URLs via HTTP HEAD/GET |
| `remote_timeout` | `Duration` | 10s | Timeout per remote URL check |
| `validate_fragments` | `bool` | `false` | Resolve `#fragment` targets against headings |
| `fail_fast` | `bool` | `false` | Stop after first error-severity issue |

## Report

`ReferenceValidationReport` contains:

| Field | Description |
|-------|-------------|
| `references_scanned` | Total references checked |
| `references_valid` | Count that passed |
| `issues` | `Vec<ReferenceIssue>` with code, message, severity, origin |
| `warnings` | Non-blocking informational messages |

Helper methods: `is_valid()` (no error-severity issues), `error_count()`.

## Issue Codes

| Code | Severity | Meaning |
|------|----------|---------|
| `MissingLocalTarget` | Error | Local file does not exist |
| `InvalidUrl` | Error | URL is syntactically malformed |
| `RemoteUnreachable` | Error | HTTP check failed or timed out |
| `RemoteDisallowed` | Info | Remote validation disabled, cannot verify |
| `MissingSourceContext` | Warning | No source path available to resolve relative target |
| `UnsupportedScheme` | Info | Non-HTTP/HTTPS scheme (e.g., `ftp://`) |
| `MissingFragmentTarget` | Error | `#slug` not found in document headings |
| `MalformedHtmlTag` | -- | Reserved for future structured HTML validation |
| `MalformedCssImport` | -- | Reserved for future CSS parsing upgrade |
| `MalformedMetaTag` | -- | Reserved for future meta tag validation |

## CLI

The `darkmatter validate-refs` subcommand exposes reference validation with text, JSON, and graph (Mermaid/DOT) output formats. It returns a non-zero exit code when error-severity issues are found.

## Source Files

- `darkmatter/lib/src/markdown/reference/validate.rs`
- `darkmatter/lib/src/markdown/reference/graph.rs`
- `darkmatter/lib/src/markdown/reference/mod.rs`
- `darkmatter/cli/src/commands/validate_refs.rs`
