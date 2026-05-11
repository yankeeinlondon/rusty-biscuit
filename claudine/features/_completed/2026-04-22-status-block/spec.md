# StatusBlock struct for Block Errors

There are many cases where the appropriate way to report an error comes with a decent amount of context, possibly a code block for reference, maybe some hints on what to do.

## StatusState Rename

Before introducing `StatusBlock`, we will evolve the existing `StatusState` enum so that `Error` is the preferred variant name while preserving backward compatibility with persisted data and existing call sites.

This feature lands the rename as an **aliased deprecation**, not an atomic rename:

- Add a new `StatusState::Error` variant.
- Mark `StatusState::Failure` with `#[deprecated(note = "use StatusState::Error instead")]`.
- Add `#[serde(alias = "Failure")]` on `Error` so any JSON persisted with `"Failure"` still deserializes into `Error`.
- `StatusBlock` uses `StatusState::Error` from day one.
- Migrating the ~23 in-repo call sites that still reference `StatusState::Failure` and then removing the `Failure` variant is **out of scope** for this feature and will be handled in a follow-up PR.

## StatusState::default_color()

Add a new public method on `StatusState` in `biscuit-terminal/lib/src/components/status.rs`:

```rust
impl StatusState {
    /// Canonical Tailwind color for this variant.
    ///
    /// Shared by `Status` (icon color) and `StatusBlock` (default border color)
    /// so both components present a harmonized palette.
    pub fn default_color(&self) -> Color { /* ... */ }
}
```

- The mapping returned by `default_color()` matches the colors already used in the `Status` component's `ICON_LOOKUP` table (`biscuit-terminal/lib/src/components/status.rs:93-169`).
- `StatusBlock` consumes `default_color()` directly for its severity-derived border color.
- Refactoring `ICON_LOOKUP` in `Status` to consume `default_color()` (to eliminate the duplicated per-variant color literals) is **out of scope** for this feature and will be handled in a follow-up PR. For this feature, the method is added and `StatusBlock` uses it; `Status` keeps its existing inlined colors.

## StatusBlock Component

We will create `StatusBlock` struct:

```rust
#[derive(Debug, Clone)]
pub struct StatusBlock {
    severity: StatusState,
    header: Option<String>,
    body: Option<RenderableContent>,
    hint: Option<String>,
    border_color: Option<Color>,
    border: String,
    layout: Layout,
}
```

This component will reuse the `StatusState` enum which is used for the `Status` component.

### Severity defaults

Because `severity: StatusState` and `StatusState` has 8 variants, the default border colour is defined for every variant:

| `StatusState` | Default border color |
| --- | --- |
| `Error` | `Red500` (Tailwind) |
| `Warning` | `Orange500` (Tailwind) — harmonized with `Status` icon |
| `Info` | `Blue500` (Tailwind) |
| `Success` | `Green500` (Tailwind) |
| `NotStarted` | `Gray500` (Tailwind) |
| `Active` | `Gray600` (Tailwind) — harmonized with `Status` icon |
| `ToolUse` | `Purple500` (Tailwind) — harmonized with `Status` icon |
| `Subagent` | `Violet500` (Tailwind) — harmonized with `Status` icon |

These defaults are supplied by `StatusState::default_color()` (see section above), so `StatusBlock`'s severity-derived border color stays in lock-step with the `Status` icon palette.

Deprecated `Failure` maps to the same color as `Error` (`Red500`) since `#[serde(alias = "Failure")]` aliases it to `Error`.

Default values will be:

| Field                 | Default                              |
| --------------------- | ------------------------------------ |
| `severity`            | (required)                           |
| `header`              | `None`                               |
| `body`                | `None`                               |
| `hint`                | `None`                               |
| `border_color`        | derived from `severity`              |
| `border`              | `"▌ "`                               |
| `layout.left_margin`  | `Margin::Chars(0)`                   |
| `layout.right_margin` | `Margin::Chars(5)`                   |
| `layout.word_wrap`    | `WordWrap::WrapProse(Some(8), None)` |

