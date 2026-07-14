---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-14T07:39:44-07:00
---

# Review 6 — Style Features

## Verdict

Not ready for production. Review 5's high-severity finding is closed: `render_to_browser` now has
a stable body-only return shape, and the new feature-free Browser-tier embedding test confirms that
it introduces no nested document scaffold. The new standalone counterpart,
`render_to_browser_document`, is not assembled as a real complete document when layout decoration
or a feature forces the wrapper. Its `<head>` is empty while metadata, authored stylesheets,
page-level CSS, and feature assets are emitted in `<body>`. This path is used by the CLI's HTML
artifact builder, so it is a production blocker rather than a cosmetic source-order issue.

## Findings

### High — Decorated and feature-bearing standalone documents lose their real head

`render_to_browser_document` returns Renderable's complete document only for the undecorated,
feature-free branch. On every other branch it calls the body-fragment serializer and wraps the
result in `<!DOCTYPE html><html><head></head><body>…</body></html>`
(`darkmatter/lib/src/layout/page.rs:1069-1092`). `wrap_browser_html` emits page metadata and an
authored inline or remote stylesheet before the wrapper (`page.rs:1774-1828`), then emits the
design-token/panel stylesheet and resolved feature assets inside the wrapper
(`page.rs:1902-1928`). The resulting standalone document therefore:

- drops the ordinary document head's charset, viewport, and title;
- places page-authored `<meta>` and remote `<link rel="stylesheet">` elements in the body;
- places page CSS and Mermaid/Popover assets in the body instead of the complete document's head;
- rejects any future feature carrying a `LinkTag`, because shared part construction always uses
  the body-only `resolve_feature_body_assets` path (`page.rs:1186-1203`) even though a standalone
  document can and must place that dependency in its head.

This contradicts the specification's complete-document asset goal and ordering contract
(`spec.md:33-34,100-103`). It also reaches users through `html_artifact`, which calls
`render_to_browser_document` for CLI HTML output (`darkmatter/cli/src/artifact.rs:50-52`). The
decorated reference snapshots currently freeze the defect; for example,
`cutover_reference__ref_page_margin_and_padding_browser.snap` starts with an empty head followed by
the design-token stylesheet inside the body wrapper.

The split API is the right ergonomic direction, but both outputs need distinct assembly policies.
Keep `render_to_browser`'s forced self-contained wrapper for embedding. Assemble
`render_to_browser_document` from an actual head and a wrapper-only body: retain charset, viewport,
title, page-authored metadata/assets, and resolved feature assets in the head in the specified
order, then place only the page wrapper/content in the body. The standalone path must use normal
feature serialization so `LinkTag` remains legal there; `HeadRequired` belongs only to the
body-only path.

Add Level-1 structural tests for decorated and feature-bearing standalone documents, including
page metadata, a remote stylesheet, Mermaid or Popover, and a synthetic link-bearing feature.
Assert that the head is non-empty and ordered, the body contains only wrapper/content, and the CLI
artifact has the same shape. Add a Browser-tier DOM test that loads this decorated/feature-bearing
standalone output and verifies the metadata/styles/scripts are children of `document.head` while
the wrapper is a child of `document.body`. Current Level-1 coverage exercises only the
feature-free no-wrapper branch (`style_features_phase5.rs:185-210`), and current Browser coverage
embeds a feature-free fragment into a host document (`browser_render.rs:1181-1248`); neither can
detect this defect.

### Medium — Maintained docs still describe the fragment API as the full-page API

The review-5 fix split fragment and document rendering, but several maintained descriptions were
not updated. `darkmatter/docs/rendering/mermaid.md:42,50-57,77-83,101-105` repeatedly calls
`DarkmatterPage::render_to_browser` the "full-page path." The baseline module says a feature-free
call to that method returns a standalone document (`darkmatter/lib/tests/style_features_baseline.rs:7-15,106-108`), although its own test now receives a bare body. The implementation notes retain
the old content-dependent contract and say the same method returns a standalone document when no
wrapper is needed (`implementation-notes.md:444-457`).

Criterion 13 is therefore incomplete. Update these directly affected docs to distinguish
`render_to_browser` (body-only fragment) from `render_to_browser_document` (standalone document),
and state that the CLI uses the latter. The public method rustdoc already makes this distinction;
the maintained guide, implementation record, and characterization-test prose should agree with
it.

