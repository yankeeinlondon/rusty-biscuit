# Per-Package Test Resolution

Status: accepted — review incorporated 2026-08-06

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

Run every CI-gating test tier of every impacted Cargo package. Do not run an
unrelated Cargo package merely because it shares a directory with one that is
impacted.

"Impacted" means the packages the commit changed, plus every package that
depends on them, transitively.

That is the package-selection decision. Test-tier and runner requirements are
separate package policy: L1 is the default gating tier; L2 and browser are
opted into by the package that owns those tests. L3 and `real` remain opt-in
local tiers because they require a focused GUI or external resources and are
not part of this CI grid.

Non-Cargo companion suites are not silently dropped by this definition. The
Homelab frontend is currently reached only because the area-level `test` recipe
runs it after several Rust packages. It must become an explicitly package-owned
companion suite (owned by `homelab-server`) and remain part of that package's L1
result. The same rule applies to future non-Cargo suites.

## What actually needs configuring — measured

The obvious objection to retiring `areas.json` is that it holds configuration
CI needs. It mostly does not. Measured across all 31 records:

| Field | Declared by | Verdict |
|---|---|---|
| `environments` | **0/31** | Never overridden. A constant, not configuration. |
| `check_os` | **0/31** | Never overridden. A constant. |
| `check_args` | 27/31 | The `-p` list is derivable. Non-package feature flags are not; they become package test/build policy. |
| `l2`, `browser` | 2–8/31 | Package test-tier ownership. Not safely derivable before scheduling: tier markers are test names or module paths, not necessarily files, and discovering them with nextest requires building. |
| `node`, `ai_provider_stubs` | **1/31 each** | Genuine runner requirements for the Homelab companion suite and Claudine tests. |
| `shards` | **2/31** | `darkmatter`, `claudine`. Removed — see § Sharding. |
| `ci: false` (+ `reason`, `owner`, `expiry`, `exclusion_class`) | 10/31 | Genuine only for a package-specific exception. Zero or few tests is not justification; those packages can gate and record `NOTHING TO RUN` when applicable. |
| `native` | **4/31** | Genuine, and a property of the **package**: `playa` needs ALSA because `playa` links ALSA. |
| `backends` | 8/31 | Genuine, and a property of the **package**: which terminal its L2 tests drive. |
| `policy_gaps` | 8/31 | Genuine, and a property of the **environment**: Windows has no tmux. Nothing to do with any package. |

**No field is a genuine property of a package area.** Two are constants, one is
derivable, the others belong to packages or environments. The directory
grouping is an accident of layout that CI mistook for a policy unit.

So the answer to "what needs configuring" is: **exceptions to the package
defaults, package-owned non-default tiers and runner requirements, and one small
table of environment capabilities.**

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
parallelize it is to build once and distribute the compiled binaries, so the
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
gates = false
exclusion-class = "promotion-pending"
owner = "@yankeeinlondon"
reason = "…"
expiry = "2027-01-31"

[package.metadata.ci.native]
ubuntu-latest = ["libasound2-dev"]

