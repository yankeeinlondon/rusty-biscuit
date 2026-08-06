# Per-Package Test Resolution

Status: draft — awaiting review

## What this changes

CI currently selects, executes, and records work by **package area** — a
directory grouping, 31 of them, each with a policy record in `areas.json`.

It must do all three by **package** — a Cargo workspace member, 72 of them.

The package area is retired as a CI concept. It is not repurposed, not kept for
configuration, and not preserved as a grouping. `areas.json` goes away.

Every remaining mention of "area" in this document exists to say that.

## Problem

The scope calculator already resolves a change to exact packages. Nothing
downstream uses that resolution.

PR #43 wired it to the compile check only:

```yaml
run: cargo check --all-targets ${{ inputs.check-args }}   # impacted packages
...
just test "${args[@]}"                                     # everything in the directory
```

`just test` is a directory-level recipe with its package list written in:

```just
test *args="": _ensure-md
    @just _test_all "claudine-catalog-types claudine claudine-contract claudine-cli claudine-gen" {{ args }}
```

A change to one Claudine package compile-checks three and then runs five test
suites. The check is seconds; the suites are the cost.

## Objective

Run every test of every impacted package. Run nothing else.

"Impacted" means the packages the commit changed, plus every package that
depends on them, transitively.

That is the entire decision. It is binary and per package.

## What actually needs configuring — measured

The obvious objection to retiring `areas.json` is that it holds configuration
CI needs. It mostly does not. Measured across all 31 records:

| Field | Declared by | Verdict |
|---|---|---|
| `environments` | **0/31** | Never overridden. A constant, not configuration. |
| `check_os` | **0/31** | Never overridden. A constant. |
| `check_args` | 27/31 | The `-p` list. Derivable from the package name once the unit is the package. |
| `l2`, `browser`, `node`, `ai_provider_stubs` | 1–8/31 | Derivable — a package has L2 tests if it has `level2_*` test files. |
| `shards` | **2/31** | `darkmatter`, `claudine`. Removed — see § Sharding. |
| `ci: false` (+ `reason`, `owner`, `expiry`, `exclusion_class`) | 10/31 | Genuine, and a property of the **package**: this package does not gate. |
| `native` | **4/31** | Genuine, and a property of the **package**: `playa` needs ALSA because `playa` links ALSA. |
| `backends` | 8/31 | Genuine, and a property of the **package**: which terminal its L2 tests drive. |
| `policy_gaps` | 8/31 | Genuine, and a property of the **environment**: Windows has no tmux. Nothing to do with any package. |

**No field is a genuine property of a package area.** Two are constants, five
are derivable, three belong to packages, one belongs to environments. The
directory grouping is an accident of layout that CI mistook for a policy unit.

So the answer to "what needs configuring" is: **three facts per package, for the
minority of packages that have them, and one small table of environment
capabilities.**

## Sharding

Sharding is removed. It is not carried forward per package, and no replacement
is introduced.

`--partition count:N/M` splits a suite across parallel jobs so the slowest job
sets wall-clock instead of the sum. That pays when *execution* dominates. Here
it does not. Measured on three Claudine shards from run 31041986681:

| shard | build | tests | job total |
|---|---|---|---|
| ubuntu 1/4 | **16.0 min** | 3.9 min | 20.6 min |
| ubuntu 2/4 | **13.1 min** | 1.3 min | 18.0 min |
| ubuntu 3/4 | **11.3 min** | 0.8 min | 15.5 min |

Compiling the test binaries is ~85% of every shard. Sharding divides only the
other 15%, and **every shard compiles everything from scratch** — parallel jobs
cannot share a build cache, because the cache is written when a job ends and
these all start together.

The arithmetic: four shards cost 74.3 minutes of compute for a 20.6 minute
wall-clock. One unsharded job would be one build (~16 min) plus all the tests
(~7 min) ≈ 23 minutes, at 23 minutes of compute. **Roughly 3.2× the compute to
save about 2.4 minutes.** On Windows, where each shard runs ~30 minutes, the
absolute waste is larger.

This is not a misconfiguration. It is a correct technique applied to a workload
whose cost profile it does not match.

