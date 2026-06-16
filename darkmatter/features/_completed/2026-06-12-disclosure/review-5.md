---
ready: false
agent: codex
model: ""
---

# Review 5

## Findings

### High: Browser disclosure behavior is only verified at Level 1

The spec describes browser disclosures as native `<details>/<summary>` elements with no JavaScript, where the summary is the clickable label and the body is revealed by native browser behavior. The current browser coverage in `darkmatter/lib/tests/disclosure_render_targets.rs` asserts string output only (`browser_target_uses_native_details_summary`, `browser_target_renders_nested_disclosures`). That is Level 1: it proves the renderer emitted HTML-shaped text, but it does not prove a browser parses it into the expected DOM or that clicking the summary toggles the disclosure.

Requirement classification:

- Browser target emits native `<details>/<summary>` and no script: strongest present Level 1.
- Browser user-observable click-to-reveal behavior: strongest present Level 1; appropriate level is Browser tier.

Add a `browser_*` test using the existing Chrome harness: render a disclosure document, assert `details` and `summary` are present, assert `script` count is zero, assert the body text is hidden while `details.open === false`, click the summary, and assert `details.open === true` with the body visible. Include a nested disclosure case if the harness can query both levels without making the test brittle.

### High: `style.disclosure.*` terminal rendering through frontmatter lacks Level 2 coverage

The spec requires `style.disclosure.width`, `max-width`, `alignment`, `color`, and `bg-color` to parse, lower through `ComponentPolicy`, and render visibly where supported. Current tests cover parsing/strict-style at Level 1 (`darkmatter/lib/src/style/parse.rs`), policy lowering at Level 1 (`darkmatter/lib/src/style/apply.rs`), and in-process terminal rendering for frontmatter color/max-width/alignment at Level 1 (`darkmatter/lib/tests/disclosure_render_targets.rs`). The Level 2 terminal tests in `darkmatter/cli/tests/level2_layout.rs` cover the body block quote dim/italic behavior and inline opener `color`/`max-width`, but not the `style.disclosure.*` frontmatter path through the built CLI.

Requirement classification:

- Terminal body dim+italic block quote: Level 2 present.
- Inline opener `color`/`max-width` terminal rendering: Level 2 present.
- Frontmatter `style.disclosure.color`/layout terminal rendering: strongest present Level 1; appropriate level is Level 2 because the requirement includes visible terminal color/layout behavior.

Add a Level 2 test that invokes the just-built `md` binary with frontmatter containing `style.disclosure.color`, `max-width`, and `alignment`, then captures the real terminal pane and verifies the RGB SGR, wrapped quoted body lines, and alignment indentation. If `bg-color` is intended to affect terminal cells for disclosures, include an SGR/background assertion too; otherwise document the target limitation explicitly.

### Medium: Disclosure width/max-width conflict is not directly tested

`apply_disclosure_style` delegates to the shared `apply_common_style` helper, so this likely behaves correctly, but the spec explicitly requires `style.disclosure.width` plus `style.disclosure.max-width` to be rejected with the same conflict behavior as other buckets. I found direct conflict tests for other component buckets, but no disclosure-specific conflict assertion.

Requirement classification:

- Disclosure `width` + `max-width` conflict rejection: strongest present indirect Level 1 through helper reuse; appropriate level is direct Level 1.

Add a focused unit test that builds a `StyleFrontmatter` with both fields under `disclosure`, calls `apply_disclosure_style`, and asserts `StyleApplyError::ComponentWidthConflict { bucket: "disclosure" }`.

### Medium: Page-render error mapping is not asserted for malformed disclosures

The block-extension processor has Level 1 tests for malformed disclosures returning `MarkdownError::MalformedDisclosure`, and `DarkmatterPage` has a separate test proving a malformed code directive maps to `PageRenderError::Render`. The spec specifically requires malformed disclosure failures to map through `PageRenderError::Render`; that exact path is not covered.

Requirement classification:

