# Diagrams

`biscuit-terminal` puts pixels on screen; it does not know about diagrams.
The diagram components (`MermaidDiagram`, `GraphExpression`, `GitGraph`) live
in `biscuit-visualized` behind its `terminal` feature (load the
`biscuit-visualized` skill, `diagram-components.md`). The dependency runs one
way: `biscuit-visualized` depends on this crate. Never add a diagram import or
a `biscuit-visualized` dependency to the library; that restores a cycle.

## Render-tree diagram hook

The render-tree renderer draws a `mermaid` fence only through a
caller-installed `DiagramRenderer` (`render_tree::{DiagramRenderer,
DiagramRenderRequest, DiagramOutcome, DiagramRenderError}`) on
`TerminalRenderOptions::diagram_renderer` / `with_diagram_renderer`:

- consulted **before** the `CodeRenderer` hook, with `TerminalMermaidMode::Image`
  at `GraphicsMode::Rich`, and with `TerminalMermaidMode::Automatic` at `Rich`
  or `Vector`; never at `Off`, never in `Code` mode
- the request carries the source, node attrs (layout is `attrs.layout`), the
  `force_graphics`-applied terminal, `available_width`, `graphics_mode`
  (the ceiling: a hook never answers with an image at `Vector`), and
  `mermaid_mode` (`Image` = image or error, `Automatic` = may answer with
  text); it is
  `#[non_exhaustive]`, so only this crate constructs it
- `Ok(DiagramOutcome::Rendered(s))`: `s` printed verbatim (an image)
- `Ok(DiagramOutcome::Nodes(fragment))`: laid out in place at the fence's
  width (`render_diagram_fragment`), code nodes through the `CodeRenderer`;
  rendered with `mermaid_mode = Code` so a `mermaid` source node in the
  fragment cannot re-enter the hook (that would recurse forever). Every
  finished row is then measured (`visible_width`); one wider than
  `available_width` makes the answer a `Declined` (fragment and its
  diagnostics dropped). This is the only place a code renderer's
  line-number gutter (`context.line_numbers`; digits + 3 cells in
  darkmatter) is accounted for, since the hook judges fit before it. Never
  applied to `Rendered`
- `Ok(DiagramOutcome::Declined)`: the ordinary code block, **no diagnostic**
  under Strict, Warn and Lossy
- no hook installed: the fence renders exactly as `TerminalMermaidMode::Code`
  would, with no diagnostic (a `tracing::debug!` notes the missing hook)
