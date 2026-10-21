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
human_review: false
message_to_agent: |-
    Phase 1 (component move) is done; context for phase 2:

    - The diagram components now live in `biscuit-visualized/src/src/components/`
      (`mermaid.rs`, `graph_expression.rs`, `git_graph.rs`), behind the new
      `terminal` feature. `biscuit-visualized` is now edition 2024. biscuit-terminal
      no longer depends on biscuit-visualized; never add it back.
    - The render-tree hook is `biscuit_terminal::render_tree::DiagramRenderer`;
      the real implementation is `biscuit_visualized::components::TerminalDiagramRenderer`
      (`components/render_hook.rs`). It currently calls
      `MermaidDiagram::new(source).try_render(terminal)` and ignores
      `DiagramRenderRequest::available_width` (behavior-preserving). Phase 2's
      "render at final display width" should start using that width there.
      `DiagramRenderRequest` is `#[non_exhaustive]`, so adding a field (e.g. the
      route in phase 3) is not a breaking change.
    - Diagram tests: `biscuit-visualized` has an `l1` integration binary
      (`tests/l1/`, `required-features = ["terminal"]`, `autotests = false`).
      `mermaid_parity.rs` / `graph_expression_parity.rs` include
      biscuit-terminal's `tests/l1/parity_helpers.rs`, and `layout_matrix.rs`
      includes biscuit-terminal's `tests/layout_matrix_support/mod.rs`, both via
      cross-package `#[path]` (declared in `source-inputs`). The 114 diagram
      layout-matrix insta snapshots are in `biscuit-visualized/src/tests/l1/snapshots/`;
      if phase 2 changes diagram sizing they may need review there.
    - Run biscuit-visualized tests with `just test` in `biscuit-visualized/`
      (all features). Windows cross-check for it needs `--all-features`:
      `just cross-check biscuit-visualized --os windows --all-features`.
    - `bt` installs no diagram hook (no render-tree path of its own promotes
      Mermaid); darkmatter installs it in
      `markdown/render_tree/entrypoints.rs::terminal_options_from_terminal_options`.
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
