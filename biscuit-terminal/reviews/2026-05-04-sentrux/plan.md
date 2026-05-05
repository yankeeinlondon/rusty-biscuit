---
title: Sentrux Quality Remediation Plan — biscuit-terminal package area
agent: open_code
phases: 5
start_phase: 3
created: '2026-05-04T17:10:05'
source_review: review-1.md
suggestions_total: 18
suggestions_critical: 4
suggestions_urgent: 6
source_files_during_phase_0:
  - biscuit-terminal/lib/src/lib.rs
docs_updated_during_phase_0: []
docs_created_during_phase_0: []
skills_files_updated_during_phase_0: []
source_files_during_phase_1:
  - biscuit-terminal/lib/src/utils/wrap_policy.rs
  - biscuit-terminal/lib/src/utils/layout.rs
  - biscuit-terminal/lib/src/utils/block_constraint.rs
  - biscuit-terminal/lib/src/utils/word_wrap.rs
  - biscuit-terminal/lib/src/utils/mod.rs
  - biscuit-terminal/lib/src/prelude.rs
  - biscuit-terminal/lib/src/components/prose.rs
  - biscuit-terminal/lib/src/components/table/types.rs
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/components/status_block.rs
  - biscuit-terminal/lib/src/components/status.rs
  - biscuit-terminal/lib/src/components/renderable.rs
  - biscuit-terminal/lib/src/components/list.rs
  - biscuit-terminal/lib/src/components/compose.rs
  - biscuit-terminal/lib/src/components/block_quote.rs
  - biscuit-terminal/lib/src/components/inline_content.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .opencode/skill/biscuit-terminal/SKILL.md
