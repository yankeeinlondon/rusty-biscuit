---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/biscuit-terminal/features/2026-10-13-graphics-where-possible/spec.md"
plan: "biscuit-terminal/features/2026-10-13-graphics-where-possible/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - "biscuit-terminal/lib/Cargo.toml"
    - "biscuit-terminal/lib/src/components/mod.rs"
    - "biscuit-terminal/lib/src/prelude.rs"
    - "biscuit-terminal/lib/src/render_tree/mod.rs"
    - "biscuit-terminal/lib/src/render_tree/options.rs"
    - "biscuit-terminal/lib/src/render_tree/render.rs"
    - "biscuit-terminal/lib/tests/l1/main.rs"
    - "biscuit-terminal/lib/tests/l1/layout_matrix.rs"
    - "biscuit-terminal/lib/tests/l1/render_tree_code_context.rs"
    - "biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs"
    - "biscuit-terminal/lib/tests/layout_matrix_support/mod.rs"
    - "biscuit-terminal/cli/Cargo.toml"
    - "biscuit-terminal/cli/src/main.rs"
    - "biscuit-terminal/cli/src/commands/graph.rs"
    - "biscuit-terminal/cli/src/commands/mermaid.rs"
    - "biscuit-terminal/cli/src/commands/quadrant.rs"
    - "biscuit-visualized/src/Cargo.toml"
    - "biscuit-visualized/src/src/lib.rs"
    - "biscuit-visualized/src/src/components/mod.rs"
    - "biscuit-visualized/src/src/components/render_hook.rs"
    - "biscuit-visualized/src/src/components/mermaid.rs"
    - "biscuit-visualized/src/src/components/graph_expression.rs"
    - "biscuit-visualized/src/src/components/git_graph.rs"
    - "biscuit-visualized/src/src/components/git_graph/tests.rs"
    - "biscuit-visualized/src/src/graph/expression.rs"
    - "biscuit-visualized/src/src/tests/mod.rs"
    - "biscuit-visualized/src/tests/l1/main.rs"
    - "biscuit-visualized/src/tests/l1/mermaid_parity.rs"
    - "biscuit-visualized/src/tests/l1/graph_expression_parity.rs"
    - "biscuit-visualized/src/tests/l1/layout_matrix.rs"
    - "biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs"
    - "biscuit-visualized/src/tests/l1/snapshots/"
    - "darkmatter/lib/Cargo.toml"
    - "darkmatter/lib/src/markdown/render_tree/entrypoints.rs"
    - "darkmatter/lib/src/markdown/render_tree/code_renderer.rs"
    - "darkmatter/lib/src/mermaid/render_terminal.rs"
    - "darkmatter/lib/benches/render_pipeline_steps.rs"
    - "darkmatter/lib/tests/l1/render_tree_hr_snapshots.rs"
    - "darkmatter/lib/tests/level2/level2_render_tree_terminal/images.rs"
    - "darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs"
    - "darkmatter/lib/tests/level3-terminal/level3_image_painting.rs"
    - "sniff/cli/Cargo.toml"
    - "sniff/cli/src/output/filesystem/deps.rs"
    - "worktree/cli/Cargo.toml"
    - "worktree/cli/src/commands/git_graph.rs"
    - "worktree/cli/src/commands/git_graph/tests.rs"
    - "worktree/cli/src/commands/list_table.rs"
    - "biscuit-icon/cli/tests/cli.rs"
    - "biscuit-tui/lib/Cargo.toml"
    - "Cargo.lock"
docs_updated_during_phase_1:
    - "biscuit-terminal/README.md"
    - "biscuit-terminal/lib/README.md"
    - "biscuit-terminal/docs/components/index.md"
    - "biscuit-terminal/docs/components/terminal_image.md"
    - "biscuit-terminal/docs/dependencies.md"
    - "biscuit-visualized/README.md"
    - "biscuit-visualized/docs/dot-graph.md"
    - "biscuit-visualized/docs/mermaid-gitgraph.md"
    - "biscuit-visualized/docs/components/mermaid_diagram.md"
    - "biscuit-visualized/docs/components/graph_expression.md"
    - "biscuit-visualized/docs/components/git_graph.md"
    - "biscuit-visualized/docs/data-visualization/visualizing-graph-expressions.md"
    - "biscuit-icon/docs/dependencies.md"
    - "biscuit-icon/lib/README.md"
    - "darkmatter/docs/dependencies.md"
    - "darkmatter/docs/rendering/mermaid.md"
    - "darkmatter/lib/README.md"
    - "darkmatter/lib/src/mermaid/README.md"
    - "docs/dependencies.md"
    - "sniff/docs/dependencies.md"
    - "worktree/docs/git-graph.md"
    - "worktree/docs/performance-testing.md"
docs_created_during_phase_1:
    - "biscuit-visualized/docs/dependencies.md"
skills_files_updated_during_phase_1:
    - ".claude/skills/biscuit-terminal/SKILL.md"
    - ".claude/skills/biscuit-terminal/cli.md"
    - ".claude/skills/biscuit-terminal/components.md"
    - ".claude/skills/biscuit-terminal/diagrams.md"
    - ".claude/skills/biscuit-terminal/mermaid-diagrams.md"
    - ".claude/skills/biscuit-visualized/SKILL.md"
    - ".claude/skills/biscuit-visualized/diagram-components.md"
    - ".claude/skills/darkmatter/terminal.md"
    - ".claude/skills/worktree/git-graph.md"
packages:
    - "biscuit-terminal"
    - "biscuit-terminal-cli"
    - "biscuit-visualized"
    - "darkmatter"
    - "sniff-cli"
    - "worktree-cli"
    - "biscuit-icon-cli"
    - "biscuit-tui"
    - "biscuit-icon"
human_review: false
message_to_agent: |-
    Phase 3 (route model) is done; context for phase 4 (probe):

    - The route lives in `biscuit-terminal/lib/src/discovery/detection/route.rs`.
      `resolve_graphics_route(&RouteInputs)` is pure; extend `RouteInputs` with
      the probe's results (Kitty query reply, structured DA1, wrapped/native
      split) rather than reading anything inside the resolver. Every
      "unknown" branch is the probe slot: `resolve_detected`'s final
      `Unknown` (incl. SSH without hints), `resolve_zellij`'s
      `zellij_kitty_enabled` (already wired: `Some(true)` -> Kitty/Verified,
      `Some(false)` -> NoSupport/Verified; nothing fills it yet), and
      `resolve_windows_terminal` (DA1 `4` is the second Sixel signal).
    - `Terminal::graphics_route` is authoritative (the old `image_support`
      field is gone; `Terminal::image_support()` projects it). Never re-resolve
      per image; TUI escape hatch (ruling 8) should let a caller supply a
      route via `Terminal::builder().graphics_route(..)` or turn probing off.
    - The L1 matrix is `biscuit-terminal/lib/tests/l1/graphics_route_matrix.rs`
      (79 tests, `route_rows!` table); add probe rows there.
    - Reason kinds: `Supported`, `NoSupport`, `Unknown`, `UserDisabled`,
      `BlockedTransport`, `InvalidInput`, `NotATerminal`. `fix` is set only
      when the blocker was observed; `bt` prints it as a hint
      (`biscuit-terminal/cli/src/hints.rs`). The hidden-tmux hint
      (`BISCUIT_GRAPHICS_TRANSPORT=tmux`) was deliberately not added yet.
    - tmux resolves to `Unknown` "planned" for every case (ruling 3), forced
      formats included; phase 5 replaces `tmux_planned`.
    - `TerminalImage::render_with_route` returns `ImageOutput::{Pixels,
      Fallback}`; a Sixel route falls back in `protocol.rs::encoder_for`
      until phase 7. `render()` now returns alt text on fallback (not "").
      Test pixel dispatch via `encoder_for` (rendering pixels queries the
      live cursor).
    - Windows Terminal version: `versions.rs` (Toolhelp32 walk + file
      version, `windows-sys` 0.61, cfg(windows)); `zellij --version` runs
      under a 500 ms deadline via `run_with_deadline`. Both are cached per
      process.
    - `just cross-check … -E '(…)'` breaks on parentheses; use positional
      nextest filters.
