# Biscuit Terminal

<table>
  <tr>
    <td><img src="../assets/biscuit-terminal-512.png" style="max-width='25%'" width=200px /></td>
    <td>
      <h2>biscuit-terminal</h2>
      <p>This shared library provides support for working with the terminal:</p>
      <ul>
        <li>terminal <b>metadata</b> (<i>13+ terminal emulators, color depth, light/dark mode, dimensions, font, OS</i>)</li>
        <li>OSC8 links, OSC52 clipboard, and OSC10/11/12 color queries</li>
        <li>multiplex detection for tmux/Zellij plus native WezTerm/Ghostty/Kitty support</li>
        <li>inline image rendering via Kitty/iTerm2 protocols with graceful fallback</li>
        <li>terminal-facing Mermaid rendering via <code>MermaidDiagram</code>, backed by <code>biscuit-visualized</code> (pure Rust)</li>
        <li>terminal-facing graph rendering via <code>GraphExpression</code>, with arrow, dash, and DOT syntax support</li>
        <li>color system: BasicColor (16 ANSI), RgbColor, WebColor (148 CSS), Tailwind (22 families × 11 shades)</li>
        <li>composable rendering components: Prose, Table, List, Section, StatusBlock, FileSystem, TwoColumn, and more</li>
      </ul>
    </td>
  </tr>
</table>

## About

The `biscuit-terminal` package area contains both a Library and CLI which focus on two distinct but complementary things:

1. Feature Discovery

    The `biscuit-terminal` library is able to provide rich feature discovery on terminals to identify things like:

    - Color Depth
    - Terminal Size (cols and rows)
    - OSC8 Link Support
    - OSC10 and OSC11 support for terminal querying
    - Image rendering supporting
    - Background color and Color Mode (light/dark)
    - Support for Underlining variants (straight, double, curly, dotted, curly, dashed) and whether underlining can have it's own color
    - The name of the terminal app being used
    - The terminal's locale, and character encoding
    - Whether a nerd font is being used in terminal

    > this discovery in the library is enabled by the `Terminal` struct.

    When a user installs the CLI, they can run `bt` without any parameters to see the information that the `biscuit-terminal` library has discovered about the current terminal.

