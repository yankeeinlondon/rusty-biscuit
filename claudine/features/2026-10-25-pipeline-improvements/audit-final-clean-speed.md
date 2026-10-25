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

The coordinator's combined parallel L1 run passed **1,269 / 1,269 tests**,
with zero tests over five seconds. All **15 clean-parity tests passed**;
the maximum was **0.969 seconds** (`bom_before_frontmatter`), compared with
the original matrix's 5.036 seconds. The new roster guard took 0.017 seconds.
All printed-output, JSON, and saved-byte assertions remain exercised across
the same 14 shared cases.

`git diff --check` passed after the edit. Original full-gate evidence is
`/tmp/pipeline-final-darkmatter-l1.log`; final evidence is
`/tmp/pipeline-final-speed-verified.log` (Nextest run
`67830caf-931f-47fc-80d7-cc8c45165925`). The combined execution took 81.071
seconds; compilation is separate from per-test durations. No independent
build, test, or lint execution overlapped the coordinator's run.

To reproduce just this slice from the repository root:

```sh
BISCUIT_TEST_FILTER='test(clean_parity::)' just test darkmatter-cli
```

This reproduction command was not an additional run. The passing evidence
comes from the coordinator's broader combined invocation.