source_files_during_phase_2:
    - "Cargo.lock"
    - "biscuit-terminal/lib/src/components/image_options.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/mod.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/encoded_png.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/encoded_png/tests.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/iterm.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/kitty.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/protocol.rs"
    - "biscuit-visualized/src/Cargo.toml"
    - "biscuit-visualized/src/src/artifact.rs"
    - "biscuit-visualized/src/src/cache/file_cache.rs"
    - "biscuit-visualized/src/src/raster/png.rs"
    - "biscuit-visualized/src/src/graph/render.rs"
    - "biscuit-visualized/src/src/mermaid/render.rs"
    - "biscuit-visualized/src/src/components/mod.rs"
    - "biscuit-visualized/src/src/components/display.rs"
    - "biscuit-visualized/src/src/components/mermaid.rs"
    - "biscuit-visualized/src/src/components/graph_expression.rs"
    - "biscuit-visualized/src/src/components/render_hook.rs"
    - "biscuit-visualized/src/src/tests/artifact_tests.rs"
    - "biscuit-visualized/src/src/tests/cache_tests.rs"
    - "biscuit-visualized/src/src/tests/graph_tests.rs"
    - "biscuit-visualized/src/src/tests/mermaid_tests.rs"
    - "biscuit-visualized/src/src/tests/raster_tests.rs"
    - "biscuit-visualized/src/tests/l1/main.rs"
    - "biscuit-visualized/src/tests/l1/display_size.rs"
docs_updated_during_phase_2:
    - "biscuit-terminal/docs/components/terminal_image.md"
    - "biscuit-visualized/README.md"
    - "biscuit-visualized/docs/components/graph_expression.md"
    - "biscuit-visualized/docs/components/mermaid_diagram.md"
    - "biscuit-visualized/docs/dependencies.md"
    - "biscuit-visualized/docs/dot-graph.md"
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - ".claude/skills/biscuit-terminal/diagrams.md"
    - ".claude/skills/biscuit-terminal/image-rendering.md"
    - ".claude/skills/biscuit-visualized/SKILL.md"
    - ".claude/skills/biscuit-visualized/artifacts-caching.md"
    - ".claude/skills/biscuit-visualized/diagram-components.md"
    - ".claude/skills/biscuit-visualized/graph-rendering.md"
    - ".claude/skills/biscuit-visualized/mermaid-rendering.md"
    - ".claude/skills/biscuit-visualized/rasterization.md"
    - ".claude/skills/os/windows.md"
source_files_during_phase_3:
    - "Cargo.lock"
    - "biscuit-terminal/lib/Cargo.toml"
    - "biscuit-terminal/lib/src/discovery/detection/route.rs"
    - "biscuit-terminal/lib/src/discovery/detection/route/tests.rs"
    - "biscuit-terminal/lib/src/discovery/detection/versions.rs"
    - "biscuit-terminal/lib/src/discovery/detection/image.rs"
    - "biscuit-terminal/lib/src/discovery/detection/mod.rs"
    - "biscuit-terminal/lib/src/terminal.rs"
    - "biscuit-terminal/lib/src/render_tree/options.rs"
    - "biscuit-terminal/lib/src/render_tree/render.rs"
    - "biscuit-terminal/lib/src/components/horizontal_rule/mod.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/mod.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/protocol.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/tests.rs"
    - "biscuit-terminal/lib/examples/discovery_probe.rs"
    - "biscuit-terminal/lib/examples/terminal_info.rs"
    - "biscuit-terminal/lib/tests/l1/main.rs"
    - "biscuit-terminal/lib/tests/l1/graphics_route_matrix.rs"
    - "biscuit-terminal/lib/tests/l1/parity_helpers.rs"
    - "biscuit-terminal/lib/tests/l1/integration.rs"
    - "biscuit-terminal/lib/tests/l1/horizontal_rule_parity.rs"
    - "biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs"
    - "biscuit-terminal/cli/src/hints.rs"
    - "biscuit-terminal/cli/src/main.rs"
    - "biscuit-terminal/cli/src/output.rs"
    - "biscuit-terminal/cli/src/commands/image.rs"
    - "biscuit-terminal/cli/src/commands/mermaid.rs"
    - "biscuit-terminal/cli/src/commands/graph.rs"
    - "biscuit-terminal/cli/tests/l1/main.rs"
    - "biscuit-terminal/cli/tests/l1/graphics_route.rs"
    - "biscuit-visualized/src/src/components/graph_expression.rs"
    - "biscuit-visualized/src/src/components/mermaid.rs"
    - "darkmatter/lib/src/markdown/output/terminal.rs"
    - "darkmatter/lib/tests/l1/render_tree_hr_snapshots.rs"
    - "darkmatter/lib/tests/level2/level2_render_tree_terminal/images.rs"
    - "darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs"
    - "darkmatter/lib/tests/level3-terminal/level3_image_painting.rs"
    - "biscuit-icon/lib/src/icon.rs"
    - "biscuit-icon/cli/src/commands.rs"
    - "worktree/cli/src/commands/list.rs"
docs_updated_during_phase_3:
    - "biscuit-terminal/README.md"
    - "biscuit-terminal/lib/README.md"
    - "biscuit-terminal/docs/components/terminal_image.md"
    - "biscuit-terminal/docs/discovery/terminal-queries.md"
    - "biscuit-terminal/docs/dependencies.md"
    - "docs/dependencies.md"
docs_created_during_phase_3:
    - "biscuit-terminal/docs/discovery/graphics-routes.md"
skills_files_updated_during_phase_3:
    - ".claude/skills/biscuit-terminal/cli.md"
    - ".claude/skills/biscuit-terminal/discovery.md"
    - ".claude/skills/biscuit-terminal/image-rendering.md"
    - ".claude/skills/biscuit-terminal/render-tree.md"
    - ".claude/skills/biscuit-terminal/terminal-struct.md"
    - ".claude/skills/os/SKILL.md"
    - ".claude/skills/os/windows-console.md"
---

# Implementation Log for 2026-10-13-graphics-where-possible (9 phases)

