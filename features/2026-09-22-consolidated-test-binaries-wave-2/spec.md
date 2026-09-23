---
area: repo
status: planned
created: 2026-09-22
owner: Ken Snyder <ken@ken.net>
origin: follow-on to 2026-09-21-consolidated-test-binaries — every remaining package with ten or more integration-test binaries, 2026-09-22
related:
    - 2026-09-21-consolidated-test-binaries
    - 2026-09-21-ci-build-feature-divergence
    - 2026-09-22-test-input-blind-spot
depends-on:
    - 2026-09-21-consolidated-test-binaries
packages:
    - sniff
    - biscuit-tui-cli
    - biscuit-file
    - schematic-gen
    - biscuit-terminal-cli
    - claudine
    - sniff-cli
    - claudine-gen
    - tree-hugger
    - dmls
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
reviewed: false
implemented: false
human_review: false
message_to_agent: |-
    Phase 2 (toolkit) is complete but NOT committed. The phase instructions forbid commits, so R12's signed toolkit commit is still owed. It covers `scripts/ci/consolidation.py`, `scripts/ci/test_consolidation.py`, and `selfproof/`. Make it before any package commit, so each package series builds on the committed toolkit.
    Read `selfproof/README.md` and the Phase 2 section of `implementation-log.md` first. For Phases 3–5 the key points are:
    - `selfproof/capture-a/` is the committed macOS before-side for all ten packages, taken at `9d44e7988` with every override selector. You can `plan` from it directly (`plan --package <p> --listings selfproof/capture-a`). Recapture only if a package's tests, features, or `.config/nextest.toml` changed since then: `plan` refuses a capture whose recorded filter differs from the live one.
    - `plan` now reproduces the ruled table with no hand edits: 18 targets; aliases `prose_cells`, `diagrams`, and `terminal_render` only; R10's two suggested rewrites; `sniff`'s `fixtures` dropped as a helper (R19); R4 and R14 through the reviewed `RULED_TARGETS` table. `ShippedWave2PlanTests` pins this. If `plan` disagrees with that test, stop and find out why. Do not hand-edit the manifest to match.
    - `move <manifest> --layout-gate test_layout.rs` does every path repair and the `common` rewrite, writes the roots, relocates proptest seeds (R18), and prints the `[[test]]` entries. It refuses on a dirty destination and changes nothing when it refuses. It does NOT edit `Cargo.toml`. Set `autotests = false`, paste the entries, and delete stale `[[test]]` entries (`biscuit-file`'s `fetch_integration` has one). A dry run of all ten packages is in `selfproof/dry-run.md`: `tree-hugger` and `biscuit-file` built and compared `identical` against `capture-a`.
    - SPP step 5 now includes `check-proptest --manifest <m> --before-rev <base>`. `body-diff` now EXITS 1 on any non-structural line. Use `--after-rev` or the working tree (default) before committing.
    - Manifest hand edits the tools will demand (all predicted in S2/R19): `dispositions` entries (`{path, detector, reason}`) for the `crate_path` string literals (`schematic-gen` ×2, `claudine-gen` ×1), for `biscuit-tui-cli`'s genuine `crate::common` users (`choose_cli`, `keyboard_protocol`), and for the self-exec identity strings (`claudine-gen` `exact_path_string`, `dmls` `exact_arg`) after repairing them. Also `"keep_inner_cfg": true` on `biscuit-tui-cli`'s `windows_captured_stdout` row BEFORE running `move` (R17).
    - Phase 3 packages (`tree-hugger`, `claudine`, `sniff`) need none of those except `sniff`'s R10 override rewrite. `sniff`'s `integration.rs` gets `#[path = "../fixtures.rs"] mod fixtures;` automatically, and `tests/fixtures.rs` stays put. The layout gate follows `#[path]`, so it is reached. `tree-hugger` still needs its unconditional `test-toolkit` dev-dependency (R7).
    - OPEN for Phase 4 (not blocking Phase 3): `biscuit-terminal-cli`'s `tests/common/pane_geometry.rs` has 10 unit tests. Each of its 9 `level2_*` binaries compiles a copy (90 identities, L1-tier, run in CI's L1 cell). With one `common` per root they fold to 10, which `plan` records as `shared_tests`. `compare` folds the copies and fails if they disagree. Recommended: accept the fold and record it as the package's disposition in `acceptance.md`. That follows first-spec §3 and `rust-testing`'s "declared once" rule, while wave 1's per-module `parity_helpers` copies are called "legacy shape, not a pattern". The alternative is keeping per-module copies, which needs a toolkit change so that `plan` projects `common::` tests per module.
    - Wave 1's `claudine-cli` row in `PACKAGE_FEATURE_SETS` is stale (its CI union gained `test-fixtures`). It is outside scope and untouched. Do not capture `claudine-cli`.
    - Carried from Phase 1, still binding: R16 (never pass `--features` to `cross-check`; `tee` its output), R13 (finish package N's remote legs before N+1's structural edit, because `cross-check` ships the working tree), and the two pre-existing `claudine-cli::l1 shipped_prompt_route_drift` failures caused by the author's uncommitted `prompts/_implement/*.md` edits. Do not "fix" those.
