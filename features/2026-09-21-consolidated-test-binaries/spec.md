---
area: repo
status: draft
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
---

# Consolidate integration tests into one binary per tier

## Outcome

A package's integration tests link into **one test binary per test tier**, not
one per source file. Every test that exists today still exists, runs in the same
tier, behind the same feature gate, on the same environments, with the same
assertions. What changes is how many executables carry them: `claudine-cli` goes
from 141 to a handful. The four largest packages hold 303 of the workspace's 533
integration test files, so they are where nearly all of the gain is.

The consequences this is for, in the order they hurt today: CI archive size and
transfer, producer link time, and local disk consumed per worktree.

## Problem and evidence

Cargo builds every top-level file in a package's `tests/` directory as its own
executable. Each one statically links the package, its whole dependency tree,
and debug information. The workspace has **533** such files across 56 packages
(counting only `<package>/tests/*.rs`; unit-test module directories under `src/`
are not separate binaries):

| Package | Top-level `tests/*.rs` files |
|---|---|
| `claudine/cli` | 139 |
| `darkmatter/lib` | 73 |
| `darkmatter/cli` | 53 |
| `biscuit-terminal/lib` | 38 |
| `sniff/lib` | 19 |
| 51 other packages | 211, none above 15 |

Four packages hold 303 of them. All figures below are single observations from
2026-09-21, to be reproduced, not baselines.

### CI: multi-gigabyte archives

From the `build (ubuntu-latest)` producer job of pull request 92 (run
`35662502516`), which packs each package's test executables into a nextest
archive and uploads it for the test cells to download:

| Archive | Uploaded | Note |
|---|---|---|
| `claudine-cli` | **2.25 GB** | "Archiving 141 binaries", 911 files, 43 s to pack |
| the archive after `claudine-gen` | > 2.13 GB, still uploading when observed | not identified; by order, `darkmatter` |
| `claudine` | 375 MB | 14 binaries |
| `claudine-gen` | 218 MB | 14 binaries |
| `biscuit-file` | 75 MB | 16 binaries |

More than 5 GB uploaded with half the archives done. Size tracks binary count,
not code size: `claudine` and `claudine-cli` share almost their entire dependency
tree, and the one with ten times the binaries is six times the size after
compression. Every consuming cell then downloads its archive again.

### CI: link time

In the completed producer job analyzed in `2026-09-21-ci-build-feature-divergence`
(run `35657985254`), the `claudine-cli` archive took 543 s of Cargo time against
178 s for `claudine`. That spec attributes the difference to recompiling
workspace crates under divergent features. Some of it is 141 links instead of
14. **The split was not measured**, and it decides how much of that 543 s this
feature recovers; measuring it is this feature's first task.

### Local: disk

On the macOS development host, in one worktree's `target/debug/deps`:

| Measure | Value |
|---|---|
| Current `claudine-cli` test executables | 136, **2.67 GB**, mean 19 MB |
| All executables over 1 MB in the directory | 2,348, **91.8 GB** |

The second figure includes stale copies, and that is the point: whenever a
package's configuration changes — a feature flag, a dependency bump, a profile —
every one of its test executables is relinked under a new hash and the old ones
stay until a sweep. A package with 141 test binaries leaves 141 stale
executables behind per change; with three, three. This host carries five
worktrees. The `storage-strategy` skill exists because of where that leads.

> **Not established:** how much of the 91.8 GB is test executables as opposed to
> package binaries, or how much is stale. The `claudine-cli` row is exact; the
> total is an upper bound on what this feature can touch.

## Scope and design decisions

### 1. One binary per tier, not one per package

Each package's `tests/` directory becomes a small number of test crates, one per
tier the package already has, with today's files as modules:

```text
tests/
  l1/main.rs          mod compose_validation; mod wrap_env; ...
  level2/main.rs      mod level2_auto_complete_chooser; ...
  browser/main.rs
  common/             shared helpers, declared once per binary that needs them
```

Tier is the right seam, and the only one that preserves today's behavior,
because tier is already how tests are gated and selected:

- **Feature gates.** `claudine/cli/Cargo.toml` carries 37 `[[test]]` entries, one
  per L2 file, each `required-features = ["terminal-tests"]`. They collapse to
  one entry for the `level2` binary. A single all-tiers binary would instead need
  `#[cfg(feature)]` on every module and would compile L2 harness code into every
  L1 build.
- **Selection.** The tier recipes filter on test **names**, with patterns like
  `test(/(^|::)level2_/)`. The `(^|::)` anchor already tolerates a module path,
  and `.config/nextest.toml` contains no `binary(...)` filter at all. A module
  named `level2_frontmatter_tables` keeps every test inside it selected exactly
  as the file of that name is today. Verify each pattern against the new
  listing; do not assume.
- **`harness = false` targets** (16 in `darkmatter/lib`, a few elsewhere) stay
  separate binaries. They have their own `main`.

Start with the four packages above. Packages with a dozen test files gain little
and are out of scope unless measurement says otherwise.

### 2. What consolidation changes, and what must be migrated with it

None of these is a reason not to do it. Each is a way to do it wrong silently.

- **Test identities change.** nextest names a test `<package>::<binary>
  <module path>::<test>`. `claudine-cli::wrap_env sets_home` becomes
  `claudine-cli::l1 wrap_env::sets_home`. Evidence receipts, plan cells, and
  artifacts are keyed on `{package, environment, tier}` and are unaffected. JUnit
  `<testsuite>` names and `.github/ci/ci-baseline.toml`, whose entries are exact
  `<testsuite>::<testcase>` identities, are affected. The baseline holds zero
  entries today, so the migration is a format note, not a data migration; confirm
  it is still zero when implementing.
- **Snapshot files are renamed.** Eight packages use `insta`, and the repository
  tracks **1,436** `.snap` files. insta derives a snapshot's file name from the
  test's module path, which begins with the binary name, so every snapshot taken
  from an integration test gets a new name. Rename them mechanically in the same
  commit as the move. A missed one must fail loudly, never regenerate: confirm CI
  runs with snapshot updates disabled, and run the consolidated suites locally
  the same way before pushing.
- **Inner attributes become outer ones.** 82 of `claudine/cli`'s test files open
  with a crate-level attribute, 76 of them `#![cfg(unix)]`. In a module, that is
  `#[cfg(unix)] mod name;` in `main.rs`. Dropping one does not fail on the
  platform that writes the change; it fails, or silently runs where it should
  not, on another. Load the `os` skill and check every moved file's attributes
  against its `mod` line by script, not by eye.
- **Guards that name test file paths.** `claudine/cli/tests/spawn_site_guard.rs`,
  `darkmatter/cli/tests/spawn_site_guard.rs`, and
  `tools/test-toolkit/src/archive_guard.rs` refer to `tests/<file>.rs` paths,
  in scan roots and allowlists. Moving files moves those paths. Each guard must
  be shown to still scan every file it scanned before — a guard that finds
  nothing passes.
- **By-name invocations.** 18 places in `just` recipes, docs, and skills pass
  `--test <binary>`. Each becomes a name filter.
- **Shared helpers.** `mod common;` is declared in each binary today, which is
  why helpers unused by one binary need `#[allow(dead_code)]`. One declaration
  per tier binary removes most of those allowances. Remove the ones that become
  unnecessary; leave the rest.

What does **not** change: nextest runs every test in its own process, so tests
that share a binary still share no memory, no statics, no `atexit` handlers, and
no working directory. `serial_test` keys, `bin_exe!`, and `manifest_dir!()`
behave as before. This would not hold under `cargo test`, which this repository
does not use; say so in the `rust-testing` skill.

### 3. The cost: the edit-one-test loop

Today, editing one test file recompiles one small crate and relinks one binary.
After consolidation it recompiles the tier's whole test crate and relinks one
larger binary. For `claudine-cli`'s L1 tier that is on the order of a hundred
modules in one crate.