## Phase 1

### Wave 1: spikes, inventory, baseline

- **Spikes.** S1 (Zellij) not runnable: `zellij` is not installed on this
  host. S2 (tmux 3.8) recorded in the plan's `## Spike results`:
  `client_termfeatures` is a bare comma-separated list (`sixel` appears as the
  bare word), `client_termtype` is empty unless the outer terminal answers
  XTVERSION, and all three variables exist on tmux >= 3.2.
- **Inventory.** Written into the plan as a checklist under Wave 3. Beyond the
  plan's initial hits: `darkmatter/lib/src/markdown/render_tree/code_renderer.rs`
  (calls `MermaidDiagram::render_to_svg`), the `layout_matrix` insta
  snapshots for the two diagram rows, the `bt` `quadrant` command
  (`QuadrantTheme`, `MermaidConfig`), and `biscuit-terminal/lib/examples/layout_matrix.rs`.
  `worktree/lib` and `sniff/cli/src/args/repo.rs` name the types only in
  comments/help text.
- **Baseline (unchanged tree, 2026-10-21):**

  | Package area | Command | Result |
  | --- | --- | --- |
  | biscuit-terminal | `just test` | 3605 passed, 57 skipped |
  | biscuit-terminal | `just test-l2` | lib 2 passed / 3114 skipped; cli 251 passed / 402 skipped |
  | biscuit-visualized | `just test` | 116 passed |
  | sniff | `just test` | 3463 passed, 33 skipped |
  | worktree | `just test` | 1529 passed, 1 failed, 15 skipped |
  | darkmatter | `just test` | 10498 passed, 10 skipped |

  The worktree failure (`list_remote_head::a_check_still_running_at_the_deadline_is_still_checking_and_the_next_run_shows_it`,
  10.3 s under load) passes alone in 0.65 s: load-sensitive and pre-existing.
- `cargo tree -p biscuit-terminal -e normal --features image` (before):
  contains `biscuit-visualized`, `mermaid-rs-renderer`, `resvg`, `viuer`.
- 191 diagram-related tests listed before the move (95 unit tests in the three
  component modules, 96 in the `l1` binary: `mermaid_parity` 36,
  `graph_expression_parity` 36, `layout_matrix` 16, `render_tree_code_context` 7,
  `render_comparison` 1).

### Rulings (Necessary Rules 1–11)

All eleven proposed answers stand as written in the plan (`yolo: true`); none
was exercised by this phase's code, which changes no route behavior. Rules
1–3, 5–9, and 11 bind phases 3–6; rule 4 (phase 7 also depends on 5) is
already satisfied by the sequential order; rule 10 lists measurements left to
the author.

### Wave 2: render-tree diagram hook (`biscuit-terminal`)

- `render_tree::{DiagramRenderer, DiagramRenderRequest, DiagramRenderError}`
  and `TerminalRenderOptions::{diagram_renderer, with_diagram_renderer}`
  (`Rc<dyn DiagramRenderer>`, matching `code_renderer`).
- `DiagramRenderRequest` carries `source`, `attrs`, `terminal` (with
  `force_graphics` applied), and `available_width`; it is `#[non_exhaustive]`
  so phase 3 can add the route without breaking callers. **Departure from the
  plan:** no separate `layout` field — the node's layout is `attrs.layout`,
  so a second copy would only drift.
- `DiagramRenderError` wraps `Box<dyn Error>` and displays as the wrapped
  error, so the existing diagnostic strings ("Mermaid promotion failed: …",
  "Mermaid rasterization failed, falling back to code block: …") are
  byte-identical to before.
- `render_code_node` no longer has a `#[cfg(feature = "image")]` gate: the
  hook's presence decides. No hook → code rendering plus
  `tracing::debug!("Mermaid promotion requested without hook; …")`. There is
  no dedicated capability-trace facility in the crate; `tracing::debug!`
  matches the existing discovery traces.
- `biscuit-terminal/lib/Cargo.toml`: `biscuit-visualized` removed;
  `image = ["dep:resvg"]`. `viuer` is left for phase 3 housekeeping (spec §9).

### Wave 3: move and consumers

- Components moved with plain `mv` (nothing staged) to
  `biscuit-visualized/src/src/components/{mermaid,graph_expression,git_graph}.rs`
  and `git_graph/tests.rs`; `crate::`/`super::` terminal paths became
  `biscuit_terminal::`, `biscuit_visualized::` became `crate::`, and doctest
  paths became `biscuit_visualized::components::…`. A diff against `HEAD`
  shows only path/import lines changed.
- New `components/mod.rs` re-exports what biscuit-terminal's prelude used to,
  plus `TerminalDiagramRenderer` (`components/render_hook.rs`), which renders
  `MermaidDiagram::new(source).try_render(terminal)` directly (no render tree,
  so no re-promotion).
- `biscuit-visualized/src/Cargo.toml`: **edition 2021 → 2024** (the moved
  `git_graph.rs` uses a let-chain). Consequences fixed: `std::env::set_var`
  in the test cache helper is now `unsafe` (callers are `#[serial]`), and one
  `collapsible_if` in `graph/expression.rs` became a let-chain. New
  `terminal = ["image", "dep:biscuit-terminal", "biscuit-terminal/image",
  "dep:renderable"]`; `clap` gating unchanged (consumers that need
  `QuadrantTheme: ValueEnum`, i.e. `bt`, enable `clap` themselves).
  `autotests = false` and a new `l1` test binary (`required-features =
  ["terminal"]`).
- Tests moved: `mermaid_parity.rs` and `graph_expression_parity.rs` to
  `biscuit-visualized/src/tests/l1/` (bodies unchanged; they include
  biscuit-terminal's `parity_helpers.rs` through a cross-package `#[path]`);
  the 114 `l1__layout_matrix__{GraphExpression,MermaidDiagram}*` insta
  snapshots (byte-identical); the per-row layout-matrix checks, copied
  verbatim into `biscuit-visualized/src/tests/l1/layout_matrix.rs` over just
  the two diagram rows, using biscuit-terminal's `layout_matrix_support`
  (also `#[path]`). Both shared files are declared in
  `[package.metadata.ci.tests] source-inputs` and spelled by an
  `include_str!` marker test (the claudine-gen/messenger precedent).
  biscuit-terminal's support module gained `tree_only_case` (a function form
  of its `tree_only!` macro) for those rows and lost its diagram rows; its
  count test is now `component_case_count_is_sixteen`.
- `mermaid_rich_mode_with_opt_in_bypasses_code_renderer` moved to
  `biscuit-visualized` `render_tree_diagram_hook::` with the real hook
  installed (its only change: `diagram_renderer: Some(..)` in the literal).
  `mermaid_promotion_failure_rejects_under_strict` now exists twice: in
  biscuit-terminal against a failing fake hook, and in biscuit-visualized
  against the real hook on a no-image terminal (the original body).
