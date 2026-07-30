---
title: How the nextest-archive passthrough landed, and where the requirements were wrong
status: implemented
created: 2026-07-28
owner: "@yankeeinlondon"
implements:
  - wsl-archive-requirements.md (R1–R4)
  - plan.md §2.2, §3.3
source_code:
  - just/devops.just
---

# `just/devops.just` archive passthrough — implementation notes

R1–R4 from `wsl-archive-requirements.md` are implemented. This document records
the one place the requirements were factually wrong, the shape of the fix, and
the evidence.

## R1 did **not** already work — `-p` and `--archive-file` are mutually exclusive

The requirement said the passthrough "may already hold via the existing `*args`
chain" and asked for empirical confirmation. It does not hold. The flags *do*
arrive at `cargo nextest run` verbatim — the `*args` chain through
`test → _test_all → _run_all → _test` is fine — but nextest rejects the
resulting command line, because `_test` also passes `-p <pkg>`:

```
$ just test --no-fail-fast --archive-file …/probe.tar.zst --workspace-remap /Volumes/coding/personal/rusty-biscuit
Testing worktree package

ℹ️ using cargo nextest run
error: the argument '--archive-file <PATH>' cannot be used with:
  --package <PACKAGES>
  --workspace
  --exclude <EXCLUDE>
  --all
  --lib
  --bin <BIN>
  --bins
  …
  --features <FEATURES>
  --all-features
  --no-default-features
  …
```

The exclusion is not a quirk of `-p`: it is *every Cargo build flag*, because the
binaries already exist. So the fix and the `--features` caveat are the same fix.

**Package selection moves into the filterset.** In archive mode `_test` drops
`-p` and runs

```
-E 'package(<pkg>) & (<the tier filter>)'
```

which selects exactly the same tests. Verified: the non-archive run and the
archive run of `worktree-cli` both execute 92 tests.

## R2 — `BISCUIT_NEXTEST_BIN`

`read -r -a nextest_bin <<< "${BISCUIT_NEXTEST_BIN:-cargo nextest}"`, used by
both the availability probe and the invocation, in `_test`, `_sanity`,
`_test_l2`, `_test_l3`, `_test_browser`, `_test_real`, and `_expected_manifest`.

## R3 — `BISCUIT_JUNIT_WORKSPACE_ROOT` / `BISCUIT_JUNIT_TARGET_DIR`

`_stage_junit` and `_stage_junit_reset` prefer the explicit values and only fall
back to `cargo metadata`.

The dangerous half of the old behaviour was the *silent* `exit 0`. Staging still
never changes a test invocation's outcome — a `set -e` failure there would turn a
green run red *after* the tests passed, and would break every dev host without
`jq` — but it is now impossible to miss. `_junit_staging_unavailable` prints:

```
⚠️  JUnit staging skipped in _stage_junit: cargo metadata is unavailable
   No report and no manifest record will be produced for this invocation,
   so a CI rollup will score this cell MISSING even though the tests ran.
   Set BISCUIT_JUNIT_WORKSPACE_ROOT and BISCUIT_JUNIT_TARGET_DIR to skip the
   cargo-metadata derivation entirely (see fixes/2026-07-27-refactor/wsl-archive-requirements.md).
```

It fires for a missing `cargo metadata`, an unusable metadata document, and a
missing `jq` (which the manifest record needs and which the original code also
swallowed silently).

**Where the report actually lands in archive mode**, since this determines
whether `BISCUIT_JUNIT_TARGET_DIR` is the right value: nextest derives its store
directory from the **remapped** workspace root, not from the extraction
directory. A run with `--workspace-remap /Volumes/coding/personal/rusty-biscuit`
wrote `/Volumes/coding/personal/rusty-biscuit/target/nextest/ci/test-results.xml`
and nothing under `--extract-to`. So `_wsl-ci.yml`'s
`BISCUIT_JUNIT_TARGET_DIR=/home/runner/rusty-biscuit/target` is correct as
written.

## R4 — archive mode never falls back to `cargo test`

