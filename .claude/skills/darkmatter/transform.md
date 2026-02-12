# Transform Pipeline

The darkmatter transform pipeline provides document preparation capabilities through the `Markdown::transform()` family of methods.

## Pipeline Stages

The pipeline executes four stages in order:

```
Input Document
    │
    ▼
┌─────────────────────┐
│ 1. Text Replacement │  Replace literal strings from frontmatter
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ 2. Interpolation    │  Expand {{ variable }} expressions
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ 3. Cleanup          │  Normalize markdown formatting
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│ 4. Normalization    │  Adjust heading levels
└─────────────────────┘
    │
    ▼
Output Document + Report
```

## API

```rust
use darkmatter::markdown::{Markdown, transform::{TransformOptions, Stage1Stages}};

// Transform with defaults
let (transformed, report) = md.transform()?;

// Transform with options
let options = TransformOptions::new()
    .with_external_state(json!({"key": "value"}))
    .with_stages(Stage1Stages::only_interpolation())
    .with_fail_fast(true);
let (transformed, report) = md.transform_with(options)?;

// In-place mutation (no clone)
let report = md.transform_mut()?;
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

## TransformReport

```rust
pub struct TransformReport {
    pub replacements_applied: usize,
    pub interpolations_applied: usize,
    pub cleanup_changed: bool,
    pub normalization_report: Option<NormalizationReport>,
    pub warnings: Vec<TransformWarning>,
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

## Module Structure

```
darkmatter/lib/src/markdown/transform/
├── mod.rs           # Public API, pipeline orchestration
├── types.rs         # TransformOptions, TransformReport, etc.
├── state.rs         # EffectiveState, merge logic
├── replacement.rs   # Text replacement engine
└── interpolation/
    ├── mod.rs       # Module exports
    ├── lexer.rs     # Tokenizer, expression finder
    ├── ast.rs       # AST types
    ├── parser.rs    # Expression parser
    └── evaluator.rs # AST evaluation
```