- Consumers: `bt` (`biscuit-visualized` with `clap`, `terminal`; imports in
  `commands/{mermaid,graph,quadrant}.rs`, `main.rs`), darkmatter (dependency,
  imports in `mermaid/render_terminal.rs` and
  `markdown/render_tree/code_renderer.rs`, hook installed in
  `terminal_options_from_terminal_options`, `diagram_renderer: None` in five
  struct literals in tests/benches), sniff-cli (`output/filesystem/deps.rs`),
  worktree-cli (`commands/{git_graph,list_table}.rs`,
  `git_graph/tests.rs`; its `biscuit-visualized` dev-dependency became the
  normal `terminal` dependency). **Departure from the plan:** `bt` installs no
  hook — none of its render-tree paths (`block`, `quote`) sets
  `TerminalMermaidMode::Image`, so a hook there would never be consulted; its
  diagram commands call the components directly.
- Stale comments fixed (drift, code is correct): biscuit-icon `tests/cli.rs`
  and its docs/README (the `image` feature no longer pulls
  `biscuit-visualized`), biscuit-tui `Cargo.toml`, sniff-cli `deps.rs`,
  worktree-cli `git_graph.rs`, darkmatter `render_terminal.rs`.
- `cargo check --workspace --all-targets` is clean.

### Docs and skills

- Moved: `docs/components/{mermaid_diagram,graph_expression,git_graph}.md`
  and `docs/data-visualization/visualizing-graph-expressions.md` to
  `biscuit-visualized/docs/`; links fixed both ways. The biscuit-terminal
  lib README's Mermaid and GraphExpression reference sections moved into
  those component pages (the README keeps a short "Diagrams" pointer). While
  moving, three claims were found wrong against the code and corrected: a
  "10KB size limit" (none exists), `MermaidTheme::for_color_mode` /
  `QuadrantTheme::apply` taking a `ColorMode` (they take `bool`), and the
  README `clap` example naming `QuadrantTheme` (that derive is
  biscuit-visualized's own `clap` feature).
- `mermaid_diagram.md` documents the render-tree hook (with a flow diagram).
- New `biscuit-visualized/docs/dependencies.md`; updated root,
  biscuit-terminal (also adds the missing `resvg` entry), darkmatter, and
  sniff dependency docs.
- Skills: biscuit-terminal `mermaid-diagrams.md` (still described the removed
  `mmdc` pipeline) replaced by `diagrams.md` (boundary, hook contract, `bt`
  diagram commands); its component table and GitGraph section moved to the
  new biscuit-visualized `diagram-components.md`; both `SKILL.md`s, the
  worktree `git-graph.md`, and darkmatter `terminal.md` updated.

### Pre-existing issues found (not caused by this phase)

- `cargo check -p biscuit-terminal --all-features --tests` fails in
  `components/horizontal_rule/mod.rs` (`serde_roundtrip` test module uses
  `HorizontalRule` without importing it) under the `serde` feature. No
  recipe or CI policy enables `serde` for biscuit-terminal; left alone.

### Wave 4: validation

- **Dependency trees** (`cargo tree -e normal`): `biscuit-terminal` has no
  `biscuit-visualized` and no `mermaid-rs-renderer`, with or without `image`;
  `resvg` appears only under `image`. `biscuit-visualized` with no features has
  no `biscuit-terminal`; with `terminal` it has `biscuit-terminal` and no
  cycle.
- **Tests after the change (macOS):**

  | Package area | Command | Result | Against baseline |
  | --- | --- | --- | --- |
  | biscuit-terminal | `just test` | 3446 passed, 55 skipped | 3605 − 166 moved (run) + 7 new; 2 moved tests were skipped |
  | biscuit-visualized | `just test` | 294 passed, 3 skipped | 116 + 168 moved + 13 new (2 helper markers, 1 layout marker, 6 copied layout checks incl. 1 ignored, 4 real-hook) |
  | biscuit-terminal | `just test-l2` | lib 2 / cli 251 passed | identical |
  | sniff | `just test` | 3463 passed | identical |
  | worktree | `just test` | 1530 passed | the baseline's load-sensitive failure passed this time |
  | darkmatter | `just test` | 10498 passed | identical |
  | worktree | `just test-l2` | 40 passed, 1 failed | `level2_list_verbose::level2_a_failed_assertion_in_a_held_pr_scene_still_reaps_the_worker` failed at 11.0 s and passes alone in 5.9 s: load-sensitive, PR-worker reaping, unrelated to the graph |
  | darkmatter | `just test-l2` | 18 + 69 + 3 passed | — |

  Every moved test name is present in `biscuit-visualized` (checked against
  the 191-name list captured before the move).
- **Doctests:** `cargo test --doc` passes for `biscuit-visualized`
  (`--all-features`, 29 passed) and `biscuit-terminal` (`--features image`,
  199 passed).
- **Lint:** `just lint` clean in biscuit-terminal, biscuit-visualized,
  darkmatter, sniff, worktree. `just check-tier-coverage` reports nothing
  stranded for biscuit-terminal or biscuit-visualized.
- **Workspace:** `cargo check --workspace --all-targets` clean.
- **Windows (`build-win-native`):** `just cross-check biscuit-terminal --os
  windows` 3078 passed; `just cross-check biscuit-visualized --os windows
  --all-features` 296 passed (the plain form refuses: the package needs an
  explicit build flag).
- **CI narrowed test-input cells** from `scripts/ci/affected_scope.py
  --resolved-plan`, run locally: sniff (2 lockfile tests), test-toolkit
  (2 `ci_workflow_contracts` tests), tree-hugger-cli (1), claudine `l1`
  (323), biscuit-icon-cli `install_recipe_enables_image_feature` — all pass.
- **Gate-skip check:** `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` →
  `change_class: package`, `packages` non-empty (biscuit-icon-cli,
  biscuit-terminal, biscuit-terminal-cli, biscuit-visualized, darkmatter,
  sniff-cli, worktree-cli, test-toolkit, claudine, sniff, tree-hugger-cli), so
  the gates were run, not skipped.

### Requirement-to-test mapping

| Behavior | Test (tier L1, F2: public `render_terminal_node`) |
| --- | --- |
| No hook: a promoted Mermaid fence renders exactly as Mermaid-as-code, no diagnostic, even under Strict | `biscuit-terminal` `render_tree_diagram_hook::mermaid_fence_without_a_diagram_hook_renders_as_the_code_block` |
| No hook: the code hook still renders the fence | `…::mermaid_fence_without_a_diagram_hook_still_reaches_the_code_hook` |
| Hook consulted before the code hook; receives source, attrs, width, terminal | `…::diagram_hook_is_consulted_before_the_code_hook` |
| Hook not consulted for Code mode, graphics Off, other or no language | `…::diagram_hook_is_not_consulted_unless_promotion_is_requested` |
| Strict / Warn / Lossy handling of a hook error (exact messages) | `…::diagram_hook_failure_follows_the_render_strictness`, `render::render_tree_tests::mermaid_promotion_failure_rejects_under_strict` |
| `force_graphics` applied before the hook | `…::diagram_hook_receives_the_forced_terminal_snapshot` |
| Child option clones keep the hook | `…::nested_mermaid_fence_reaches_the_diagram_hook_at_its_narrowed_width` |
| Real hook output equals the component's own output (Kitty image) | `biscuit-visualized` `render_tree_diagram_hook::hook_draws_the_fence_exactly_as_the_component_does` |
| Real hook failure: Strict rejects; Warn shows code with the component's error; invalid source falls back | `…::mermaid_promotion_failure_rejects_under_strict`, `…::failed_promotion_warns_with_the_component_error_and_renders_code`, `…::invalid_mermaid_source_falls_back_to_code_under_warn` |
| Components behave unchanged after the move | the 95 unit tests, 72 parity tests, and layout-matrix checks against the 114 unchanged snapshots, all in `biscuit-visualized` |
| Darkmatter's terminal entry point installs the hook (a `MermaidMode::Image` fence becomes a Kitty image; `Text` stays code) | darkmatter `markdown::render_tree::entrypoints::tests::render_tree_terminal_draws_mermaid_through_the_installed_hook` (added after the gates; mutation-checked: with `diagram_renderer: None` it fails on the code-panel output; darkmatter `just lint` re-run clean) |
| Darkmatter otherwise unchanged | darkmatter `just test` / `just test-l2` (identical counts) |

The phase touches no parser or configuration reader, so the input-robustness
matrix does not apply.

### Concurrent activity in this worktree

While this phase ran, another process committed five unrelated planning
commits here (`b81730ccd`..`2f7276503`, darkmatter/claudine spec moves, 10:25–10:30)
and left this phase's 13 darkmatter files **staged** (an older snapshot of
them). None of this phase's changes were in those commits. Because this phase
must leave nothing staged, and a staged snapshot could be swept into an
unrelated commit, the 13 paths were unstaged with `git restore --staged`
(index only; the working tree, which held the newer content, was untouched).
Afterwards: `git diff --cached` is empty, and the darkmatter hook-wiring and
HR snapshot tests still pass.

