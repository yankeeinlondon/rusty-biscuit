---
title: "Spike 4 — can the sniff area-drift contract run on a hosted runner?"
created: 2026-09-15
status: complete
results: reviews/2026-09-15-python-test-code/spike-4-results.md
timebox: 1 day
source: reviews/2026-09-15-python-test-code/review.md §1.2
decides: where AC15 is enforced, and whether the cheap sniff query is acceptable
---

# Spike 4 — Can the sniff area-drift contract run on a hosted runner?

## Question

AC15 makes `sniff` the authority on package areas, and `CLAUDE.md` names
area-derivation as load-bearing. But the three contracts that enforce it are
`@unittest.skipUnless(shutil.which("sniff"), "requires sniff")`
(`test_resolved_plan.py:272`, `:299`, `:334`), and `ci-tooling` runs on
`ubuntu-latest` installing only `nextest` and `just`. **They always skip in CI.**
The invariant is verified only when a developer happens to have `sniff` on `PATH`.

Can it run on a hosted runner, and at what cost?

## Why it is open

Two costs, and a semantic trap.

**Cost 1 — building sniff.** Installing it means building a workspace member inside
`ci-tooling`.

**Cost 2 — the fan-out.** `test_every_workspace_members_area_matches_sniff`
(`:300`) spawns **73 `sniff` subprocesses** through a
`ThreadPoolExecutor(max_workers=8)`. Measured on this host: **14.5s wall, 24s user,
85s system**. The system time is the tell — a single `sniff repo package-area` call
runs at ~750% CPU (it parallelizes internally), so an 8-way pool multiplies an
already-parallel process. This is not sniff being slow; it is the harness fighting it.

**The trap.** A much cheaper query exists, and it answers a *different question*.

## The cheap query, measured

Inverting the loop — `sniff repo package-areas --json` once, then
`sniff repo packages --package-area <A> --json` per area (32 areas, so 33 calls
instead of 73):

| Approach | Calls | Wall | User | System |
|---|---:|---:|---:|---:|
| Current: `package-area` per directory, 8-way pool | 73 | 14.5s | 24s | 85s |
| Inverted: `packages --package-area` per area | 33 | **2.75s** | 1.1s | **1.7s** |

~5× wall, ~50× system time. But:

```
biscuit-test-harness -> ABSENT
```

The inverted map **loses `biscuit-test-harness` entirely** — which is precisely the
divergence `SNIFF_SELF_INCONSISTENT` (`test_resolved_plan.py:55`) exists to
document:

> `sniff repo package-area` run from `biscuit-test-harness/` answers
> `biscuit-test-harness`, but `sniff repo package-areas` does not list that name…
> Named here rather than tolerated silently: when sniff is fixed, this entry fails
> and gets deleted.

The cheap query asks "which packages does sniff *list* under this area". The
contract asks "what area does sniff *detect* for this directory". AC15 makes the
second authoritative. So the optimization narrows the contract, and the one case it
loses is the one the fixture was written to catch.

There is also a third option worth pricing: `sniff repo package-areas --package
<PKG> --json` answers the per-package area in one call without a `cd` (~0.14s
measured), but is still one process per package.

## Preconditions

- `sniff` present locally for the comparison steps.
- Know which sniff binary CI would get: a `cargo build -p sniff-cli` in-job, a
  prebuilt artifact, or `just install`.

## Steps

### 1. Price the build (~2h)

On `ubuntu-latest`, warm `Swatinem/rust-cache`:

```bash
time cargo build -p sniff-cli --release
```

Then cold. Record both. If `ci-tooling` already compiles anything that shares
sniff's dependency graph, note it — the marginal cost is what matters.

### 2. Decide the query semantics (~2h) — **the real decision**

Not a measurement; a ruling. Three candidates:

| Option | Contract verified | Cost |
|---|---|---|
| **A. Keep per-directory** | what AC15 names: sniff's *detection* rule | 14.5s / 85s sys |
| **B. Inverted only** | sniff's area *universe* | 2.75s / 1.7s sys, **loses `biscuit-test-harness`** |
| **C. Both** | detection (authoritative) + universe (second assertion) | A + 2.75s |

Option B is the only one that is cheap, and it is the only one that changes what is
being proven. Option C is defensible — `test_the_area_universe_is_a_subset_of_sniffs`
(`:335`) already asserts against the universe, so a second, cheaper cross-check has
precedent — but it does not reduce the cost that motivated the spike.

Before ruling, establish whether the fan-out is expensive because of sniff or
because of the pool: **re-time option A with `max_workers=1` and with
`max_workers=4`.** If serial is not much worse than 8-way, the pool is the problem
and option A may be affordable unchanged.

### 3. Prototype `ci-tooling` with sniff (~2h)

Add the build + the three contracts. Measure total job delta against the current
`ci-tooling` wall time.

### 4. Check the other tool-gated contracts (~1h)

While sniff is in the job, confirm nothing else in `test_resolved_plan.py` silently
depended on its absence.

## Results to record

| Measurement | Value |
|---|---|
| `cargo build -p sniff-cli --release`, warm | |
| …cold | |
| Option A wall/sys at `max_workers` 1 / 4 / 8 | / / , 14.5s/85s @8 measured |
| `ci-tooling` wall before | |
| `ci-tooling` wall after | |
| Query semantics ruling | A / B / C |

## Decision rule

- **Total job delta under ~2 minutes** → install sniff in `ci-tooling`, delete all
  three `skipUnless`, and AC15 becomes a real gate. Preferred outcome.
- **Delta over that** → move the three contracts to a scheduled job that builds sniff
  once, **and** change the skip message in `ci-tooling` to name the job that does
  enforce them. A skip saying `"requires sniff"` is dishonest about coverage; one
  saying `"enforced by the nightly area-drift job"` is not.
- **On the query semantics:** default to **A**. Adopt **B** only if step 2 shows the
  detection rule and the universe agree for every member *including*
  `biscuit-test-harness` — i.e. only once sniff is fixed and
  `SNIFF_SELF_INCONSISTENT` is empty. Until then B silently deletes the fixture's
  reason for existing.

## Ships regardless of the outcome

- An honest skip message wherever the contract is not enforced (§1.2).
- The `max_workers` measurement — if the pool is the cost driver, that is a one-line
  fix worth taking whatever else is decided.
- A recorded answer to "where is AC15 actually enforced?", which today has no
  written answer.

## Risks

- **Stubbing sniff would defeat the whole contract.** `sniff_package_area`
  (`test_resolved_plan.py:58-64`) is deliberate: *"the point of the drift contract is
  to ask the other tool, not to compare the planner against a second copy of its own
  rule."* No option in this spike may replace sniff with a fixture.
- **Building sniff in `ci-tooling` couples a CI-infrastructure job to a workspace
  package.** If sniff stops compiling, the CI-tooling gate turns red for a reason
  unrelated to CI tooling. The scheduled-job fallback avoids this; weigh it.
- **`SNIFF_SELF_INCONSISTENT` is a dated observation** ("measured 2026-09-11 over all
  73 members"). Re-measure during the spike; if it is now empty, option B becomes
  available and the entry should be deleted as its comment instructs.
