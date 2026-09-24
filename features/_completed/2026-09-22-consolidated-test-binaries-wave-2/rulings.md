---
kind: rulings
feature: 2026-09-22-consolidated-test-binaries-wave-2
created: 2026-09-23
plan_phase: 1
rev: 208051f75
decided_by: claude/opus (implementing agent), adopting the plan's recommended defaults except where Phase 1 evidence below amends them
author_override: any ruling may be overridden by the author before the phase that first depends on it; each ruling names that phase
carries_forward: 2026-09-21-consolidated-test-binaries rulings R1–R9, except where restated here
---

# Phase 1 rulings

Each ruling gives the decision, the evidence behind it, and what it binds
later phases to. Where Phase 1 evidence contradicted the plan's recommended
default, the ruling says so and records the amended decision. No ruling is
left at "recommended".

Evidence files are cited relative to this feature directory:
`spikes/s1-feature-sets.md`, `spikes/s2-hazards.md`, `spikes/s3-cross-check.md`,
`spikes/s4-deps.md`, and `baseline/*`.

## R1 — Generalize `consolidation.py` for the ten packages (binds Phase 2)

**Decided: as recommended, with one amended set.** The ten packages join
`PACKAGES` and `PACKAGE_FEATURE_SETS` as reviewed data. Nothing is derived from
justfiles. The capture-time check stays: the CI `features` union must be one of
the listed sets.

| Package | Feature sets |
|---|---|
| `tree-hugger` | `()` |
| `claudine` | `()` |
| `sniff` | `()`, `(remote)`, `(network)` |
| `sniff-cli` | `()`, `(test-fixtures)` |
| `biscuit-file` | `()`, `(fetch)` |
| `schematic-gen` | `()`, `(terminal-tests)` |
| `biscuit-terminal-cli` | `()`, `(terminal-tests)` |
| `claudine-gen` | `()`, `(terminal-tests)` |
| `dmls` | `()`, `(effects-instrumentation)`, `(terminal-tests)`, `(terminal-tests, effects-instrumentation)` |
| `biscuit-tui-cli` | `()`, `(terminal-tests)` |

The amendments come from S1:

- **`dmls` gains `(terminal-tests)`.** `darkmatter`'s `test-l2` builds `dmls`
  with `terminal-tests` alone.
- **`claudine`'s open item resolves to `()`.** `claudine/justfile` passes no
  feature to `claudine` in any recipe, and `claudine` has no
  `[package.metadata.ci.tests]` table.

`dmls` is the one in-scope package with `l1-include-slow = true`. `capture`
records the `L1-include-slow` selector for it. It has no `slow_` test today,
so the two states are expected to be equal. That makes the check a no-op proof,
not a skipped one.

## R2 — Where the mover and acceptance checks live (binds Phase 2)

**Decided: as recommended.** Three scripts are promoted into
`consolidation.py` subcommands that take explicit inputs:

- `pilot/apply-move.py` → `move <manifest> [--layout-gate <file>]`;
- `acceptance/metadata-check.py` → `check-metadata --manifest <file>…`; and
- `acceptance/body-diff.py` → `body-diff --manifest <file> --base-rev <rev> --markdown <out>`.

Each gets `test_consolidation.py` fixtures. The wave-1 originals stay
untouched as that feature's record.

`move` also:

- emits root `mod` lines in rustfmt order; and
- omits `common` from a root whose modules do not use it.

Both were manual steps in every wave-1 package.

Why not copy the scripts again: the first feature's directory moves to
`_completed` when its author closes it, so any hard-coded path into it breaks.
A second copy of roughly 370 lines would also drift from the first.

## R3 — Target names (binds Phases 3–5)

**Decided: as recommended for naming, with `sniff-cli` amended to two
targets.**

- Tiers are named `l1`, `level2`, and `level3`. When one tier in a package has
  more than one feature contract, the feature-bearing target takes a suffix,
  and the feature-less one keeps the bare name as the default contract.
- **`biscuit-file`** gets `l1` and `l1-fetch`.
- **`sniff-cli` gets `l1` and `level2`**, with `level2` declaring
  `required-features = ["test-fixtures"]`. The plan's
  `level2`/`level2-fixtures` split is **not** needed. See R14.

This refines wave-1 R2.

## R4 — Where L1-tier tests that require a feature go (binds Phases 4–5)

**Decided: as recommended.** The target set is decided by the distinct
`required-features` sets.

- An L1-tier test that requires feature F joins the package's existing target
  for F (the `harness_integrity` precedent).
