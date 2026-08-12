---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-13T15:59:33-07:00
---

# Review 4 — Style Features

## Verdict

Not ready for production. The implementation and its Level-1 and Chromium Browser evidence are
otherwise strong: the Darkmatter area test recipe completes successfully, Renderable's Level-1
suite passes, and all 99 Browser-tier tests pass, including real Mermaid 11.6.0 execution,
fallback delivery, popover geometry, focus, hover, and navigation. The production blocker is the
strongest evidence for the user-facing popover interactions: all three genuine macOS Level-3
tests fail on a host their availability gate declares provisioned. CDP success cannot substitute
for failed OS keyboard and pointer injection.

## Findings

### High — All genuine Level-3 popover interactions remain red

`just test-l3 --no-fail-fast` ran the three macOS OS-injection tests serially and all three failed
on every configured retry:

- `level3_popover_tab_focuses_anchor_and_reveals_prompt` reported
  `active=false;vis=hidden`;
- `level3_popover_enter_activates_link` reported an empty `nav=` value; and
- `level3_popover_pointer_hover_reveals_prompt` left the prompt hidden.

The improved harness does call `focus_window_or_diagnose`, and none of these runs stopped at its
`document.hasFocus()` diagnostic (`darkmatter/lib/tests/browser_render.rs:1892-1915`). That proves
the CDP page considered its document focused, but it does not prove the subsequently launched
`cliclick` process delivered an event to that exact Chrome window. Likewise,
`accessibility_trusted()` probes an `osascript` System Events no-op
(`biscuit-test-harness/src/cliclick.rs:38-70`); success establishes that probe's automation path,
not successful delivery of a `cliclick` Quartz keyboard or pointer event. The current host passes
that gate while every injected interaction has no observable effect.

Add an OS-injected canary that the live page must observe before making the product assertion,
and diagnose canary failure as harness provisioning. Verify that activation targets the Chrome
instance/window created by the harness, not merely an application named `Google Chrome`, then fix
the coordinate or keyboard delivery path until the canonical Level-3 recipe is green. The shared
recipe now correctly uses `-j 1`, so global-window concurrency from review 3 is closed. Until a
clean Level-3 run exists, criterion 7's keyboard reachability, ordinary Enter navigation, and
pointer-hover behavior have passing Browser-tier CDP tests but failing evidence at the required
verification level. This is a production blocker under the review rubric.

### Medium — Authoritative and implementation documentation still describes retired behavior

The specification now correctly defines Mermaid as script-only and body-only output as a real
fragment, but several maintained descriptions still contradict production:

- the authoritative Darkmatter skill says the resolver injects shared Mermaid CSS
  (`.claude/skills/darkmatter/terminal.md:166-171`), although the resolver emits no Mermaid CSS;
- Darkmatter's baseline test documentation makes the same one-CSS-plus-one-script claim
  (`darkmatter/lib/tests/style_features_baseline.rs:7-10,51-54`);
- the implementation notes claim `render_to_browser` wraps a complete `<!DOCTYPE html>` document
  inside the page div (`implementation-notes.md:448-456`), while the current implementation and
  Browser DOM test intentionally emit a body fragment with no nested document; and
- the notes still summarize production Mermaid dedup as one CSS block plus one script
  (`implementation-notes.md:519-530`). Renderable's baseline module and assertion messages also
  continue to describe the old pipeline as inert “today”
  (`renderable/tests/style_features_baseline.rs:4-8,85-105`), even though that particular test is
  now only establishing fragment-level non-injection.

Update these statements to distinguish request-only fragment rendering from outer document
resolution, describe Mermaid's `themeVariables` script-only bundle, and describe the actual
body-fragment wrapper. Acceptance criterion 13 and the repository drift rules require maintained
documentation and comments to match shipped behavior.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| 1. Mermaid asset dedup | Level 1 on fragment/streaming and Darkmatter page paths | Appropriate; passed with exactly one module script and no Mermaid CSS |
| 2. Markdown neutrality | Level 1 render and snapshot tests | Appropriate; passed |
| 3. Interactive browser default | Browser tier with the real vendored Mermaid 11.6.0 engine | Appropriate; passed |
| 4. Compatibility defaults | Level 1 mode/default matrix plus Browser static-SVG paths | Appropriate; passed |
| 5. Body-only placement | Level 1 snapshots plus Browser DOM-parent/nesting assertion | Appropriate; passed |
| 6. Divergent request config | Deferred by the specification until a config-bearing feature | Not applicable to fieldless v1 |
| 7. Popover behavior | Browser computed-style/geometry and CDP interaction pass; all genuine macOS Level-3 tests fail | Gap: the strongest appropriate key/pointer verification is red |
| 8. `MermaidHtml` retirement | Source audit and compilation | Appropriate; passed |
| 9. Resolver failures | Level 1 typed error matches in Renderable and Darkmatter | Appropriate; passed |
| 10. Side-channel preservation | Level 1 map/hook/parity/order/cache tests | Appropriate; passed |
| 11. Asset safety and fallback | Level 1 escaping/version tests, Browser failure paths, and real-engine success | Appropriate; passed |
| 12. Cross-platform and regression | macOS Level 1 and Browser suites pass; macOS Level 3 fails; Windows/Linux were not executed | Partial: deterministic paths pass on the available host, but the interaction gate is red |
| 13. Documentation cleanup | Source and documentation audit | Partial: core public guides improved, but the maintained skill, notes, and test descriptions still drift |

## Prior-review closure

- The Level-3 recipe is serialized with `-j 1`, Chrome activation is attempted, document focus is
  checked, and a macOS Accessibility probe was added. The tests still fail after those changes,
  so the user-observable interaction finding remains open.
- Firefox/WebKit automation is no longer a requirement: the specification now limits automated
  verification to Chromium and explicitly accepts the right-edge fallback limitation on engines
  without CSS anchor positioning. Review 3's multi-engine finding is therefore closed by the
  revised supported-engine contract.
- The specification and production resolver now agree that Mermaid is script-only and carries
  its palette through `themeVariables`. Residual maintained-document drift is captured above.
- Browser-harness and popover documentation now correctly classify CDP input as Browser-tier and
  name the genuine OS-input Level-3 tests. Review 3's tier-classification finding is closed.

## Ergonomics and performance

No material API-ergonomics or performance regression was found in the reviewed feature path.
Feature identity remains typed, resolver ownership is explicit, resolution is cached and
deduplicated in first-seen order, and body-only injection avoids a second inner document. The
recommended work is confined to making the Level-3 harness produce trustworthy passing evidence
and correcting documentation drift; no additional feature abstraction is warranted.

## Verification performed

- `just test` from `renderable/`: 529 passed, 16 skipped.
- `just test` from `darkmatter/`: completed successfully across the area packages.
- `just test-browser` from `darkmatter/`: 99 passed, 5,566 skipped.
- `just test-l3` from `darkmatter/`: failed on the first Level-3 interaction after all retries.
- `just test-l3 --no-fail-fast` from `darkmatter/`: 0 passed, 3 failed, 5,662 skipped; every
  genuine OS-input popover test failed on every retry.
- GitNexus exploration used the current `rusty-biscuit` index to trace feature resolution,
  body-only browser rendering, and final page assembly. No Rust symbols were edited during this
  review.
