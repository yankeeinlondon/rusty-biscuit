---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-13T14:55:14-07:00
---

# Review 3 — Style Features

## Verdict

Not ready for production. The prior Mermaid integration, resolve-once, and public-rustdoc findings
are closed: the Browser tier now executes the vendored Mermaid 11.6.0 engine successfully, a
stateful resolver is called once per deduplicated feature, and `render_to_browser` documents its
feature-bearing behavior and typed failures. The remaining production blockers are both
user-observable. The genuine Level-3 popover tests fail on the available macOS host even when run
one at a time, and the specification's required Firefox/WebKit validation is still absent for a
fallback layout that can overflow at the right viewport edge.

## Findings

### High — The genuine Level-3 popover suite is present but fails

The new tests do cross the OS boundary: they launch headed Chrome and use `cliclick` for Tab,
Return, and pointer movement (`darkmatter/lib/tests/browser_render.rs:1800-2038`). That corrects
review 2's classification problem in the test code, but it does not provide passing Level-3
evidence. On the available macOS host, `just test-l3` failed every popover probe across its
configured retries: Tab left `active=false;vis=hidden`, Return left `nav=` empty, and pointer
movement left the prompt hidden. Running
`level3_popover_tab_focuses_anchor_and_reveals_prompt` alone with `-j 1` also failed all four
attempts, so concurrency is not the only cause.

The harness currently assumes that a coordinate click makes the newly launched Chrome window the
focused OS window. It neither activates/asserts the intended Chrome window nor checks Accessibility
authority before claiming the Level-3 resource is available. `l3_injection_available` checks only
the OS, `cliclick` executable, and Chrome executable (`browser_render.rs:1854-1859`). This makes a
focus/permission provisioning failure indistinguishable from a product interaction failure.

There is a second deterministic hazard once focus is fixed: `_test_l3` does not pass `-j 1`
(`just/devops.just:559-567`), although these tests compete for the single global frontmost window
and pointer. The neighboring Browser recipe correctly explains why `#[serial]` cannot serialize
nextest's per-test processes and therefore uses `-j 1` (`just/devops.just:577-581`).

Make the headed-browser harness activate and verify the exact target window before injection,
diagnose missing Accessibility authority as a harness prerequisite, serialize the Level-3 recipe,
and record a clean canonical `just test-l3` run. Until then, criterion 7's keyboard reachability,
ordinary Enter navigation, and pointer behavior have only passing Browser-tier CDP evidence; their
strongest appropriate Level-3 tests are red. Under the review rubric this is a production blocker.

### High — Firefox/WebKit progressive fallback and viewport behavior remain unverified

The specification requires the chosen prompted-link design to be validated in Chromium, Firefox,
and WebKit (`spec.md:287-290`), with viewport-edge and focus coverage (`spec.md:292-295`). The new
Browser-tier geometry and interaction tests cover Chromium only. The public guide explicitly says
Firefox and WebKit have not been executed (`darkmatter/docs/rendering/popover.md:115-142`).

This is not merely missing redundant coverage. The base CSS positions the prompt with `left:0`,
while right-edge flipping exists only inside Chromium's CSS-anchor-positioning `@supports` branch
(`renderable/src/browser/feature.rs:336-359`). An engine without that feature keeps a
max-width-capped panel whose left edge is fixed to a right-edge link; limiting the width does not
prevent its right edge from leaving the viewport. The documentation acknowledges that this
fallback has no edge-flip evidence.

Run real-browser interaction and geometry checks in Firefox and WebKit, then either implement a
portable no-overflow fallback or explicitly revise the specification's supported-engine contract.
Because prompt visibility, navigation, and viewport placement are user-observable browser
behavior, Chromium-only evidence does not satisfy the stated multi-engine requirement.

### Medium — The Mermaid CSS acceptance contract no longer matches production

Acceptance criterion 1 still requires one Mermaid CSS block, and the implementation target still
describes Mermaid as a JS-and-CSS feature (`spec.md:211-218`). Production now intentionally returns
`FeatureAssets { css: None, ... }` and passes the palette through real Mermaid
`themeVariables` (`darkmatter/lib/src/mermaid/feature.rs:79-94`). That design is technically sound
and the real-engine Browser test proves it works, but it does not implement the written criterion.

The apparent criterion-1 CSS coverage in Renderable uses a test-only resolver that returns
`.mermaid{display:block}` (`renderable/src/tree/render/browser.rs:5051-5090`), not
`DarkmatterFeatureResolver`. Darkmatter's actual dedup test now checks one module script and one
`themeVariables` object instead (`darkmatter/lib/tests/style_features_phase5.rs:142-172`). Several
comments and implementation notes still claim the production resolver emits one Mermaid CSS block.

