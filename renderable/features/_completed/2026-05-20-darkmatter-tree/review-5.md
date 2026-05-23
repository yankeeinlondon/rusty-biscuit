---
ready: false
agent: codex
model: ""
---

# Review: Darkmatter Tree Rendering Migration, Iteration 5

## Findings

### High: terminal tree path ignores `TerminalOptions::color_depth`, invalidating no-color behavior and benchmarks

The experimental terminal entry point documents that it maps `TerminalOptions` into `TerminalRenderOptions` and its own comment says pinned `max_width` / `color_depth` should be respected, but the implementation only uses `max_width` when building the terminal context. If `max_width` is set, `terminal_options_from_terminal_options` always creates `Terminal::new_optimistic(...)`, whose color depth is `TrueColor`; if `max_width` is not set, it uses host detection. In neither branch is `opts.color_depth` applied (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:160`).

This has two consequences:

- `render_tree_terminal(md, TerminalOptions { color_depth: Some(ColorDepth::None), ... })` can still emit color/style-capability-dependent output through a TrueColor tree context, unlike the legacy renderer's explicit no-color path.
- The `migration/terminal_no_color` benchmark group does not measure a no-color tree renderer. It pins the legacy side to `ColorDepth::None`, but the tree side still uses `pinned_tree_terminal_options()`, which builds a TrueColor optimistic terminal (`darkmatter/lib/benches/migration_parity.rs:202`, `darkmatter/lib/benches/migration_parity.rs:212`, `darkmatter/lib/benches/migration_parity.rs:302`). The baseline note then treats the reported tree numbers as no-color evidence, but that is not the path being measured.

This violates the entry-point option mapping contract and the spec's DMTR-6 requirement for pinned terminal options and a real `terminal_no_color` group (`renderable/features/2026-05-20-darkmatter-tree/spec.md:165`). Add a shared conversion from darkmatter's `ColorDepth` to `biscuit_terminal::discovery::detection::ColorDepth`, apply it in `terminal_options_from_terminal_options`, and make the benchmark tree helper accept the intended color depth. Add a Level 1 entry-point test that `ColorDepth::None` produces no ANSI color SGRs for styled content, plus a benchmark-shape test or assertion that the no-color tree context is actually `ColorDepth::None`.

Verification level: no Level 1 test currently covers `TerminalOptions::color_depth` on the tree entry point or benchmark tree options. Because color/no-color output is user-observable terminal behavior and benchmark evidence gates public cutover, this is a production-readiness gap.

## Verification-Level Summary

| Requirement | Strongest observed verification | Assessment |
| --- | --- | --- |
| `==mark==` and `⌄dim⌄` reach tree entry points | Level 1 entry-point tests; Level 2 terminal tests for visible dim/mark styling | Adequate for the experimental terminal/browser surfaces. |
| Mixed mark/dim nesting | Level 1 fold tests for dim-inside-mark and mark-inside-dim | Prior gap appears closed. |
| HR attributes affect terminal/browser output | Level 1 renderer tests plus Level 2 terminal glyph assertion | Prior gap appears closed for terminal; browser exposes data attributes rather than full CSS styling, which is consistent with the documented internal-path gap. |
| Mark/dim/HR benchmark corpus uses span-aware fold | Benchmark helper routes `mark_dim_hr` through `fold_markdown_spanned_with_frontmatter` | Prior gap appears closed. |
| `TerminalOptions::color_depth = None` is honored by tree terminal rendering | No matching test; implementation ignores the option | Gap. |
| `migration/terminal_no_color` compares legacy no-color against tree no-color | No matching assertion; implementation uses a TrueColor tree context | Gap. |

## Production Readiness

Not ready for production.

Iteration 5 resolves the prior mixed-nesting, HR-rendering, and span-aware benchmark-path findings, but the terminal option adapter still drops `color_depth`. That means one user-visible terminal option and the no-color benchmark evidence are both testing the wrong behavior.

## Verification Performed

- Read `spec.md`, `span-aware-processor-design.md`, and `entry-point-shape.md`.
- Reviewed `darkmatter::markdown::render_tree::{span, fold, entrypoints}`, `darkmatter/lib/tests/render_tree_parity.rs`, `darkmatter/lib/tests/level2_render_tree_terminal.rs`, `biscuit-terminal` and browser thematic-break rendering, and `darkmatter/lib/benches/migration_parity.rs`.
- Attempted targeted `cargo test` runs for the darkmatter entry-point dim test and biscuit-terminal HR hint test. They were still blocked/compiling after the non-interactive time budget, so I stopped them and do not claim test results.
- The requested `root` skill is unavailable in this session's skill catalog; I used the provided repo instructions and the `renderable` skill.
