---
kind: rulings
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 1
decided_by: claude/opus (implementing agent), adopting the plan's recommended defaults except where evidence below amends them
author_override: any ruling may be overridden by the author before the phase that first depends on it; each ruling names that phase
---

# Phase 1 rulings

Each ruling states the decision, the evidence behind it, and what it binds
later phases to. Where Phase 1 evidence contradicted the plan's recommended
default, the ruling says so and records the amended decision. No ruling is
left at "recommended".

## R1 — Toolkit home and language

**Decided: Python, `scripts/ci/consolidation.py`**, with subcommands
`inventory`, `plan`, `capture`, `compare`, `check-attributes`, and
`check-snapshots`, plus a pytest suite `scripts/ci/test_consolidation.py`.
This matches the `affected_scope.py`/`completion.py` house pattern.
Consequences:

- It runs only locally and on build hosts. No workflow invokes it, so no CI
  cell, matrix entry, or gate is added (spec §Out of scope).
- The Phase 1 evidence scripts (`baseline/capture-listings.sh`,
  `baseline/inventory.py`, `baseline/consumer-sweep.py`, `spikes/s2-scan.py`)
  are its prototypes. Phase 2 folds their logic in, and must reproduce
  `baseline/inventory.json` and the listings from the unmigrated tree as its
  no-op self-proof. The prototypes stay as the record of how the baseline was
  produced. They are not a second implementation to maintain.
- Tier expressions are obtained by running `just _tier_filter <tier> <pkg>`
  (with `BISCUIT_TEST_FILTER` and `BISCUIT_L1_INCLUDE_SLOW` explicitly
  controlled), exactly as `capture-listings.sh` does.

## R2 — Target names, directory layout, neutral module aliases

**Decided: as recommended, with two amendments from S1/S2.**

- **Target names:** `l1`, `level2`, `level3`, `browser`, `real`. When one tier
  has more than one feature contract in a package, each gets a
  feature-suffixed name: darkmatter's `level3-terminal` (`terminal-tests`) and
  `level3-browser` (`browser-tests`). Target names never affect `test(...)`
  selection (S1 §2), and no recipe, config, or workflow uses a `binary(...)`
  filter today (checked with `git grep`).
- **Layout:** `tests/<target>/main.rs` plus the moved files as sibling
  modules, with one shared `tests/common/` per package that stays where it is.
  Each root that needs it declares `#[path = "../common/mod.rs"] mod common;`
  once. `common`'s own children keep resolving inside `tests/common/`
  (verified, S2). Helper directories used by one moved file
  (`*_support/`, `image_test_support/`, `level2_render_tree_terminal/`,
  `error_guards/`) move beside that file under `tests/<target>/`, so its
  existing `mod`/`#[path]` spelling keeps working. A helper used by more than
  one file stays at `tests/` and is declared once at the root.
- **Nested crate root:** darkmatter's `tests/error_snapshots/main.rs` becomes
  `tests/<target>/error_snapshots/mod.rs`, with its 17 children unchanged.
  Renaming it to `mod.rs` keeps every child's resolution. Its contract is the
  L1 contract (no required features), so it joins `l1`.
- **Module names:** the former target name, **unless** that segment would
  change any test's verdict under (a) any `just _tier_filter` expression, or
  **(b, amendment) any `filter` in `.config/nextest.toml`
  overrides**. Several overrides use *unanchored* regexes
  (`test(/level2_/)`, `test(/browser_/)`), which can match a segment that
  the anchored tier filters do not.
- **Alias form (amendment):** the former name with its marker prefix
  removed (`level2_errors` → `errors`), with `_module` appended if that
  collides. An alias that merely prefixes the name (`x_level2_errors`) is
  rejected, because it still matches the unanchored override regexes.
- **Collisions (S1 §5):** a module name may not equal another module or a
  root-declared helper (`common`, a helper directory) in the same target. The
  manifest rejects it, and `compare` fails on any duplicate projected
  identity.
- **Evidence:** `baseline/inventory.md` §Module-name hazard and §Feature-gated
  targets. **Exactly one alias is required at this revision:**
  darkmatter-cli's `level2_harness_integrity`. It is gated on
  `terminal-tests`, but none of its four tests carries `level2_`, so all four
  run in **L1** today (CI's darkmatter-cli L1 cell builds with
  `terminal-tests`, which is why its L1 count is 578 there vs 574 without the
  feature). The feature boundary puts it in the `level2` binary. As
  `mod level2_harness_integrity;` its tests would silently move to L2, so it
  becomes `mod harness_integrity;` and keeps its L1 placement inside the
  `level2` binary. Every other marker-named target holds only tests that
  carry the same marker. Three claudine-cli targets are cfg-absent on macOS
  (`level2_windows_provided_partial_file_capture`,
  `level3_linux_sequence_ctrl_c`, `level3_windows_sequence_ctrl_c`). Their
  source `#[test]` names all carry the marker, so they are provisionally
  alias-free, to be confirmed from the Linux/Windows on-host listings in
  Phase 3. The planner must still run the projection, because the result is
  per-revision.