source_files_during_phase_2:
  - biscuit-terminal/cli/src/commands/mod.rs
  - biscuit-terminal/cli/src/commands/shared.rs
  - biscuit-terminal/cli/src/commands/color_parse.rs
  - biscuit-terminal/cli/src/commands/image.rs
  - biscuit-terminal/cli/src/commands/mermaid.rs
  - biscuit-terminal/cli/src/commands/flowchart.rs
  - biscuit-terminal/cli/src/commands/quadrant.rs
  - biscuit-terminal/cli/src/commands/pie.rs
  - biscuit-terminal/cli/src/commands/git_graph.rs
  - biscuit-terminal/cli/src/commands/timeline.rs
  - biscuit-terminal/cli/src/commands/state_diagram.rs
  - biscuit-terminal/cli/src/commands/erd.rs
  - biscuit-terminal/cli/src/commands/graph.rs
  - biscuit-terminal/cli/src/commands/prose.rs
  - biscuit-terminal/cli/src/commands/quote.rs
  - biscuit-terminal/cli/src/commands/list.rs
  - biscuit-terminal/cli/src/commands/pad.rs
  - biscuit-terminal/cli/src/commands/columns.rs
  - biscuit-terminal/cli/src/commands/dir.rs
  - biscuit-terminal/cli/src/commands/xy_chart.rs
  - biscuit-terminal/cli/src/main.rs
  - biscuit-terminal/cli/src/args.rs
  - biscuit-terminal/cli/src/types.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-terminal/lib/src/discovery/mod.rs
  - biscuit-terminal/lib/src/discovery/detection/mod.rs
  - biscuit-terminal/lib/src/discovery/detection/app.rs
  - biscuit-terminal/lib/src/discovery/detection/color.rs
  - biscuit-terminal/lib/src/discovery/detection/connection.rs
  - biscuit-terminal/lib/src/discovery/detection/dimensions.rs
  - biscuit-terminal/lib/src/discovery/detection/image.rs
  - biscuit-terminal/lib/src/discovery/detection/multiplex.rs
  - biscuit-terminal/lib/src/discovery/detection/osc8.rs
  - biscuit-terminal/lib/src/discovery/detection/styling_caps.rs
  - biscuit-terminal/lib/src/discovery/fonts/mod.rs
  - biscuit-terminal/lib/src/discovery/fonts/types.rs
  - biscuit-terminal/lib/src/discovery/fonts/nerd.rs
  - biscuit-terminal/lib/src/discovery/fonts/window_size.rs
  - biscuit-terminal/lib/src/discovery/fonts/parser.rs
  - biscuit-terminal/lib/src/discovery/fonts/wezterm.rs
  - biscuit-terminal/lib/src/discovery/fonts/ghostty.rs
  - biscuit-terminal/lib/src/discovery/fonts/kitty.rs
  - biscuit-terminal/lib/src/discovery/fonts/alacritty.rs
  - biscuit-terminal/lib/src/discovery/fonts/iterm2.rs
  - biscuit-terminal/lib/src/discovery/os_detection/mod.rs
  - biscuit-terminal/lib/src/discovery/os_detection/types.rs
  - biscuit-terminal/lib/src/discovery/os_detection/os_type.rs
  - biscuit-terminal/lib/src/discovery/os_detection/family.rs
  - biscuit-terminal/lib/src/discovery/os_detection/linux.rs
  - biscuit-terminal/lib/src/discovery/os_detection/ci.rs
  - biscuit-terminal/lib/src/discovery/osc_queries/mod.rs
  - biscuit-terminal/lib/src/discovery/osc_queries/types.rs
  - biscuit-terminal/lib/src/discovery/osc_queries/parse.rs
  - biscuit-terminal/lib/src/discovery/osc_queries/query.rs
  - biscuit-terminal/lib/src/discovery/osc_queries/support.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - biscuit-terminal/lib/src/components/table/mod.rs
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/components/table/types.rs
  - biscuit-terminal/lib/src/components/table/cell.rs
  - biscuit-terminal/lib/src/components/table/column.rs
  - biscuit-terminal/lib/src/components/table/width.rs
  - biscuit-terminal/lib/src/components/filesystem/mod.rs
  - biscuit-terminal/lib/src/components/filesystem/error.rs
  - biscuit-terminal/lib/src/components/filesystem/icons.rs
  - biscuit-terminal/lib/src/components/filesystem/tree_chars.rs
  - biscuit-terminal/lib/src/components/filesystem/tree_node.rs
  - biscuit-terminal/lib/src/components/filesystem/metrics.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/style.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/browser.rs
  - biscuit-terminal/lib/src/components/compose.rs
  - biscuit-terminal/lib/src/prelude.rs
  - biscuit-terminal/lib/examples/table_showcase.rs
docs_updated_during_phase_4:
  - biscuit-terminal/lib/src/components/table/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - biscuit-terminal/lib/src/components/prose.rs
  - biscuit-terminal/lib/src/components/terminal_image.rs
  - biscuit-terminal/lib/src/utils/color.rs
  - biscuit-terminal/lib/src/utils/mod.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
  - biscuit-terminal
---

# Sentrux Quality Remediation Plan — biscuit-terminal

This plan operationalizes the 18 suggestions in [`review-1.md`](./review-1.md).
The work is **structural refactoring** — file splits, cycle breaks, and namespace
re-shaping — with **no public-API change** intended. The plan sequences
foundational/blocking work first (cycle break → CLI splits → detection splits →
component giants → medium components) and targets zero behavioral regression at
every phase boundary.

## Scope and goals

**In scope:** the `biscuit-terminal/lib/` and `biscuit-terminal/cli/` crates only.
No other workspace member is touched.

**Goals (acceptance criteria for the entire plan):**

1. `cycle_count` drops from **1 → 0** (Phase 1).
2. `god_file_count` drops from **5 → 0** — every file ≥ 2,000 lines is split
   into a directory module with single-responsibility files (Phases 2–5).
3. Orphan dead test files are either wired in or deleted (Phase 0).
4. Module inception `components::table::table::Table` collapses to
   `components::table::Table` (Phase 4).
5. `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` for
   both `biscuit-terminal` and `biscuit-terminal-cli` pass at every phase
   boundary.
6. No behavioral change in `bt` CLI output, rendering output, or library
   public surface (verified by existing inline + level-1/level-2 test suites
   moving with their subject modules).

