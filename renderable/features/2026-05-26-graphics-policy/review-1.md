---
ready: false
agent: codex
model: ""
---

# Review 1

## Findings

### High -- Terminal `Vector` still rasterizes horizontal rules

The spec's tier table requires terminal HR output to use Unicode / ASCII at both `Off` and `Vector`, and to rasterize only at `Rich`. The tree terminal renderer currently groups `Vector` with `Rich` and calls `render_image_tier` whenever the terminal advertises image support (`biscuit-terminal/lib/src/render_tree/render.rs:421`). That means callers asking for vector-only / no-raster output can still get Kitty/iTerm image escapes.

Requirement verification level: this has Level 1 coverage for `Off` and `Rich`, but no test for `Vector`; the current tests would not catch this. Because the user-observable behavior is terminal glyph/image rendering, production readiness also needs Level 2 capture in a real terminal or multiplexer proving `Vector` produces text and no image payload.

Recommended fix: make terminal HR `Vector` use `render_text_tier`, add an L1 regression for `Vector` on a Kitty-capable fake terminal, and add an L2 capture case for the real-terminal output.

### High -- Terminal Mermaid promotion ignores the required Mermaid opt-in

The spec says `GraphicsMode` is a ceiling, not an opt-in: Mermaid fences must remain code unless legacy `MermaidMode::Image` enables promotion. The terminal tree renderer promotes any `lang="mermaid"` code block whenever `graphics_mode == Rich` (`biscuit-terminal/lib/src/render_tree/render.rs:1154`), and `darkmatter` maps the default `TerminalImageMode::Auto` to `GraphicsMode::Rich` without carrying `MermaidMode` into the terminal context (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:216`). This changes the public default from "Mermaid remains code" to "try to rasterize Mermaid."

Requirement verification level: current strongest coverage is Level 1 and does not assert the default no-promotion contract. Terminal Mermaid image output is user-observable terminal rendering, so a ready implementation needs L1 policy tests plus Level 2 real-terminal capture for the promoted image/text fallback behavior.

Recommended fix: add a terminal Mermaid promotion mode to the render-tree options/context or install the Mermaid-promoting renderer only when the darkmatter entry point sees `MermaidMode::Image`. Then test default `Off`/`Text` as code and `Image + Rich` as promotion.

### High -- Browser `GraphicsMode::Off` can still promote Mermaid through the code-renderer hook

`render_code_block` suppresses Mermaid promotion under `GraphicsMode::Off`, but then falls through to the generic `code_renderer` hook (`renderable/src/tree/render/browser.rs:731`, `renderable/src/tree/render/browser.rs:751`). Darkmatter always installs `TerminalCodeRenderer`, whose browser hook unconditionally attempts Mermaid SVG rendering for `lang="mermaid"` (`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:169`). So `GraphicsMode::Off + code_renderer` can still emit SVG, violating the spec's "Off degradation is lossless code block" requirement.

Requirement verification level: existing L1 tests cover `Off` without a Mermaid-aware code renderer, but not the darkmatter entry-point shape that actually installs one. Browser SVG output should have L1 policy coverage and browser-tier DOM/computed-style coverage for the styled SVG path.

Recommended fix: under `GraphicsMode::Off`, return the plain code block before consulting any hook for Mermaid, or pass the effective graphics policy into the hook and require the hook to honor it. Add a regression using a renderer that would otherwise return SVG.

### High -- Darkmatter maps browser `MermaidMode::Image` to interactive Mermaid, not static SVG

The spec says browser Mermaid at `Vector`/`Rich` should emit a pre-rendered static `<svg>` when promotion is enabled; interactive mermaid.js is an orthogonal browser opt-in and default off. The darkmatter mapping sends legacy `MermaidMode::Image` to `BrowserMermaidMode::Interactive` (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:166`), which bypasses the static SVG renderer and emits `<pre class="mermaid">` without solving the asset/loader story.

Requirement verification level: current strongest verification is Level 1, and it asserts the incorrect mapping. Browser user-visible output should be verified with L1 policy tests plus browser-tier DOM checks that promoted output is an actual SVG for the static path.

