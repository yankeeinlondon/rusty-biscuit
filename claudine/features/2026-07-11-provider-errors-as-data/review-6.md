---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-13T12:07:58-07:00
spec: 2026-07-11-provider-errors-as-data/spec.md
implemented: false
description: A **feature** review of `2026-07-11-provider-errors-as-data/spec.md`
feature: 2026-07-11-provider-errors-as-data/review-6.md
---

# Review 6: Provider Errors as Data

## Verdict

The feature is **not ready for production**. The runtime migration, generated
vocabulary, provider-aware parser wiring, deterministic drift enforcement, and
fail-closed resume lifecycle all pass their appropriate Level-1 verification.
Review 5's source-liveness finding is resolved: the fleet review now records a
bounded probe of all 13 cited URLs and keeps failures advisory. The required
live Codex pilot and remaining-provider fleet still have not run through the
real research workflow, however, and the deterministic gate does not fully
enforce two provenance/coverage rules promised by D7/D10.

## Findings

### High: The required live Phase-B pilot and fleet still have not run

The specification requires a Codex research run with the D10 checks live and
review of actual resume/convergence telemetry before the remaining roster is
run (`spec.md:540-548`). The implementation records the opposite:

- `_pilot-codex.md:10-20` says the pilot document was authored directly by the
  implementing agent and passed through the mechanical checks because the live
  `claudine sequence` workflow was unavailable.
- `_fleet-review.md:11-20` says the remaining nine documents were also authored
  directly and then schema/checker validated.

The Level-1 tests correctly prove the local mechanism: findings cause a
budgeted resume, correction replaces the findings report, exhaustion fails the
command, and the report survives. They do not prove that a real provider
research session receives the findings prompt, edits the intended document,
retains the research contract, or converges within two attempts. The recorded
claims that the clean authored documents “converged in a single pass” and that
two attempts are sufficient are therefore projections, not the telemetry B2
requires.

Run the Codex pilot through `_fleet.md` with a real resumable provider and
retain the attempt/resume/correction telemetry. Review that checkpoint before
running the remaining roster through the same workflow. If direct authoring
plus synthetic lifecycle tests is now the intended acceptance standard, amend
B2/B3 explicitly instead of recording the substitute as the specified run.

### Medium: The deterministic gate does not fully enforce its D7/D10 data contract

Two promised input classes are missing from the gate:

1. `_schema.yaml:32-40` states that `empirical` evidence requires a scrubbed
   fixture path plus capture notes and that the deterministic gate enforces the
   conditional constraint. `provenance_for` only distinguishes `seed` from
   non-seed and accepts every non-seed record with any nonempty `source`
   (`gen/src/agent_errors_check.rs:360-418`). An `empirical` row with an
   arbitrary string and no capture notes therefore passes. No empirical-specific
   test exists.
2. D10 defines motivating-class coverage as overload/capacity vocabulary in any
   bucket, including `429`/`503`. `check_motivating_class` scans keyword text,
   optional code names, and gaps, but never the numeric `code` value itself
   (`gen/src/agent_errors_check.rs:421-443`). A real numeric 429/503 code bucket
   is incorrectly treated as uncovered unless its optional name or a gap also
   contains a recognized term.

Add typed empirical evidence fields (or an equivalently strict, testable source
format) and validate the fixture path plus capture notes. Include numeric code
values in motivating-class evaluation. Add Level-1 positive and negative tests
for both rules. The current research corpus contains no `empirical` records and
its Kimi coverage is acknowledged by a gap, so these are gate-contract defects
rather than evidence of incorrect generated runtime tables today.

### Low: `NeedleHygiene` has a stale and inaccurate doc comment

`gen/src/agent_errors_check.rs:146-147` says interior whitespace is rejected and
that the substring matcher could never see it. The implementation checks only
empty, leading/trailing-whitespace, and uppercase needles, and an interior-space
substring can match normally. The code agrees with D10's stated hygiene rule;
the comment is the drift. Remove “interior” and describe the rule as authored
input hygiene rather than an impossible match.

## Requirement Verification

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Research frontmatter is the sole executable source; facts collision and generated drift fail loudly | Level 1: real-corpus loader, source-collision, archived-seed, deterministic generation, and committed-drift tests | Meets the required level. |
| Every parser consumes generated vocabulary; Kimi codes are exact/code-first; Kilo retains its runtime identity | Level 1: parser classifier tests, complete Kimi projection/precedence tests, and discriminating OpenCode/Kilo identity tests | Meets the required level. |
| Accepted Codex overload/capacity additions classify correctly without broad collisions | Level 1: parser-level positive, precedence, and negative prose fixtures | Meets the required level. |
| Gate outcomes are explicit and durable; persistence failure stops processing; exhausted remediation fails the command | Level 1: generator IO/process tests, production lifecycle execution tests, and a real CLI process test with a fake resumable provider | Meets the required level for the local mechanism. |
| Non-seed provenance, including empirical captures, is mechanically coherent | Level 1 covers missing `source` and invented `seed`; no empirical fixture/capture validation test | Gap. The documented empirical contract is not enforced. |
| Motivating-class coverage recognizes vocabulary in every supported bucket | Level 1 covers text needles and gaps; no numeric 429/503 code-value case | Gap. Numeric code values are ignored. |
| Codex pilot and remaining fleet run through the real research workflow with measured convergence | Manufactured documents and fake-provider Level-1 lifecycle tests only; no live provider workflow execution | Gap. This is an external-provider verification obligation, not a terminal tier. |
| Source liveness is reported without becoming a blocking gate | Operational review artifact records 13 bounded URL probes, all reachable, while preserving advisory semantics | Meets the specified advisory requirement. |

This feature has no terminal-rendering, keyboard, paste, IME, mouse, or
scrolling requirement. Level 2 and Level 3 are therefore not applicable;
Level 1 is the correct tier for its local parser, generator, and CLI behavior.
The missing live fleet run is not remediated by an L2/L3 terminal test because
the missing boundary is the real networked provider research session.

## Verification Performed

- `claudine-gen`: 136 Level-1 tests passed, including the real research corpus,
  archived seeds, gate IO/process behavior, projection, and drift suites.
- 16 focused provider classifier/identity tests passed.
- Both committed fleet-lifecycle tests passed.
- The real CLI `provider_error_finalize` process test passed.
- `cargo run --quiet -p claudine-gen -- check` reported
  `stream vocabulary.rs: clean` and all other generated artifacts clean.
- `cargo clippy -p claudine-gen --all-targets -- -D warnings` passed.
- The area-wide `just test` was stopped after exceeding the non-interactive
  command-duration bound; no test had failed, and all feature-relevant tests
  reached before interruption passed. The bounded feature suites above were
  then rerun to completion.

GitNexus inspection found all eight classifiers calling the shared cascade and
the generated accessor, with the OpenCode/Kilo identity seam explicit. The
index was three documentation-only commits behind HEAD, so its symbol graph
still covered the reviewed code. No code symbol was edited by this review.
