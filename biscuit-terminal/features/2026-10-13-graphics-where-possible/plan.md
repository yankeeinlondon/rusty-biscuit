---
total_phases: 9
created: 2026-10-21
phase: 1
agent: "claude/sonnet"
yolo: true
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
source_files_during_phase_4:
    - "Cargo.lock"
    - "biscuit-terminal/lib/Cargo.toml"
    - "biscuit-terminal/lib/src/discovery/mod.rs"
    - "biscuit-terminal/lib/src/discovery/raw_mode.rs"
    - "biscuit-terminal/lib/src/discovery/startup_exchange.rs"
    - "biscuit-terminal/lib/src/discovery/tty_query.rs"
    - "biscuit-terminal/lib/src/discovery/tty_query/mod.rs"
    - "biscuit-terminal/lib/src/discovery/tty_query/unix.rs"
    - "biscuit-terminal/lib/src/discovery/tty_query/windows.rs"
    - "biscuit-terminal/lib/src/discovery/detection/mod.rs"
    - "biscuit-terminal/lib/src/discovery/detection/probe.rs"
    - "biscuit-terminal/lib/src/discovery/detection/route.rs"
    - "biscuit-terminal/lib/src/discovery/detection/color.rs"
    - "biscuit-terminal/lib/src/discovery/osc_queries/query.rs"
    - "biscuit-terminal/lib/src/discovery/osc_queries/support.rs"
    - "biscuit-terminal/lib/src/terminal.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/protocol.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/tests.rs"
    - "biscuit-terminal/lib/examples/discovery_probe.rs"
    - "biscuit-terminal/lib/tests/common/pty.rs"
    - "biscuit-terminal/lib/tests/l1/main.rs"
    - "biscuit-terminal/lib/tests/l1/graphics_probe_matrix.rs"
    - "biscuit-terminal/lib/tests/l1/graphics_route_matrix.rs"
    - "biscuit-terminal/lib/tests/l1/level1_graphics_probe.rs"
    - "biscuit-terminal/lib/tests/level2/main.rs"
    - "biscuit-terminal/lib/tests/level2/level2_graphics_probe_wezterm.rs"
    - "biscuit-terminal/lib/tests/level2/level2_windows_console_query.rs"
    - "biscuit-terminal/cli/src/output.rs"
docs_updated_during_phase_4:
    - "biscuit-terminal/docs/discovery/terminal-queries.md"
    - "biscuit-terminal/docs/discovery/graphics-routes.md"
    - "biscuit-terminal/docs/components/terminal_image.md"
    - "biscuit-terminal/docs/research/graphics/multiplexing.md"
    - "biscuit-terminal/lib/README.md"
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - ".claude/skills/biscuit-terminal/SKILL.md"
    - ".claude/skills/biscuit-terminal/cli.md"
    - ".claude/skills/biscuit-terminal/discovery.md"
    - ".claude/skills/biscuit-terminal/image-rendering.md"
    - ".claude/skills/biscuit-terminal/terminal-struct.md"
    - ".claude/skills/os/windows-console.md"
source_files_during_phase_5:
    - "biscuit-terminal/lib/src/discovery/detection/tmux.rs"
    - "biscuit-terminal/lib/src/discovery/detection/mod.rs"
    - "biscuit-terminal/lib/src/discovery/detection/route.rs"
    - "biscuit-terminal/lib/src/discovery/detection/probe.rs"
    - "biscuit-terminal/lib/src/discovery/startup_exchange.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/tmux.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/mod.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/kitty.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/protocol.rs"
    - "biscuit-terminal/lib/src/components/terminal_image/tests.rs"
    - "biscuit-terminal/lib/examples/discovery_probe.rs"
    - "biscuit-terminal/lib/tests/l1/main.rs"
    - "biscuit-terminal/lib/tests/l1/graphics_route_matrix.rs"
    - "biscuit-terminal/lib/tests/l1/graphics_tmux_matrix.rs"
    - "biscuit-terminal/lib/tests/level2/main.rs"
    - "biscuit-terminal/lib/tests/level2/level2_graphics_tmux.rs"
docs_updated_during_phase_5:
    - "biscuit-terminal/docs/discovery/graphics-routes.md"
    - "biscuit-terminal/docs/discovery/terminal-queries.md"
    - "biscuit-terminal/docs/components/terminal_image.md"
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - ".claude/skills/biscuit-terminal/discovery.md"
    - ".claude/skills/biscuit-terminal/image-rendering.md"
---

# Plan: Graphics Where Possible

Implements the finalized spec in this folder (`spec.md`). Section numbers
(§) below refer to that spec's **Design** sections.

## Summary and Definition of Done

### The work

Three bodies of work, delivered in nine phases:

1. **Package boundary flip (phase 1).** `MermaidDiagram`, `GraphExpression`,
   and `GitGraph` move from `biscuit-terminal` to `biscuit-visualized`
   (exposed as `biscuit_visualized::components`). `biscuit-visualized` gains an
   optional `terminal` feature depending on `biscuit-terminal`;
   `biscuit-terminal` drops `biscuit-visualized` but keeps optional `resvg`.
   The render-tree renderer, which called `MermaidDiagram` directly, gets an
   `Rc` diagram hook that `biscuit-visualized` implements and `bt` /
   darkmatter install. No behavior change.
2. **Sizing and encoding fixes (phase 2).** Diagrams render at final display
   width; diagram PNGs reach the Kitty/iTerm2 encoders without re-encoding;
   `RenderRequest::transparent_background` becomes a three-way background
   choice; PNG encoding uses `CompressionType::Default` + `FilterType::Adaptive`.