Recommended fix: map legacy `MermaidMode::Image` to `BrowserMermaidMode::StaticSvg`, keep `Interactive` reachable only through an explicit browser-specific option, and update the test that currently expects `Interactive`.

### High -- `TerminalImageMode::Force` is mapped but not enforced

The spec defines `force_graphics` as the capability override for `TerminalImageMode::Force`: attempt image protocol output even when detection says unsupported. The implementation sets `context.force_graphics = true` (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:230`), but the render paths continue to pass the unmodified detected `Terminal` into `render_image_tier` and Mermaid rendering (`biscuit-terminal/lib/src/render_tree/render.rs:427`, `biscuit-terminal/lib/src/render_tree/render.rs:1156`). The `force_graphics` field is not consumed by those decisions.

Requirement verification level: current strongest verification is Level 1 mapping-only. The requirement is user-observable terminal image behavior, so it needs L1 policy tests and Level 2 capture for real terminal output when forcing graphics.

Recommended fix: apply `force_graphics` at the renderer boundary by constructing an effective terminal/capability snapshot or by passing an explicit force flag into graphical render helpers. Add tests that fail with unsupported capability unless force is set.

### High -- Rich terminal images are still a TODO

The spec requires `TerminalImage` / image nodes to choose alt text at `Off`/`Vector` and image protocol output at `Rich`. The tree terminal image branch still returns alt text in every tier and records `"inline terminal images not yet implemented in tree renderer"` at `Rich` (`biscuit-terminal/lib/src/render_tree/render.rs:762`). The plan marks this phase complete, but the implementation does not provide the required Rich-tier behavior.

Requirement verification level: current strongest verification is none for Rich inline image output on the tree path. This is terminal graphical rendering and needs L1 policy coverage plus Level 2 real-terminal capture for the image protocol rendering/fallback.

Recommended fix: either wire the `TerminalImage` renderer with enough URL/path metadata to render at `Rich`, or mark this portion incomplete in the plan and keep the feature not ready until it lands.

### Medium -- Promotion failures do not honor the strictness model

The spec says failed Mermaid promotion records a diagnostic and falls back under `Warn` / `Lossy`, but escalates under `Strict`. Terminal Mermaid failure always pushes a lossy diagnostic and then falls back (`biscuit-terminal/lib/src/render_tree/render.rs:1158`); the function returns `String`, so it cannot reject under strictness. Browser static-SVG fallback similarly returns plain code when the hook returns `None` without surfacing a diagnostic through the render result (`renderable/src/tree/render/browser.rs:736`).

Requirement verification level: L1 is appropriate for strictness policy behavior. Current coverage does not exercise strict Mermaid promotion failure.

Recommended fix: make promotion paths return `Result` or otherwise route through the existing lossy/strict helper, and add strict-mode tests that force promotion failure.

## Test Rigor Notes

- Terminal HR text/image selection, terminal Mermaid image promotion, `TerminalImageMode::Force`, and Rich terminal images are user-observable terminal rendering requirements. Current coverage is primarily Level 1 using manufactured `Terminal` capability snapshots; that is not enough to mark the feature ready.
- Browser HR SVG and browser Mermaid static SVG are user-observable browser output. The implementation has L1 HTML string tests, but no browser-tier DOM/computed-style verification for the restored SVG behavior.
- No Level 3 requirement was identified in this spec because it does not define keyboard/mouse/IME behavior.

## Verification Performed

- Read `renderable/features/2026-05-26-graphics-policy/spec.md` and `plan.md`.
- Inspected the touched `renderable`, `biscuit-terminal`, and `darkmatter` implementation paths.
- Ran `cargo test -p renderable --lib --color=never`: passed.
- Ran `cargo test -p biscuit-terminal --lib render_tree_thematic_break --color=never`: passed.
- Ran `cargo test -p darkmatter --lib browser_options_mapping_maps_mermaid --color=never`: passed.

## Production Readiness

Not ready for production. The implementation compiles in the focused checks, but several high-severity policy requirements are either implemented with the wrong behavior or not implemented at all, and real-terminal/browser verification is below the level required for the user-observable graphics behavior.
