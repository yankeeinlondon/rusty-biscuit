# Phase 4 real-composition latency record

## Scope and method

Phase 4 investigated the three `claudine-cli` tests terminated after 90 seconds
on Ubuntu in CI run `31651014023`. The source run used a standard two-core
`ubuntu-latest` runner. Local measurements were taken on 2026-08-13 with the
nextest default profile and a previously built debug binary.

Host discovery used `sniff os --json` and `sniff hardware --json`:

- macOS 27.0, Darwin 27.0.0, arm64
- Apple M4 Max, 16 physical/logical cores
- 128 GiB memory
- APFS workspace storage

Absolute timings are not compared across those runner classes. Work counts and
the number of processes per test are the stable evidence.

## Pre-change test baseline

| Test | CLI processes | macOS nextest time |
|---|---:|---:|
| `shipped_implement_prompt_runs_real_router_target` | 1 | 5.014s |
| `compose_perf_stdout_matches_non_perf` | 2, serial within the test | 8.059s |
| `inline_compose_perf_stdout_matches_non_perf` | 2, serial within the test | 8.059s |

The two parity tests ran concurrently with each other in the baseline command;
each individual test still paid for two serial CLI processes. All three tests
were bounded locally rather than hung.

## Instrumented invocation samples

The `--perf` report now projects request-owned timings from
`InvocationWorkSnapshot`. These are overlapping diagnostic breakdowns, so they
do not participate in the report tree's structural reconciliation.

| Stage | Repository launch, external non-repository source | Non-repository launch and source |
|---|---:|---:|
| Process wall clock | 3.58s | 0.26s |
| Invocation capture | 229.8ms | 5.8ms |
| Repository observation | 14.6ms | 2.3ms |
| Topology initialization | 212.9ms | not run |
| Launch-context capture | 0.931ms | 1.9ms |
| System-prompt preparation | 83.1ms | 4.2ms |
| Composition preparation | 0.619ms | 2.4ms |
| Provider handoff / agent execution | 80.5ms | 81.2ms |

The repository sample reported two Git discoveries, one topology probe, and two
topology reuses. The two discoveries are distinct and required: one observes
the launch repository and one confirms that the external authored source has no
repository. The non-repository sample reported one Git discovery, zero topology
probes, and zero topology reuses.

Focused library regressions additionally prove one launch-context construction
for one preparation epoch. A confirmed non-repository launch never initializes
topology. No duplicate or super-linear discovery was measured inside either
process shape, so Phase 4 makes no production discovery-cache change.

## Selected bound

The parity tests were restructured into one CLI process per test. Perf and
non-perf modes each assert the same fixed stdout fixture, preserving parity
transitively while keeping every test below the per-test budget. A serial
post-change run measured:

| Test | macOS nextest time |
|---|---:|
| `slow_compose_non_perf_stdout_matches_shared_fixture` | 3.427s |
| `slow_compose_perf_stdout_matches_shared_fixture` | 3.339s |
| `slow_inline_compose_non_perf_stdout_matches_shared_fixture` | 3.295s |
| `slow_inline_compose_perf_stdout_matches_shared_fixture` | 3.331s |
| `shipped_implement_prompt_runs_real_router_target` | 4.930s |

No timeout was increased. The `slow_` marker excludes the four process-heavy
parity cases from `sanity`; the complete L1 `test` recipe continues to run them.

## Local validation

- `cd claudine && just test`: passed all five package gates
  (`claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`,
  and `claudine-gen`).
- `cd claudine && just lint`: passed all five package gates.
- The focused performance-report suite passed 14 tests, and the four split
  process-level stdout tests passed through nextest.

## Ubuntu proof record

Two consecutive `ubuntu-latest` runs remain required after these uncommitted
changes are available to CI. Record both run IDs here with the emitted stage
timings and work counts. This session cannot create or push a commit by task
instruction, so it cannot dispatch a CI run containing the Phase 4 changes.
