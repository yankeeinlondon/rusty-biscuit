Terminal Image Support Review (Biscuit vs Darkmatter)

- Context
    - Biscuit Terminal (`biscuit-terminal/lib/src/components/terminal_image.rs`): ships a full Kitty/iTerm renderer that base64-encodes PNGs and emits escape sequences directly. Supports width specs (`fill`/percent/chars), alt text, measured cell sizes, Kitty chunking, and iTerm width params. Capability detection is env-heuristic (`discovery::detection::image_support`), only returns Kitty/None, with a special-case iTerm2 branch when Kitty + app==ITerm2. No size caps, no base-path scoping or traversal guard, no remote fetch (local files only), and no viuer/sixel fallback.
    - Darkmatter (`darkmatter/lib/src/markdown/output/terminal.rs`): viuer-based renderer with Kitty/iTerm detection via viuer helpers, rejects remote URLs, canonicalizes against a base path to block traversal, enforces ~10MB size cap, and falls back to alt text placeholders. Mermaid support lives in `darkmatter/lib/src/mermaid/render_terminal.rs` (mmdc -> PNG -> viuer) with graphics disabled for plain-string returns to preserve stdout ordering.

- Comparison
    - Protocol/rendering: Biscuit Terminal hand-rolls Kitty/iTerm sequences with its own sizing logic; Darkmatter delegates rendering to viuer. Both target Kitty/iTerm, but Darkmatter inherits viuer’s detection/quirks while Biscuit Terminal controls everything (and currently lacks sixel/other fallbacks as well).
    - Capability detection: Biscuit Terminal uses env heuristics and never returns `ITerm`; the iTerm path is only taken when Kitty is reported and `TERM_PROGRAM` says iTerm2. Darkmatter relies on viuer’s runtime detection. Neither reports capability reasons to callers.
    - Security/IO: Darkmatter blocks remote URLs, canonicalizes against a base path, and caps files at ~10MB. Biscuit Terminal loads arbitrary local paths with no size limit or traversal/base-path guard and would inline very large files.
    - Layout/UX: Biscuit Terminal honors width specs and avoids upscaling by default, but does not flush surrounding text or manage spacing beyond cursor advance; Darkmatter flushes before viuer and skips extra spacing after images, reusing alt text for fallbacks.
    - Mermaid: Darkmatter renders mermaid to PNG for terminals; Biscuit Terminal has no mermaid path.

- Suggestions for Biscuit Terminal
    - Add safety rails: canonicalize against a caller-provided base path, reject traversal, and enforce a reasonable size cap before encoding. Consider optional MIME/extension allowlist.
    - Improve capability detection: use viuer’s `get_kitty_support`/`is_iterm_supported` or OSC queries to confirm support instead of env-only heuristics; surface `ImageSupport::ITerm` when applicable and report why images are skipped.
    - Remote policy: either explicitly block remote URLs (with a clear error) or add guarded fetch with size/type limits and cache dir.
    - Output hygiene: flush surrounding buffered text before emitting escapes and provide a configurable cursor advance/spacing policy to match Darkmatter’s UX.
    - Mermaid parity: optionally add a mermaid-to-PNG path (could reuse Darkmatter’s pipeline or resvg) so CLI output keeps diagrams aligned with markdown rendering elsewhere.

- Suggestions for Darkmatter
    - Remote images: optionally support guarded HTTP fetch with size/type caps and caching; currently all remote URLs fall back to text even when graphics are allowed.
    - Protocol breadth/telemetry: add knobs to force kitty/iterm/sixel (via viuer config) and surface skip reasons/capability state to callers instead of only debug logs.
    - Layout controls: expose max width/height ratios or per-image overrides; the current `min(term_width-4, 60)` clamp can underuse wide terminals.
    - Mermaid pipeline: apply the same size/base-path checks as inline images and consider reusing a shared renderer so viuer config and logging are centralized.
    - Tests: target decision points (tty vs non-tty, unsupported terminal, oversize files, mermaid failure) to lock in fallbacks and spacing.

- Integration ideas (Darkmatter leveraging Biscuit Terminal)
    - Treat `biscuit-terminal` as the shared dependency: extract its image rendering/core detection into a reusable module (or slim crate) with base-path/size guards so Darkmatter can drop its bespoke renderer.
    - Shared config surface: define a `TerminalImageOptions` (render toggle, base path, max size, remote policy, scaling, spacing) that both crates honor for consistent CLI flags and docs.
    - Mermaid reuse: host the mermaid-to-PNG flow alongside the shared renderer so Biscuit Terminal gains diagrams and Darkmatter reuses the same viuer/escape config.
    - Test harness: provide a mockable viuer adapter + fixtures to assert identical spacing/fallback output across both crates without real terminals.
