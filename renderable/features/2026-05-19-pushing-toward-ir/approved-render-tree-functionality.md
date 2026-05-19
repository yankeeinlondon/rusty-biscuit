# Approved Render Tree Functionality

## RT-COMPOSE-001: Explicit no-separator sequence rendering

**APPROVED**

This feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: Compose's public contract is ordered concatenation with no automatic
separators. The current `NodeKind::Root` rendering contract is ordered block
rendering with blank-line separators in Terminal and Markdown. Treating Compose
as a plain root would change observable output for basic inputs like `["foo",
"bar"]`. The render tree needs an explicit sequence/fragment join policy so
components can preserve target-agnostic structural children without inheriting
document-block spacing.

Required behavior:

- Add a typed render-tree representation for sequence joining. This may be a
  dedicated node kind or a typed `NodeAttrs` hint on `Root`; prefer the smallest
  change that keeps exhaustive renderer handling explicit.
- Support at least `SequenceJoin::None`, meaning render children in order with
  no renderer-inserted separator.
- Terminal, Markdown, MarkdownPlus, and Browser renderers must honor the same
  child order and no-separator semantics.
- Normal document `Root` behavior must remain unchanged unless the sequence
  marker is present.
- Validation must reject sequence semantics in structurally invalid positions
  if the chosen representation can appear outside a block/container context.
- Tests must cover root/document behavior unchanged, Compose-style no-separator
  behavior, nested sequences, and mixed inline/block children.
