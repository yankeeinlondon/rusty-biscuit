---
title: "Spike 3 — what does test_build_key.py cost on a cold runner?"
created: 2026-09-15
status: complete
results: reviews/2026-09-15-python-test-code/spike-3-results.md
timebox: half a day
source: reviews/2026-09-15-python-test-code/review.md §1.1, §5.6
decides: how test_build_key.py is wired into CI and just ci-local
---

# Spike 3 — What does `test_build_key.py` cost on a cold runner?

## Question

`test_build_key.py` (12 tests) is executed by **nothing** — not `ci.yml`, not
`just/ci-local.just`, not `.githooks/`. It must be wired in (§1.1). But it may
compile Rust while doing so (§5.6). Which, how long, and therefore wired in *where*?

## Why it is open

`build_key.helper_command()` (`scripts/ci/build_key.py:65-78`) resolves
`ci-build` as:

> An explicit binary where one is already built, otherwise a `cargo run`
> invocation against `scripts/Cargo.toml`.

Six of the twelve tests reach it (`DigestTests` ×5, `CanonicalizationTests` ×1),
plus `test_the_cli_and_the_module_agree_on_one_input` which drives the binary
directly. Warm on this host the whole suite is **0.13s**. Cold it is a Rust build
inside what presents as a unit suite — and the `subprocess.run` at
`test_build_key.py:139` has **no `timeout=`**, the only unbounded subprocess in the
directory.

This is very likely *why* the suite was never wired in: whoever tried it hit a
surprise build and moved on. The spike exists to replace that guess with a number.

## What is at stake if it stays unwired

`build_key.py`'s module docstring states the contract it defends:

> The rule this suite exists to defend is negative: there is no second digest. A
> planner that quietly fell back to `hashlib` when `ci-build` was missing would
> still emit a plan, and every downstream comparison against a producer's realized
> manifest would silently stop meaning anything.

It also holds the only pinned XXH64 vectors in the repository
(`test_build_key.py:94-97`) — a swapped hashing implementation would re-key every
build and no gate would notice.

## Preconditions

- A clean `ubuntu-latest` container (or a runner with `scripts/target` removed) and
  the pinned toolchain from `rust-toolchain.toml`.
- Know what `ci-tooling` has already built by the time this step would run: it does
  `rustup show`, installs `nextest` and `just`, then runs the Python suites. Check
  whether any earlier step populates `scripts/target`.

## Steps

### 1. Cold measurement (~1h)

```bash
rm -rf scripts/target
time python3 scripts/ci/test_build_key.py
```

Record wall time and whether `cargo` ran (`CARGO_LOG=info`, or watch for
`Compiling`).

### 2. Warm measurement (~15m)

Repeat without removing `scripts/target`. Record.

### 3. Establish what `ci-tooling` already has (~30m)

Walk `ci.yml`'s `ci-tooling` job in order. Does anything before the Python suites
build `scripts/Cargo.toml`? If a later step does, moving `test_build_key.py` after it
is free.

### 4. Split feasibility (~30m)

Confirm which tests need no binary at all:

- `HelperResolutionTests` — 4 tests; three set `BISCUIT_*` to a stub and assert a
  *refusal*, so they need no real binary. The fourth (`:76`) calls
  `helper_command()` and asserts only the shape of the returned argv.
- `CanonicalizationTests` — 1 test; calls `planned_keys`, **does** need the binary.
- `DigestTests` — 7 tests; all need it.

Verify by running with `scripts/target` removed and `cargo` off `PATH`; the
no-binary set must still pass or fail-loudly rather than hang.

## Results to record

| Measurement | Value |
|---|---|
| Cold wall time | |
| Cold: did `cargo` compile? | |
| Warm wall time | 0.13s (measured, macOS) |
| Does `ci-tooling` already build `scripts/`? | |
| Tests that pass with no binary and no `cargo` | |

## Decision rule

- **Cold under ~30s** → add to `ci-tooling` and to the `just ci-local` self-test
  loop (`just/ci-local.just:449`) unconditionally. Simplest outcome.
- **Cold 30s–3min** → add a `ci-build` build step to `ci-tooling` before it, or
  reorder after a step that already builds `scripts/`. Keep it out of the
  `just ci-local` loop, which must stay fast enough to run on every push.
- **Cold over ~3min** → split: `HelperResolutionTests` (+ the shape-only test) run
  everywhere; `DigestTests` and `CanonicalizationTests` run only where the binary
  exists, behind the `require_tool()` guard from §1.2 — so they *fail* under `CI` if
  the binary is absent rather than skipping.

## Ships regardless of the outcome

- `timeout=` on `test_build_key.py:139`. No unbounded subprocess should exist in a
  suite the hook runs.
- `test_build_key.py` executed by something.
- `test_build_baseline_revision.py` wired into `ci-tooling` as well — it has no
  Cargo dependency (it reads blobs and builds temp repositories, 5.8s measured), so
  it needs no spike, only the same PR.

## Risks

- **Measuring the wrong cold.** A GitHub runner with a warm `Swatinem/rust-cache`
  is not cold. Measure both, and treat the cache as an optimization, not a
  producer/consumer contract (the repo's own standing rule).
- **The split option weakens the contract.** If `DigestTests` ends up gated, the
  pinned XXH64 vectors are only checked where the binary happens to exist — the same
  failure mode as §1.2. Prefer building the binary over gating the tests.
