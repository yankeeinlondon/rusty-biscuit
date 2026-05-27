# CI/CD in Rusty Biscuit

## Overview

Rusty Biscuit's CI/CD runs on **GitHub Actions** and is layered to match the [testing tier
taxonomy](./testing-in-rusty-biscuit.md): fast feedback first, then full coverage, then
nightly/advisory work. Releases are automated through [release-plz](https://release-plz.dev) but
**no crate is published to crates.io** &mdash; GitHub releases and signed tags are the only
distribution channel today.

The local entry point for everything CI does is `just`. Every CI job ultimately shells out to a
`just` recipe defined in `justfile` or `just/*.just`, so a green local `just all <area>` is a strong
predictor of a green PR.

## Pipeline Layers

The pipeline has four conceptual layers. Each layer answers a different question.

| Layer | Question it answers | Blocking? |
|-------|--------------------|-----------|
| **Local pre-push hook** | Did I obviously break the areas I touched? | Opt-in (`warn`/`strict`/`off`) |
| **PR gates** | Does the canonical suite pass on a clean Linux runner? | Yes |
| **Area-scoped workflows** | Does the package area I changed still pass its own deeper checks? | Yes when triggered |
| **Nightly / advisory** | Did anything drift since yesterday? | No |

### Layer 1 &mdash; Local pre-push hook

`.githooks/pre-push` runs `just pre-push <areas>` before `git push` completes. It is controlled by
`RUSTY_BISCUIT_PRE_PUSH`:

- `off` &mdash; skip entirely
- `warn` &mdash; run, report, but never block the push (the default)
- `strict` &mdash; run and block the push on failure

The hook picks areas dynamically via `just changed-areas` against the upstream branch, falling back
to `RUSTY_BISCUIT_PRE_PUSH_AREAS` and finally a hardcoded short list. Install it once with:

```bash
ln -s ../../.githooks/pre-push .git/hooks/pre-push
```

The hook itself is regression-tested in CI by `hooks-tests.yml`, so changes to `.githooks/**`,
`justfile`, or `just/**` re-validate the contract.

### Layer 2 &mdash; PR gates (always required)

Two workflows run on every pull request to `main`. Both use `Swatinem/rust-cache@v2` with a
workflow-specific `shared-key` and pin the **stable** toolchain via `dtolnay/rust-toolchain`.

#### `sanity.yml` &mdash; fast confidence (~5 min budget, 10 min hard timeout)

Runs `just sanity` across the curated area list. `sanity` is the L1 subset that excludes slow tests,
L2 (terminal/PTY), L3 (OS injection), browser, and real-resource tiers. This is the first signal a
PR author sees and is intended to surface obvious breakage within a coffee break.

#### `test.yml` &mdash; canonical suite (30 min timeout)

Runs `just check-canonical` followed by `just all`, where `all` is the composite
`sanity &rarr; lint &rarr; doctest &rarr; test &rarr; test-l2 &rarr; test-browser`. `check-canonical`
asserts every curated area defines the full 12-recipe canonical surface; this is how the monorepo
keeps area `justfile`s uniform. Missing tiers in individual areas degrade gracefully &mdash; an area
with no L2 tests simply contributes a no-op.

### Layer 3 &mdash; Area-scoped workflows (path-filtered)

These workflows only trigger when their package area changes, so they add depth without taxing
unrelated PRs. Each owns its own cache key and 30 min timeout.

| Workflow | Scope | What it adds beyond the PR gate |
|----------|-------|----------------------------------|
| `claudine-tests.yml` | `claudine/**` | Drops stub provider CLIs (`claude`, `opencode`, `roo`, `gemini`, `aider`, `codex`, `goose`, `kimi`, `qwen`) into `$RUNNER_TEMP` so sniff detection succeeds in dry-run integration tests; pins `COLORTERM=truecolor` for snapshot fidelity. |
| `darkmatter-tests.yml` | `darkmatter/**` | Pins `COLORTERM=truecolor`; additionally re-runs color-depth tests under `NO_COLOR=1` to catch snapshot regressions. |
| `hooks-tests.yml` | `.githooks/**`, `justfile`, `just/**` | Runs `test-pre-push.sh` and `test-changed-areas.sh` with `fetch-depth: 0` so `git diff upstream..HEAD` works. |
| `messenger-desktop-tests.yml` | `messenger/**` | OS matrix across `ubuntu-latest`, `windows-latest`, `macos-latest`, **plus** a WSL2 (Ubuntu 24.04) submatrix via `Vampire/setup-wsl`. Installs `pkg-config`, `libssl-dev`, `libdbus-1-dev` on Linux. |
| `sniff-performance.yml` | `sniff/**` | Runs a narrow Criterion subset (`ci-bench-ids.txt`) with `--save-baseline ci`. Artifact-only Phase A &mdash; no regression gate yet. |

Path filters always include the workflow file itself and root `Cargo.toml`, so workflow edits and
dependency bumps still re-run the relevant suite.

### Layer 4 &mdash; Nightly and advisory

#### `coverage.yml` &mdash; on every PR and push to `main`, report-only

Runs `just coverage` across all areas with `cargo-llvm-cov` and uploads per-package and aggregated
LCOV files as artifacts. Coverage **does not gate merges**; it's a delta-watching tool.

#### `bench-nightly.yml` &mdash; 00:00 UTC daily + `darkmatter/**` pushes to `main`

Runs darkmatter's Criterion suite and uploads results to
[Bencher.dev](https://bencher.dev) (`BENCHER_PROJECT` repo variable). Uses `--err` so the run fails
on regression, but the failure is informational &mdash; it does not retroactively gate the commit
that introduced it.

#### `fuzz-nightly.yml` &mdash; 02:00 UTC daily

Two matrix jobs (`biscuit-file`, `darkmatter`) on the **nightly** toolchain, capped at 10,000 runs
and 300 s per target. The interesting policy bits:

- **Replay-first.** Committed crash corpora (`fuzz/artifacts/<target>/`) are replayed with
  `-runs=0` before any new fuzzing. A regression in a previously-fixed crash fails the run
  immediately and loudly.
- **Auto-issue on new crash.** When a *new* crash is found, the workflow opens a GitHub issue
  de-duplicated by target marker + crash signature. Reproduction instructions are embedded in the
  issue body.
- **Advisory.** Fuzz nightly never gates merges; it produces actionable issues instead.

#### `build-integrations.yml` &mdash; on `release: published`

Cross-compiles the three Unfolded Circle integrations (`arcam-amp-integration`,
`eversolo-integration`, `sony-receiver-integration`) to `aarch64-unknown-linux-musl` using `cross`,
then uploads tarballs to the GitHub release via `gh release upload`. `fail-fast: false` so one
target's failure doesn't strand the others.

## Release Strategy

Releases are automated end-to-end by `release-plz.yml`, which runs only on pushes to `main` in the
public repository.

### Two-job flow

1. **`release-pr`** runs `release-plz release-pr`. It opens (or updates) a single **draft**
   release PR labeled `release`, `automated`. The PR contains version bumps and changelog updates
   for every package with relevant commits since the last tag. Concurrency is configured to be
   **non-cancelling** so two concurrent runs cannot race the PR head.
2. **`release-plz-release`** runs `release-plz release` after that PR merges. It creates git tags
   shaped `{{ package }}-v{{ version }}` and publishes GitHub releases with the rendered changelog.

### What we do *not* do

- **No crates.io publishing.** `publish = false` is set workspace-wide in `release-plz.toml`. If a
  crate ever needs to ship to crates.io, that's an explicit, per-package decision.
- **No version bumping on PRs that don't touch a published package.** `release-plz` is
  conventional-commit aware (`feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `chore(deps)`,
  `chore`) and skips `chore(release)`.
- **No release for excluded packages.** `tui-chrome`, `tui-chrome-cli`, `tabby`, and the `ui`
  package opt out.

### Versioning policy

- **SemVer with `semver_check = true`.** release-plz fails the release PR if a package's public API
  changed incompatibly relative to its proposed bump.
- **Per-package changelogs** at `<area>/CHANGELOG.md` for nine packages (the rest aggregate into the
  workspace root changelog).
- **Conventional commit prefixes** drive both the bump level and the changelog section. See the
  `commit_parsers` array in `release-plz.toml` for the canonical mapping.

## Caching and Performance

Every Rust workflow uses `Swatinem/rust-cache@v2` with `workspaces: ". -> target"` and a workflow-
or matrix-scoped `shared-key`. Examples: `sanity`, `test`, `coverage`, `bench-nightly-darkmatter`,
`claudine-tests`, `darkmatter-tests`, `sniff-bench`, `messenger-desktop-<os>`. Cache keys are
intentionally per-workflow rather than global &mdash; this trades hit rate for protection against
a poisoned target directory taking down the entire pipeline.

Concurrency is configured per-workflow:

- PR gates and area workflows: `cancel-in-progress: true` per ref, so force-pushing a fix cancels
  the prior run.
- `release-plz` and the fuzz/bench nightlies: **never cancel** &mdash; partial state from these is
  always more useful than nothing.

## Required Toolchain on Runners

Every workflow pins one toolchain explicitly &mdash; no implicit `rustup default`.

- **Stable** for sanity, test, coverage, area-scoped, build-integrations, and release-plz.
- **Stable + `llvm-tools-preview`** for coverage.
- **Nightly** only for `fuzz-nightly`.

Shared CLI tools used in CI:

- `cargo-nextest` &mdash; the canonical test runner for L1 tiers.
- `just` &mdash; orchestration entry point for every job.
- `cargo-llvm-cov` &mdash; coverage.
- `cargo-fuzz` &mdash; fuzz targets.
- `cross` &mdash; integration cross-compilation.
- `bencher` &mdash; nightly benchmark upload.

## Policy Summary

What a reviewer can rely on when approving a PR:

1. **Sanity and test passed on `ubuntu-latest` against stable Rust.** Both are required.
2. **`just check-canonical` confirms the area structure is well-formed** &mdash; no `justfile`
   recipe drift snuck in.
3. **If the PR touched a path-filtered area (`claudine`, `darkmatter`, `messenger`, `sniff`,
   `.githooks`),** the corresponding area-scoped workflow also passed.
4. **Coverage is reported but not gated.** Treat coverage as a delta to inspect, not a number to
   defend.
5. **Bench and fuzz drift is captured nightly,** not on the PR itself. A regressed fuzz target
   files an auto-issue rather than blocking your merge.
6. **Releases never happen from a PR branch.** They happen from `main` via release-plz's draft
   PR, which is itself reviewed before merge.

What CI explicitly does **not** guarantee:

- **Cross-platform coverage for arbitrary areas.** Only `messenger` runs the full Linux/macOS/
  Windows/WSL matrix today; other areas are validated on Linux only.
- **Performance regressions blocking merge.** Bench results are tracked in Bencher but not gated.
- **External-resource (L4 `test-real`) tests passing.** Those tiers are explicitly excluded from
  CI; they live on developer machines and the homelab.

## Adding a New Workflow

Before adding a new workflow, check:

1. **Does an existing canonical recipe cover this?** If yes, prefer adding the area to the
   curated list in the root `justfile` over inventing a new workflow.
2. **Is this area-scoped or repo-wide?** Repo-wide work goes in `sanity.yml` / `test.yml`; area
   work gets a `<area>-tests.yml` with path filters that include the workflow file itself and
   root `Cargo.toml`.
3. **Should this gate merges or just report?** Mirror the existing pattern &mdash; coverage,
   bench, fuzz, and sniff-performance are non-gating; everything else is.
4. **Pick a unique `shared-key`** for the cache so you don't share state with an unrelated job.
5. **Pin the toolchain explicitly** with `dtolnay/rust-toolchain`; never rely on the runner's
   default.

## Pointers

- Workflow definitions: `.github/workflows/`
- Release config: `release-plz.toml`
- Test tier taxonomy: [`testing-in-rusty-biscuit.md`](./testing-in-rusty-biscuit.md) and
  `.claude/skills/rust-testing/SKILL.md`
- Pre-push hook: `.githooks/pre-push`, tested by `.githooks/tests/`
- Canonical recipe definitions: root `justfile` and `just/*.just`
