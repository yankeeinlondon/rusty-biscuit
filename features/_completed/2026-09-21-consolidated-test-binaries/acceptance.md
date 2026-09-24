---
kind: acceptance
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-22
verdict: met-with-pending
pending:
    - acceptance 10, CI producer observations
---

# Acceptance record: consolidated test binaries

This walks the twelve acceptance criteria in `spec.md`. Each row names the
committed artifact that shows it, and the command that re-runs it where one
exists. Paths are relative to this feature directory unless they start at the
repository root.

**Summary.** Ten criteria are met. Criterion 5 is met as far as a tool can
show, and it still needs the reviewer's reading, as the spec says. One is
pending, and nothing pending is counted as a pass:

- **10:** CI observations. No ordinary run has selected the packages yet.

Criterion 4 closed on 2026-09-22: every package's consolidated suites now run
on native Windows and under WSL2 (§4). Its Windows clause was narrowed the
same day — on-host base-versus-migrated listings are no longer required there;
the spec says why.

Evidence per package lives in `pilot/` (`claudine-cli`), `darkmatter/`,
`darkmatter-cli/`, and `biscuit-terminal/`, next to the four manifests
`*-migration.json`.

| Package | Base revision | Old targets | New targets |
|---|---|---:|---|
| `claudine-cli` | `9621882ae` | 139 | `l1`, `level2`, `level3`, `real` |
| `darkmatter` | `048e44f7a` | 74 | `l1`, `level2`, `level3-terminal`, `level3-browser`, `browser` |
| `darkmatter-cli` | `cb9a3d38b` | 53 | `l1`, `level2` |
| `biscuit-terminal` | `beca6c368` | 38 | `l1`, `level2` |

## 1. Explicit targets and a complete manifest: met

The author signed off on 2026-09-22 that these four manifests are the sole
authoritative mapping (spec human-review item 2, option A), so no second,
hand-reviewed record exists to drift from them.

- Each package's `Cargo.toml` sets `autotests = false` and lists its
  `[[test]]` targets.
- `acceptance/metadata-check.py` compares `cargo metadata` with each
  manifest's `targets` on name, path, and required features. It also checks
  `autotests = false`, and that every old target maps to exactly one module.
  Output: `acceptance/metadata-check.txt` (PASS, 139/74/53/38 old targets
  mapped). A planted extra target and a swapped required feature both make it
  exit 1 (implementation log, Phase 8).
- The criterion's third clause, no undeclared crate root, is enforced by the
  criterion 12 guard.

## 2. Identity preserved, four ways: met on macOS and Linux

`comparison-darwin-linux.{json,md}` in each package directory has verdict
`identical` and 0 failures. It compares selected, excluded-as-other-tier,
ignored, and platform-absent sets for every feature set and selector, test by
test, on macOS and Linux. Produced by `scripts/ci/consolidation.py compare`
from the committed `capture-*` listings. Native Windows is covered by
criterion 4, which is pending.

## 3. Required-feature contracts kept: met

`acceptance/metadata-check.py` checks each target's exact required features
against the manifest. Darkmatter's Level 3 tests stay in two targets:
`level3-terminal` needs `terminal-tests` and `level3-browser` needs
`browser-tests`. A swapped feature on `level3-browser` fails the check.

## 4. Platform conditions and platform proof: partly pending

- **Met: conditions moved onto module declarations.**
  `attribute-check.json` in each package directory
  (`consolidation.py check-attributes`) has 0 failures.
- **Met: macOS and Linux compile and list.** The same comparisons as
  criterion 2. Linux ran on `build-linux`.
- **Met: native Windows, on-host.** Every package's consolidated suites were
  run on `build-win-native` with `just cross-check <package> --os windows` on
  2026-09-22, the first time that leg has ever worked (see "What the Windows
  run cost"):

  | Package | Windows | WSL2 |
  |---|---|---|
  | `claudine-cli` | 2,250 passed, 11 skipped | 2,767 passed, 241 skipped |
  | `darkmatter` | 6,478 passed, 1 pre-existing failure, 68 skipped | 6,493 passed, same failure, 68 skipped |
  | `darkmatter-cli` | 717 passed, 69 skipped | 716 passed, 69 skipped |
  | `biscuit-terminal` | 2,923 passed, 58 skipped | 2,940 passed, 60 skipped |

  The one `darkmatter` failure on both hosts is the pre-existing
  `schema_phase_validation::public_docs_and_skill_describe_required_and_eager_as_independent_axes`
  named in §8, which Phase 4 proved fails identically on the unmigrated base.
- **Met: WSL2 archive path.** The `--os wsl` leg produces the `ubuntu-latest`
  record and consumes it as `wsl2-ubuntu` from a second checkout, which is the
  relocation this criterion asks for. Column two above.

**How it was closed.** `cross-check` now keeps one clone per origin worktree
under the host's `CODING_DIR` (`B:\coding` on `build-win-native`, 173 GB
free), so the full `W:` that blocked this no longer decides whether the leg
can run.

