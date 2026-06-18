---
suggestions: 18
suggestions_critical: 4
suggestions_urgent: 6
---

# biscuit-terminal Sentrux Quality Review — 2026-05-04

> **Note on baseline:** the documented baseline at
> `biscuit-terminal/.sentrux/baseline.json` was not present at review time and
> the Sentrux MCP/CLI tools required interactive permission grants that this
> non-interactive session cannot satisfy. Findings below are derived from a
> manual structural inspection of the source tree (file size, module
> declarations, use-graph spot checks, and impl-block density) using the same
> dimensions Sentrux scores: **modularity** (Newman 2004), **acyclicity**
> (Martin 2003), **depth** (Lakos 1996), **equality** (Gini 1912), and
> **redundancy** (Kolmogorov). Re-run `sentrux scan` and re-grade once
> permissions allow.

## biscuit-terminal-cli

### `critical`: All-in-one `cli/src/commands.rs` god file (2,226 lines, 45 free functions)

**Problem.** `cli/src/commands.rs` holds rendering logic for **every**
subcommand — image, all 10 Mermaid types, graph-expression, prose, quote,
list, columns, padleft/padright, and dir — plus the chart helpers
(`parse_pie_data`, `parse_xy_data`, `extract_color`, `parse_hex_color`,
`build_pie_init_directive`, etc.) and SGR stripping. With 45 free functions in
one file, **equality** (Gini) and **modularity** (Newman) scores both suffer.

**Files touched.** `biscuit-terminal/cli/src/commands.rs`.

**Fix.** Convert `commands.rs` to `commands/` with one file per subcommand
family:

```
cli/src/commands/
├── mod.rs              // pub use re-exports + shared helpers
├── shared.rs           // is_dark_mode, detect_terminal_*, apply_*_layout, output_render_meta
├── image.rs            // render_image
├── mermaid.rs          // build_mermaid_diagram, display_mermaid, handle_mermaid_error
├── flowchart.rs
├── quadrant.rs
├── pie.rs              // parse_pie_data, parse_single_pie_entry, build_pie_init_directive, render_pie_chart
├── git_graph.rs
├── xy_chart.rs         // parse_xy_data, render_xy_chart
├── timeline.rs
├── state_diagram.rs
├── erd.rs
├── graph.rs            // display_graph, render_graph_expression, handle_graph_error
├── prose.rs
├── pad.rs              // render_pad_left, render_pad_right
├── quote.rs
├── list.rs
├── columns.rs
├── dir.rs              // render_dir + strip_sgr_sequences
└── color_parse.rs      // extract_color, parse_hex_color (shared by chart commands)
```

This also unblocks the next `urgent` suggestion (slim down `main.rs`).

### `urgent`: `cli/src/main.rs` has a ~500-line subcommand `match` block

**Problem.** `main()` (`main.rs:71-574`) destructures every subcommand variant
in a single match, builds local strings, and forwards 5–20 arguments to
`render_*` functions. Every new command grows this single function. Hurts
**equality** (one function owns most CLI edges) and **modularity**.

**Files touched.** `biscuit-terminal/cli/src/main.rs`,
`biscuit-terminal/cli/src/args.rs`.

**Fix.** Add a `Run` trait that each `Command` variant dispatches itself, and
move the per-variant rendering call into the corresponding subcommand struct.
With clap-derive this is most cleanly done by promoting each variant body to a
`#[derive(Args)]` struct and letting `Command::run(&self, ctx)` delegate:

```rust
// cli/src/args.rs
#[derive(Subcommand, Debug)]
pub enum Command {
    Image(ImageArgs),
    Flowchart(FlowchartArgs),
    Quadrant(QuadrantArgs),
    // …
}

pub trait Run {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()>;
}

// cli/src/main.rs
match args.command {
    Some(cmd) => cmd.run(&ctx),
    None => print_default(&ctx),
}
```

Each variant impl lives next to its renderer in `commands/<name>.rs` (see the
`critical` `commands.rs` split above).

### `urgent`: `cli/src/args.rs` is a 1,448-line monolith

**Problem.** All 17 subcommand variants and their inline `#[command]`/`#[arg]`
configuration are crammed into one file along with `Args`, `LayoutArgs`,
`ShellType`, `GraphInputSyntaxArg`, `GraphOrientationArg`, and the
completion-helper imports. Touching one subcommand’s flags forces re-reading
the entire file.