- Malformed disclosure maps through page rendering as `PageRenderError::Render`: strongest present indirect Level 1; appropriate level is direct Level 1.

Add a `DarkmatterPage::render` or `render_to_browser` test with an authored malformed disclosure and assert the returned error is `PageRenderError::Render` and contains the malformed-disclosure reason.

## Notes

I did not find a clear functionality gap in the core parser, render-tree lowering, CLI output compatibility, JSON document export, or transclusion unification. The remaining blockers are verification-level gaps against the review rubric, not evidence that the implementation is currently broken.

Verification run during this review:

- `cargo test --color=never -p darkmatter disclosure --tests` passed.
- `cargo test --color=never -p darkmatter-cli markdown_plus --test cli` passed.

I did not run `just test-l2`; the rusty-biscuit testing guide says Level 2 tests must run through the shared harness recipe, and this review only needed to inspect whether the right Level 2 cases exist.

## Resolution (2026-06-14)

All four findings were verification-level gaps; each is closed by a new test at
the appropriate level (no production-code behavior change was required — the
implementation was already correct).

- **Finding #1 (browser disclosure only at Level 1) — resolved.** Added two
  browser-tier tests to `darkmatter/lib/tests/browser_render.rs` that drive a
  real headless Chromium:
  `browser_disclosure_click_reveals_body` asserts the rendered HTML parses into
  a native `<details>`/`<summary>` DOM with zero `<script>`, the body is
  not-visible while `details.open === false`, and clicking the summary flips
  `details.open === true` with the body visible;
  `browser_nested_disclosure_toggles_independently` proves the nested
  disclosure stays hidden until the outer opens and then toggles on its own
  summary click. Visibility is read via `Element.checkVisibility()` (a
  bounding-box height check is unreliable here: Chrome lays the closed body out
  at its intrinsic size while visually hiding it via the
  `content-visibility: hidden` `::details-content`). This required a small,
  general `evaluate(script) -> String` method on the shared `BrowserHarness`
  (and its `ChromeHarness` impl), because each `computed_style` call opens a
  fresh page and so could not observe a click; `evaluate` performs the whole
  query→click→re-read interaction in one page. Verified: both tests pass against
  a real Chrome.
- **Finding #2 (`style.disclosure.*` frontmatter lacked Level 2) — resolved.**
  Added `level2_disclosure_honors_frontmatter_style_color_width_alignment` to
  `darkmatter/cli/tests/level2_layout.rs`. It runs the just-built `md`
  (`run_md_built`) on a fixture carrying `style.disclosure.color`, `max-width`,
  and `alignment`, then asserts against the real WezTerm pane: the red-500
  truecolor SGR (semicolon or ITU colon form), the body wrapped into multiple
  `│` quoted lines by `max-width`, and the centered-block left indent. The
  `bg-color` terminal-cell case is documented as a non-assertion in the test
  (the disclosure terminal target renders its body as a dim/italic block quote
  and does not fill component background cells; browser-tier coverage exercises
  component `bg-color`). Verified against the built binary in a WezTerm pane.
- **Finding #3 (disclosure width/max-width conflict untested) — resolved.**
  Added `disclosure_width_and_max_width_together_rejected` to
  `darkmatter/lib/src/style/apply.rs`, alongside a `style_with_disclosure`
  helper. It builds a `StyleFrontmatter` with both `width` and `max-width` under
  `disclosure`, calls `apply_disclosure_style`, and asserts
  `StyleApplyError::ComponentWidthConflict { bucket: "disclosure" }`. Verified.
- **Finding #4 (page-render error mapping untested) — resolved.** Added
  `render_errors_on_malformed_disclosure` to `darkmatter/lib/src/layout/page.rs`.
  A malformed disclosure (empty summary) is rejected during the fold
  (`run_sub_fold` propagates the block-extension error), so unlike a malformed
  code directive it also fails the terminal `render` path; the test asserts the
  returned error is `PageRenderError::Render(_)` whose message carries the
  malformed-disclosure reason ("Malformed disclosure"). Verified.