### What the Windows run cost

The leg had never run to completion before, so it found four defects of its
own. None belongs to this migration; all are fixed in this change:

1. `scripts/cross-check.sh` looked for `ci-build.exe` under
   `<clone>\scripts\target\release`. `scripts` is a root-workspace member,
   so its binaries are in the workspace `target\`.
2. Git for Windows refused this repository's deepest paths (`Filename too
   long`) until the clone sets `core.longpaths true`.
3. Every Unix leg reported the exit status of the host's `~/.bash_logout`
   rather than the run's, because the run script was handed to a login shell.
   A green WSL run was summarized `FAIL` — the symptom the `os` skill recorded
   on 2026-09-15 without a cause. Fixed by `bash -lc`.
4. `AssignProcessToJobObject` was fatal in `windows_wait_loop`, and an SSH
   session's processes are already inside a Job that forbids nesting, so every
   provider launch under `sequence_budget` failed with `Access is denied.
   (0x80070005)`. It now degrades to terminating the child alone. This one is
   a real Claudine defect, not a harness artifact: it breaks any Windows run
   under SSH.

A fifth was a test defect this run exposed: `wrap_compose_validation`'s
Windows provider stub was a `goose.cmd`, and Rust refuses to spawn a batch
file with a newline-bearing argument — which Claudine's composed prompt always
is. It is now a compiled `.exe`, as every other Claudine provider fixture
already was.

## 5. Test bodies unchanged: met, for reviewer confirmation

`acceptance/body-diff.md` (`acceptance/body-diff.py`) diffs every old file at
its base against its new module at `HEAD`. It sorts each changed line as
structural, comment, or other, and lists every "other" line. Those fall into
four groups, all allowed by spec §3 or recorded in the log:

- `include_str!`/fixture path repairs for the extra directory level,
- guard path sets and exclusions that now carry the tier directory
  (`darkmatter-cli` spawn-site guard, Phase 5 "Spawn-guard exclusion follows
  the tier directory"; claudine `error_guards` allowlist paths),
- rewritten selector and path text in failure messages (Phase 7),
- claudine's `test_placement.rs` layout gate: new code, the criterion 12
  guard, declared as manifest `additions`.

No assertion's expected value, input, timeout, or skip decision changed. The
criterion 2 comparison shows the same tests, in the same tier, ignored the same
way. The spec makes this a review criterion, so the reviewer should read the
"other lines" list.

`body-diff.py` reads each module at `HEAD`, so its record does not yet include
the review-1 fixes, which are uncommitted as this is written. Re-run it after
they land: it will add `wrap_compose_validation`'s Windows provider stub (a
`goose.cmd` replaced by a compiled `.exe`, §4) as a fifth "other" group, and
claudine's `test_placement.rs` layout gate — already classified as manifest
`additions` — loses its duplicated module-graph parser to
`test_toolkit::test_layout`.

## 6. Snapshots: met

`snapshot-mapping.json` and `snapshot-check.json` in each package directory
(`consolidation.py check-snapshots`) show 0 failures:

| Package | Moved | Unaffected |
|---|---:|---:|
| `claudine-cli` | 3 | 4 |
| `darkmatter` | 200 | 1 |
| `darkmatter-cli` | 0 | 0 |
| `biscuit-terminal` | 1,086 | 117 |

Snapshot contents are byte-identical. The final sweep ran with
`INSTA_UPDATE=no`, and no `.snap.new` file exists in the tree (checked
2026-09-22).

## 7. Path-based guards cover the same files: met

`guard-scans-after.md` in each package directory lists every scanned path
before and after. The baseline is `baseline/guard-scans.md`. The only
additions are the new crate roots and the layout-gate module. Nothing is
missing and no file changed eligibility. The guards are the claudine and
darkmatter-cli spawn-site guards, and the repository archive-path guard.

## 8. Local suites: met, with pre-existing failures named

Phase 8 sweep, 2026-09-22 on macOS: `INSTA_UPDATE=no just test claudine
darkmatter biscuit-terminal` ran 18,819 tests. 18,818 passed, 1 failed (the
pre-existing failure below), and 73 were skipped. All four criterion 12 guards
passed. `just lint` passed in all three areas, and `just check-canonical`
passed in all 27.

| Area | `just test-l2` | `just test-browser` | `just check-tier-coverage` |
|---|---|---|---|
| `claudine` | 231 + 3 (Phase 3) | n/a | pass (Phase 8) |
| `darkmatter` | 3 + 18 + 69 (Phases 4, 5) | 43/43 (Phase 4) | 1 stranded, pre-existing (Phase 8) |
| `biscuit-terminal` | lib 2/2, CLI 76/76 (Phase 6) | 54/54 (Phase 6) | pass (Phase 8) |

`test-l2` and `test-browser` were not re-run in Phase 8. No test source
changed since Phase 7, and the Phase 7 edits were comments and messages.

Pre-existing, not caused by the migration, and proven identical on the
unmigrated base in Phase 4:

- `darkmatter::l1 schema_phase_validation::public_docs_and_skill_describe_required_and_eager_as_independent_axes`
  reads `darkmatter/docs/topics/schema-definition.md`, which `aa1f03c70`
  deleted.
- `just check-tier-coverage darkmatter` reports one stranded `real_`-named L1
  test (`schema_phase_validation::real_shipped_inline_schema_uses_normal_resolution_and_phase_path`).

Level 3 and real-provider tests keep their opt-in: `level3`/`real` need
`terminal-tests`/`real-tests` and the `RUN_LEVEL3`/real gates, which are
unchanged. L2 runs used the WezTerm backend without taking focus.

## 9. Pilot measured: met

`measurements.md` records the before and after series (clean build, warm
edit-to-one-test latency over five alternating trials, peak compiler memory,
target count, executable size) and the seam decision. Median edit latency rose
0.30 s (+12%), below the split guardrail of more than 50% and more than 5 s, so
there is one binary per execution contract.

## 10. CI observations: pending

`ci-observations.md`. The branch has not been pushed, so no ordinary producer
run has selected a migrated package, and the spec forbids triggering one. The
file has the pull request 92 context and the harvest procedure.

## 11. Docs honest: met

- `baseline/consumer-sweep.py --after` reports **0 active**
  `--test <old-binary>` hits. Output: `baseline/test-selector-consumers-after.md`.
  The remaining hits are records, annotated with dates in
  `baseline/test-selector-consumers.md`.
- The `rust-testing` skill (commit `f501b107f`,
  `.claude/skills/rust-testing/SKILL.md` §"Consolidated Integration-Test
  Binaries") documents the layout, positional name filtering, the feature
  boundary, and the two-sided process-isolation contract. An independent
  reviewer passed it in Phase 7.

## 12. Structural guard inside each Level 1 binary: met

| Package | Guard test (in `l1`) |
|---|---|
| `claudine-cli` | `test_placement::every_test_source_is_compiled_by_a_declared_target`, plus 2 layout-gate tests |
| `darkmatter` | `test_layout::every_test_source_is_compiled_by_a_declared_target` |
| `darkmatter-cli` | `test_layout::every_test_source_is_compiled_by_a_declared_target` |
| `biscuit-terminal` | `test_layout::every_test_source_is_compiled_by_a_declared_target` |

All four guards call the shared `test_toolkit::test_layout`, which has 12 unit
tests. Claudine kept a byte-for-byte copy of the walker until 2026-09-22, when
review 1 found that both copies read a `mod name;` token inside an unexpanded
macro as a real declaration — which marks the file reachable and lets an
orphaned test source pass the gate. The shared walker now blanks macro token
trees before the scan, two regressions cover it (one on the parser, one on a
whole layout), and Claudine's duplicate is deleted in favor of the shared
implementation, so the two cannot drift again. Each guard went red on a planted stray root and an
undeclared module, then green (Phases 3–6). The three shared-gate packages
were also shown red on an undeclared `tests/level9/main.rs`.
None is a new binary or CI gate. All four passed in the Phase 8 sweep.