Per-package resolution weakens the case further: a package's suite is a fraction
of a directory's, and only impacted packages build at all.

## Future considerations — not in scope

Both of these address the same finding: **compilation, not execution, is what CI
spends its time on.** Neither is proposed here. Both are recorded because the
sharding measurement is the evidence that would justify them, and that evidence
should not have to be gathered twice.

### Build once, run many

If a single package's *execution* time ever becomes genuinely large, the way to
parallelise it is to build once and distribute the compiled binaries, so the
parallel jobs run rather than rebuild.

**The machinery already exists here.** The WSL2 leg uses
`cargo nextest archive`: `nextest-archive.tar.zst` is built once on Linux and
handed to a guest that runs the tests with no toolchain present.

Applied to parallel execution that is one build job plus N run-only jobs —
wall-clock near the build time, compute close to a single job. That is where
splitting a suite starts paying.

Should be justified by a measurement showing some package's execution time is
large *after* the per-package split — not adopted because sharding used to
exist.

### Compilation caching with a remote backend

CI uses `Swatinem/rust-cache@v2` and no compiler cache. `kache` was tried and
removed, measured at **0–6% hit rates (0.4–2.3% weighted by compile cost, ~2–15s
saved)**.

The reason it failed is worth keeping, because it is not a statement about
kache: `kache-action@v1` fell back to the **GitHub Actions cache, whose entries
are immutable and branch-scoped**, so a store shared by all same-platform jobs
could never accumulate. The cache backend defeated it, not the tool.

`docs/kache-strategy.md` records the condition for revisiting: **an S3/R2
backend, and a measured comparison against a no-kache control.** An
S3-compatible bucket removes both the 10 GB quota and the immutability problem,
and the same remote could warm developer machines — kache's designed
multi-machine path. Costs named there: bucket plus credentials in CI secrets,
and the daemon being the least-proven part of kache on Windows.

Version authority stays at `.github/kache-version` (0.12.0) and
`just install-kache` remains an opt-in per-host developer tool. Nothing about
this spec changes that.

If compile time is ever attacked directly, this is the lever — and it is a
larger one than anything in this spec, since compilation is ~85% of a test job.

## Where the survivors live

**Package facts go in the package's own manifest**, following the
`[package.metadata.benchmarks]` pattern already used across this workspace:

```toml
[package.metadata.ci]
gates = false                                  # replaces `ci: false`
reason = "…"                                   # required when gates = false
expiry = "2027-01-31"
native.ubuntu-latest = ["libasound2-dev"]      # replaces `native`
l2-backends = ["tmux", "wezterm"]              # replaces `backends`
```

A package's requirements travel with the package. Moving `playa` does not
require editing a separate registry, and a new package cannot be silently
un-owned.

**Environment capabilities go in one table** — which environments can host tmux,
a browser, Node. Eight `policy_gaps` records today are all restatements of two
facts: Windows has no tmux, and the WSL2 leg runs from an archive with no
terminal server. Declared once, applied everywhere.

## Requirements

**R1 — The package is the unit.** One test job per impacted package per
environment. No job covers more than one package.

**R2 — Reverse-dependency expansion is retained.** A change to `biscuit-speaks`
still tests `claudine`, `claudine-cli`, `claudine-contract`, `research`, and
`research-cli`, because they consume it. Narrowing removes the directory
fan-out; it must never narrow the dependency closure.

**R3 — Result identity is keyed on package.** `{package, environment, tier}`,
replacing `{area, environment, tier}`. Breaking change to the result document,
the merge verdict, and the 32 known-failure entries. See § Migration.

**R4 — Every impacted package runs all of its tests.** There is no partial
selection within a package. Impacted means the whole suite.

**R5 — Package requirements are declared in the package's manifest.** Under
`[package.metadata.ci]`. Nothing outside the package declares what the package
needs.

**R6 — Environment capabilities are declared once.** Not per package, not per
directory.

**R7 — Derivable facts are derived, not declared.** Whether a package has L2
tests or browser tests is read from the package, not written down. A declaration
that can drift from the thing it describes is a defect waiting to happen.