**Files touched.** `biscuit-terminal/cli/src/args.rs`.

**Fix.** Once the variants are promoted to per-command structs (previous
suggestion), move each `XxxArgs` struct into its `commands/<name>.rs`. `args.rs`
shrinks to the top-level `Args`, the `Command` enum, the small shared
`LayoutArgs`/`ShellType`/`Graph*Arg` enums, and the `args_for_*` completion
helpers — a few hundred lines.

### `nice-to-have`: `cli/src/output.rs` (686 lines) — likely candidate for a focused split

**Problem.** Without per-section data this file is the next-largest CLI node
and likely concentrates rendering of the default `bt` output (capability
table, font table, OSC table, multiplex/SSH banners). Not as severe as
`commands.rs` but it skews the CLI **equality** distribution.

**Files touched.** `biscuit-terminal/cli/src/output.rs`.

**Fix.** During the `commands/` split, audit `output.rs` for sections that map
1:1 to a subcommand and move them. Keep generic emitters (e.g., the
`println!` banner and shared formatters) in `output.rs`; push command-specific
rendering into the corresponding `commands/<name>.rs`.

## biscuit-terminal

### `critical`: Cyclic dependency between `utils::layout` and `utils::block_constraint`

**Problem.** The two foundational text-layout utilities form a hard cycle,
which is the single largest hit to the **acyclicity** score and contaminates
every upstream component that pulls either symbol.

- `lib/src/utils/layout.rs:1-7` — imports
  `block_constraint::{split_lines, visible_width, wrap_lines}`.
- `lib/src/utils/block_constraint.rs:3` — imports `layout::WordWrap`.

Because `WordWrap` is a *data* enum (no rendering logic) and the
`split_lines/visible_width/wrap_lines` helpers are *string-shape* operations,
they should not depend on each other. The cycle is artificial.

**Files touched.**

- `biscuit-terminal/lib/src/utils/layout.rs`
- `biscuit-terminal/lib/src/utils/block_constraint.rs`
- `biscuit-terminal/lib/src/utils/word_wrap.rs` (transitively pulls both)

**Fix.** Move `WordWrap` (and any sibling data enums it carries) into a leaf
module — e.g. `utils/wrap_policy.rs` — that has no dependencies on other
utils modules. Both `layout` and `block_constraint` import from `wrap_policy`
and the cycle dissolves.

```rust
// utils/wrap_policy.rs  (new — leaf module, no crate-internal deps)
#[derive(Debug, Clone)]
pub enum WordWrap {
    None,
    Wrap(Option<u16>),
    WrapProse(Option<u16>, Option<u16>),
    Truncate,
}

// utils/layout.rs
use crate::utils::wrap_policy::WordWrap;
use crate::utils::block_constraint::{split_lines, visible_width, wrap_lines};

// utils/block_constraint.rs
use crate::utils::wrap_policy::WordWrap;          // was layout::WordWrap
```

### `critical`: God file `components/filesystem.rs` (6,346 lines, ≈282 item-bearing lines)

**Problem.** A single file holds the `FileSystem` component, `TreeNode`,
`MetricKind`, `FileMetrics`, `FileSystemError`, the `icons` submodule (Nerd
Font + Unicode tables), the `tree_chars` submodule, **four** `impl FileSystem`
blocks (`:681`, `:1093`, `:1106`, `:2122`) plus the `Renderable` impl and a
2,500-line inline test module. This concentrates ~13 % of the entire
library’s LOC in one file and is the dominant **modularity** and **equality**
outlier.

**Files touched.** `biscuit-terminal/lib/src/components/filesystem.rs`.

**Fix.** Convert `filesystem.rs` into `components/filesystem/` with a
single-responsibility split. A reasonable target layout:

```
components/filesystem/
├── mod.rs              // pub use + module wiring
├── error.rs            // FileSystemError
├── icons.rs            // existing `icons` submodule, lifted out
├── tree_chars.rs       // existing `tree_chars` submodule, lifted out
├── tree_node.rs        // TreeNode + its impls
├── metrics.rs          // MetricKind + FileMetrics
├── builder.rs          // ensure_tree_built / scan logic
├── render.rs           // Renderable impl + helpers
└── tests/              // move inline tests to a tests/ subdir
```

Keep `pub use` re-exports in `mod.rs` so external API doesn’t change.