- **Consequence for target naming:** a consolidated binary is named for its
  feature contract's dominant tier, but tier selection stays per test.
  `level2` in darkmatter-cli will hold four L1-tier tests, as
  biscuit-terminal's `*_parity` files already mix L1 and browser tests
  inside one L1 target today.

## R3 — What counts as a target-wide setting

**Decided: as recommended.** A target-wide setting is any per-`[[test]]` key
Cargo honors other than `name`/`path`: `harness`, `required-features`,
`edition`, `test`, `doctest`, `bench`, `doc`, `proc-macro`, `crate-type`.
It also includes any crate-root-only attribute or crate-global construct
from S2's detector list.

Inventory outcome (proven, not assumed): the only such key in use is
`required-features`. No test target sets `harness`, `edition`, or any other
key. S2 found zero crate-root-only attributes and zero crate-global
constructs. The execution contracts are therefore exactly
`required-features` × tier:

| Package | Contracts found | Expected consolidated targets |
|---|---|---|
| `claudine-cli` | none (102), `terminal-tests` (34: 29 `level2_`, 5 `level3_`), `real-tests` (3) | `l1`, `level2`, `level3`, `real` |
| `darkmatter` | none (70), `terminal-tests` (`level2_render_tree_terminal`, `level3_image_painting`), `browser-tests` (`browser_render`, `level3_popover`) | `l1`, `level2`, `level3-terminal`, `level3-browser`, `browser` |
| `darkmatter-cli` | none (42), `terminal-tests` (11 `level2_`, of which `level2_harness_integrity` holds only L1-tier tests; see R2) | `l1`, `level2` |
| `biscuit-terminal` | none (37), `terminal-tests` (1 `level2_`) | `l1`, `level2` |

The first-draft concern about darkmatter's 16 `harness = false` entries is
settled: they are `[[bench]]` targets and do not move.

## R4 — `slow_` Level 1 tests

**Decided: `slow_` tests stay inside the L1 consolidated binary in every
package.** Selection is filter-based: `BISCUIT_L1_INCLUDE_SLOW` changes the
`-E` expression, not the build. Darkmatter's before-listings are captured
under both slow-policy states.

**Correction to the plan's premise:** at this revision there are **zero**
`slow_`-marked tests in the four packages' **integration-test targets**. The
claudine `wrap_sigint` case the plan cites was renamed earlier.
`claudine/cli/tests/wrap_sigint.rs:11` documents that the `slow_` prefix was
dropped because it kept the test out of every recipe. The only `slow_` test
in scope is a darkmatter **lib unit test**,
`markdown::compose::tests::rendering::slow_compose_cleanup_preserves_quoted_marker_looking_indented_code`.
Consolidation does not move it. It is the single difference between
darkmatter's two slow-policy listings (6,494 vs 6,495 selected under L1, for
every feature set), which makes it a free control: the after-state
comparison must show exactly the same one-test difference. The ruling still
binds, because a future integration `slow_` test must not create a new
binary.

## R5 — Static test-identity consumers