[package.metadata.ci.tests]
tiers = ["L1", "L2"]
l2-backends = ["tmux", "wezterm"]
features = ["playa"]
all-features = false
l1-include-slow = false
runner-tools = ["ai-provider-stubs", "darkmatter-md-fixture"]
companion-suites = ["homelab-frontend"]
```

A package with no CI metadata defaults to `gates = true` and the L1 tier. The
ten area exclusions are audited package by package, not copied wholesale.
Packages excluded only because the area had zero or few tests (including the
current `visualizer`, `reaper`, `agent-sandbox`, and `tabby` rationales) become
gating packages; this makes their first future test run automatically. A
non-default tier or runner tool is declared only when needed. A `gates = false`
record must retain `exclusion-class`, `owner`, `reason`, and the expiry rules
enforced today; the example is a schema illustration, not one record containing
every possible field. Metadata validation rejects unknown fields, invalid tier
or tool names, conflicting `features`/`all-features`, expired exclusions, an L2
tier without backends, and companion suite names with no registered canonical
recipe.

A package's direct requirements travel with the package. Job provisioning uses
the union of `native` requirements across the package's target-relevant Cargo
dependency closure, not just the package being tested. This matters when a
dependent job compiles a native dependency such as `playa`. Test tiers,
runner-only tools, and companion suites do **not** propagate: those describe the
tests owned by the selected package, not the packages it compiles.

Native requirements are installed everywhere the selected package's binaries
are built or executed. For WSL2, the environment table maps the Ubuntu package
key to both the Linux archive builder and any guest runtime libraries; satisfying
only the builder is insufficient when an archived binary dynamically links a
library the guest does not have.

`features`, `all-features`, and `l1-include-slow` preserve the existing package
test contracts now encoded in area justfiles (for example `biscuit-hash
--all-features`, `sniff --features remote`, and Darkmatter's inclusion of
`slow_`). They are forwarded to compile-check, archive construction, and the
canonical test recipe consistently; the package must not compile-check one
feature set and test another accidentally.

`runner-tools` is a closed vocabulary implemented by the reusable workflow, not
an arbitrary command surface. It contains `ai-provider-stubs`,
`darkmatter-md-fixture`, `node-22`, `pnpm-10`, `l2-parallel-self-spawn`, and
`neovim` (dmls's L2 suite drives a real Neovim inside tmux).
The fixture entry preserves Claudine's clean-checkout `md` binary setup, which
a direct `_test claudine` invocation would otherwise lose. The self-spawn
entry preserves claudine-cli's measured `min(cores, 8)` parallel L2 mode — its
suite is dominated by self-isolating tests that each spawn their own tmux
session, so it runs `_test_l2`'s parallel mode rather than the shared-pane
`-j 1` path. `homelab-frontend` is a closed companion-suite
name that invokes the existing non-focusing frontend test recipe and attributes
its producer status to `homelab-server`/L1. Companion suites must emit
machine-readable evidence or a producer failure; a green Rust JUnit report must
not hide a failed or skipped companion suite.

**Environment capabilities go in `.github/ci/environments.json`** — one
versioned, schema-validated table defining runner labels, native-package
installer keys, and whether an environment can host tmux, a headless browser,
Node/pnpm, or archive-only execution. Eight `policy_gaps` records today are all
restatements of two facts: Windows has no tmux, and the WSL2 leg runs from an
archive with no terminal server. Declared once, applied everywhere. The table
describes capability only; package policy still decides which tiers are
expected, so an unsupported required tier becomes an explicit `POLICY GAP`
rather than disappearing from the result grid.

## Requirements

**R1 — The package is the unit.** One L1 test job per gating impacted package
per supported environment. Non-default tier jobs are also package-scoped. No
result-producing job covers more than one Cargo package.

**R2 — Reverse-dependency expansion is retained.** A change to
`biscuit-speaks` selects `biscuit-speaks` itself and its full current closure:
`biscuit-speaks-cli`, `claudine`, `claudine-cli`, `claudine-contract`,
`research`, and `research-cli`. Narrowing removes the directory fan-out; it
must never narrow the dependency closure.

**R3 — Result identity is keyed on package.** `{package, environment, tier}`,
replacing `{area, environment, tier}`. Breaking change to the result document,
the merge verdict, and the 32 known-failure entries. See § Migration.

**R4 — Every selected package runs every CI-gating tier it owns.** L1 excludes
the deliberately opt-in L2, L3, browser, `real`, and package-specific `perf`
filters. It also excludes `slow` by default; the existing Darkmatter exception
remains explicit package policy. Declared L2 and browser tiers run separately
and without partial test selection. L3 and `real` are not CI-gating tiers in
this grid.

**R5 — Package requirements are declared in the package's manifest.** Under
`[package.metadata.ci]`. Native prerequisites are collected from the selected
package's target-relevant dependency closure; test tiers, runner tools, and
companion suites apply only to their declaring package. Archive build and guest
execution environments both receive their applicable native prerequisites.

**R6 — Environment capabilities are declared once.** Not per package, not per
directory.

**R7 — Defaults are derived and declarations are proved.** Package identity,
manifest path, dependency closure, and default L1 ownership come from Cargo
metadata. Non-default tier declarations are necessary because nextest tier
membership is encoded in test identities and cannot be discovered by the scope
job without building. Contract tests use `cargo nextest list` with the canonical
filter to prove each declared L2/browser tier is non-vacuous and to fail when a
package owns such tests without declaring the tier.

**R7a — Sharding is removed.** No package is partitioned, and `--partition` is
not passed. See § Sharding for the measurement.

**R8 — Local behavior is unchanged.** `just test` in a directory runs that
directory's packages, as now. `just _test <package>` runs one. Both exist today
and neither changes.

**R9 — CI runs canonical recipes.** L1, L2, browser, and lint invoke `_test`,
`_test_l2`, `_test_browser`, and `_lint`, respectively. Compile-check remains
`cargo check --all-targets -p <package>` because there is no per-package
canonical check recipe. Package-specific behavior currently embedded in an
area recipe must be preserved as declared runner tooling or a named companion
suite; `_sanity` is not a substitute for the full L1 suite.

**R10 — A package with no tests is not a pass.** The job may be scheduled to
discover that fact, but a successful invocation with zero selected tests records
the explicit `NOTHING TO RUN` state. It is neither a pass implying coverage nor
a missing result that blocks. A package excluded with `gates = false` is instead
`NOT SCHEDULED`, with its governance metadata; these states are not conflated.

**R11 — Guarded by tests.** Contract tests assert the matrix is package-derived,
dependency requirements reach their jobs, declared tiers contain tests,
undeclared non-default tiers are rejected, companion suites still execute, and
every result producer stamps the package identity. Each assertion is
non-vacuous.

**R12 — Existing non-Rust coverage is preserved.** `homelab-server` continues
to run its Vue/Vitest suite on its Node-capable leg. `claudine-cli` continues to
receive inert provider-discovery stubs. A per-package conversion that drops
either behavior is a regression even if every Rust JUnit report passes.

## Migration — the cost of R3

Result identity is the substantive work. The implementation may be staged
internally, but the checked-in policy and workflow cutover is atomic: no merged
state may have two authoritative policy stores or emit both area-keyed and
package-keyed result documents.

**The result document.** `CellKey` becomes `{package, environment, tier}`
through the rollup, verdict, baseline, skip-budget, policy-gap, and comparison
paths. `RunRecord.area`, producer-status `area`, scope `area_names`, artifact
fallback parsing, and shard identity are removed. The result schema version is
bumped, and readers reject the old version with an explicit migration error.

**The 32 known-failure entries are not a one-to-one mechanical rename.** Two
entries (`claudine-gen-drift` and `coverage`) are synthetic identities, not
directories or Cargo packages. An area cell may contain failures from more than
one package, lint has producer status but no JUnit package evidence, and the
thirteen shard-keyed entries (eight Darkmatter, five Claudine) cease to have
distinct identities when sharding is removed. JUnit-backed failures can seed candidate package entries because
the invocation manifest records a package, but every candidate is verified
against a package-keyed run. Synthetic entries stay outside the package
baseline until their specialized producers join the result contract.

**Sequencing.** Re-key from the *same run* that first produces package-keyed
results. Generate a candidate baseline from that run, review one-to-many area
splits and collapsed shard entries, then land it with the workflow and schema
cutover. Earlier is guessing; later leaves entries unmatched, and an unmatched
scheduled entry blocks.

**Job count.** A narrow change produces far fewer jobs than today. A full-scope
change can produce more — up to the gating subset of 72 packages across the
default environments, plus declared tier jobs, rather than 31 directory calls.
The implementation must calculate and record the exact expanded job count and
verify it remains below GitHub Actions matrix/reusable-workflow limits. A full
workspace run is mandatory before cutover even though full-scope runs became
rarer after `Cargo.lock` stopped unconditionally triggering them.

## Out of scope

**Specialized-only scenarios remain invisible to the merge gate.** The
`messenger`, `playa`, `biscuit-tui`, and `rendezvous` specialized workflows
upload no result artifacts, so their additional feature/platform scenarios do
not block the package verdict. Standard package L1 ownership is decided by this
spec independently; in particular, newly selected Rendezvous packages must not
be treated as covered merely because the specialized workflow exists. Wiring
the extra scenarios into the verdict is a coverage problem, not scope
resolution.

**The expected-test manifest producer.** `_expected_manifest` and
`ci-rollup --expected-manifest` are wired into no workflow. Their existing data
shape is already package-indexed, but any future producer must generate on the
target environment and only for the package and tier that the job ran.

## Acceptance criteria

1. A change to one package produces a job for that package and its dependents,
   and no job for unrelated packages that happen to share its directory.
2. A change to `biscuit-speaks` produces the exact closure named in R2.
3. Every result-producing test job names exactly one package and runs every test
   selected by that package's declared CI tiers and feature policy.
4. `areas.json` is deleted, and no workflow, recipe, or script reads it.
5. A package needing system libraries, Cargo features, runner tools, companion
   suites, or specific L2 backends declares them in its own manifest. Native
   requirements also reach jobs for packages that compile it as a dependency.
6. Environment capabilities are declared once and applied to every package.
7. Results are recorded per `{package, environment, tier}` under a bumped schema
   version. Verified package baseline entries match; synthetic and obsolete
   shard entries are not falsely renamed.
8. A package with no selected tests records `NOTHING TO RUN`; an excluded
   package records `NOT SCHEDULED` with valid governance metadata.
9. `just test` in a directory, with no CI environment, behaves exactly as today.
10. Contract tests fail when the matrix is replaced by a static or
    directory-derived one.
11. No job passes `--partition`, and no shard identity appears in any result.
12. Tier-contract tests fail for both an undeclared non-default test and a
    declared but empty tier.
13. The Homelab frontend, Claudine `md` fixture, provider stubs, existing Cargo
    feature selections, and Darkmatter slow-test policy survive the cutover.
14. A full-scope dry run proves the expanded matrix is within platform limits,
    every scheduled cell emits evidence, and the package baseline is valid.
15. No package is excluded merely because it currently has zero or few tests;
    every remaining `gates = false` record has package-specific justification.

## Measurement

Justified by compute saved, so measured rather than asserted. For a
single-package change in a multi-package directory, record before and after:

- number of test jobs scheduled;
- packages actually tested;
- wall-clock from run start to verdict;
- total billed runner minutes;
- Rust cache restore result and cache size per package/environment key.

A reorganized matrix that does not reduce observed runtime has not met the
objective.

## Implementation decisions requiring measurement

1. **Build-cache keys.** `Swatinem/rust-cache` is keyed per directory today —
   `area-ci-${area}-test-${environment}`. When the unit is the package, that key
   has to change, and the choice is not obvious: key per package and each job
   gets a smaller, better-targeted cache but there are more of them to warm; key
   more coarsely and jobs share a larger cache but churn it against each other.

   This matters more than the matrix change itself. Compilation is ~85% of a
   test job, so the cache key decides whether per-package resolution actually
   reduces runtime or merely redistributes it. It should be **measured** on a
   real run, not chosen by argument. The implementation is not complete until
   one key strategy is selected from measured before/after data and recorded in
   the measurement artifact.

2. **`gates = false`.** Preserve governance metadata for exclusions that remain
   after package-level review. Non-gating is never inferred from zero observed
   tests: that would silently exempt a package and fail to notice when it gains
   its first test.

3. **Migration order.** Dual-read code may exist while the change is developed,
   but the repository cutover is atomic. The same merge deletes `areas.json`,
   moves all consumers, bumps the result schema, and installs the verified
   package baseline.
