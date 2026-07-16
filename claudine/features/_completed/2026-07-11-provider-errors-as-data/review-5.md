---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-13T10:38:05-07:00
spec: 2026-07-11-provider-errors-as-data/spec.md
implemented: false
description: A **feature** review of `2026-07-11-provider-errors-as-data/spec.md`
feature: 2026-07-11-provider-errors-as-data/review-5.md
---

# Review 5: Provider Errors as Data

## Verdict

The feature is **not ready for production**. The generated runtime vocabulary,
provider-aware parser wiring, deterministic gate, and exhausted-remediation
failure path are implemented and pass their appropriate Level-1 verification.
The stale D10 wording identified in review 4 is also corrected. However, Phase
B's required live pilot and fleet execution never occurred, and the recorded
source-liveness advisory did not probe any cited URL. Both records currently
describe substitutes as completed specification checkpoints.

## Findings

### High: The required live Phase-B pilot and fleet were replaced by an in-process proxy

The specification requires a live Codex pilot with the D10 checks active and a
fleet run over the remaining roster (`spec.md:392-401`, `spec.md:536-548`). The
pilot is supposed to measure real resume telemetry—whether resumes fire,
whether the research session corrects its document, and whether the budget
converges—before the pattern graduates.

The implementation records that neither live execution happened:

- `_pilot-codex.md:10-20` says the live `claudine sequence` run was unavailable
  and that a mechanical proxy was accepted instead.
- `_fleet-review.md:9-20` says the remaining nine documents were authored by the
  implementing agent and only the schema/checker mechanics were run.
- `plan.md:435` likewise records that the live fleet was not runnable.

The Level-1 tests are strong for the local mechanics: they prove a manufactured
bad document produces findings, the fake resumable provider makes three
attempts, and exhausted findings abort the real `claudine compose` process.
They cannot establish that a real provider research session receives the
findings prompt, edits the intended document, preserves the research contract,
or converges within the chosen budget. Those are the observations B2 explicitly
requires before B3 and graduation.

Run the Codex pilot through `_fleet.md` with a real resumable provider, retain
the requested resume/convergence telemetry, obtain the specified checkpoint,
then run the remaining roster through the same workflow. If the project intends
to accept authored-as-researcher documents and synthetic resume verification as
the permanent contract, amend the specification explicitly instead of marking
the live checkpoints complete.

### Medium: The source-liveness advisory contains no liveness results

D10 lists source liveness as an advisory check: cited URLs should be probed and
unresolvable sources reported without blocking the fleet (`spec.md:493-504`).
`_fleet-review.md:76-93` labels its section a source-liveness advisory but states
that the session did not probe the URLs. It classifies citations as
“strong” or “weaker,” which assesses source quality rather than liveness.

Perform a bounded, non-blocking URL check and record per-source outcomes,
including timeouts or network-unavailable results as advisory states. The
check must remain non-fatal as specified. If URL probing is intentionally out
of scope, remove the check from D10 rather than claiming it was produced.

## Requirement Verification

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Research frontmatter is the sole executable vocabulary source and generation is deterministic | Level 1: real-input projection tests, full-roster archived-seed tests, drift tests, and `claudine-gen check` | Meets the required level. |
| Every parser consumes generated data; Kimi codes remain exact and code-first | Level 1: provider classifier tests plus the complete Kimi mapping projection test | Meets the required level. |
| Kilo retains its own identity and vocabulary while sharing OpenCode's parser | Level 1: discriminating injected-vocabulary test, event/summary identity test, and invalid-identity test | Meets the required level. |
| Accepted Codex capacity additions win the intended bucket without broad collisions | Level 1: parser-level positive and negative/collision fixtures | Meets the required level. |
| Gate outcomes distinguish `clean`, `findings`, and `gate_error`, and report-write failure stops processing | Level 1: checker process and atomic-replacement integration tests | Meets the required level. |
| Findings surviving two resumes fail the command and preserve the report | Level 1: real `claudine compose` process test with a fake resumable provider | Meets the required level. |
| Codex pilot and remaining fleet run through the real research workflow with convergence telemetry | No live provider workflow execution; Level-1 manufactured/fake-provider proxies only | Gap. The spec explicitly requires the live pilot and fleet. |
| Source liveness is reported advisory-only | Document assessment without URL probes | Gap. Source quality was reviewed, but liveness was not checked. |

This feature has no terminal-rendering, terminal-input, keyboard, paste, IME,
mouse, or scrolling requirements. Level 2 and Level 3 tests are therefore not
applicable; Level 1 is appropriate for its local runtime and CLI behavior. The
live research workflow is an external-provider verification obligation rather
than a terminal-emulator test.

## Verification Performed

- 25 focused provider-classification, provider-identity, lifecycle, and error
  transport tests passed.
- 18 generator integration tests passed across the deterministic gate,
  vocabulary projection, archived-seed identity, and committed drift suites.
- Both fleet lifecycle tests passed, including exhausted remediation.
- The real CLI `provider_error_finalize` process test passed.
- `cargo run --quiet -p claudine-gen -- check` reported the generated stream
  vocabulary clean.
- `just lint` passed for all five Claudine packages and repository guards.

GitNexus inspection found the generated accessor feeding all eight parser
classifiers and the OpenCode/Kilo identity seam. No code symbol was edited by
this review; only this review artifact and the specification's review counter
changed.