## Phase 2

### What was built

- **Background enum** (`biscuit-visualized/src/src/artifact.rs`):
  `RenderRequest::transparent_background: bool` is replaced by
  `background: Background` (`Theme` default, so exports are unchanged;
  `Transparent`; `Solid { r, g, b }`), applied to SVG and PNG by both
  renderers (graph: background `<rect>` with the theme surface, the solid hex,
  or none; Mermaid: theme background replaced by the hex, or `none` plus edge
  labels `none`). New `RenderRequest::row_height: Option<u32>` pads a PNG's
  height up to whole rows, below the drawing, transparent or in the solid
  color. Builders `with_row_height` and `with_background`.
- **Cache identity**: `RenderRequest::cache_identity()` (scale, target width,
  row height, background, and the PNG encoding for PNG requests) is the one
  request block both renderers put in their keys. The key schema moved to
  `v2|`, so every earlier artifact misses.
- **PNG encoder settings**: the rasterizer encodes through the `png` crate
  (0.17, tiny-skia's own version, now a direct optional dependency under
  `image`) at `Compression::Default` with adaptive filtering, replacing
  tiny-skia's fast/Sub `encode_png`. `TerminalImage::encode_as_png` uses
  `PngEncoder::new_with_quality(CompressionType::Default, FilterType::Adaptive)`.
  Measurement (once, macOS, image 0.25.10): the 400x400 iTerm2 app icon
  (`sips -z 400 400` of `iTerm2 App Icon for Release.icns`) encodes to
  **53,114 bytes before** (`write_to`) and **43,044 bytes after** (−19%); raw
  RGBA is 640,000 bytes.
- **Encoded-PNG input path** (`biscuit-terminal`): new public `EncodedPng`
  (`terminal_image/encoded_png.rs`) validates before anything is reserved
  (≤ 10 MiB default file-size limit, readable PNG header, dimensions within
  `image`'s default 512 MiB decode allocation) and carries the header's pixel
  size. `TerminalImage::from_encoded_png` stores it in a private field; every
  protocol path (standalone Kitty, WezTerm cells, iTerm2, inline, and
  `render_as_kitty`/`render_as_iterm2`) now takes pixels from one
  `TerminalImage::pixels()` source, so encoded bytes pass through unchanged
  and are decoded only by `load_image()` or an actual resize
  (`render_as_iterm2` at a different pixel width). Path-based constructors and
  their validation are unchanged; the encoded input has no path, so it cannot
  bypass them.
- **Display-size rendering** (`biscuit-visualized/src/src/components/display.rs`):
  `MermaidDiagram` and `GraphExpression` rasterize at columns × cell width
  (measured, else the 8x16 fallback, never zero), padded to whole cell rows,
  and hand the artifact's bytes to `TerminalImage::from_encoded_png` pinned at
  `ImageWidth::Characters(columns)`. Size is measured on every render, so a
  resize gets a new raster. The render-tree hook now sizes the diagram to
  `DiagramRenderRequest::available_width`.
- **Atomic cache store** (found while testing; see below).

### Departures and decisions

- **Padding fill for `Theme`** is transparent, not the theme color: resolving
  an arbitrary theme/`%%{init}%%` CSS color at raster time needed a color
  parser, and terminal adapters use `Transparent` (Kitty/iTerm2) or `Solid`
  (Sixel, phase 7). An inverted (opaque theme) diagram's last partial row
  therefore ends in a transparent strip under one row tall, the same visual
  result as before this phase (Kitty also left it unpainted).
- **Pass-through whenever no resize happens**, not only "when sizes match":
  Kitty/iTerm2 always scaled the PNG themselves and the old path re-encoded at
  the original resolution, so re-encoding never changed geometry. A resize
  (decode + Lanczos + encode) happens only where it did before
  (`render_as_iterm2` with a different pixel width).
- **Behavior fix (GraphExpression and Percent widths):** the components used
  to hand `TerminalImage` the original `ImageWidth`, which it re-resolved
  against the full terminal width without the component's margins (90% of 80
  = 72 columns drawn into a 60-column slot). Pinning the resolved columns
  fixes it; covered by `a_graph_with_margins_is_displayed_at_its_resolved_columns`.
  The 114 layout-matrix snapshots did not change.
- **Housekeeping left for phase 3** as the plan schedules it: the
  `protocol.rs` iTerm2 comment and the `viuer` removal.

### Found while testing: non-atomic artifact store

`FileCache::store` used `fs::write`, which truncates then writes: a concurrent
render of the same key (two `bt` processes, or two nextest processes) could
take the cache hit and read a half-written PNG. It surfaced on macOS as an
intermittent `try_render` failure in the new display tests and passed alone.
Fixed by staging under a unique name in the cache directory and renaming into
place. The first version used `tempfile::NamedTempFile::persist`; on Windows,
with fresh cache keys (a unique `%%` comment in the test diagram to defeat the
host's persisted cache), both concurrent Mermaid tests failed 3/3 with
os error 2: `TempPath::keep()` after a rename touches the vanished temp path.
The final version uses plain `std::fs::write` + `std::fs::rename` and treats a
refused replace as success when the key's file already exists (identical
content-addressed bytes). The same fresh-key Windows check then passed 3/3.
Recorded in the `os` skill (`windows.md`).

### Requirement-to-test mapping

