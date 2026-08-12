---
ready: true
agent: codex/default
created: 2026-07-09T16:28:09
---

# Review 3: Provider Metadata Automation

## Verdict

Production ready.

No blocking findings remain. Review 2's high-severity signal-contract gap is closed: `DetectionMode` is again exactly `Declarative | Bespoke`, the schema mirrors that vocabulary, and `signals check` now fails unless every compiled record positively replays. The two Antigravity app-log observations are honestly retained as research `gaps` rather than compiled as runtime records without an ingestion path. Review 2's wrapper-boundary coverage gap is also addressed by a spawned-binary Level-1 integration test that exercises the bounded stdout tail through exit-payload synthesis, the signal hub, and the persisted wrapper summary.

## Findings

None.

## Prior-Finding Closure

### Review 2 high: `documentation` mode bypassed runtime and replay guarantees

Closed.

- `DetectionMode::Documentation` and all checker skip/reporting paths are removed.
- The signals sidecar accepts only `declarative | bespoke`.
- The two unwired Antigravity app-log observations were moved from `records` to `gaps`, which is the correct representation until the wrapper gains an app-log ingestion path.
- The fleet checker now enforces `positives_passed == records` after ordinary replay failures are checked.
- `claudine signals check --json` completed with 83 records, 83 positive passes, 83 negative passes, and zero failures.

### Review 2 medium: exit-payload repair lacked wrapper-path integration coverage

Closed.

The new `wrap_antigravity_exit_signal` integration test spawns a fake `agy`, emits more lines than the ten-line stdout tail retains, places the authentication diagnostic at the end, exits nonzero, and verifies that the persisted wrapper summary contains `auth_invalid` with `source: exit`. This covers the production seam that was broken in Review 1 rather than beginning at a manually assembled library payload.

## Requirement Verification Matrix

| Requirement | Strongest verification | Assessment |
|---|---:|---|
| Deterministic research/roster/override inputs generate the committed provider catalog | Level 1 generator, pipeline, UX, and drift tests | Appropriate. The combined catalog-types/generator run passed 112/112, including generated provider data, signals, families, catalog, schema compatibility, and deterministic drift checks. |
| Static wrapper facts are catalog-driven, including default model-flag delivery | Level 1 unit and source-invariant tests | Appropriate. Review 1's `model_cli_flag` gap remained closed. |
| Every compiled signal record drives shipped runtime behavior and positively replays its evidence | Level 1 production-engine fixture replay | Appropriate. The checker passed 83/83 records and no non-runtime detection mode remains. |
| Antigravity stdout exit diagnostics survive wrapper capture and emit `auth_invalid` | Level 1 spawned-binary integration test | Appropriate tier. The test covers bounded-tail capture, post-wait synthesis, signal dispatch, and summary persistence; no terminal emulator behavior is involved. |
| Metadata-driven rendering and provider dispatch remain centralized | Level 1 component/dispatch tests, with the feature's existing Level-2 terminal suite for rendered terminal behavior | Appropriate. This iteration changes no glyph, width, SGR, scrolling, or terminal-emulator contract. |
| Keyboard, hotkey, paste, IME, or mouse behavior | Not applicable | The feature defines no OS-input behavior requiring Level 3. |

## Verification Performed

- `cargo nextest run -p claudine-catalog-types -p claudine-gen --color=never`: 112 passed.
- `cargo nextest run -p claudine -E 'test(/signals/)' --color=never`: 86 passed; 3,302 unrelated tests skipped by the filter.
- `cargo run -q -p claudine-cli -- signals check --json`: 83 records, 83 positive passes, 83 negative passes, zero failures.
- `git diff --check` over the iteration-3 implementation files: clean.
- The targeted `claudine-cli` integration-test command was started twice but did not finish linking within the non-interactive 60-second command budget, so no local execution result is claimed for that one test. Its source and registration were reviewed, and the production CLI binary compiled and ran successfully for the fleet checker.

