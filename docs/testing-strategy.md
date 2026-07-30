---
title: Testing Strategy
status: living
created: 2026-05-24
---

# Rusty Biscuit Testing Strategy

This document is the human-facing reference for testing in the rusty-biscuit
monorepo. It covers the test-level taxonomy, the canonical `just` recipe set,
nextest filtersets, and how non-canonical or excluded areas relate to the wider
testing initiative.

For the short, agent-facing summary see `.claude/skills/rust-testing/SKILL.md`
(authored in Phase 6 of the testing-best-practices initiative).

## Test Levels

| Level | Identifier | What it covers | Default behavior |
| ----- | ---------- | -------------- | ---------------- |
| L1    | (default)  | Fast in-process unit and integration tests. No external resources. | Always runs. |
| L2    | `level2_`  | Real terminal / PTY / local harness tests. | Skips cleanly when harness is unavailable; hard-fails for the backends named in `BISCUIT_TEST_REQUIRED_BACKENDS`, or for every backend when `BISCUIT_TEST_LEVEL_REQUIRED=2`. |
| L3    | `level3_`  | OS keyboard or mouse injection (cliclick / WezTerm window focus). | Always skipped unless `RUN_LEVEL3=1`. |
| Browser | `browser_` | Headless browser tests via `biscuit-browser-harness`. | Skips cleanly when Chrome is absent; hard-fails when `BISCUIT_BROWSER_REQUIRED=1`. |
| Real  | `real_`    | Tests against real devices, networks, or provider APIs. | Always `--ignored` unless explicitly opted-in via the relevant env vars. |
| Slow  | `slow_`    | Otherwise-ordinary tests that exceed the sanity time budget. | Excluded from `sanity`, included in `test`. |

Nextest does not yet expose user-named filterset aliases as a stable feature, so
the canonical filter expressions live in the shared `_sanity`, `_test_l2`,
`_test_l3`, `_test_browser`, and `_test_real` recipes in `just/devops.just` and
are passed to `cargo nextest run -E '...'` directly. `.config/nextest.toml`
documents the expressions in its header comment and contains only the retry
and slow-timeout profiles — not the tier filtersets. The authoritative
expressions are:

```
level2  = test(/level2_/)
level3  = test(/level3_/)
browser = test(/browser_/)
real    = test(/real_/)
slow    = test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/) + test(/slow_/)
```

Runtime skip-vs-fail behavior is enforced via the `require_level!` macro and
helpers in the `tools/test-toolkit` crate.

## Canonical `just` Recipe Set

Every curated package area in the root `justfile` `areas` variable exposes the
same 12 recipes. Recipes that don't apply to a particular area are explicit
no-ops with a one-line `echo` explaining why; they intentionally do not error
so that area-based root orchestrators such as `just lint` keep iterating.
Root `just test` instead discovers every Cargo workspace package, continues
after package failures, and reports all failed packages at the end.

| Recipe         | Purpose |
| -------------- | ------- |
| `sanity`       | Fast confidence (≤15s). `cargo nextest run --lib --bins -E '!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/) + test(/slow_/))'`. |
| `test`         | Full L1 suite for this area. |
| `test-l2`      | Real-terminal tests via `test(/level2_/)`. |
| `test-l3`      | OS keyboard/mouse tests via `test(/level3_/)` (`RUN_LEVEL3=1`). |
| `test-browser` | Browser tests via `test(/browser_/)`. |
| `test-real`    | Real-resource tests via `test(/real_/)`. |
| `lint`         | `cargo clippy -- -D warnings`. |
| `bench`        | Criterion benchmarks. |
| `coverage`     | Per-package LCOV via `cargo llvm-cov`. |
| `doctest`      | `cargo test --doc`. |
| `fuzz`         | `cargo +nightly fuzz run` targets. |
| `all`          | `sanity → lint → doctest → test → test-l2 → test-browser`. |

The shared per-package boilerplate is implemented as the `_*` private recipes
in `just/devops.just`. Each package's own `justfile` thin-wraps these to add
its CLI/library crates and any package-specific notes.

The validator `just _check_canonical` (also in `just/devops.just`) asserts that
the current package area exposes all 12 recipes. CI runs this against every
curated area.

## Curated Areas and Exclusions

The canonical migration applies to the **curated `areas` list in the root
`justfile`** — not every Cargo workspace member. The current list is:

```
biscuit-hash biscuit-location biscuit-speaks biscuit-terminal
biscuit-tui schematic biscuit-file unchained-ai playa
tree-hugger darkmatter sniff model-citizen
claudine research queue homelab
```

