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
- their direct reverse Cargo dependencies (transitive dependents are
  deliberately not selected — decided 2026-08-13 to cap run cost; a
  regression observable only through an intermediate package surfaces when
  that intermediate is next touched or on a manual full run),
- curated package areas owning those packages.

The selected package matrix calls `_package-ci.yml`, which compile-checks
Windows, runs L1 on Linux, Windows, and macOS (plus WSL2), and runs each
package's lint/documentation guards. Per-package policy — L2/browser tier
ownership, native libraries, Cargo features, runner tools, and companion
suites — lives in each package's `[package.metadata.ci]`; environment
capabilities live in `.github/ci/environments.json`.
Changes to global build/test configuration conservatively select every package.

### Layer 3 — Affected coverage and specialized workflows

On pull requests, `ci.yml` passes the affected package closure to one `cargo llvm-cov` invocation
and uploads one LCOV artifact (`lcov-affected`). It does not perform a package-by-package pass and
then repeat the workspace.

Specialized runtime contracts are **reusable workflows called by `ci.yml`**, not independently
path-triggered ones, so a commit produces one CI run rather than a wall of overlapping ones. Each
is selected from affected scope and gated on preflight:

| Workflow | Selected when | Unique evidence |
|---|---|---|
| `biscuit-tui-windows-captured-stdout.yml` | `biscuit-tui` in scope | attached-console captured-stdout boundary |

Messenger and all three Rendezvous crates are owned by their ordinary
package-keyed L1 cells on Ubuntu, Windows, macOS, and WSL2. Messenger declares
all-feature coverage and the closed `messenger-desktop-stubs` runner tool. The
native workflow builds and verifies all six helpers once before L1 and exports
`MESSENGER_STUB_BIN_DIR`; the WSL2 archive workflow ships Linux helpers as a
sidecar to its toolchain-free guest. JUnit and producer-status artifacts retain
the package/environment/tier identity consumed by `ci-verdict`.
`sniff-performance.yml`
stays independent because its PR leg is artifact-only and its scheduled leg measures work counts,
not correctness. `build-integrations.yml` stays release-triggered.

### Layer 4 — Nightly and advisory

Each scheduled workflow owns its own name, schedule slot, artifacts, and summary so none can be
mistaken for required validation: fuzz 02:00, sniff-performance 04:00 UTC, maintenance audit
Mondays 07:00.

Coverage and workspace benchmarking left CI on 2026-08-12: coverage is a local tool (`just
coverage` per package), and `bench-nightly`'s Bencher.dev upload had been failing silently for
weeks — performance testing returns as the opt-in, package-owned design in
`features/2026-08-12-perf-opt-in/spec.md`.

The 90-minute budget is provisional: warm scheduled runs measured 14–18 minutes, but every
cold-cache run was truncated by the previous 30-minute ceiling, so the cold duration has never
actually been observed. Tighten the budget — or split the 16 bench targets across parallel jobs —
once a cold run has been recorded.

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

#### `maintenance-audit.yml` — Mondays 07:00 UTC and manual

