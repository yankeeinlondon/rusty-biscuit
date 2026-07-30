---
title: What `just/devops.just` must expose for the wsl2-ubuntu leg
status: requirements
created: 2026-07-27
owner: "@yankeeinlondon"
consumers:
  - .github/workflows/_wsl-ci.yml
blocks:
  - plan.md §3.3 (WSL2 as a first-class environment)
---

# `just/devops.just` requirements for `nextest archive` runs

`_wsl-ci.yml` implements plan decision 2.2: build `x86_64-unknown-linux-gnu` once
on `ubuntu-latest` with `cargo nextest archive`, then *run* those binaries inside
a WSL2 Ubuntu guest. No rustup, no toolchain, no compile in the guest, and
binaries byte-identical to the Linux leg's.

The consequence that drives everything below: **the guest has no Cargo and no
rustc.** `cargo-nextest` is a standalone binary and can execute an archive
without a Rust installation, but anything in the canonical recipe chain that
reaches for `cargo` breaks there.

`_wsl-ci.yml` has a step named *"Verify the archive passthrough is wired"* that
greps `just/devops.just` for the three environment variables below and fails with
a pointer to this file if they are absent. Wiring them makes that step pass; no
workflow change is needed afterwards.

## R1 — `--archive-file` and `--workspace-remap` must reach nextest verbatim

`_wsl-ci.yml` invokes:

```bash
cd "$GUEST_CHECKOUT/<area>"
just test --no-fail-fast \
  --archive-file "$archive" \
  --workspace-remap /home/runner/rusty-biscuit
```

`just test` → `_test_all` → `_run_all` → `_test <pkg> <args>` → `cargo nextest run`.
The flags must arrive at `cargo nextest run` unmodified and unre-quoted. This may
already hold via the existing `*args` chain — please confirm it survives
`_run_all`'s spec splitting rather than assuming it, and add a recipe-level test.

`--workspace-remap` is not optional. The archive records the *build-host* source
paths (`/home/runner/work/rusty-biscuit/rusty-biscuit`); the guest checkout lives
at `/home/runner/rusty-biscuit`. Without the remap, every test that resolves a
fixture relative to the workspace root fails with a confusing missing-path error.

## R2 — `BISCUIT_NEXTEST_BIN` (default `cargo nextest`)

`_test` currently probes and invokes nextest as a Cargo subcommand:

```bash
if cargo nextest --version >/dev/null 2>&1; then
    ...
    cargo nextest run -p {{ pkg }} -E "$filter" --no-tests=pass {{ args }}
```

Both the probe and the invocation need to go through one overridable variable:

```bash
nextest_bin=(${BISCUIT_NEXTEST_BIN:-cargo nextest})
if "${nextest_bin[@]}" --version >/dev/null 2>&1; then
    ...
    "${nextest_bin[@]}" run -p {{ pkg }} -E "$filter" --no-tests=pass {{ args }}
```

`_wsl-ci.yml` exports `BISCUIT_NEXTEST_BIN="cargo-nextest nextest"`, which is the
documented standalone invocation. Without this the probe fails (no `cargo` on
PATH), `_test` silently falls back to `cargo test`, and the leg dies with
"cargo: command not found" instead of running the suite.

The same treatment is needed in `_test_l2`, `_test_l3`, `_test_browser`,
`_test_real`, and `_sanity` for consistency, but only `_test` is load-bearing for
this leg.

## R3 — `BISCUIT_JUNIT_WORKSPACE_ROOT` / `BISCUIT_JUNIT_TARGET_DIR`

`_stage_junit` and `_stage_junit_reset` derive their paths from
`cargo metadata --no-deps`, and both `exit 0` when that command is unavailable.
In the guest that means the staging is skipped **silently**: no JUnit copy and no
manifest record, so the `wsl2-ubuntu` cell reports MISSING for a run that
actually executed. That is precisely the failure mode plan §0.2 exists to
eliminate.

Both recipes should prefer explicit values when given:

```bash
workspace_root="${BISCUIT_JUNIT_WORKSPACE_ROOT:-}"
target_dir="${BISCUIT_JUNIT_TARGET_DIR:-}"
if [[ -z "${workspace_root}" || -z "${target_dir}" ]]; then
    # existing `cargo metadata` derivation
fi
```

`_wsl-ci.yml` exports:

```
BISCUIT_JUNIT_WORKSPACE_ROOT=/home/runner/rusty-biscuit
BISCUIT_JUNIT_TARGET_DIR=/home/runner/rusty-biscuit/target
```

`BISCUIT_CI_AREA`, `BISCUIT_CI_ENVIRONMENT` (`wsl2-ubuntu`) and `BISCUIT_CI_SHARD`
are already exported by the job, so the manifest records land with the right
identity once staging runs at all.

## R4 — archive mode must not fall back to `cargo test`

When `--archive-file` is present, the `cargo test` fallback branch cannot work
(it recompiles). It should fail loudly with the reason rather than emit a
misleading compile error. A one-line guard at the top of `_test` is enough:

```bash
if [[ " {{ args }} " == *" --archive-file "* ]] && ! "${nextest_bin[@]}" --version >/dev/null 2>&1; then
    echo "--archive-file requires cargo-nextest; set BISCUIT_NEXTEST_BIN" >&2
    exit 1
fi
```

## Caveat — feature flags in archive mode

`--features` is a *build* flag. An area whose `just test` passes build flags
per package — `messenger` does: `_test_all "messenger --features desktop; messenger-cli"`
— cannot forward them to an archive run, because the archive was already built.
Those features must instead be baked into the archive build, which is why
`areas.json` carries them on `check_args`
(`-p messenger -p messenger-cli --features messenger/desktop`) and `_wsl-ci.yml`
passes `check_args` to `cargo nextest archive`.

Two options for the recipes, in preference order:

1. Have `_test` drop build-only flags (`--features`, `--all-features`,
   `--no-default-features`) when `--archive-file` is present, and log that it did.
2. Keep them and accept that any area with per-package feature flags cannot use
   the archive path until its recipe is restructured.

Option 1 keeps "the same canonical recipe" true for every area. Option 2 limits
the `wsl2-ubuntu` environment to feature-flag-free areas, which today excludes
`messenger` — the one area whose WSL coverage was the original motivation.

## L2 is deliberately out of scope

`wsl2-ubuntu` is absent from `L2_PROVISIONED_ENVIRONMENTS` in
`scripts/ci/affected_scope.py`. A `nextest archive` carries test binaries and the
non-test binaries they depend on, but the L2 tier additionally needs a live tmux
server and the shared-harness broker's re-exec path, neither of which has been
demonstrated to survive `--workspace-remap`. No `apt-get install tmux` is
performed in the guest, because installing a backend that no test can reach is
exactly the "installed tmux plus zero tmux tests" non-evidence plan §1.1 rejects.

Re-scope WSL L2 as separate work once the L1 leg has run green at least once.