3. **Route model and its inputs (phases 3-8).** A `Route`
   (format, transport, placement, confidence, reason) replaces the single
   `ImageSupport` answer and is authoritative for `TerminalImage`, diagram
   components, `bt`, and the capability report. It is fed by: env/identity
   rules and overrides (3), a probe folded into the existing color round trip
   incl. a native Windows backend (4), tmux passthrough/wrapping (5), Unicode
   placeholders (6), Sixel via `icy_sixel` (7), and a half-block fallback for
   plain images (8). Phase 9 writes the human verification checklist.

### What success looks like

Every item is observable without a human at a terminal:

- [x] `cargo tree -p biscuit-terminal -e normal` (and with `--features image`)
      contains neither `biscuit-visualized` nor `mermaid-rs-renderer`;
      `resvg` is present only under `image`. `cargo tree -p biscuit-visualized`
      (no features) has no `biscuit-terminal`; with `terminal` there is no cycle.
- [x] The moved components' existing tests pass unchanged from
      `biscuit-visualized`; without a render hook a Mermaid fence stays a code block.
- [ ] A diagram PNG whose rendered size equals the display size reaches the
      Kitty/iTerm2 encoder byte-identical (L1 assertion); the 400x400 icon
      sample encodes smaller than before.
- [ ] An environment-variable L1 matrix covers every Goals-table row that the
      environment alone decides, with the outcome the table names.
- [ ] `BISCUIT_GRAPHICS` / `BISCUIT_GRAPHICS_TRANSPORT` follow the override
      rules (invalid value = no pixel route + accepted values in the reason).
- [ ] Windows SSH server sessions never get Kitty or Sixel; mintty gets iTerm2.
- [ ] `bt` prints at most one stderr hint per distinct reason, only when stderr
      is a terminal; `--json` carries the reason as data.
- [ ] The probe parser handles fragmented/out-of-order replies; known
      incompatible terminals, CI, pipes, and Windows-SSH are never queried.
- [ ] tmux wrapping (ESC doubling, per-chunk), the 1 MiB iTerm2 budget, and
      placeholder grid encoding pass L1; tmux L2 passes with passthrough on/off.
- [ ] Sixel output is deterministic for fixed fixtures and leaves terminal mode
      state unchanged.
- [ ] Plain images with no pixel route render half-blocks (truecolor + Unicode),
      else alt text; pipes get plain text.
- [ ] `just test`, `just test-l2`, `just lint` pass for `biscuit-terminal`,
      `biscuit-visualized`, and every migrated consumer; Windows backend and
      route tests pass via `just cross-check <pkg> --os windows` on `$BUILD_WIN`.
- [ ] Docs, skills, READMEs, and `dependencies.md` files updated per the spec's
      Documentation section; `verification.md` written (phase 9).

### Conventions for every phase

- Use `just test` / `just test-l2` / `just lint`; never `cargo test`, never
  `cargo fmt`, never commit unless asked. Load `rust-testing` before writing tests.
- L2 terminal windows must never take focus (harness `SpawnVisibility`).
- Every behavior change includes a pass over the symbol's `///` docs and
  inline comments (repo comment-quality rules).
- Docs describe current behavior and never name a feature/fix spec; behavior
  not yet built is marked **planned** until the landing change removes the marker.
- US English everywhere.
- Subagents in this plan run in a **non-interactive** session tree; briefs must
  say so and never wait on another subagent's output.

---

## Phase 1: Component move (and pre-work)

**Goal:** diagram components live in `biscuit-visualized`; the dependency
direction is flipped; behavior is unchanged. Also the home for rulings and
spikes that inform later phases.

### Necessary Rules

Rulings the spec leaves open. Each carries a **proposed answer** so work can
proceed (`yolo: true`); the author may overturn any of them, and the affected
phase is named.

1. **Empty-string overrides (phases 3).** `BISCUIT_GRAPHICS=""` and
   `BISCUIT_GRAPHICS_TRANSPORT=""` are *invalid* (spec says "exactly" the
   listed lowercase values), not unset. Non-UTF-8 values are invalid too.
   Proposed: invalid.
2. **Sixel before the encoder exists (phases 3→7).** The `Sixel` format is
   resolvable from phase 3 (forced, Windows Terminal version signal), but the
   encoder lands in phase 7. Until then `TerminalImage` treats a Sixel route
   as "no usable encoder" → fallback with a distinct reason. Proposed: yes;
   remove the interim in phase 7.
3. **tmux before phase 5 (phase 3).** Today tmux resolves to `None`.
   Phase 3 keeps `None` with reason "tmux transport not yet implemented"
   (mark **planned** in docs) and does not regress any working setup.
4. **Phase 7 dependency (spec table says 2, 3, 4).** The spec's phase 7 also
   covers "tmux's own Sixel handling", which needs tmux detection from phase 5.
   Proposed: phase 7 depends on 5 as well (the sequential order here already
   satisfies it).
5. **How mintty / cterm / wast / Konsole are identified (phase 3).** The spec
   lists them in tables but not the env signals. Proposed: reuse the existing
   `TerminalApp` detection where it already identifies them; add mintty via
   `TERM_PROGRAM=mintty` (set by mintty >= 3.x) with `MSYSTEM` as a secondary
   hint only when `TERM_PROGRAM` is absent. Anything not found in the existing
   detector and not in this list is a gap the implementer records, not invents.
6. **"Legacy conhost with no modern terminal" (phase 4).** Defined for the
   never-probe rule as: Windows, no `WT_SESSION`, no `TERM_PROGRAM`, not
   mintty/ConEmu/WezTerm, and not detected as an SSH server session.
   Proposed: this definition; record it in `graphics-routes.md`.
7. **Reading the `WindowsTerminal.exe` file version (phase 3).** Walking the
   parent chain should use `sniff` (host discovery rule); reading the file
   version needs a Windows API call (`windows-sys`, cfg(windows) only).
   Proposed: confirm `sniff` exposes parent-process image paths; if not, do the
   walk inside `biscuit-terminal` behind `cfg(windows)` and note the gap for
   the `sniff` owner. Either way, version unreadable = "signal unavailable".
