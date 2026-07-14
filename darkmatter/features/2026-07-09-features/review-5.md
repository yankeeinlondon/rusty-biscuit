---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-13T17:54:15-07:00
---

# Review 5 — Style Features

## Verdict

Not ready for production. The review-4 blockers are closed: all three genuine macOS Level-3
popover tests now pass through `cliclick` after a live keyboard/pointer canary and exact-window
PID resolution, and the documentation named by review 4 now describes the script-only Mermaid
bundle and real body-fragment wrapper accurately. The remaining production blocker is the public
browser-output contract: `DarkmatterPage::render_to_browser` returns a complete HTML document for
feature-free content but a body fragment when content requests a feature. That content-dependent
root shape contradicts the specification's body-only ruling and is unsafe for embedders.

## Findings

### High — The body-only API returns a full document on its bare-body path

The specification says `DarkmatterPage::render_to_browser` returns a body-only fragment and may
return a bare body when no decoration is configured (`spec.md:124-130`). The implementation instead
returns `rendered.output.document` when layout is undecorated and no feature is requested
(`darkmatter/lib/src/layout/page.rs:1102-1115`). The feature-focused snapshots explicitly pin that
result as `<!DOCTYPE html><html>...` (`darkmatter/lib/tests/style_features_phase5.rs:57-79`), while
the same method returns a wrapper fragment as soon as a Mermaid fence or prompted link is present.

This makes the public API's document-vs-fragment shape depend on Markdown content. An embedder can
receive a complete document for an ordinary page, then receive a single embeddable `<div>` after a
prompted link is added. It also leaves the feature-free embedding case from the specification
unimplemented; the Browser-tier DOM test verifies only the feature-bearing wrapper.

Make the API contract content-independent. Under the current specification,
`render_to_browser` should return `body` for an undecorated feature-free page and the wrapper
fragment for decorated/feature-bearing pages; the standalone-document snapshot belongs on
`Markdown::as_html` or a dedicated document-rendering method. Add a Level-1 byte-shape test and a
Browser-tier test that embeds the feature-free result in a host document and verifies there is no
nested document scaffold. If the complete-document behavior is actually intended, revise the
specification and split document and fragment rendering into distinct APIs instead of retaining a
content-dependent return shape.

### Medium — The checked implementation plan still claims a Mermaid CSS bundle

The active plan's completed Phase-5 items say Darkmatter moved Mermaid browser CSS into the
resolver and that two Mermaid blocks inject one CSS block plus one module script
(`plan.md:171-179`). Production and the specification are intentionally script-only: the palette
is passed through `themeVariables`, `DarkmatterFeatureResolver` sets `css: None`, and the current
tests assert that no Mermaid CSS is emitted. The same plan also says the resolver consumes
`FeatureContext` semantic colors, while the page supplies an empty `semantic_colors` vector and
the resolver derives its palette from the captured `ThemePair`.

Update the completed plan to record the implementation that actually shipped. Either remove the
semantic-color claim as superseded by the `ThemePair` design, or wire that context deliberately;
do not leave a checked architectural promise describing a path production never uses.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| 1. Mermaid asset dedup | Level 1 plus Browser-tier live DOM with the real vendored Mermaid 11.6.0 engine | Appropriate; one module script, no Mermaid CSS, and both diagrams render |
| 2. Markdown neutrality | Level-1 Markdown/MarkdownPlus render and snapshot tests | Appropriate; passed |
| 3. Interactive browser default | Browser tier with real Mermaid execution | Appropriate; passed |
| 4. Compatibility defaults | Level-1 mode/default matrix plus Browser static-SVG paths | Appropriate; passed |
| 5. Body-only placement | Level 1 and Browser tier for feature-bearing wrappers; only a Level-1 snapshot of a full document for feature-free output | Gap: the narrow feature-bearing case passes, but the specified bare-body contract is unimplemented and lacks an embedding test |
| 6. Divergent request config | Deferred by the specification until a config-bearing feature exists | Not applicable to fieldless v1 |
| 7. Popover behavior | Level 1 markup/style, Browser CDP geometry/input, and genuine macOS Level 3 OS input | Appropriate; all levels passed, including Tab, Enter, and pointer hover |
| 8. `MermaidHtml` retirement | Source audit and compilation | Appropriate; no live references remain |
| 9. Resolver failures | Level-1 typed error matches in Renderable and Darkmatter | Appropriate; passed |
| 10. Side-channel preservation | Level-1 map, hook, parity, order, and resolver-count tests | Appropriate; passed |
| 11. Asset safety and fallback | Level-1 escaping/version tests plus Browser success/fallback/total-failure paths | Appropriate; passed |
| 12. Cross-platform and regression | macOS Level 1, Browser, and Level 3; portable production paths and cfg-gated macOS tests | Appropriate on the available host; Windows/Linux were not executed |
| 13. Documentation cleanup | Source/reference audit and maintained-doc review | Partial: review-4 targets are fixed, but the checked Phase-5 plan still describes retired Mermaid CSS and unused semantic-color wiring |

## Prior-review closure

- Review 4's Level-3 blocker is closed. `just test-l3 --no-fail-fast` ran serially and all three
  OS-injection tests passed after observing their live canaries and targeting the nonce-titled
  Chrome window's actual Accessibility PID.
- Review 4's named documentation drift is closed in the Darkmatter skill, implementation notes,
  and both baseline test modules. They now distinguish fragment request collection from outer
  resolution, describe Mermaid as script-only, and describe the wrapper as a real body fragment.
- The new Level-3 suite is correctly isolated in `level3_popover.rs`, outside the Browser-tier
  binary, and remains gated by `RUN_LEVEL3=1` through the canonical recipe.

## Ergonomics and performance

Feature resolution remains typed, first-seen ordering is deterministic, and resolver execution is
memoized/deduplicated appropriately. No performance regression was found. The main ergonomic
problem is the content-dependent return shape of `render_to_browser`; separating document and
fragment APIs would make the ownership boundary explicit and prevent embedders from inspecting
generated HTML to determine what they received.

## Verification performed

- `just test` from `renderable/`: 529 passed, 16 skipped.
- Feature-focused Darkmatter Level 1: 14 passed across `style_features_baseline` and
  `style_features_phase5`.
- `just test-browser` from `darkmatter/`: 99 passed, 5,566 skipped.
- `just test-l3 --no-fail-fast` from `darkmatter/`: 3 passed, 5,662 skipped; the CLI crate had no
  Level-3 tests.
- `cargo nextest run -p biscuit-test-harness`: 80 passed.
- `just lint` passed for Darkmatter, `darkmatter-cli`, DMLS, and Renderable;
  `cargo clippy -p biscuit-test-harness --all-targets -- -D warnings` passed.
- The full Darkmatter Level-1 area run was stopped at the non-interactive subprocess time limit
  after 2,709 passes and no failures; the bounded feature suites above completed independently.
- `git diff --check` passed before the review-file edits.
- GitNexus refresh/query attempts were abandoned after the non-interactive timeout; no finding
  relies on stale graph data, and no Rust symbol was edited during this review.