### Areas intentionally excluded from the canonical set

| Workspace member | Why it is not in `areas` |
| ---------------- | ------------------------ |
| `biscuit-clipboard`, `biscuit-visualized`, `messenger`, `tabby`, `worktree`, `dmls` | Smaller utility crates or experimental areas not yet promoted into the curated list. |
| `tools/test-toolkit`, `biscuit-test-harness`, `biscuit-browser-harness` | Test-only infrastructure crates. Their tests run as part of the consumers (darkmatter, claudine, biscuit-terminal, etc.). |
| `schematic/schema` | Generated code; intentionally excluded from the workspace and rebuilt via `just generate` in the `schematic` area. |
| `so-you-say` | A *binary*, not a package area — it ships from `biscuit-speaks/cli`. Lifecycle is owned by the `biscuit-speaks` area. |

If a new area is added to the root `areas` list later, it **must** expose the
canonical 12 recipes before landing, validated by `just _check_canonical`.

### Multi-crate areas

A handful of areas wrap more than the typical `{name}` + `{name}-cli` pair.
Each shared `_*` recipe is invoked once per crate so coverage is uniform:

- `schematic` — `schematic-define`, `schematic-definitions`, `schematic-gen`.
- `unchained-ai` — `unchained-ai`, `unchained-ai-cli`, `unchained-ai-gen`, `model_id`.
- `homelab` — `homelab`, `homelab-cli`, `homelab-server`, plus the
  `*-integration` device crates (`arcam-amp`, `eversolo`, `sony-receiver`,
  `unfolded-integration-helper`). The Vue/Vite frontend under
  `homelab/server/frontend` is exercised via the existing `test-frontend`
  recipe and `pnpm` scripts.

## Recipe-by-recipe notes

### `sanity`
Auto-detects which of `--lib` / `--bins` the package exposes via `cargo
metadata`, so the same `_sanity` shared recipe works for lib-only crates,
bin-only crates, and the common lib+CLI combo without per-package wiring.
Doctests are excluded; they run via `doctest`.

### `test-l2`, `test-l3`, `test-browser`
These select tests via stable name prefixes (`level2_`, `level3_`,
`browser_`). The runtime `require_level!(Level::L2, harness_check, Backend::Tmux)`
macro from `test-toolkit` decides whether a selected test should skip cleanly or
panic based on `BISCUIT_TEST_REQUIRED_BACKENDS`, `BISCUIT_TEST_LEVEL_REQUIRED`,
and `BISCUIT_BROWSER_REQUIRED`. The third argument may instead be a plain string
label (`"PTY (/dev/ptmx)"`, `"WezTerm + cliclick"`) for composite or
non-backend requirements that no single backend identity describes.

`BISCUIT_TEST_REQUIRED_BACKENDS` also turns on execution recording: each gate
appends a `{backend, test, decision}` line to
`$BISCUIT_JUNIT_STAGE_DIR/backend-executions.jsonl`, and `test-l2` brackets the
whole tier with `backend-proof reset` before the run and `backend-proof verify`
after it. This closes the availability-is-not-execution gap — an installed
`tmux` plus zero tmux tests is not evidence, and `verify` fails the tier when a
required backend produced no executed test. The bracket is per tier, not per
package: `_test_l2_all` claims ownership for multi-package areas via
`BISCUIT_BACKEND_PROOF_OWNER` so a per-package `reset` cannot erase earlier
packages' evidence. Nothing runs and nothing is written when the variable is
unset.

`test-l2` additionally pre-spawns one shared terminal pane per backend
(WezTerm, kitty, tmux, Apple Terminal) via `biscuit-harness-broker`
before invoking nextest, exports the pane ids via `BISCUIT_SHARED_*`
env vars, and runs nextest with `-j 1`. Tests call
`<Backend>Harness::shared_or_spawn()` from their `SharedHarness` init
closures, which attaches to the pre-spawned pane when the env var is
set and spawns a fresh background pane otherwise. The recipe tears
the shared panes down in a trap after nextest exits, so a single
2–3 s spawn cost per backend is paid once per `just test-l2`
invocation instead of once per test. Backends whose tooling is
missing on the host (e.g. no `WEZTERM_UNIX_SOCKET`) are silently
skipped at spawn time and tests fall back to per-process spawning,
which itself skips cleanly via `require_level!`.

### `test-real`
Real-resource tests are typically `#[ignore]`d and gated on per-package env
vars (e.g. `ARCAM_REAL_HOST`, `SONY_REAL_HOST`). The recipe selects them via
the `test(/real_/)` filter expression but they remain skip-clean when the
resource is absent.