### `critical`: God file `components/table/table.rs` (5,156 lines, 3 in-file test modules)

**Problem.** All table logic — `TableCellContent`, `Conditional`,
`TableColumn`, `Table`, `MeasuredColumn`, `TableWidthMeasurements`,
`TableWidthPlan`, `TableWidthError`, two `impl Table` blocks, the
`Renderable` impl, and **three** separate `#[cfg(test)] mod tests {}` blocks
at lines 2283, 2518, and 2633 — lives in one file. Multiple test modules in
the same file is a **redundancy** signal (Sentrux flags duplicate symbol
declarations) and the 2,283-line implementation body itself is a
**modularity** failure. Module inception (`components::table::table::Table`)
makes the public path noisier than necessary.

**Files touched.**

- `biscuit-terminal/lib/src/components/table/table.rs`
- `biscuit-terminal/lib/src/components/table/mod.rs`

**Fix.** Flatten the inception and split by responsibility:

```
components/table/
├── mod.rs              // declares submodules + re-exports Table, TableColumn …
├── cell.rs             // TableCellContent + From impls + Display
├── column.rs           // TableColumn + Conditional
├── width.rs            // MeasuredColumn / TableWidthMeasurements / TableWidthPlan / TableWidthError
├── table.rs            // Table struct + impls
├── render.rs           // Renderable impl
├── types.rs            // (existing) ColumnType, Currency, VerticalAlign
└── tests.rs            // consolidate the three test modules into one
```

Update `mod.rs`:

```rust
pub mod cell;
pub mod column;
pub mod width;
pub mod render;
pub mod table;
pub mod types;

pub use cell::TableCellContent;
pub use column::{Conditional, TableColumn};
pub use table::Table;
pub use width::{MeasuredColumn, TableWidthMeasurements, TableWidthPlan, TableWidthError};
```

This collapses `components::table::table::Table` to `components::table::Table`
without an external API break (callers using the existing `prelude` are
unaffected).

### `urgent`: Orphan test files in `components/` are silently dead

**Problem.** `components/horizontal_rule_test.rs` (240 lines) and
`components/horizontal_rule_snapshot.rs` (111 lines) exist on disk but are
**not declared** as modules anywhere. No `mod horizontal_rule_test;` or
`#[path = …] mod` exists in `components/mod.rs` or in `horizontal_rule.rs`.
The contained `#[cfg(test)] mod tests { … }` blocks are never compiled or
executed. Sentrux flags this as **redundancy** (unused source nodes) and the
loss of test coverage is a quality risk on its own.

**Files touched.**

- `biscuit-terminal/lib/src/components/horizontal_rule_test.rs`
- `biscuit-terminal/lib/src/components/horizontal_rule_snapshot.rs`

**Fix.** Either wire them in or delete them.

If they are still needed:

```rust
// components/horizontal_rule.rs  (bottom of file)
#[cfg(test)]
#[path = "horizontal_rule_test.rs"]
mod horizontal_rule_test;

#[cfg(test)]
#[path = "horizontal_rule_snapshot.rs"]
mod horizontal_rule_snapshot;
```

If they were superseded by the inline `#[cfg(test)] mod tests` at
`horizontal_rule.rs:1064`, delete both files — they are pure dead code today.

### `urgent`: `components/horizontal_rule.rs` mixes terminal and browser rendering (3,152 lines)

**Problem.** This file owns both `impl Renderable for HorizontalRule`
(lines 193–865) **and** `impl BrowserRenderable for HorizontalRule`
(lines 866–1031) plus a 537-line tier-1 SVG→PNG pipeline that uses `resvg`,
`Cursor`, `TerminalImage`, and `Tailwind`. The SVG/HTML rendering and the
ANSI/Unicode tier rendering have very different dependency footprints.

**Files touched.** `biscuit-terminal/lib/src/components/horizontal_rule.rs`.

**Fix.** Split along the rendering axis:

```
components/horizontal_rule/
├── mod.rs              // pub use; struct + builder methods only
├── style.rs            // RuleStyle, RuleAlignment, RuleWeight enums
├── tiers.rs            // tier1 (SVG→PNG), tier2 (Unicode), tier3 (ASCII) helpers
├── terminal.rs         // impl Renderable
├── browser.rs          // impl BrowserRenderable + MarginToCss
└── tests.rs
```

