---
phases: 5
created: 2026-06-02
start_phase: 1
---

# Non-Structural Component Exemptions Execution Plan

## Phase 1: Scope and Inventory

- [ ] Read `renderable/features/2026-06-02-non-structural/spec.md` and record the success criteria: exemption criterion documented, Exemption Register consumable by the cutover, and node-kind helper verification completed.
- [ ] Read the related cutover, graphics-policy, perf-gate, and component-catalog references listed in the spec to identify exactly which files must be updated or verified.
- [ ] Inventory all exempt components from the register: `PadLeft`, `PadRight`, `InlineContent`, `Status`, `GraphExpression`, `FileTree`, `HorizontalRule`, `TerminalImage`, `MermaidDiagram`, and `DarkmatterPage`.
- [ ] Inventory the node-kind semantics that must stay on-tree: `NodeKind::ThematicBreak`, `NodeKind::Image`, and `NodeKind::Code { lang: "mermaid", .. }`.
- [ ] Mechanically search `biscuit-terminal`, `darkmatter`, and `renderable` for each exempt component and each node kind to establish current render paths before edits.
- [ ] Validation checkpoint: confirm every off-tree component found by the searches is either listed in the register, already tree-render-only, or explicitly recorded as a blocker needing classification before Phase 5.

## Phase 2: Documentation and Register Integration

- [ ] Update the cutover spec's Acceptance Criteria #2 wording to the narrowed criterion: every component the darkmatter document pipeline renders is tree-render-only, while enumerated non-structural components are exempt.
- [ ] Update `renderable/docs/components.md` so each exempt component has an IR-state annotation or note that matches the Exemption Register category and justification.
- [ ] Add or update any per-area documentation that currently claims all renderable components must migrate to the tree so it instead uses the document-pipeline participation criterion.
- [ ] Review touched rustdoc and Markdown comments for drift introduced by the new criterion; fix or remove comments that imply helper components are document nodes.
- [ ] Validation checkpoint: documentation readers can answer which components are exempt, why they are exempt, and which document node kinds still require tree renderer ownership.

## Phase 3: Node-Kind Renderer Verification

- [ ] Verify `NodeKind::ThematicBreak` renders terminal output through the tree renderer entry points; any `HorizontalRule` usage must be below the node renderer as a helper call.
- [ ] Verify `NodeKind::ThematicBreak` renders browser output through the tree renderer entry points; any SVG or rule builder usage must be below the node renderer as a helper call.
- [ ] Verify `NodeKind::ThematicBreak` renders Markdown output through the tree renderer entry points and does not route through a legacy document serializer.
- [ ] Verify `NodeKind::Image` renders through the tree renderer graphics-policy path; `TerminalImage` may encode a selected terminal tier only after the node renderer chooses the policy.
- [ ] Verify `NodeKind::Code { lang: "mermaid", .. }` remains a code node until the Mermaid-aware renderer promotes it according to `MermaidMode` and `GraphicsMode`; `MermaidDiagram` may be called only below that promotion boundary.
- [ ] Verify the darkmatter document pipeline does not emit a bare `Status`; `StatusBlock` remains the document-content type and stays on the tree.
- [ ] Parallelizable: split the ThematicBreak, Image, Mermaid, and Status verification tasks across separate implementers because they depend only on Phase 1 inventory.
- [ ] Validation checkpoint: each helper exemption has an evidence note pointing to the renderer function, test, or mechanical search result that proves document lowering is owned by the tree renderer.

## Phase 4: Remediation and Focused Tests

- [ ] If Phase 3 finds a document path that bypasses the tree renderer for ThematicBreak, reroute it through `render_*_node` or `render_*_document` while keeping `HorizontalRule` as an internal helper only.
- [ ] If Phase 3 finds a document path that dispatches directly to `TerminalImage`, reroute it through the image node graphics-policy renderer before terminal-tier encoding.
- [ ] If Phase 3 finds Mermaid document rendering routed directly to `MermaidDiagram`, move that call beneath the code-node promotion boundary.
- [ ] If Phase 3 finds bare `Status` emitted from a darkmatter document path, replace it with the appropriate tree-rendered document component or classify the path as non-document UI.
- [ ] Add focused tests or snapshots for any remediated renderer path so the legacy direct route cannot regress silently.
- [ ] Update nearby comments and docs for any changed behavior, assuming the code path is authoritative if comments and code disagree.
- [ ] Validation checkpoint: all remediation tasks are either completed with tests or explicitly marked not needed with evidence from Phase 3.

## Phase 5: Cutover Readiness Validation

- [ ] Run mechanical searches confirming no remaining document-pipeline route calls the legacy `RuleProcessor`, `output/html.rs`, `output/terminal.rs`, or exempt helper components directly for `Image`, `ThematicBreak`, or Mermaid code nodes.
- [ ] Run the narrow package tests covering `renderable`, `biscuit-terminal`, and `darkmatter` renderer behavior touched or verified by this work.
- [ ] Run any existing component-catalog or documentation validation commands that cover `renderable/docs/components.md` and the feature specs.
- [ ] Update the tree-cutover Phase 4/5 checklist, if present, with the final exemption-register and node-kind verification status.
- [ ] Record final evidence in the implementation notes or PR summary: changed docs, verified renderer paths, tests run, and any remaining cutover blockers.
- [ ] Validation checkpoint: the cutover can consume this spec without treating the registered non-structural components as blockers, and all document-pipeline components remain either tree-render-only or explicitly tracked for cutover.
