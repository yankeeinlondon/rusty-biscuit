---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-13T13:03:45-07:00
---

# Review 2 — Style Features

## Verdict

Not ready for production. The implementation now closes the prior functional defects around
body-only HTML, direct `HtmlPage` injection, Mermaid theme propagation, document-wide popover
IDs, and typed Darkmatter errors. The remaining blockers are verification defects: the popover
keyboard/pointer tests are labeled Level 3 but inject CDP events rather than OS events, and the
interactive Mermaid browser suite replaces Mermaid 11.6.0 with a handwritten stub. Those tests
cannot establish the two central user-facing behaviors at the rigor required by the specification.

## Findings

### High — The popover interaction tests are Browser-tier CDP tests, not Level 3

`ChromeHarness::drive` sends `Input.dispatchKeyEvent` and `Input.dispatchMouseEvent` directly
through Chrome DevTools Protocol (`biscuit-browser-harness/src/lib.rs:446-516`). The four tests in
`darkmatter/lib/tests/browser_render.rs:1625-1776` are nevertheless named `level3_*`, gated with
`require_level!(Level::L3, ...)`, and documented as Level 3.

That classification conflicts with both the review rubric and the repository's `rust-testing`
contract: Level 3 requires OS keyboard/mouse injection into a real window. CDP dispatch is useful
Browser-tier evidence because it exercises Chromium focus traversal and default actions, but it
bypasses the operating system's input path. `RUN_LEVEL3=1` changes only the gate, not the event
source. The review run also observed one leak-handle retry in
`level3_popover_tab_reaches_anchor`, although the retry passed.

Rename and run the CDP probes as `browser_*`, then add genuine Level-3 tests that launch a visible
browser window and inject Tab, Shift+Tab, Enter, and pointer movement through the host OS
(`cliclick` on macOS, `xdotool` on Linux, and an equivalent Windows input harness). Until that
exists, criterion 7's keyboard, navigation, and pointer behavior has the wrong strongest test
level and cannot be considered production-ready.

### High — The live Mermaid tests never execute the pinned Mermaid library

The browser fixture maps both CDN specifiers to local `data:` modules
(`darkmatter/lib/tests/browser_render.rs:1920-1949`). Its successful module is a handwritten object
whose `initialize` records the supplied config and whose `run` manufactures an SVG
(`darkmatter/lib/tests/browser_render.rs:1812-1837`). Consequently,
`browser_mermaid_interactive_renders_svg_in_live_dom` proves that the bootstrap can call the API
shape the test itself defines; it does not prove that Mermaid 11.6.0 exports that shape, accepts
the selected `themeVariables`, parses the emitted diagram source, or produces a correctly themed
SVG.

Keep the stub tests for deterministic primary/fallback/total-failure behavior, but add one
network-free Browser-tier integration using the exact pinned Mermaid 11.6.0 module served locally.
Assert that a representative diagram becomes a real Mermaid SVG and that computed fill, stroke,
and text colors differ under the resolved light/dark palettes. Without the real dependency in the
success path, the goal that interactive Mermaid is functional and criterion 3 remain
insufficiently verified.

### Medium — The fragment document path resolves every feature twice

`render_browser_document` installs the resolver and immediately calls
`page.resolved_feature_head()` (`renderable/src/tree/render/browser.rs:219-226`), then returns the
page. Final `HtmlPage::render` calls `resolved_feature_head()` again
(`renderable/src/html/mod.rs:343-356`). A custom resolver may use interior state, perform expensive
work, or return a different result on its second invocation; the trait does not require purity.
This also contradicts the architecture's final-assembler, resolve-once model.

Resolve only at final rendering, or cache the first resolved asset bundle on `HtmlPage` and render
that exact result. Add a counting/stateful resolver test proving one resolution per deduplicated
feature per final page.

### Medium — `DarkmatterPage::render_to_browser` rustdoc still states a false equivalence

The public rustdoc says default layout output equals
`md.as_html(HtmlOptions::default())` with no wrapper
(`darkmatter/lib/src/layout/page.rs:976-977`). Feature-bearing content is an intentional exception:
Mermaid defaults to Interactive on this path and prompted links/Mermaid force the body wrapper so
their assets can be embedded. The `## Errors` section also omits the now-typed
`PageRenderError::FeatureResolution` case.

Qualify the equivalence as feature-free content only, document the interactive Mermaid/body-only
feature behavior at this API, and list unresolved/`HeadRequired` feature failures. This is comment
drift on the behavior-changing public symbol and leaves criterion 13 incomplete.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| 1. Mermaid asset dedup | Level 1 assertions plus Browser-tier live-DOM test with two stub-rendered diagrams | Appropriate for dedup/order; passed |
| 2. Markdown neutrality | Level 1 render and snapshot tests | Appropriate; passed |
| 3. Interactive browser default | Browser tier, but Mermaid is replaced by a handwritten stub | Gap: the actual pinned integration is not executed |
| 4. Compatibility defaults | Level 1 mode/default tests and Browser-tier static-SVG tests | Appropriate; passed |
| 5. Body-only placement | Level 1 full-fragment snapshots plus Browser-tier DOM-parent/nesting assertion | Appropriate; passed |
| 6. Divergent request config | Deferred by the specification until the first config-bearing feature | Not applicable to fieldless v1 |
| 7. Popover | Level 1 markup/ID tests, Browser-tier computed style/geometry, and CDP input mislabeled Level 3 | Gap: OS-level Level 3 is absent; Chromium is the only verified engine |
| 8. `MermaidHtml` retirement | Source/reference audit and compilation | Appropriate; passed |
| 9. Resolver failures | Level 1 typed matches in Renderable and Darkmatter | Appropriate; passed |
| 10. Side-channel preservation | Level 1 map/hook/parity/order tests | Appropriate; passed |
| 11. Asset safety/failure fallback | Level 1 escaping/version checks plus Browser-tier stubbed import failures | Appropriate for bootstrap fallback; actual Mermaid integration remains the criterion-3 gap |
| 12. Cross-platform/regression | macOS Level 1 and Browser gates; source paths are platform-neutral | Passed on the available host; Windows/Linux were not executed in this review |
| 13. Documentation cleanup | Source/reference audit and updated browser guides | Gap: public `render_to_browser` rustdoc is stale/incomplete |

## Prior-review closure

- Body-only rendering now embeds a real body fragment with no nested document scaffold, and a
  real-browser DOM test verifies wrapper placement.
- Public `HtmlPage::render` is fallible and injects direct, nested, custom-resolver, and unresolved
  feature requests.
- Mermaid palettes now reach `mermaid.initialize` through `theme: 'base'` and
  `themeVariables`; the unused CSS-variable approach is gone.
- Popover IDs are allocated while the final `HtmlPage` renders, so separately produced fragments
  receive document-unique associations.
- `PageRenderError::FeatureResolution` now retains the typed `FeatureResolveError`.

## Verification performed

- `just test` from `renderable/`: 528 passed, 16 skipped.
- Focused Darkmatter feature binaries: 14 passed, 0 skipped.
- `just test-browser` from `darkmatter/`: 94 passed, 5,567 skipped.
- `RUN_LEVEL3=1 just test-l3`: the four Darkmatter-library CDP tests passed, with one leak-handle
  retry; the area recipe was stopped when it moved on to a lengthy `darkmatter-cli` rebuild. These
  four passes are Browser-tier evidence, not OS-injection Level 3 evidence.
- `git diff --check`: passed.
- GitNexus exploration was run against the current worktree index for the streaming assembler,
  body-only page path, and prompted-link path; no source symbols were edited in this review.
