---
feature: "@renderable/features/2026-05-20-darkmatter-tree"
prompt: |-
    Before we kickoff the Darkmatter implementation of the new [tree-rendering](@renderable/docs/tree-rendering.md)
    architecture we should decide how _fold diagnostics_ and _render diagnostics_ are returned together.
    
    We need to ensure that the first implementation does not not blur:
    
    - parse/fold issue vs renderer lowering issue vs accepted target loss
    - Deliverable: a small result type or convention, even if internal.
last_updated: 2026-05-20
---


The tree pipeline has two phases that each produce non-fatal
[`Diagnostic`]s:

1. **Fold diagnostics** — raised by [`fold_markdown_to_document`] during
   event-stream → [`Document`] construction. Cover unsupported parser
   variants, structural irregularities (unclosed containers), and lossy
   conversions at the tree level.

2. **Render diagnostics** — raised by [`render_*_document`] while lowering
   [`NodeKind`] variants to target-specific output. Cover unsupported node
   types for a given target, lossy conversions (e.g. color flattened to
   grayscale), and validation findings.

These two streams must not be merged into a single `Vec<Diagnostic>`. A
merged stream loses the origin: a [`DiagnosticKind::Lossy`] from the fold
means something fundamentally different from one at the render boundary —
the former is a limitation of the tree model; the latter is a limitation
of the render target.

### Convention

**The fold keeps its own diagnostics.** [`fold_markdown_to_document`]
continues to return `(Document, Vec<Diagnostic>)`. This is not changing.

**The render keeps its own diagnostics.** [`render_*_document`] continues
to return `Result<Rendered<T>, RenderError>`, where [`Rendered<T>`]
carries render-phase diagnostics. This is not changing.

**A composed pipeline returns them side by side.** When darkmatter
migrates its public render paths (`as_html`, `for_terminal`) to the tree
pipeline, the composed entry point returns a type that carries both
streams:

```rust
/// Result of the full fold-then-render pipeline.
///
/// Internal to `darkmatter` for the first implementation. Relocating to
/// `renderable` is a minor move if other crates later need the same
/// composition.
pub struct PipelineResult<T> {
    /// The rendered output.
    pub output: T,
    /// Non-fatal diagnostics from the fold phase (parse/structural).
    pub fold_diagnostics: Vec<Diagnostic>,
    /// Non-fatal diagnostics from the render phase (lowering/target-loss).
    pub render_diagnostics: Vec<Diagnostic>,
}
```

No `Diagnostic` is ever silently discarded. Callers who need a flat view
can `chain()` the two vectors; callers who need origin-aware filtering
(e.g. "only fail on fold errors") can do so without guessing.

Fatal render errors are not diagnostics and must stay in the function's error
channel:

```rust
pub type PipelineRenderResult<T> = Result<PipelineResult<T>, RenderError>;
```

If Darkmatter needs to wrap `RenderError` in `MarkdownError` for a legacy public
API, the wrapper must preserve the render-phase origin in its message or source
chain. Do not convert a fatal `RenderError::InvalidTree` into an in-band output
string.

`PipelineResult` lives in `darkmatter`, **not** in `renderable`, because
`Rendered<T>` is shared with component rendering (Flow B) where there is
no fold. Adding a `fold_diagnostics` field to `Rendered<T>` would couple
the render phase to the fold phase and pollute every component caller with
an always-empty field.

### `DiagnosticKind` meaning per phase

| `DiagnosticKind` | Fold phase                                                                  | Render phase                                                                              |
|------------------|-----------------------------------------------------------------------------|-------------------------------------------------------------------------------------------|
| `Unsupported`    | Parser variant with no `NodeKind` mapping (e.g. math, definition lists)     | `NodeKind` variant with no target representation (e.g. `FootnoteReference` in plain text) |
| `Lossy`          | Content approximated during tree construction (e.g. GFM alert kind dropped) | Target cannot faithfully represent a tree feature (e.g. color depth, layout width)        |
| `Structural`     | Malformed event stream (unclosed container, stray end event)                | Never produced by renderers — structural issues are caught at validation                  |
| `Validation`     | Not currently produced by the fold                                          | Tree validation findings before rendering (e.g. empty `Root`, orphaned `TableCell`)       |

### Why this is sufficient for the first implementation

- **No new `DiagnosticKind` variants needed.** The existing four variants
    already classify the nature of each diagnostic. Phase is conveyed by
    which `Vec<Diagnostic>` the diagnostic lives in, not by a tag on the
    diagnostic itself.

- **No changes to `Rendered<T>` or `Diagnostic`.** The convention is
    purely additive — a new wrapper type in `darkmatter` that composes two
    existing things.

- **The strictness boundary stays clear.** `RenderStrictness` governs the
    render phase only. Fold diagnostics are non-fatal by default, but the
    composed Darkmatter entry point may choose a policy that rejects selected
    fold diagnostics before rendering. If a caller wants to treat fold
    `Unsupported` as fatal, it checks `fold_diagnostics` before calling render
    and returns a Darkmatter error that says the rejection happened before
    target lowering. This keeps the existing `RenderError` variants
    (`Unsupported`, `LossyRejected`, `InvalidTree`) exclusively render-phase
    concerns.

- **Future extension is additive.** If a later phase needs its own
    diagnostic stream (e.g. a compose/tree-rewrite pass between fold and
    render), it adds a field to `PipelineResult` without touching the
    existing two. If the streams eventually need structured filtering,
    `PipelineResult` gains methods — the fields themselves do not change.

### Public legacy API policy

The first implementation is internal, so it can return `PipelineResult<T>`
directly. When a public legacy method is eventually backed by the tree path,
its existing return type still matters:

- `Markdown::as_html` returns `MarkdownResult<String>`.
- `for_terminal` returns `Result<String, MarkdownError>`.
- CLI commands expect rendered output on stdout and diagnostics/errors through
  the established Darkmatter error path.

For those public paths, non-fatal diagnostics should be logged with enough
phase context to debug parity issues:

```text
phase = "fold" | "render"
target = "browser" | "terminal" | "markdown" | "markdown-plus"
kind = DiagnosticKind
message = ...
```

The parity harness and experimental APIs can expose full vectors. Public stable
APIs should not grow diagnostic-bearing return types until that is an explicit
API change.

### Parity ledger policy

Accepted differences should record phase, target, fixture, and reason. A
render-only lossy diagnostic and a fold-time lossy diagnostic must not share a
single anonymous "known drift" row. The minimum ledger key is:

```text
(fixture, target, phase, diagnostic-kind-or-output-facet)
```