- hook `Err`: `Strict` returns `RenderError::LossyRejected` ("Mermaid promotion
  failed: …"); `Warn` records a lossy diagnostic and falls back to code;
  `Lossy` falls back silently
- cloned child options keep the hook (`Rc`), so nested fences reach it
- user-facing page: `biscuit-terminal/docs/render-tree.md`

`biscuit_visualized::components::TerminalDiagramRenderer` is the real hook;
darkmatter installs it. It decides image availability with
`TerminalImage::route_has_pixels(&Terminal)` (true iff `render_with_route`
would pick a pixel encoder: the format is not `None` and a Zellij route has a
measured cell size); use that, never a bare `image_support()`, to choose an
image route up front. It sizes the diagram to `available_width` (the
fence's columns: a list item's or quote's inner width), not the full terminal
width. The diagram components rasterize at that display size (width =
columns × cell width, height padded to whole cell rows) and hand the PNG to
`TerminalImage::from_encoded_png` unchanged (see
[Image Rendering](./image-rendering.md#already-encoded-png-encodedpng)), so
this crate never decodes or re-encodes a diagram for Kitty or iTerm2. `bt` has no render-tree path that promotes Mermaid, so
it installs none. The library's own tests use fakes
(`lib/tests/l1/render_tree_diagram_hook.rs`).

## Diagram colors (the OSC 4 palette)

The only diagram-specific thing this crate provides is color discovery.
`biscuit-visualized`'s `TerminalColors::detect()` (used by
`components::mermaid::MermaidDiagram::new`) reads
`discovery::osc_queries::{bg_color, text_color, ansi_palette}`; ANSI slots
1–6 become the diagram's six accents. `ansi_palette()` is `#[doc(hidden)]`,
not public API, and reads the once-per-process startup exchange: sixteen
`ESC ] 4 ; N ; ? BEL` queries ride with OSC 10/11 behind the same gate
(`palette_query_wanted()` = `color_query_wanted()`: TTY, not CI, no
multiplexer, probing on, not the Windows SSH server, a terminal known to
answer). **Native Windows sends them too**, through the same console query
backend (`tty_query/windows.rs`) as OSC 10/11, so Windows Terminal gives
diagrams its own accents. An unanswered slot is `None` (never an error);
`biscuit-visualized` then derives that accent from the background. See
[Discovery](./discovery.md).

## `bt` diagram commands

The CLI depends on `biscuit-visualized` (`clap`, `terminal`) for these
commands; the library does not. Rendering is pure Rust (Merman, a
Mermaid.js 12.2.0 reimplementation, and `layout-rs`); no Node.js or `mmdc` is
involved. Diagrams use the terminal-derived palette above; `--inverse` swaps
in the opposite fixed theme (`MermaidDiagram::inverted`) on a solid
background.

Without images (`TerminalImage::route_has_pixels(term)` false), `bt` takes
the component's fallible `MermaidDiagram::try_present` and lays the text
drawing (`txt` node + partial note) or the source (`mermaid` node) out with
darkmatter's code renderer at `text_columns`, then applies margins once
(`commands/mermaid.rs::render_mermaid_fallback`). Ineligible families, a
drawing too wide, and a policy that forbids text exit 0 with the source; a
source that does not parse, or that has a math label (`$$…$$`; Merman's
`math` capability is not built in), is an error and exits non-zero whenever
the source is shown (restrictive `--feature`, `MERMAID_FALLBACK=code-block`,
or a drawing too wide), via the component's parse-and-capability-plan check;
a text drawing that fits shows the formula literally and exits 0. Never use the infallible `present` in a
CLI: it hides the error. With images, the image route is unchanged: an
image-export failure reports the error and exits non-zero, and
`--meta`/`--debug` describe images only. Every Mermaid subcommand takes
`--feature` (`no-mermaid-fallbacks`, `no-mermaid-partial-fallbacks`,
`no-mermaid-partial-warning`; shared `MermaidFeatureArgs` in `cli/src/args.rs`)
folded into the component's `MermaidFallbackPolicy`; with none,
`MERMAID_FALLBACK=code-block` means source. User docs:
`biscuit-terminal/docs/cli.md`.

**Code highlighting in `bt`.** The CLI (not the library) depends on
darkmatter's library and installs its code renderer
(`commands/shared.rs::darkmatter_code_renderer`, the one allowed use of the
deprecated `TerminalCodeRenderer`) in the Mermaid fallback, `quote`, and
`block`, so `bt` and `md` highlight identically, Mermaid syntax included. The
chain `bt` → darkmatter → biscuit-visualized → biscuit-terminal has no cycle;
`cargo tree -p biscuit-visualized -p biscuit-terminal` must list neither
darkmatter nor syntect.

### Shared options

All diagram commands share common options:

| Option | Description |
|--------|-------------|
| `--example` / `-e` | Render example with command shown |
| `--width` / `-w` | Width: `50%`, `80ch`, `80`, or `fill` |
| `--inverse` | Solid background with inverted colors |
| `--title` / `-t` | Add title above diagram |
| `--json` | Output as JSON for scripting |

#### Flowchart

```bash
bt flowchart "A --> B --> C"
bt flowchart --vertical "Start --> Process --> End"
bt flowchart "A[Input] --> B{Decision}" "B -->|Yes| C[Output]"
```

Options:
- `--vertical`: Render top-down instead of left-right

#### Quadrant Chart

```bash
bt quadrant "Item A: [0.3, 0.6]" "Item B: [0.7, 0.4]"
bt quadrant --x-axis "Low --> High" --y-axis "Small --> Large" "Item: [0.5, 0.5]"
bt quadrant --theme magic-quadrangle "Leaders: [0.8, 0.8]" "Niche: [0.2, 0.2]"
```

Data points: `"Label: [x, y]"` where x,y are 0.0-1.0

Options:
- `-x`/`--x-axis`, `-y`/`--y-axis`: Axis labels
- `--top-left`, `--top-right`, `--bottom-left`, `--bottom-right`: Quadrant labels
- `--theme`: `default` or `magic-quadrangle`
- `--point-radius`, `--label-size`: Point styling
- `--q1-fill` through `--q4-fill`: Custom quadrant colors (hex)

Inline point styling:
```bash
bt quadrant "Item: [0.5, 0.5] color: #ff3300, radius: 10"
```

#### Pie Chart

```bash
bt pie-chart "Dogs: 386" "Cats: 85" "Birds: 15"
bt pie-chart --show-data "Dogs: 386" "Cats: 85"
bt pie-chart "TypeScript: 45 #3178c6" "Rust: 35 #dea584"
```

Input formats:
- Simple: `"Label: value"`
- Semicolon-delimited: `"Label1: 10; Label2: 20"`
- With color: `"Label: value #rrggbb"` or `"Label: value color: #rrggbb"`

Options:
- `--show-data`: Show percentages on slices

#### Git Graph

```bash
bt git-graph "commit" "branch feature" "checkout feature" "commit" "merge feature"
bt git-graph "commit id: \"abc123\"" "commit tag: \"v1.0\""
```

Commands: `commit`, `branch <name>`, `checkout <name>`, `merge <name>`, `cherry-pick id: "..."`

#### Bar Chart

```bash
bt bar-chart 10 20 15 25
bt bar-chart --x-axis "Q1,Q2,Q3,Q4" --y-axis Sales 10 20 15 25
bt bar-chart --horizontal --show-data-label 1 8 7 5
bt bar-chart --line 10 20 15 25  # Add trend line
bt bar-chart --aspect-ratio 2.0 --width 60% --inverse 10 20 15 25
```

Input formats: JSON array `"[1,8,7]"`, comma-separated `"1,8,7"`, or space-separated `1 8 7`

Options:
- `--horizontal`: Horizontal bars
- `--show-data-label`: Show values on bars
- `--line`: Also render as line
- `--aspect-ratio`: Width/height ratio (default: 1.5)

#### Line Chart

```bash
bt line-chart 1 8 7 5 9 3
bt line-chart --x-axis "Mon,Tue,Wed" --y-axis Temperature 20 22 19
bt line-chart --bar 1 8 7 5  # Add bars under line
bt line-chart --show-data-label --horizontal 1 8 7 5
bt line-chart --aspect-ratio 1.8 --inverse --width 60% 1 8 7 5
```

Same options as bar-chart, plus `--bar` to add bars.

#### Timeline

```bash
bt timeline "2020: Project started" "2021: First release" "2022: Major update"
bt timeline --section "Early Years" "2020: Founded" --section "Growth" "2022: Series A"
```

Format: `"YYYY: Description"`

Options:
- `-s`/`--section`: Group events into sections

#### State Diagram

```bash
bt state-diagram "[*] --> Idle" "Idle --> Running" "Running --> [*]"
bt state-diagram "[*] --> Idle" "Idle --> Running: start" "Running --> Stopped: stop"
```

Syntax:
- `[*]`: Start/end state
- `State1 --> State2`: Transition
- `State1 --> State2: label`: Labeled transition

#### Entity Relationship Diagram

```bash
bt erd "Customer ||--o{ Order : places" "Order ||--|{ LineItem : contains"
bt erd --entity "Customer { id int PK, name string }" "Customer ||--o{ Order : places"
```

Relationships:
- `||--||`: One to one
- `||--o{`: One to many
- `}o--o{`: Many to many
- `||--o|`: One to zero or one

Options:
- `-E`/`--entity`: Entity definition with attributes

## Related

- [bt Command](./cli.md) - CLI reference
- [Image Rendering](./image-rendering.md) - Image protocol details
- [Render-tree renderer](./render-tree.md)