| Behavior | Test (tier, rung) |
| --- | --- |
| Encoded PNG bytes reach Kitty unchanged (Kitty and WezTerm paths, `render_as_kitty`); rows reserved = padded height / cell height; `c=`/`r=` | `biscuit-terminal` `terminal_image::encoded_png::tests::kitty_receives_the_bytes_unchanged_and_reserves_the_padded_rows` (L1, F2) |
| …and iTerm2 unchanged; generic `name=` | `…::iterm2_receives_the_bytes_unchanged` |
| Inline (`TwoColumn`) and standalone dispatch send the same bytes | `…::the_inline_path_sends_the_same_bytes_as_the_standalone_path` |
| Fixture is mutation-sensitive (re-encoding would change it) | `…::the_fixture_would_change_if_it_were_re_encoded`; mutation check: forcing a re-encode in `ImagePixels::png` failed the 3 pass-through tests |
| Actual resize decodes and encodes at the new width | `…::an_actual_resize_decodes_and_encodes_at_the_new_width` |
| `load_image` decodes encoded bytes (pixels for later formats) | `…::pixels_decode_from_the_encoded_bytes` |
| Validation: not PNG / empty / truncated → `ImageLoadError`; > 10 MiB → `FileTooLarge` (exact sizes); 30000² header → `Limits`, with a 4x4 control of the same hand-built header | `…::bytes_that_are_not_a_png_are_rejected`, `…::more_bytes_than_the_file_size_limit_are_rejected_before_reading_the_header`, `…::dimensions_beyond_the_allocation_limit_are_rejected_without_decoding`, `…::the_pixel_size_comes_from_the_header` |
| `encode_as_png` lossless and smaller than `image`'s default | `…::encoding_keeps_the_pixels_and_beats_the_image_crate_default` |
| Rasterizer encoding lossless and smaller than tiny-skia's | `biscuit-visualized` `tests::raster_tests::requests::the_encoding_is_lossless_and_smaller_than_tiny_skias` |
| Row padding: rounds up, exact multiple unpadded, absent = unpadded | `…::requests::the_height_is_padded_up_to_whole_rows` |
| Aspect ratio kept, padding below, transparent / solid fill | `…::requests::padding_sits_below_an_unstretched_drawing` |
| Background: `solid_hex`; default request is `Theme`, no row height | `tests::artifact_tests::only_a_solid_background_has_a_color`, `…::default_render_request` |
| Cache identity differs for every background, geometry, padding; PNG-only encoding field | `tests::artifact_tests::the_cache_identity_covers_background_geometry_and_encoding` |
| Graph: solid `<rect>` replaces theme surface; every background/geometry is its own artifact; same request is a cache hit (control) | `tests::graph_tests::graph_render_separates_backgrounds_and_geometry` |
| Mermaid: solid color in SVG; artifacts separated; real diagram padded to 16 px rows | `tests::mermaid_tests::mermaid_render_separates_backgrounds_and_geometry` |
| Store replaces atomically, leaves no staged file | `tests::cache_tests::storing_a_key_again_replaces_the_artifact_without_leftovers` |
| Mermaid component → Kitty byte-identical, 30 cols = 240 px, height multiple of 16, rows advanced | `biscuit-visualized` `l1 display_size::a_mermaid_png_reaches_kitty_byte_identical_at_its_display_size` (L1, F2: crosses crates) |
| Mermaid component → iTerm2 byte-identical | `…::a_mermaid_png_reaches_iterm2_byte_identical` |
| GraphExpression → Kitty byte-identical at display size | `…::a_graph_png_reaches_kitty_byte_identical_at_its_display_size` |
| Margins: displayed at resolved columns (60, not 72) | `…::a_graph_with_margins_is_displayed_at_its_resolved_columns` |
| Hook sizes a nested fence to the list item's columns | `…::the_hook_sizes_a_nested_fence_to_its_available_columns` |
| Hook sizes to `available_width` even when the terminal snapshot is wider | `…::the_hook_sizes_to_the_available_columns_not_the_terminal_width`; mutation check: removing the hook's narrowing fails it (the nested-list test alone did not, because list narrowing already narrows the terminal) |
| Cell pixels: measured, fallback, never zero | `components::display::tests::*` (2) |
| Existing SVG images, graphical rules, file images unchanged | existing `biscuit-terminal` suite (3457 pass) and the unchanged 114 layout-matrix snapshots |

No parser or configuration reader was added; the input-robustness matrix does
not apply (`EncodedPng` validates a binary image, covered above).

### Gates

| Scope | Command | Result |
| --- | --- | --- |
| biscuit-terminal | `just test` | 3457 passed, 55 skipped (3446 + 11 new) |
| biscuit-terminal | `just test-l2` | lib 2 / cli 251 passed |
| biscuit-terminal | `just lint` | clean |
| biscuit-visualized | `just test` | 310 passed, 3 skipped (294 + 16 new) |
| biscuit-visualized | `just lint` | clean (`test-l2` not applicable) |
| both | `just check-tier-coverage` | nothing stranded |
| doctests | `cargo test --doc` (terminal_image filter; visualized all) | 9 / 29 passed |
| darkmatter | nextest `test(/mermaid\|image\|diagram\|graph/)` (lib, `effects-instrumentation`) | 281 passed |
| darkmatter-cli | same filter, `test-fixtures` | 19 passed |
| sniff-cli | `test(/graph\|deps\|dot\|image/)` | 23 passed |
| worktree + worktree-cli | `test(/graph/)` minus tier markers | 134 passed |
| biscuit-icon | `just test` | 230 passed |
| CI narrowed cells | darkmatter shipped-schema (2), sniff Cargo.lock (2), test-toolkit `archive-path-guard` | all pass |
| workspace | `cargo check --workspace --all-targets` | clean |
| Windows | `just cross-check biscuit-visualized --os windows --all-features` | 312 passed (after the store fix; fresh-key race check 3/3) |
| Windows | `just cross-check biscuit-terminal --os windows` | 3089 passed |

Gate-skip check: `git status --porcelain --untracked-files=all | awk '{print
$NF}' | xargs python3 scripts/ci/affected_scope.py` → `change_class:
package`, `packages: [biscuit-terminal, biscuit-visualized, test-toolkit,
darkmatter, sniff]`, so the gates were run, not skipped.

No pre-existing failures were hit; no test was load-sensitive on rerun except
the store race above, which was a real defect.

## Phase 3

### What was built

- **Route model** (`biscuit-terminal/lib/src/discovery/detection/route.rs`, new):
  `GraphicsRoute { format, transport, placement, confidence, reason }`,
  with `GraphicsFormat` (`Kitty|Iterm2|Sixel|None`), `GraphicsTransport`
  (`Direct|Tmux`), `GraphicsPlacement` (`Ordinary|Placeholders`),
  `RouteConfidence` (`Forced|Verified|Known|Unknown`), and
  `RouteReason { kind, message, fix }`. `RouteReasonKind` has the spec's five
  kinds plus `Supported` (a pixel route was chosen) and `NotATerminal`
  (needed so `force_graphics` can refuse piped output). All serialize as data.