## Constraints

- Keep `pub use` re-exports in every `mod.rs` so external API doesn't change.
- Move inline test modules with their subject code during splits.
- Use `cargo metadata --no-deps --format-version 1` for package names, not
  directory assumptions.
- Root `just` covers `biscuit-terminal`; prefer `just test` / `just lint` at
  phase boundaries.

## Phase 0: Quick Wins — Orphan Tests and Documentation

**Goal:** Eliminate dead source nodes and fix a documentation gap with
near-zero risk.

| # | Severity | Finding | Action |
|---|----------|---------|--------|
| 1 | urgent | `horizontal_rule_test.rs` and `horizontal_rule_snapshot.rs` exist on disk but are never compiled | Determine if they were superseded by the inline tests at `horizontal_rule.rs:1064`. If so, delete both. If still needed, wire them in with `#[path]` mod declarations. |
| 2 | nice-to-have | `lib.rs` doc-comment omits `errors` module | Add ` - [`errors`] — Public error types` to the module list in the top-level doc comment. |
| 3 | nice-to-have | `table/table.rs` has three `#[cfg(test)] mod tests` blocks at lines 2283, 2518, 2633 | Rename to `mod cell_tests`, `mod column_tests`, `mod render_tests` (or consolidate once the file is split in Phase 4). |

**Deliverables:**
- [x] Orphan test files resolved (deleted — superseded by inline tests in `horizontal_rule.rs`)
- [x] `errors` module listed in `lib.rs` doc-comment
- [x] Duplicate test module names investigated — only one `mod tests` exists in `table.rs`; no disambiguation needed (deferred to Phase 4 split)

**Validation:**
```bash
cargo test -p biscuit-terminal --lib
cargo clippy -p biscuit-terminal --all-targets -- -D warnings
```

**Risk:** Minimal. No production code changes; test-only and documentation.

---

## Phase 1: Break the Cycle — `WordWrap` Leaf Extraction

**Goal:** Dissolve the cyclic dependency between `utils::layout` and
`utils::block_constraint` by extracting `WordWrap` to a leaf module with
zero crate-internal dependencies.

| # | Severity | Finding | Action |
|---|----------|---------|--------|
| 4 | critical | `layout` imports from `block_constraint` and vice versa — artificial cycle via `WordWrap` data enum | Create `utils/wrap_policy.rs` containing `WordWrap` (and any sibling data-only enums). Both `layout` and `block_constraint` import from `wrap_policy` instead. Update `utils/mod.rs` to declare the new module. |

**Detailed steps:**

1. Create `lib/src/utils/wrap_policy.rs` — move `WordWrap` enum there (data
   only, no imports from other utils modules).
2. Update `lib/src/utils/layout.rs` — change `use crate::utils::block_constraint::WordWrap` to `use crate::utils::wrap_policy::WordWrap` (or wherever it currently imports from).
3. Update `lib/src/utils/block_constraint.rs` — change `use crate::utils::layout::WordWrap` to `use crate::utils::wrap_policy::WordWrap`.
4. Update `lib/src/utils/mod.rs` — add `pub mod wrap_policy;`.
5. Update any other files importing `WordWrap` from `layout` or `block_constraint` to import from `wrap_policy` instead.
6. Verify `cargo check` succeeds with no warnings.

**Deliverables:**
- [ ] `utils/wrap_policy.rs` created with `WordWrap` enum
- [ ] All import sites updated
- [ ] `utils/mod.rs` updated
- [ ] Cycle confirmed broken (no mutual import between layout ↔ block_constraint)

**Validation:**
```bash
cargo test -p biscuit-terminal --lib
cargo clippy -p biscuit-terminal --all-targets -- -D warnings
```

**Risk:** Low. `WordWrap` is a data enum with no behavior; this is a pure
re-location. All downstream consumers get the same type via a different path.
The `prelude` re-exports remain unchanged.

---

## Phase 2: CLI Refactor — Split God Files