Browser-only consumers no longer pay to compile the `resvg`/`TerminalImage`
chain when they import `HorizontalRule` for SVG output.

### `urgent`: `discovery/detection.rs` is a kitchen-sink module (1,571 lines)

**Problem.** Every static-detection function lives in one file:
`color_depth`, `color_mode`, `is_tty`, `get_terminal_app`, `terminal_width`,
`terminal_height`, `dimensions`, `image_support`,
`image_support_with_reason`, `osc8_link_support`, `multiplex_support`,
`underline_support`, `italics_support`, `dim_support`, `detect_connection`,
plus enums `ImageSupport`, `DetectionMethod`, `TerminalApp`, `Connection`,
`ColorDepth`, `ColorMode`, `UnderlineSupport`, `MultiplexSupport`,
`SshClient`, `MoshClient`. Hurts **equality** (one node owns most of the
discovery edges) and **modularity** (no cluster boundary inside discovery).

**Files touched.** `biscuit-terminal/lib/src/discovery/detection.rs`.

**Fix.** Split into capability-aligned submodules so import sites only pay
for what they use:

```
discovery/detection/
├── mod.rs              // re-exports for backward compatibility
├── app.rs              // TerminalApp, get_terminal_app
├── color.rs            // ColorDepth, ColorMode, color_depth, color_mode
├── dimensions.rs       // terminal_width/height/dimensions, is_tty
├── image.rs            // ImageSupport, ImageSupportResult, image_support*
├── osc8.rs             // osc8_link_support
├── styling_caps.rs     // italics_support, dim_support, underline_support, UnderlineSupport
├── multiplex.rs        // MultiplexSupport, multiplex_support
└── connection.rs       // Connection, SshClient, MoshClient, detect_connection
```

### `urgent`: `discovery/fonts.rs` per-terminal parser duplication (2,110 lines)

**Problem.** Per-terminal name/size parsers are copy-pasted in shape:
`parse_wezterm_font_name`/`_size`, `parse_ghostty_font_name`/`_size`,
`parse_iterm2_font_setting`, `parse_kitty_font_name`/`_size`,
`parse_alacritty_font_name`/`_size`, plus `query_ghostty_config` and
`query_iterm2_font_name`/`_size`. Sentrux’s **redundancy** dimension
(Kolmogorov-style sub-tree similarity) penalises this duplication.

**Files touched.** `biscuit-terminal/lib/src/discovery/fonts.rs`.

**Fix.** Introduce a `TerminalFontParser` trait + a registry keyed by
`TerminalApp`:

```rust
trait TerminalFontParser {
    fn parse_name(&self, content: &str) -> Option<String>;
    fn parse_size(&self, content: &str) -> Option<u32>;
    fn config_path(&self) -> Option<PathBuf>;
}

struct WezTermParser;
struct GhosttyParser;
struct ITermParser;
struct KittyParser;
struct AlacrittyParser;
// each in its own discovery/fonts/<name>.rs file

fn parser_for(app: TerminalApp) -> Option<&'static dyn TerminalFontParser> { /* … */ }
```

`font_name()`/`font_size()` then become a 6-line dispatch instead of a
chain of `if app == … else if app == …`.

### `important`: `components/prose.rs` (2,505 lines) conflates token grammar, styling, and rendering

**Problem.** `prose.rs` carries the `Prose` builder, the token grammar
(`{{bold}}`, `<red>…</red>`, OSC8 link parser), the styling map, and the
`Renderable` impl, plus 800 lines of inline tests. That mix concentrates
change risk on one node and inflates incoming edges from every consumer that
needs just one of the three concerns. Hurts **modularity** and **equality**.

**Files touched.** `biscuit-terminal/lib/src/components/prose.rs`.

**Fix.** Split into `components/prose/` with `tokens.rs` (tag parser),
`styles.rs` (color/weight tables and resolution), `prose.rs` (the `Prose`
struct + builder), and `render.rs` (the `Renderable` impl). Re-export from
`mod.rs`.

### `important`: `components/terminal_image.rs` (2,594 lines) bundles three image protocols

**Problem.** Kitty graphics protocol encoding, iTerm2 inline-image protocol,
width parsing (`ImageWidth`, `parse_filepath_and_width`),
cursor-save/restore/scroll math, and protocol-fallback selection all share
one file. Each protocol could be its own peer node with a thin
`ImageProtocol` trait above them.