## Requirement-to-verification assessment

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| 1. Mermaid asset dedup | Level 1 plus Browser-tier execution with the real vendored Mermaid 11.6.0 engine | Appropriate; one bootstrap initializes both diagrams |
| 2. Markdown neutrality | Level-1 Markdown/MarkdownPlus byte and snapshot tests | Appropriate; passed |
| 3. Interactive browser default | Browser tier with real Mermaid execution, fallback, and total-load-failure probes | Appropriate for the interaction; passed |
| 4. Compatibility defaults | Level-1 mode/default matrix plus Browser static-SVG paths | Appropriate; passed |
| 5. Body-only placement | Level-1 wrapper/ordering tests plus Browser host-document embedding for feature-free and feature-bearing fragments | Appropriate; review 5's gap is closed |
| 6. Divergent request config | Deferred by the specification until a config-bearing feature exists | Not applicable to fieldless v1 |
| 7. Popover behavior | Level 1 markup/style, Browser geometry/focus/navigation, and genuine macOS Level 3 OS keyboard/pointer injection | Appropriate; all exercised paths passed |
| 8. `MermaidHtml` retirement | Source audit and compilation | Appropriate; no live references remain |
| 9. Resolver failures | Level-1 typed error tests for unresolved features and body-only `HeadRequired` | Partial: body-only behavior passes, but the standalone path incorrectly uses the body-only link rejection policy |
| 10. Side-channel preservation | Level-1 map, hook, streaming/fragment parity, order, and resolver-count tests | Appropriate; passed |
| 11. Asset safety and fallback | Level-1 escaping/version assertions plus Browser primary, fallback, and total-failure execution | Appropriate; passed |
| 12. Cross-platform and regression | macOS Level 1, Browser, and Level 3; full-page snapshot covers only the feature-free no-wrapper branch | Gap: decorated/feature-bearing standalone head placement has no Level-1 structural test or Browser DOM test; Windows/Linux were not executed |
| 13. Documentation cleanup | Maintained-doc source audit | Gap: multiple maintained surfaces still call the body fragment API the full-page/standalone path |

## Prior-review closure

- Review 5's body-only contract blocker is closed. `render_to_browser` returns a bare body for an
  undecorated, feature-free page and a forced wrapper fragment for decorated or feature-bearing
  content; its Browser-tier host embedding has no nested document scaffold.
- Review 5's plan drift is closed. Phase 5 now records the script-only Mermaid bundle and the
  `ThemePair`/color-mode palette source accurately.
- Popover input coverage remains rigorous: Level 1 validates bytes, Browser tests validate parsed
  DOM/layout/navigation, and Level 3 validates genuine macOS keyboard and pointer encoding.

## Ergonomics and performance

Separating fragment and standalone APIs removes the content-dependent return type identified in
review 5 and is the correct public design. The shared render pass also avoids reparsing Markdown.
No material performance regression was found. The remaining design correction is to retain typed
head/body parts until each public serializer applies its own placement policy; reconstructing a
document by wrapping an already serialized body fragment loses semantics and makes future
link-bearing features impossible on the standalone path.

## Verification performed

- `just test` from `renderable/`: 529 passed, 16 skipped.
- Feature-focused Darkmatter Level 1: 16 passed across `style_features_baseline` and
  `style_features_phase5`.
- `just test-browser` from `darkmatter/`: 102 passed, 5,566 skipped, including real Mermaid,
  fallback/failure, popover geometry/focus/navigation, and feature-free fragment embedding.
- `just test-l3 --no-fail-fast` from `darkmatter/`: 3 passed, 5,665 skipped in Darkmatter; the CLI
  crate had 0 Level-3 tests run and 623 skipped.
- `cargo nextest run --color=never -p biscuit-test-harness`: 80 passed.
- `just lint` passed for Darkmatter, `darkmatter-cli`, DMLS, and Renderable.
- `git diff --check` passed before and after the review-file edits.
- The production code paths are written without OS-specific behavior; execution was limited to
  the available macOS host, so Windows and Linux remain unexecuted.
- GitNexus concept/context queries were used for orientation. The index does not contain the new
  uncommitted `render_to_browser_document` symbol, so findings were verified directly against the
  current source and tests. No Rust symbol was edited during this review.