**Goal:** Break the CLI's three monolith files into per-command modules with
a `Run` trait dispatch, reducing `main.rs` from ~500-line match to a single
`cmd.run(&ctx)` call.

| # | Severity | Finding | Action |
|---|----------|---------|--------|
| 5 | critical | `commands.rs` (2,226 lines, 45 free functions) holds all subcommand rendering | Convert to `commands/` directory with one file per subcommand family. Shared helpers live in `commands/shared.rs`. |
| 6 | urgent | `main.rs` has ~500-line subcommand match block | Add a `Run` trait; each `Command` variant dispatches via `cmd.run(&ctx)`. |
| 7 | urgent | `args.rs` (1,448 lines) crams all 17 subcommand variants | Promote each variant to a per-command `#[derive(Args)]` struct, co-located in `commands/<name>.rs`. |
| 8 | nice-to-have | `output.rs` (686 lines) concentrates default `bt` output rendering | Audit for sections that map 1:1 to a subcommand; keep generic emitters in `output.rs`. |

**Target CLI structure:**

```
cli/src/
├── main.rs              # ~30 lines: parse args → cmd.run(&ctx)
├── args.rs              # Args struct, Command enum, shared enums (LayoutArgs, ShellType, Graph*Arg)
├── output.rs            # default bt output (capability table, font table, OSC table)
├── types.rs             # (unchanged)
└── commands/
    ├── mod.rs           # pub use re-exports + Run trait + CliContext
    ├── shared.rs        # is_dark_mode, detect_terminal_*, apply_*_layout, output_render_meta
    ├── image.rs
    ├── mermaid.rs       # build_mermaid_diagram, display_mermaid, handle_mermaid_error
    ├── flowchart.rs
    ├── quadrant.rs
    ├── pie.rs           # parse_pie_data, parse_single_pie_entry, build_pie_init_directive
    ├── git_graph.rs
    ├── xy_chart.rs      # parse_xy_data, render_xy_chart
    ├── timeline.rs
    ├── state_diagram.rs
    ├── erd.rs
    ├── graph.rs         # display_graph, render_graph_expression, handle_graph_error
    ├── prose.rs
    ├── pad.rs           # render_pad_left, render_pad_right
    ├── quote.rs
    ├── list.rs
    ├── columns.rs
    ├── dir.rs           # render_dir + strip_sgr_sequences
    └── color_parse.rs   # extract_color, parse_hex_color
```

**Ordering within phase:**

1. **Split `commands.rs` → `commands/`** — extract functions into per-command
   files, keeping the same function signatures. `mod.rs` re-exports everything.
   This is the largest mechanical change; do it first.
2. **Promote `args.rs` variants → per-command `XxxArgs` structs** — each
   `commands/<name>.rs` owns its args struct alongside its render function.
   `args.rs` shrinks to the top-level `Args`, `Command` enum, and shared types.
3. **Add `Run` trait** — define `Run::run(self, ctx: &CliContext) ->
   color_eyre::Result<()>`. Each per-command file implements `Run` for its
   args struct. `main.rs` becomes `match args.command { Some(cmd) =>
   cmd.run(&ctx), None => print_default(&ctx) }`.
4. **Audit `output.rs`** — move any command-specific rendering to its
   `commands/<name>.rs`.

**Deliverables:**
- [ ] `commands/` directory with 18 module files
- [ ] `Run` trait + `CliContext` struct in `commands/mod.rs`
- [ ] `main.rs` reduced to ~30 lines
- [ ] `args.rs` reduced to shared types only
- [ ] `output.rs` audited

**Validation:**
```bash
cargo test -p biscuit-terminal-cli
cargo run -p biscuit-terminal-cli -- bt --help
cargo run -p biscuit-terminal-cli -- prose "hello {{bold}}world{{reset}}"
cargo clippy -p biscuit-terminal-cli --all-targets -- -D warnings
```

**Risk:** Medium. Large number of files moved but no behavioral change.
Every subcommand keeps the same argument names and output. The `bt` CLI
integration tests validate this.