---

# Consolidate integration tests in the ten remaining double-digit packages

## Outcome

Every workspace package that still has ten or more integration-test binaries
is migrated the same way `2026-09-21-consolidated-test-binaries` migrated
`claudine-cli`, `darkmatter`, `darkmatter-cli`, and `biscuit-terminal`: tests
that share an execution contract link into one binary, and every test keeps its
tier, features, operating-system conditions, and assertions.

That feature's design sections 1–7, its alternatives, its out-of-scope list,
and its twelve acceptance criteria apply here unchanged unless this spec says
otherwise. This spec records only what is different: the package inventory, the
hazards already known for these packages, and the reuse of the first feature's
tooling in place of a second pilot.

## Evidence

At `e51fb0373`, `cargo metadata` reports **232 integration-test targets across
51 packages**. After the first feature, no package has more than 19. These ten
have ten or more, 132 in all (57% of the workspace):

| Package | Manifest | Targets | Contracts found by the inventory below | Expected targets |
|---|---|---:|---|---:|
| `sniff` | `sniff/lib` | 19 | L1, no features (2 files `cfg(target_os = "windows")`) | 1 |
| `biscuit-tui-cli` | `biscuit-tui/cli` | 15 | L1; L1 + `terminal-tests` (`cfg(windows)`); L2 + `terminal-tests`; L3 + `terminal-tests` | 3–4 |
| `biscuit-file` | `biscuit-file/lib` | 15 | L1; L1 + `fetch` | 2 |
| `schematic-gen` | `schematic/gen` | 14 | L1; L1 + `terminal-tests` | 2 |
| `biscuit-terminal-cli` | `biscuit-terminal/cli` | 14 | L1; L2 + `terminal-tests` | 2 |
| `claudine` | `claudine/lib` | 13 | L1, no features | 1 |
| `sniff-cli` | `sniff/cli` | 11 | L1; L2 + `test-fixtures`; L2, no features | 3 |
| `claudine-gen` | `claudine/gen` | 11 | L1; L2 + `terminal-tests` | 2 |
| `tree-hugger` | `tree-hugger/lib` | 10 | L1, no features | 1 |
| `dmls` | `darkmatter/dmls` | 10 | L1; L2 + `terminal-tests` | 2 |
| **Total** | | **132** | | **19–20** |

The contract column is a first reading from target names and
`required-features`, not the checked inventory. Tiers are decided by test
*paths* under `just _tier_filter`, not by target names, so each package's
generated inventory is authoritative and may change the expected count. If it
does, the plan records why.

If the counts hold, the workspace drops from 232 integration-test targets to
about 120, and the largest remaining package has 8 (`worktree-cli`,
`biscuit-clipboard-cli`).

Per-binary weight varies more here than in the first four packages. In pull
request 92's producer job, `claudine` archived 375 MB and `claudine-gen`
218 MB, each for 14 binaries, and `biscuit-file` archived 75 MB for 16. The
`claudine` family therefore likely dominates this spec's CI saving, and the
smaller packages mostly reduce link count and target-directory churn. These
are single observations, not baselines.

## What differs from the first feature

### 1. No second pilot

The first feature's `claudine-cli` pilot answered the seam question: one binary
per execution contract, with edit-to-one-test latency up 0.30 s (+12%) and
peak compiler memory unchanged. Each package here has at most a fifth of that
pilot's test count, so no package is expected to come close to the split
guardrail (median edit latency up by more than 50% **and** more than 5 s).

Acceptance criterion 9 is replaced by the lightweight observation the first
feature recorded for its Phase 4–6 packages. For each package, record
test-target count and test-executable bytes, before and after, with
`cargo test --no-run --locked -p <package> --features <set>`,
`RUSTC_WRAPPER=""` (kache really off; see the first feature's measurement
amendments), and the before side in its own `--target-dir`. If a package's
consolidated `l1` crate takes noticeably longer than expected in the ordinary
edit loop, run the full edit-latency protocol for that package before moving
on, and apply the original guardrail.

### 2. Reuse the tooling, do not rebuild it

Use the first feature's tooling as it is, extending it only where a package
needs something new:

- `scripts/ci/consolidation.py` for inventory, capture, compare, and
  attribute checks;
- the first feature's generalized mover;
- its `acceptance/metadata-check.py` and `acceptance/body-diff.py`, pointed at
  this feature's manifests; and
- `test_toolkit::test_layout` for criterion 12's guard in each package's `l1`
  binary.

Each package gets its own `<package>-migration.json` manifest in this
feature's directory. As in the first feature, the manifests are the only
authoritative old-to-new mapping.

`biscuit-file` and `tree-hugger` do not yet dev-depend on `test-toolkit`. It
depends on none of the ten packages, so adding it creates no cycle. Record in
`docs/dependencies.md` whether it adds any crate to either package's graph.

