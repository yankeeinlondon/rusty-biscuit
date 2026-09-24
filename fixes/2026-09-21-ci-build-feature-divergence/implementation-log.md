---
spec: /Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/fixes/2026-09-21-ci-build-feature-divergence/spec.md
plan: fixes/2026-09-21-ci-build-feature-divergence/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1:
    - scripts/feature-attribution.rs
    - scripts/feature-attribution-tests.rs
    - scripts/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/Cargo.lock
    - scripts/ci/fixtures/feature-attribution/core/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/core/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/util/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/util/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/owner-own/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/owner-own/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/owner-plain/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/owner-plain/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/owner-third/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/owner-third/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/owner-wsdep/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/owner-wsdep/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/vendor/base/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/vendor/base/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/vendor/twin-1/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/vendor/twin-1/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/vendor/twin-2/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/vendor/twin-2/src/lib.rs
    - fixes/2026-09-21-ci-build-feature-divergence/timed-pass.sh
    - fixes/2026-09-21-ci-build-feature-divergence/attribution-data/render-attribution.py
docs_updated_during_phase_1:
    - fixes/2026-09-21-ci-build-feature-divergence/spec.md
    - fixes/2026-09-21-ci-build-feature-divergence/plan.md
docs_created_during_phase_1:
    - fixes/2026-09-21-ci-build-feature-divergence/attribution-2026-09-21.md
    - fixes/2026-09-21-ci-build-feature-divergence/implementation-log.md
skills_files_updated_during_phase_1:
    - .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_2: []
docs_updated_during_phase_2:
    - fixes/2026-09-21-ci-build-feature-divergence/implementation-log.md
    - fixes/2026-09-21-ci-build-feature-divergence/plan.md
    - fixes/2026-09-21-ci-build-feature-divergence/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3:
    - fixes/2026-09-21-ci-build-feature-divergence/implementation-log.md
    - fixes/2026-09-21-ci-build-feature-divergence/plan.md
    - fixes/2026-09-21-ci-build-feature-divergence/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
    - repo-deps
---

# Implementation Log for 2026-09-21-ci-build-feature-divergence (4 phases)

> The plan refers to `implementation-notes.md`; this file is that document
> (the plan states "the same document is this fix's implementation log").

## Phase 1

