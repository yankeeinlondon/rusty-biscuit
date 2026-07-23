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
| L2    | `level2_`  | Real terminal / PTY / local harness tests. | Skips cleanly when harness is unavailable; hard-fails when `BISCUIT_TEST_LEVEL_REQUIRED=2`. |
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
`browser_`). The runtime `require_level!(Level::L2, harness_check)` macro from
`test-toolkit` decides whether a selected test should skip cleanly or panic
based on `BISCUIT_TEST_LEVEL_REQUIRED` and `BISCUIT_BROWSER_REQUIRED`.

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
| `BISCUIT_TEST_LEVEL_REQUIRED=2` | CI use; missing L2 harness panics instead of skipping. |
| `BISCUIT_BROWSER_REQUIRED=1` | CI use; missing Chrome panics instead of skipping. |
| `RUN_LEVEL3=1` | Explicit opt-in for OS-keyboard-injection tests. |

Per-package legacy variables such as `DARKMATTER_LEVEL2_REQUIRED` are
deprecated and removed as of Phase 6. Use the unified `BISCUIT_*` contract
exclusively.

## Platform Coverage (CI)

The tier taxonomy above decides *what* runs; this section decides *where* (which
OS). The repo mandate is that every package compiles and works on macOS,
Windows, and Linux, so CI is organized around a single reusable workflow
(`.github/workflows/_area-ci.yml`, invoked via `workflow_call`) that every
curated area calls through a thin per-area caller. Platform behavior is uniform
by construction instead of accreting per-area and per-incident.

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
  parallel matrix jobs (e.g. darkmatter). Combined with the build cache this
  keeps wall-clock under the 30-min ceiling.
- **Build cache**: the reusable workflow enables `kache` (GitHub Actions cache
  backend) on the Linux/macOS legs; Windows is unsupported by kache and uses
  `Swatinem/rust-cache` only.
- **Soft legs report but do not gate**: `_area-ci.yml`'s `soft-os` input
  (default `["windows-latest"]`) marks a test leg `continue-on-error`. This is
  how a platform is lit up before its latent cross-platform backlog is burned
  down. Read Windows *test* results accordingly — they are evidence, not a gate,
  until the leg is deliberately promoted to a required check.
- **Integration candidates use required native legs**: a caller that supplies
  `soft-os: '[]'` makes every configured L1 host blocking. Biscuit File,
  Darkmatter, and Claudine use this strict mode. Darkmatter also enables the
  reusable Linux L2 and browser jobs; Claudine enables Linux L2 and installs
  portable inert provider stubs for discovery-dependent tests.
- **Warnings and lint are gates**: the reusable workflow exports
  `RUSTFLAGS=-D warnings` for native compilation and tests, and its Linux lint
  job runs the package area's `just lint` recipe. Area-specific documentation,
  generated-artifact, and typed-error guards wired into that recipe therefore
  remain blocking CI checks.

### Feature-gated surfaces

`cargo check --all-targets` resolves only a package's **default** features, so
code behind an off-by-default `cfg` compiles on no platform unless a step names
the feature. When a feature gates real code, add it to that area's
cross-platform compile check — otherwise the matrix stays green precisely
because it never builds the code in question.

The live case is `sniff`'s `remote` (which implies `network`), gating the
provider client and its Wiremock test, bench, and example targets:

- `test.yml`'s `sniff-cross-platform` job runs a second
  `cargo check -p sniff --all-targets --features remote` alongside the
  default-feature check, and `cd sniff && just test` runs the `sniff` half with
  `--features remote`, so the provider suites are executed — not merely
  compiled — on macOS, Linux, and Windows.
- Downstream areas reach the same surface through their dependency edges rather
  than through a flag: `darkmatter/lib` declares
  `sniff = { features = ["remote"] }`, so the Darkmatter Linux and Windows legs
  already build the provider source. `claudine` reaches it transitively via
  `darkmatter` and carries a macOS/Windows compile-check leg in
  `claudine-tests.yml` (compile-only — its CLI tests rely on POSIX PATH stubs
  and its Windows Ctrl+C handling is a known gap).

### Retired / folded workflows

The bespoke single-behavior Windows workflows (`playa-windows`,
`claudine-windows-ctrl-c`, `biscuit-tui-windows-captured-stdout`) fold into their
areas' reusable-workflow calls as Windows matrix legs / gated tests. `coverage`
(report-only, Linux), `bench-nightly` and `fuzz-nightly` (nightly), and
`build-integrations` (on release) remain standalone by design.

## Why this matters

- Agents can always type `just sanity`, `just test`, `just test-l2` and get the
  same behavior regardless of which area they are in.
- CI workflows iterate over the curated `areas` list and rely on the canonical
  recipe names being present.
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
| Runtime `require_level!` macro instead of proc-macro attribute | Avoids compile-time overhead and a new proc-macro crate. Skip-vs-fail behaviour is explicit in the test body. |
| `sanity` excludes doctests | Doctest compile cost would blow the ≤15 s per-package budget. |
| `just all` order: sanity → lint → doctest → test → test-l2 → test-browser | Fast-fail order: cheapest signals surface first. |
| test-l3, test-real, fuzz, bench excluded from `all` | They require explicit opt-in (devices, OS keyboard focus, nightly toolchain, quiet CPU). |
| Coverage is report-only | Gates create perverse incentives and false alarms on legitimate refactors. |
| Fuzz corpus stored in-repo (seed only) | Avoids Git LFS dependency. Only minimized crash inputs are committed back. |
| tmux as default L2 backend | Most portable: headless, runs on any CI runner without GUI. |
| `[package.metadata.benchmarks] required = false` convention | Grep-able opt-out for pure data crates. Enforced by reviewer discretion only. |
| One reusable `_area-ci.yml` + thin per-area callers | Uniform platform behavior; a CI fix lands everywhere at once instead of per-area drift. |
| macOS compile-checked (not full-tested) on PRs | GitHub macOS runners bill ~10× Linux; the `check` job catches macOS compile drift cheaply while full L1 runs on Linux + Windows. |
| Windows runs full L1 | Windows is the highest-risk platform for silent API/type drift; compile-only would miss runtime-shaped bugs. |
| `kache` on Linux/macOS legs only | kache does not support Windows (compilation fails there); Windows stays on `Swatinem/rust-cache`. |

See also:

- `tools/test-toolkit/src/lib.rs` — `Level`, `require_level!`, and env contract.
- `biscuit-test-harness/README.md` — L2 harness backends and `SharedHarness`.
- `biscuit-browser-harness/README.md` — browser harness API.
- `just/devops.just` — shared `_*` lifecycle recipes and `_check_canonical`.
- `.config/nextest.toml` — retry and slow-timeout profiles; documents the
  canonical filter expressions in its header comment.