2. Component Rendering

    The `biscuit-terminal` library defines a **Renderable** trait which provides a consistent interface for components. These components include:

    - [`BlockQuote`](./docs/components/block_quote.md)
    - [`FileSystem`](./docs/components/file_system.md)
    - [`GraphExpression`](./docs/components/graph_expression.md) (graph adapter backed by `biscuit-visualized`)
    - [`MermaidDiagram`](./docs/components/mermaid_diagram.md) (Mermaid adapter backed by `biscuit-visualized`)
    - [`OrderedList` and `UnorderedList`](./docs/components/list.md)
    - [`PadLeft`](./docs/components/pad_left.md) and [`PadRight`](./docs/components/pad_right.md)
    - [`Progress`](./docs/components/progress.md)
    - [`Prose`](./docs/components/prose.md) — capability-aware styling with
      [graceful degradation](./docs/components/prose.md#graceful-degradation) for
      OSC8 hyperlinks and double-underline on terminals like Apple Terminal
    - [`Section`](./docs/components/section.md)
    - [`Status`](./docs/components/status.md)
    - `StatusBlock`
    - [`Table`](./docs/components/table.md)
    - [`TerminalImage`](./docs/components/terminal_image.md)
    - [`TextBlock`](./docs/components/text_block.md)
    - [`Todo`](./docs/components/todo.md)
    - [`TwoColumn`](./docs/components/two_column.md)

    As well as compositional components:

    - [`Compose`](./docs/components/compose.md) and [`InlineContent`](./docs/components/inline_content.md)

    These components all respect the `Layout` struct's ideas of margins, word-wrap, and other useful features.

## StatusBlock

`StatusBlock` is the canonical composite for Claudine-style warning and error surfaces. It
combines an optional `Status` header, an optional bordered body, and an optional hint into a
single renderable with severity-derived defaults.

```rust
use biscuit_terminal::prelude::{Prose, StatusBlock, StatusState};

let block = StatusBlock::new(StatusState::Error)
    .header("<b>Shell Expansion Failed</b>")
    .body(Prose::new("Missing closing brace in `${...}` directive."))
    .hint("Check the template syntax and retry.");
```

Default behavior:

- `border = "┃ "`
- `left_margin = 0`
- `right_margin = 5`
- `word_wrap = WordWrap::WrapProse(Some(8), None)`

Those defaults are chosen so the `┃` border aligns visually with a preceding `Status` icon/header
line. You can override the border color with `.border_color(...)` and the glyph with
`.border(...)` when a call site needs a different visual treatment.

Severity defaults:

| StatusState | Default color |
|-------------|---------------|
| `Error` | `Tailwind::Red500` |
| `Warning` | `Tailwind::Orange500` |
| `Info` | `Tailwind::Blue500` |
| `Success` | `Tailwind::Green500` |
| `NotStarted` | `Tailwind::Gray500` |
| `Active` | `Tailwind::Gray600` |
| `ToolUse` | `Tailwind::Purple700` |
| `Subagent` | `Tailwind::Violet500` |

Use `StatusState::default_color()` when you want the canonical accent or border color for a
severity without duplicating the mapping yourself.

`StatusState::Failure` is deprecated in favor of `StatusState::Error`. Persisted JSON using
`"Failure"` remains compatible because it deserializes through the alias on `Error`.

## BlockError

`BlockError` is the terminal-rendering contract for errors. Any
`std::error::Error` can implement `BlockError` to produce a consistent
**Block Style Error** — a `Status` title line over a red-bordered
`StatusBlock` body with an optional hint — using a single trait method.

```rust
use std::error::Error;
use std::fmt;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::prelude::*;

#[derive(Debug)]
struct CycleDetected { chain: Vec<String> }

impl fmt::Display for CycleDetected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cycle detected: {}", self.chain.join(" -> "))
    }
}

impl Error for CycleDetected {}

impl BlockError for CycleDetected {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        StatusBlock::new(self.severity())
            .error_header(ErrorHeader::new("CycleDetected", "transclusion cycle"))
            .body(format!("chain: {}", self.chain.join(" -> ")))
            .hint("Break the cycle by removing one of the edges.")
    }
}

let err = CycleDetected { chain: vec!["a.md".into(), "b.md".into(), "a.md".into()] };
let term = Terminal::new();
eprintln!("{}", err.report_block_error(&term));
```

Key methods:

- `status_block(&Terminal) -> StatusBlock` — the single required method. Build
  a configured (but unrendered) block; the default `report_block_error`
  renders it for you.
- `severity() -> StatusState` — defaults to `Error`. Override for warnings.
- `report_block_error(&Terminal) -> String` — default composes
  `status_block(term).render(term)`. Override to add a cause chain.
- `report_block_error_optimistic(Option<u32>) -> String` — non-TTY fallback
  using an 80-column optimistic terminal. Great for logs, pipelines, and
  tests.
- `block_source() -> Option<&dyn BlockError>` — wrapper errors return their
  inner `BlockError` here; see `render_with_causes`.

Supporting helpers live alongside the trait:

- `ErrorHeader::new("ErrorName", "summary")` — canonical title line shape.
- `StatusBlockExt::error_header(ErrorHeader)` — sets the header in one call.
- `render_with_causes(&err, &term)` — stacks wrapper + nested-cause blocks
  under a dim `Caused by:` caption.

See
[`darkmatter/docs/error-rendering.md`](../darkmatter/docs/error-rendering.md)
for an end-to-end rendering contract and adoption guide.

Each downstream crate that implements `BlockError` for its own error types is
responsible for exposing its own downcast registry function (e.g.
`darkmatter::markdown::errors::as_block_error`). The crate-level
`as_block_error` in `biscuit_terminal::errors` returns `None` unconditionally
so that callers can chain `.or_else(crate_local_registry)` without conflict.
See the darkmatter registry in
[`darkmatter/lib/src/markdown/errors/mod.rs`](../darkmatter/lib/src/markdown/errors/mod.rs)
for a concrete example.

## Testing

biscuit-terminal follows the **Level 1 / 2 / 3** testing vocabulary:

| Level | Description | Location |
|-------|-------------|----------|
| **Level 1** | PTY-based tests using `expectrl` — no real terminal required | `lib/tests/level1_*.rs` |
| **Level 2** | Real-terminal tests using the shared `biscuit-test-harness` crate | `cli/tests/level2_*.rs` |
| **Level 3** | OS-level keyboard injection (not applicable — biscuit-terminal has no interactive input) | — |

### Running Level-2 tests locally

Level-2 tests require a running terminal emulator with remote-control enabled:

**WezTerm:**
```bash
export WEZTERM_UNIX_SOCKET="/path/to/wezterm.sock"
just test-l2
```

**Kitty:**
```bash
export KITTY_LISTEN_ON="unix:/path/to/kitty.sock"
just test-l2
```

Both environment variables are normally set automatically by the respective terminal emulator for child shells. If you started WezTerm or Kitty from a launcher (e.g., macOS Dock, Spotlight, or Linux .desktop file), the variables may not propagate to your test shell. In that case, locate the socket and export it manually before running tests.

### CI behaviour

Level-2 tests **skip cleanly** when the required terminal is unavailable — no `#[ignore]` markers are used. On GitHub-hosted runners (which lack WezTerm and Kitty), the tests print `skipping: requires <X>` and exit successfully. This keeps CI green while still providing a local regression net on developer machines.

### Running only Level-2 tests

```bash
just test-l2          # all Level-2 tests
just test-l2 -- --nocapture   # with output visible
```

## More Information

For more information on either the library or CLI refer to more detailed documents on each:

- [Biscuit Terminal Library](./lib/README.md) for details on how to use `biscuit-terminal` programmatically
- [The Biscuit Terminal CLI](./cli/README.md) for details on how to leverage `biscuit-terminal` from the terminal