- Only when no such target exists does it get its own `l1-<feature>` target,
  as with `biscuit-file`'s `fetch_integration`.
- Tier selection stays filter-based. CI's L1 cell builds with the CI feature
  union, so it still selects these tests.

Dispositions:

| Case | Disposition |
|---|---|
| `biscuit-tui-cli` `windows_captured_stdout` (F5) | Joins `level2` as `#[cfg(windows)] mod windows_captured_stdout;`, which is the wave-1 practice that `check-attributes` enforces. **Unlike** wave 1, the file also **keeps** its inner `#![cfg(windows)]`, which is legal and redundant in a module file. `tools/test-toolkit/tests/ci_workflow_contracts.rs:7067` asserts that the file contains it (R17). The module name has no marker, so its one test stays L1. |
| `biscuit-terminal-cli` F3 (43 tests in `level2_prose_cells.rs`, 6 in `level2_diagrams.rs`) | Stay in `level2` under neutral aliases (R5). The Phase 1 scan confirms 43 + 6 = 49 unmarked tests. No other marker-named file in scope has an unmarked test. |
| `schematic-gen` `terminal_capture` (F2) | Becomes `schematic-gen`'s `level2` target. Spec hazard disposition: **moot, no L1 test**. |

`test_inputs.py` treats every `level2*` binary as holding no L1 test. R15
explains why this ruling does not change what CI narrows today.

## R5 — Neutral aliases (binds Phases 4–5)

**Decided: as recommended.** Wave-1 R2 carries forward. A former target name
becomes the module name unless `plan`'s projection shows it would newly match,
or stop matching, a `_tier_filter` expression or an override filter. In that
case the marker prefix is stripped.

Required aliases at `208051f75`:

- `real_terminal_render` → `terminal_render`. It has 14 tests, all named
  `level2_*`. As `real_terminal_render::level2_…`, the path would also carry a
  `real_` segment, and `biscuit-tui`'s `test-real` is a stub (F4).
- `level2_prose_cells` → `prose_cells`.
- `level2_diagrams` → `diagrams`.

None of the three alias names collides with an existing file, directory, or
reserved module in its package. Any other alias `plan` emits is reviewed and
recorded in that package's evidence. Former `level2_*` targets whose tests all
carry the marker keep their names. The rule is mechanical and does not tidy
names.

## R6 — Mixed-tier files stay whole (binds Phases 3–5)

**Decided: as recommended.** This generalizes wave-1 R4. A source file whose
tests carry different markers (F6) lands whole in the target its
`required-features` dictates, and the filter decides the tier. Files are never
split.

## R7 — `test-toolkit` dev-dependency (binds Phases 3–4)

**Decided, amended: five packages, not two.** S4 found that three more
packages need it. `schematic-gen`, `biscuit-terminal-cli`, and `claudine-gen`
carry `test-toolkit` only as an **optional** regular dependency enabled by
`terminal-tests`. Their feature-less `l1` binary could not name
`test_toolkit::test_layout`.

| Package | Change |
|---|---|
| `biscuit-file`, `tree-hugger` | Add an unconditional `[dev-dependencies]` entry. |
| `schematic-gen`, `biscuit-terminal-cli`, `claudine-gen` | Keep the optional regular dependency (`dep:test-toolkit` in `terminal-tests`) **and** add an unconditional `[dev-dependencies]` entry. |

The precedent is `biscuit-terminal/lib/Cargo.toml:74` and `:98` from wave 1.

Evidence from S4:

- No cycle: the `test-toolkit` graph contains none of the ten packages.
- Every crate each package gains is already in `Cargo.lock` at the version
  the workspace builds, and every addition is dev-only.

Each package commit adds its line to `docs/dependencies.md` and its area's
`docs/dependencies.md` (see `spikes/s4-deps.md` §Documentation obligations).
`tree-hugger` has no area dependency doc, and none is created for one line.

## R8 — What "noticeably longer" means (binds Phases 3–5)

**Decided: as recommended.** Each package gets one warm edit-to-one-module
observation before the move and one after:

1. Touch one L1 module file.
2. Run `cargo nextest run -p <pkg> -E "$(just _tier_filter L1 <pkg>)" <module>`
   with `RUSTC_WRAPPER=""` and the kache shims removed from `PATH`.

The full five-trial protocol (wave-1 R6) runs only if the after figure exceeds
the before by **more than 50% and more than 5 s**. Otherwise the target-count
and executable-bytes rows in `measurements.md` are the evidence.

