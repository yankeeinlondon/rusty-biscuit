---
kind: acceptance
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-22
verdict: met-with-pending
pending:
    - acceptance 4, native Windows on-host listings (compile evidence only)
    - acceptance 4, WSL2 archive path
    - acceptance 10, CI producer observations
---

# Acceptance record: consolidated test binaries

This walks the twelve acceptance criteria in `spec.md`. Each row names the
committed artifact that shows it, and the command that re-runs it where one
exists. Paths are relative to this feature directory unless they start at the
repository root.

**Summary.** Nine criteria are met. Criterion 5 is met as far as a tool can
show, and it still needs the reviewer's reading, as the spec says. Two are
partly pending, and nothing pending is counted as a pass:

- **4:** native-Windows on-host listings and the WSL2 archive path. Both are
  blocked by free space on the Windows build host.
- **10:** CI observations. No ordinary run has selected the packages yet.

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
- **Pending: native Windows.** The evidence is compile-only. Every package's
  consolidated targets cross-compile for `x86_64-pc-windows-gnu` with 0
  warnings (implementation log, Phases 3–6). There are **no on-host Windows
  listings**, so the criterion's "listings" clause is not met for Windows.
  `just cross-check <package> --os windows` refuses at the storage preflight.
  `build-win-native`'s `W:` drive had 36.77 GB free on 2026-09-22, below the
  50 GiB floor, and the recipe's reclaim found nothing to remove.
- **Pending: WSL2 archive path.** No qualifying evidence. On 2026-09-22, SSH to
  `build-win` reset at key exchange. The `os` skill ties that symptom to the
  same full `W:` drive, since the guest's VHDX lives there. No ordinary nightly
  run has built these packages yet.

**To close:** free `W:`, then for each package run `consolidation.py capture`
on-host for the base and migrated trees and `compare` with the darwin and
linux captures. Then run `just cross-check <package> --os windows` and
`--os wsl`.

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
| `claudine-cli` | `test_placement::every_test_source_is_compiled_by_a_declared_target`, plus 3 layout-gate tests |
| `darkmatter` | `test_layout::every_test_source_is_compiled_by_a_declared_target` |
| `darkmatter-cli` | `test_layout::every_test_source_is_compiled_by_a_declared_target` |
| `biscuit-terminal` | `test_layout::every_test_source_is_compiled_by_a_declared_target` |

The three non-claudine guards call the shared `test_toolkit::test_layout`,
which has 10 unit tests. Each guard went red on a planted stray root and an
undeclared module, then green (Phases 3–6). The three shared-gate packages
were also shown red on an undeclared `tests/level9/main.rs`.
None is a new binary or CI gate. All four passed in the Phase 8 sweep.
