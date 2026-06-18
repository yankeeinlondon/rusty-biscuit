---
ready: false
agent: codex
model: ""
---

# Review: OpenCode New Stderr Log Format

## Findings

### High: lifecycle promotion still misses the observed new-format records without `service=...`

The implementation parses the new `timestamp=... level=...` envelope, but lifecycle classification still dispatches solely on the `service` tag:

- `claudine/lib/src/stream/logs/opencode/errors.rs:452`
- `claudine/lib/src/stream/logs/opencode/errors.rs:456`

That means the exact examples that motivated the fix in the spec remain semantically unclassified when they omit `service`:

- `timestamp=... message=loop session.id=... step=1`
- `timestamp=... message=stream providerID=... modelID=...`
- `timestamp=... message=evaluated permission=glob...`

Those observed examples are in `spec.md:17-24`. They carry enough information for `StepLoop`, `LlmCall`, and `PermissionEvaluated`, but because `service` is absent, `classify_lifecycle` returns `None` and the bridge treats them as `Unclassified`.

Impact: raw passthrough is suppressed because structured unclassified records are consumed, but the feature still fails the spec's semantic requirements: watchdog semantic activity, model/provider summary enrichment, subagent lineage, and permission signal promotion are not restored for the observed OpenCode format.

The tests do not catch this because the new fixture and classifier tests add `service=...` to all lifecycle samples:

- `claudine/lib/tests/fixtures/logs/opencode-new-format-lifecycle.txt:2-11`
- `claudine/lib/src/stream/logs/opencode/errors.rs:932-1039`

Suggested fix: either infer lifecycle class from `message=` plus required tags when `service` is absent, or normalize an inferred service during parsing for the known message/tag shapes. Add tests using the spec's observed service-less samples.

### High: user-visible compose passthrough behavior is not verified through the wrapper path

The user-visible regression is "raw `timestamp=` lines appear in the terminal during `compose`." The strongest new coverage is Level 1 in-process parser/classifier/bridge testing:

- parser unit tests in `events.rs`
- classifier unit tests in `errors.rs`
- bridge in-process test at `claudine/lib/src/stream/logs/opencode/reasoning.rs:1478`

That is useful, but it does not run `claudine compose` or the OpenCode wrapper stderr thread, so it does not prove the actual CLI path suppresses raw stderr passthrough or refreshes wrapper state in the way users observe it. The spec's verification checklist explicitly includes "No raw `timestamp=` lines appear in terminal output during `compose`" and "Bridge returns `Consumed` for every new-format line observed in the wild" (`spec.md:317-320`).

Verification level: current strongest level is Level 1 in-process. For this requirement, Level 1 is appropriate, but it should be a Level 1 CLI/wrapper integration test with a fake `opencode` executable that emits representative new-format stderr while `claudine compose` runs. Level 2 is not required because the behavior is byte routing/filtering, not terminal emulator rendering. Current coverage is therefore at the right level category but the wrong integration boundary.

Suggested fix: add a CLI integration test that invokes `claudine compose --opencode` with a fake `opencode` on `PATH`, emits both observed service-less and service-tagged new-format stderr lines, and asserts stderr does not contain raw `timestamp=` lines while summary/semantic effects are present where applicable.

## Verification Level Matrix

| Requirement | Strongest current verification | Review |
|---|---:|---|
| New-format headers parse as structured records | Level 1 unit | Adequate |
| Old-format headers still parse | Existing Level 1 unit | Adequate for parser compatibility |
| New-format lifecycle records classify and promote semantic events | Level 1 unit/in-process bridge | Inadequate samples: tests only cover records with `service=...`, not the observed service-less records |
| Raw `timestamp=` lines do not appear during `compose` | Level 1 in-process bridge only | Gap: needs Level 1 CLI/wrapper integration |
| Summary enrichment from model/provider/subagent lineage | Level 1 bridge fixture | Gap for observed service-less records |
| Permission events from new format are promoted | Level 1 classifier/bridge | Gap for observed service-less `message=evaluated permission=...` sample |
| Terminal rendering, colors, glyphs, keyboard behavior | Not applicable | No Level 2 or Level 3 requirement here |

## Tests Run

```text
cargo test -p claudine new_format --lib --color=never
```

Result: passed, 21 tests.

## Production Readiness

Not ready for production. The parser envelope work is in place, but the implementation does not yet satisfy the semantic recovery requirements for the observed new-format lines, and the user-visible `compose` stderr filtering path is not covered end to end.