Amend criterion 1 and the implementation notes to describe the intentional script-only Mermaid
bundle, or add a purposeful Mermaid stylesheet if one is actually required. Do not preserve a
test-only CSS asset merely to make a stale criterion appear covered.

### Medium — Test documentation still misclassifies CDP input as Level 3

The feature tests themselves now correctly name CDP probes `browser_*`, but the shared browser
harness rustdoc still calls CDP input a “faithful equivalent” of a user keypress and calls
`ChromeHarness::drive` the Level-3 driver (`biscuit-browser-harness/src/lib.rs:272-278,488-494`).
The popover guide likewise lists nonexistent old `level3_popover_*` names and says their CDP input
is Level 3 (`darkmatter/docs/rendering/popover.md:132-134`). The actual Level-3 test names and input
source differ.

Correct these descriptions so future reviews and CI reports cannot mistake passing Browser-tier
CDP checks for OS-injection evidence. This is behavioral documentation drift in the exact area
review 2 identified.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| 1. Mermaid asset dedup | Level 1 tests on both render paths | Script dedup passes; the required production Mermaid CSS block is absent, so the written criterion is not met |
| 2. Markdown neutrality | Level 1 render/snapshot tests | Appropriate; passed |
| 3. Interactive browser default | Browser tier with the real vendored Mermaid 11.6.0 engine | Appropriate; passed |
| 4. Compatibility defaults | Level 1 mode/default tests plus Browser-tier static-SVG tests | Appropriate; passed |
| 5. Body-only placement | Level 1 snapshots plus Browser-tier DOM-parent/nesting assertion | Appropriate; passed |
| 6. Divergent request config | Deferred by the specification until a config-bearing feature | Not applicable to fieldless v1 |
| 7. Popover behavior | Browser-tier computed style/geometry and CDP interaction pass; genuine macOS Level 3 fails | Gap: key/pointer requirements have red Level-3 evidence, and Firefox/WebKit behavior is unverified |
| 8. `MermaidHtml` retirement | Source audit and compilation | Appropriate; passed |
| 9. Resolver failures | Level 1 typed matches in Renderable and Darkmatter | Appropriate; passed |
| 10. Side-channel preservation | Level 1 map/hook/parity/order tests | Appropriate; passed |
| 11. Asset safety/failure fallback | Level 1 escaping/version tests, Browser-tier stub failure paths, and real-engine success | Appropriate; passed |
| 12. Cross-platform/regression | macOS Level 1 and Browser suites; Level 3 fails; Windows/Linux not executed | Partial: deterministic non-L3 paths pass on the host, but the required interaction gate is red |
| 13. Documentation cleanup | Source audit plus updated Mermaid/render rustdoc | Partial: prior API drift is fixed, but Mermaid CSS and input-tier descriptions remain stale |

## Prior-review closure

- The live Mermaid test now serves the exact vendored 11.6.0 ESM import closure over loopback and
  verifies real SVG structure plus distinct computed light/dark node fills.
- `HtmlPage` memoizes the resolved feature head, and a stateful counting-resolver test proves one
  resolution per deduplicated feature across eager validation and final rendering.
- `DarkmatterPage::render_to_browser` now qualifies its feature-free equivalence, explains the
  forced feature wrapper, and documents typed `UnresolvedFeature`/`HeadRequired` failures.
- The former CDP tests are correctly routed to the Browser tier, and genuine OS-input tests were
  added. Those new Level-3 tests currently fail, so that prior finding is improved but not closed.

## Verification performed

- `just test` from `renderable/`: 529 passed, 16 skipped.
- Focused Darkmatter feature binaries: 14 passed.
- Focused real Mermaid Browser test: 1 passed against vendored Mermaid 11.6.0.
- `just test-browser` from `darkmatter/`: 99 passed, 5,566 skipped.
- `just test-l3` from `darkmatter/`: failed. All three OS-input popover tests failed repeatedly;
  nextest reported 0 passes before fail-fast cancellation.
- Isolated `level3_popover_tab_focuses_anchor_and_reveals_prompt` with `-j 1`: failed all four
  configured attempts (`active=false;vis=hidden`).
- `git diff --check`: passed.
- GitNexus exploration used the current `features` branch index at `HEAD` to trace body-only
  rendering, interactive Mermaid collection, final `HtmlPage` assembly, and prompted-link output.
  No Rust symbols were edited during this review.
