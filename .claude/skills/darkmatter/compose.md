# Compose Pipeline

The darkmatter compose pipeline provides document preparation through three phases.

## Pipeline Overview

**Inline Pre** (serial):

1. **Frontmatter Interpolation** - `{{ variable }}` in frontmatter resolves before effective state is built
2. **Schema Validation** - Validate frontmatter against `$schema` or `ComposeOptions::baseline_schema`. Runs after `--set` / `--state` overrides and frontmatter interpolation, but before shell expansion. **Coerces** schema-recognized top-level scalars to their declared types (default-on, e.g. the string `"true"` → real boolean) and writes the coerced values back into frontmatter, skipping `$(...)`-pending values. Problems on fields still holding `$(...)` are deferred to downstream re-validation only when frontmatter shell expansion is enabled; when it is disabled they fail fast
3. **Frontmatter Shell Expansion** - top-level `$(cmd)` frontmatter values execute after interpolation and write trimmed `stdout` back into frontmatter
4. **Text Replacement** - `replace:` frontmatter replaces literal strings
5. **Page Blocks** - `::block`/`::end-block` conditional regions
6. **Interpolation** - `{{ variable }}` expressions expand to values
7. **Shell Expansion** - Execute `::shell` directives execute approved commands and inject combined `stdout` + `stderr`
8. **Link Resolve** - Resolve all local link targets (Markdown hyperlinks/images and supported HTML embeds) to absolute paths

**Transclusion** (prepared serially, resolved concurrently via Rayon):


- `::file ./doc.md` - Include markdown with recursive processing
- `::code ./main.rs` - Include as fenced code block
- `::toc-linking` - Generate heading link lists from external documents' raw source headings
- `prologue` / `epilogue` - Frontmatter-driven file includes
- `when="..."` conditions, cycle detection, depth limits
- Heading re-leveling for included markdown (H6 overflow handled gracefully)

**Inline Post** (serial):

- **Cleanup** - Normalizes markdown formatting
- **Normalization** - Adjusts heading levels

**Finalization** (root-only serial):

- **Link Normalization** - Converts absolute path links back into portable forms:
    - **Same-repo**: Paths inside the same git repository are made relative to the document
    - **Home-dir**: Paths under the user's home directory use the `~/` prefix
    - **ENV-var**: Paths under whitelisted environment variables (e.g. `PROJECT_ROOT`) use `${VAR}/` prefix

## API

```rust
use darkmatter::markdown::{Markdown, compose::{ComposeOptions, ComposeOperation}};

// Compose with all operations enabled (default)
let (composed, report) = md.compose()?;

// Only run specific operations
let options = ComposeOptions::new()
    .only(&[ComposeOperation::Interpolation])
    .with_external_state(json!({"key": "value"}))
    .with_fail_fast(true);
let (composed, report) = md.compose_with(options)?;

// Disable specific operations
let options = ComposeOptions::new()
    .disable(ComposeOperation::Cleanup)
    .disable(ComposeOperation::Normalization);

// With a baseline schema (library-only; no CLI flag)
let baseline: darkmatter::markdown::schemas::SimplifiedSchema = /* ... */;
let options = ComposeOptions::new()
    .with_baseline_schema(baseline);

// In-place mutation (no clone)
let report = md.compose_mut()?;

// Full pipeline with transclusion (requires source file path)
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
{{ color || "unknown" }}
{{ primary || secondary || "default" }}
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

Inline code spans (single backticks) ARE interpolated — the common
templating pattern `` `var_{{ phase }}` `` works without any opt-in.

Fenced and indented code blocks are skipped by default to preserve
literal code samples. Set `interpolate_code_blocks: true` (frontmatter)
or call `ComposeOptions::with_interpolate_code_blocks(true)` to opt
fenced blocks back into the scan.

```markdown
Inline: `{{ evaluated }}`             # always interpolated

```
{{ not_evaluated_by_default }}        # skipped unless opted in
```
```

## ComposeReport

```rust
pub struct ComposeReport {
    pub replacements_applied: usize,
    pub interpolations_applied: usize,
    pub toc_links_generated: usize,
    pub shell_expansions_applied: usize,
    pub shell_approvals_used: usize,
    pub page_blocks_rendered: usize,
    pub page_blocks_skipped: usize,
    pub transclusions_applied: usize,
    pub transclusions_skipped: usize,
    pub link_resolves_applied: usize,
    pub link_normalizations_applied: usize,
    pub max_transclusion_depth: usize,
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
- TOC-linking and non-structural transclusion failures are downgraded to warnings
- Structural transclusion errors (cycles, max depth) still return immediately
- Warnings recorded in report

With `fail_fast: true`:
- Interpolation parse/evaluation errors return immediately
- TOC-linking and other non-structural transclusion failures return immediately
- Structural transclusion errors still return immediately

## Transclusion

The transclusion phase runs after Inline Pre when a source file path is provided. It resolves file-based includes.

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

- **Cycle detection**: Prevents infinite recursion from ancestry repetition while allowing shared DAG dependencies
- **Max depth limits**: Configurable depth for nested transclusion
- **Heading re-leveling**: Included markdown headings are adjusted to fit the nesting context (H6 overflow handled gracefully)
- **TOC linking source model**: `::toc-linking` reads headings from the referenced file's raw source, not its recursively composed output

## Module Structure

```
darkmatter/lib/src/markdown/compose/
├── mod.rs           # Public API, pipeline orchestration
├── types.rs         # ComposeOperation, ComposeOptions, ComposeReport, etc.
├── schema_validation.rs # Always-on schema validation stage
├── state.rs         # EffectiveState, merge logic
├── replacement.rs   # Text replacement engine
├── link_resolve.rs  # Link resolution (absolute paths)
├── link_normalization.rs # Link normalization (portable paths)
└── interpolation/
    ├── mod.rs       # Module exports
    ├── lexer.rs     # Tokenizer, expression finder
    ├── ast.rs       # AST types
    ├── parser.rs    # Expression parser
    └── evaluator.rs # AST evaluation
```