8. **TUI escape-hatch API shape (phase 4).** Spec requires "supply a resolved
   route, or turn construction-time probing off" without naming the API.
   Proposed: `TerminalBuilder`-style options on `Terminal` construction
   (`Terminal::new_with(TerminalOptions { route: Option<Route>, probe: ProbeMode })`
   where `ProbeMode::{Auto, Off}`), plus env `BISCUIT_TERMINAL_PROBE=off`
   for non-library callers. The implementer reconciles with the existing
   `new_optimistic` constructor and records the final names in the docs.
9. **Placeholder image-ID allocation (phase 6).** Spec requires distinct
   nonzero IDs per live upload and ID-in-foreground-color encoding.
   Proposed: a per-process monotonically increasing counter in 24-bit range
   (truecolor fg) wrapping past 0xFFFFFF back to 1; 256-color terminals use
   ordinary placement per the spec's color-fidelity rule.
10. **Wider measurements are the author's call.** Not scheduled:
    Sixel encode time on large diagrams across hosts, Kitty placeholder
    behavior on non-macOS hosts, and tmux < 3.3 behavior (excluded by the
    version floor). Listed so the author can request them.
11. **Env-var input matrix scope.** `BISCUIT_GRAPHICS*` is configuration input
    (see *Input robustness* in phase 3). Default outcome (stated once): every
    invalid value yields no pixel route, a reason listing the accepted values,
    no query written, and no output. Exceptions: absent and `auto` → detection.

### Spikes

Each runs once, here, on the current macOS host; results are recorded in
`## Spike results` at the bottom of this file and in the implementation log.
Neither widens beyond what the spec asks.

- [x] **S1: Zellij pane Kitty query** *(informs phase 4)*
    - If `zellij --version` >= 0.45 is installed: run Zellij in a windowless
      harness/tmux-free pty (no focus), send the spec's one-pixel `a=q` query
      plus a DA1 sentinel, record whether Zellij replies `OK` for that ID,
      and whether the reply arrives before DA1.
    - If Zellij >= 0.45 is not installed: record "not runnable here"; the L2
      Zellij tests skip through the harness path, and phase 4 ships the
      interim rule untested live.
- [x] **S2: tmux format variables** *(informs phase 5/7)*
    - On the host's tmux, run `tmux -V` and
      `tmux display -p '#{client_termname}|#{client_termtype}|#{client_termfeatures}'`
      for an attached client in a throwaway session; confirm the three
      variables exist, and record their exact output shapes (e.g. whether
      `sixel` appears as a bare comma-separated feature) and the minimum tmux
      version that has `client_termfeatures`.

### Wave 1 (parallel: spikes and mapping)

- [x] **Spikes S1 and S2** as above (two independent subagents; each brief
      states the session is non-interactive and must not take window focus).