**Files touched.** `biscuit-terminal/lib/src/components/terminal_image.rs`.

**Fix.**

```
components/terminal_image/
├── mod.rs              // TerminalImage struct + public API
├── width.rs            // ImageWidth, parse_filepath_and_width
├── cursor.rs           // save/restore/scroll-compensation logic (the doc’d quirk in MEMORY)
├── protocol.rs         // trait ImageProtocol
├── kitty.rs            // KittyProtocol impl
└── iterm.rs            // ITermProtocol impl
```

### `important`: `utils/color.rs` (2,254 lines) holds 5 color types and 4 wrappers

**Problem.** `Octet`, `BasicColor`, `RgbColor`, `HdrColor`, `WebColor`,
`Tailwind`, `Color`, plus four `…Wrapper` types implementing
`RenderableWrapper` all live in one file. Each color family is independently
useful and should not require recompiling Tailwind/HDR consumers when basic
ANSI changes.

**Files touched.** `biscuit-terminal/lib/src/utils/color.rs`.

**Fix.** Split into `utils/color/` with `octet.rs`, `basic.rs`, `rgb.rs`,
`hdr.rs`, `web.rs`, `tailwind.rs`, `color_enum.rs` (the `Color` enum), and
`wrappers.rs`. Keep a `mod.rs` that re-exports the same public surface so
the `prelude` is unaffected.

### `important`: `discovery/os_detection.rs` (1,276 lines) and `discovery/osc_queries.rs` (1,339 lines)

**Problem.** Each of these is a single file accumulating per-OS / per-query
branches with the typical platform-specific `#[cfg(target_os = …)]` cascade.
They are not as severe as `detection.rs` but are at the edge of where
**equality** starts penalising the discovery cluster.

**Files touched.**

- `biscuit-terminal/lib/src/discovery/os_detection.rs`
- `biscuit-terminal/lib/src/discovery/osc_queries.rs`

**Fix.** Adopt a per-platform-file pattern (`os_detection/macos.rs`,
`os_detection/linux.rs`, `os_detection/windows.rs`) with `cfg`-gated `mod`
declarations in `mod.rs`. For `osc_queries`, split per-query
(`bg_color.rs`, `fg_color.rs`, `cursor_color.rs`, `clipboard.rs`,
`cell_size.rs`).

### `important`: Module inception `components::table::table::Table`

**Problem.** Public path is `biscuit_terminal::components::table::table::Table`
because `components/table/mod.rs` only declares `pub mod table;` and
`pub mod types;`. The `#[allow(clippy::module_inception)]` attribute is a
warning-suppression that points at the underlying naming smell. Hurts
**modularity** (clusters are mis-nested) and **depth** (an extra path segment
on every import).

**Files touched.**

- `biscuit-terminal/lib/src/components/table/mod.rs`
- `biscuit-terminal/lib/src/components/table/table.rs`

**Fix.** When you split `table.rs` (see the `critical` suggestion above),
hoist `Table` and friends to `components/table/mod.rs` (or rename `table.rs`
→ `core.rs`) and re-export. The path becomes `components::table::Table`.

### `nice-to-have`: `lib.rs` doc-comment omits the `errors` module

**Problem.** `lib/src/lib.rs` ships a top-level `## Modules` list that names
`terminal`, `discovery`, `components`, and `utils` but not `errors`. The
`errors` module is `pub` and on the public surface, so external readers
can’t locate it from rustdoc.

**Files touched.** `biscuit-terminal/lib/src/lib.rs`.

**Fix.** Add `` - [`errors`] - Public error types (BlockError, prelude) `` to
the module list.

### `nice-to-have`: Consolidate the multiple `#[cfg(test)] mod tests` blocks per file

**Problem.** Several files have more than one inline `mod tests` block (most
notably `table/table.rs` with three at lines 2283, 2518, 2633). Sentrux
treats duplicate same-named child modules as **redundancy**, and it makes
`cargo test path::tests::…` ambiguous to the eye.

**Files touched.**

- `biscuit-terminal/lib/src/components/table/table.rs`
- (any other file with multiple in-file test mods after the splits land)

**Fix.** Either rename them (`mod cell_tests`, `mod column_tests`,
`mod render_tests`) or, once the file is split (the `critical` suggestion),
co-locate one `mod tests` per submodule.