---

## Phase 3: Detection Layer — Capability-Aligned Submodules

**Goal:** Split the discovery layer's two kitchen-sink modules and eliminate
per-terminal parser duplication, improving modularity without changing any
detection results.

| # | Severity | Finding | Action |
|---|----------|---------|--------|
| 9 | urgent | `detection.rs` (1,571 lines) holds all static-detection functions | Split into `detection/` with capability-aligned submodules: `app.rs`, `color.rs`, `dimensions.rs`, `image.rs`, `osc8.rs`, `styling_caps.rs`, `multiplex.rs`, `connection.rs`. |
| 10 | urgent | `fonts.rs` (2,110 lines) has copy-pasted per-terminal parsers | Introduce `TerminalFontParser` trait + per-terminal implementations. `font_name()`/`font_size()` become a 6-line dispatch. |
| 11 | important | `os_detection.rs` (1,276 lines) and `osc_queries.rs` (1,339 lines) | Adopt per-platform / per-query file pattern. |

**Target structure:**

```
discovery/
├── mod.rs               # existing + re-exports for new submodules
├── detection/
│   ├── mod.rs           # re-exports for backward compatibility
│   ├── app.rs           # TerminalApp, get_terminal_app
│   ├── color.rs         # ColorDepth, ColorMode, color_depth, color_mode
│   ├── dimensions.rs    # terminal_width/height/dimensions, is_tty
│   ├── image.rs         # ImageSupport, ImageSupportResult, image_support*
│   ├── osc8.rs          # osc8_link_support
│   ├── styling_caps.rs  # italics_support, dim_support, underline_support
│   ├── multiplex.rs     # MultiplexSupport, multiplex_support
│   └── connection.rs    # Connection, SshClient, MoshClient, detect_connection
├── fonts/
│   ├── mod.rs           # TerminalFontParser trait + parser_for() dispatch
│   ├── wezterm.rs
│   ├── ghostty.rs
│   ├── iterm2.rs
│   ├── kitty.rs
│   └── alacritty.rs
├── os_detection/
│   ├── mod.rs
│   ├── macos.rs
│   ├── linux.rs
│   └── windows.rs
├── osc_queries/
│   ├── mod.rs
│   ├── bg_color.rs
│   ├── fg_color.rs
│   ├── cursor_color.rs
│   ├── clipboard.rs
│   └── cell_size.rs
├── clipboard.rs         # (unchanged)
├── config_paths.rs      # (unchanged)
├── cursor_position.rs   # (unchanged)
├── eval.rs              # (unchanged)
├── locale.rs            # (unchanged)
├── mode_2027.rs         # (unchanged)
└── raw_mode.rs          # (unchanged)
```

**Ordering within phase:**

1. **Split `detection.rs`** — largest change, do first. Each capability module
   is self-contained. `detection/mod.rs` re-exports everything so existing
   `use` paths still resolve.
2. **Refactor `fonts.rs`** — define `TerminalFontParser` trait, extract per-
   terminal implementations. Verify `bt` font detection still works.
3. **Split `os_detection.rs`** — per-platform `cfg`-gated files.
4. **Split `osc_queries.rs`** — per-query files.

**Deliverables:**
- [x] `detection/` directory with 8 capability submodules
- [x] `fonts/` directory with trait + 5 implementations
- [x] `os_detection/` directory with per-platform files
- [x] `osc_queries/` directory with per-query files
- [x] All `discovery/mod.rs` re-exports updated

**Validation:**
```bash
cargo test -p biscuit-terminal --lib
cargo run -p biscuit-terminal-cli -- bt
cargo clippy -p biscuit-terminal --all-targets -- -D warnings
```

**Risk:** Low-medium. Detection logic is purely refactored, not rewritten.
The `Terminal::new()` path and all `bt` inspection output remain identical.

---

## Phase 4: Component Giants — Filesystem, Table, HorizontalRule

**Goal:** Split the three largest component files and collapse the
`table::table::Table` inception, resolving 3 critical and 2 urgent findings.