A guard at the top of `_test`: if `--archive-file` is present and the nextest
binary does not answer `--version`, exit 1 with the reason. A `cargo test`
fallback recompiles, which is the one thing an archive exists to avoid.

## The `--features` caveat — resolved as option 1

`_archive_drop_build_flags` drops build-only flags in archive mode and logs what
it dropped:

```
  archive mode: dropped build-only flag(s) --features desktop --release — already baked into the archive
```

Dropped: `--features`, `--all-features`, `--no-default-features`, `--release`,
`--target`, `--target-dir`, `--cargo-profile`, `--build-jobs`, `--lib`, `--bins`,
`--tests`, `--benches`, `--examples`, `--all-targets` (both `--flag value` and
`--flag=value` spellings).

This unblocks `messenger` — the area whose WSL coverage motivated the work —
without restructuring its recipe. The features are not lost: they are baked into
the archive at build time via `areas.json`'s `check_args`, which `_wsl-ci.yml`
already passes to `cargo nextest archive`. Outside archive mode the flags are
forwarded verbatim, unchanged.

**The correctness of this rests on an invariant nothing currently enforces:** the
`--features` an area's `test` recipe passes must be a subset of what its
`check_args` bakes into the archive. If they diverge, the archive run silently
tests a different feature set than the native run. Worth a contract test
comparing `areas.json` `check_args` against each area's `_test_all` spec; not in
scope here.

## Filter expressions are now single-sourced

`_tier_filter <tier> [pkg]` is the one place a tier's nextest filter is written.
`_test`, `_sanity`, `_test_l2`, `_test_l3`, `_test_browser`, `_test_real`, and
`_expected_manifest` all read it. This is a prerequisite for the expected-test
manifest: expected and observed are only comparable if one expression chose both.

It also absorbed the two pre-existing L1 special cases — `worktree-cli`'s extra
`perf_` exclusion and the `BISCUIT_TEST_FILTER` override — which were previously
inlined in `_test` and invisible to anything else.

## Evidence

All runs on macOS against a real `cargo nextest archive --archive-file … -p
worktree-cli` (11 binaries, 369 files).

**Guest simulation.** `PATH` reduced to `{just, jq, cargo-nextest}` plus
`/usr/bin:/bin:/usr/sbin:/sbin` — `command -v cargo` and `command -v rustc` both
report nothing, and `/usr/bin/env bash` resolves to **bash 3.2.57**, which is a
stricter portability bar than any CI runner.

| check | result |
|---|---|
| `just test … --archive-file … --workspace-remap …` in the guest | 92 tests run, 92 passed, both packages green |
| staged output | `L1/worktree.xml`, `L1/worktree-cli.xml`, `manifest.jsonl` with `report_present: true` for both |
| archive mode, `BISCUIT_NEXTEST_BIN` unset, no cargo | exit 1, `--archive-file requires cargo-nextest` |
| archive mode with `--features desktop --release` | flags dropped and logged; 92 tests still run |
| non-archive mode with `--features nope-does-not-exist` | still forwarded — `error: the package 'worktree-cli' does not contain this feature` |
| no `BISCUIT_JUNIT_*` overrides and no `cargo metadata` | loud warning ×2, staging directory empty |
| with the overrides | staging directory populated, manifest records written |

**Non-vacuity.** With the R4 guard neutered to `if false && …`, the same guest
invocation produced exactly the failure mode the requirements predicted:

```
ℹ️ using cargo test
/var/folders/…/_test: line 627: cargo: command not found
❌ The worktree-cli has failing tests!
```

The guard was restored immediately afterwards.

## Not done

- **L2 in archive mode.** Deliberately out of scope per the requirements;
  `_test_l2` gained `BISCUIT_NEXTEST_BIN` but not archive-mode package
  selection. Passing `--archive-file` to any tier other than `_test` will hit
  nextest's raw `cannot be used with --package` error.
- **`_sanity` in archive mode.** Same reason, plus `--lib`/`--bins` are
  themselves build flags.
- No `.github/workflows/**` file was touched.
