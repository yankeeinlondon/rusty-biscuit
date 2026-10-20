# Test Suite Audits

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

Use this page only for an explicitly requested comprehensive audit or
test-performance spec. Success is a complete evaluation with targeted
remediation, not rewriting every test or making every test equally fast. The
tooling is [test-audit-tooling.md](test-audit-tooling.md).

## Inventory and disposition

Inventory test definitions and shared helpers across the scoped crates:
embedded unit tests, integration targets, doctests, and benches. Reconcile
the source population with what the runner discovers for each platform, tier,
and feature selection, and record gated or ignored tests even when they
cannot run locally. A family may share a row only when its members are
enumerated and share setup and proof.

For each test or family record: the behavior proved and whether the
assertions catch a plausible failure; the boundary and external inputs; setup
cost, waits, and cleanup; recipe, tier, features, platforms, and skip reason;
measured duration or work counts with provenance; and a disposition
(satisfactory, fixed in scope, or follow-up with acceptance criteria).

Timing thresholds prioritize investigation; they never decide inventory
membership. Inspect every consumer of a problematic helper. An allowlist is
not proof: each exception needs a reason, and a structural guard rejects stale
exemptions and has negative cases showing it detects violations.

## Measurement

- **Three costs, kept apart:** build/setup, runner elapsed time, and the sum
  of per-test durations. A sum is not wall time under parallelism.
- **Counts travel with timings.** Report identities, failures, skips,
  timeouts, retries, and slow marks next to every duration, so lost coverage
  cannot read as a speed-up.
- **Local before/after:** warm each revision with the exact recipe you will
  time (each recipe unifies features for its own selection), then alternate
  repeated runs on one host with matched toolchain, features, profile,
  concurrency, and inputs. Report the per-revision drift bracket and the
  paired ratio of each alternating pair; when host drift exceeds the effect,
  only the paired reading counts. Measure cold builds in isolated build
  directories, not by clearing the developer's cache.
- **CI:** compare each environment only against its own baseline, report the
  matched-test cohort apart from added or removed tests, and disclose failed
  attempts. Local timing does not set a CI target; choose targets after
  attribution. To sample one commit more than once, rerun the same CI run,
  since every later push is a different source state.
- **Work counts over timings** for eliminated discovery, requests, or scans.
  Where no counter exists, count breakpoint hits on the function's *entry*
  location in the suite's own test binary (a regex breakpoint also hits every
  closure it instantiates), or use a `PATH` shim that logs its working
  directory for "where did the child launch". Removing accidental checkout
  discovery from fixtures does not fix the product's own startup cost;
  measure that separately on representative repositories.

## Gates built from reports

Use `tools/test-audit`; do not write another reporting script inside a fix
directory. A report becomes a gate only when it rejects malformed reports,
missing artifacts or tests, duplicate identities, invalid durations, and
failed runs; one that prints a miss and exits 0 is not a gate. Declare
platform exclusions per environment instead of requiring identical
cross-platform counts, and fail when an excluded test *does* run on that leg,
so the list cannot outlive its `cfg`.

## Remediation and closure

Repair shared fixture defaults before migrating consumers. Preserve
observable assertions; when one is defective, document its replacement and
the extra failure it now detects. Do not shrink test populations, weaken
cleanup, or change production defaults to hit a timing target. Keep
deterministic coverage apart from intentional host, API, or terminal tests,
and audit unavailable tiers statically with their runtime evidence reported
as pending.

Close with a reconciled inventory, addressed findings, linked deferrals,
coverage differences, local and CI evidence, and remaining uncertainty.
"Implemented," "verified locally," and "verified on CI" are separate claims.