### `bench`
Pure data crates or crates without measurable hot paths may opt out via
`[package.metadata.benchmarks] required = false` in their `Cargo.toml`. Their
`bench` recipe becomes a documented no-op. This is enforced by reviewer
discretion only; no static checker.

### `fuzz`
Fuzz infrastructure is scoped to Phase 5 of the testing-best-practices
initiative (`biscuit-file/fuzz`, `darkmatter/fuzz`). Until those land, the
recipe is a no-op for every area. Fuzz is **never** part of `sanity`, `test`,
or any PR-blocking gate.

## Environment variables

| Variable | Purpose |
| -------- | ------- |
| `BISCUIT_TEST_LEVEL=1\|2\|3` | Runtime gate; tests above this level skip cleanly. |
| `BISCUIT_TEST_LEVEL_REQUIRED=2` | CI use; missing L2 harness panics instead of skipping. All-or-nothing — prefer `BISCUIT_TEST_REQUIRED_BACKENDS`. |
| `BISCUIT_TEST_REQUIRED_BACKENDS` | CI use; comma-separated `tmux,wezterm,kitty,apple-terminal`. The named backends panic when unavailable while the rest still skip. Also enables execution recording for `backend-proof verify`. |
| `BISCUIT_BROWSER_REQUIRED=1` | CI use; missing Chrome panics instead of skipping. |
| `RUN_LEVEL3=1` | Explicit opt-in for OS-keyboard-injection tests. |
| `BISCUIT_JUNIT_STAGE_DIR` | Staging root for JUnit reports and `backend-executions.jsonl`. Defaults to `target/nextest/ci-reports`. |

Per-package legacy variables such as `DARKMATTER_LEVEL2_REQUIRED` are
deprecated and removed as of Phase 6. Use the unified `BISCUIT_*` contract
exclusively.

## Platform Coverage (CI)

The tier taxonomy above decides *what* runs; this section decides *where* (which
OS). `.github/workflows/ci.yml` calculates changed workspace packages and their
reverse Cargo dependencies, maps them to the curated policy in
`.github/ci/areas.json`, and fans the resulting matrix into the reusable
`.github/workflows/_area-ci.yml`. Platform behavior is uniform without starting
jobs for unrelated areas.

A bootstrap `preflight` job runs first (3 OSes for global CI/tooling changes, a
scoped OS set for package-local changes) and gates the area fan-out via
`needs: [scope, preflight]`. Within each area, `check` (macOS compile), `lint`
(build + clippy), and `test` (L1 shards) are **independent** gates; only the
expensive `l2`/`browser` tiers stage behind `test`. Lint deliberately does not
gate L1 — one clippy hint used to delete every L1 leg's evidence for the whole
area, which is how Claudine's Windows tests never ran. Independent areas run in
parallel (`fail-fast: false`).

### Toolchain

`rust-toolchain.toml` pins the exact version (`channel = "1.97.1"`,
`components = ["clippy", "rustfmt"]`), not a floating `stable`, so local and CI
builds are provably identical and rustfmt/clippy stay stable. Required CI honors
this file — each toolchain step is `rustup show` (which materializes the pinned
toolchain); there are no `dtolnay/rust-toolchain@stable` overrides. The scheduled
(and manual) `.github/workflows/rust-latest-stable.yml` advisory workflow tests
the latest stable toolchain (`RUSTUP_TOOLCHAIN=stable`) and runs
`cargo fmt --check`. It is **non-required** — advisory only.

### Policy

| Concern | Linux (`ubuntu-latest`) | Windows (`windows-latest`) | macOS (`macos-latest`) |
|---------|-------------------------|-----------------------------|--------------------------|
| Compile (`cargo check --all-targets`) | via test job | via test job | dedicated `check` job |
| L1 (`just test`) | full, shardable | full | compile-check only |
| L2 (`test-l2`) | yes (tmux) | skips (harness absent) | opt-in |
| Browser | yes | skips | opt-in |
| L3 (`level3_`) | opt-in (`RUN_LEVEL3=1`) | opt-in | opt-in |

- **macOS is compile-checked on every PR, not full-tested** — GitHub macOS
  minutes bill ~10× Linux. The `check` job still catches macOS-specific compile
  breakage (`cfg(target_os = "macos")` paths) cheaply; full macOS L1 runs on the
  nightly schedule or an on-demand label.
- **Windows runs full L1** — it is the platform most prone to silent API/type
  drift (the `HRESULT`, `PATH`-casing, and `VARIANT_BOOL` classes of bug that
  compile-only checks would miss at runtime). Windows-only tests stay gated
  (`#[ignore]` / `level3_`).