## R9 — Evidence layout (binds Phases 3–6)

**Decided: as recommended.** Each package gets
`<package>-migration.json` at this feature's top level, plus a `<package>/`
directory containing:

- `capture-before/`, `capture-after/`, `capture-linux-before/`,
  `capture-linux-after/`;
- `comparison-{darwin,linux,darwin-linux}.{json,md}`;
- `attribute-check.json`;
- `snapshot-mapping.json` and `snapshot-check.json`, where snapshots exist;
- `metadata-check.txt` and `body-diff.md`;
- `test-inputs.md` (spec hazard 6): the probe path from
  `baseline/test-inputs.md`, with its unit set before and after, and the
  `cargo nextest list` of each unit;
- `layout-guard.md`: red on a planted stray root, red on an undeclared
  module, then green;
- `guard-scans-after.md`, where a path guard exists; and
- `cross-check-{windows,wsl}.txt`, the `tee`d output of `cross-check`
  (R16).

## R10 — Override rewrites (binds Phases 3–4)

**Decided: as recommended.**

| Package | Rewrite | Profiles |
|---|---|---|
| `biscuit-terminal-cli` | `test(=level2_render_tree_style_in_wezterm)` → `test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)` | `default`, `ci` |
| `sniff` | `test(=test_detect_completes_in_reasonable_time)` → `test(=integration::test_detect_completes_in_reasonable_time)` | `ci` |

Each rewrite is shown matching exactly one test with a committed
`cargo nextest list -E '<filter>'`, run under the profile's feature set.
Package-level filters (the `claudine-l1` test group, `sniff-windows-l1`) and
the unanchored `test(/level2_/)` and `test(/browser_/)` are re-proven by
`capture`'s per-override listing, not by reasoning.

## R11 — Scope under drift (binds Phases 3–6)

**Decided: as recommended.** Scope is the ten named packages. At `208051f75`
the next-largest packages are `biscuit-clipboard-cli` and `worktree-cli`, with
8 targets each. If any other package reaches ten before closeout, the work
stops and the author is asked (`human-in-the-loop`). Scope is never widened
silently.

## R12 — Commits (binds Phases 2–6)

**Decided: as recommended.** Commits are authorized per package, in this
series:

1. manifest and evidence;
2. structural move;
3. area docs.

This follows wave-1 R7. Phase 2's toolkit change is its own commit. Every
commit:

- uses the author's identity and is signed;
- carries no agent-attribution trailer (the repo `CLAUDE.md` overrides the
  harness default); and
- is checked with `git verify-commit HEAD`.

A signing failure halts the work. `--no-gpg-sign` is never used.

## R13 — Sequencing overlap (binds Phases 3–5)

**Decided: as recommended, with S3's correction.** Wave-1 R8 carries forward.
Package N's remote verification may overlap package N+1's read-only baseline.
Package N+1's structural move starts only after package N's local commit.

S3 shows that `cross-check` ships the **working tree**, not `HEAD`: tracked
edits and untracked, non-ignored files, laid over `origin/<branch>` or
`origin/main`. "Remote legs run against committed revisions" therefore holds
only if the tree is clean when the leg starts. Package N+1's baseline is
read-only, so it does not dirty the tree. Still, no structural edit for N+1
may start until N's remote legs have finished, not merely N's local commit.
Otherwise N's Windows and WSL2 evidence would describe a mixed tree.

## R14 — `sniff-cli` has one Level 2 feature contract, not two (new; binds Phase 5)

**Decided: one `level2` target requiring `test-fixtures`.** This amends
spec §3 hazard 3 and the plan's R3 and expected-target table.

**Evidence:** `sniff/cli/tests/level2_recent_commits_rendering.rs:22` is
`#![cfg(feature = "test-fixtures")]`, as are the other three `level2_*` files
(S1 §Feature gates inside test sources). Its target declares no
`required-features`, but without the feature it links an **empty** binary.
Its tests import `biscuit_test_harness`, which only `test-fixtures` brings
in. So the tests do declare `test-fixtures`, just in source rather than in
the manifest. Putting it in the `test-fixtures` target enables no feature its
tests do not already require. The defect the first feature's §1 guards
against is therefore not present.

**Consequence:**

- `sniff-cli` becomes **2** targets (`l1`, `level2`).
- The expected workspace total drops by one (19 → **18** in scope).
- The four-way comparison for the `()` set must show zero tests before and
  after for this file.
