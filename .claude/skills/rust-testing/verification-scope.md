# Scope Verification Gates by Blast Radius

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

Before running any final build, test, or lint gate:

1. Record the changed packages and package areas.
2. Use GitNexus upstream impact on changed symbols to identify downstream
   consumers, then use `sniff repo packages`, `sniff repo package-areas`, and
   `sniff repo package-dependencies` to map that impact to executable scopes.
3. Run build, test, and lint for every affected package area. Use its local
   `just build`, `just test`, and `just lint` recipes, or an exact package
   selector when the repository provides a narrower supported recipe. To run
   exactly what CI's `lint` and `test` gates run for the branch's affected
   packages, use `just ci-local` at the root (`--dry-run` prints the scope).
   The pre-push hook runs `just ci-local --l2`: lint and L1 for source-changed
   packages, compile-check for their direct reverse dependencies, and every
   hostable L2 suite using a non-focusing backend (tmux, background WezTerm, or
   keep-focus Kitty). For a clean outgoing `HEAD`, it
   publishes exact-tree evidence so CI can omit the detected macOS, Linux,
   native Windows, or WSL2 environment.
   A non-comment change to a global path such as `.config/nextest.toml`
   selects the **whole** workspace in `ci-local` and CI alike (73 packages,
   ~45 minutes locally on 2026-09-08); budget for it before touching runner
   configuration.
4. Report the selected scope and commands with the gate results.

For durable native CI, declare package policy in the package's own
`[package.metadata.ci]`. The dependency-aware `.github/workflows/ci.yml`
caller calculates source-changed workspace packages plus check-only direct
reverse Cargo dependencies, reads that policy, and fans the resulting matrix into
`.github/workflows/_package-ci.yml` — one result-producing job per package. A
bootstrap `preflight` job gates that fan-out (`needs: [scope, preflight]`); it
is **prerequisites only** and runs no test suite of any language.

**CI's own suites are owned by two ordinary gating packages.** `repo-deps`
(`scripts/`, a root-workspace member since the cicd-redundancies fix) owns the
`ci-plan`/`ci-rollup` Nextest suites and the `scripts/ci/test_*.py` contracts;
`test-toolkit` (`tools/test-toolkit/`, whose CI exclusion record is retired)
owns `ci_workflow_contracts` and the `tools/test-audit` typecheck and
Vitest pair. Both fan out like any other package, so a test added to either runs
in that package's own cell — `just _test repo-deps` / `just _test test-toolkit`
from the repository root — and a change under `scripts/` or
`tools/test-toolkit/` now schedules real CI work.
For each package, `check`, `lint` (build + clippy), and `test` (L1) are
independent gates; only the expensive `l2`/`browser` tiers stage behind
`test`. Lint does not gate L1 — one clippy hint must not delete a package's
entire test evidence. Every configured L1 leg blocks: the `soft_os` policy is
retired, because `continue-on-error` removed a leg from the run's verdict
rather than merely making it non-blocking. The L2 leg provisions tmux,
verifies it (`tmux -V`), and sets `BISCUIT_TEST_REQUIRED_BACKENDS` to the
plan cell's `backends` (declared ∩ hostable), so an installed-but-never-exercised
backend fails the tier. The shared workflow denies warnings in the `lint` job only
(`_lint` passes `-D warnings` to clippy directly, so the same bar applies
locally). `check` is a compile gate and does not promote warnings — dead code
is not a build failure, and platform-conditional dead code is normal.
Sharding is removed: no job passes `--partition` (compilation is ~85% of a
shard and every shard pays it in full, so four shards cost ~3.2× the compute
to save ~2.4 minutes — see `fixes/2026-08-06-cicd/spec.md` § Sharding). L1
runs with `--no-fail-fast`, and CI selects the `ci` nextest profile
explicitly.

Coverage is a local tool, not a CI producer (decided 2026-08-12): run a
package's `just coverage` recipe for an LCOV report. CI generates none.

Do not use `cargo build --workspace`, `cargo check --workspace`, a bare root
`cargo build`/`cargo check`/`cargo test`, or an unscoped root `just` lifecycle
recipe as a generic final safety net. An exposed enum or public API change is a
reason to include its actual downstream consumers, not all workspace members.
A workspace-wide run is appropriate only when the user explicitly requests a
release/CI aggregation task or when a documented repository-wide invariant
cannot be verified from the dependency-derived scope; record that reason before
running it.

At the repository root, `just test` delegates to `_test_workspace`. It uses
Cargo metadata as the package source of truth and runs every selected package
in one local Nextest invocation with `--no-fail-fast`. Optional selectors may
be exact package names or package-area paths. Package-area `_test_all` recipes
use the same one-scheduler path locally, but retain per-package execution under
the CI profile/environment for JUnit staging and summaries. CI `features` and
local `local-features` remain separate metadata contracts.

Run `just check-test-interrupts` to verify that every package-area `test`
recipe also preserves Ctrl+C as exit `130`.