This is the real trade and it is not settled by reasoning. Measure, on
`claudine-cli`, before and after: the time from touching one test file to that
test's result, and the time for a clean build of the package's tests. Codegen
units parallelize a large crate, and one link replaces many, so the expectation
is a modest loss on the first and a large gain on the second. If the first
regresses badly, the remedy is a finer seam — a few binaries per tier, grouped
by subject — not a return to one per file. Record the numbers and the decision
here.

### Alternatives considered

- **Strip or drop debug information from the CI test profile.** Shrinks archives
  without moving a file, and is worth doing on its own. It does nothing for link
  count, local disk, or stale copies, and it costs CI backtraces their line
  numbers. Complementary, smaller, and not a substitute.
- **Skip archives on pull requests.** A pull request schedules no WSL2 leg, which
  is the archive's main consumer, so its Linux archives feed only hosted Linux
  cells that could compile for themselves. That reopens
  `2026-09-12-single-os-compile`'s compile-once ruling, which this feature does
  not need to do to succeed. Out of scope; note it there if it is ever wanted.
- **Move integration tests into unit tests.** Changes what the tests can reach —
  integration tests see only the public API, and the CLI ones drive the real
  binary. Not a packaging decision.

### Out of scope

- Divergent feature configurations multiplying workspace compiles —
  `2026-09-21-ci-build-feature-divergence`. The two compound: each extra
  configuration of a package relinks every one of its test binaries, so this
  feature also shrinks what that one costs.
- Deleting, merging, or rewriting any test. A test that moves is byte-identical
  apart from its attributes becoming outer attributes.
- Sweeping existing stale artifacts. That is `storage-strategy`'s job; this
  feature reduces how fast they accumulate.
- Any new CI run, matrix cell, fixture, or gate.

## Sequencing

Do this before `2026-09-21-ci-build-feature-divergence`. It is mechanical, its
gain does not depend on a design ruling, and it changes that spec's evidence: the
543 s `claudine-cli` archive mixes link cost and recompile cost, and only after
consolidation does what remains belong to feature divergence. Attributing first
and consolidating second would mean attributing twice.

Within this feature, go one package per change, `claudine-cli` first: it is the
largest, it has every hazard above, and its result decides whether the seam in
part 1 is right before three more packages adopt it.

## Acceptance criteria

1. **No test is lost, gained, or moved between tiers.** For each migrated
   package, the set of test function paths (module path and name, binary
   stripped) listed by nextest is identical before and after, per tier and per
   feature set, and the script that compares them is committed beside the
   change. Compare on macOS and on Linux: `cfg` attributes make the sets differ
   by platform, and that difference must itself be unchanged.
2. Every moved file's former crate-level attributes appear as outer attributes on
   its `mod` declaration, verified by script.
3. Every `.snap` file is accounted for: renamed or untouched, none orphaned, none
   newly created. The consolidated suites pass with snapshot updates disabled.
4. Each path-naming guard is shown to scan the same set of files as before, by
   listing what it scanned, not by its passing.
5. `just test`, `just test-l2`, and `just lint` pass in each migrated package
   area, and `just test-browser` where the package has that tier. The strict
   pre-push hook passes. Reuse qualifying evidence for other environments; the
   change is platform-sensitive only through criterion 2, which a script checks
   on every platform's behalf.
6. The numbers from "The cost" are recorded for `claudine-cli`: edit-one-test
   latency and clean test-build time, before and after, with the host's load
   average beside each (see `2026-09-21-host-contention` for why).
7. Recorded, not gated: `claudine-cli`'s test-executable count and on-disk size
   locally, and its archive size and Cargo build time in the next producer job
   that runs anyway. Do not trigger a run to obtain them.
8. The `rust-testing` skill states the layout rule for new integration tests —
   add a module to the tier's binary, never a new top-level file — and why it is
   safe under nextest. A guard that fails on a new top-level `tests/*.rs` file
   in a migrated package keeps the count from creeping back; add it to an
   existing guard rather than creating a new suite.