- In Phase 5, `plan`'s inventory must confirm that the file's inner `cfg`
  still gates every item. If it does not, this ruling reverts to the plan's
  two-target split.

**Author override:** the spec says "keep them in separate targets", so the
author may restore the split before Phase 5. The cost of doing so is one more
target with no test-identity benefit.

## R15 — `test_inputs.py`'s binary-name heuristic (new; binds Phases 3–5 evidence)

**Decided: no tool change in this feature. Every package's `test-inputs.md`
proves the unit set unchanged.**

`scripts/ci/test_inputs.py:254` (`NON_L1_BINARIES`) and `:264` (`_unit`)
return no narrowed unit for any reference inside a test binary whose name
starts with `level2`, `level3`, `browser`, or `real`. This rests on the
assumption that such a binary holds only non-L1 tests. R4 and wave-1's
`harness_integrity` both violate that assumption on purpose.

Effect in this wave:

- **F3 (`biscuit-terminal-cli`):** no change. The old binaries
  (`level2_prose_cells`, `level2_diagrams`) already match the prefix, so any
  reference in them is already dropped. The same holds for the files that read
  fixtures today (`level2_cursor_and_hygiene.rs:39` and `level2_image.rs:53`
  both resolve to `None` in `baseline/test-input-probe-before.json`).
- **F5 (`windows_captured_stdout`):** it would lose narrowing on moving into
  `level2`. But the probe finds **zero** test-input references in all of
  `biscuit-tui-cli`, so nothing observable changes.
- **`l1-fetch` and `level2`-named targets elsewhere:** unaffected. `l1-fetch`
  does not match the prefix, and the other `level2` binaries hold L2 tests.

Changing the heuristic would change the CI planner's scheduling, which is
outside this feature's scope ("no new CI run, matrix cell, or gate"). It is
recorded here as a follow-up for the author. The probe re-run in each
package's `test-inputs.md` is the guard: any unit that disappears is a
regression that stops the package.

## R16 — Windows evidence from `cross-check` (new; binds Phases 3–5)

**Decided: no `cross-check` change. Capture the output with `tee`.** Per S3:

- The default (archive) mode builds each package's CI `features` union. So
  `biscuit-tui-cli` gets `terminal-tests` (`windows_captured_stdout` is
  compiled) and `sniff` gets `remote`. `sniff`'s Windows-only files need no
  feature.
- Per-test PASS lines are printed live. No JUnit report is kept locally, and
  `--no-tests=pass` means a green leg does not prove that a `cfg`-gated test
  ran.

So every Windows and WSL2 leg is run as
`./scripts/cross-check.sh <pkg> --os <os> 2>&1 | tee <pkg>/cross-check-<os>.txt`.
For `sniff` and `biscuit-tui-cli`, the evidence cites the named PASS line of
each Windows-only test.

**Do not pass `--features` to `cross-check`.** Any build flag switches every
host to native mode, with no tier filter and no receipt.

S3's smallest optional improvement, not adopted here, is to fetch the Windows
JUnit report before `scripts/cross-check.sh:990` deletes it. The `os` skill's
statement about feature flags was wrong, and was corrected in this phase
(`.claude/skills/os/build-hosts.md`).

## R17 — Code in other packages that reads a moved test file (new; binds Phases 3–5)

**Decided: the path repair lands in the owning package's structural-move
commit, even though the reader is in another package.** This is the
"repair identities that changed because the source moved" class from wave-1
R9.

`baseline/test-selector-consumers.md` finds one run-time reader of an in-scope
test file outside its own package:

- `tools/test-toolkit/tests/ci_workflow_contracts.rs:7053` reads
  `biscuit-tui/cli/tests/windows_captured_stdout.rs`.
- It asserts that the file has no `#[ignore`, has no fixed sleep, and does
  have `#![cfg(windows)]`.

In Phase 5 the literal becomes the file's new path under `tests/level2/`, and
`test-toolkit` joins that commit's `packages`. `test-toolkit`'s L1 must pass on
the moved tree. Because the reader goes through a `read()` helper rather than
an inline `repo_root().join("…")`, `test_inputs.py` may not see this reference.
The move must not rely on CI narrowing to catch it.

`claudine/lib/src/provider/tests.rs:254` names `gen/tests/registry_coverage.rs`
in a failure-message string. It is prose, not a read, so it is updated in
`claudine-gen`'s docs pass.

The remaining 39 file-path references are comments and docs. Each package's
docs pass updates them from that list.

