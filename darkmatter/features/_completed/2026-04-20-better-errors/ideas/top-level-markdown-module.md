# MarkdownError — Block Style Error Ideas

Source: `darkmatter/lib/src/markdown/types.rs:15`

`MarkdownError` is the top-level error enum for the `markdown` module. It wraps errors from every compose pipeline stage (transclusion, shell expansion, page blocks, TOC linking, frontmatter, context merge) as well as I/O and rendering errors. Because it is the primary error type users see at the CLI, it is the highest-value target for Block Style Error improvements.

## Block Style Pattern

Every improved error message uses this structure (from [spec](../spec.md)):

- **Title line**: `Status` with `StatusState::Error` — bold red error name + bold title via `Prose`
- **Body block**: `StatusBlock` with red vertical bar (`┃`) and prose body
- **Hint**: optional `Prose` hint line below the block

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>FrontmatterParse</b>: YAML parsing failed")
    .body(Prose::new("...descriptive text..."))
    .hint("<dim>Hint: ...</dim>")
```

---

## Variant-by-Variant Improvements

### `FrontmatterParse` (from `YamlParseError`)

Current message: `"Failed to parse frontmatter: {0}"` — delegates entirely to `YamlParseError`'s Display.

**Improvement 1: Surface the offending line and key**

```
✗ FrontmatterParse: invalid YAML in frontmatter
┃ Line 5, column 12: unexpected character ':'
┃ 
┃   3 | title: My Doc
┃   4 | author: 
┃ > 5 | date: 2024-13-99:
┃                ^
┃ Expected a string, number, or boolean value.
Hint: Check YAML syntax at the frontmatter delimiter (---). Common mistakes include
      trailing colons, unquoted strings with special characters, and incorrect indentation.
```

**Improvement 2: Identify the likely malformed key when possible**

When the YAML parser returns a line/column, extract the key name at that position from the raw frontmatter text and include it in the title:

```
✗ FrontmatterParse: the 'date' key has an invalid value
┃ The value '2024-13-99:' could not be parsed as valid YAML.
┃ If this value contains special characters (like ':'), wrap it in quotes.
```

Use `Status::from_prose("<b>FrontmatterParse</b>: the <red>'date'</red> key has an invalid value")` for the header to highlight the key in red.

---

### `FrontmatterMerge(String)`

Current message: `"Failed to merge frontmatter: {0}"` — a free-form string with no structure.

**Improvement 1: Name the conflicting keys**

```
✗ FrontmatterMerge: conflicting keys during frontmatter merge
┃ The following keys could not be merged between parent and child documents:
┃   - title (string vs array)
┃   - tags (missing in child)
┃ Merge strategy requires both values to be the same type.
Hint: Use the 'replace:' frontmatter directive to explicitly override values
      instead of relying on implicit merge behavior.
```

**Improvement 2: Show the parent → child file paths**

When a merge fails during transclusion, include both file paths in the body:

```
┃ Parent:  docs/index.md (frontmatter line 1)
┃ Child:   docs/partials/header.md (frontmatter line 1)
┃ Conflict on key 'layout': parent has 'wide', child has 'narrow'.
```

Use `Prose` tokens for the file paths: `<dim>Parent:</dim>  docs/index.md`.

---

### `FileLoad` (from `std::io::Error`)

Current message: `"Failed to load file: {0}"` — wraps the raw `io::Error`.

**Improvement 1: Distinguish permission vs not-found vs other I/O errors**

```
✗ FileLoad: file not found
┃ The file 'docs/guide.md' does not exist at the expected location.
┃ 
┃ Resolved path: /projects/my-docs/docs/guide.md
Hint: Check the file path in your ::file or ::code directive. Relative paths
      resolve from the source document's directory, not the CWD.
