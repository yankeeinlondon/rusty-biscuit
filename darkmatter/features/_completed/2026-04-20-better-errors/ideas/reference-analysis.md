# Reference Analysis — Block Style Error Ideas

Covers the two error enums in the reference analysis subsystem:

- **`ReferenceError`** — `darkmatter/lib/src/markdown/reference/errors.rs:7`
- **`FileTreeError`** — `darkmatter/lib/src/markdown/reference/file_tree/mod.rs:44`

---

## `ReferenceError`

Seven variants covering directive parsing, missing context, validation, compose propagation, file resolution, I/O, and URL parsing.

| Variant | Current `#[error]` message | Block Style candidate? |
|---------|---------------------------|----------------------|
| `ParseDirective` | `"Failed to parse directive at line {line}: {message}"` | **Yes** — high value |
| `MissingSourceContext` | `"Missing source context for reference '{reference}' at line {line}"` | **Yes** — high value |
| `Validation` | `"Validation error: {0}"` | Low — opaque string, no structured fields |
| `Compose` | `"{0}"` (transparent) | No — delegated to `MarkdownError` display |
| `FileReference` | `#[error(transparent)]` | No — delegated to `biscuit_file::FileReferenceError` |
| `Io` | `#[error(transparent)]` | No — generic |
| `Url` | `#[error(transparent)]` | No — generic |

### Variant: `ParseDirective`

**Why block style?** Users write `::file`, `::code`, `::toc-linking` directives inline in markdown. A parse failure is almost always a typo or malformed argument — the error should show the exact line and suggest the correct syntax.

**Proposed block:**

```
⤫ ReferenceError: Failed to parse directive
┃ Line 42 of <b>docs/getting-started.md</b>:
┃ <dim>::file ./intro.md extra-token</dim>
┃                      ^^^^^^^^^^^^ unexpected argument
┃
┃ <dim>Hint:</dim> The <b>::file</b> directive accepts one path argument.
┃   ::file ./intro.md
┃   ::file ./intro.md when="debug"
```

**biscuit-terminal implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>ReferenceError:</b> Failed to parse directive")
    .body(Prose::new(format!(
        "Line {line} of <b>{file}</b>:\n\
         <dim>{directive_text}</dim>\n\
         {pointer}\n\
         {message}"
    )))
    .hint("The <b>::file</b> directive accepts one path argument.\n  \
           ::file ./intro.md\n  \
           ::file ./intro.md when=\"debug\"")
```

**Required enrichments to `ParseDirective`:**
- Add `file: PathBuf` (source file being analyzed)
- Add `directive_text: String` (raw line content)
- Compute a `^` pointer from `line` offset + `message`

---

### Variant: `MissingSourceContext`

**Why block style?** Reference analysis needs the source file path to resolve relative links. When it's missing, the user likely invoked analysis on a `Markdown` created from a string rather than a file. The error should explain this and show the offending reference.

**Proposed block:**

```
⤫ ReferenceError: Missing source context
┃ Cannot resolve reference <b>./images/diagram.png</b> at line 15.
┃ The markdown document was created from an in-memory string
┃ and has no file-system context for relative path resolution.
┃
┃ <dim>Hint:</dim> Load the document from a file path instead:
┃   let md = Markdown::try_from(Path::new("docs/guide.md"))?;
```

**biscuit-terminal implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>ReferenceError:</b> Missing source context")
    .body(Prose::new(format!(
        "Cannot resolve reference <b>{reference}</b> at line {line}.\n\
         The markdown document was created from an in-memory string\n\
         and has no file-system context for relative path resolution."
    )))
    .hint("Load the document from a file path instead:\n  \
           let md = Markdown::try_from(Path::new(\"docs/guide.md\"))?;")
```

**Required enrichments to `MissingSourceContext`:**
- Already has `reference: String` and `line: usize` — sufficient for the block.

---

## `FileTreeError`

Four variants for the `FileTree` component's builder and construction errors.

| Variant | Current `#[error]` message | Block Style candidate? |
|---------|---------------------------|----------------------|
| `PathNotFound` | `"path not found: {path.display()}"` | **Yes** — high value |
| `NotAFile` | `"not a file: {path.display()}"` | **Yes** — moderate value |
| `Markdown` | `#[from] MarkdownError` | No — delegated |
| `Reference` | `#[from] ReferenceError` | No — delegated |

### Variant: `PathNotFound`

**Why block style?** `FileTree::new()` is a common entry point (CLI `md graph doc.md`). When the path doesn't exist, the user needs to see the path they passed and likely a hint about their current working directory or a typo.

**Proposed block:**

```
⤫ FileTreeError: Path not found
┃ <b>docs/getting-started.md</b> does not exist.
┃
┃ <dim>Hint:</dim> Check that the path is relative to the current
┃ working directory, or use an absolute path.
┃   md graph ./docs/getting-started.md
```

**biscuit-terminal implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>FileTreeError:</b> Path not found")
    .body(Prose::new(format!(
        "<b>{}</b> does not exist.",
        path.display()
    )))
    .hint("Check that the path is relative to the current working \
           directory, or use an absolute path.\n  \
           md graph ./docs/getting-started.md")
```

**Required enrichments to `PathNotFound`:**
- Already has `PathBuf` — sufficient. The hint is static.

---

### Variant: `NotAFile`

**Why block style?** Users may accidentally pass a directory to `md graph`. The error should clarify that only files are accepted.

**Proposed block:**

```
⤫ FileTreeError: Not a file
┃ <b>docs/</b> is a directory, not a file.
┃ FileTree requires a markdown file path.
┃
┃ <dim>Hint:</dim> Point to a specific .md file:
┃   md graph docs/index.md
```

**biscuit-terminal implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>FileTreeError:</b> Not a file")
    .body(Prose::new(format!(
        "<b>{}</b> is a directory, not a file.\n\
         FileTree requires a markdown file path.",
        path.display()
    )))
    .hint("Point to a specific .md file:\n  \
           md graph docs/index.md")
```

**Required enrichments to `NotAFile`:**
- Already has `PathBuf` — sufficient.

---

## Summary

| Error | Variant | Enrichment needed | Block components |
|-------|---------|-------------------|------------------|
| `ReferenceError` | `ParseDirective` | Add `file`, `directive_text` fields | `Status`(Error) + `StatusBlock` + `Prose` |
| `ReferenceError` | `MissingSourceContext` | None (fields sufficient) | `Status`(Error) + `StatusBlock` + `Prose` |
| `FileTreeError` | `PathNotFound` | None (fields sufficient) | `Status`(Error) + `StatusBlock` + `Prose` |
| `FileTreeError` | `NotAFile` | None (fields sufficient) | `Status`(Error) + `StatusBlock` + `Prose` |

All four variants follow the Block Style Error pattern from the spec:

1. **Title line** — `Status` with `StatusState::Error` renders the icon (red `⤫`) and prose header (`<b>ErrorName:</b> Title text`)
2. **Body block** — `StatusBlock` wraps body content in a `BlockQuote` with `Tailwind::Red500` left border (`┃`)
3. **Hint line** — `StatusBlock::hint()` renders a `Prose`-formatted suggestion below the block
