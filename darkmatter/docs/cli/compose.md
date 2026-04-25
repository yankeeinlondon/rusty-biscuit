## Overview

The `compose` command runs markdown through the transform pipeline and outputs the composed result.

It is used for document assembly workflows like:

- interpolation (`{{ ... }}`)
- replacement transforms
- transclusion (`::file`, `::code`, and related directives)

Unlike `read`, `compose` focuses on transformed content, not display rendering defaults.

## Reporting

### Usage

```bash
# Compose a file
md compose doc.md

# Compose stdin
cat doc.md | md compose
md compose -

# Provide default values as JSON or JSON5
md compose doc.md --state '{"name":"Alice","env":"prod"}'
md compose doc.md --state '{name: "Alice", env: "prod"}'

# Override values using --set
md compose doc.md --set '{"name":"Bob"}'

# Override values using shorthand setters
md compose doc.md name=Bob iteration=1
md compose - name=Alice draft=false

# Include frontmatter in output
md compose doc.md --frontmatter
md compose doc.md --fm

# Render composed output as HTML or JSON
md compose doc.md --output html
md compose doc.md --output json

# Show output via temp artifact
md compose doc.md --show

# Report shell commands that compose could execute, without running them
md compose doc.md --shell
```

### Shorthand Setter Syntax

Positional arguments matching `key=value` are treated as override setters, equivalent to adding the key-value pair to `--set`. This provides a more ergonomic syntax for common overrides:

```bash
md compose doc.md iteration=1 draft=false name=Alice
```

**Value parsing**: Values are parsed as JSON5 with a string fallback. This means:
- Numbers (`iteration=1`) and booleans (`draft=false`) are parsed as their native types
- Objects and arrays can use JSON5 syntax: `meta={author:"Alice"}`
- Unquoted strings are treated as literal strings: `name=Alice`
- Empty values (`empty=`) resolve to an empty string

**Key grammar**: Keys must match `[A-Za-z_][A-Za-z0-9_-]*`. Tokens with `/`, `:`, or other path-like characters in the key portion are treated as input paths rather than setters. This means `./foo=bar.md` is correctly classified as a file path.

**Limitation**: Shorthand setters only set top-level keys. For nested paths, use `--set` with dot-notation or JSON objects.

### Arguments

- `[ARGS]`: Positional arguments — an optional input file path followed by zero or more `key=value` setters. Use `-` for stdin. If no path is provided, reads stdin when piped; otherwise errors.

### Options

- `--state <JSON>`: default values as JSON or JSON5; fills in null/missing frontmatter keys without overriding existing values.
- `--frontmatter` / `--fm`: include frontmatter in the output (default: body only).
- `--output <markdown|text|html|json|ast|auto>`: output format (default: `markdown`).
- `--show`: open output via temp artifact.
- `--allow-missing-hyperlinks`: allow broken hyperlink targets; emit composed content to stdout and report issues on stderr with exit code 0.
- `--allow-missing-image-refs`: allow broken image reference targets; same behavior as above for images.
- `--allow-missing-transclusions`: allow missing transclusion targets; the directive is removed from the output and issues are reported on stderr with exit code 0.
- `--allow-any-missing-reference`: combines all `--allow-missing-*` flags.
- `--allow-ctx-override`: allow non-object `ctx` frontmatter. By default, a document that defines `ctx` as a non-object (e.g., a string or array) causes a hard error. This flag downgrades the error to a warning, and the runtime context is used instead.
- `--shell`: report shell commands discovered in the compose tree and exit without executing them. The report includes body `::shell` directives and top-level frontmatter `$(...)` shell expressions, including commands discovered through markdown transclusions.
- `--perf`: emit a structured performance report to stderr after compose completes. The report includes both command-level timings (input loading, context capture, validation, option construction) and per-stage compose pipeline timings. The report is printed after any compose warnings and deferred validation issues.

### Compose Warnings

Compose warnings (context merge collisions, partial runtime capture failures, etc.) are printed to stderr after the composed content is written to stdout.

### Output Behavior

**Default (`--output markdown`)**

- Prints composed markdown content.
- Frontmatter is consumed as pipeline input and stripped from output unless `--frontmatter` is specified.

**`--frontmatter` / `--fm`**

- Includes frontmatter in the markdown output, reflecting any values filled in by `--state`.
- Key order is preserved from the source document; new keys appear at the end.

**`--output auto`**

- Treated the same as markdown for compose.

**`--output html|json`**

- Emits HTML or AST JSON from the composed document.

**`--show` behavior**

- For markdown/auto: prints composed content and opens markdown artifact.
- For html/json: opens artifact instead of printing to stdout.

### Precedence

Override values are applied in this order (highest priority last):

1. **Document frontmatter** — base values from the document itself
2. **`--state`** — fills null/missing keys without overriding existing values
3. **`--set`** — unconditional overwrite of matching keys
4. **Shorthand `key=value`** — unconditional overwrite, highest CLI priority

Shorthand setters are merged into the same `--set` override map. On key collision, the shorthand value wins.

```bash
# --state provides defaults, --set overrides, shorthand overrides both
md compose doc.md --state '{"name":"Alice"}' --set '{"name":"Bob"}' name=Charlie
# Result: name is "Charlie"
```

### Transform Context

- If `[INPUT]` is a file path, compose sets source-file context for relative transclusion resolution.
- If input is stdin (`-` or piped with no input arg), source-file-relative path resolution is not available.
- All file path arguments support `@`-prefixed file references (resolved from git root).

### Validation and Errors

- **Reference validation** runs automatically before composition when input is a file path. If any references (hyperlinks, images, transclusions, CSS imports, etc.) point to missing targets, compose reports grouped errors and exits with code 2.

  ```
  Invalid Hyperlink(s)
  - the preparation.md reference to @darkmatter/docs/text-replacement.md is not valid
  - the preparation.md reference to @darkmatter/docs/interpolation.md is not valid

  19 references scanned, 17 valid, 2 issues
  ```

  Issues are grouped by category (hyperlinks, images, transclusions, etc.) with the source document shown as a clickable link and broken targets in red.

- **Allow flags** relax validation for specific categories. When all errors belong to allowed categories, compose proceeds normally: content goes to stdout, issues are reported on stderr, and the exit code is 0. If any errors remain in non-allowed categories, compose still blocks with exit code 2.

  ```bash
  # Compose despite broken links — content to stdout, warnings to stderr
  md compose doc.md --allow-missing-hyperlinks

  # Compose despite any broken reference
  md compose doc.md --allow-any-missing-reference

  # Pipe composed output while still seeing warnings
  md compose doc.md --allow-missing-hyperlinks > output.md
  ```

- Invalid JSON/JSON5 in `--state` returns an error.
- Non-object JSON (array/string/number/etc.) in `--state` returns an error.
- Transform failures return non-zero exit with error details.

## Lessons Learned

- `compose` defaults to markdown because composed document output is the primary workflow.
- Frontmatter acts as transform configuration and is intentionally stripped unless `--fm` is used.
- Explicit `--state` is best for script-driven parameterization; it fills gaps without overwriting intent.
- JSON5 support for `--state` makes shell usage more ergonomic (no quoting keys).

## Issues

- The compose transform report (warnings from the library compose pipeline) is still discarded in CLI output; only reference validation issues are surfaced.