```

For permission errors:

```
✗ FileLoad: permission denied
┃ Cannot read '/projects/my-docs/private/secret.md'.
┃ The current user does not have read access to this file.
Hint: Run `ls -la /projects/my-docs/private/secret.md` to check permissions.
```

**Improvement 2: Suggest similar filenames on not-found**

When a `::file` or `::code` directive references a missing file, use a Levenshtein check against sibling files to suggest alternatives:

```
┃ Did you mean 'guide.md'? Similar files in docs/:
┃   - guide.md
┃   - getting-started.md
```

---

### `UrlFetch` (from `reqwest::Error`)

Current message: `"Failed to fetch URL: {0}"` — wraps the raw `reqwest::Error`.

**Improvement 1: Separate network, DNS, timeout, and HTTP status errors**

```
✗ UrlFetch: HTTP 404 — resource not found
┃ The URL returned a non-success status code.
┃ 
┃   GET https://example.com/missing-doc.md → 404 Not Found
Hint: Verify the URL is correct. The remote resource may have been moved or deleted.
```

For connection errors:

```
✗ UrlFetch: connection refused
┃ Could not connect to https://internal-api.local/docs.md
┃ The server may be down or unreachable from this network.
Hint: If this is an internal URL, check VPN connectivity or try `curl -I <url>`.
```

**Improvement 2: Include the directive origin (line number)**

When the URL fetch was triggered by a `::url` directive, include the line:

```
┃ Directive at line 42: ::url https://example.com/missing-doc.md
```

---

### `ThemeLoad(String)`

Current message: `"Failed to load theme: {0}"` — a free-form string.

**Improvement 1: List available themes when an unknown theme is named**

```
✗ ThemeLoad: unknown theme 'midnight'
┃ 'midnight' is not a recognized theme name.
┃ 
┃ Available themes:
┃   base16-ocean.dark, base16-eighties.dark, inspired-github
┃   solarized-dark, solarized-light
Hint: Theme names are case-sensitive. Use `md --list-themes` to see all options.
```

Use `Prose` with `<dim>` for the theme list to de-emphasize it:

```
Prose::new("<dim>Available themes:\n  base16-ocean.dark, inspired-github, ...</dim>")
```

**Improvement 2: File-not-found for custom theme paths**

```
✗ ThemeLoad: theme file not found
┃ The custom theme file 'themes/custom.tmTheme' does not exist.
┃ Resolved from CWD: /projects/my-docs/themes/custom.tmTheme
Hint: Custom themes must be TextMate .tmTheme files. Provide an absolute path
      or a path relative to the working directory.