- [x] **Consumer inventory**
    - Search the whole workspace (not just the spec's list) for
      `MermaidDiagram`, `GraphExpression`, `GitGraph`, the prelude re-exports,
      examples, doctests, `layout_matrix_support`, and diagram-only font/raster
      helpers. Initial hits: `biscuit-terminal/cli` (mermaid/graph/git_graph
      commands, args, main), `biscuit-terminal/lib` (prelude, render_tree,
      terminal_image, tests: `mermaid_parity`, `graph_expression_parity`,
      `layout_matrix`, `render_tree_code_context`), `worktree/cli` and
      `worktree/lib/src/graph`, `sniff/cli`, `darkmatter/lib`
      (`mermaid/render_terminal.rs`, `markdown/render_tree/*`).
    - Write the final list as a checklist under "Wave 3".
- [x] **Baseline capture**
    - Run `just test` and `just test-l2` for `biscuit-terminal` and
      `biscuit-visualized`, plus `just test` for darkmatter, sniff-cli, worktree
      (lib + CLI); record pass counts so "no behavior change" is checkable.
      Snapshot the `cargo tree` for `biscuit-terminal` (before).

### Wave 2 (sequential: seams in `biscuit-terminal`)

*Prerequisite: Wave 1 inventory.*

- [x] **Render hook trait**
    - In `biscuit-terminal/lib/src/render_tree/options.rs` add a diagram hook
      (trait object, `Rc` ownership matching the existing hooks). Input:
      source, node attributes, effective terminal/route, available width,
      layout. Output: rendered string or typed error.
    - Consulted only for Mermaid promotion at `Rich` fidelity with
      `TerminalMermaidMode::Image`, **before** the ordinary code-rendering hook.
      No hook → fall through to code block; preserve strict/warn/lossy
      handling (existing `mermaid_promotion_failure_rejects_under_strict`
      test keeps passing). Child option clones retain the hook.
    - Replace the direct `MermaidDiagram` call in `render_tree/render.rs` (~l.1750).
    - Capability debug trace records "Mermaid promotion requested without hook".
- [x] **Terminal feature surgery**
    - `biscuit-terminal/lib/Cargo.toml`: remove `biscuit-visualized` dep and the
      diagram half of `image`; `image = ["dep:resvg"]` (plain SVG images and
      graphical rules keep working).

### Wave 3 (parallel: move + consumers)

*Prerequisite: Wave 2 compiles (use temporary scaffolding as needed).*

- [x] **Move components (visualized side)**
    - `git mv` `mermaid.rs`, `graph_expression.rs`, `git_graph.rs` (+
      `git_graph/` and `tests.rs`) into `biscuit-visualized/src/src/components/`;
      expose as `biscuit_visualized::components::{MermaidDiagram,GraphExpression,GitGraph}`
      so they do not collide with the artifact-level `MermaidDiagram` names.
    - `biscuit-visualized/src/Cargo.toml`: optional `biscuit-terminal`
      (+`renderable` if required) behind a `terminal` feature that enables the
      existing `image` feature and `biscuit-terminal/image`. Preserve `clap`
      gating. No terminal-crate re-export (it would restore the cycle).
    - Implement the render hook in `biscuit-visualized` (adapter renders
      directly, never recursively promoting its own fence).
    - Move the parity/matrix tests into `biscuit-visualized` test targets;
      register in the new target roots, keep tier/feature gating. Update
      `[package.metadata.ci.tests]` and the justfile (`--all-features`) if
      the feature layout changes.
- [x] **Migrate consumers** (one subagent per package, disjoint files; each
      brief is non-interactive and names its files):
    - [x] `biscuit-terminal/cli` (`bt` diagram commands; installs the hook)
    - [x] `darkmatter` (installs the hook; `mermaid/render_terminal.rs`, render_tree)
    - [x] `sniff/cli` (`deps.rs`, `args/repo.rs`)
    - [x] `worktree` lib + CLI (`list_table.rs`, `git_graph.rs`, tests, graph tests)
    - [x] Remaining inventory hits (renderable references, examples, doctests)
- [x] **Consumer inventory (final list, from the Wave 1 search)**
    - [x] `biscuit-terminal/lib/src/components/{mermaid,graph_expression,git_graph}.rs`,
          `git_graph/tests.rs` (move; `components/mod.rs` and `prelude.rs` drop them)
    - [x] `biscuit-terminal/lib/src/render_tree/render.rs` (direct `MermaidDiagram`
          call and `mermaid_promotion_failure_rejects_under_strict`)
    - [x] `biscuit-terminal/lib/tests/l1/{mermaid_parity,graph_expression_parity}.rs` (move)
    - [x] `biscuit-terminal/lib/tests/l1/layout_matrix.rs` +
          `tests/layout_matrix_support/mod.rs` (diagram rows + their insta snapshots move)
    - [x] `biscuit-terminal/lib/tests/l1/render_tree_code_context.rs`
          (`mermaid_rich_mode_with_opt_in_bypasses_code_renderer`)
    - [x] `biscuit-terminal/cli`: `commands/{mermaid,graph,git_graph,quadrant}.rs`,
          `main.rs` (`available_font_families`), `Cargo.toml`
    - [x] `darkmatter/lib`: `mermaid/render_terminal.rs`,
          `markdown/render_tree/code_renderer.rs` (`render_to_svg`),
          `markdown/render_tree/entrypoints.rs` (install hook), struct-literal
          `TerminalRenderOptions` sites (src + tests + benches), `Cargo.toml`
    - [x] `sniff/cli`: `output/filesystem/deps.rs`, `Cargo.toml`
          (`args/repo.rs` only names the command in docs)
    - [x] `worktree/cli`: `commands/git_graph.rs`, `commands/git_graph/tests.rs`,
          `commands/list_table.rs`, `Cargo.toml` (`worktree/lib` only names
          `GitGraph` in comments)
    - [x] Examples/doctests: `biscuit-terminal/lib/examples/layout_matrix.rs`
          (uses the support module), doctests inside the moved files
    - [x] Docs naming the moved types (`docs/components/*`,
          `docs/data-visualization/*`, READMEs, `dependencies.md`, skills)
- [x] **Docs move (mechanical)**
    - Move `docs/components/{mermaid_diagram,graph_expression,git_graph}.md` and
      any `docs/data-visualization/` pages describing them to
      `biscuit-visualized/docs/`; fix links both ways.

### Wave 4 (validation checkpoint)

- [x] `cargo tree -p biscuit-terminal -e normal` and `--features image`: no
      `biscuit-visualized`, no `mermaid-rs-renderer`; `resvg` only under `image`.
- [x] `cargo tree -p biscuit-visualized` (default and `--features terminal`):
      artifact-only has no `biscuit-terminal`; terminal enabled has no cycle.
- [x] Moved tests pass **unchanged** (diff the test bodies: only imports differ).
- [x] A render tree without the hook leaves Mermaid fences as code (new L1).
- [x] `just test`, `just test-l2`, `just lint` for `biscuit-terminal`,
      `biscuit-visualized`, darkmatter, sniff-cli, worktree (lib + CLI); counts
      match the Wave 1 baseline.
- [x] Windows compile evidence: `just cross-check biscuit-visualized --os windows`
      and `biscuit-terminal`.
- [x] Record spike results.

---

## Phase 2: Sizing and double encoding

**Goal:** diagrams are rasterized once, at final display size, and their PNG
bytes pass through unchanged (§7, §9 PNG item). *Depends on phase 1.*

### Wave 1 (parallel, disjoint packages)

- [x] **Background enum** (`biscuit-visualized`)
    - Replace `RenderRequest::transparent_background: bool` with an explicit
      enum: theme background (default, preserves today's exports) / transparent /
      solid RGB. Apply consistently to SVG and PNG
      (`graph/render.rs` ~l.394/447/477, mermaid render, `artifact.rs`).
    - Update every `RenderRequest` literal (tests in `mermaid_tests.rs`,
      `artifact_tests.rs`, worktree spike files only if they compile in CI).
    - Cache identity: include resolved background, target width/height/padding,
      and PNG encoding options in the `biscuit-hash` key; bump the cache
      schema/version so old artifacts invalidate. Tests: same source with
      different background or geometry ⇒ different keys.
- [x] **PNG encoder settings**
    - `biscuit-visualized/src/src/raster/png.rs` and `TerminalImage`'s
      `encode_as_png`: use `PngEncoder::new_with_quality(CompressionType::Default,
      FilterType::Adaptive)`.
    - Acceptance: the 400x400 iTerm2 app icon from scoping encodes smaller than
      before, measured once on one host; record numbers in the implementation log.
- [x] **Encoded-PNG input path** (`biscuit-terminal`)
    - Add a `TerminalImage` input that takes already-encoded PNG bytes with
      pixel dimensions and cell footprint carried alongside. Existing
      path-based validation and image access policy stay in force for
      caller-supplied files; the internal artifact path is not a bypass.
    - Kitty/iTerm2 encoders send those bytes unchanged when the size matches;
      decode only for Sixel/half-blocks, validation that needs it, or an actual
      resize. Enforce decode/allocation limits before reserving rows.
    - Apply to **both** the inline and standalone dispatch paths (currently
      separate in `terminal_image/mod.rs`).

### Wave 2 (sequential; prerequisite: Wave 1)

- [x] **Display-size rendering in the components**
    - In `biscuit-visualized::components`, compute width via the existing
      `terminal_image/width.rs` resolver (`ImageWidth` precedence; no
      `Layout::width`/`max_width`), honoring parent available width and
      indentation. Measure cell pixels when available, otherwise the
      documented estimate; never zero.
    - Call `RenderRequest::target_width` with the pixel width; derive height
      from aspect ratio, round the reserved cell rectangle up, pad unused
      pixels rather than stretching. Recompute on resize.
    - Terminal adapters request transparent background for Kitty/iTerm2 (solid
      for Sixel arrives in phase 7).

### Wave 3 (validation checkpoint)

- [x] L1: PNG bytes reach the encoder unchanged when sizes match; aspect ratio
      and padding; background cache separation; inline/standalone dispatch
      parity; existing SVG image and graphical-rule behavior unchanged.
- [x] `just test`, `just test-l2`, `just lint` for both packages and consumers.

---

## Phase 3: Route model

**Goal:** a `Route` replaces the single image answer; deterministic
environment-only resolution; overrides; Windows-SSH/mintty/WT/Zellij rules;
capability report; hints; housekeeping (§1, §5 detection, §6, §9). *Depends on phase 1.*

### Input robustness (override variables)

Applies to `BISCUIT_GRAPHICS` and `BISCUIT_GRAPHICS_TRANSPORT` only (the probe
reply parser is covered in phase 4's L1 list; it is not file/config input).
**Default outcome:** an invalid value yields no pixel route, a reason listing
the accepted values, no query written, nothing emitted. **Exceptions:** absent
and `auto` → detection; `none` → deliberate disabling (not an error).

- [x] **One shared validator** parses both variables (they differ only in
      accepted set). Test shapes once at that validator, through the public
      route result: absent, empty (invalid, ruling 1), uppercase (`KITTY`,
      invalid), unknown word, whitespace-padded, non-UTF-8 (unix), and the
      control case (each valid value).
- [x] Per-variable wiring: each variable gets the control case plus one
      invalid shape proving it uses the validator.
- [x] Code-smell grep before closing: `#[serde(default)]` on load-bearing
      fields, `unwrap_or_default()`, `.ok()` on the env parse, `filter_map(.. as_str())`.
      An invalid value must never be coerced to `auto`.

### Wave 1 (sequential core)

- [x] **Route type**
    - New module in `discovery/detection/` (e.g. `route.rs`): format
      (`Kitty|Iterm2|Sixel|None`), transport (`Direct|Tmux`), placement
      (`Ordinary|Placeholders`), confidence (`Forced|Verified|Known|Unknown`),
      reason (structured: kind = no-support / unknown / user-disabled /
      blocked-transport / invalid-input, plus optional fix text only when the
      blocker was observed).
    - `ImageSupport` gains `Sixel`; `image_support()` / `image_support_with_reason()`
      become projections of the route. Update exhaustive matches, serialization
      tests, JSON consumers together (biscuit-icon, worktree-cli, darkmatter,
      `bt` output). Half-blocks are not image-protocol support.
    - Route is authoritative: update the public capability field and
      render-tree terminal snapshot together, including caller-supplied
      snapshots; never re-resolve per image. `TerminalImage` consumes the route.
- [x] **Resolution function** implementing the §1 flowchart for environment-only
      inputs: non-tty → none; override; Windows+`SSH_CONNECTION` rule; multiplexer
      branch (Zellij interim rule; tmux interim per ruling 3); known terminals
      from env and `LC_TERMINAL`; "unknown" leaves the probe slot for phase 4.
    - Known-terminal format table (Kitty family → Kitty; iTerm2/mintty/cterm →
      iTerm2; Terminal.app → none; WT → Sixel by version signal only until
      phase 4 adds the DA1 signal). iTerm2 list = iTerm2, WezTerm, Konsole,
      mintty, cterm; iTerm2 format by identity only, never by probe.
    - Conflict rule: current client information beats inherited hints; generic
      `xterm-256color`, remote-inherited `WT_SESSION`, and a mux server's old
      env do not establish identity.
- [x] **Policy ceilings** — `GraphicsMode` (`Off` / `Vector` / `Rich`),
      `TerminalMermaidMode`, `BISCUIT_GRAPHICS=none` (disables pixels and
      half-blocks), `force_graphics` (works only when env is `auto`; cannot
      override `none`, lower ceiling, non-tty, blocked transport). Pipes never
      get graphics controls or queries.

### Wave 2 (parallel; prerequisite: Wave 1)

- [x] **Windows Terminal version signal** (`cfg(windows)`): parent-chain walk to
      `WindowsTerminal.exe`, file version >= 1.22 ⇒ Sixel (rulings 7). Unit-test
      the version-comparison logic on all OSes; the file read on Windows only.
- [x] **Zellij gating**: `zellij --version` under 500 ms deadline, no shell
      interpolation. 0.45+ with a known Kitty-family inherited identity ⇒ Kitty,
      ordinary placement, confidence `Known` (interim, until phase 4). Earlier,
      unknown version, or Kitty disabled ⇒ fallback with reason (Sixel needs
      phase 4's DA1). Never tmux wrapping, placeholders, or iTerm2 format.
      Enabled/disabled state is independent of version in the L1 matrix.
- [x] **Capability report and hints**
    - `bt` (no args, `--json`): show route + reason; keep the existing
      image-support field and add route fields.
    - Hint: one stderr line, only if stderr is a terminal, at most once per
      distinct reason per invocation; library returns structured reasons and
      never writes. JSON stays parseable (reason as data). Name a fix only when
      its blocker was observed (e.g. `allow-passthrough` only when tmux
      passthrough was observed off).
    - `TerminalImage`/`bt image` with no usable route: valid image ⇒ fallback
      (text path until phase 8) with reason; invalid paths, policy violations,
      corrupt images stay errors. Resolve/load/encode **before** reserving rows.
      Partial write ⇒ return I/O failure, no second format appended.
- [x] **Housekeeping**
    - Correct the `protocol.rs:49-50` comment (reason: iTerm2 ignores `q=2`,
      replies `Gi=…;OK`, and width-only `c=` places nothing).
    - Remove `viuer` from `biscuit-terminal/lib/Cargo.toml`; leave
      `DetectionMethod::Viuer` and the `TerminalImageError` viuer variant.
      Update `docs/dependencies.md`.

### Wave 3 (parallel docs/tests; prerequisite: Wave 2)

- [x] **L1 env matrix**: one test per Goals-table row the environment decides
      (direct, Zellij enabled/disabled/old/unknown, tmux interim, SSH no hints,
      Windows SSH ± iTerm2-list outer, mintty, WT, Terminal.app/conhost,
      overrides, invalid overrides, forced format vs blocked transport,
      policy precedence). Each with a control row.
- [x] **Docs**: create `docs/discovery/graphics-routes.md` (developer-first:
      route table, both env vars, hints; unbuilt items marked **planned**);
      update `docs/components/terminal_image.md`; README overview touch-ups.
- [x] **`os` skill**: add the Windows OpenSSH/system-conhost finding
      (APC/DCS stripped, OSC passes, WezTerm ships its own ConPTY) to
      `windows-console.md`.

### Checkpoint

- [x] `just test`, `just lint` on `biscuit-terminal` + consumers that consume
      `ImageSupport`; `just cross-check biscuit-terminal --os windows`.

---

## Phase 4: Probe

**Goal:** when the environment cannot decide, one query joins the existing
color round trip; TUIs can opt out; hidden tmux across SSH is detected; native
Windows gets a query backend (§2). *Depends on phase 3. Uses spike S1.*

### Wave 1 (parallel, disjoint files)

- [x] **Structured query engine** (`discovery/tty_query.rs`, `osc_queries/`)
    - Extend (don't fork) the engine: structured per-request results and a
      structured DA1 result; a caller-chosen transport for **both** the request
      and the DA1 sentinel. Preserve existing color/cursor behavior; extending
      graphics probing must not enable other probes in a mux.
    - Write through the controlling terminal, never captured stdout. Keep
      locking, raw-mode restore, bounded reads, deadlines, silence budgets.
      Never probe in a synchronized-output block (mode 2026) or from a TUI loop.
- [x] **Reply parsers (pure)**
    - Kitty: valid one-pixel `a=q` (direct data, fresh nonzero ID, dimensions and
      pixel format matching payload); accept only a complete `OK` for that ID;
      distinguish rejection from silence.
    - DA1: parse structurally; Sixel = parameter `4` in a primary DA reply, not
      the digit anywhere. Prefer correlated Kitty success over Sixel.
    - L1: fragmented, out-of-order, interleaved-with-typeahead, duplicate-ID,
      wrong-ID, error-reply (`ENOTSUPPORTED`) fixtures; native-vs-tunneled DA1
      kept as separate results.
- [x] **Native Windows backend** (`cfg(windows)`, `query.rs` Windows arm today a stub)
    - Same contract: request + DA1, deadline and silence budget, console modes
      restored. Serves colors and graphics (so Windows gets measured
      fg/bg, selecting diagram theme and Sixel background) and the WT DA1 `4`
      signal (either-signal rule with phase 3's version check).
    - Never probed: Windows SSH sessions, legacy conhost (ruling 6).
    - Typeahead: non-reply key events written back with `WriteConsoleInput`.
      L2 on `build-win-native` (windowless ConPTY via `xpty`; see the `os`
      skill; answer the startup DA1 per that skill) types keys during the probe and
      checks they arrive after. If it cannot show them returned: ship
      discard-only (as on Unix), delete the test, record in the implementation log.

### Wave 2 (sequential; prerequisite: Wave 1)

- [x] **Wire into `Terminal::new()`**
    - The Kitty query joins the existing once-per-process OSC 10/11 exchange;
      the DA1 that already ends it answers Sixel. No new probing moment.
    - Probe runs only when environment cannot decide (unknown SSH endpoint;
      Zellij with unknown config). Never for known-compatible direct terminals,
      known-incompatible terminals, CI, redirected stdout, disabled graphics,
      Windows-SSH. Never automatically select Kitty for iTerm2.
    - Cache per process **and** output route (not across transports or
      caller-supplied snapshots); busy-input skip is not a negative; geometry
      is not cached with capability.
    - Replace Zellij's interim rule with the pane query (confidence `Verified`);
      query success does not imply cell-size knowledge; missing geometry or
      placement failure ⇒ distinct reason + fallback.
- [x] **TUI escape hatch** per ruling 8; first construction probes, so docs say
      "construct before any input loop starts".
- [x] **Hidden tmux across SSH**
    - `TERM=tmux-*|screen-*` and no `TMUX` ⇒ also send a *wrapped* Kitty query
      (wrap exactly once, with its own wrapped DA1 ordering sentinel). Correlated
      `OK` ⇒ tmux transport (ordinary placement; forwarded
      `LC_TERMINAL=iTerm2` ⇒ iTerm2 format through passthrough, wrapping from
      phase 5). Unconfirmed ⇒ fallback + hint naming `BISCUIT_GRAPHICS_TRANSPORT=tmux`.
    - `BISCUIT_GRAPHICS_TRANSPORT=tmux` forced: wrap, confidence unverified,
      ordinary placement unless outer terminal identified.
    - The wrapped probe is always implemented; no-reply ⇒ unknown.

### Wave 3 (validation checkpoint)

- [x] L1: parser matrix above; "never queried" list (known incompatible,
      CI, pipe, disabled, Windows SSH); busy-input skip; cache keying.
- [x] L1 (PTY): per `biscuit-terminal` skill notes — answer the DA1 sentinel
      (`ProbeAnswer::every(DA1_QUERY, DA1_REPLY)`) and rebuild `discovery_probe`
      before narrowed runs.
- [x] L2: WezTerm; Windows ConPTY (`xpty`) on `$BUILD_WIN`; Zellij if >= 0.45
      installed (else harness skip).
- [x] Update `docs/discovery/terminal-queries.md` for the intentional graphics
      probe exception, input ownership, and platform limits; update
      `graphics-routes.md`.
- [x] `just cross-check biscuit-terminal --os windows`.

---

## Phase 5: tmux transport

**Goal:** graphics through tmux passthrough with correct outer-terminal
identification (§3). *Depends on phases 3, 4. Uses spike S2.*

### Wave 1 (parallel, disjoint files)

- [x] **tmux facts reader**
    - `TMUX` detection; `tmux -V` (floor 3.3; older/unreadable ⇒ fallback with
      version in the reason); `allow-passthrough` via
      `tmux show-options -Apv -t "$TMUX_PANE" allow-passthrough`.
    - Run read-only commands without shell interpolation, with a deadline.
      Missing executable / inaccessible socket / unsupported option / failure ⇒
      **unknown**, never "passthrough off".
    - Outer-terminal identity from the client attached to the target pane
      (`#{client_termname}`, `#{client_termtype}`, `show-environment`); never an
      arbitrary client or the server's startup env. Disagreeing clients ⇒ a
      format all can handle, or fallback.
- [x] **Wrapper** (pure): `ESC P tmux; <seq with every ESC doubled> ESC \`, per
      Kitty chunk; never wrap twice; mode 2026 never wrapped. L1: ESC doubling,
      nested terminators, empty/large sequences.
- [x] **iTerm2 size budget** (pure): base64 expansion + metadata + ESC doubling +
      tmux envelope against a fixed 1 MiB budget; oversize ⇒ fallback with a
      size-limit reason **before** reserving rows. Never split `File=` into
      multiple OSCs. L1 at boundary (limit-1, limit, limit+1).

### Wave 2 (sequential; prerequisite: Wave 1)

- [x] **Route integration**
    - Replace phase 3's interim tmux `None`: Kitty/Ghostty outside ⇒ Kitty
      (placement upgrade arrives in phase 6; until then ordinary placement);
      WezTerm/unidentified ⇒ Kitty ordinary through passthrough; iTerm2 outside
      ⇒ iTerm2 format through passthrough; passthrough off ⇒ fallback + hint
      `set -g allow-passthrough on` (only when observed off).
    - Native tmux Sixel precedence is wired in phase 7.
    - Reply transport for the probe under tmux uses phase 4's wrapped round trip.
    - Document WezTerm-outside limits (drift on resize/copy mode, may persist
      over other windows).
- [x] **TerminalImage tmux emission**: wrapped, chunked, quiet-mode Kitty with
      nonzero IDs; no extra pixel format on partial failure.

### Wave 3 (validation checkpoint)

- [x] L1: wrapping, budget boundaries, version floor, passthrough
      on/off/unknown, client-disagreement, route-table rows for tmux.
- [x] L2 through `biscuit-test-harness` tmux backend: passthrough on and off
      (capture assertions per `rust-testing`); no window focus.
- [x] Docs: `graphics-routes.md` tmux section incl. known limits (fresh client
      renders blank placeholders — documented, not fixed).

---

## Phase 6: Placeholders

**Goal:** Kitty/Ghostty under tmux get Unicode placeholders so images follow
the pane (§3 Placement). *Depends on phases 2, 5.*

### Wave 1 (parallel)

- [ ] **Placeholder grid encoder** (pure)
    - Virtual placement upload (`U=1`) through passthrough; grid of `U+10EEEE`
      with image ID in the foreground color and row/column combining marks
      (Kitty's diacritic table), printed as ordinary text through tmux.
    - Image IDs per ruling 9; reset fg color after the grid; require Unicode and
      truecolor, otherwise ordinary placement with documented limits.
    - L1 fixtures: 1x1, wide, tall, max rows/cols table bound, ID encoding, fg
      reset at end.
- [ ] **Selection rule**: placeholders only when outer terminal is positively
      identified as Kitty or Ghostty (Zellij rejects `U=1`; WezTerm ignores it).

### Wave 2 (sequential; prerequisite: Wave 1)

- [ ] **Integration** into `TerminalImage` for inline and standalone paths;
      sizing from phase 2 (cell rectangle includes padding).
- [ ] L2 (tmux harness, only if a Kitty/Ghostty backend is available, else
      skip): upload bytes + grid captured.

### Checkpoint

- [ ] `just test`, `just test-l2`, `just lint`; update `graphics-routes.md`.

---

## Phase 7: Sixel

**Goal:** Sixel encoder, detection, and tmux-native Sixel (§5, §3 Sixel).
*Depends on phases 2, 3, 4, 5 (ruling 4).*

### Wave 1 (parallel)

- [ ] **Encoder**
    - Add `icy_sixel` 0.7 (no cargo feature flag); update
      `docs/dependencies.md`.
    - Deterministic settings: bounded palette, no dithering by default, opaque
      output. Sixel has no partial transparency: `TerminalImage` composites
      plain images on the measured background; if unavailable use the light/dark
      theme background and expose that estimate in diagnostics.
    - Preserve terminal mode state (no scrolling or palette private modes left
      changed); follow existing cursor ownership rules; test output at the
      bottom of the viewport.
    - L1: fixed fixture ⇒ golden bytes.
- [ ] **Diagram adapters**: `biscuit-visualized` requests a solid RGB
      background (phase 2 enum) for Sixel routes; cache identity already
      separates it.
- [ ] **Detection completion**: DA1 parameter `4` from the relevant renderer
      (WT: either-signal rule, with phase 4 query); remove ruling-2 interim
      "no encoder" fallback; never auto-select Sixel for the Windows-SSH
      system-conhost path.

### Wave 2 (sequential; prerequisite: Wave 1)

- [ ] **tmux-native Sixel**: *established* only when all three hold: tmux >= 3.4,
      tmux's own unwrapped DA1 contains `4`, and `#{client_termfeatures}` for the
      pane's client contains `sixel` (S2 gives the exact shape). Takes
      precedence over passthrough routes even if Kitty is available; emitted
      unwrapped. Passthrough off + native established ⇒ Sixel; else fallback + hint.
- [ ] **Zellij < 0.45 / Kitty disabled**: positive DA1 Sixel from the pane ⇒
      Sixel, else fallback.

### Wave 3 (validation checkpoint)

- [ ] L1: golden Sixel, mode-state preservation, viewport-bottom, background
      selection/estimate diagnostics, route rows (tmux native, Zellij old, WT).
- [ ] L2: tmux built with Sixel when available on the host (else skip).
- [ ] Docs + `biscuit-terminal` and `biscuit-visualized` skills updated for the
      route model and boundary; READMEs.

---

## Phase 8: Half-blocks

**Goal:** plain images with no pixel route fall back to half-blocks, then alt
text (§6). *Depends on phase 3.*

### Wave 1

- [ ] **Half-block renderer**: `▀` with truecolor fg/bg; degrade to alt text
      (explicit alt text wins over file name) when truecolor or Unicode is
      missing. Sanitize alt text/file names as literal text (no control
      injection; L1 with embedded ESC/OSC/CR). Non-tty stdout ⇒ plain text
      description, never color blocks.
- [ ] **Policy**: honors `BISCUIT_GRAPHICS=none`, `GraphicsMode` ceilings
      (`Off`/`Vector` forbid raster-derived half-blocks), image access policy,
      and render-tree placeholder style; does not bypass explicit policies.
- [ ] **Wiring**: `TerminalImage` and `bt image`, inline and standalone; decode
      only here (limits enforced); route reason unchanged so hints still apply.

### Checkpoint

- [ ] L1: truecolor/256/no-Unicode/pipe matrix; sanitization; policy precedence.
- [ ] L2 (Apple Terminal or any available harness backend; skip cleanly):
      half-block output captured with no escape garbage.
- [ ] Docs: `terminal_image.md` half-block section; remove **planned** markers.

---

## Phase 9: Human verification checklist

**Goal:** write `verification.md` from the spec's checklist. *Depends on 1-8.*

- [ ] **Write `verification.md`** in this folder: one section per row of the
      spec's "Human verification checklist" (iTerm2, Windows SSH server,
      Windows Terminal, mintty, Ghostty/Kitty/WezTerm under tmux, SSH from
      local tmux, Zellij 0.45+, Terminal.app). Each with the exact command,
      what to look for, and a blank result line for terminal + multiplexer
      versions. Use the final command names/flags from the implementation.
- [ ] **Final sweep (not human checks)**
    - [ ] All **planned** doc markers removed or justified.
    - [ ] `os` skill, `biscuit-terminal` and `biscuit-visualized` skills,
          both `dependencies.md` files, and both READMEs reflect the end state.
    - [ ] Comment-drift pass on every touched symbol; note any drift fixed.
    - [ ] Implementation log records: ruling outcomes, spike results, PNG size
          numbers, and whether Windows typeahead re-injection shipped.
    - [ ] Final `just test`, `just test-l2`, `just lint` on all affected packages;
          Windows evidence via `just cross-check`.
    - [ ] Stop at "implementation complete, ready for review" (do not move the
          spec to `_completed`, do not commit unless told).

## Spike results

*(Filled in during Phase 1, Wave 1, 2026-10-21, macOS host.)*

### S1: Zellij pane Kitty query

**Not runnable here.** `zellij` is not installed on this host (`command not
found`). The L2 Zellij tests skip through the harness path, and phase 4 ships
the interim Zellij rule untested live.

### S2: tmux format variables

Host tmux: `tmux 3.8`. Throwaway server (`tmux -L <name>`), client attached
through `script -q /dev/null tmux attach` (no window, no focus).

- With **no client attached**, `display -p` prints `||` (all three empty).
- Attached with `TERM=xterm-256color`:
  `client_termname=xterm-256color`, `client_termtype=` (empty),
  `client_termfeatures=bpaste,ccolour,clipboard,cstyle,focus,RGB,title`.
- Attached with `TERM=wezterm`:
  `client_termname=wezterm`, `client_termtype=` (empty),
  `client_termfeatures=bpaste,focus,RGB,sixel,title`.
- Shapes: `client_termfeatures` is a bare, comma-separated feature list with
  no spaces; `sixel` appears as the bare word `sixel` (here from the default
  `terminal-features[3] wezterm:sixel` entry). `client_termtype` is filled
  from the outer terminal's XTVERSION/DA reply and is **empty** when the
  outer terminal does not answer (a `script` pty never does), so it cannot be
  relied on alone.
- Versions (tmux `CHANGES`): `client_termtype` exists since 2.4;
  `terminal-features` (the source of `client_termfeatures`) arrived in 3.2,
  so `client_termfeatures` needs >= 3.2, below the spec's 3.3 floor.
  `allow-passthrough` defaults to `off`.