Reports what has moved upstream for every value the repository pins on purpose — the required Rust
version, the kache pin, `cargo-nextest`, third-party GitHub Action versions, and the runner image —
and changes nothing. The job always succeeds; a finding is information. Pins advance only through a
reviewed change (see [Advancing a pinned value](#advancing-a-pinned-value)).

#### `build-integrations.yml` — on `release: published`

Cross-compiles the three Unfolded Circle integrations (`arcam-amp-integration`,
`eversolo-integration`, `sony-receiver-integration`) to `aarch64-unknown-linux-musl` using `cross`,
then uploads tarballs to the GitHub release via `gh release upload`. `fail-fast: false` so one
target's failure doesn't strand the others.

## Release Strategy

Releases are automated end-to-end by `release-plz.yml` in the public repository.

### Two-job flow

1. After the `ci` workflow **succeeds** on `main` (a `workflow_run` trigger, not a bare push — release
   automation follows the validation it depends on rather than racing it), **`release-pr`** runs
   `release-plz release-pr`. It opens (or updates) a single **draft**
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
`coverage`, `bench-nightly-darkmatter`, and `sniff-bench`. Cache keys are
intentionally per-workflow rather than global — this trades hit rate for protection against
a poisoned target directory taking down the entire pipeline.

Concurrency is configured per-workflow:

- Dependency-scoped CI: `cancel-in-progress: true` per ref, so force-pushing a fix cancels
    the prior run.

- `release-plz` and the fuzz/bench nightlies: **never cancel** — partial state from these is
    always more useful than nothing.

## Required Toolchain on Runners

`rust-toolchain.toml` pins one **exact** Rust version for the whole repository, and required CI
honors that file with `rustup show` rather than overriding it with a floating channel. Local and CI
therefore resolve the same compiler — which also stabilizes rustfmt and Clippy, curing the
`main`↔branch formatting drift documented in `CLAUDE.md`.

Two deliberate overrides exist, both outside required CI:

- **`rust-latest-stable.yml`** sets `RUSTUP_TOOLCHAIN=stable` to test the newest compiler in
  advance. Advisory; it cannot change required-CI behavior.
- **`fuzz-nightly.yml`** uses nightly because `cargo-fuzz` requires it.

Coverage adds `llvm-tools-preview` on top of the pinned toolchain.

### Advancing a pinned value

The maintenance audit reports drift; advancing a pin is a reviewed change:

1. Check the most recent `rust-latest-stable` run (for a toolchain bump) or the upstream release
   notes (for an action, kache, or nextest bump).
2. Update the single authority — `rust-toolchain.toml`, `.github/kache-version`, or the `uses:`
   pin — never a second copy.
3. Run `cargo fmt --all --check` (read-only; never write-mode), plus the affected areas' `just
   build`, `just test`, and `just lint`.
4. Review newly enabled compiler and Clippy diagnostics rather than silencing them.
5. Keep action-version upgrades in their own commit, separate from behavior changes.

Roll back by reverting that one authority value; nothing else encodes it.

Shared CLI tools used in CI:

- `python3`, `jq`, and `gh` — scope calculation and GitHub orchestration.
- Node.js, npm, and pnpm — frontend legs declared with the `node` capability.
- `cargo-nextest` — the canonical test runner for L1 tiers.
- `just` — orchestration entry point for every job.
- `cargo-llvm-cov` — coverage.
- `cargo-fuzz` — fuzz targets.
- `cross` — integration cross-compilation.
- `bencher` — nightly benchmark upload.
- `release-plz` — release planning and publication.

The root `just init` recipe has a CI/CD stage that ensures the applicable
local equivalents. Binaries encapsulated entirely inside a third-party action
remain owned by that action.

## Policy Summary

What a reviewer can rely on when approving a PR:

1. **Every affected gating package passed its configured environment matrix** against the exact
   pinned Rust version in `rust-toolchain.toml`. Native L1 runs on Linux,
   Windows, and macOS; `wsl2-ubuntu` is a distinct archive-based L1 cell.
2. **`just check-canonical` confirms the area structure is well-formed** — no `justfile`
   recipe drift snuck in.

3. **Downstream workspace consumers are included automatically** from Cargo's dependency graph;
   the two specialized runtime contracts are selected from that same scope by `ci.yml`.

4. **Coverage is reported but not gated.** Treat coverage as a delta to inspect, not a number to
   defend.

5. **Bench and fuzz drift is captured nightly,** not on the PR itself. A regressed fuzz target
   files an auto-issue rather than blocking your merge.

6. **Releases never happen from a PR branch.** They happen from `main` via release-plz's draft
   PR, which is itself reviewed before merge.

What CI explicitly does **not** guarantee:

- **All-target compile coverage on every environment.** The dedicated Windows
    check compiles examples and benches; L1 on Linux, macOS, and WSL2 compiles
    and runs test targets but does not promise every non-test target.

- **Performance regressions blocking merge.** Bench results are tracked in Bencher but not gated.
- **External-resource (L4 `test-real`) tests passing.** Those tiers are explicitly excluded from
    CI; they live on developer machines and the homelab.

## Adding a New Workflow

Before adding a new workflow, check:

1. **Does an existing canonical recipe cover this?** If yes, register the area in the root
   `justfile` and declare the package's CI policy in its `[package.metadata.ci]`
   instead of inventing a new workflow.

2. **Is this a canonical area contract or a specialized contract?** Canonical work belongs in
   `_package-ci.yml` and the package policy; specialized hardware, IPC, or console behavior may justify
   its own file. If it does, make it a **reusable** workflow (`workflow_call` +
   `workflow_dispatch`, no `push`/`pull_request` triggers, no own `concurrency` group) and add a
   scope-gated job to `ci.yml` that calls it. A self-triggering workflow reintroduces the wall of
   parallel runs per commit that the orchestrator exists to prevent.

3. **Should this gate merges or just report?** Mirror the existing pattern — coverage,
   bench, fuzz, sniff-performance, and the maintenance audit are non-gating; everything else is.
   A non-gating workflow needs its own name, schedule slot, artifact names, and summary.

4. **Pick a unique `shared-key`** for the cache so you don't share state with an unrelated job.
5. **Honor `rust-toolchain.toml`** with `rustup show`; never override it with a floating
   `dtolnay/rust-toolchain@stable`, and never rely on the runner's default. Nightly and
   latest-stable overrides are deliberate exceptions, documented where they occur.
6. **Install native prerequisites before building** with `just _ensure-native-libs <area>`, so a
   `-sys` crate cannot fail to compile for a missing system library.

## Pointers

- Workflow definitions: `.github/workflows/`
- Release config: `release-plz.toml`
- Test tier taxonomy: [`testing-in-rusty-biscuit.md`](./testing-in-rusty-biscuit.md) and
    `.claude/skills/rust-testing/SKILL.md`

- Pre-push hook: `.githooks/pre-push`, tested by `.githooks/tests/`
- Canonical recipe definitions: root `justfile` and `just/*.just`
