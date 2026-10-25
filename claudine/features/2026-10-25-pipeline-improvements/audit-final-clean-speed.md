---
kind: implementation-log
---

# Final gate: clean-parity speed

The full Darkmatter L1 gate passed 12,391 tests, but
`darkmatter-cli::l1 clean_parity::every_parity_case_matches_through_md_clean`
took **5.036 seconds**, exceeding the five-second per-test limit. That test
serially ran 14 shared cases, spawning `md clean` three times per case for
printed output, JSON, and saved bytes.

The matrix now has one independently scheduled test per case, retaining all
14 inputs, 42 CLI calls, and every assertion. Each test owns an isolated
`CliProcessFixture`. A new roster guard compares the scheduled names with the
shared case list and rejects duplicate scheduling. Each generated test keeps
the shared source `include_str!` in its body.

Counts: **one original slow identity modified**, **15 gross new identities**,
**one old identity replaced**, **net +14 identities**. The net increase is
**one genuinely new coverage guard** and **13 extra scheduling identities**.
No production, fixture-case, L2, runner, or CI configuration changed.

## Verification

Scoped parallel L1 verification is pending the coordinator's full-area gate
finishing. No build or test runs overlap that gate.

`git diff --check` passed after the edit. Original full-gate evidence is
`/tmp/pipeline-final-darkmatter-l1.log`.
