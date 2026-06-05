---
ready: false
agent: codex
model: ""
---

# Review - Iteration 1

## Findings

### High - Phase 5 sign-off treats the legacy public document pipeline as non-active

The non-structural spec's verification condition requires a mechanical search to find no remaining document-pipeline route that calls `RuleProcessor`, `output/html.rs`, `output/terminal.rs`, or the helper components for `Image`, `ThematicBreak`, and Mermaid code nodes after deletion (`renderable/features/2026-06-02-non-structural/spec.md:154`). The implementation marks that condition complete in `phase-5-notes.md:10` and concludes the legacy serializers "are not active document-pipeline routes" at `phase-5-notes.md:44`.

That conclusion does not match the actual public pipeline. `Markdown::as_html` still delegates to `output::as_html` (`darkmatter/lib/src/markdown/mod.rs:595`), `Markdown::as_terminal` still delegates to `output::for_terminal` (`darkmatter/lib/src/markdown/mod.rs:620`), and `as_terminal_with_layout` still delegates to `output::terminal::for_terminal_with_layout` (`darkmatter/lib/src/markdown/mod.rs:626`). `DarkmatterPage::render` then calls `md.as_terminal_with_layout(...)` (`darkmatter/lib/src/layout/page.rs:867`). The tree entry points are still `pub(crate)` and explicitly documented as not touching the public APIs (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:7`).

This means the feature has verified the experimental tree entry points, but not the document pipeline named by the spec. It should not mark the Phase 4/5 checklist complete or set the cutover spec's Decision #5 to "Verification completed" until either:

- the public document render APIs route through the tree entry points and the mechanical search is rerun, or
- the notes and cutover spec are narrowed to say only "tree entry points verified; public document-pipeline cutover remains pending."

Verification level: Level 1/static inspection only. This is adequate for the routing claim, but the inspected route contradicts the implementation's sign-off.

### Medium - Component catalog link to the Exemption Register is broken

`renderable/docs/components.md:79` links "Exemption Register" to `#exemption-register`, but that anchor does not exist in `components.md`; the actual register lives in `renderable/features/2026-06-02-non-structural/spec.md#exemption-register`. This is a documentation feature, and the catalog is one of the main places readers are supposed to discover why exempt components do not block bespoke deletion.

Change the link to point directly at the spec anchor:

```markdown
[Exemption Register](../features/2026-06-02-non-structural/spec.md#exemption-register)
```

Verification level: documentation/static check.

## Test Coverage Notes

The implementation has good Level 1 coverage for the tree renderers named in the spec, and there is an existing Level 2 WezTerm test for the styled HR terminal path (`darkmatter/lib/tests/level2_render_tree_terminal.rs:553`). I did not find a Level 3 requirement in this feature because it does not assert OS keyboard or input-encoder behavior.

The blocking coverage gap is not a missing terminal tier test; it is that the "no remaining document-pipeline route" verification was applied to the internal tree entry points while the public document pipeline still routes through the legacy serializers.

## Readiness

Not ready for production. The exemption register itself is coherent, but the cutover readiness/sign-off is materially overstated until the public document-pipeline routing claim is corrected.
