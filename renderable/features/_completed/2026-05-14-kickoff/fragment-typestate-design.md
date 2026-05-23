# BrowserFragment Typestate Design

Models `BrowserFragment` as a typestate machine so the compiler enforces the
construction workflow: choose a node shape, refine it, finalize it, then
render.

## States

Five marker types implement a sealed `FragmentState` trait:

- `Shape` — initial state. `node` is `None`. Only the `define_as_*` builders
  are exposed.
- `RefineVoid` — a void tag has been chosen. Attributes can be added; children
  cannot.
- `RefineBlock` — a block tag has been chosen. Attributes and children can be
  added.
- `RefineText` — a text fragment has been chosen. No attributes, no children.
  This state exists (rather than jumping straight to `Ready`) so the shared
  cross-cutting builders apply uniformly to every kind of fragment.
- `Ready` — fragment is fully composed and renderable.

A `Refine` sub-trait, also sealed, is implemented for `RefineVoid` /
`RefineBlock` / `RefineText` only, and carries the shared cross-cutting
builders.

```text
Shape ──define_as_block_tag──▶ RefineBlock ─┐
      ──define_as_void_tag──▶ RefineVoid ───┼── finalize() ──▶ Ready
      ──define_as_text_fragment──▶ RefineText ─┘
```

## Struct shape

```rust
pub struct BrowserFragment<S: FragmentState = Shape> {
    node: Option<ComposableNode>,
    stylesheet: Option<ComponentStylesheet>,
    features: Vec<PageFeature>,
    metadata: HashMap<MicrodataKey, String>,
    dependency_links: Vec<LinkTag>,
    _state: PhantomData<S>,
}
```

- All fields private. External access goes through methods so the typestate
  invariants cannot be circumvented.
- `node` is `Option<ComposableNode>` rather than `Option<HtmlNode>`. The
  top-level node of a fragment is always one of `BlockTag` / `VoidTag` /
  `TextFragment`, but `ComposableNode` is the natural carrier because
  block-tag children also live as `ComposableNode` (allowing nested
  `Component(BrowserRenderable)` children).
- Default type parameter `S = Shape` so `BrowserFragment::new()` works
  without a turbofish.

## Method layout

| Impl block | Methods |
|------------|---------|
| `BrowserFragment<Shape>` | `new`, `define_as_block_tag`, `define_as_void_tag`, `define_as_text_fragment` |
| `impl<S: Refine> BrowserFragment<S>` | `with_stylesheet`, `add_feature`, `add_metadata_keypair`, `add_linked_dependency` |
| `BrowserFragment<RefineVoid>` | `add_attribute`, `finalize` |
| `BrowserFragment<RefineBlock>` | `add_attribute`, `add_child(ComposableNode)`, `finalize` |
| `BrowserFragment<RefineText>` | `finalize` |
| `BrowserFragment<Ready>` | `render`, `validate_render_content` |

State transitions consume `self` and return a new `BrowserFragment<NewState>`;
in-state builders take `mut self` and return `Self` (fluent style).

## `finalize()` is infallible

```rust
pub fn finalize(self) -> BrowserFragment<Ready>;
```

`finalize` always succeeds. Validation is a separate concern handled by
`validate_render_content()` on `Ready` so callers who don't need validation
aren't forced to handle a `Result`.

## Children: `ComposableNode`

`add_child` on `RefineBlock` accepts `ComposableNode`, which includes the
`Component(BrowserRenderable)` variant. This is the recursion point that
lets components compose other components — the whole point of the
composition proposal.

`html/tag/mod.rs` exposes `HtmlBlockTag` and `HtmlVoidTag` as `pub(crate)`
(currently private) so the `define_as_*` builders can construct them
without leaking their internals outside the crate.

## Out of scope

- Body of `render()` and `validate_render_content()` — separate work.
- `BrowserRenderable` trait definition — its existence is assumed by
  `ComposableNode::Component`, but spec lives elsewhere.
- Page-level aggregation (stylesheet rollup, feature dedup, etc.) — handled
  by the page layer, not the fragment.