```

---

### `AstParse(String)`

Current message: `"Failed to parse AST: {0}"` — a free-form string.

**Improvement 1: Show the problematic markdown fragment**

```
✗ AstParse: malformed markdown in AST generation
┃ The markdown parser encountered unexpected content near:
┃ 
┃   ...heading content with [[broken link syntax
┃ 
┃ This may be caused by unbalanced brackets, unclosed code fences, or
┃ deeply nested structures exceeding the parser's limits.
Hint: Try isolating the problematic section by rendering half the document at a time.
```

**Improvement 2: Include byte offset or line range**

```
┃ Error occurred at byte offset 4823 (approximately line 120).
```

---

### `InvalidLineRange(String)`

Current message: `"Invalid line range: {0}"` — a free-form string.

**Improvement 1: Parse and explain the invalid range syntax**

```
✗ InvalidLineRange: '50-30' is not a valid line range
┃ Line ranges must have a start ≤ end. Got start=50, end=30.
┃ 
┃ Supported formats:
┃   L20        — single line
┃   L10-20     — inclusive range
┃   L10-       — from line 10 to end of file
Hint: Line numbers are 1-based. The start line must not exceed the end line.
```

Use `Prose` for the supported formats section:

```
Prose::new("<b>Supported formats:</b>\n  L20 — single line\n  L10-20 — inclusive range\n  L10- — from line 10 to end")
```

**Improvement 2: Flag out-of-bounds ranges**

```
✗ InvalidLineRange: line range exceeds file length
┃ Requested: L800-L900
┃ File has only 742 lines.
Hint: Use `L742-` to reference from line 742 to the end of the file.
```

---

### `Serialization` (from `serde_json::Error`)

Current message: `"Serialization error: {0}"` — wraps `serde_json::Error`.

**Improvement 1: Identify the field that failed to serialize**

```
✗ Serialization: JSON serialization failed
┃ Could not serialize the 'frontmatter' field to JSON.
┃ The value contains a type that cannot be represented in JSON (e.g., a map with non-string keys).
Hint: Check frontmatter values for unsupported types. YAML anchors and tags
      may produce values that have no JSON equivalent.
```

**Improvement 2: Contextualize where serialization was happening**

```
┃ Context: serializing AST to JSON for --output json
┃ 
┃ serde_json error: key must be a string at line 1 column 2840
```

---

### `Transform(String)`

Current message: `"Transform error: {0}"` — a free-form string.

**Improvement 1: Name the transform stage that failed**

```
✗ Transform: cleanup transform failed
┃ The cleanup stage of the compose pipeline encountered an error.
┃ Reason: {original string message}
┃ 
┃ Stage: Inline Post → Cleanup
┃ Source: docs/index.md
```

**Improvement 2: Include the document path and stage name**

```
Status::from_prose("<b>Transform</b>: cleanup failed in <red>docs/index.md</red>")
```

---

### `Transclusion` (from `TransclusionError`)

Current message: `"Transclusion error: {0}"` — delegates to the inner type.

**Improvement 1: Show the transclusion chain (ancestor path)**

This is the most important wrapper to improve because transclusion errors are deeply nested and the chain of file references is critical for debugging.

```
✗ Transclusion: cycle detected
┃ A circular transclusion was detected in the following chain:
┃ 
┃   docs/index.md
┃   → docs/partials/header.md (line 5)
┃   → docs/partials/nav.md (line 12)
┃   → docs/index.md ← cycle!
┃ 
┃ Transclusion depth: 3
Hint: Remove the circular ::file reference, or use 'when=' conditions to prevent
      the infinite loop.
```

Use `Prose` with `<red>` for the cycle marker:

```
Prose::new("→ docs/index.md <red>← cycle!</red>")
```

**Improvement 2: Show the exact directive line on parse errors**

```
✗ Transclusion: failed to parse directive at line 23
┃   ::file ./header.md set=invalid-json
┃                        ^^^^^^^^^^^^^
┃ Expected a JSON5 object for 'set=' but got: invalid-json
Hint: The 'set=' value must be a valid JSON5 object, e.g.: set={\"layout\":\"wide\"}
```

---

### `TocLinking` (from `TocLinkingError`)

Current message: `"TOC linking error: {0}"` — delegates to the inner type.

**Improvement 1: List valid cleanup services on invalid name**

`TocLinkingError::InvalidCleanupService` already has `{service}` and `{line}`, but doesn't enumerate valid options.

```
✗ TocLinking: invalid cleanup service 'strip_headers'
┃ At line 15: 'strip_headers' is not a recognized cleanup service.
┃ 
┃ Available services:
┃   emoji_leader, emoji_trailing, emoji, number, capitalize
Hint: Service names use snake_case and are case-insensitive.
      Use 'cleanup=emoji capitalize' to apply multiple services.
```

**Improvement 2: Show the directive line for file-not-found**

```
✗ TocLinking: file not found at line 27
┃ Directive: ::toc-linking ./sidebar.md | ./fallback.md
┃ Neither target exists:
┃   - ./sidebar.md (not found)
┃   - ./fallback.md (not found)
┃ Append '| false' to suppress this error when files are optional.
Hint: The pipe syntax creates a fallback chain. The last option '| false'
      silences the error entirely when no file matches.
```

---

### `ShellExpansion` (from `ShellExpansionError`)

Current message: `"Shell expansion failed: {0}"` — delegates to the inner type.

**Improvement 1: Show approval workflow for `ApprovalRequired`**

`ShellExpansionError::ApprovalRequired` already carries `command`, `whitelist_path`, and `blacklist_path` but just says "Approval required for '{command}'".

```
✗ ShellExpansion: approval required for 'git log'
┃ At line 18: the command 'git log' is not on the whitelist.
┃ 
┃ Whitelist: ~/.config/darkmatter/shell-whitelist.md
┃ Blacklist: ~/.config/darkmatter/shell-blacklist.md
┃ 
┃ To approve this command, run:
┃   md --approve-shell 'git log'
┃ 
┃ Or add it to your whitelist file manually.
Hint: Shell commands require explicit approval for security. Pre-approved commands
      are listed in your whitelist file. Use --allow-shell to skip approvals for
      trusted documents.
```

Use `Prose` for the approval command to make it stand out:

```
Prose::new("To approve this command, run:\n  <bold>md --approve-shell 'git log'</bold>")
```

**Improvement 2: Show stderr on `ExecutionFailed`**

`ShellExpansionError::ExecutionFailed` already carries `stdout` and `stderr` but they are not surfaced distinctly.

```
✗ ShellExpansion: command failed (exit code 1)
┃ At line 42: 'jq .version package.json'
┃ 
┃ stderr:
┃   parse error: Invalid numeric literal at line 1, column 5
┃ 
┃ stdout: (empty)
Hint: The command exited with a non-zero code. Check the command output above
      for details. Use 'when_exit_code=' or 'except_exit_code=' in the ::shell
      directive to handle expected failures gracefully.
```

---

### `PageBlock` (from `PageBlockError`)

Current message: `"Page block error: {0}"` — delegates to the inner type.

**Improvement 1: Visual pairing for `UnmatchedEnd` / `UnterminatedBlock`**

`PageBlockError::UnmatchedEnd` has `{line}`, `UnterminatedBlock` has `{line}` — both are good candidates for showing the source context.

```
✗ PageBlock: unmatched ::end-block at line 87
┃ There is no matching ::block directive for this ::end-block.
┃ 
┃   85 | Some content here
┃   86 | 
┃ > 87 | ::end-block
┃          ^^^^^^^^^^
┃ No ::block was opened before this line.
Hint: Every ::end-block must have a corresponding ::block with a matching 'when='
      condition. Check for accidentally deleted or mis-indented ::block lines.
```

For `UnterminatedBlock`:

```
✗ PageBlock: unterminated ::block starting at line 42
┃ The ::block opened at line 42 was never closed with ::end-block.
┃ 
┃ > 42 | ::block when="production"
┃         ^^^^^^^^^^^^^^^^^^^^^^^^
┃   43 | Production-only content
┃   44 | More content
┃   ... (end of file)
┃ 
┃ The file ends without closing this block.
Hint: Add ::end-block after the last line of conditional content.
      Page blocks must always be explicitly closed.
```

**Improvement 2: Nesting depth context for deeply nested blocks**

```
┃ Block nesting depth: 4 (parent blocks at lines 12, 28, 42, 56)
┃ The unmatched ::end-block may belong to one of these parent blocks.
```

---

### `Reference` (from `ReferenceError`)

Current message: `"Reference error: {0}"` — delegates to the inner type.

**Improvement 1: Show the reference type and raw markdown on parse errors**

```
✗ Reference: could not parse link reference
┃ At line 30: the markdown link has an unrecognized format.
┃ 
┃   [My Link](https://example.com "title" extra)
┃                                     ^^^^^
┃ Unexpected content after the link title.
Hint: Standard markdown links use [text](url) or [text](url "title").
      Check for trailing characters or malformed URL syntax.
```

**Improvement 2: List validation failures with file paths**

When `ReferenceError::Validation` fires during `md graph --validate`:

```
✗ Reference: validation failed
┃ The following references in docs/index.md could not be resolved:
┃ 
┃   Line 12: [API Docs](./api.md) — file not found
┃   Line 45: ![logo](/images/logo.png) — image missing
┃   Line 78: [External](https://down.example.com) — HTTP 503
Hint: Run `md graph docs/index.md --validate` to see the full dependency
      report with all broken references.
```

---

### `CtxMerge` (from `CtxMergeError`)

Current message: `"Context error: {0}"` — wraps the single `InvalidUserCtx` variant.

**Improvement 1: Show the actual value and expected shape**

```
✗ CtxMerge: document 'ctx' must be a JSON object
┃ The frontmatter 'ctx' key has type 'array', but it must be a JSON object.
┃ 
┃ Found:
┃   ctx: [1, 2, 3]
┃ 
┃ Expected:
┃   ctx:
┃     key: value
┃     another: value
Hint: The 'ctx' frontmatter key is used for runtime context variables like
      {{ ctx.today }}. It must be a flat or nested object, not an array or scalar.
```

Use `Prose` for the code blocks:

```
Prose::new("<b>Found:</b>\n  ctx: [1, 2, 3]\n\n<b>Expected:</b>\n  ctx:\n    key: value")
```

**Improvement 2: Suggest using `allow_override` in permissive mode**

```
Hint: If this is intentional, use --allow-override-ctx to replace the document's
      ctx with the runtime context. Otherwise, change the frontmatter 'ctx' to an object.
```

---

## Cross-Cutting Themes

### 1. Source Context Snippets

Many variants (`FrontmatterParse`, `PageBlock`, `ShellExpansion`, `Transclusion`) already carry line numbers. The Block Style Error should include a **source snippet** showing the offending line with a `>` marker and `^` pointer, rendered as `Prose` with `<red>` highlights.

### 2. Composable Hints via `Prose`

The `hint()` on `StatusBlock` accepts `Prose`-formatted strings. Every variant should include at least one actionable hint, using:
- `<dim>` for de-emphasized context
- `<bold>` for commands the user should run
- `<red>` for the specific error detail (file path, key name, line number)

### 3. Chained Error Visualization

Since `MarkdownError` wraps sub-errors (`TransclusionError`, `ShellExpansionError`, etc.), the Block Style Error for the outer type should show the **chain**:

```
✗ MarkdownError: transclusion pipeline failed
┃ The compose pipeline could not complete due to a transclusion error.
┃ 
┃ Caused by:
┃   ✗ TransclusionError: file not found './header.md'
┃   ┃ At line 5 in docs/index.md
┃   ┃ Directive: ::file ./header.md
┃   ┃ 
┃   ┃ Resolved path: /projects/my-docs/header.md (does not exist)
┃   Hint: Check relative path resolution. Paths in ::file directives
┃         resolve from the source document's directory.
```

This uses nested `StatusBlock` instances — the outer one for `MarkdownError` and an inner `StatusBlock` (indented) for the specific `TransclusionError` variant.

### 4. Error-Type-Specific `BlockError` Trait Implementation

Each variant of `MarkdownError` should map to a `BlockError` trait method that returns a `StatusBlock`. The trait impl for `MarkdownError` would delegate to the inner error's own `BlockError` impl when available, falling back to a generic block for wrapped `io::Error` / `reqwest::Error` / `serde_json::Error` types.
