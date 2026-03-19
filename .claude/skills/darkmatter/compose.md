# Compose Pipeline

The darkmatter compose pipeline provides document preparation through two stages.

## Pipeline Overview

**Stage 1** (in-memory transforms — 5 sub-stages):

1. **Text Replacement** - `replace:` frontmatter replaces literal strings
2. **Interpolation** - `{{ variable }}` expressions expand to values
3. **TOC Linking** - `::toc-linking` directives expand to heading link lists
4. **Cleanup** - Normalizes markdown formatting
5. **Normalization** - Adjusts heading levels

**Stage 2** (file-based transclusion):

- `::file ./doc.md` - Include markdown with recursive processing
- `::code ./main.rs` - Include as fenced code block
- `prologue` / `epilogue` - Frontmatter-driven file includes
- `when="..."` conditions, cycle detection, depth limits
- Heading re-leveling for included markdown (H6 overflow handled gracefully)

## API

```rust
use darkmatter::markdown::{Markdown, compose::{ComposeOptions, Stage1Stages}};

// Stage 1 only (compose with defaults)
let (composed, report) = md.compose()?;

// Stage 1 with options
let options = ComposeOptions::new()
    .with_external_state(json!({"key": "value"}))
    .with_stages(Stage1Stages::only_interpolation())
    .with_fail_fast(true);
let (composed, report) = md.compose_with(options)?;

// In-place mutation (no clone)
let report = md.compose_mut()?;

// Stage 1 + Stage 2 (requires source file path for transclusion)
let md = Markdown::try_from(std::path::Path::new("docs/root.md"))?;
let options = ComposeOptions::new()
    .with_source_file("docs/root.md");
let (composed, report) = md.compose_with(options)?;
println!("{}", report.summary());
```

## Text Replacement

The `replace:` frontmatter key enables literal string replacement.

```yaml
---
replace:
  PLACEHOLDER: actual value
  VERSION: "2.0"
---
This PLACEHOLDER will become "actual value".
Version: VERSION
```

### Replacement Rules

- Keys must be literal strings (case-sensitive)
- Overlap: longest key wins, then lexicographic order
- Values: scalars only (strings, numbers, booleans, null)
- `null` → empty string
- Non-map `replace` silently skipped
- Single-pass (replacements not re-scanned)

## Interpolation

Expressions between `{{ }}` are evaluated and replaced with values.

### Variable Resolution

| Pattern | Description |
|---------|-------------|
| `{{ foo }}` | Frontmatter value |
| `{{ user.name }}` | Nested object path |
| `{{ ctx.today }}` | Runtime context |
| `{{ env.HOME }}` | Environment variable |

### Context Values (`ctx.*`)

| Key | Description |
|-----|-------------|
| `ctx.now` | ISO 8601 local datetime |
| `ctx.utc` | ISO 8601 UTC datetime |
| `ctx.today` | Local date (YYYY-MM-DD) |
| `ctx.yesterday` | Yesterday's date |
| `ctx.tomorrow` | Tomorrow's date |
| `ctx.dow` | Day of week (Monday, etc.) |
| `ctx.dow_abbr` | Abbreviated (Mon, etc.) |
| `ctx.year` | Current year |
| `ctx.month` | Month number (01-12) |
| `ctx.month_name` | Month name (January, etc.) |
| `ctx.month_name_abbr` | Abbreviated (Jan, etc.) |

### Fallback Expressions

```handlebars
{{ color | "unknown" }}
{{ primary | secondary | "default" }}
```

Uses first truthy value, or the fallback.

### Ternary Expressions

```handlebars
{{ active ? "enabled" : "disabled" }}
{{ count > 0 ? "has items" : "empty" }}
```

### Comparison Operators

- `==` - equality
- `!=` - inequality
- `>` - greater than
- `>=` - greater than or equal
- `<` - less than

Numeric strings auto-convert for comparisons.

### Helper Functions

```handlebars
{{ length(name) }}           // String length
{{ length(items) }}          // Array length
{{ length(data) }}           // Object key count

{{ number("42") }}           // Parse string to number
{{ number(x, -1) }}          // With default on failure

{{ round(3.7) }}             // Round to integer (4)
{{ round(value, 0) }}        // With default
```

### Code Region Protection

Expressions inside code spans and fenced code blocks are NOT processed:

```markdown
Inline: `{{ not_evaluated }}`

```
{{ also_not_evaluated }}
```
```

## ComposeReport

```rust
pub struct ComposeReport {
    pub replacements_applied: usize,
    pub interpolations_applied: usize,
    pub toc_links_generated: usize,
    pub cleanup_changed: bool,
    pub normalization_report: Option<NormalizationReport>,
    pub warnings: Vec<ComposeWarning>,
}

// Check for changes
if report.has_changes() {
    println!("{}", report.summary());
}
```

## Error Handling

With `fail_fast: false` (default):
- Parse errors leave original `{{ expression }}` in place
- Evaluation errors leave original in place
- Warnings recorded in report

With `fail_fast: true`:
- Parse errors logged and continue
- Evaluation errors logged and continue
- (Future: may return errors)

## Stage 2: Transclusion

Stage 2 runs after Stage 1 when a source file path is provided. It resolves file-based includes.

### Block Directives

```markdown
<!-- Include another markdown file (recursive) -->
::file ./chapter.md

<!-- Include as fenced code block -->
::code ./main.rs

<!-- Conditional include -->
::file ./appendix.md when="include_appendix"
```

### Frontmatter Directives

```yaml
---
prologue: ./header.md
epilogue: ./footer.md
---
```

- `prologue` content is prepended before the document body
- `epilogue` content is appended after the document body

### Safety Features

- **Cycle detection**: Prevents infinite recursion from circular includes
- **Max depth limits**: Configurable depth for nested transclusion
- **Heading re-leveling**: Included markdown headings are adjusted to fit the nesting context (H6 overflow handled gracefully)

## Module Structure

```
darkmatter/lib/src/markdown/compose/
├── mod.rs           # Public API, pipeline orchestration
├── types.rs         # ComposeOptions, ComposeReport, etc.
├── state.rs         # EffectiveState, merge logic
├── replacement.rs   # Text replacement engine
└── interpolation/
    ├── mod.rs       # Module exports
    ├── lexer.rs     # Tokenizer, expression finder
    ├── ast.rs       # AST types
    ├── parser.rs    # Expression parser
    └── evaluator.rs # AST evaluation
```
