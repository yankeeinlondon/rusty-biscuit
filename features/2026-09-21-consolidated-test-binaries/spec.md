---
area: repo
status: planned
created: 2026-09-21
owner: Ken Snyder <ken@ken.net>
origin: review of pull request 92's `build (ubuntu-latest)` producer job and of local disk use, 2026-09-21
related:
    - 2026-09-21-ci-build-feature-divergence
    - 2026-09-12-single-os-compile
packages:
    - claudine-cli
    - darkmatter
    - darkmatter-cli
    - biscuit-terminal
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-5.6-sol
reviewed_on: 2026-09-21
review_iterations: 0
human_review: false
message_to_agent: |-
    Phase 1 (rulings, spikes, baselines) is complete. Read `rulings.md` first: several plan assumptions were corrected by evidence, and Phase 2's task text now has an "Adjustments from Phase 1 evidence" block at the top of the Phase 2 section that supersedes it where they differ.

    Key facts for Phase 2 (toolkit):
    - The skip baseline is NOT the only static identity store (R5). `.config/nextest.toml` has exact-name overrides for two in-scope tests (`compose_loop_rate_limit_pause_waits_then_continues` in claudine-cli `loop_cli.rs`, `every_catalog_variable_survives_ambient_options` in darkmatter `ambient_ctx_capture.rs`) that silently stop matching once the file becomes a module. `capture` must also list under every override `filter` (read verbatim from the config), and `compare` must check them.
    - Exactly one neutral alias is required today: darkmatter-cli `level2_harness_integrity` (gated on `terminal-tests`, but its 4 tests are L1-tier) becomes `harness_integrity` inside the `level2` binary. `plan` must reproduce this from projection, not hard-code it. Alias form = strip the marker prefix (R2); prefixing is rejected because unanchored override regexes like `test(/level2_/)` would still match.
    - A `required-features` target that is not enabled is absent from `rust-suites`, not an empty suite (S1 section 4). Platform-absent sets need on-host captures from Linux/Windows (S1).
    - The before-listings are at `baseline/listings/*.json.gz` (114 files, macOS, 18 package x feature-set combinations x every `_tier_filter` tier, plus darkmatter's `BISCUIT_L1_INCLUDE_SLOW=1` L1), with `SHA256SUMS` over the raw JSON. `baseline/inventory.json` is the migration manifest's before side. The Phase 1 prototypes (`baseline/*.py|sh`, `spikes/s2-scan.py`) are what `consolidation.py` should fold in; its no-op self-proof reproduces those outputs.
    - The S3 measurement before series was taken on a loaded shared host (load average 10 to 57). Phase 3 must re-run the before side in a pre-migration worktree alternating with the after side, and must first add an edit-only mode to `spikes/s3-measure.sh` without changing the measured command (see `measurements.md`).
    - R9 decides how test-code strings that name `--test <old-binary>` or a test path are handled. The self-exec `--exact public_entry_points::...` string in darkmatter's L2 support is a permitted identity repair. Regenerate-command strings (including the one persisted into `claudine/docs/providers/dispatch-inventory.json`) are updated in each package's docs pass as a separate commit. The author may override R9 before Phase 3.
    - Pre-existing, out of scope, preserved as-is: darkmatter `schema_phase_validation::real_shipped_inline_schema_uses_normal_resolution_and_phase_path` carries a `real_` prefix, so no recipe runs it; biscuit-terminal's 1,086 `layout_matrix__*` snapshots belong to an `#[ignore]`d test.
---

# Consolidate compatible integration tests into shared binaries

> **Reader's note (review, 2026-09-21).** The first draft proposed exactly one
> binary per tier. That boundary does not preserve the repository's existing
> feature contract: Darkmatter's Level 3 browser test requires
> `browser-tests`, while its Level 3 image-painting test requires
> `terminal-tests`. This revision uses one binary per **compatible execution
> contract**: tier, required feature set, standard test-harness mode, and any
> target-wide configuration that cannot be expressed safely on a module. Most
> tiers still produce one binary. The distinction prevents consolidation from
> enabling features for tests that do not declare them.

## Outcome

A migrated package links integration tests that have the same execution
contract into one test binary instead of one executable per source file. Every
test that exists today remains present, in the same tier, behind the same Cargo
features and operating-system conditions, with the same assertions.

For `claudine-cli`, the expected result is four integration-test targets: Level
1, Level 2, Level 3, and real-provider tests. It has 139 integration-test
targets today. The pull request 92 archive's “141 binaries” also counted two
binaries outside those integration-test targets, so 141 is not the before-count
for this feature.

The consequences this is for, in the order they hurt today, are CI archive
size and transfer, producer link time, and local disk consumed per worktree.
The design also protects the edit-one-test loop and peak compiler memory; those
costs are measured during the first-package pilot before the other packages
move.

## Problem and evidence

Cargo normally builds every top-level file in a package's `tests/` directory
as its own executable. Each executable statically links the package, its
dependency tree, and debug information.

At the reviewed revision, Cargo metadata reports **523 integration-test
targets across 51 workspace packages**. The filesystem has 522 top-level
`tests/*.rs` files; Darkmatter adds one explicitly declared nested target at
`tests/error_snapshots/main.rs`. The four packages in scope contain 303 of the
522 top-level files:

| Package | Top-level `tests/*.rs` files | Cargo integration-test targets |
|---|---:|---:|
| `claudine-cli` | 139 | 139 |
| `darkmatter` | 73 | 74 |
| `darkmatter-cli` | 53 | 53 |
| `biscuit-terminal` | 38 | 38 |
| 47 other packages | 219 | 219 |

All performance and size figures below are single observations from 2026-09-21
to be reproduced, not baselines.

### CI: multi-gigabyte archives

The `build (ubuntu-latest)` producer job for pull request 92, run
`35662502516`, packed each package's test executables into a Nextest archive
and uploaded it for test cells to download:

| Archive | Uploaded | Note |
|---|---:|---|
| `claudine-cli` | **2.25 GB** | 141 total archived binaries, 911 files, 43 seconds to pack |
| archive after `claudine-gen` | more than 2.13 GB when observed | not identified in the log; likely `darkmatter` from build order, so do not treat this as a confirmed package measurement |
| `claudine` | 375 MB | 14 binaries |
| `claudine-gen` | 218 MB | 14 binaries |
| `biscuit-file` | 75 MB | 16 binaries |

More than 5 GB had uploaded when half the archives were complete. The
comparison suggests that repeated static linkage is a major contributor, but
it does not isolate that cause from binary contents or compression. Every
consuming cell downloads its package archive again.

### CI: link time

In the completed producer job analyzed by
`2026-09-21-ci-build-feature-divergence`, run `35657985254`, the
`claudine-cli` archive took 543 seconds of Cargo time, compared with 178 seconds
for `claudine`. The related fix explains how divergent dependency features can
recompile workspace crates. This feature addresses the other contributor:
linking many test executables. The 543 seconds has not been attributed between
those causes. Measuring a clean build before and after the `claudine-cli`
pilot is therefore required before claiming a link-time improvement.

### Local disk

On the macOS development host, one worktree's `target/debug/deps` contained:

| Measure | Value |
|---|---:|
| Current `claudine-cli` test executables | 136, **2.67 GB**, mean 19 MB |
| All executables over 1 MB in the directory | 2,348, **91.8 GB** |

The second row includes package binaries, current artifacts, and stale copies.
It is an upper bound on what this feature can affect, not an estimate of test
binary waste. The `claudine-cli` row is the direct observation.

When a package's configuration changes, Cargo can relink every test target
under a new hash while older artifacts remain until a sweep. Reducing the
target count reduces that accumulation rate. Removing existing stale artifacts
is outside this feature.

## Scope and design decisions

### 1. Consolidate by compatible execution contract

Tests may share a binary only when all of these properties match:

- repository tier: Level 1, Level 2, Level 3, browser, or real-provider, plus
  the package's existing policy for `slow_` Level 1 tests;
- the exact `required-features` set on the current Cargo test target;
- standard Rust test-harness mode; and
- target-wide settings that cannot safely move to a module, if an inventory
  finds any.

Operating-system conditions such as `cfg(unix)`, `cfg(windows)`, or
`cfg(target_os = "macos")` remain module conditions inside a compatible
binary. They do not create another binary by themselves.

This boundary means Darkmatter keeps separate Level 3 binaries for its
`browser-tests` and `terminal-tests` feature sets. It also corrects a factual
error in the first draft: Darkmatter's 16 `harness = false` entries are
`[[bench]]` targets, not integration tests, and this feature does not move
them. If the implementation inventory discovers a real custom-harness test,
that test remains a separate target.

Before moving a package, generate a checked inventory from `cargo metadata` and
the package manifest that records every current test target, source path,
required features, and harness mode. Cargo metadata does not expose every
manifest setting, so neither source is sufficient alone. The inventory is the
migration manifest and the input to the before/after identity comparison;
filenames alone are not authoritative.

### 2. Make Cargo target discovery explicit

Each migrated package sets `autotests = false` and declares every consolidated
target explicitly. This prevents a later top-level helper file from silently
becoming another executable and gives reviewers one authoritative target list.

A representative layout is:

```text
tests/
  l1/
    main.rs
    compose_validation.rs
    wrap_env.rs
  level2/
    main.rs
    auto_complete_chooser.rs
  common/
    mod.rs
```

The corresponding manifest entries make the nested crate roots visible to
Cargo and preserve feature gating:

```toml
[package]
autotests = false

[[test]]
name = "l1"
path = "tests/l1/main.rs"

[[test]]
name = "level2"
path = "tests/level2/main.rs"
required-features = ["terminal-tests"]
```

Cargo does not auto-discover `tests/l1/main.rs`; the explicit `path` is
required. Each crate root declares shared helpers once, for example with
`#[path = "../common/mod.rs"] mod common;`. Moved test modules use
`crate::common` instead of redeclaring `mod common;` in every former binary.
This removes repeated compilation of the helper module and most of its
per-binary dead-code allowances.

The module name should normally retain the former test-target name. That gives
the migration script a direct normalization:

```text
before: <old-binary>::<test-path>
after:  <consolidated-binary>::<old-binary>::<test-path>
```

Nextest's `test(...)` filter matches the test path, not the binary name. Moving
a former target name such as `level2_errors` into the module path can therefore
change selection even when the test function name did not change. Before using
the former target name as a module, compare every test in that target against
the current tier filters. If the new module segment would newly match a marker,
use a neutral module alias and record that alias in the migration manifest.
The before/after tier partition must remain identical either way.

### 3. Permit only structural source edits

The first draft said moved files would be byte-identical except for attributes.
That is not possible: 187 files in the four package trees currently declare
`mod common;`, and nested module resolution changes when their source moves.

Allowed edits are limited to the mechanics of making the old test crate a
module:

- convert file-level crate attributes to the equivalent attribute on the
  module declaration when the attribute controls whether the module exists;
- replace repeated helper-module declarations and imports with imports from
  the consolidated crate root;
- repair relative `mod`, `#[path]`, `include_*`, fixture, and snapshot paths
  that changed because the source file moved; and
- resolve names that were crate-local before consolidation but now share a
  crate namespace.

Test bodies, inputs, assertions, timeouts, skip decisions, tier markers, and
feature requirements do not change in this feature. If a test needs a behavior
change to survive consolidation, stop that package's migration and treat the
behavior as a separate defect.

Before editing, scan for crate-global constructs such as exported macros,
global allocators, link/export attributes, startup constructors, and duplicated
crate-root symbols. A source-level `cfg` inventory is necessary but not
sufficient: larger binaries can also expose link-time collisions that did not
exist when every file was a crate.

### 4. Preserve tier selection and operating-system reachability

The canonical tier expressions come from `just _tier_filter`. They are based
on path-segment prefixes such as `level2_` and `browser_`; no implementation or
verification script may re-create those expressions independently.

For every package and supported feature set, capture Nextest's JSON listing
before the move and after it. Normalize only the intentional binary-to-module
identity change described above, then compare these sets separately:

- selected tests for each tier;
- tests excluded as another tier;
- ignored tests; and
- tests absent because the target platform's `cfg` removed them.

Counts are not enough: a lost test and a newly selected test can cancel each
other numerically. The comparison must use exact normalized identities.

Crate-level platform attributes become outer attributes on the relevant module
declaration, for example `#[cfg(unix)] mod compose_validation;`. A committed
check compares every moved file's former crate-level attributes with its module
declaration. Compilation and listings must still be checked on macOS, Linux,
and native Windows; a source scan on macOS cannot prove that the Windows module
graph compiles. WSL2 follows the Linux target configuration but separately
proves that the Linux-built Nextest archive remains portable to a different
checkout path.

### 5. Migrate identity consumers and snapshots deliberately

Consolidation intentionally changes the raw Nextest identity. Completion
records and artifacts are keyed by `{package, environment, tier}`, so their
storage keys do not change. The exact test identities inside expected manifests,
JUnit reports, and skip approvals do change.

The CI skip baseline is empty at review time. Confirm it is still empty before
each package migration. If it is no longer empty, translate each affected
identity with the same checked migration manifest; never delete an approval or
broaden it to a count. No historical JUnit artifact is rewritten.

Insta derives snapshot locations from the assertion source directory and, by
default, prefixes filenames with `module_path!()`. Moving an assertion under a
tier directory and adding a wrapper module can therefore change both its
snapshot directory and filename. Move each affected snapshot mechanically,
preserve its contents, and run with `INSTA_UPDATE=no`. A missed mapping must
fail; implementation must not regenerate or accept snapshots to make the suite
green.

Path-sensitive guards also move with the sources. At minimum, inspect the
Claudine and Darkmatter CLI spawn-site guards and the repository archive-path
guard. For each guard, record the exact file set scanned before and after.
Passing while scanning zero or fewer unintended files is not evidence.

### 6. Preserve the developer's narrow-test workflow

Today a developer can pass `--test <old-binary>` to select one source file's
target. After consolidation, the former binary is a module. Current, active
just recipes, skills, and documentation that recommend `--test` must be updated
to use Nextest's positional test-name filter while retaining the tier's
canonical `-E` expression. For example, an area recipe can run the tests whose
names contain `wrap_env` without replacing the Level 1 filter.

Historical completed specs and review logs are records of commands that were
correct at the time and are not rewritten. The implementation inventory must
distinguish those records from active documentation.

### 7. Keep Nextest's process isolation as an explicit dependency

Nextest runs each test case in its own process, even when cases share a test
binary. Consequently, consolidation does not make test cases share mutable
statics, current directories, environment changes, `atexit` handlers, or
`serial_test` state during canonical repository runs.

That statement is not true for `cargo test`, whose standard harness runs cases
from one binary in the same process. The `rust-testing` skill must document
both sides of the contract and state that migrated packages are tested through
the canonical Nextest-backed recipes. Documentation and new recipes must not
present `cargo test` as an equivalent way to run a migrated suite.

Code that runs before libtest selects a case, including a global constructor or
allocator, would still run from every Nextest process in the consolidated
binary. The pre-migration crate-global scan in section 3 protects this less
obvious boundary.

### 8. Pilot `claudine-cli` before the other packages

Move `claudine-cli` first. It is the largest package and exercises the feature,
platform, helper, path-guard, Level 2, Level 3, real-provider, and snapshot
hazards described above.

Measure before and after with the same Rust toolchain, target triple, Cargo
profile, feature set, linker, worker count, and otherwise idle host. Use fresh,
separate target directories for clean builds. For the edit-one-test loop, run
at least five alternating before/after trials after warm-up and report the
median and slowest observation. Record compiler peak resident memory as well as
elapsed time; one larger Rust crate can trade link count for a memory spike.
Use Sniff to record the host and load context once per measurement series.

The default seam remains one binary per compatible execution contract. Split a
large contract into stable subject groups before moving the other packages if
the pilot shows either of these practical regressions:

- median edit-to-one-test latency rises by both more than 50 percent and more
  than five seconds; or
- the consolidated target cannot compile reliably on a supported runner or
  exceeds its available memory.

The two-part latency threshold avoids treating a small absolute change as a
failure while still protecting a clearly slower inner loop. Subject groups
must be stable product areas, not arbitrary numbered shards, and must keep the
same tier and feature contract.

## Alternatives considered

- **One binary for every tier without regard to features.** This minimizes
  target count but silently enables a union of features. Darkmatter's two Level
  3 feature sets demonstrate the defect today. Rejected.
- **A fixed number of subject binaries per tier.** It provides a predictable
  upper bound on binary size and edit latency, but chooses complexity before
  measurement. Reserved as the pilot fallback in section 8.
- **Keep one binary per source file.** It preserves the current edit loop but
  retains the archive, link-count, and stale-artifact costs. Rejected unless the
  pilot shows consolidation is not viable on a supported host.
- **Strip or drop debug information from the CI test profile.** This can shrink
  archives without moving source files. It does not reduce link count or local
  stale copies, and it can reduce backtrace quality. Complementary and outside
  this feature.
- **Skip archives on pull requests.** This reopens the compile-once artifact
  ruling in `2026-09-12-single-os-compile`. Consolidation does not need that
  policy change to succeed. Out of scope.
- **Move integration tests into unit-test modules.** Integration tests verify
  the public API boundary and CLI tests drive the real executable. Moving them
  changes what they can access and is not a packaging-only change. Rejected.

## Out of scope

- Divergent feature configurations multiplying workspace compiles, owned by
  `2026-09-21-ci-build-feature-divergence`. The two costs compound because each
  distinct configuration links every test target in that configuration.
- Deleting, merging, renaming, or rewriting test behavior.
- Consolidating benchmarks, examples, or real custom-harness tests.
- Sweeping existing stale artifacts.
- Adding a CI run, matrix cell, or gate solely for this feature. Use local and
  remote build hosts for discovery, then record results from the next ordinary
  CI run that already selects the packages.

## Sequencing

Perform the `claudine-cli` pilot before implementing
`2026-09-21-ci-build-feature-divergence`. The pilot is mostly structural and
changes the related fix's timing evidence: only after reducing the link count
can the remaining clean-build time be attributed more confidently to feature
divergence.

After the pilot meets the acceptance criteria and the measurement guardrail,
migrate `darkmatter`, `darkmatter-cli`, and `biscuit-terminal` one package at a
time. Recreate the target inventory and verification evidence for each package;
do not assume the pilot's module and feature shapes apply to the others.

## Acceptance criteria

1. Each migrated package has `autotests = false`, an explicit Cargo test-target
   list, and no undeclared integration-test crate root. The checked migration
   manifest maps every old target to exactly one consolidated target and module.
2. Every old test has exactly one normalized identity after the move. For each
   supported feature set and platform, the selected, excluded, ignored, and
   platform-absent sets are identical before and after; no comparison relies on
   counts alone.
3. Every target retains its exact required-feature contract. In particular,
   Darkmatter's browser-backed and terminal-backed Level 3 tests remain in
   separate targets.
4. Every moved file's former crate-level platform condition is represented on
   its module declaration, and macOS, Linux, and native-Windows compilation and
   listings confirm the resulting module graph. The WSL2 archive path is
   verified when qualifying evidence is available; missing evidence is reported
   as pending rather than called a pass.
5. Test bodies, inputs, assertions, timeouts, and skip decisions are unchanged.
   Source differences outside the allowed structural edits in section 3 fail
   review.
6. Every tracked snapshot has a checked old-to-new mapping or is proven
   unaffected. Snapshot contents do not change, no `.snap.new` files exist, and
   the suites pass with `INSTA_UPDATE=no`.
7. Each path-based guard reports the same intended source coverage before and
   after. The report lists paths, so an empty or accidentally narrowed scan
   cannot pass silently.
8. `just test`, `just test-l2`, and `just lint` pass in every migrated package
   area; `just test-browser` passes where the package owns browser tests.
   Level 3 and real-provider tests retain their existing opt-in and resource
   requirements. Terminal and browser verification does not take host focus.
9. The `claudine-cli` pilot records clean test-build time, warm edit-to-one-test
   latency, peak compiler memory, produced test-target count, and on-disk test
   executable size before and after under the matched conditions in section 8.
   The chosen one-contract or subject-group seam follows the stated guardrail.
10. The next ordinary producer job that selects each migrated package records
    archive size, archive file count, packing time, and Cargo build time. No CI
    run is triggered only to obtain these observations, and they are reported as
    observations rather than pass/fail gates.
11. Active just recipes, documentation, and skills no longer recommend an old
    `--test <binary>` selector for migrated targets. The `rust-testing` skill
    documents the consolidated layout, positional name filtering, feature
    boundary, and Nextest process-isolation dependency.
12. Every migrated package has a structural test inside an existing
    consolidated Level 1 binary that rejects an unexpected test crate root or
    undeclared module. Claudine extends its existing test-placement guard; the
    other packages add the check to their consolidated Level 1 target, not as a
    new test binary or CI gate.