| # | Severity | Finding | Action |
|---|----------|---------|--------|
| 12 | critical | `filesystem.rs` (6,346 lines) — dominant modularity/equality outlier | Convert to `components/filesystem/` with `error.rs`, `icons.rs`, `tree_chars.rs`, `tree_node.rs`, `metrics.rs`, `builder.rs`, `render.rs`, and `tests/`. |
| 13 | critical | `table/table.rs` (5,156 lines, 3 test modules) | Flatten inception: split into `cell.rs`, `column.rs`, `width.rs`, `table.rs`, `render.rs`, `types.rs`, `tests.rs`. Re-export from `table/mod.rs`. |
| 14 | important | Module inception `components::table::table::Table` | Resolved by the table split — hoist `Table` to `table/mod.rs` re-export. |
| 15 | urgent | `horizontal_rule.rs` (3,152 lines) mixes terminal and browser rendering | Split along rendering axis: `mod.rs` (struct + builder), `style.rs` (enums), `tiers.rs` (tier helpers), `terminal.rs` (`Renderable`), `browser.rs` (`BrowserRenderable`), `tests.rs`. |

**Target structures:**

```
components/filesystem/
├── mod.rs              # pub use + module wiring
├── error.rs            # FileSystemError
├── icons.rs            # Nerd Font + Unicode tables
├── tree_chars.rs       # tree-drawing characters
├── tree_node.rs        # TreeNode + impls
├── metrics.rs          # MetricKind + FileMetrics
├── builder.rs          # ensure_tree_built / scan logic
├── render.rs           # Renderable impl + helpers
└── tests/              # inline tests moved here

components/table/
├── mod.rs              # re-exports Table, TableColumn, etc.
├── cell.rs             # TableCellContent + From impls
├── column.rs           # TableColumn + Conditional
├── width.rs            # MeasuredColumn, TableWidthMeasurements, TableWidthPlan, TableWidthError
├── table.rs            # Table struct + impls
├── render.rs           # Renderable impl
├── types.rs            # ColumnType, Currency, VerticalAlign
└── tests.rs            # consolidated test module

components/horizontal_rule/
├── mod.rs              # pub use; struct + builder methods
├── style.rs            # RuleStyle, RuleAlignment, RuleWeight
├── tiers.rs            # tier1 SVG→PNG, tier2 Unicode, tier3 ASCII
├── terminal.rs         # impl Renderable
├── browser.rs          # impl BrowserRenderable + MarginToCss
└── tests.rs
```

**Ordering within phase:**

1. **Split `table/`** — flattens inception, resolves test-module duplication.
   Start here because `table/mod.rs` already exists and the change is
   additive.
2. **Split `filesystem.rs`** — largest file in the entire library. Move
   inline tests first, then extract types, then split impls.
3. **Split `horizontal_rule.rs`** — separate terminal/browser rendering axes.

**Deliverables:**
- [ ] `components/filesystem/` directory with 8+ module files
- [ ] `components/table/` restructured with flattened public path
- [ ] `components/horizontal_rule/` directory with rendering-axis split
- [ ] All `components/mod.rs` re-exports updated
- [ ] No `#[allow(clippy::module_inception)]` needed for table

**Validation:**
```bash
cargo test -p biscuit-terminal --lib
cargo run -p biscuit-terminal-cli -- dir biscuit-terminal/lib/src --depth 2
cargo run -p biscuit-terminal-cli -- flowchart "A --> B"
cargo clippy -p biscuit-terminal --all-targets -- -D warnings
```

**Risk:** Medium. Large files split but logic unchanged. The `Renderable`
trait implementations move but their behavior doesn't. Existing tests travel
with their modules.

---

## Phase 5: Medium Components — Prose, TerminalImage, Color

**Goal:** Address the remaining three `important`-severity suggestions by
splitting medium-sized component and utility files into focused submodules.