Base for every Phase 1 observation: `fix/ci-build-feature-divergence` at
`6c9ee2e60` (the merge of pull request 96, identical to `main`'s merge base),
macOS host, toolchain 1.98.1. No CI run was triggered by any Phase 1 task.

### Wave 1 — spikes

#### S1 (Task 1.1) — divergence table re-derived at the base

Method: `cargo tree -p <owner> [--features <ci features>] -e normal,build,dev
--format '{p}|{f}' --prefix depth`, rows keyed on `(crate, version)`, the root
package's own line excluded. Run with and without `--target
x86_64-unknown-linux-gnu`; both give the same rows.

- **Reproduced exactly when `claudine-cli` is resolved with no features:** 630
  shared crates, 13 divergent — `clap`, `clap_builder`, `crossterm`,
  `darkmatter`, `hyper`, `hyper-util`, `libc`, `mio`, `proc-macro2`,
  `serde_core`, `time`, `tokio-stream`, `tower`. So the spec's table was
  derived without the owner's CI features.
- **With the CI features the owner actually builds** (`daemon-tests`,
  `terminal-tests`, `test-fixtures` from `[package.metadata.ci.tests]`): 631
  shared, **24 divergent**. The eleven additional rows, all added by
  `claudine-cli`: `cast +std`, `flate2 +any_zlib,zlib-rs`,
  `getrandom 0.2.17 +js,js-sys,rdrand,std,wasm-bindgen`, `half +num-traits`,
  `num +default`, `num-integer +default`, `num-traits +libm`,
  `rustls +log,logging`, `smallvec +serde`, `tokio +signal`,
  `xxhash-rust +xxh32`.
- None of the eleven is one of the spec's three base flags, but `smallvec`,
  `num-traits`, `getrandom`, and `tokio` sit well down the graph. Their
  weight is settled by the seconds ranking (Task 1.8) rather than asserted
  here. S1 says to escalate "if a base flag row appears". See the Task 1.8
  reading for whether any of them qualifies.
- Cross-owner rows for the whole ten-package set come from the
  `feature-attribution` report's `third_party_divergence` (82 third-party
  `(crate, version, side)` rows on the Linux target). See Task 1.8.

#### S2 (Task 1.2) — which owners demand `libc/extra_traits`

**All three `nix` versions in the lock enable it:** `nix 0.26.4`,
`0.29.0`, and `0.31.3` each declare `[dependencies.libc] features =
["extra_traits"]` in their published manifests. Per owner, on the Linux
target, the owners that resolve the target-side `libc` with `extra_traits` are:

| Owner | Chain |
|---|---|
| `claudine-cli` | dev-dep `expectrl 0.8.0` → `nix 0.26.4` (direct dependency of `expectrl`; also via `ptyprocess 0.5.0`) |
| `sniff-cli` | dev-dep `expectrl 0.8.0` → `nix 0.26.4` |
| `repo-deps` | `ctrlc 3.5.2` (the default `local-tools` feature) → `nix 0.31.3` |

The other seven owners resolve `libc` as `default,std` only. `nix 0.29.0`
(via `xpty` → `unchained-ai`) is outside this selection. So the spec's
estimate of two demanding owners is three: `repo-deps` demands the variant
too, through a chain that has nothing to do with `expectrl`.

#### S3 (Task 1.3) — `expectrl` upstream

crates.io API (2026-09-23): newest `expectrl` is **0.9.0** (2026-05-11). Its
normal dependencies still include `nix ^0.26` directly and `ptyprocess ^0.5.0`,
and the newest `ptyprocess` is still 0.5.0 (2025-09-12). Evidence:
<https://crates.io/api/v1/crates/expectrl/0.9.0/dependencies>,
<https://crates.io/api/v1/crates/ptyprocess>. **Source removal is blocked
upstream** (ruling 3), and S2 shows it would not retire the flag anyway,
because `nix 0.31.3` via `ctrlc` enables the same feature for `repo-deps`.

#### S4 (Task 1.4) — does `cache-workspace-crates` help?

Scratch repository with a two-member workspace and one registry crate
(`itoa`), outside the monorepo, under `mktemp -d /tmp/fa-s4.XXXX`:

```sh
git clone src-repo checkout && (cd checkout && CARGO_TARGET_DIR=$S/target cargo build --offline)
tar -C $S -cf cache.tar target            # what rust-cache saves (mtimes kept)
rm -rf checkout target; sleep 2
git clone src-repo checkout; tar -C $S -xf cache.tar   # fresh checkout + restore
(cd checkout && CARGO_TARGET_DIR=$S/target cargo build --offline -v)
```

Result: `Fresh itoa`; `Dirty util: the file util/src/lib.rs has changed
(…, 2s after last build …)`; `Dirty app: the dependency util was rebuilt`.
**Confirmed:** a fresh checkout's mtimes make restored workspace artifacts
stale, so `cache-workspace-crates` would store artifacts CI can never reuse.
Ruling 5 stands and the option stays dismissed.

### Wave 2 — rulings (Task 1.5)

All seven plan rulings are recorded in `spec.md`: the Open Question ruling
(Option 1, Option 2 fallback condition, Option 3 rejected) in its Open
Questions section, and rulings 1–7 as a dated "Rulings" subsection of *Scope
and design decisions*. S3 and S4 evidence is attached to rulings 3 and 5.
Spec `status` flipped `draft-spec` → `finalized-spec`.

Two spike findings changed the evidence without changing a ruling, and both
are written into the spec beside the relevant ruling:

- `libc/extra_traits` has a third demanding owner (`repo-deps` via `ctrlc` →
  `nix 0.31.3`), which reinforces ruling 3.
- `proc-macro2/span-locations` is **target-side only** under the owner's
  explicit `--target` (see Task 1.6). The host-side `proc-macro2` under every
  derive macro is never re-identified, so the spec's "likeliest single largest
  cause" premise does not hold.

### Wave 3 — attribution script (Task 1.6)

`scripts/feature-attribution.rs` is a new `[[bin]]` in `repo-deps`, behind
`required-features = ["local-tools"]`. It adds no new dependencies (it uses
`cargo_metadata`, `biscuit-terminal`, `serde`, `serde_json`, and `anyhow`,
all already declared), so `docs/dependencies.md` has nothing to update.

- **Inputs:** `--owners` (modeled in the producer's order, sorted by package
  name, as `produce` sorts), `--target` (the triple the producer passes;
  defaults to the host), each owner's `[package.metadata.ci.tests]`
  `features` and `sidecars`, and `.github/ci/sidecars.json`. Per owner it
  models the archive invocation (`cargo tree -e normal,build,dev`) and then
  each sidecar build (`cargo tree -e normal,build`), which are the same
  invocations `ci-build produce` runs in the owner tree.
- **Configuration identity:** package `(name, version, source)`, its own
  resolved features, host or target side, and the identities of its non-dev
  dependencies, interned across owners. This is Cargo's unit identity minus
  what is constant within one owner job (profile, rustflags, target).
  - Host side is inferred from the tree: a proc-macro, a build dependency, or
    anything beneath one. Under an explicit `--target` those are separate
    units. This is what separates target-side from host-side
    `proc-macro2`.
  - Dev edges are excluded from identity (they feed test targets only), but
    their feature unification is already present in the child nodes.
- **Cause chain:** each configuration beyond a crate's first is compared with
  the *nearest* earlier configuration (fewest root causes, earliest on a tie).
  Causes are the deepest crates whose own features (or dependency set) differ.
  Each is classified as `own`, `workspace-dependency`, or `third-party`, and
  names the crate, the features added or removed, and the side.
- **Traps baked in:**
  - Packages are keyed on `(name, version, source)`. Test
    `two_versions_of_one_crate_are_not_a_divergence` uses `fa-twin` 1.0.0 and
    2.0.0.
  - The root's own display is never compared as a third-party row, and the
    root's library identity excludes dev edges. Test
    `a_root_package_shares_the_configuration_its_dependents_build` uses
    `fa-core` with `default = []`, built both as a root and as a dependency.
- **Seconds (`--events`):** joins a `ci-build` wrapper counter directory
  whose events carry the owner as `BISCUIT_CI_BUILD_PACKAGE`. Only library
  and build-script units count; test harnesses and binaries belong to the
  owner itself. An owner's seconds for a crate are split across the
  configurations of it that owner built first. A configuration with several
  causes splits its seconds evenly between them. A compile the model does not
  predict is reported in `unexplained_compiles` rather than absorbed.
- **Output:** terminal tables through `biscuit-terminal` (`Prose`, `Table`),
  plus `--json` (schema version 1).
- **Bug caught by the tests during development:** the diff memo was shared
  across crates while the `own` label depended on which crate was being
  compared, so `fa-util +wide` came out as a workspace dependency. The fix
  labels `own` at the top level only, and the memoized diff is now
  crate-independent.
- **Known limitation:** a dependency's feature difference that is caused by
  the divergent crate's own feature (`feat = ["dep/x"]`) is listed as an
  extra root beside the `own` cause rather than folded into it.

**Tests:** `scripts/feature-attribution-tests.rs`, compiled as the bin's
`#[cfg(test)] mod tests` and selected by L1. No tier markers. Fixture reads
use `repo_root().join("scripts/ci/fixtures/…")`.

| Requirement | Test |
|---|---|
| own-feature cause | `own_feature_cause_names_the_crate_and_flag` |
| workspace-dependency cause | `workspace_dependency_cause_is_the_dependency_not_the_dependent` |
| third-party cause, re-identifying every crate above it | `third_party_cause_reaches_every_workspace_crate_above_it` |
| every configuration beyond the first explained; origin split | `every_configuration_beyond_the_first_is_explained` |
| divergent fixture mirroring `one_owner_tree_shares_a_dependency_compile_without_unifying_features` (same `shared-deps` workspace) | `a_deliberately_divergent_dependency_is_two_configurations` |
| `(crate, version)` keying trap | `two_versions_of_one_crate_are_not_a_divergence` |
| root display trap | `a_root_package_shares_the_configuration_its_dependents_build` |
| seconds join, harness exclusion, unexplained compiles | `timed_pass_seconds_are_charged_to_the_configurations_an_owner_built_first` |
| tree parsing: `(*)`, repeated leaves, dev/build headers, host inference | `parse_tree_links_repeated_and_marked_packages_to_one_node`, `parse_tree_marks_proc_macros_and_build_dependencies_as_host_side`, `parse_package_reads_version_source_and_proc_macro_marker` |
| malformed input | `parse_tree_rejects_malformed_input`, `owners_are_required` |
| JSON round trip, rendered output | `the_json_report_round_trips`, `the_rendered_report_names_each_cause` |

New fixture: `scripts/ci/fixtures/feature-attribution/`, a standalone
workspace with a committed `Cargo.lock` and `vendor/` path crates excluded from
the workspace so that they classify as third-party. It is resolved by
`cargo tree` only and never compiled.

**Model check against the analyzed CI job:** on `x86_64-unknown-linux-gnu`
the ten-package selection resolves 27 workspace crates in **92**
configurations. The analyzed job compiled 28 distinct workspace crates **89**
times at an earlier tree (before `test-fixtures` joined `claudine-cli`'s CI
features). The model and the observation agree to within the tree drift.

### Wave 3 — timed pass (Task 1.7)

- **Storage check:** `/Volumes/coding` had 739 GiB free before the pass.
- **Script:** `fixes/2026-09-21-ci-build-feature-divergence/timed-pass.sh`.
  The ten owners run in the producer's order (sorted by package name, which
  is what `produce` does; `just ci-local --plan` was not needed to learn it).
  Each gets `cargo test --no-run --profile test --target
  aarch64-apple-darwin --package <owner> --features <ci features>`, which is
  the compile `cargo nextest archive` runs, followed by its sidecar
  `cargo build --profile test --package … --bin …` builds from
  `.github/ci/sidecars.json`.
  - A warm pass came first (the dependency cache's role). Then every
    workspace crate's `.fingerprint`, `build`, `incremental`, and `deps`
    artifacts were purged (a fresh checkout's role). Then came the measured
    pass.
  - All builds used one dedicated tree, `target/feature-attribution-timing`,
    so the host-global `kache` wrapper and this worktree's everyday builds
    were not mixed into it. `CARGO_INCREMENTAL=0` and `KACHE_DISABLED=1`
    were set. Every rustc was timed by the `ci-build` wrapper
    (`RUSTC_WRAPPER`, `BISCUIT_CI_BUILD_WRAP=1`, owner label in
    `BISCUIT_CI_BUILD_PACKAGE`), the same event mechanism the CI owner uses.
  - Build only. No test executed and no terminal or browser window opened.
- **Fidelity caveat:** these are macOS seconds per rustc process, not Linux
  runner seconds. The CI job summed 1,674 s of owner build time; the local
  measured pass took 389 s of wall time (warm pass 621 s). Linux seconds in
  the table are estimates that weight Linux configurations with macOS
  per-crate medians.
- **Persisted** under `attribution-data/`: `timed-pass-events.json` (165
  measured-pass events), `timed-pass-owner-seconds.txt` (warm and measured
  wall seconds per owner), and the JSON reports. The raw target tree is not
  kept.
- **First run failed** on a `CDPATH`-polluted `$(cd … && pwd)`. The script
  now unsets `CDPATH`, and the rerun was clean.

### Wave 4 — attribution table and reading (Task 1.8)

Published: `attribution-2026-09-21.md`, generated by
`attribution-data/render-attribution.py` from the JSON reports. It covers
the selection, the tree (`6c9ee2e60`), the method, totals, the seconds
split, per-crate seconds, the ranked causes, the `--align` what-ifs, the
accounting for the spec's thirteen rows, and every configuration on the
Linux target. **Every configuration beyond a crate's first has a cause**
(92 configurations / 65 divergent on Linux; 95 / 68 on macOS). The timed
join left no unexplained compile.

While writing the table, `feature-attribution` gained `sole_seconds` and
`involved_seconds` per cause, plus an `--align <crates>` what-if that
refuses workspace crates. Both were needed to answer "is the alignment crate
warranted". Tests: `co_causes_split_seconds_and_bound_what_each_removes`,
`a_workspace_crate_is_never_aligned`, and
`aligning_a_fixture_flag_collapses_the_configurations_it_caused`. The latter
runs end to end through real `cargo tree` on the fixture.

#### One-page reading

**Split (acceptance criterion 1), in compile seconds.** Of the 297.9 s the
macOS timed pass spent on configurations beyond a crate's first,
**third-party-inherited divergence is 277.1 s (93%)** and **workspace-own
divergence is 20.8 s (7%)**: own features 12.2 s plus a workspace
dependency's features 8.7 s. The Linux estimate is the same shape, ≈275 s
versus ≈23 s (92% / 8%). The ruling-protected workspace-own divergence
(`biscuit-file/fetch`, `biscuit-hash/blake3`,
`darkmatter/effects-instrumentation`, `renderable/hint-access-counter`, …)
is cheap. The cost is inherited.

**Where it lands.** Three crates carry 80% of the divergent seconds:
`schematic-definitions` 146.1 s (49%; 7 configurations at about 24 s each),
`sniff` 46.0 s (15%), and `darkmatter` 45.7 s (15%).

**The spec's premise does not hold.** No third-party flag is the sole cause
of more than 1.8 s. The divergence arrives in bundles: a `claudine-cli`
configuration differs on 10–25 flags at once, and a `sniff-cli` one on
`libc`, `errno` (macOS), `mio`, and `crossterm` together. Collapsing a
configuration needs *every* co-cause aligned. The `--align` what-if
therefore decides the extent:

| Aligned | Seconds removed (macOS timed) | Linux estimate |
|---|---|---|
| `libc`, `proc-macro2`, `serde_core` (the spec's eligible set) | 0.6 s | ≈1 s |
| + `mio` (+ `errno` on macOS); `mio` is ruling 4's "default no" | 36.9 s | ≈36 s |
| every third-party crate outside the "default no" set | 49.4 s | ≈82 s |
| every divergent third-party crate (hakari-like) | 210.8 s | ≈214 s |

**`proc-macro2/span-locations` is not the "likeliest single largest cause".**
The owner passes `--target`, so the dev-dependency's flag reaches only the
target-side `proc-macro2`. It re-identifies six workspace crates that link
`syn`/`proc-macro2` as normal dependencies, never the derive macros. Its
sole seconds are 0.

**Recommended extent of part 2 (for the author's ruling, see
`human_review_items` in the spec):**

- *Alignment crate:* **not warranted** on the spec's eligible terms, which
  remove about 1 s. It only pays when it also aligns flags ruling 4 marks
  "default no" (`mio`, `hyper`, `tower`, `time`, `tokio-stream`, `clap`,
  `crossterm`) and a long tail of others, which approaches `cargo-hakari`.
  The spec rejected that as a first step.
- *Ruling 1's fallback:* the trigger ("the alignment crate is not warranted
  at all") has fired. But Option 2 (a planner watch plus isolating the guard)
  would also remove 0 s, because `span-locations` is never a sole cause. The
  proportionate outcome is to leave `proc-macro2/span-locations` divergent
  with no isolation and no planner change. That departs from the recorded
  ruling, so it needs the author's amendment.
- *Per-flag remedies:* `libc/extra_traits` leave divergent (source removal
  blocked upstream; alignment alone 0.6 s). `serde_core/default` leave
  divergent (no removal path; alignment alone ≈0 s). The "default no" set
  stays divergent (ruling 4). Workspace-own rows stay divergent by design.
- *The lever this spec did not list:* `schematic-definitions`, 55k lines of
  API definitions with light direct dependencies, reaches the whole
  runtime stack (`biscuit-file` → `gix`/`reqwest`/`tokio`/`mio`/`libc`)
  through one incidental edge. `schematic-define`'s `openapi` feature
  enables `biscuit-file` (`yaml`) only for its re-exported `serde_yaml_ng`
  (`schematic/define/src/openapi/{options.rs,import/builder.rs}`). Depending
  on `serde_yaml_ng` directly would take `schematic-definitions` out of most
  of those bundles: a "remove the source" remedy on a workspace edge. The
  upper bound is its 146 s. It keeps some divergence through `serde_core`,
  `serde_json`, and `indexmap` flags, so the true saving is lower, and it is
  not yet measured.

**The spec's escalation trigger (S1).** Eleven rows appeared beyond the
spec's thirteen, among them `smallvec +serde`, `num-traits +libm`,
`getrandom`, and `tokio +signal`, which sit fairly low in the graph. None
changes the conclusion above. They are part of the bundles, and the
all-third-party what-if already counts them.

### Phase 1 validation

- `just test repo-deps` (macOS): **467 passed, 1 skipped** (the skip was
  already there before this change). That includes all 18
  `feature-attribution` tests and
  `one_owner_tree_shares_a_dependency_compile_without_unifying_features` with
  the rest of `scripts/ci-build-archive-tests.rs`, all unchanged.
- `just _lint repo-deps` (the per-package clippy `just lint` runs,
  `--all-targets -D warnings`): clean. It needed one fix: a `Selection`
  struct replaced a `type_complexity` tuple.
- `just cross-check repo-deps --os linux` (build-linux, from an
  ubuntu-latest archive): **467 passed, 1 skipped**.
- `just cross-check repo-deps --os windows` (build-win-native): all 18
  `feature-attribution` tests pass. 3 `ci-build` archive tests fail for
  rig-environment reasons in code this phase did not touch:
  `every_build_record_the_shipped_planner_writes_round_trips_through_this_reader`
  and `every_real_build_record_declares_only_payloads_the_producer_can_emit`
  fail with "python3 is required", and
  `one_owner_tree_shares_a_dependency_compile_without_unifying_features`
  fails because `produce-owner.sh` could not execute the `.exe` wrapper
  through bash. Windows runs after merge (push to `main`), and these are not
  caused by this change.
- GitNexus `detect_changes` (scope `all`): risk low, 0 affected processes.
  The new files are untracked, so they are not in its diff.
- No CI run was triggered. No test executed during the timed pass.
- `docs/dependencies.md` is untouched: no crate or dependency edge was added.
  The `rust-devops` skill (`ci-cd.md`) records the finding and how to read
  the table (acceptance criterion 7, first half), and lists
  `feature-attribution` among `repo-deps`' binaries.
- **Stopping for human review** (spec `human_review: true`): the evidence
  says the planned alignment crate saves about 1 s, so the author should
  choose Phase 2's direction before any remedy work.

## Phase 2

Base: `fix/ci-build-feature-divergence` at `9ac58f1c8`, macOS host. The
`rust-devops` skill was loaded before any Phase 2 work. No CI run was
triggered. No manifest, lock file, or source file is changed by this phase:
the one manifest edit below was a temporary what-if, reverted in the same
step (`git status` clean afterward).

**Author ruling status.** Phase 1 stopped with `human_review: true` and two
open items (the direction of part 2, and the amendment to the Open Questions
ruling). Neither had been answered when Phase 2 started. Task 2.1 records
decisions and changes no source, so it follows the recorded rulings and marks
every row that needs the author with **pending ruling**. Nothing that needs
the ruling was executed: no isolation, no alignment, and no option B edit.

### Sizing option B (the `schematic-define` → `biscuit-file` edge)

Phase 1 gave option B only a 146 s upper bound. It is now measured, because
it is the only candidate large enough to change the outcome.

- **What-if edit** (temporary, reverted): in `schematic/define/Cargo.toml`,
  the `openapi` feature's `"biscuit-file"` became `"dep:serde_yaml_ng"`, and
  the optional `biscuit-file = { …, features = ["yaml"] }` dependency became
  `serde_yaml_ng = { version = "0.10", optional = true }` (the same version
  `biscuit-file` re-exports). `use biscuit_file::serde_yaml_ng;` became
  `use serde_yaml_ng;` in `src/openapi/options.rs` and
  `src/openapi/import/builder.rs`. `cargo check -p schematic-define
  --all-features` compiled the library. The integration test
  `tests/openapi_tests.rs` also imports `biscuit_file::serde_yaml_ng`, so the
  real change touches that one line too (or keeps `biscuit-file` as a
  dev-dependency; its dev edge is only built when `schematic-define` is an
  owner, which it is not in this selection).
- **Linux model** (`feature-attribution --target x86_64-unknown-linux-gnu`):
  92 → **86** configurations (65 → 59 divergent). `schematic-define` and
  `schematic-definitions` each go **7 → 4**. The four left are `claudine`'s
  (now shared with `claudine-cli`'s sidecar build, `claudine-gen`,
  `darkmatter`, `darkmatter-cli`, `dmls`, and `sniff-cli`), plus
  `claudine-cli` (`serde_core +default`), `repo-deps`, and `sniff` (both
  `serde_json` feature differences).
- **macOS seconds** (joined with the Phase 1 timed-pass events): the model no
  longer predicts four measured compiles: `darkmatter`'s and `sniff-cli`'s
  builds of both crates, **52.1 s**. The join also hides a fifth. `claudine-cli`
  compiled both crates twice (23.7 s + 1.2 s for its sidecar
  configuration), and that configuration now equals `claudine`'s. The join
  charges an owner's compiles to the configurations it built first, so it
  still counts both. Removed in total: **≈77 s**, about 26% of the 297.9 s
  divergent seconds.
- For comparison, on the same measured seconds: the spec's eligible trio
  0.6 s; trio + `mio` + `errno` 36.9 s; every third-party crate outside the
  "default no" set 49.4 s; every divergent third-party crate (hakari-like)
  210.8 s.

Option B keeps ruling `2026-09-12-single-os-compile` whole. It unifies no
feature. It deletes one incidental workspace edge whose only purpose was a
re-export, so it is a "remove the source" remedy. The spec lists source
removal only for third-party chains, so option B still needs the author's
ruling and a spec line before it lands.

### Decision table (Task 2.1)

Seconds are macOS timed, split evenly between a configuration's causes
(**split**), with **sole** being what removing that cause alone is certain
to save. Configuration counts are macOS / Linux. Shapes: *member-declared*
(a workspace manifest names the flag), *transitive* (a third-party crate
some owners hold enables it), *workspace-own* (a workspace crate's own
features). Isolation applies only to the member-declared shape. Source
removal is tried before alignment on every row.

#### The spec's three base flags

| Crate / flag | Shape | Configs | Split / sole | Remedy | Rationale |
|---|---|---|---|---|---|
| `libc` `+extra_traits` | transitive: `claudine-cli` and `sniff-cli` via dev-dep `expectrl 0.8` → `nix 0.26.4`; `repo-deps` via `ctrlc` → `nix 0.31.3` | 38 / 31 | 34.5 s / 0.6 s | **leave divergent** | Isolation does not apply (no member declares it). Source removal is blocked upstream (ruling 3: `expectrl 0.9.0` still needs `nix ^0.26`), and `ctrlc` would keep the flag even without `expectrl`. Alignment would save 0.6 s, because the flag always arrives with `mio`/`errno` (and with `claudine-cli`'s 10–25-flag bundle), so it collapses no configuration alone. |
| `proc-macro2` `+span-locations` | member-declared: `claudine/cli` dev-dependency for the source-scan guard | 0 / 7 | 0 s / 0 s | **leave divergent — pending ruling** | Ruling 1 aligns it only "whenever the alignment crate is created". Phase 1 shows the crate is not warranted, which triggers ruling 1's Option 2 fallback (isolate the guard plus a planner watch). But the flag reaches only the target-side `proc-macro2` and is never a sole cause (0 s), so Option 2 would add a planner-contract change for no saving. Option 2 is not executed. This phase has no isolation task, and acceptance criterion 2 bars isolation work without a recorded ruling that fits. The author's amendment is the open review item. |
| `serde_core` `+default` | transitive: `claudine-cli` via `axum 0.8.9`; `repo-deps` via `camino` ← `cargo_metadata` | 33 / 31 | 13.9 s / 0 s | **leave divergent** | No source-removal path: `axum` is in `claudine-cli`'s closure by design (the rendezvous daemon), and `cargo_metadata` is `repo-deps`' core input. Alignment would save 0 s alone. |

#### Third-party flags in ruling 4's "default no" set

All **leave divergent** (ruling 4). No per-flag ruling is recorded in the
spec, which ruling 4 requires before any of them is aligned. Isolation does not
apply (all transitive or ordinary normal dependencies). Source removal does
not apply: each is a capability its owner uses (the daemon's HTTP server,
terminal event handling, CLI parsing).

| Crate / flags | Configs | Split / sole | Owners that add it |
|---|---|---|---|
| `mio` `+default +log` | 17 / 17 | 18.2 s / 0 s | `claudine-cli`, `sniff-cli` |
| `hyper` `+server` / `+full` | 20 / 19 | 13.2 s / 0 s | `claudine-cli`, `darkmatter` |
| `hyper-util` `+server +server-auto +service` | 20 / 19 | 13.2 s / 0 s | `claudine-cli`, `darkmatter` |
| `clap`, `clap_builder` `+env +unstable-ext +wrap_help` | 10 / 10 each | 7.1 s / 0 s each | `claudine-cli` (and others without `env`) |
| `time` `+local-offset` | 11 / 11 | 4.1 s / 0 s | `claudine-cli` |
| `tower` (twelve flags) | 11 / 11 | 4.1 s / 0 s | `claudine-cli` |
| `crossterm` `+events …` | 6 / 6 | 3.3 s / 0 s | `claudine-cli`, `sniff-cli` |
| `tokio-stream` `+net` | 7 / 7 | 1.5 s / 0 s | `claudine-cli` |

`mio` is the only one with a material bundle effect: aligning it with the
trio and `errno` removes 36.9 s. That is the spec's option D, and it needs a
ruling 4 entry in the spec first.

#### Remaining third-party flags

All **leave divergent**. None is on the spec's eligible list, so each would
need its own spec ruling (ruling 4, last sentence), and several add
observable behavior (`tokio +signal/+test-util`, `getrandom +js`,
`reqwest` feature sets, `rustls` crypto provider). Isolation does not apply,
and each arrives as an ordinary dependency feature of a crate its owner uses.
Aligning every one of them together with the trio removes 49.4 s (macOS) /
≈82 s (Linux estimate), less than option B alone on macOS.

| Group | Crates (macOS split seconds) | Likely origin (inferred from which owners diverge, not traced per crate) |
|---|---|---|
| `errno` (macOS only) | `errno` 27.0 s | `sniff-cli`/`claudine-cli` terminal stack (`default`) versus others (no default) |
| async runtime | `tokio` 16.2 s, `tokio-util` 3.1 s, `bitflags` 1.5 s (macOS), `slab` 0.2 s, `futures-sink`, `futures-util`, `futures-io` | per-owner `tokio` feature sets (`+signal`, `+full`, `+test-util`, `-fs -process`) |
| `claudine-cli`'s daemon bundle | `num-traits` 7.4 s, `smallvec` 5.2 s, `getrandom` 4.0 s, `xxhash-rust` 1.8 s, `num-integer` 1.0 s, `half` 0.8 s, `num` 0.7 s, `fastrand` (host) 0.7 s, `syn +visit` 0.1 s | `rendezvous-daemon`/`duckdb`, the guard test, and `biscuit-hash`'s `xxh32` under the CI features |
| JSON / text | `serde_json` 8.9 s, `either` 6.2 s, `url` 4.9 s (the only sole seconds here: 1.8 s), `ttf-parser` 4.7 s, `bytecount` 4.7 s, `memchr` 1.7 s, `regex-automata` 1.3 s, `regex-syntax` 1.1 s, `regex` 1.0 s, `aho-corasick` 0.2 s | `repo-deps`' `serde_json +unbounded_depth`, `sniff`'s narrower closure, and `test-toolkit`/harness no-default builds |
| TLS / config / ids (`claudine-cli` sidecar, `darkmatter-cli`, `dmls`, `claudine-gen` no-default closures) | `hyper-rustls`, `rustls`, `rustls-webpki`, `hashbrown`, `uuid` 3.1–3.3 s each; `toml_parser`, `toml_datetime`, `toml_writer`, `winnow` 3.2 s each; `which`, `bit-set`, `bit-vec` ≤1.2 s | builds that lack `biscuit-file/fetch`'s `reqwest`/`gix` stack drop these features |
| compression / unicode / OS | `rustix` 3.8 s, `tinystr`, `zerovec`, `simd-adler32`, `miniz_oxide` 3.2 s each, `objc2-core-foundation` 0.8 s (macOS), `linux-raw-sys` (Linux) | `sniff` and `sniff-cli`'s narrower closures |
| HTTP client / misc | `tower-http`, `reqwest` 0.8 s each; `bytemuck`, `num-rational`, `num-bigint`, `once_cell`, `serde`, `indexmap`, `chrono`, `sha1`, `form_urlencoded`, `gix`, `gix-*` ≤0.2 s each | `sniff -network -remote` (under `repo-deps`), `biscuit-terminal -image` |

#### Workspace-own divergence (ruling-protected)

All **leave divergent by design**. These are exactly what
`2026-09-12-single-os-compile` protects, and no row aligns a workspace
crate's own feature. Total 20.8 s (7%).

| Crate / flag | Kind | Split / sole |
|---|---|---|
| `darkmatter` `+effects-instrumentation` (`claudine-cli`), `+browser-tests +terminal-tests` (`darkmatter`), `+work-counters` (`dmls`) | own | 8.2 s / 7.0 s |
| `biscuit-file` `±fetch` (`biscuit-file`, `repo-deps`, `sniff`) | own 0.2 s; as a workspace dependency 6.5 s | — / 0 s |
| `renderable` `+hint-access-counter` (`darkmatter`) | own 1.9 s; as a workspace dependency 1.2 s | 1.9 s sole |
| `darkmatter-cli` `+terminal-tests` | own | 1.3 s / 1.3 s |
| `biscuit-hash` `-blake3` (`sniff`) | own 0.1 s; as a workspace dependency 0.6 s | 0.1 s sole |
| `darkmatter` `±effects-instrumentation` as a workspace dependency | workspace dependency | 0.4 s |
| `sniff` `-network -remote` (`repo-deps`), `biscuit-terminal` `-image` (`repo-deps`), `test-toolkit` `+backend-proof` | own | ≤0.2 s each |

**Missing-declaration hazard (report-only):** none found. Each owner is a
separate Cargo invocation, so a package's own archive never resolves another
owner's workspace features. Every workspace-own row above is a feature its
own owner (or its own dependency chain) declares. Nothing was changed.

#### New remedy, outside the spec's list

| Edge | Shape | Configs removed | Seconds removed | Remedy | Rationale |
|---|---|---|---|---|---|
| `schematic-define` `openapi` → `biscuit-file` (`yaml`), used only for the re-exported `serde_yaml_ng` | workspace edge (incidental) | 6 on Linux (3 each for `schematic-define` and `schematic-definitions`); 6 on macOS | **≈77 s** macOS (52.1 s visible to the join + 25.0 s from `claudine-cli`'s duplicate compile) | **remove the source — pending ruling** | The single largest lever, and it unifies nothing. Measured above. Not executed, because the author has not ruled and the spec does not yet list the remedy. If approved, it runs as a Task 2.2 change with the manifest edit, three `use` lines, `schematic/docs/dependencies.md` (plus the root `docs/dependencies.md` if it lists the edge), and `just test schematic-define`. |

#### Conclusion for Phases 3 and 4

- **The alignment crate is not warranted** on the spec's eligible terms
  (0.6 s). Phase 3 collapses to its no-op rationale unless the author rules
  "default no" or off-list flags eligible (options C or D).
- **Ruling 1's Option 2 fallback is triggered on paper but not executed:**
  it removes 0 s and extends the planner contract. The recommended
  amendment is "leave `proc-macro2/span-locations` divergent, guard stays in
  `claudine-cli`". This is the second review item.
- **Task 2.2:** no flag row assigns "remove the source". Every
  third-party source removal is blocked or not applicable, as recorded above.
  Option B is the only candidate, and it waits on the author's ruling.

### Phase 2 validation

- **Requirement-to-test mapping.** Phase 2 changes no behavior. It changes no
  source, manifest, or lock file, so there is no new test. The one
  measurement (option B) reused the Phase 1 tool, whose 18 tests already
  cover the `--align` what-if and the timed-events join.
- `just test repo-deps` (macOS): **467 passed, 1 skipped** (the skip was
  already there). It includes
  `one_owner_tree_shares_a_dependency_compile_without_unifying_features` and
  the rest of `scripts/ci-build-archive-tests.rs`, unchanged.
- `just _lint repo-deps`: clean.
- GitNexus `detect_changes` (scope `all`): 0 changed symbols, risk low. The
  only changed file is this log (plus plan and spec frontmatter).
- No cross-OS run: nothing compiled differs from Phase 1.
- `docs/dependencies.md` is untouched: no dependency edge moved (the option B
  what-if was reverted).
- **Stopping for human review** again (spec `human_review: true`). The two
  items from Phase 1 stand, now with option B measured at ≈77 s.

## Phase 3

Base: `fix/ci-build-feature-divergence` at `0760976a0`, macOS host, toolchain
1.98.1. No CI run was triggered. No source, manifest, or lock file is changed
by this phase. The one manifest edit below was a temporary check, reverted
with `git checkout` in the same step (`git status` clean afterward).

**Author ruling status.** Both `human_review_items` from Phases 1–2 were still
unanswered when Phase 3 started. Neither changes this phase's outcome:

- The plan says Phase 3 "exists only if Task 2.1 concluded the alignment crate
  is warranted". Task 2.1 concluded it is not. The spec's eligible entries
  (`libc`, `proc-macro2`, `serde_core`) remove 0.6 s.
- Options A and B both leave the crate unwarranted.
- Options C and D need a ruling-4 entry in the spec before any alignment
  entry is written (ruling 4, and plan Task 3.4's last bullet). No such entry
  exists, so an agent cannot choose them.

So Phase 3 collapses to its no-op rationale under every ruling the author
could give without first writing a new spec entry. Option B is a Task 2.2
change and still waits on the author. It was not executed here.

### Task 3.1 — publish interaction (ruling 7): does not block

This was run even though no fan-out follows. It is a cheap check with no
side effects, and it is the one technical prerequisite that options C and D
would hit. 71 of the 74 workspace members are publishable (no
`publish = false`), so the trap was real.

Temporary edit: a `tools/feature-alignment` crate (`publish = false`, one
`libc` entry with `extra_traits`, empty `src/lib.rs`) added to the root
`members`, and wired into `biscuit-hash/lib/Cargo.toml` as
`[dev-dependencies.feature-alignment] path = "../../tools/feature-alignment"`
(no version). `biscuit-hash` was chosen because it is publishable and has only
registry dependencies, so a full verify build is possible.

| Command | Result |
|---|---|
| `cargo package -p biscuit-hash --list --allow-dirty` | succeeds; 9 files |
| `cargo publish -p biscuit-hash --dry-run --allow-dirty` | packages, verifies (compiles the packaged crate), stops at "aborting upload due to dry run" |
| normalized `Cargo.toml` inside the `.crate` | `[dev-dependencies]` lists `criterion` only; **0** mentions of `feature-alignment` (it is still in `Cargo.toml.orig`) |

Cargo strips a versionless path dev-dependency from the manifest it publishes,
so the spec's dev-only wiring does not block publishing. No ruling amendment
is needed. Reverted with `git checkout -- Cargo.toml Cargo.lock
biscuit-hash/lib/Cargo.toml`, the temporary crate and `target/package`
outputs were removed, and `git status` was clean.

(A self-inflicted slip during the revert: the backup copies of the root and
member `Cargo.toml` shared a basename and one overwrote the other. Restoring
from git, which was clean before the step, avoided any damage. Verified with
`git status` and `cargo metadata`.)

### Tasks 3.2–3.4 — closed, not built

- **3.2 (scaffold):** no `tools/feature-alignment` crate. It would be a changed
  direct dependency of every member (the fan-out the spec records) for a
  0.6 s saving.
- **3.3 (fan-out):** no member manifest or `Cargo.lock` edited, so there is no
  fan-out cost to record.
- **3.4 (entries):** none. Under ruling 1, `proc-macro2/span-locations` is
  aligned only if the crate exists, so it stays divergent. The guard stays in
  `claudine-cli`. Whether ruling 1's Option 2 fallback applies is the
  still-pending Open Questions amendment. It is not executed, for the same
  reason as in Phase 2 (0 s saved).
- `docs/dependencies.md` and the `rust-devops` skill are untouched: no crate or
  edge was added. The skill's Phase 1 note already records why alignment of
  the base trio does not pay.

If the author later rules C or D (with the ruling-4 spec entries), Tasks
3.2–3.4 run as written, and Task 3.1's result already clears their
prerequisite.

### Phase 3 validation

- **Requirement-to-test mapping.** No behavior changed, so there is no new
  test. The one check (Task 3.1) is recorded above with its commands.
- `just test repo-deps` (macOS): **467 passed, 1 skipped** (the skip was
  already there), including
  `one_owner_tree_shares_a_dependency_compile_without_unifying_features` and
  the rest of `scripts/ci-build-archive-tests.rs`, unchanged.
- `just _lint repo-deps`: clean.
- GitNexus `detect_changes` (scope `all`): risk low, 0 affected processes; only
  `plan.md` sections are touched (plus this log and spec frontmatter).
- No cross-OS run: nothing compiled differs from Phase 1.
- **Still stopped for human review** (spec `human_review: true`). The two items
  stand. Phase 4's content depends on whether option B lands first.
