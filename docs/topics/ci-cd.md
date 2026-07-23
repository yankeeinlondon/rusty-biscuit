# CI/CD in Rusty Biscuit

## Overview

Rusty Biscuit's CI/CD runs on **GitHub Actions** and is layered to match the [testing tier
taxonomy](./testing-in-rusty-biscuit.md): fast feedback first, then full coverage, then
nightly/advisory work. Releases are automated through [release-plz](https://release-plz.dev) but
**no crate is published to crates.io** — GitHub releases and version tags are the only
distribution channel today.

Package-area gates use the same `just` recipes developers run locally. Scope selection is handled
by `scripts/ci/affected_scope.py`, which uses Cargo metadata to expand changed workspace packages
through reverse dependencies before platform jobs start.

## Pipeline Layers

The pipeline has four conceptual layers. Each layer answers a different question.

| Layer                     | Question it answers                                               | Blocking?                      |
|---------------------------|-------------------------------------------------------------------|--------------------------------|
| **Local pre-push hook**   | Did I obviously break the areas I touched?                        | Opt-in (`warn`/`strict`/`off`) |
| **Dependency-scoped CI**  | Do changed packages and their consumers pass on native runners?   | Yes                            |
| **Affected coverage**     | Did the changed package closure lose exercised behavior?          | Report-only                    |
| **Nightly / advisory**    | Did anything drift since yesterday?                               | No                             |

### Layer 1 — Local pre-push hook

`.githooks/pre-push` runs `just pre-push <areas>` before `git push` completes. It is controlled by
`RUSTY_BISCUIT_PRE_PUSH`:

- `off` — skip entirely
- `warn` — run, report, but never block the push (the default)
- `strict` — run and block the push on failure

The hook picks areas dynamically via `just changed-areas` against the upstream branch, falling back
to `RUSTY_BISCUIT_PRE_PUSH_AREAS` and finally a hardcoded short list. Install it once with:

```bash
ln -s ../../.githooks/pre-push .git/hooks/pre-push
```

The hook itself is regression-tested in CI by `hooks-tests.yml`, so changes to `.githooks/**`,
`justfile`, or `just/**` re-validate the contract.

### Layer 2 — Dependency-scoped CI

`ci.yml` runs on pull requests and pushes to `main`. Its first job validates the canonical recipe
surface, obtains the changed file set from the event's exact base and head SHAs, and calculates:

- workspace packages containing those files,
- all transitive reverse Cargo dependencies,
- curated package areas owning those packages.

The selected area matrix calls `_area-ci.yml`, which compile-checks macOS, runs L1 on Linux and
Windows, and runs each area's lint/documentation guards. Area-specific policy—strict versus soft
Windows, sharding, L2, browser tests, and provider stubs—lives in `.github/ci/areas.json`.
Claudine's generator/signals guard and Darkmatter's `NO_COLOR` guard remain conditional jobs in
the same workflow. Changes to global build/test configuration conservatively select every area.

### Layer 3 — Affected coverage and specialized workflows

On pull requests, `ci.yml` passes the affected package closure to one `cargo llvm-cov` invocation
and uploads one LCOV artifact. It does not perform a package-by-package pass and then repeat the
workspace. Path-filtered specialized workflows such as hooks, Messenger desktop behavior,
Rendezvous IPC, and Sniff performance remain separate because they test contracts outside the
canonical area matrix.

### Layer 4 — Nightly and advisory

#### `coverage.yml` — nightly and manual, report-only

Runs one `cargo llvm-cov --workspace` command and uploads the aggregated LCOV artifact. Coverage
**does not gate merges**; PRs already receive an affected-scope report from `ci.yml`.

#### `bench-nightly.yml` — 00:00 UTC daily + `darkmatter/**` pushes to `main`

Runs darkmatter's Criterion suite and uploads results to
[Bencher.dev](https://bencher.dev) (`BENCHER_PROJECT` repo variable). Uses `--err` so the run fails
on regression, but the failure is informational — it does not retroactively gate the commit
that introduced it.

#### `fuzz-nightly.yml` — 02:00 UTC daily

Two matrix jobs (`biscuit-file`, `darkmatter`) on the **nightly** toolchain, capped at 10,000 runs
and 300 s per target. The interesting policy bits:

- **Replay-first.** Committed crash corpora (`fuzz/artifacts/<target>/`) are replayed with
    `-runs=0` before any new fuzzing. A regression in a previously-fixed crash fails the run
    immediately and loudly.

- **Auto-issue on new crash.** When a *new* crash is found, the workflow opens a GitHub issue
    de-duplicated by target marker + crash signature. Reproduction instructions are embedded in the
    issue body.

- **Advisory.** Fuzz nightly never gates merges; it produces actionable issues instead.

#### `build-integrations.yml` — on `release: published`

Cross-compiles the three Unfolded Circle integrations (`arcam-amp-integration`,
`eversolo-integration`, `sony-receiver-integration`) to `aarch64-unknown-linux-musl` using `cross`,
then uploads tarballs to the GitHub release via `gh release upload`. `fail-fast: false` so one
target's failure doesn't strand the others.

## Release Strategy

Releases are automated end-to-end by `release-plz.yml` in the public repository.

### Two-job flow

1. On each push to `main`, **`release-pr`** runs `release-plz release-pr`. It opens (or updates) a single **draft**
   release PR labeled `release`, `automated`. The PR contains version bumps and changelog updates
   for every package with relevant commits since the last tag. Concurrency is configured to be
   **non-cancelling** so two concurrent runs cannot race the PR head.

2. When a merged PR labeled `release` closes, **`release-plz-release`** runs `release-plz
   release`. It creates git tags shaped `{{ package }}-v{{ version }}` and publishes GitHub
   releases with the rendered changelog. Ordinary pushes do not start the publishing job.

### What we do *not* do

- **No crates.io publishing.** `publish = false` is set workspace-wide in `release-plz.toml`. If a
    crate ever needs to ship to crates.io, that's an explicit, per-package decision.

- **No version bumping on PRs that don't touch a published package.** `release-plz` is
    conventional-commit aware (`feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `chore(deps)`,
    `chore`) and skips `chore(release)`.

- **No release for excluded packages.** `biscuit-tui`, `biscuit-tui-cli`, `tabby`, and the `ui`
    package opt out.

### Versioning policy

- **SemVer checks are disabled.** `semver_check = false` avoids release-plz regenerating
    intentionally untracked nested-workspace lockfiles; with `publish = false`, there is no
    crates.io consumer requiring that publication gate.

- **Per-package changelogs** at `<area>/CHANGELOG.md` for nine packages (the rest aggregate into the
    workspace root changelog).

- **Conventional commit prefixes** drive both the bump level and the changelog section. See the
    `commit_parsers` array in `release-plz.toml` for the canonical mapping.

## Caching and Performance

Every Rust workflow uses `Swatinem/rust-cache@v2` with `workspaces: ". -> target"` and a workflow-
or matrix-scoped `shared-key`. Examples: `area-ci-<area>-<job>-<os>`, `coverage-affected`,
`coverage`, `bench-nightly-darkmatter`, `sniff-bench`, and `messenger-desktop-<os>`. Cache keys are
intentionally per-workflow rather than global — this trades hit rate for protection against
a poisoned target directory taking down the entire pipeline.

Concurrency is configured per-workflow:

- Dependency-scoped CI: `cancel-in-progress: true` per ref, so force-pushing a fix cancels
    the prior run.

- `release-plz` and the fuzz/bench nightlies: **never cancel** — partial state from these is
    always more useful than nothing.

## Required Toolchain on Runners

Every workflow pins one toolchain explicitly — no implicit `rustup default`.

- **Stable** for dependency-scoped CI, coverage, build-integrations, and release-plz.
- **Stable + `llvm-tools-preview`** for coverage.
- **Nightly** only for `fuzz-nightly`.

Shared CLI tools used in CI:

- `cargo-nextest` — the canonical test runner for L1 tiers.
- `just` — orchestration entry point for every job.
- `cargo-llvm-cov` — coverage.
- `cargo-fuzz` — fuzz targets.
- `cross` — integration cross-compilation.
- `bencher` — nightly benchmark upload.

## Policy Summary

What a reviewer can rely on when approving a PR:

1. **Every affected curated area passed its configured native matrix against stable Rust.**
2. **`just check-canonical` confirms the area structure is well-formed** — no `justfile`
   recipe drift snuck in.

3. **Downstream workspace consumers are included automatically** from Cargo's dependency graph;
   specialized path-filtered contracts run when their own paths match.

4. **Coverage is reported but not gated.** Treat coverage as a delta to inspect, not a number to
   defend.

5. **Bench and fuzz drift is captured nightly,** not on the PR itself. A regressed fuzz target
   files an auto-issue rather than blocking your merge.

6. **Releases never happen from a PR branch.** They happen from `main` via release-plz's draft
   PR, which is itself reviewed before merge.

What CI explicitly does **not** guarantee:

- **Full macOS runtime coverage for every area.** Most areas compile-check macOS and run L1 on
    Linux and Windows; Sniff and specialized workflows opt into broader native runtime evidence.

- **Performance regressions blocking merge.** Bench results are tracked in Bencher but not gated.
- **External-resource (L4 `test-real`) tests passing.** Those tiers are explicitly excluded from
    CI; they live on developer machines and the homelab.

## Adding a New Workflow

Before adding a new workflow, check:

1. **Does an existing canonical recipe cover this?** If yes, register the area in the root
   `justfile` and `.github/ci/areas.json` instead of inventing a new workflow.

2. **Is this a canonical area contract or a specialized contract?** Canonical work belongs in
   `_area-ci.yml` and the area policy; specialized hardware, IPC, performance, or hook behavior
   may justify a small path-filtered workflow.

3. **Should this gate merges or just report?** Mirror the existing pattern — coverage,
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
