---
$schema: "@.claudine/schemas/review.yaml"
ready: true
implemented: false
agent: codex/default
created: 2026-07-14T09:42:16-07:00
---

# Review 7 — Style Features

## Verdict

Ready for production. Review 6's blocker is closed: decorated and feature-bearing standalone
renders now assemble a real, ordered `<head>` and a wrapper-only `<body>`. The CLI HTML artifact
uses that path, and a Browser-tier live-DOM test verifies that metadata, Popover CSS, and the
Mermaid module script are children of `document.head` while rendered content remains under
`document.body`.

Every active v1 user-observable requirement has evidence at the appropriate level. In particular,
the browser behavior is not inferred from source snapshots: real headless Chromium verifies
standalone placement, Mermaid execution/fallback, Popover focus/navigation, computed styling, and
viewport geometry. The existing Level-3 macOS tests remain the strongest evidence for real OS
keyboard/pointer delivery to Popover interactions; that unchanged path was not rerun in this
iteration.

## Findings

No production-blocking finding was identified.

### Medium — Standalone `<link>` legality is still not covered at the Darkmatter integration boundary

Review 6 requested a synthetic link-bearing feature test for the standalone path. The production
fix is structurally correct: `render_to_browser_document` serializes resolved features with
`serialize_features_head` (`darkmatter/lib/src/layout/page.rs:1100-1107`), Renderable proves that
head serialization emits `<link>` before CSS and script
(`renderable/src/browser/feature.rs:472-485`), and Darkmatter proves that the same synthetic
feature raises `HeadRequired` on the body-only path
(`darkmatter/lib/src/layout/page/tests.rs:3042-3093`). However, no Darkmatter test injects that
synthetic resolver through standalone document assembly and asserts that its `<link>` reaches the
real head rather than being rejected or dropped.

This does not block v1 because Mermaid and Popover use only inline assets; no shipped v1 feature
produces a `LinkTag`. Before the first link-bearing feature ships, add a focused Darkmatter
assembly test so the cross-layer contract is protected rather than relying on source composition
of two separately tested units.

### Low — Body-fragment documentation overstates the single-root shape

The implementation notes say every decorated `render_to_browser` result is a forced
single-element `.darkmatter-page` wrapper (`implementation-notes.md:452-458`), and the helper
comment says page metadata and authored stylesheets live inside that wrapper
(`darkmatter/lib/src/layout/page.rs:1794-1808,1819-1823`). The code appends those elements before
opening the wrapper, so a page with `page.meta` or `page.stylesheet` has sibling `<meta>`/`<style>`/
`<link>` nodes followed by the wrapper. The code is authoritative and the output remains a
body-only fragment, but the single-element claim is inaccurate. Correct the comments and
implementation notes, or deliberately move valid fragment assets inside the wrapper if a
single-root contract is intended.

### Low — The body-parts helper performs avoidable full-document work

`render_browser_document_body` always assembles a complete document and then renders the inner
head again (`renderable/src/tree/render/browser.rs:353-371`). Darkmatter's body-only API discards
that complete document, while decorated/feature-bearing standalone renders discard it and build a
second document around the returned parts. This creates an unnecessary allocation proportional to
the rendered body and duplicates head assembly. It is not a correctness regression and no
material slowdown was observed, but the longer-term API should return streamed body/head parts and
assemble a complete document only for the caller that requests one.

## Requirement-to-verification assessment

| Requirement | Strongest verification | Assessment |
|---|---|---|
| 1. Mermaid asset dedup | Level 1 plus Browser-tier execution with the real vendored Mermaid engine | Appropriate; one module bootstrap initializes both diagrams and no Mermaid CSS is emitted |
| 2. Markdown neutrality | Level-1 Markdown/MarkdownPlus byte and snapshot tests | Appropriate |
| 3. Interactive browser default | Browser tier with live Mermaid execution and fallback/failure probes | Appropriate |
| 4. Compatibility defaults | Level-1 mode/default matrix plus Browser static-SVG paths | Appropriate |
| 5. Body-only placement | Level 1 plus Browser-tier host-document embedding for feature-free and feature-bearing fragments | Appropriate |
| 6. Divergent request config | Deferred by the specification until a config-bearing feature exists | Not applicable to fieldless v1 |
| 7. Popover behavior | Level 1 markup/style, Browser focus/navigation/geometry, and existing genuine macOS Level 3 OS input | Appropriate |
| 8. `MermaidHtml` retirement | Source audit and compilation | Appropriate; no live references remain |
| 9. Resolver failures | Level-1 typed unresolved/`HeadRequired` tests | Appropriate for active v1; dormant standalone link integration gap recorded above |
| 10. Side-channel preservation | Level-1 map, hook, streaming/fragment parity, ordering, and resolver-count tests | Appropriate |
| 11. Asset safety and fallback | Level-1 escaping/version assertions plus Browser primary/fallback/total-failure execution | Appropriate |
| 12. Cross-platform and regression | macOS Level 1 and Browser; portable production paths; unchanged Level-3 macOS evidence | Appropriate on the available host; Windows/Linux were not executed |
| 13. Documentation cleanup | Maintained-doc and retired-symbol source audit | Acceptance criterion satisfied; one non-blocking body-fragment wording issue remains |

## Prior-review closure

- Review 6's high-severity standalone-head defect is closed. Level 1 verifies ordered charset,
  design tokens, page metadata, remote stylesheet, and feature assets; the Browser tier verifies
  their actual DOM parents.
- The CLI artifact regression is closed: the decorated `md --output html` path now produces a
  non-empty head and wrapper-only body.
- Review 6's maintained Mermaid guide and characterization prose drift is closed; both browser
  APIs and the CLI ownership split are now named accurately.
- The requested synthetic standalone link-bearing integration test was not added; it is retained
  as the medium, pre-first-link-feature action above.

## Ergonomics and performance

The split between `render_to_browser` (body fragment) and `render_to_browser_document`
(standalone document) is now coherent and content-independent. Typed resolved assets survive
until the output-specific placement policy, which is the right long-term boundary: body-only
renders reject head dependencies and standalone renders accept them. No OS-specific production
logic was introduced. The only performance opportunity found is the duplicate document/head
assembly noted above.

## Verification performed

- `just test` from `renderable/`: 529 passed, 16 higher-tier tests skipped.
- Focused Darkmatter Level 1 (`style_features_baseline`, `style_features_phase5`, and
  `cutover_reference`): 24 passed.
- Focused CLI artifact regression: 1 passed, 7 unselected tests skipped.
- `just test-browser` from `darkmatter/`: 103 passed, 5,568 non-Browser tests skipped.
- `just lint` passed for Renderable, Darkmatter, `darkmatter-cli`, and DMLS.
- `git diff --check` passed; the retired `MermaidHtml` / `render_for_html` symbol audit found no
  live code, maintained-doc, or Darkmatter-skill references.
- GitNexus reported low risk for the current implementation diff and no affected indexed
  execution process. Its index does not model the newly added `render_to_browser_document`
  symbol, so the standalone path was also reviewed directly in current source and tests.
- Execution was limited to the available macOS host. The changed production paths contain no
  OS-specific APIs, commands, path handling, or line-ending assumptions.