**R7a — Sharding is removed.** No package is partitioned, and `--partition` is
not passed. See § Sharding for the measurement.

**R8 — Local behaviour is unchanged.** `just test` in a directory runs that
directory's packages, as now. `just _test <package>` runs one. Both exist today
and neither changes.

**R9 — CI runs canonical recipes.** CI invokes `_test`, `_test_l2`, `_sanity` —
the same per-package recipes the directory loop already calls.

**R10 — A package with no tests is not a pass.** It records "nothing to run":
never a pass implying coverage, never a missing result that blocks.

**R11 — Guarded by tests.** Contract tests assert the matrix is package-derived
and that package requirements reach their jobs. Each proven non-vacuous.

## Migration — the cost of R3

Result identity is the substantive work and cannot be done incrementally
alongside the old identity.

**The result document.** `CellKey` becomes `{package, environment, tier}`
through the rollup, verdict, and comparison paths.

**The 32 known-failure entries.** Every entry names a directory. Each must be
re-keyed to the package that actually fails — mechanically derivable, because
every JUnit report already records the package for every test.

**Sequencing.** Re-key from the *same run* that first produces package-keyed
results. Earlier is guessing; later leaves every entry unmatched, and an
unmatched entry blocks.

**Job count.** A narrow change produces far fewer jobs than today. A full-scope
change produces more — ~72 packages × environments rather than ~31 directories ×
environments. Full-scope runs are already rare since `Cargo.lock` stopped
triggering them.

## Out of scope

**Specialised workflows are invisible to the merge gate.** `messenger`,
`playa`, `biscuit-tui`, and `rendezvous` run outside the grid and upload no
result artifacts, so a failure in them does not block a merge — `messenger` has
no other CI ownership at all. Real gap, different problem: verdict *coverage*,
not scope *resolution*.

**The expected-test manifest.** `_expected_manifest` and
`ci-rollup --expected-manifest` are wired into no workflow — zero references in
either CI file, so this cannot break them. **Constraint for whoever wires them
up:** generate per package, matching the package the job ran.

## Acceptance criteria

1. A change to one package produces a job for that package and its dependents,
   and no job for unrelated packages that happen to share its directory.
2. A change to `biscuit-speaks` produces jobs for the Claudine and Research
   packages that consume it.
3. Every test job names exactly one package and runs all of that package's tests.
4. `areas.json` is deleted, and no workflow, recipe, or script reads it.
5. A package needing system libraries or specific L2 backends declares them in
   its own manifest, and its job receives them.
6. Environment capabilities are declared once and applied to every package.
7. Results are recorded per `{package, environment, tier}`, and the re-keyed
   known-failure entries match.
8. A package with no tests records "nothing to run".
9. `just test` in a directory, with no CI environment, behaves exactly as today.
10. Contract tests fail when the matrix is replaced by a static or
    directory-derived one.
11. No job passes `--partition`, and no shard identity appears in any result.

## Measurement

Justified by compute saved, so measured rather than asserted. For a
single-package change in a multi-package directory, record before and after:

- number of test jobs scheduled;
- packages actually tested;
- wall-clock from run start to verdict.

A reorganised matrix that does not reduce observed runtime has not met the
objective.

## Open questions for review

1. **Build-cache keys.** `Swatinem/rust-cache` is keyed per directory today —
   `area-ci-${area}-test-${environment}`. When the unit is the package, that key
   has to change, and the choice is not obvious: key per package and each job
   gets a smaller, better-targeted cache but there are more of them to warm; key
   more coarsely and jobs share a larger cache but churn it against each other.

   This matters more than the matrix change itself. Compilation is ~85% of a
   test job, so the cache key decides whether per-package resolution actually
   reduces runtime or merely redistributes it. It should be **measured** on a
   real run, not chosen by argument.

2. **`gates = false`.** Ten records are exclusions carrying `reason`, `owner`,
   and `expiry`. Moving them into manifests preserves the governance metadata —
   confirm that is wanted, rather than treating non-gating as derivable from
   "this package has no tests".

3. **Migration order.** `areas.json` cannot be deleted until every consumer
   moves. Is a transitional period where both exist acceptable, or must the
   cutover be atomic?