- **L2/browser skip cleanly on non-Linux** via `require_level!`; no special
  casing. `BISCUIT_TEST_LEVEL_REQUIRED=2` is set only on the Linux leg (where the
  tmux harness is guaranteed) so a genuinely broken harness still hard-fails.
- **Heavy areas shard** their L1 run via nextest `--partition count:i/N` across
  parallel matrix jobs. Darkmatter keeps 4 shards (measured 6.7–8.2 min/shard);
  Claudine runs a 4-shard L1 (`["1/4","2/4","3/4","4/4"]`), sized from a measured
  cold run (~3964 tests, ~27 min unsharded → ~7 min/shard). L1 shards run with
  `--no-fail-fast` so one slow or failing test cannot suppress the rest of a
  shard's evidence, and per-shard JUnit is uploaded under collision-free names.
  Combined with the build cache this keeps wall-clock under the 30-min ceiling.
- **CI selects the `ci` nextest profile** explicitly (`NEXTEST_PROFILE: ci` in
  `_area-ci.yml`; nextest logs `nextest profile: ci`). In `.config/nextest.toml`
  `[profile.ci]` sets `retries = 0`, so a deterministic L1 failure runs exactly
  once; scoped `retries = 2` overrides remain only for the `test(/level2_/)` and
  `test(/browser_/)` tiers (documented resource contention). `[profile.default]`
  keeps `retries = 3` for local dev — only the CI profile went to 0.
- **Build cache**: CI caches Cargo artifacts with `Swatinem/rust-cache@v2` on
  every native leg and uses no rustc wrapper. The `kache` wrapper was removed
  from CI on 2026-07-30 after measuring 0-6% hit rates (0.4-2.3% weighted by
  compile cost); it remains a per-host developer opt-in via
  `just install-kache`, pinned by `.github/kache-version`.
- **Every configured L1 leg gates**: there is no `continue-on-error` on any area
  gate. The retired `soft-os` input did not merely make a leg non-blocking — it
  removed the leg from the run's verdict, so 14 permanently red Windows areas
  read as a normal run. A known failure is recorded in the results baseline
  instead, which keeps it counted and visible.
- **Optional tiers**: Darkmatter enables the reusable Linux L2 and browser jobs;
  Claudine enables Linux L2 and installs portable inert provider stubs for
  discovery-dependent tests.
- **Lint is the warning gate; `check` is not**: `RUSTFLAGS=-D warnings` is
  scoped to the `lint` job alone. It is deliberately **not** set for the test
  tiers, where it made a plain rustc warning fail the build so no test ran, nor
  for `check`, where it reported dead code as `error: could not compile`,
  attributed a dependency's warning to whichever area built it, and could not be
  reproduced by `just check`. The lint job's real authority is the recipe:
  `_lint` passes `-D warnings` to clippy directly, so the same bar applies
  locally. Area-specific documentation, generated-artifact, and
  typed-error guards wired into `just lint` remain blocking CI checks.
- **Coverage uses the same dependency scope on PRs**: one `cargo llvm-cov`
  invocation selects every affected workspace package. A nightly/manual
  workflow makes one workspace-wide pass; coverage is not repeated after the
  PR lands on `main`.

### Feature-gated surfaces

`cargo check --all-targets` resolves only a package's **default** features, so
code behind an off-by-default `cfg` compiles on no platform unless a step names
the feature. When a feature gates real code, add it to that area's
cross-platform compile check — otherwise the matrix stays green precisely
because it never builds the code in question.

The live case is `sniff`'s `remote` (which implies `network`), gating the
provider client and its Wiremock test, bench, and example targets:

- Sniff's entry in `.github/ci/areas.json` adds `sniff/remote` to the
  all-target compile check and runs the full L1 suite on macOS, Linux, and
  Windows. Its `just test` recipe executes the provider suites with `remote`
  enabled rather than merely compiling them.
- Downstream areas reach the same surface through their dependency edges rather
  than through a flag: `darkmatter/lib` declares
  `sniff = { features = ["remote"] }`, so the Darkmatter Linux and Windows legs
  already build the provider source. `claudine` reaches it transitively via
  `darkmatter`; the dependency-aware scope includes those consumers whenever
  the Sniff surface changes.

### Orchestrated and standalone workflows

The bespoke single-behavior workflows (`playa-windows`,
`biscuit-tui-windows-captured-stdout`, `rendezvous-tests`,
`messenger-desktop-tests`) keep their own files — they test runtime contracts the
shared area matrix cannot host — but they are **reusable workflows called by
`ci.yml`** and selected from affected scope, so one commit produces one CI run.