| # | Severity | Finding | Action |
|---|----------|---------|--------|
| 16 | important | `prose.rs` (2,505 lines) conflates token grammar, styling, and rendering | Split into `components/prose/` with `tokens.rs`, `styles.rs`, `prose.rs`, `render.rs`. |
| 17 | important | `terminal_image.rs` (2,594 lines) bundles three image protocols | Split into `components/terminal_image/` with `width.rs`, `cursor.rs`, `protocol.rs`, `kitty.rs`, `iterm.rs`. |
| 18 | important | `color.rs` (2,254 lines) holds 5 color types and 4 wrappers | Split into `utils/color/` with `octet.rs`, `basic.rs`, `rgb.rs`, `hdr.rs`, `web.rs`, `tailwind.rs`, `color_enum.rs`, `wrappers.rs`. |

**Target structures:**

```
components/prose/
├── mod.rs              # pub use re-exports
├── tokens.rs           # tag parser ({{bold}}, <red>…</red>, OSC8 links)
├── styles.rs           # color/weight tables and resolution
├── prose.rs            # Prose struct + builder
└── render.rs           # Renderable impl

components/terminal_image/
├── mod.rs              # TerminalImage struct + public API
├── width.rs            # ImageWidth, parse_filepath_and_width
├── cursor.rs           # save/restore/scroll-compensation logic
├── protocol.rs         # ImageProtocol trait
├── kitty.rs            # KittyProtocol impl
└── iterm.rs            # ITermProtocol impl

utils/color/
├── mod.rs              # re-exports
├── octet.rs
├── basic.rs
├── rgb.rs
├── hdr.rs
├── web.rs
├── tailwind.rs
├── color_enum.rs       # Color enum
└── wrappers.rs         # RenderableWrapper impls
```

**Ordering within phase:**

1. **Split `color.rs`** — foundational utility used everywhere; do it first so
   downstream imports stabilize early.
2. **Split `prose.rs`** — token grammar is self-contained and cleanly
   separable.
3. **Split `terminal_image.rs`** — protocol split is straightforward; the
   `ImageProtocol` trait provides a clean seam.

**Deliverables:**
- [ ] `components/prose/` directory with 4 module files
- [ ] `components/terminal_image/` directory with 6 module files
- [ ] `utils/color/` directory with 8 module files
- [ ] All `mod.rs` re-exports updated
- [ ] No file in `lib/src/` exceeds ~1,500 lines (except possibly test-heavy
      files with tracked exceptions)

**Validation:**
```bash
cargo test -p biscuit-terminal --lib
cargo test -p biscuit-terminal-cli
cargo run -p biscuit-terminal-cli -- prose "Hello {{bold}}world{{reset}}"
cargo run -p biscuit-terminal-cli -- image --debug fixtures/tiny.png
cargo clippy -p biscuit-terminal --all-targets -- -D warnings
just test
just lint
```

**Risk:** Low-medium. Each split follows the same pattern established in
Phases 3–4. Logic is moved, not rewritten.

---

## Cross-Phase Validation

After every phase boundary:

```bash
cargo test -p biscuit-terminal --lib
cargo test -p biscuit-terminal-cli
cargo clippy -p biscuit-terminal --all-targets -- -D warnings
cargo clippy -p biscuit-terminal-cli --all-targets -- -D warnings
```

After Phase 5 (final validation):

```bash
just test
just lint
```

## Summary

| Phase | Theme | Critical | Urgent | Important | Nice | Files Split |
|-------|-------|----------|--------|-----------|------|-------------|
| 0 | Quick Wins | 0 | 1 | 0 | 2 | 0 (cleanup only) |
| 1 | Cycle Break | 1 | 0 | 0 | 0 | 1 created |
| 2 | CLI Refactor | 1 | 2 | 0 | 1 | 3 → 18+ files |
| 3 | Detection Layer | 0 | 2 | 1 | 0 | 4 → 20+ files |
| 4 | Component Giants | 2 | 1 | 1 | 0 | 3 → 20+ files |
| 5 | Medium Components | 0 | 0 | 3 | 0 | 3 → 18+ files |
| **Total** | | **4** | **6** | **5** | **3** | |
