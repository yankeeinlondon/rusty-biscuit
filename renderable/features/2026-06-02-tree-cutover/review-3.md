---
ready: false
agent: codex
model: ""
---

# Review 3 — Tree Cutover

## Verdict

Not ready for production.

Iteration 3 closes the review-2 production-path blockers: `Markdown::as_html`
now renders through the tree, malformed browser code directives are restored to
fatal `InvalidLineRange` errors, structured link attributes survive the
tree-backed browser path, and decorated `DarkmatterPage::render` now reaches
`render_tree_terminal_with_layout`. The remaining blocker is the final cutover
contract itself: the bespoke serializers and `RuleProcessor` are still present,
exported, and directly callable even though the spec resolved this as a hard
flip-and-delete with no deprecation window.

## Findings

### High — Phase 5 deletion gate is still not satisfied

The spec's cutover sequence does not end at flipping the public convenience
entry points. Phase 4 requires confirming no caller constructs a legacy renderer
or `RuleProcessor`, and Phase 5 requires deleting `output/html.rs`,
`output/terminal.rs`, `RuleProcessor`, and the dead comparison support once the
acceptance criteria hold (`renderable/features/2026-06-02-tree-cutover/spec.md`,
Phases 4-5). Decision #8 explicitly chose a hard flip-and-delete with no runtime
flag or deprecation window.

Current code still exposes the legacy browser and terminal serializers:

- `darkmatter/lib/src/markdown/output/mod.rs:27` keeps `pub mod html`.
- `darkmatter/lib/src/markdown/output/mod.rs:29` keeps `pub mod terminal`.
- `darkmatter/lib/src/markdown/output/mod.rs:33` re-exports
  `output::as_html`.
- `darkmatter/lib/src/markdown/output/mod.rs:35`-`37` re-export
  `for_terminal` / `write_terminal`.
- `darkmatter/lib/src/markdown/output/html.rs:153` keeps the legacy
  event-stream `as_html`, and it still constructs `RuleProcessor` at
  `darkmatter/lib/src/markdown/output/html.rs:195`.
- `darkmatter/lib/src/markdown/output/terminal.rs:857` keeps the legacy
  `for_terminal`, which delegates to the legacy serializer.
- `darkmatter/lib/src/markdown/block/mod.rs:13` publicly re-exports
  `RuleProcessor`, and `darkmatter/lib/src/markdown/block/rule_processor.rs:275`
  keeps the public iterator type.

This means external or in-crate callers can still choose the renderer this
feature is supposed to retire. The main `Markdown::as_html`,
`Markdown::as_terminal`, and decorated `DarkmatterPage::render` paths are on the
tree now (`darkmatter/lib/src/markdown/mod.rs:604`,
`darkmatter/lib/src/markdown/mod.rs:660`-`663`, and
`darkmatter/lib/src/layout/page.rs:871`-`875`), but the hard deletion gate is
still open. Until the legacy serializers are removed, or the spec is amended to
define a supported legacy compatibility surface, the implementation is not
production-ready under this spec.

Verification level: Level 1 is sufficient to prove removal/export behavior
(compile/API surface plus mechanical search). The current state fails that
verification because the modules, exports, and constructors remain.

### Medium — Documentation still describes stale legacy reachability

There is comment drift after the decorated terminal branch was flipped to the
tree:

- `darkmatter/lib/src/markdown/output/terminal.rs:839`-`842` says
  `for_terminal_with_layout` remains production-reachable for decorated
  `DarkmatterPage` layouts pending cutover.
- `darkmatter/lib/src/markdown/render_tree/mod.rs:37`-`38` says the legacy
  `InlineStyleProcessor` and `RuleProcessor` still back the public renderers.

The code now routes the decorated layout branch through
`render_tree_terminal_with_layout`, so these comments are stale. Per repo
convention, assume the code is correct and remove or rewrite the legacy claims
as part of the deletion cleanup.

Verification level: Level 1 inspection is sufficient for documentation/API
drift.

## Requirement Status

| Requirement | Status | Strongest relevant verification |
|---|---:|---|
| `Markdown::as_html` on tree | Implemented | Level 1 API tests for malformed directives and structured link metadata; browser/computed-style coverage for stylesheet behavior |
| default `Markdown::as_terminal` on tree | Implemented | Level 2 public-entry real-terminal coverage |
| decorated `DarkmatterPage::render` on tree | Implemented | Level 1 direct entry-point tests plus Level 2 CLI/page-layout captures for component layout, hyperlink, image, and list behavior |
| no browser functional/fidelity regressions found in review 2 | Addressed | Level 1 for string/error contracts; browser test coverage for computed code-block background |
| `YamlBlock` tree render only | Implemented | Level 1 unit/parity coverage |
| delete bespoke darkmatter serializers / `RuleProcessor` | Not implemented | Mechanical Level 1 search shows public exports and constructors remain |

## Notes

I did not re-run the full test suite during this review. The findings above are
from code inspection of the current implementation, prior review dispositions,
and the present test inventory. No Level 3 coverage is required for this feature
because the spec does not assert OS keyboard or mouse input behavior.
