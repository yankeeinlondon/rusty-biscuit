# Review: CodeRenderer Terminal Color Context

## Summary

The spec is directionally sound: widening `CodeRenderer::render_terminal_code` before Darkmatter implements it is the right ordering, and the dependency-boundary reasoning is correct. A renderable-owned terminal code context is also the cleanest way to keep `renderable` independent of `biscuit-terminal` while preventing Darkmatter from re-detecting terminal capabilities.

The design still has a few gaps that could make the Darkmatter tree-rendering migration harder than it needs to be. The biggest issues are exact capability semantics, Darkmatter option parity, and how much code-block metadata the hook will need once it renders real Markdown fences instead of only `YamlBlock`.

## Recommendations

### 1. Make `TerminalCodeContext` the settled API, not an illustrative option

The spec lists the struct approach as "recommended (pending confirmation)", but every later requirement assumes a context boundary. This should be settled before implementation.

Recommended shape:

```rust
pub struct TerminalCodeContext {
    pub width: u32,
    pub color_depth: ColorDepth,
    pub color_mode: ColorMode,
}
```

Add a `new(width, color_depth, color_mode)` constructor and derive `Debug, Clone, Copy, PartialEq, Eq` if the fields are `Copy`. Passing `&TerminalCodeContext` is fine, but if all fields are `Copy`, passing by value is simpler and avoids lifetimes for implementers.

### 2. Define `ColorMode::Unknown` semantics explicitly

`biscuit-terminal` has `ColorMode::Unknown`; Darkmatter's highlighting `ColorMode` only has `Light` and `Dark`. The spec says the hook should expose unknown backgrounds, but it does not say how Darkmatter should resolve `Unknown` when selecting a syntect theme.

Add an explicit rule, for example:

- `renderable::color::ColorMode::Unknown` means "the terminal renderer could not determine the background."
- Code renderers must not run ambient detection to resolve it.
- Darkmatter should map `Unknown` to its configured `TerminalOptions.color_mode`, or to `Dark` if the tree-rendering entry point does not yet carry a configured option.

Without this, the new context can still lead to inconsistent output because the Darkmatter implementation will need to invent a fallback.

### 3. Decide whether `ColorDepth::Minimal` is preserved or collapsed

FR-4 says the map must distinguish at least no color, 16-color, 256-color, and true-color. `biscuit-terminal` also has `Minimal` for 8 colors. If `renderable` uses a coarser enum, `Minimal` will either collapse into "no color" or "16 color"; neither is lossless.

For a shared capability descriptor, prefer mirroring `biscuit-terminal`:

```rust
pub enum ColorDepth {
    None,
    Minimal,
    Basic,
    Enhanced,
    TrueColor,
}
```

Then Darkmatter can consciously map `Minimal` to its current `Colors16` or `None` behavior. That keeps the boundary lossless even if Darkmatter remains coarser internally.

### 4. Do not over-claim current Darkmatter color-depth behavior

The problem statement says `TerminalOptions.color_depth` drives downsampling of 24-bit SGR for 256/16-color terminals. In the current code, code-block and prose paths still emit many `38;2` / `48;2` true-color sequences; `ColorDepth::None` is the only clearly enforced terminal-wide downgrade for code-heavy output.

Suggested correction: state that Darkmatter currently uses `color_depth` for the no-color early return and for building the shared `biscuit-terminal::Terminal`, and that the tree migration should preserve or improve the intended depth behavior. If true downsampling is a goal, add it as a separate acceptance criterion for the Darkmatter implementation, not this precursor.

DECISION: true downsampling IS A GOAL

### 5. Add conversion helpers at the boundary

The affected-code table says to map in `biscuit-terminal/src/discovery/detection/color.rs`, but the spec does not say what API should own that mapping.

Avoid scattering `match` expressions at call sites. Add one of these:

- `impl From<biscuit_terminal::discovery::detection::ColorDepth> for renderable::color::ColorDepth` in `biscuit-terminal`, where the source type is local to the crate doing the impl.
- Local helper functions near the terminal tree renderer, if you want the conversion to stay private.

The first option is more reusable and keeps `render_code_node` focused on rendering.

### 6. Add tests that assert the context is actually passed through

Updating the Phase 3 stub is necessary but not enough. Add focused tests in `biscuit-terminal` that install a stub `CodeRenderer` and assert it receives:

- `available_width`, not root `width`;
- the configured `ColorDepth`;
- the configured `ColorMode`, including `Unknown`;
- no ambient detection after a manually constructed `TerminalRenderContext`.

The width case matters because nested render contexts adjust `available_width`; a future regression could accidentally pass `context.width` and still compile.

### 7. Include Darkmatter mapping tests in the follow-up migration criteria

Even though Darkmatter is out of scope for this precursor, the spec is explicitly a precursor to Darkmatter. Add future-work acceptance notes for the next spec:

- Darkmatter maps `TerminalCodeContext.color_depth` to `TerminalOptions.color_depth` without auto-detection.
- Darkmatter maps `TerminalCodeContext.color_mode` to its `highlighting::ColorMode` using the explicit `Unknown` fallback rule.
- A test forces conflicting ambient env vars and render context values, then verifies code blocks follow the render context.

This is the scenario the spec is trying to protect; capture it now so the next migration does not miss it.

### 8. Preserve code-block metadata beyond color

`CodeRenderHints` currently only carries `header_row`, `language_label`, and `highlight`. Darkmatter Markdown fences support richer info strings through `CodeBlockMeta`: title, line numbering, and line highlighting. The spec does not need to solve that here, but it should call out that color context alone is not enough for full Darkmatter code-block parity.

Recommended addition to future work: design how `NodeKind::Code` preserves the raw info string or structured `CodeBlockMeta` equivalents before claiming tree-rendered Darkmatter code blocks are parity-complete.

### 9. Clarify the no-color behavior expected from `CodeRenderer`

FR-6 says the built-in plain renderer is unaffected, but the hook's behavior under `ColorDepth::None` is unspecified. A custom renderer returning ANSI output under `None` would violate the renderer-wide capability model.

Add a contract to `CodeRenderer` docs:

- Implementers should treat `ColorDepth::None` as "emit no ANSI styling."
- If an implementation cannot honor the capability context, it should return `None` and let the plain renderer handle the block.

This keeps the hook consistent with the rest of terminal tree rendering.

### 10. Be careful with placement under `renderable::color`

Putting terminal capability descriptors in `renderable::color` is reasonable, but the module currently describes itself as color data shared across render targets and says terminal ANSI emission lives in `biscuit-terminal`. Add wording that these are capability descriptors, not terminal rendering APIs.

If the naming feels crowded, `TerminalColorDepth` / `TerminalColorMode` would reduce ambiguity. If you keep plain `ColorDepth` / `ColorMode`, recommend aliases at use sites:

```rust
use renderable::color::{ColorDepth as RenderColorDepth, ColorMode as RenderColorMode};
```

## Suggested Spec Edits Before Implementation

- Move D-4 from "recommended" to "settled".
- Settle OQ-2 in favor of the five-variant enum unless there is a strong reason to intentionally collapse 8-color support.
- Add explicit `Unknown` fallback semantics for Darkmatter.
- Correct the description of current Darkmatter `color_depth` behavior.
- Add context pass-through tests to acceptance criteria.
- Add future-work criteria for Darkmatter code metadata and no ambient detection.

With those changes, the precursor should set up the Darkmatter tree-rendering migration cleanly instead of only fixing the immediate trait-signature problem.