## R18 — Proptest regression files move to proptest's own location (new; binds Phases 2–3)

**Decided: a regression file moves to
`tests/proptest-regressions/<stem>.txt`, byte-identical. It does not move
beside its module.** Phase 2's `move` performs this mechanically, and
`check-snapshots` (or an equivalent check) verifies it alongside the snapshot
mapping.

**Evidence (S2 H-P1, re-read from source in this phase):** proptest 1.11.0's
default persistence is `SourceParallel("proptest-regressions")`
(`proptest-1.11.0/src/test_runner/failure_persistence/file.rs:77-81`). It
walks up from the test's source file to the nearest ancestor holding a
`lib.rs` or `main.rs` (`:336-367`).

- **Before a move,** `tests/` has no `main.rs`. The walk fails and falls back
  to `<source>.proptest-regressions` beside the file.
- **After a move,** `tests/<target>/main.rs` exists, so the file resolves to
  `tests/proptest-regressions/<stem>.txt`.

In scope:

- **`biscuit-file`:** `tests/yaml_mutation.proptest-regressions` (2 seeds) →
  `tests/proptest-regressions/yaml_mutation.txt`.
- **`claudine`:** `typed_stream_protocols.rs` uses `proptest!` with no
  committed seeds. Nothing moves.

Proof of the rule: Phase 2 proves it once with a throwaway scratch crate and a
deliberately failing property. This is a local run only, not a CI cell. The
result goes in the self-proof evidence.

**Wave-1 defect found:** wave 1 moved darkmatter's two regression files
(5 seeds) to `darkmatter/lib/tests/l1/*.proptest-regressions`, where proptest
no longer reads them. It is filed as
`darkmatter/fixes/_unscheduled/proptest-regressions-after-consolidation/spec.md`
and is outside this feature's scope.

## R19 — Executable recipes and detector false positives (new; binds Phases 3–5)

**Decided:**

- **Recipes that execute a former `--test` target** are updated in the
  package's structural-move commit, not its docs pass, because they stop
  working at the move:
  - `schematic/justfile:271` (`test-e2e`) → `--test l1 -- --ignored e2e_generation::`.
    The filter is required: without it the recipe would run every ignored
    test in `l1`.
  - `biscuit-tui/justfile:127-129` (`test-pty`) → `--test l1 keyboard_protocol::`,
    `--test l1 completions_shell::`, and `--test l1 choose_cli::pty::`.

  Each rewritten recipe is run once, and the evidence shows it selects the
  same tests as before.
- **`crate::` inside string literals** is recorded as a manifest disposition
  (`crate_path`: "inside a string literal naming generated code; not a
  path"). It is not an edit. The sites are
  `schematic/gen/tests/e2e_generation.rs:214`,
  `schematic/gen/tests/http_client.rs:471`, and
  `claudine/gen/tests/pipeline.rs:166`.
- **Self-exec `--exact` identity strings** are permitted structural repairs
  (wave-1 R9). Each is shown to be load-bearing by running once with the old
  string and seeing it fail:
  - `claudine/gen/tests/level2_report_terminal.rs:160` →
    `level2_report_terminal::level2_report_probe`;
  - `darkmatter/dmls/tests/stdio_subprocess.rs:85` →
    `stdio_subprocess::child_guard_cancellation_probe`.
- **Path-keyed self-exclusion:** `sniff/cli/tests/spawn_site_guard.rs:44`
  (`relative == "spawn_site_guard.rs"`) becomes `"l1/spawn_site_guard.rs"`,
  following the claudine-cli precedent. The scan diff is recorded in
  `guard-scans-after.md`.
- **`sniff`'s `tests/fixtures.rs`** is today both an auto-discovered target
  with 0 tests and a child module of `integration`. It becomes a helper, not
  a root module. Dropping its empty target loses no test identity. The target
  count is then 20 → 1, with 19 root modules.

## Corrections recorded against the spec and plan

The spec body is not edited.

- **Counts:** 236 workspace targets and 136 in scope at `208051f75` (the spec
  read 232 and 132 at `e51fb0373`). F1 stands.
- **Expected targets:** **18**, not 19, because of R14. The workspace would drop
  from 236 to about **118**.
- **`dmls` feature sets:** four, not three (R1).
- **`test-toolkit` dev-dependency:** five packages, not two (R7).
- **`cross-check`** ships the working tree, not `HEAD` (R13).
- **Spec hazard 3** rests on a premise that does not hold (R14).
