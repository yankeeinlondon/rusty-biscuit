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
    Phase 5 is implemented but NOT committed (the phase instructions forbid it). The working tree holds two packages' moves, `sniff-cli` (11 → 2) and `biscuit-tui-cli` (15 → 3), plus two toolkit changes and one evidence-tool fix. Read the Phase 5 section of `implementation-log.md` first.
    Commit order (R12):
    - Toolkit commit 1: `scripts/ci/consolidation.py` + `test_consolidation.py`, the `path_key` disposition in `body-diff` (4 new tests). It must land before `sniff-cli`'s evidence, whose `body-diff` needs it.
    - Toolkit commit 2: `test_consolidation.py`'s five host-tool gates now call `tool_guard.require_tools`. This fixes `test-toolkit::ci_workflow_contracts no_ci_python_suite_gates_a_host_tool_outside_the_shared_guard`, which was already red at `8255ee228` (Phase 2's `ebd261004` introduced it). If one commit is preferred for both toolkit changes, that is fine; they touch the same test file.
    - Then each package's series: manifest and evidence, structural move, area docs. The `biscuit-tui-cli` structural move includes `tools/test-toolkit/tests/ci_workflow_contracts.rs` (R17), `biscuit-tui/justfile` `test-pty` (R19), and its new unconditional `biscuit-test-harness` dev-dependency. `docs/dependencies.md` gained one line for that dev-dependency. `darkmatter/lib/tests/l1/context_functions.rs:234` is a comment-only docs change for `sniff-cli`.
    - `measure/test-input-check.py` gained an optional after-path argument (evidence tooling).
    For Phase 6:
    - Expected workspace total: 236 − 136 + 18 = 118 integration-test targets. `check-metadata` across all ten manifests already passes (`metadata-check-all-ten.txt`).
    - `baseline/test-input-probe.py` enumerates `git ls-files`. On an uncommitted tree it misses moved files; use a temporary `GIT_INDEX_FILE` (`git add -A <paths>`) or run it after the commits.
    - Left as history in the docs pass: `.claude/skills/rust-testing/SKILL.md:140` (a past-tense account naming `biscuit-tui/cli/tests/windows_captured_stdout.rs`). The consumer sweep `--after` will list it; annotate it or update it, your call.
    - Level 2 on this host: tmux and WezTerm work; Kitty has no usable instance (`os` skill, `macos.md`). L3 was not run (it takes focus).
    - Known failures NOT caused by this feature: `claudine-cli`'s 2 `shipped_prompt_route_drift` failures (unchanged), and 3 env-gated `biscuit-tui-cli` `keyboard_protocol` PTY tests under `just test-pty` (identical on the base; not in CI).
    - For the author, not blocking: the `test-pty` failures above; `sniff-cli`'s `spawn_site_guard` self-exclusion key is untested (a wrong key passes silently); plus Phase 4's two items (`schematic/justfile` `check-drift`, `dmls` `child_guard_reaps_process_during_unwind`).
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