### 3. Known identity hazards

These were found while writing this spec. The inventory must confirm them and
look for others.

- **`biscuit-tui-cli`'s `real_terminal_render` needs a neutral module alias.**
  Its 14 tests are named `level2_*`. Moved into a module called
  `real_terminal_render`, their paths would also match `(^|::)real_`, so the
  `real` tier would start selecting Level 2 tests. This is the same case as
  `darkmatter-cli`'s `level2_harness_integrity` alias in the first feature.
- **Former target names that begin with a tier marker become path segments.**
  `biscuit-terminal-cli`, `claudine-gen`, `dmls`, and `sniff-cli` have
  `level2_*` targets, and `biscuit-tui-cli` has `level3_chord_select`. Any
  test in those files whose own name lacks the marker would change tier. The
  four-way before/after comparison must catch this; do not reason it away from
  names.
- **`sniff-cli` has two Level 2 feature contracts.**
  `level2_recent_commits_rendering` requires no features, while the other three
  Level 2 targets require `test-fixtures`. Keep them in separate targets.
  Adding `test-fixtures` to the first would enable a feature its tests do not
  declare, which is the defect the first feature's section 1 exists to prevent.
- **`biscuit-tui-cli`'s `windows_captured_stdout` is Level 1 under
  `terminal-tests`, and `schematic-gen`'s `terminal_capture` also requires
  `terminal-tests`.** Their tests stay in the Level 1 tier. Decide from the
  inventory whether each shares a target with the package's Level 2
  `terminal-tests` tests (the `harness_integrity` precedent) or gets its own.
  Neither may join the feature-less `l1` target.
- **Two `.config/nextest.toml` overrides name tests in scope by exact path.**
  `test(=level2_render_tree_style_in_wezterm)` (`biscuit-terminal-cli`, slow
  timeout) and `test(=test_detect_completes_in_reasonable_time)` (`sniff`,
  `threads-required` in the `ci` profile) would silently match nothing after
  the move. Rewrite each to its new module-qualified path in every profile
  that carries it, and show each override matching exactly one test afterward.
- **Test-input identities change.** `2026-09-22-test-input-blind-spot` resolves
  a changed file to exact `binary_id(...) & test(=...)` identities. Confirm
  that `scripts/ci/test_inputs.py` derives the new module paths from the
  migrated target roots with no stored identity to translate. Run it on a
  fixture or file each migrated package reads, and show the narrowed filter
  selects the same tests before and after.

Package-level test groups (`sniff-windows-l1` matches
`package(sniff) + package(sniff-cli)`) and `package(...)` filters are
unaffected.

### 4. Sequencing

Migrate one package at a time and commit each package separately, with its
manifest and evidence. Start with the three single-contract packages
(`tree-hugger`, `claudine`, `sniff`); they have no feature boundary and check
the tooling on new ground cheaply. Then do the two-contract packages
(`biscuit-file`, `schematic-gen`, `biscuit-terminal-cli`, `claudine-gen`,
`dmls`), and finally `sniff-cli` and `biscuit-tui-cli`, which carry the
hazards above. Rebuild the inventory for each package; do not assume an
earlier package's shape.

## Out of scope

- The 37 packages with fewer than ten integration-test targets.
- Everything the first feature ruled out of scope, including benchmarks (every
  `harness = false` entry in these ten manifests is a `[[bench]]`), test
  behavior changes, stale-artifact sweeps, and any CI run or gate added only
  for this feature.

## Acceptance criteria

The first feature's criteria 1–8 and 10–12 apply to each package here, read
with these changes:

1. **Criterion 4** uses that feature's narrowed Windows clause: before/after
   listing comparisons on macOS and Linux; on native Windows and WSL2, the
   consolidated suites run with `just cross-check <package> --os windows` and
   `--os wsl`. `sniff` and `biscuit-tui-cli` have Windows-only test files, so
   their Windows runs must show those modules compiled and run.
2. **Criterion 9** is replaced by section 1's per-package target count and
   executable bytes, before and after, with the full edit-latency protocol
   only if section 1's trigger fires.
3. **Criterion 12**'s guard uses the shared `test_toolkit::test_layout` in
   every package, and is shown failing on a planted stray root and an
   undeclared module, then passing.
4. **New:** each hazard in section 3 has a recorded disposition, with the
   evidence showing it handled: the alias in the manifest, the separate
   target, the rewritten override matching one test, the test-input filter
   matching the same tests.
5. **New:** after the last package, `cargo metadata` shows no workspace package
   with ten or more integration-test targets, and the acceptance record gives
   the new workspace total.
6. **New:** no package ends with a stranded test. A module alias or a former
   target name that starts with a tier marker can move tests out of L1 into a
   tier whose recipe is a stub, where they run nowhere. `just
   check-tier-coverage <area>` is clean for every area touched, and CI's L1
   producers refuse any that remain (`completion-test-stranded`, added
   2026-09-23). The expectations implementers follow are in
   `prompts/_test-tiers.md`.