**Decided, amended.** The recommended premise ("the CI skip baseline is the
only static identity store") is **not true**. Phase 1 found four classes of
committed state that name test identities:

1. **`.github/ci/ci-baseline.toml`** is empty at this revision (no entries).
   The per-package checkpoint confirms it is still empty, or translates each
   entry through the manifest.
2. **`.config/nextest.toml` override filters with exact test names.** Two are
   in scope:
   `test(=compose_loop_rate_limit_pause_waits_then_continues)` (claudine-cli,
   `loop_cli.rs`) and `test(=every_catalog_variable_survives_ambient_options)`
   (darkmatter, `ambient_ctx_capture.rs`). Each appears in both the `default`
   and `ci` profiles. After the move, the test path gains a module prefix and
   the `=` filter silently stops matching, so the test loses its extended
   `slow-timeout`. **Binding:** each package migration rewrites its own
   exact-name overrides to the new path in the same change. `capture` also
   lists every override `filter` read from `.config/nextest.toml` (read
   verbatim, never re-derived), and `compare` requires each override to select
   the same normalized identities before and after. The other exact-name
   overrides (`level2_render_tree_style_in_wezterm` in `biscuit-terminal-cli`,
   `test_detect_completes_in_reasonable_time` in `sniff`) belong to packages
   outside this feature.
3. **Committed artifacts that embed a `--test` selector string:**
   `claudine/docs/providers/dispatch-inventory.json` (`"regenerate"` field,
   written from `dispatch_inventory.rs`'s `REGEN_COMMAND` and compared by that
   test). See R9.
4. **Frozen test fixtures** (`tools/test-audit/fixtures/claudine-compat/families.json`,
   `tools/test-audit/fixtures/junit/*.xml`) and completed-spec attribution
   files name `claudine-cli::dispatch_inventory`-style identities. These are
   historical corpora for other tools' tests. They are **not** rewritten.

Expected manifests and completion records are derived per run by
`_expected_manifest` from the live tree, and JUnit artifacts are historical,
so no translation layer is built. That part of the recommendation stands.

## R6 — Measurement protocol

**Decided: as recommended.** The protocol is written into `measurements.md`
§Protocol and applied by `spikes/s3-measure.sh`. In brief:

- pinned `rust-toolchain.toml` (1.98.1), host triple, `test` profile (dev),
  the canonical recipe's feature set, default linker, default Cargo job count
  (`hw.ncpu`)
- `RUSTC_WRAPPER` unset (kache off) and a fresh, separate `--target-dir` per
  clean build
- a Sniff host/load snapshot per series; peak RSS via `/usr/bin/time -l`
- five alternating before/after edit-one-test trials after one warm-up,
  reporting median and slowest, with the §8 guardrail (split only if the
  median rises by both >50% and >5 s, or the target will not compile
  reliably within runner memory)

**Recorded deviation:** "otherwise idle host" is best effort on this
developer host. Each series records the Sniff load snapshot, and the
before/after comparison is valid only when both series ran under comparable
load. `measurements.md` states the load for each series.

## R7 — Sequencing and status

**Decided: as recommended.** The spec `status` moves `draft-spec → planned`
in this phase, which is the only production-file change. The `claudine-cli`
pilot lands before any implementation of
`2026-09-21-ci-build-feature-divergence`. Each package migration is its own
reviewable commit series: inventory + manifest, structural move, verification
evidence, then docs. The implementing agent never moves the spec to
`_completed`.

## R8 — Parallel-rollout boundary

**Decided: as recommended.** Migrations and their reviews are sequential.
Before-listing capture for the next package may run during the previous
package's remote-verification wait, because listings are read-only evidence
against an untouched tree. Phase 1 already captured all four packages'
before-listings, so this is exercised from the start. The next package's
capture must be **re-run** if any of its test sources change before its
migration begins. The before side must describe the tree that is actually
migrated.

## R9 — Identity-bearing strings inside test code (new)

S2 and the consumer sweep found strings in **test code** that name a test
path or a `--test <old-binary>` selector. Spec §3 forbids test-body edits
beyond the structural list, while acceptance 11 requires active guidance to
stop recommending `--test <old-binary>`. Each case is decided separately:

| Site | Kind | Decision |
|---|---|---|
| `darkmatter/lib/tests/level2_render_tree_terminal/support/mod.rs:432` (`--exact public_entry_points::level2_render_probe_entrypoint`) | self-re-exec **identity** that the move changes | **Permitted structural edit** ("repair identities that changed because the source moved", the same class as a path repair). Prefix the new module path. Phase 4 must show the repair is load-bearing: run the L2 test once with the old string and confirm it fails. Otherwise it is not yet known whether a missed repair fails or passes silently. |
| `claudine/cli/tests/dispatch_inventory.rs:94` (`REGEN_COMMAND`, serialized into the committed `claudine/docs/providers/dispatch-inventory.json`) | developer instruction, **persisted** | **Update in the package migration as a separate commit** (docs pass): change the constant to the positional-filter form and re-bless the JSON. The committed diff must be exactly the `regenerate` line, which the reviewer checks. Leaving it stale would make the committed file recommend a selector that no longer exists. |
| `claudine/cli/tests/shipped_prompt_route_drift.rs:150` (failure-message text) and `darkmatter/lib/tests/benchmark_fixtures.rs:136` (emit-mode header comment, not compared on the verify path) | developer instruction, not an assertion input | **Update in the docs pass** of the owning package migration, as a separate commit from the structural move. |

The author may overrule the second and third rows before Phase 3 (claudine)
or Phase 4 (darkmatter). The alternative is to leave these strings stale and
record them as known-stale in `baseline/test-selector-consumers.md`. That
would contradict acceptance 11 for the persisted JSON.

## Corrections recorded against the spec and plan (no spec body edited)

- The spec says darkmatter's `tests/error_snapshots/main.rs` is an
  "explicitly declared nested target". It is **auto-discovered**: no
  `[[test]]` entry names it (`baseline/inventory.json`:
  `declared_in_manifest: false`). The migration outcome is unchanged.
- The plan says 131 of claudine-cli's files declare `mod common;`. The scan
  finds **129** (claudine-cli) + 51 (darkmatter-cli) + 7
  (biscuit-terminal) + 0 (darkmatter) = **187**, which matches the spec's
  total.
- Workspace context: 74 workspace members, 51 with integration-test targets,
  523 targets. This confirms the spec's "51 packages" (47 others + 4 in
  scope) and 523.
- There are zero `slow_` tests in scope (R4).
- The static-identity premise is widened (R5).