**Alignment rationale.** The `▌` border glyph is intended to sit in the same column as the `Status` icon on a preceding header line. Because `Status` has no left margin (its `Layout::default()` produces `Margin::None`, which resolves to 0 columns), `StatusBlock`'s `left_margin` is also `Margin::Chars(0)` so the bar and the icon align vertically. The `"▌ "` glyph carries a single-character trailing gap so body content isn't flush against the bar. The `right_margin` of `Margin::Chars(5)` leaves enough whitespace at the right edge for comfortable reading without compressing the rendered width.

### Builder Methods

```rust
/// Set a prose-formatted header line rendered as a Status outside the block quote.
///
/// When present, renders as `Status::from_prose(header).state(severity)`.
/// Prose markup is supported (e.g. `<blue>{rel_path}</blue>`).
pub fn header(mut self, header: impl Into<String>) -> Self;

/// Set the body content rendered inside the BlockQuote.
///
/// Accepts any `RenderableContent` — plain string, `Prose`, `Compose`, or
/// pre-rendered text (e.g. darkmatter output).
pub fn body(mut self, body: impl Into<RenderableContent>) -> Self;

/// Set a prose-formatted hint rendered below the block quote.
///
/// Intended for actionable advice (e.g. "Check that `just` is on your `PATH`").
pub fn hint(mut self, hint: impl Into<String>) -> Self;

/// Override the severity-derived border colour.
pub fn border_color(mut self, color: Color) -> Self;

/// Override the border glyph (default: `"▌ "`).
pub fn border(mut self, border: impl Into<String>) -> Self;
```

The standard `Renderable` layout builders (`left_margin`, `right_margin`, `word_wrap`, etc.) are inherited automatically through the trait default methods.

### Renderable implementation

`StatusBlock` implements `Renderable` with `is_block_level() → true`.

#### Render output layout

```text
{header Status line}\n           ← only if header is Some
\n                               ← blank separator between header and body
{BlockQuote(body)}\n             ← only if body is Some
\n                               ← blank separator between body and hint
{Prose(hint)}\n                  ← only if hint is Some
```

When only a subset of parts is present, the blank separators collapse:

- Header only → just the header line (no trailing blank)
- Body only → just the BlockQuote (no leading/trailing blank)
- Body + hint → BlockQuote + newline + Prose hint
- Header + body → header + newline + BlockQuote
- All three → header + newline + BlockQuote + newline + Prose hint

The rendered string does **not** end with a trailing blank line; callers manage surrounding spacing.

#### Pseudocode

```rust
fn render(&self, term: &Terminal) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(ref header_text) = self.header {
        // `severity` already IS a `StatusState`, so it is passed through as-is.
        let status = Status::from_prose(header_text)
            .state(self.severity.clone());
        parts.push(status.render(term));
    }

    if let Some(ref body) = self.body {
        let mut block = BlockQuote::new(body.clone(), None::<&str>)
            .with_left_block_color(self.resolved_border_color())
            .with_border(&self.border);
        block.layout_mut().left_margin = self.layout.left_margin.clone();
        block.layout_mut().right_margin = self.layout.right_margin.clone();
        block.layout_mut().word_wrap = self.layout.word_wrap.clone();
        parts.push(block.render(term));
    }

    if let Some(ref hint_text) = self.hint {
        parts.push(Prose::new(hint_text).render(term));
    }

    parts.join("\n")
}
```

## Migration

### Claudine Call Sites

### 1. `live_semantic_sink::render_error_block`

Before:

```rust
let (label, border_color) = error_kind_presentation(kind);
let body = format!("<red><b>{label}</b></red>\n{escaped}");
let prose = Prose::new(body).with_word_wrap(WordWrap::WrapProse(None, None));
let mut block = BlockQuote::new(RenderableContent::from(prose), None::<&str>)
    .with_left_block_color(border_color)
    .with_border("▌ ");
block.layout_mut().left_margin = Margin::Chars(0);
block.layout_mut().right_margin = Margin::Chars(0);
let rendered = block.render(&self.terminal);
for line in rendered.lines() {
    self.emit_section_line(section, line);
}
```

After:

```rust
let (label, border_color) = error_kind_presentation(kind);
let body = format!("<red><b>{label}</b></red>\n{escaped}");
// NOTE: `.left_margin(Margin::Chars(0))` is now redundant with the new
// `StatusBlock` default, but kept here for traceability with the pre-migration
// behavior. `.right_margin(Margin::Chars(0))` is an intentional override of
// the new default (`Margin::Chars(5)`) to preserve the current rendered width.
let block = StatusBlock::new(StatusState::Error)
    .body(Prose::new(body))
    .border_color(border_color)
    .left_margin(Margin::Chars(0))
    .right_margin(Margin::Chars(0));
let rendered = block.render(&self.terminal);
for line in rendered.lines() {
    self.emit_section_line(section, line);
}
```

### 2. `live_semantic_sink::render_warning_header_and_body`

Before:

```rust
let header_rendered = Status::from_prose(header_prose)
    .state(StatusState::Warning)
    .render(&self.terminal);
for line in header_rendered.lines() {
    self.emit_section_line(section, line);
}
let body = Prose::new(body_prose.to_string())
    .with_word_wrap(WordWrap::WrapProse(None, None));
let mut block = BlockQuote::new(RenderableContent::from(body), None::<&str>)
    .with_left_block_color(Color::Tailwind(Tailwind::Orange700))
    .with_border("┃ ");
block.layout_mut().left_margin = Margin::Chars(0);
block.layout_mut().right_margin = Margin::Chars(0);
let body_rendered = block.render(&self.terminal);
for line in body_rendered.lines() {
    self.emit_section_line(section, line);
}
```

After:

```rust
// `header_prose: String` and `body_prose: &str` per the existing
// `render_warning_header_and_body` signature — both flow through the
// builder unchanged.
//
// The pre-migration code used `Tailwind::Orange700` for the warning border.
// The new `StatusBlock` default for `Warning` is `Orange500` (harmonized with
// the `Status` icon). We explicitly override with `Orange700` here so this
// migration is a pure refactor with no visual color change. If we later want
// to adopt the harmonized default, drop the `.border_color(...)` line.
//
// `.left_margin(Margin::Chars(0))` is redundant with the new default; kept
// for traceability. `.right_margin(Margin::Chars(0))` is an intentional
// override of the new default (`Margin::Chars(5)`).
let block = StatusBlock::new(StatusState::Warning)
    .header(header_prose)
    .body(Prose::new(body_prose.to_string()))
    .border_color(Color::Tailwind(Tailwind::Orange700))
    .border("┃ ")
    .left_margin(Margin::Chars(0))
    .right_margin(Margin::Chars(0));
let rendered = block.render(&self.terminal);
for line in rendered.lines() {
    self.emit_section_line(section, line);
}
```

### 3. `shell_expansion_error::ShellExpansionReport`

Before: `ShellExpansionReport` struct with `header: Status`, `body: Option<BlockQuote>`, `hint: Option<String>`, plus `build_header`, `build_body`, `build_hint` functions that manually wire Status + BlockQuote + Prose.

After:

```rust
fn render_with_terminal(source_path: &Path, error: &ShellExpansionError, term: &Terminal) {
    let report = build_error_block(source_path, error);
    log::message("");
    log::message(&report.render(term));
    log::message("");
}
```

Where `build_error_block` returns a `StatusBlock`:

```rust
fn build_error_block(source_path: &Path, error: &ShellExpansionError) -> StatusBlock {
    let relative = relative_to_cwd(&canonicalize_or_self(source_path));
    let header_markup = build_header_prose(&relative, error);
    let body = build_body_content(source_path, error);
    let hint = build_hint(error);

    StatusBlock::new(StatusState::Error)
        .header(header_markup)
        .body(body)
        .hint(hint)
}
```

> Note: `body(body)` relies on the blanket `impl<T: Renderable + 'static> From<T> for RenderableContent` in `biscuit-terminal/lib/src/components/renderable.rs`, so an explicit `RenderableContent::from(body)` is redundant here.

### 3.5. `live_semantic_sink::render_file_tool_error`

No direct changes needed — `render_file_tool_error` is a thin wrapper that builds a header string and delegates to `render_warning_header_and_body`, which is migrated in section 2. The checklist item is retained below for traceability.

### 4. `error_report::AgentErrorReport`