- **Pure resolution**: `resolve_graphics_route(&RouteInputs)`, where
  `RouteInputs` carries TTY state, `is_windows`, a captured env map, the
  Zellij version, a `zellij_kitty_enabled` slot for the phase-4 pane query,
  and the Windows Terminal version. `RouteInputs::from_process()` reads the
  real process; `graphics_route()` resolves it. Order: nonterminal output →
  `BISCUIT_GRAPHICS`/`BISCUIT_GRAPHICS_TRANSPORT` → Windows SSH server →
  Zellij → tmux → known terminal (`TERM_PROGRAM`/`LC_TERMINAL`; inside SSH
  `LC_TERMINAL` wins) → local hints (`WT_SESSION`, `MSYSTEM`, `TERM=*kitty*`,
  `ITERM_*`) → unknown (probe slot).
- **One shared override validator** (`parse_choice`) for both variables:
  exact lowercase match on the raw `OsStr`; empty, padded, uppercase,
  unknown, and non-UTF-8 values are `InvalidInput` (never coerced to auto),
  with the accepted values in the message and `fix`; the value is
  `{:?}`-quoted so a control character cannot reach the terminal.
- **Versions** (`versions.rs`, new): `zellij --version` with a 500 ms
  deadline (no shell; child killed past the deadline; cached per process);
  on native Windows a Toolhelp32 parent-chain walk (≤ 16 hops, cycle-safe)
  to `WindowsTerminal.exe` and its `VS_FIXEDFILEINFO` file version
  (`windows-sys` 0.61, `cfg(windows)` only).
- **`ImageSupport`** gained `Sixel`; `image_support()` /
  `image_support_with_reason()` are projections of the route.
- **`Terminal`**: the public `image_support` field is replaced by
  `graphics_route` (authoritative), with `Terminal::image_support()` as the
  projection. Builder: `.graphics_route(..)` and `.image_support(..)` (a
  caller-supplied `Known` direct route via `From<ImageSupport>`).
  `TerminalRenderContext` lost its duplicate `image_support` field; the
  render tree reads the route from its `terminal` snapshot, so a
  caller-supplied snapshot is never re-resolved.
- **Policy**: `GraphicsRoute::forced_by_caller()` implements
  `force_graphics` (render tree) and darkmatter's `TerminalImageMode::Force`:
  it upgrades only `NoSupport`/`Unknown` routes to Kitty (`Forced`) and never
  lifts `UserDisabled`, `InvalidInput`, `BlockedTransport`, or `NotATerminal`.
  `GraphicsMode` remains the ceiling (existing gates; new test with a forced
  route).
- **`TerminalImage`**: `render_with_route(&term) -> Result<ImageOutput, _>`
  (`ImageOutput::Pixels` / `ImageOutput::Fallback { text, reason }`). A
  route without a usable encoder validates the image first (corrupt stays an
  error), then returns the alt text with control characters stripped. A Sixel
  route falls back with its own reason until phase 7 (ruling 2).
  `TerminalRenderable::render` now returns that alt text instead of `""`.
  The iTerm2 rule moved into `encoder_for`: a Kitty route to `TerminalApp::ITerm2`
  uses the iTerm2 format unless the route is `Forced`; its comment now gives
  the real reason (iTerm2 ignores `q=2`, replies `Gi=…;OK`, and places nothing
  for width-only `c=`), replacing the "cursor placement" comment.
- **`bt`** (`cli/src/hints.rs` new, `output.rs`, `commands/image.rs`,
  `commands/mermaid.rs`, `commands/graph.rs`): `bt --json` keeps
  `image_support` and adds `graphics_route`; the pretty report shows format,
  transport/placement/confidence, reason, and fix. One stderr hint per
  distinct reason per process, only when stderr is a terminal and the reason
  has a fix; `InvalidInput` reads as a configuration error. `bt image`
  prints the fallback text plus the hint; load errors stay errors.
- **Consumers**: biscuit-visualized `MermaidDiagram`/`GraphExpression`,
  the render tree's image path, darkmatter's legacy `ImageRenderer`, and
  biscuit-icon now match `ImageOutput` instead of treating `""` as "no
  image" (the `bt` agent found the diagram regression: piped `bt flowchart`
  printed `[Image: image]`). worktree-cli's private
  `detect_image_support_from_env` (a CLI making format decisions) is replaced
  by the library route resolved for stderr
  (`RouteInputs::from_process().with_output_is_tty(stderr_is_tty)`).
- **Housekeeping**: `viuer` removed from `biscuit-terminal/lib/Cargo.toml`
  (`DetectionMethod::Viuer` and `TerminalImageError::ViuerError` kept);
  both `dependencies.md` files updated.

### Departures and decisions

- **Rulings applied**: 1 (empty = invalid; non-UTF-8 invalid), 2 (Sixel
  route → fallback with a distinct reason), 3 (tmux → no pixel route,
  reason "planned", no fix since passthrough was not observed; a forced
  format or `BISCUIT_GRAPHICS_TRANSPORT=tmux` also resolves to this), 5
  (mintty via `TERM_PROGRAM=mintty`, `MSYSTEM` only when `TERM_PROGRAM` is
  absent; cterm via `TERM_PROGRAM=cterm`, an assumed value no one has
  verified; Konsole and wast were already identified), 7 (sniff exposes no
  public parent-chain-with-image API; the walk lives in biscuit-terminal —
  **gap noted for the sniff owner**).
- **Risk accepted under ruling 5**: Git Bash started in a bare legacy conhost
  window (no `TERM_PROGRAM`, `MSYSTEM` set, no `WT_SESSION`) is read as
  mintty and gets the iTerm2 format, which conhost should swallow as an
  unknown OSC. Not observed; listed for phase 9.
- **Windows Terminal outside native Windows** (WSL inside WT): `WT_SESSION`
  is honored on any OS when not in SSH, but the version is unreadable there,
  so it resolves to `Unknown` with the `BISCUIT_GRAPHICS=sixel` fix. The DA1
  signal in phase 4 is the real answer there.
- **Hidden tmux hint** (`TERM=tmux-*` without `TMUX`): not emitted in phase 3;
  naming `BISCUIT_GRAPHICS_TRANSPORT=tmux` would lead to a "planned" route.
  It belongs with the wrapped probe (phase 4) and tmux transport (phase 5).
- **`bt --json` with an invalid override while piped** reports
  `NotATerminal`, not `InvalidInput`: the spec's flowchart checks stdout
  before the override, and the plan's default outcome ("no query, nothing
  emitted") holds either way. The invalid-override hint is covered by a unit
  test that resolves with a terminal.
- **Windows SSH rule** checks `LC_TERMINAL` before `TERM_PROGRAM`, like
  every other SSH session (the docs agent found the first version inverted).
- **`render()` behavior change**: a terminal without a usable route used to
  get `""`; it now gets the alt text (`[Image: name]`), the plain-text
  fallback the spec asks for (half-blocks replace it in phase 8). Callers
  that keep their own placeholder style match `ImageOutput` instead.
