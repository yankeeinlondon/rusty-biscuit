# Test Suite Audits

Use this reference for an explicitly requested comprehensive audit or
test-performance specification. Success is complete evaluation with targeted
remediation, not rewriting every test or making every test equally fast.

## Inventory and disposition

Inventory test definitions and shared helpers across the scoped crates,
including embedded unit tests, integration targets, doctests, and benchmarks.
Reconcile source coverage with runner discovery for each supported platform,
tier, and feature selection. Record gated/ignored tests even when they cannot
run locally. Families may share a row only when their members are explicitly
enumerated and have the same setup and proof; give exceptions their own rows.

For each test/family record:

- the behavior proved and whether assertions distinguish a plausible failure;
- the required boundary and external inputs (CWD, home/config/cache, environment,
  repository, installed tools, network, terminal, browser, hardware);
- setup cost, waits, resource ownership/cleanup, and shared-state coordination;
- canonical recipe, tier, required features, platform coverage, and skip reason;
- measured duration/work counts where available, with provenance;
- disposition: satisfactory, remediation in scope, or linked follow-up with
  reason and acceptance criteria.

Timing thresholds prioritize investigation but never determine inventory
membership. Inspect all consumers of a problematic helper. Do not count a
guard allowlist as proof of correctness; every remaining exception needs a
specific reason. Structural guards should reject stale exemptions and include
negative cases showing that violations are detected.

## Measurement

Keep three costs separate: build/setup time, runner execution elapsed time,
and the sum of individual test durations. A serial sum is not wall clock when
tests execute concurrently. Track test identities/counts, failures, skips,
timeouts, retries, and slow cases alongside speed so lost coverage cannot look
like an optimization.

For local before/after measurements, warm each revision's artifacts and
alternate repeated runs on the same host with matching toolchain, features,
profile, concurrency, and fixture inputs. Record revision, dirty changes,
platform, cache state, and run commands. Measure cold builds separately using
isolated build directories rather than clearing the developer's working cache.
Use the full relevant suite to exercise contention after targeted diagnosis.
Warm with the exact recipe you will time: each recipe unifies features for
its own package selection, so a raw `nextest run --no-run` over a wider set
does not produce the artifacts a narrower recipe builds. Report both the
whole-series drift bracket (max − min per revision) and the paired ratio of
each alternating pair; when the host drifts by more than the effect, only the
paired reading survives, and say which rule each number came from.

For CI, retain per-package/tier/environment artifacts and run identities.
Compare each environment against its own compatible baseline and report a
matched-test cohort separately from newly added or removed tests. Choose
performance targets after attribution and baseline collection; report misses
and their causes. Local timing does not establish a CI target. Do not select
only successful attempts without disclosing intervening failures.

Prefer deterministic work-count assertions for eliminated discovery, requests,
or repeated scans. Preserve representative real-boundary tests and measure
product startup on representative repositories separately: removing accidental
checkout discovery from fixtures does not fix the product's discovery cost.
Where no counter exists, an lldb breakpoint hit count on the suite's own test
binary is one without instrumentation or root: resolve the binary through the
recipe's package selection so feature unification names the same artifact,
and count only the function's *entry* location — a regex breakpoint also
lands on every closure the function instantiates and inflates the aggregate
several-fold. A `PATH` shim that logs its working directory is the equivalent
for "which directory did the child launch from".

The shared `tools/test-audit` package is that reporting script for every
area ([test-audit-tooling.md](test-audit-tooling.md)); do not write another
one inside a fix directory. If a reporting script becomes a gate, require it to reject malformed reports,
missing expected artifacts/tests, duplicate identities, invalid durations, and
failed runs. A report that prints a miss but exits successfully is not a gate.
Account explicitly for platform exclusions rather than requiring identical
cross-platform test counts: declare them per environment, report an excluded
test's absence apart from violations, and fail when an excluded test *does*
run on that leg, so the list cannot outlive the `cfg` that justified it.
Sampling one commit more than once on `main` needs `gh run rerun <id>`; every
later push is a different source state, and a `main` concurrency group that
cancels in-flight runs makes that an operator call rather than a script's.

## Remediation and closure

Repair shared fixture defaults before migrating consumers. Preserve observable
assertions; when an assertion itself is defective, document its replacement
and the additional failure it now detects. Do not reduce test populations,
weaken cleanup, or alter production defaults merely to hit a timing target.

Keep deterministic coverage separate from intentional host/API/terminal tests.
Use existing tier recipes and ensure no terminal or browser gains focus. Audit
unavailable tiers statically and report their runtime evidence as pending.

Close with a reconciled inventory, addressed findings, linked deferrals,
coverage differences, local and CI evidence, and remaining uncertainty.
“Implemented,” “verified locally,” and “verified on CI” are separate claims.
