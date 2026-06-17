---
ready: true
agent: codex
model: ""
---

# Review 2

> **Resolution (2026-06-02).** Both findings addressed.
>
> - **High — Browser `Vector` allowed interactive Mermaid.** `render_code_block`
>   now caps `Interactive` to `StaticSvg` under `GraphicsMode::Vector`
>   (script-capable presentation exceeds the `Vector` "no scripts" ceiling);
>   only `Rich` reaches the mermaid.js path. The misnamed lock-in test is
>   replaced by `mermaid_interactive_under_vector_degrades_to_static_svg`,
>   which asserts `Vector` degrades to SVG and `Rich` reaches mermaid.js.
> - **High — Static-SVG promotion failure bypassed strictness via the hook.**
>   Mermaid promotion no longer rides the overloaded
>   `CodeRenderer::render_browser_code` `Option`. A new dedicated, fallible
>   `CodeRenderer::render_browser_mermaid` returns `Some(svg)` on success and
>   `None` on failure; the renderer's `StaticSvg` branch routes its `None`
>   through strictness (reject under `Strict`, diagnose + fall back under
>   `Warn`). Darkmatter's hook implements it (returns `None` when
>   `MermaidDiagram::render_to_svg` fails) and its `render_browser_code` no
>   longer special-cases mermaid. Tests:
>   `browser_mermaid_returns_svg_or_none_never_silent_code_block`,
>   `browser_code_does_not_promote_mermaid`, plus the existing
>   `mermaid_static_svg_failure_honors_strictness`.

## Findings

### High -- Browser `GraphicsMode::Vector` still allows interactive Mermaid

The spec defines `GraphicsMode::Vector` as "scalable vector / native-markup
graphics" with "no scripts", and its tier table maps browser Mermaid at
`Vector` to a static inline `<svg>` (`spec.md:108`, `spec.md:131`). The current
browser renderer treats `BrowserMermaidMode::Interactive` as valid for both
`Vector` and `Rich` (`renderable/src/tree/render/browser.rs:717`,
`renderable/src/tree/render/browser.rs:753`), and the test suite explicitly
locks that in with `mermaid_interactive_with_vector_graphics_mode`
(`renderable/src/tree/render/browser.rs:1760`).

That violates the graphics ceiling: `Interactive` is a richer/script-capable
presentation than `Vector` permits. `Vector + Interactive` should either
degrade to `StaticSvg`/plain code according to the chosen policy or be rejected
under `Strict`; only `Rich + Interactive` should be allowed to use the
client-side mermaid.js path.

Requirement verification level: browser user-visible Mermaid output needs L1
policy tests plus browser-tier DOM verification for the SVG/interactive shape.
The strongest current test is L1 and asserts the wrong behavior.

### High -- Browser Mermaid promotion failure still bypasses strictness through the darkmatter hook

The renderer's `StaticSvg` branch assumes any `Some(fragment)` from the
`CodeRenderer` hook means successful SVG promotion
(`renderable/src/tree/render/browser.rs:754`). But darkmatter's hook catches
`MermaidDiagram::render_to_svg()` failures, logs a warning, and then continues
to render a normal highlighted code block, returning `Some(fragment)` anyway
(`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:169`,
`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:178`,
`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:197`). The outer
renderer therefore never sees the failure, cannot emit a render-tree diagnostic,
and cannot reject under `RenderStrictness::Strict`.

The added strictness regression only covers the "no hook installed" case
(`renderable/src/tree/render/browser.rs:1848`), not the production darkmatter
entry-point shape where `MermaidMode::Image` maps to `BrowserMermaidMode::StaticSvg`
and installs `TerminalCodeRenderer` (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:175`).
This leaves the spec's failure contract partially unimplemented.

Recommended fix: make Mermaid SVG promotion report success/failure explicitly
instead of overloading `Option<BrowserFragment>`, or keep Mermaid promotion out
of the generic code-renderer hook and expose a fallible Mermaid-specific hook.
Then add a regression with a hook that fails SVG generation but can still render
normal code, verifying `Strict` rejects and `Warn` records a diagnostic.

Requirement verification level: L1 is appropriate for strictness/failure
policy, but the current L1 coverage misses the production hook behavior.

## Test Rigor Notes

- Terminal HR `Vector` now has both L1 coverage and a Level 2 WezTerm capture
  test, which is the right verification level for terminal-rendered glyph/image
  selection.
- Browser HR SVG and browser Mermaid static SVG remain mostly L1/string-level
  verified. For production readiness, browser-visible SVG requirements should
  also have browser-tier DOM/computed-style checks.
- No Level 3 requirement was identified; this feature does not specify keyboard,
  mouse, paste, IME, or other OS-input behavior.

## Verification Performed

- Read `renderable/features/2026-05-26-graphics-policy/spec.md`, `plan.md`, and
  `review-1.md`.
- Inspected the staged implementation changes in `renderable`, `biscuit-terminal`,
  and `darkmatter`.
- Ran `cargo test -p renderable --lib mermaid_interactive_with_vector_graphics_mode --color=never`: passed, confirming the current Vector/Interactive behavior.
- Ran `cargo test -p renderable --lib mermaid_static_svg_failure_honors_strictness --color=never`: passed for the no-hook case.
- Ran `cargo test -p darkmatter --lib browser_code_mermaid_attempts_svg_then_fallback --color=never`: passed, confirming the darkmatter hook falls back internally.

## Production Readiness

Not ready for production. The terminal-side review-1 issues look substantially
addressed, but browser Mermaid still violates the `GraphicsMode` ceiling and
the production static-SVG hook path does not enforce the requested strictness
contract.