Standalone by design: `coverage` (nightly/manual, report-only), `bench-nightly`
and `fuzz-nightly` (nightly, advisory), `maintenance-audit` (weekly, advisory),
`sniff-performance` (its own measurement contract), and `build-integrations`
(on release). Each owns a distinct name, schedule slot, artifacts, and summary
so a scheduled failure is never read as an L1 regression.

## Why this matters

- Agents can always type `just sanity`, `just test`, `just test-l2` and get the
  same behavior regardless of which area they are in.
- CI validates the curated `areas` list against `.github/ci/areas.json`, then
  runs only the dependency-derived subset.
- Drift between packages is detectable: `just _check_canonical` either passes
  or names the missing recipes.

## Fuzz Playbook

Fuzz targets live in `<crate>/fuzz/` and use `cargo-fuzz` (nightly Rust only).
Each target directory contains:

- `Cargo.toml` — fuzz-suite manifest depending on the parent crate.
- `rust-toolchain.toml` — pins `channel = "nightly"`.
- `fuzz_targets/<name>.rs` — one binary per target.
- `corpus-seed/` — small, hand-curated seed inputs committed to the repo.
- `crashes/<target>/` — minimized crash inputs committed as regression fixtures.

### Running fuzz targets locally

```bash
cd biscuit-file/lib/fuzz
cargo +nightly fuzz run pdf_extract -- -runs=1000
cargo +nightly fuzz run toml_roundtrip -- -runs=1000

cd darkmatter/lib/fuzz
cargo +nightly fuzz run markdown_parser -- -runs=1000
```

### When to add a new fuzz target

A parser/decoder is a good fuzz candidate if **all** are true:

1. It accepts data from outside the process boundary.
2. A crash, hang, or OOM in it is a real defect.
3. It has a stable surface area.

Top candidates in priority order: `biscuit-file` parsers, `darkmatter` markdown,
`claudine` hook JSON, `tree-hugger` queries, `schematic` schema definitions.

### CI integration

Fuzz runs nightly via `.github/workflows/fuzz-nightly.yml`. It is **never**
part of `sanity`, `test`, or PR-blocking gates because it requires nightly
Rust and long wall-clock times.

## Decision Log

| Decision | Rationale |
|----------|-----------|
| Runtime `require_level!` macro instead of proc-macro attribute | Avoids compile-time overhead and a new proc-macro crate. Skip-vs-fail behavior is explicit in the test body. |
| `sanity` excludes doctests | Doctest compile cost would blow the ≤15 s per-package budget. |
| `just all` order: sanity → lint → doctest → test → test-l2 → test-browser | Fast-fail order: cheapest signals surface first. |
| test-l3, test-real, fuzz, bench excluded from `all` | They require explicit opt-in (devices, OS keyboard focus, nightly toolchain, quiet CPU). |
| Coverage is report-only | Gates create perverse incentives and false alarms on legitimate refactors. |
| Fuzz corpus stored in-repo (seed only) | Avoids Git LFS dependency. Only minimized crash inputs are committed back. |
| tmux as default L2 backend | Most portable: headless, runs on any CI runner without GUI. |
| `[package.metadata.benchmarks] required = false` convention | Grep-able opt-out for pure data crates. Enforced by reviewer discretion only. |
| One dependency-aware `ci.yml` caller + reusable `_area-ci.yml` | Uniform platform behavior without starting jobs for unrelated areas. |
| macOS compile-checked (not full-tested) on PRs | GitHub macOS runners bill ~10× Linux; the `check` job catches macOS compile drift cheaply while full L1 runs on Linux + Windows. |
| Windows runs full L1 | Windows is the highest-risk platform for silent API/type drift; compile-only would miss runtime-shaped bugs. |
| `Swatinem/rust-cache@v2` on every native leg | CI uses no rustc wrapper. `kache` was measured at 0-6% hit rate through the GitHub Actions cache backend and removed from CI on 2026-07-30; it stays a per-host developer opt-in with one version authority (`.github/kache-version`). |

See also:

- `tools/test-toolkit/src/lib.rs` — `Level`, `require_level!`, and env contract.
- `biscuit-test-harness/README.md` — L2 harness backends and `SharedHarness`.
- `biscuit-browser-harness/README.md` — browser harness API.
- `just/devops.just` — shared `_*` lifecycle recipes and `_check_canonical`.
- `.config/nextest.toml` — retry and slow-timeout profiles; documents the
  canonical filter expressions in its header comment.
