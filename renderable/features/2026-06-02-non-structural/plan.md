---
phases: 5
created: 2026-06-02
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - renderable/features/2026-06-02-non-structural/plan.md
docs_created_during_phase_1:
  - renderable/features/2026-06-02-non-structural/phase-1-notes.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2: []
docs_updated_during_phase_2:
  - renderable/docs/components.md
  - renderable/features/2026-06-02-non-structural/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-terminal/lib/tests/level1_apple_terminal_prose.rs
  - darkmatter/lib/tests/snapshots/render_tree_hr_snapshots__terminal_mark_dim_hr.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__condition__eval.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__condition__parse.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__image_ref__malformed_markdown_with_context.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__image_ref__malformed_markdown_without_context.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__link__malformed_markdown_with_context.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__link__malformed_markdown_without_context.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__page_block__condition_delegates.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__page_block__parse_directive.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__page_block__unmatched_end.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__page_block__unterminated_block.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__reference__parse_directive.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__transclusion__condition_eval.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__transclusion__condition_parse.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__transclusion__invalid_frontmatter_assignment.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__transclusion__invalid_reassigned_frontmatter_property.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__transclusion__invalid_reference.snap
  - darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__transclusion__parse_directive.snap
docs_updated_during_phase_3:
  - renderable/features/2026-06-02-non-structural/plan.md
docs_created_during_phase_3:
  - renderable/features/2026-06-02-non-structural/phase-3-notes.md
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - biscuit-terminal/lib/src/render_tree/render.rs
docs_updated_during_phase_4:
  - renderable/features/2026-06-02-non-structural/plan.md
docs_created_during_phase_4:
  - renderable/features/2026-06-02-non-structural/phase-4-notes.md
skills_files_updated_during_phase_4: []
packages:
  - renderable
  - biscuit-terminal
  - darkmatter
---

# Non-Structural Component Exemptions Execution Plan

## Phase 1: Scope and Inventory

- [x] Read `renderable/features/2026-06-02-non-structural/spec.md` and record the success criteria: exemption criterion documented, Exemption Register consumable by the cutover, and node-kind helper verification completed.
- [x] Read the related cutover, graphics-policy, perf-gate, and component-catalog references listed in the spec to identify exactly which files must be updated or verified.
- [x] Inventory all exempt components from the register: `PadLeft`, `PadRight`, `InlineContent`, `Status`, `GraphExpression`, `FileTree`, `HorizontalRule`, `TerminalImage`, `MermaidDiagram`, and `DarkmatterPage`.
- [x] Inventory the node-kind semantics that must stay on-tree: `NodeKind::ThematicBreak`, `NodeKind::Image`, and `NodeKind::Code { lang: "mermaid", .. }`.
- [x] Mechanically search `biscuit-terminal`, `darkmatter`, and `renderable` for each exempt component and each node kind to establish current render paths before edits.
- [x] Validation checkpoint: confirm every off-tree component found by the searches is either listed in the register, already tree-render-only, or explicitly recorded as a blocker needing classification before Phase 5.

## Phase 2: Documentation and Register Integration

- [x] Update the cutover spec's Acceptance Criteria #2 wording to the narrowed criterion: every component the darkmatter document pipeline renders is tree-render-only, while enumerated non-structural components are exempt.
- [x] Update `renderable/docs/components.md` so each exempt component has an IR-state annotation or note that matches the Exemption Register category and justification.
- [x] Add or update any per-area documentation that currently claims all renderable components must migrate to the tree so it instead uses the document-pipeline participation criterion.
- [x] Review touched rustdoc and Markdown comments for drift introduced by the new criterion; fix or remove comments that imply helper components are document nodes.
- [x] Validation checkpoint: documentation readers can answer which components are exempt, why they are exempt, and which document node kinds still require tree renderer ownership.

## Phase 3: Node-Kind Renderer Verification

- [x] Verify `NodeKind::ThematicBreak` renders terminal output through the tree renderer entry points; any `HorizontalRule` usage must be below the node renderer as a helper call.
- [x] Verify `NodeKind::ThematicBreak` renders browser output through the tree renderer entry points; any SVG or rule builder usage must be below the node renderer as a helper call.
- [x] Verify `NodeKind::ThematicBreak` renders Markdown output through the tree renderer entry points and does not route through a legacy document serializer.
- [x] Verify `NodeKind::Image` renders through the tree renderer graphics-policy path; `TerminalImage` may encode a selected terminal tier only after the node renderer chooses the policy.
- [x] Verify `NodeKind::Code { lang: "mermaid", .. }` remains a code node until the Mermaid-aware renderer promotes it according to `MermaidMode` and `GraphicsMode`; `MermaidDiagram` may be called only below that promotion boundary.
- [x] Verify the darkmatter document pipeline does not emit a bare `Status`; `StatusBlock` remains the document-content type and stays on the tree.
- [x] Parallelizable: split the ThematicBreak, Image, Mermaid, and Status verification tasks across separate implementers because they depend only on Phase 1 inventory.
- [x] Validation checkpoint: each helper exemption has an evidence note pointing to the renderer function, test, or mechanical search result that proves document lowering is owned by the tree renderer.

## Phase 4: Remediation and Focused Tests

- [x] Phase 3 found no document path bypassing the tree renderer for `ThematicBreak`; tree renderer already owns the lowering in all three targets. `HorizontalRule` remains a private helper below the node renderer. No remediation needed.
- [x] Phase 3 found no document path dispatching directly to `TerminalImage`; the tree renderer handles `NodeKind::Image` directly (terminal renders alt text as a documented parity gap). No remediation needed.
- [x] Phase 3 found no `MermaidDiagram` call in the tree renderer; `NodeKind::Code { lang: "mermaid" }` correctly stays a code node until the graphics-policy promotion boundary is implemented. No remediation needed.
- [x] Phase 3 found no darkmatter document path emitting bare `Status`; `StatusBlock` is the document-content type and stays on the tree. No remediation needed.
- [x] Existing tests and snapshots already cover all verified tree-renderer paths; no remediated paths required new tests. See `phase-4-notes.md` for the test inventory.
- [x] Fixed a pre-existing clippy lint error in `biscuit-terminal/lib/src/render_tree/render.rs` (struct initialization pattern). No behavior-changing edits required comment or doc updates.
- [x] Validation checkpoint: all remediation tasks explicitly marked not needed with evidence from Phase 3. See `phase-4-notes.md`.

## Phase 5: Cutover Readiness Validation

- [ ] Run mechanical searches confirming no remaining document-pipeline route calls the legacy `RuleProcessor`, `output/html.rs`, `output/terminal.rs`, or exempt helper components directly for `Image`, `ThematicBreak`, or Mermaid code nodes.
- [ ] Run the narrow package tests covering `renderable`, `biscuit-terminal`, and `darkmatter` renderer behavior touched or verified by this work.
- [ ] Run any existing component-catalog or documentation validation commands that cover `renderable/docs/components.md` and the feature specs.
- [ ] Update the tree-cutover Phase 4/5 checklist, if present, with the final exemption-register and node-kind verification status.
- [ ] Record final evidence in the implementation notes or PR summary: changed docs, verified renderer paths, tests run, and any remaining cutover blockers.
- [ ] Validation checkpoint: the cutover can consume this spec without treating the registered non-structural components as blockers, and all document-pipeline components remain either tree-render-only or explicitly tracked for cutover.