Before: manually assembles a `Compose` with title + body list + hint + suggestions, wraps it in `BlockQuote` with margins and border.

After:

```rust
let mut compose = Compose::default();
compose.add_prose(title_prose);
if let Some(body_list) = &self.body_list { /* add items */ }
if let Some(hint) = &self.hint { /* add hint */ }
if let Some(suggestions) = &self.suggestions { /* add suggestions */ }

// `AgentErrorReport` intentionally overrides both margins: the pre-migration
// layout used 2-char left/right margins, which we preserve here. These are
// intentional overrides of the new `StatusBlock` defaults (`left_margin = 0`,
// `right_margin = 5`).
let block = StatusBlock::new(StatusState::Error)
    .body(compose)
    .border_color(border_color)
    .left_margin(Margin::Chars(2))
    .right_margin(Margin::Chars(2));
log::message("");
log::message(&block.render(term));
log::message("");
```

## Tests

Unit tests in `status_block.rs` following the conventions in `block_quote.rs`:

| Test                                     | Description                                                   |
| ---------------------------------------- | ------------------------------------------------------------- |
| `body_only`                              | BlockQuote rendered with red border, no header or hint        |
| `with_header`                            | Status header line above the BlockQuote                       |
| `with_hint`                              | Prose hint below the BlockQuote                               |
| `all_parts`                              | Header + body + hint with correct spacing                     |
| `error_severity_uses_red500`             | `StatusState::Error` resolves to `Red500` border by default   |
| `warning_severity_colors`                | Warning uses `Orange500` and `StatusState::Warning`           |
| `info_severity_colors`                   | Info uses `Blue500` and `StatusState::Info`                   |
| `default_color_matches_status_icon`      | `StatusBlock::new(state).resolved_border_color()` equals `StatusState::default_color()` for at least `Error`, `Warning`, `Info`, `Success`, `Active`, `ToolUse`, `Subagent` |
| `custom_border_color_overrides_severity` | Explicit `border_color()` wins over severity default          |
| `custom_border_glyph`                    | `border("┃ ")` replaces default `"▌ "`                        |
| `body_from_plain_string`                 | `body("plain text")` renders inside BlockQuote                |
| `body_from_prose`                        | `body(Prose::new("<b>bold</b>"))` renders styled content      |
| `body_from_compose`                      | `body(compose)` renders composed parts inside BlockQuote      |
| `margins_respected`                      | Custom left/right margins narrow the BlockQuote               |
| `render_optimistic_matches_render`       | Both render paths produce equivalent output                   |
| `is_block_level`                         | Returns `true`                                                |
| `clone_preserves_all_fields`             | Cloned instance renders identically                           |
| `debug_output`                           | `format!("{:?}", block)` contains `"StatusBlock"`             |
| `empty_body_no_block_quote`              | When body is `None`, no BlockQuote is emitted                 |

## Implementation checklist

- [ ] Add `StatusState::Error` variant, mark `StatusState::Failure` `#[deprecated]`, and add `#[serde(alias = "Failure")]` on `Error` for backward compatibility
- [ ] Add public `StatusState::default_color()` method in `biscuit-terminal/lib/src/components/status.rs` returning the canonical Tailwind color per variant
- [ ] Create `biscuit-terminal/lib/src/components/status_block.rs` with `StatusBlock` and full `Renderable` impl
- [ ] Register in `components/mod.rs`
- [ ] Export from `prelude.rs`
- [ ] Add unit tests (above table)
- [ ] Migrate `claudine/cli/src/commands/wrap/live_semantic_sink.rs::render_error_block`
- [ ] Migrate `claudine/cli/src/commands/wrap/live_semantic_sink.rs::render_warning_header_and_body`
- [ ] Migrate `claudine/cli/src/commands/wrap/live_semantic_sink.rs::render_file_tool_error` (delegates to `render_warning_header_and_body`; no direct code change)
- [ ] Migrate `claudine/cli/src/output/shell_expansion_error.rs`
- [ ] Migrate `claudine/cli/src/output/error_report.rs`
- [ ] Update `.claude/skills/biscuit-terminal/SKILL.md` and `biscuit-terminal/README.md`