- **Behavior change by spec**: `force_graphics` / darkmatter `Force` no longer
  emits graphics into piped stdout (route `NotATerminal`). A caller-supplied
  snapshot (`.image_support(ImageSupport::None)`) still upgrades, so the
  existing force tests pass unchanged; darkmatter's
  `test_image_renderer_force_mode_attempts_protocol_render` now supplies its
  terminal explicitly, and a new test pins the piped case.
- **Stale doc fixed** (left by phase 2): `terminal_image.md` "Programmatic
  Use" now uses `TerminalImage::new(Path)` / `from_spec("photo.jpg|50%")`.
- **Tooling note**: `just cross-check … -E '…'` breaks on parentheses (the
  recipe re-splits arguments); positional nextest filters work.

### Requirement-to-test mapping

| Behavior | Test (tier, rung) |
| --- | --- |
| Every Goals-table row the environment decides: direct identities (unchanged), mintty/`MSYSTEM`/cterm/`LC_TERMINAL`, non-tty beats overrides, SSH with/without hints and `LC_TERMINAL` precedence, remote `WT_SESSION`/`ITERM_*` not identity, Windows SSH server (iTerm2 list → iTerm2; Kitty/Sixel blocked, forced too), Windows Terminal versions (≥1.22 Sixel, older + fix, unreadable + `BISCUIT_GRAPHICS=sixel` fix), Zellij (version × enabled state independent; iTerm2 never), tmux interim, `BISCUIT_GRAPHICS_TRANSPORT=direct/tmux`, every override value, policy of forced format vs blocked transport, projection | `biscuit-terminal` `l1 graphics_route_matrix::*` (79 tests, L1, F1 through the public `resolve_graphics_route`) |
| Shared override validator shapes: absent (control), `""`, `KITTY`, `potato`, `" kitty"`, `"kitty\n"`, non-UTF-8 (unix), control-character value never reaches the message raw; transport wiring (control + `TMUX`) | same file, `graphics_value_*`, `transport_*` rows |
| `from_process` keeps a non-UTF-8 override raw; reads the override variables | `discovery::detection::route::tests::from_process_*` (L1, F2: real process env) |
| `force_graphics` lifts only NoSupport/Unknown; never UserDisabled/InvalidInput/NotATerminal/BlockedTransport; pixel routes kept | `route::tests::a_caller_force_lifts_only_no_support_and_unknown`; render tree `render_tree_force_graphics_does_not_lift_user_disabled_graphics` (F2, renders the tree) |
| `GraphicsMode` caps a forced route; Rich control sends it | `render_tree_graphics_mode_caps_a_forced_route` |
| Existing force behavior on caller snapshots unchanged | existing `render_tree_image_force_graphics_emits_protocol_on_unsupported_terminal`, `…thematic_break_force_graphics…`, `diagram_hook_receives_the_forced_terminal_snapshot` |
| darkmatter `Force`: caller snapshot upgrades; piped route stays none | `markdown::output::terminal::tests::test_image_renderer_force_mode_*` (2) |
| Caller-supplied `ImageSupport` → Known direct route, projection round-trips; route serializes as data | `route::tests::a_caller_supplied_image_support_is_a_known_direct_route`, `…the_route_serializes_as_data` |
| `TerminalImage` fallback: alt text + route reason; explicit alt wins; Sixel interim reason; `render_inline` refuses Sixel; corrupt image stays an error; controls stripped | `components::terminal_image::tests::route_dispatch::*` (5 render tests) |
| Encoder choice: Known Kitty on iTerm2 → iTerm2; forced Kitty on iTerm2 → Kitty; iTerm2 route on any app | `route_dispatch::*` (3 `encoder_for` tests) |
| Version parsing and floors; deadline kills a slow child; parent-chain walk (case, self, missing, cycle, hop limit); `VS_FIXEDFILEINFO` decode; Windows read does not panic | `route::tests::version*`, `versions::tests::*` (Windows tests run on build-win-native) |
| `bt`: hint decision (tty, fix, once per reason, distinct reasons, config-error label, resolved invalid override); report lines; `--json` route field; piped `bt image` fallback without controls; corrupt image fails | `biscuit-terminal-cli` `hints::tests::*` (6), `output::tests::images_line_*` (2), `test_terminal_metadata_serialization`; `l1 graphics_route::*` (4, F3: spawns `bt`) |
| Piped diagram commands still print the source block (regression the `bt` agent found) | existing `integration_test::test_graph_expression_falls_back_to_code_block_on_non_tty`, `…dot_fallback_uses_dot_info_string` (failed before the visualized fix, pass after) |

Input robustness: the default outcome (no pixel route, accepted values in
the reason and fix, no query, nothing emitted) holds for every invalid shape;
the only exceptions are absent/`auto` (detection) and `none`
(`UserDisabled`). Code-smell grep over `route.rs`/`versions.rs`: no
`#[serde(default)]`, `unwrap_or_default()`, or `filter_map(.. as_str())`;
the `.ok()` uses are version parsing and Win32 conversions, where failure
means "unreadable", never the override parse.

### Gates

| Scope | Command | Result |
| --- | --- | --- |
| biscuit-terminal | `just test` | 3565 passed, 55 skipped |
| biscuit-terminal | `just test-l2` | lib 2 / cli 251 passed |
| biscuit-terminal | `just lint` | clean |
| biscuit-terminal | `just check-tier-coverage biscuit-terminal` | nothing stranded |
| biscuit-visualized | `just test` / `just lint` | 310 passed, 3 skipped / clean |
| darkmatter | `--features terminal-tests,effects-instrumentation --tests` check; nextest `test(/image\|mermaid\|diagram\|graph\|force\|Force\|hr_\|horizontal_rule\|render_tree/)` | compiles; 622 passed |
| darkmatter-cli | `test-fixtures`, `test(/image\|mermaid\|diagram\|graph\|render/)` | 78 passed |
| worktree-cli | `test(/list\|graph\|image/)` minus tier markers | 216 passed |
| biscuit-icon + cli | `--all-features` (all tests) | 241 passed |
| sniff-cli | `test(/graph\|deps\|dot\|image/)` | 23 passed |
| workspace | `cargo check --workspace --all-targets`; clippy `-D warnings` on darkmatter, worktree-cli, biscuit-icon(+cli) | clean |
| Windows | `just cross-check biscuit-terminal --os windows` | 3183 passed (also `versions`/`route` filtered, 15/15, by the subagent) |

Gate-skip check: `git status --porcelain --untracked-files=all | awk '{print
$NF}' | xargs python3 scripts/ci/affected_scope.py` → `change_class:
package`, `packages: [biscuit-icon, biscuit-icon-cli, biscuit-terminal,
biscuit-terminal-cli, biscuit-visualized, darkmatter, worktree-cli,
test-toolkit, sniff, tree-hugger-cli]` (the tree also holds phase 2's
uncommitted changes), so the gates were run, not skipped.

Pre-existing, not caused by this phase: `biscuit-icon` warns `unused import
super::*` in `render_in_cell_tests` under `--features image` (present at
HEAD); `biscuit-terminal --all-features` fails to compile the `serde`
feature's `horizontal_rule` test module (`HorizontalRule` not in scope; the
`serde` feature is not in CI's feature list). No test was load-sensitive.
