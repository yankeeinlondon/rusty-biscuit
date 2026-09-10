---
fix: 2026-09-07-faster-darkmatter-tests
phase: 2
created: 2026-09-08
revision: 973d1d9184601ab3409654955ffacc249f4870a5
host: arm64-darwin (Darwin 27.0.0), Apple M4 Max, 16 cores
toolchain: rustc 1.97.1 (8bab26f4f 2026-07-14)
nextest: cargo-nextest 0.9.136
tool: "@rusty-biscuit/test-audit@0.1.0"
---

# Test inventory — darkmatter, darkmatter-cli, dmls, zed-dmls-cli

Phase 2 of [`plan.md`](plan.md), satisfying RB1 and opening AC1. Document-only:
no Rust source or test file in the four packages was changed.

Every number here is mechanical. The runner population comes from the eleven
`cargo nextest list --message-format json` captures under
[`enumeration/`](enumeration/); the source population comes from the
tree-sitter attribute scan in [`sources.json`](sources.json) /
[`sources.md`](sources.md); the cost figures come from Phase 1's verbatim
console logs under [`baseline/local/`](baseline/local/), joined to the family
declarations in [`families.json`](families.json) by `test-audit attribute`.
Nothing was estimated.

> **Local numbers are attribution only.** They set no target. Budgets stay
> pending until Phase 3 ratifies them against CI evidence; the table that will
> hold them sits in [Baseline](#baseline), beside the baseline pointers, so
> budget review and evidence review stay one act (spec RB5).

## How to reproduce the reconciliation

```sh
cd tools/test-audit
CFG=../../darkmatter/fixes/2026-09-07-faster-darkmatter-tests/audit.config.json
just run reconcile --config $CFG          # families.json × inventory.md × captures × source scan
just run attribute ../../darkmatter/fixes/2026-09-07-faster-darkmatter-tests/baseline/local/test-l1-local.log --config $CFG --markdown
```

The gate fails on an identity assigned to zero families, an identity assigned
to more than one, a family that matches nothing without an `expectEmpty`
reason, an inventory row whose count disagrees with the captures, a row with
no disposition, a source test the runner never lists that no exclusion
explains, a stale exclusion, and a capture present on disk but undeclared. It
exits 1 on any of those and 2 on a usage error. The Phase 2 run is recorded in
[`log.md`](log.md#phase-2--complete-inventory-2026-09-08).

## Population

| Package | Runner identities | Source attributes | Δ | Explained by |
|---|---:|---:|---:|---|
| `darkmatter` | 6,446 | 6,461 | 15 | 19 `cfg(windows)` source-only − 4 runner-only (see below) |
| `darkmatter-cli` | 734 | 737 | 3 | 2 `cfg(windows)` + 1 `cfg(not(target_os = "macos"))` |
| `dmls` | 646 | 648 | 2 | 2 `cfg(windows)` |
| `zed-dmls-cli` | 27 | 28 | 1 | 1 `cfg(windows)` |
| **total** | **7,853** | **7,874** | **21** | 25 exclusions − 4 runner-only |

7,853 identities across 120 build targets, under the union of every feature
selection a canonical recipe or CI leg uses. Every source-only identity is a
declared exclusion in `families.json` with its gate and its real execution
route; see [cfg/feature exclusions](#cfgfeature-exclusions).

**Corrections to the plan's grounding facts** (verified here; the captures and
scan supersede the estimates):

- The plan's **≈7,879 attributes** is 7,874 by the current scan (7,862 in the
  Phase 1 scan: the tool now resolves 12 `proptest!` bodies as `fromMacro`
  tests that the earlier run counted as runner-only). The per-package split
  the plan quotes (lib 6466 / cli 737 / dmls 648 / zed 28) is 6,461 / 737 /
  648 / 28.
- **`#[ignore]` sites: 5, not 11.** The plan's 11 was a grep over the string
  `#[ignore` and counted six doc-comment mentions. The five real attributes are
  enumerated in [Ignored and slow tests](#ignored-and-slow-tests). Three of
  the doc-comment mentions are **stale**: the `schema_plus_phase1` module docs
  at `lib/src/markdown/schemas/simplified/grammar.rs:2731`, `convert.rs:2308`
  and `mod.rs:949` still say their tests are `#[ignore]`-gated "until the phase
  lands", but no test in those modules carries the attribute — the phases
  landed and the attributes were removed while the prose stayed. Code is
  correct, comment is wrong (`CLAUDE.md` § Code Comment Quality); recorded for
  a comment-only cleanup commit, not edited in this document-only phase.
- **Darkmatter does own nextest overrides.** The plan's "none of them scopes
  darkmatter" and the Phase 1 log's "darkmatter owns zero nextest overrides"
  were both derived from grepping `.config/nextest.toml` for the string
  `darkmatter`. Four override *targets* (seven blocks) are darkmatter
  identities named by full module path or leaf; one of them binds nothing. See
  [Runner override census](#runner-override-census). Assumption 6 still holds
  as written — `git diff main -- .config/nextest.toml` must stay empty — but it
  is now a freeze on *existing* darkmatter overrides, not a statement that
  there are none.
- The **50 tests the local `just test` skips** are exactly: 44 in-process unit
  tests whose names start with `browser_`, the 5 `#[ignore]` tests, and the 1
  `slow_` test. The `BISCUIT_L1_INCLUDE_SLOW=1` run skips 49 (the `slow_` test
  joins).

### Enumeration substrate

Each capture records its command, revision, dirty entries, toolchain, nextest
version and platform in [`enumeration/captures.json`](enumeration/captures.json);
stderr sits beside each JSON as `<label>.err`. The reconciler rejects a JSON
file in that directory that the manifest does not declare (which is why the
Phase 1 source scan moved from `enumeration/sources.json` to the fix root).

| Capture | Package(s) | Features | Identities | Route(s) |
|---|---|---|---:|---|
| `l1-local-all` | all four | — | 7,711 | `just test` (one scheduler; local L1 cohort) |
| `lib-bare` | `darkmatter` | — | 6,380 | `just sanity`, `just doctest` |
| `lib-terminal` | `darkmatter` | `terminal-tests` | 6,400 | `just test-l2` |
| `lib-browser` | `darkmatter` | `browser-tests` | 6,426 | `just test-browser` |
| `lib-terminal-browser` | `darkmatter` | both | 6,446 | `just test-l3`; **CI L1 feature contract** |
| `lib-effects` | `darkmatter` | `effects-instrumentation` | 6,380 | dedicated capture; Phase 7 also enables the counters in canonical L1 (adds no identity) |
| `cli-bare` | `darkmatter-cli` | — | 661 | `just sanity`, `just test-l3` |
| `cli-terminal` | `darkmatter-cli` | `terminal-tests` | 734 | `just test-l2`; **CI L1 feature contract** |
| `dmls-bare` | `dmls` | — | 643 | `just sanity` |
| `dmls-terminal` | `dmls` | `terminal-tests` | 646 | `just test-l2`; **CI L1 feature contract** |
| `zed-cli-bare` | `zed-dmls-cli` | — | 27 | every route |

Feature deltas: `terminal-tests` adds 20 identities to the lib (19 in
`level2_render_tree_terminal` + 1 `level3_image_painting`), 73 to the CLI (11
L2 binaries) and 3 to `dmls`; `browser-tests` adds 46 to the lib (43
`browser_render` + 3 `level3_popover`); `effects-instrumentation` adds none.
**CI builds each package with its declared `[package.metadata.ci.tests]`
features for every tier including L1**, so the CI L1 population is the
`lib-terminal-browser` / `cli-terminal` / `dmls-terminal` listing minus the
tier-prefixed names — which is how six unprefixed tests inside feature-gated
binaries come to run on CI and nowhere locally (see
[Tests with no local route](#tests-with-no-local-route)).

### Build targets that list no test

| Target | Why |
|---|---|
| `darkmatter-cli::bin/md` | binary target, no `#[test]` — normal |
| `darkmatter::declined_path_transclusion` | whole file is `#![cfg(windows)]`; its three identities exist on `windows-latest` only (family `lib-compose-e2e-windows-only`, `expectEmpty`) |

### Source ↔ runner reconciliation

**Runner-only (4).** `markdown::schemas::coerce::tests::{string_to_number_integer_and_decimal, decimal_string_against_integer_field_still_produces_float, number_and_boolean_to_string, large_integral_strings_match_serde_json_number_model}`
are listed by the runner but missed by the source scan. Cause: in
`lib/src/markdown/schemas/coerce.rs:933-937` a `//` line comment sits between
`#[test]` and `#[allow(clippy::approx_constant)]`, and the scanner does not
carry attributes across an interleaved comment. Coverage is unaffected (the
runner is the authority); this is a `tools/test-audit` diagnostic gap and is
noted in `test-audit-tooling.md`. Not a violation.

**Macro-expanded (12).** Twelve `proptest!` bodies (`escape.rs` ×2,
`benchmark_fixtures.rs` ×1, `schema_quoting_safety.rs` ×7,
`schemas_grammar_proptest.rs` ×2) are resolved by grammar as `fromMacro`
source tests and matched to the runner by leaf name.

**Source diagnostics (54).** All `parse-error` on the `raw` / `&raw`
identifier (tree-sitter-rust 0.24 gap, documented) plus two in
`dmls/src/bench.rs:248-251`; every affected item still parses and no test is
lost (per-package source counts reconcile).

#### cfg/feature exclusions

Every source test the runner never lists on this host, its gate, and where it
actually executes. These are the 25 `exclusions` in `families.json`; the gate
fails if one goes stale.

| Package | Gate | Tests | Execution route |
|---|---|---:|---|
| `darkmatter` | `#[cfg(windows)]` | 19 | `windows-latest` L1 (`_package-ci.yml` test job). 14 in `compose/link_normalization.rs`, 1 each in `compose/link_resolve.rs`, `compose/expression/functions/mod.rs`, `compose/tests/schema.rs`; 3 in `lib/tests/declined_path_transclusion.rs`. CI baseline: windows lib L1 lists 6,319 vs 6,333 on macOS — the 19 are present there and the 33 `cfg(unix|macos)` tests below are absent. |
| `darkmatter-cli` | `#[cfg(windows)]` | 2 | `windows-latest` L1 (`cli/src/args/completion.rs:273,294`); CI baseline 667 vs 665 confirms +2 (and +4 harness-integrity, −1 macOS-only, +1 not-macOS). |
| `darkmatter-cli` | `#[cfg(not(target_os = "macos"))]` | 1 | `ubuntu-latest`, `windows-latest`, `wsl2-ubuntu` L1. Its twin `compose_redirected_does_not_spawn_appearance_defaults` is `#[cfg(target_os = "macos")]` and runs here and on `macos-latest`. |
| `dmls` | `#[cfg(windows)]` | 2 | `windows-latest` L1 (`overlay/doc_links.rs:257`, `workspace/mod.rs:122`); CI baseline 645 vs 643. |
| `zed-dmls-cli` | `#[cfg(windows)]` | 1 | `windows-latest` L1 (`lib.rs:986`); CI baseline 26 vs 27 (+1 windows, −2 unix). |

The mirror image — tests this host lists that `windows-latest` cannot — is
also enumerable from the scan: 32 `#[cfg(unix)]` lib tests (6 `editor`, 4
`shell_expansion::alias`, 12 `shell_expansion::executor`, and 10 singletons
across `compose`, `predict_conflicts`, `reference_integration`), 5
`#[cfg(target_os = "macos")]` lib tests (1 `repository_scope` unit test + the 4
L3 tests), 1 `#[cfg(any(unix, windows))]`, and 2 `#[cfg(unix)]` in
`zed-dmls-cli`. They are ordinary L1 tests on the legs that compile them
(`rust-testing` § OS-specific tests are ordinary tests); none is mis-tiered.

## Runner override census

`.config/nextest.toml` at this revision, at the merge-base `a9e88c069`, and on
`main` carries **seven override blocks whose filter names a darkmatter
identity**. The branch-side diff is empty (Phase 1) and must stay empty
(assumption 6). The plan's grounding fact that none scopes darkmatter was
wrong: the filters use `test(=…)` by module path or leaf, not `package(…)`.

| Filter | Profiles | Setting | Binds to | Local measurement | Note |
|---|---|---|---|---:|---|
| `test(=markdown::compose::preflight::acceptance_tests::execution_subset_of_approval_across_randomized_conditions)` | default, ci | `slow-timeout = 30s × 3` | `lib-override-preflight-proptest` (1) | 6.93 s (L1), 4.55 s (sanity), 4.31 s (include-slow) | proptest acceptance test; comment cites "~7 s isolated" — matches |
| `test(=every_catalog_variable_survives_ambient_options)` | default, ci | `30s × 3` | `lib-override-ambient-capture` (1 of 2) | 1.38 s | comment cites ~1.9 s/capture on native Windows and 48 s under WSL load; the fixture-repository remediation already landed |
| `test(=markdown::compose::tests::rendering::slow_compose_cleanup_preserves_quoted_marker_looking_indented_code)` | default only | `30s × 3` | `lib-override-slow-cleanup` (1) | 7.72 s (include-slow run) | also the area's only `slow_` test — belt and braces |
| `test(=level2_render_tree_style_in_wezterm)` | default, ci | `30s × 3` | **nothing** | — | no test of that name exists in any capture or in `darkmatter/` source; `test(=…)` is exact-match and nextest does not warn on an unbound filter. Dead since the L2 suite was reorganized into `level2_render_tree_terminal/`. |

The remaining L1 timing floor in the area is the local/CI `slow_` split
(`_tier_filter` drops the `slow_` exclusion under `BISCUIT_L1_INCLUDE_SLOW=1`,
which `darkmatter/justfile` sets under `NEXTEST_PROFILE=ci` or
`BISCUIT_CI_ENVIRONMENT`). No `test-group` binds a darkmatter package; the
default `slow-timeout = 5s × 6` and `leak-timeout` apply everywhere else.

**Disposition of the census.** Assumption 6 freezes the file for this fix, so
none of the four is repaired or removed here; the dead `level2_render_tree_style_in_wezterm`
entry and any re-justification of the three live ones are a **follow-up**
(filed in Phase 11 with this table as evidence). AC7 remains satisfied because
no override is *added* as a performance fix.

## Canonical recipe reconciliation

All twelve canonical recipes exist in `darkmatter/justfile` and every
executing one selects the four packages by name.

| Recipe | Delegates to | Selection | Status |
|---|---|---|---|
| `sanity` | `_sanity_all` | `darkmatter darkmatter-cli dmls zed-dmls-cli`, `--lib --bins`, `!slow` filter | exists; **53.5 s wall locally against the ≤15 s contract** (lib `--lib` alone 44 s) |
| `test` | `_test_local_all` (local) / `_test_all` + `BISCUIT_L1_INCLUDE_SLOW=1` (CI) | four packages, L1 filter | exists; two cohorts by construction |
| `test-l2` | `_test_l2_all` | lib, cli, dmls each `--features terminal-tests` | exists; `zed-dmls-cli` has no L2 tier (correct) |
| `test-l3` | `_test_l3` ×2 | lib `--features terminal-tests,browser-tests`; cli bare | exists; refuses unattended (exit 1 without `BISCUIT_L3_TAKE_FOCUS=1`) |
| `test-browser` | `_test_browser` | lib `--features browser-tests` | exists; `-j 1` |
| `test-real` | — | — | **declared no-op** ("not applicable for darkmatter"); `audit.config.json` route `test-real` is `kind: absent` |
| `lint` | `_lint` ×4 + `check-zed` | four packages + WASM extension | exists; `check-zed` hard-fails without the `wasm32-wasip2` target (installed on this host) |
| `bench` | `cargo bench -p darkmatter --benches` | lib only | exists; not a test route |
| `coverage` | `_coverage` ×2 | lib, cli | exists; dmls and zed-dmls-cli are not covered by the recipe (recorded, not a defect: coverage is a local tool) |
| `doctest` | `_doctest` ×2 | lib, cli | exists; dmls and zed-dmls-cli doctests are not run by any recipe (their source carries none that the runner would list — recorded) |
| `fuzz` | `_fuzz lib markdown_parser` | `lib/fuzz` | exists; nightly only |
| `all` | sanity → lint → doctest → test → test-l2 → test-browser | — | exists; L3 and `test-real` are deliberately outside `all` |

`just check-tier-coverage` (root) exists and would catch a stub `test-<tier>`
recipe stranding prefixed tests; it does **not** catch the inverse case found
here (unprefixed tests inside a feature-gated binary), which is why that case
is enumerated by hand below.

## Ignored and slow tests

Six identities are gated by something other than a tier prefix or `cfg`. Each
is its own family or its own row so the gate can count it.

| Identity | Gate | Why | Execution route | Family |
|---|---|---|---|---|
| `darkmatter markdown::cleanup::perf_profile::f25_cleanup_profile_raw_samples` | `#[ignore = "measurement harness; run explicitly with DM_PERF_RAW_DIR set"]` | writes Criterion-shaped raw samples for the performance follow-up | manual: `DM_PERF_RAW_DIR=<dir> cargo nextest run -p darkmatter --run-ignored ignored-only -E 'test(/raw_samples/)'`; the harness returns `None` without the variable, so `--run-ignored` alone is a no-op | `lib-unit-ignored-perf-harness` |
| `darkmatter markdown::render_tree::build_context::finding_35_7::f35_7_link_policy_raw_samples` | `#[ignore = "measurement harness; opt in with DM_PERF_RAW_DIR"]` | same | same | same |
| `darkmatter markdown::render_tree::code_renderer::tests::f23_code_surface_raw_samples` | `#[ignore = …]` + `#[serial]` | same; serial because it reads the shared code-surface cache | same | same |
| `darkmatter layout::page::tests::finding_35_6::f35_6_rhythm_raw_samples` | `#[ignore = "measurement harness; opt in with DM_PERF_RAW_DIR"]` | same | same | same |
| `darkmatter markdown::compose::shell_expansion::alias::tests::resolve_alias_ll` | `#[ignore = "requires an ll alias in the user's login-shell configuration"]` | resolves `ll` through the developer's real login shell; asserts only that the executable is non-empty *if* the alias exists, otherwise prints and passes | manual `--run-ignored`; depends on the host shell configuration, so it proves nothing portable | `lib-unit-ignored-host-shell` |
| `darkmatter markdown::compose::tests::rendering::slow_compose_cleanup_preserves_quoted_marker_looking_indented_code` | `slow_` prefix + default-profile override | merge-regression test performing several complete compose passes; 7.7 s here | excluded from local `just test` and `just sanity`; included under `BISCUIT_L1_INCLUDE_SLOW=1` (CI L1 on all four legs) — the one-identity local/CI cohort delta Phase 1 confirmed | `lib-override-slow-cleanup` |

Doctests: `just doctest` reports **180 passed, 10 ignored** for the lib
(``` ```ignore ``` / `no_run`-style doc blocks; not runner identities) and 0
for the CLI.

## Baseline tests with no local route

Six identities are compiled only when a test feature is enabled but carry no
tier prefix, so the tier recipe that enables the feature filters them out and
the L1 recipe never compiles them. They run **only on CI L1**, where the
package is built with its declared features. A clean local run is not evidence
for them.

| Identity | Binary (`required-features`) | Locally | CI |
|---|---|---|---|
| `darkmatter-cli::level2_harness_integrity::{md_shim_resolves_to_cargo_built_binary, assert_shim_resolves_to_built_accepts_valid_link, assert_shim_resolves_to_built_rejects_foreign_link, md_shim_path_is_absolute_temp_dir_link}` | `level2_harness_integrity` (`terminal-tests`) | never (`test-l2` selects 69 of the 73 terminal identities; these are the 4 skipped) | L1 on all four legs (cli L1 = 665 = 661 + 4) |
| `darkmatter::level2_render_tree_terminal::images::pixel_classification_distinguishes_magenta_from_black` | `level2_render_tree_terminal` (`terminal-tests`) | never (`test-l2` ran 18 of 19) | L1 on all four legs |
| `darkmatter::browser_render::sanitized_real_mermaid_retains_diagram_geometry` | `browser_render` (`browser-tests`) | never (`test-browser` ran 42 of 43) | L1 on all four legs — and it **returns early when no Mermaid toolchain produces an SVG**, so a runner without that toolchain records a pass that verified nothing |

Phase 7 repaired both unprefixed routes. The pixel classifier now lives in the
always-built `image_pixel_classification` integration binary and runs in L1.
The Mermaid identity is now prefixed `browser_`, is selected by
`just test-browser`, and uses `require_level!` to record an explicit Level-2
skip if `mmdc` is absent; if `mmdc` is present but produces no SVG, the test
fails.

The reverse mis-tiering also existed: **44 in-process tests were named
`browser_*`** (28 in `layout::page::tests`, 4 in `markdown::code_block::tests`,
6 in `markdown::render_tree::code_renderer::tests`, 4 in
`markdown::render_tree::entrypoints::tests`, 2 in
`lib/tests/disclosure_render_targets.rs`). Phase 7 renamed their leaf prefixes
to `render_browser_*`, preserving their assertions while making all 44 part of
the cross-platform L1 cohort. Family `lib-browser-prefixed-inprocess` retains
its baseline identifier so the before/after audit remains traceable.

## Non-nextest entry points

| Entry point | Route | Executes tests? | Notes |
|---|---|---|---|
| lib doctests | `just doctest` → `_doctest darkmatter` | yes: 180 passed / 10 ignored, 1.58 s (merged compile 0.32 s) | not in any nextest listing; not in `sanity` |
| cli doctests | `just doctest` → `_doctest darkmatter-cli` | 0 doctests | recorded as empty, not as coverage |
| `darkmatter` benches (16) | `just bench` (`cargo bench -p darkmatter --benches`), `bench-schema`, `bench-compose`, `bench-render`, `bench-baseline`, `bench-compare` | no (Criterion, `harness = false`) | `schema_validation`, `effective_schema_ownership`, `render_tree`, `compose_pipeline`, `render_pipeline`, `prose_highlighter`, `render_pipeline_steps`, `compose_schema_transclusion`, `render_code_heavy`, `reference_graph`, `phase6_interpolation`, `phase8_render`, `phase9_remote`, `phase10_residuals`, `phase11_evidence`, `clean_hot_paths`. Compiled by `cargo nextest list`? No — bench targets are not test targets; the scan covers `lib/benches` as a source root and finds no `#[test]` there. |
| `darkmatter-fuzz::markdown_parser` | `just fuzz` → `_fuzz lib markdown_parser`; `.github/workflows/fuzz-nightly.yml` job `darkmatter` (02:00 UTC) | no (libFuzzer) | own workspace under `lib/fuzz` (`cargo-fuzz = true`, nightly toolchain pin, `corpus-seed/`, `crashes/`); artifacts uploaded as `fuzz-crashes-darkmatter`; opens an issue on a new crash |
| `dmls` index bench | `just bench-dmls [tier]` → `cargo build -p dmls --release`, `dmls --gen-corpus <tier> ../target/dmls-bench/<tier>`, `dmls --bench-index` | no | seeded synthetic corpus, byte-identical run to run, reused until `rm -rf`; tiers `tiny-100 … large-20k` |
| Zed WASM extension compile | `just check-zed` (`cargo check --locked --manifest-path dmls/zed-dmls/Cargo.toml --target wasm32-wasip2`) | no (compile gate) | workspace-excluded crate with **zero `#[test]`**; also run by `just lint` and `just check`; **hard-fails when the target is not installed** |
| Zed extension package | `just zed-verify` → `check-zed` + `zed-package` + manifest/archive assertions (`jq`, `tar`, `grep`) | no (script assertions) | **CI runs it** on the `dmls` lint job when `runner-tools` contains `zed-extension` (`_package-ci.yml:519`); the spec's presumption that the extension sits outside recipe selection is corrected: it is outside `just test`, not outside CI. Local run needs a provisioned `zed-extension` binary. |
| Zed extension contract (in-tree) | `just test` → `dmls::zed_extension_contract` (1) and `dmls::packaging_contract` (1) | yes (family `dmls-l1-packaging-contracts`) | passive file reads pinning `extension.toml` and the `just dist` asset names |
| `dmls/vscode-dmls` | `just install-vscode-package` (`npm install`, `vsce package`, `code --install-extension`) | **no automated tests exist** (`package.json` has no `scripts`, no test runner, one `extension.js`) | **documented absence**: a manual packaging/install check. Absence is not coverage. |
| perf harness | `DM_PERF_RAW_DIR=… --run-ignored ignored-only` | yes, 4 ignored identities | see [Ignored and slow tests](#ignored-and-slow-tests) |
| effects counters | `--features effects-instrumentation` | adds no identity | process-wide `engine_build_count` / `network_attempt_count`; a Phase 7 proof instrument, not a route |

## Shared fixture machinery

First-class rows for everything more than one test binary depends on. None of
these is a test; each is inventoried for what it controls and what it leaves
to the ambient process.

| Helper | Consumers | Controls | Leaves ambient | Disposition |
|---|---|---|---|---|
| `cli/tests/common/mod.rs::md_cmd()` | no remaining consumers; helper deleted in Phase 6 | formerly a raw `assert_cmd::Command::cargo_bin("md")` | formerly inherited CWD, home, PATH, Git, rendering, and application inputs | satisfactory after Phase 6: every L1 call site uses `CliProcessFixture`; the guard reports 0 raw sites across 41 governed files |
| `common/mod.rs::md_file()` | content-only inputs in layout/rendering and other tests | a `NamedTempFile` in the system temp dir | the process launch remains fixture-isolated; tests whose source context or repository topology matters now use fixture-owned paths | satisfactory; retained where the file's location is not the behavior under test |
| `common/mod.rs::mock_http_server()` / `MockHttpServer` | `compose_remote_caching.rs` only (10 servers, 12 tests after Phase 8) | ephemeral `127.0.0.1:0` bind; request count plus captured method/path/headers | nothing: nonblocking accept checks shutdown every 10 ms, in-flight reads time out at 1 s, and `Drop`/`shutdown()` joins the worker within the documented 2 s bound | satisfactory after Phase 8; a no-request regression proves early teardown and the real CLI cases assert local request content |
| `common/mod.rs::baseline::{dir, load_json, normalize, paths_to_redact}` | `graph.rs`, `validate_refs.rs` | loads byte-for-byte JSON baselines from `darkmatter/features/[_completed/]2026-06-17-cli-atheist/baseline/json/` via `CARGO_MANIFEST_DIR`; redacts temp paths and FNV ids | reads the checkout (a shipped-artifact corpus, deliberately) | satisfactory (Phase 3 decision 4 measures it) |
| `common/mod.rs::layout::{parse_cli, resolved_page, fill_for, …, style_prop_fixture}` | `layout_alignment.rs` (10), `layout_fill.rs`, `layout_style_frontmatter.rs` | in-process `Cli::try_parse_from` + `DarkmatterPage` policy resolution at `Terminal::new_optimistic(120)`; `style_prop_fixture()` points at `example-docs/rendering/style-prop.md` | nothing (no process) | satisfactory |
| `cli/tests/common/level2.rs` (638 lines, `cfg(feature = "terminal-tests")`) | the 11 CLI L2 binaries | `md_bin()` via `bin_exe!("md")` (never the host `md`), shared WezTerm/tmux harnesses with `atexit` cleanup, completion-sentinel deadlines, then `capture_settled()` until the prompt and two byte-identical frames prove final state | nothing beyond the declared real-terminal backends | satisfactory after Phase 8; all three copied 250 ms post-command sleeps were removed |
| `lib/tests/level2_render_tree_terminal/support/mod.rs` (1,188 lines) | the 6 modules of `level2_render_tree_terminal` | WezTerm decision + shared pane, sentinel poll (50 ms to deadline), render-to-tempfile helpers, PNG probe + pixel classification, SGR/OSC parsers | nothing beyond the WezTerm socket requirement | satisfactory |
| `lib/tests/layout_matrix_support/mod.rs` (225 lines) | `layout_matrix.rs` and the `layout_matrix` example | scenario matrix for `CodeBlock::yaml` through the deprecated `TerminalCodeRenderer` adapter | nothing | satisfactory |
| `lib/tests/error_snapshots/helpers.rs` (54 lines) | the 16 modules of `error_snapshots` (110 tests) | `SourceContext` builders with a fixed `/tmp/test/<display>` *display* path (never touched on disk), ANSI strip, render helpers | nothing | satisfactory |
| `dmls` `ClientFixture` (three private copies: `lsp_session.rs`, `suggest_constraint_phase1.rs`, `no_side_effects.rs`) | 108 tests | `lsp_server::Connection::memory()` pair; notifications/requests buffered; per-test `tempfile::tempdir()` workspaces | nothing (in-memory protocol); `lsp_session.rs` now owns and joins its server thread on normal shutdown and unwind | satisfactory after Phase 8; all 88 main-session tests synchronize on protocol messages, not sleeps; the smaller passive fixtures remain effect-free |

## Guard scope

Phase 5's structural guard (`darkmatter/cli/tests/spawn_site_guard.rs`, two
gates over the L1 population) governs `darkmatter/cli/tests` only. The other
packages' spawn populations are governed differently, on purpose:

| Population | Sites | Why not under the guard |
|---|---|---|
| `dmls/tests/stdio_subprocess.rs` (`bin_exe!("dmls")`) | 1 production-child site | Different binary and contract: a live-child LSP stdio session (initialize → shutdown → exit over real pipes), now launched from an isolated CWD/home/XDG tree and owned by an RAII guard with cross-platform timed wait, kill, and reap. The unwind regression launches the current test binary as a bounded probe. |
| `zed-dmls-cli/tests/cli.rs` (`ZedDmlsFixture::command`) | 6 | Phase 6D migrated all six sites to one package-local fixture. Every command now has fixture-owned CWD, home, config, cache, and temp directories; empty `PATH`; scrubbed Git, rendering, and Darkmatter application inputs; and explicit no-system-Git/no-color defaults. The state-touching tests retain explicit fixture-owned staging/data/log arguments. The real binary and checked-in Zed extension remain the intended end-to-end boundary. |
| `dmls/tests/level2_editor_neovim.rs` (`nvim` shell-outs), `cli/tests/level2_*` (`md_bin()` via `bin_exe!("md")` in `common/level2.rs`) | — | Tier-excluded by the same prefixes the guard mirrors (`level2_`/`level3_`/`browser_`/`real_`/`slow_`): real-terminal/real-tool coverage. Phase 8 gives Neovim and its DMLS child per-test HOME/XDG roots and all pane assertions use bounded final-condition polling. |

A future `test_toolkit` promotion (Phase 11 follow-up) is the point at which a
guard per package area becomes cheap enough to revisit for dmls and
zed-dmls-cli; until then the remaining dmls row above carries the governance.

## Baseline

Phase 1B owns the full record ([`log.md`](log.md#1b--baseline)). Pointers only,
plus the per-family attribution this phase adds.

- **Revision** `973d1d9184601ab3409654955ffacc249f4870a5`, sibling-fix dirty
  entries present and disjoint, toolchain rustc 1.97.1 / nextest 0.9.136,
  profile `default`, host load 10–70 during the runs (contended; attribution
  only).
- **Compatible CI baseline** run `34008778001` (`main` @ `03ce3f8c1`), 21/21
  cells clean; per-leg headline table in the log. One sample per leg.
- **Local runs** (`baseline/local/runs.jsonl`; console logs beside it):

| Cohort | Recipe | Identities run / skipped | Runner elapsed | Summed | Attribution |
|---|---|---:|---:|---:|---|
| `sanity` | `just sanity` | 6,305 / 48 | 44.33 + 0.28 + 1.95 + 0.05 s | 738.2 s | 17 families (lib/bin suites only) |
| `l1-local` | `just test` | 7,661 / 50 | 68.07 s | 1,081.1 s | 50 families |
| `l1-local-include-slow` | `BISCUIT_L1_INCLUDE_SLOW=1 just test` | 7,662 / 49 | 57.50 s | 907.1 s | 51 families (+`lib-override-slow-cleanup`) |
| `l2` | `just test-l2` | 18 + 69 + 3 | 16.23 + 98.36 + 2.52 s | 117.1 s | 4 families |
| `browser` | `just test-browser` | 86 | 31.26 s | 31.3 s | 4 families (42 `browser_render` + 44 `browser_`-prefixed in-process) |
| `l3` | `just test-l3` | — | — | — | **pending**: unattended host, harness refused |
| `doctest` | `just doctest` | 180 / 10 ignored | 1.58 s | — | not a family |

Build/setup for each timed run was ≈2 s (artifacts warm from the captures);
the recipes report nextest's own elapsed, so build and runner time are
separated by subtraction from the wall column in `runs.jsonl`.

### Attribution (Phase 3)

Local timing attributes cost; it sets no target. Three evidence sets, all at
revision `973d1d9` with the same `target/debug/md` the Phase 1 runs used:

- **Per-family, per-leg CI attribution** — `attribution/ci-<leg>-<tier>.json`,
  derived by `test-audit attribute <staging>/<leg>/<tier>/*.xml --json` from
  baseline run `34008778001`'s JUnit; every file was re-derived 2026-09-08 and
  matched to floating-point summation order (tool
  `@rusty-biscuit/test-audit@0.1.0`).
- **Per-binary local attribution** — `attribution/local-l1-by-binary.json` /
  `local-l2-by-binary.json` from the Phase 1 `just test` / `just test-l2`
  console logs (68.07 s elapsed / 1,081.06 s summed / 7,661 results; 0
  violations). Host was contended (load 10–70) — attribution only.
- **Launch-cost probe** — `attribution/launch-probe.sh` →
  `launch-probe-{outside,pkg-cwd}.{json,md}` (hyperfine 1.20.0, 15 runs,
  warmup 3, `--shell=none`) and `launch-probe-burst-*.{json,md}` (16-wide
  bursts, 5 runs): the two launch directories the CLI tests actually use —
  nextest's ambient package CWD (`darkmatter/cli`, inside the monorepo) and a
  directory outside any repository — over the subcommands that dominate the
  504 `md` spawn sites (502 `md_cmd()` call sites + 2 inline spawns;
  first-subcommand census below). Identity in `launch-probe-identity.txt`.

#### Decision 1 — composition vs discovery vs rendering vs setup

Per-invocation `md` costs (mean of 15; the suite's shape is doc-outside +
CWD-inside, because `md_file()` writes to the system temp dir while nextest
sets the package dir as CWD):

| Probe | outside repo | at `darkmatter/cli` | delta |
|---|---:|---:|---:|
| `md --version` (launch floor) | 6.9 ms | — | — |
| `md hash <temp doc>` | 6.4 ms | 7.2 ms | +0.8 ms |
| `md schema validate <temp doc>` | 7.2 ms | 7.5 ms | +0.3 ms |
| `md clean <temp doc>` | 30.5 ms | 30.4 ms | −0.1 ms |
| `md compose <temp doc>` | 66.6 ms | 294.8 ms | **+228.2 ms** |
| `md compose <doc in checkout>` | 527.2 ms | 758.1 ms | +230.9 ms |
| 16-wide burst `hash` | 17.5 ms (1.1 ms/spawn) | 17.6 ms | — |
| 16-wide burst `compose` | 114.9 ms (7.2 ms/spawn) | 692.5 ms (43.3 ms/spawn) | **6.0×** |

The three-way split of the `md` cohort's cost:

- **Process launch — measured and rejected as a driver.** The floor is
  ~7 ms/spawn; 504 sites ≈ 3.5 s of CPU suite-wide, and it parallelizes
  cleanly (16 hash spawns: 17.6 ms wall). The "launch cost × 504" hypothesis
  is now a finding: it is not a material suite cost.
- **Ambient discovery — real, concentrated, and serial.** Compose-family
  invocations pay +228 ms whenever the CWD is inside the monorepo (which
  nextest guarantees), +461 ms more when the composed document itself lives
  in the checkout (`baseline::` JSON, `example-docs`, `CARGO_MANIFEST_DIR`
  fixtures). Unlike launch it does **not** parallelize: 16 concurrent
  in-repo composes take 6× their outside-repo wall, so the ambient path
  contends under nextest's 16-way scheduling. First-subcommand census of the
  504 sites (literal first token, ~400 carry one; the rest are
  variable-driven): compose 99, clean 87, hash 66, schema 63, get 22,
  graph 11, set 9, refs/rm/html/code-block 8 each — the ~110 compose-family
  spawns carry ≥25 s of ambient-discovery CPU in the CLI L1 cohort alone.
- **Actual command work — dominates.** Family attribution below; compose
  families lead every leg. Rendering is a modest L1 share and a large L2
  share (capture tier). Test setup beyond launch shows up as the gap between
  per-test means and probe costs (e.g. `cli-l1-clean-hash-frontmatter`
  80 ms/test mean vs 30 ms probe clean ≈ spawns + temp-file setup +
  assertions per test).

Family-group sums (summed seconds; CI from run `34008778001` L1 cells, local
from the Phase 1 contended run):

| Group | ubuntu | macos | windows | wsl2 | local |
|---|---:|---:|---:|---:|---:|
| lib compose unit families | 422.7 | 278.8 | 590.4 | 626.1 | 621.3 |
| lib compose e2e + context/git | 75.9 | 44.1 | 120.7 | 101.0 | 64.7 |
| CLI `md` cohort (L1 integration) | 182.9 | 138.5 | 311.4 | 250.5 | 185.9 |
| schemas + shipped corpus | 52.4 | 48.9 | 78.1 | 45.8 | 54.6 |
| render (L1; L2 capture and browser excluded) | 27.0 | 59.3 | 34.1 | 24.7 | 31.2 |
| dmls | 31.1 | 31.7 | 42.9 | 32.1 | 27.3 |
| everything else (markdown-rest, snapshots, zed, cli-unit, …) | 92.5 | 98.3 | 137.9 | 101.4 | 96.0 |
| **L1 total** | **884.6** | **699.5** | **1,315.4** | **1,181.6** | **1,081.1** |

Within the CLI cohort, compose-family binaries lead everywhere (local:
`compose_remote_caching` 30.7 s, `compose_schema` 25.1 s, `compose_state_set`
21.5 s, `compose_basic` 12.0 s, `compose_interpolation` 12.0 s), while
hash/clean/schema binaries sit near their probe costs — matching the split
above. WSL2 and native Windows amplify the same families (2–2.4× ubuntu on
`lib-unit-compose-core`, 5.8× on `zed-cli-raw-spawn`), they do not reorder
them.

#### Decision 2 — command paths that genuinely require host tools

Enumerated from the family rows; each is either a **named fixture escape**
(the tool reaches the test through an escape whose call site names it) or a
**genuine tool test** (the tool's behavior is the subject):

| Path | Tool | Verdict |
|---|---|---|
| `cli-l1-compose-shell` (8) | POSIX shell + coreutils (`echo`, `sleep`, `rm` resolution) — shell expansion is the behavior under test | genuine tool test; Phase 6C routes it through the `host_path()` escape with a call-site comment |
| git-dependent context tests (`lib-git-context` 8, `lib-override-ambient-capture` 2, `lib-unit-compose-context` 143, git-using parts of `lib-compose-e2e`) | `git` (repository discovery) | genuine tool need with **fixture-owned topology**: repository-local identity, no host hooks/signing; `git` joins the minimal native set, ambient host repositories are what must not survive (Phases 6C/7) |
| `cli-l2-render-capture` (66), `cli-l2-tmux-schema-about` (3), `cli-l2-harness-integrity` (4) | tmux (CI) + WezTerm (`level2.rs` shared harness, local) | genuine tool tests (terminal behavior is the subject); WezTerm is a POLICY GAP on CI — no runner hosts a GUI terminal; covered by the Phase 1B local run |
| `lib-l2-terminal` (18) | WezTerm | genuine tool test; same POLICY GAP, local coverage only |
| `dmls-l2-neovim` (3) | nvim headless + tmux | genuine tool test; CI provisions nvim on the two L2 legs |
| `lib-browser` (43 after Phase 7) | headless Chrome; one identity additionally needs `mmdc` | genuine tool tests; the Mermaid identity has an explicit recorded availability gate |
| `lib-browser-prefixed-inprocess` (44 baseline identities) | none | not tool tests — Phase 7 renamed them `render_browser_*` so they run in L1 |
| `browser_sanitized_real_mermaid_retains_diagram_geometry` (1) | Mermaid toolchain | genuine tool test, now browser-routed with an explicit Level-2 availability gate and no skip-as-pass for a broken installed CLI |
| `lib-unit-ignored-host-shell` (`resolve_alias_ll`, ignored) | the developer's login shell | genuine host test with an explicit reason string; deterministic behavior stays in stub-shell tests |
| the 504 `md` spawn sites | none beyond the binary (`cargo_bin` resolution, no PATH) | no host tool — what they need controlled is CWD/home/Git plumbing, i.e. Phase 4's fixture, not a PATH escape |
| `mock_http_server` consumers (10) | none (loopback socket) | no host tool; Phase 8 owns the worker lifecycle |
| `zed-cli-raw-spawn` (6), `dmls-l1-stdio-subprocess` (1) | none beyond the binaries under test | no host tool |

Minimal native tool set for Phase 4's default PATH: coreutils + git. Every
other tool above is either a tier-gated genuine tool test or a named escape.

#### Decision 3 — DMLS/extension checks reachable from existing recipes

| Surface | Recipe | CI leg(s) that run it |
|---|---|---|
| dmls unit + bin args (517) | `just sanity` / `just test` | L1 on all four legs |
| dmls LSP sessions, passive proof, workspace fixtures, stdio subprocess, packaging contracts (126) | `just test` | L1 on all four legs |
| dmls L2 neovim (3) | `just test-l2` (`terminal-tests`) | L2 cells: ubuntu-latest, macos-latest (nvim provisioned per leg by `_package-ci.yml`) |
| dmls index bench | `just bench-dmls` | none — a bench route, not a test route |
| zed-dmls-cli unit/bin/raw-spawn (27) | `just sanity` / `just test` | L1 on all four legs |
| Zed extension contract, in-tree (2) | `just test` (`dmls` package) | L1 on all four legs |
| Zed WASM extension compile (`dmls/zed-dmls`, zero tests) | `just check-zed` — standalone and inside `just lint` / `just zed-verify` | dmls **lint job, ubuntu-latest** (the lint job is ubuntu-only; the `zed-extension` provisioning adds `wasm32-wasip2`) |
| Zed extension package verify | `just zed-verify` | `_package-ci.yml:519` — dmls lint job, ubuntu-latest, gated on `runner-tools ⊇ zed-extension` (packager pinned via `.github/ci/zed-extension.json`) |
| `dmls/vscode-dmls` | `just install-vscode-package` (manual) | none — documented absence |

Reachability verdict: every dmls/zed surface **except `vscode-dmls` is
reachable from an existing recipe**. The WASM extension's two checks are
reachable but single-leg (ubuntu lint job) and outside `just test` — correct
per the spec's route requirement, and now stated with the leg that runs it.

#### Decision 4 — repeated corpus/setup operations worth consolidating

Measured (local L1 summed; worst CI leg in parentheses):

| Repeated operation | Family / binaries | Ids | Local summed | Worst CI |
|---|---|---:|---:|---:|
| Shipped schema corpus scans | `lib-shipped-schema-corpus` (`meta_schema_phase1/4/5/6/repo`) | 55 | 5.27 s | 8.22 s (win) |
| Passive schema corpus + literal/projection scans | `lib-passive-schema` (`meta_schema_phase3`, `meta_schema_reference_graph`, `schemas_literal_expression`, `schemas_source_projection`) | 99 | 5.51 s | 10.81 s (win) |
| Full-corpus golden re-conversion | `lib-snapshots-insta` (`schemas_convert_snapshots`) | 52 | 10.17 s | 18.79 s (win) |
| Schema detect/validate tables | `lib-fixture-tables` | 2 | 0.12 s | 0.25 s (win) |
| Embedded unit schemas | `lib-unit-schemas` | 901 | 32.05 s | 37.22 s (win) |
| `example-docs` consumers (lib) | `lib-shipped-example-docs` (`style_frontmatter`) | 14 | 0.46 s | 0.73 s (macos) |
| `example-docs` style parity (render path) | `lib-render-e2e` (`style_frontmatter_parity`) | 106 | 9.92 s | 37.76 s (macos) |
| `baseline::` JSON fixture loaders | `graph` (0.53 s) + `validate_refs` (0.50 s) binaries | 26 | 1.03 s | (inside `cli-l1-compose-render-graph`) |
| `example-docs` + terminal capture (CLI L2) | `cli-l2-render-capture` (`level2_frontmatter_tables` 13.10 s of it) | 66 | 94.31 s (L2) | 6.93 s L2 cell (capture-dominated) |

Verdict:

- **Cost does not justify corpus merging.** The corpus/setup operations above
  total ≈12.5 s of the 1,081 s local L1 sum (1.2%; worst CI leg ≈1.5%). No
  merge passes a cost test.
- **Diagnostic-quality-limited consolidation that is justified:** the
  `meta_schema_*` binaries re-parse the same shipped schemas per binary —
  Phase 7's shared passive corpus test (extended per regression) covers the
  same contract at one parse; and the three dmls `ClientFixture` copies
  (already noted in Shared fixture machinery) consolidate in Phase 8.
- **Not justified:** `schemas_convert_snapshots` (each row is a distinct
  golden; merging loses failure localization), the `example-docs` consumers
  (already 0.46 s, and they are the representative normal-invocation tests
  the spec requires), and the `baseline::` loaders (byte-for-byte
  shipped-artifact coverage is the point of the family).
- The repeated-setup cost that actually matters is per-spawn ambient
  discovery (decision 1) — addressed by the Phase 4/6/7 boundaries, not by
  corpus merges.

#### Attribution discipline and production findings

No speedup is extrapolated from any substage. The standing reason is the
[redundant-walk results](../2026-07-16-redundant-walk/results.md): that fix's
spec-derived acceptance threshold (≥10% **and** ≥500 µs) was falsified by
direct same-run decomposition — the suspected walk cost ≈160 µs (1.5%),
~6× smaller than assumed, because the measured floor was shared validation
work, not the suspected I/O. Attribution precedes any percentage target, and
this phase sets none.

No new production defect surfaced during attribution. The probe's in-repo
compose cost (+228 ms at monorepo CWD; +461 ms for a document inside the
checkout) corroborates the already-documented `ComposeContext::capture()`
walk behavior that motivated `ambient_ctx_capture.rs`'s purpose-built
fixture repository and the 2026-07-16 redundant-walk fix — known, documented
behavior with landed test-side remediation, not a new finding. Nothing was
filed; no production code was touched.

### Budgets

**Pending — one green CI baseline run per leg exists; the gate requires
three consecutive.** `test-audit attribute budgets` over
[`attribution/budgets-pending.json`](attribution/budgets-pending.json)
(per-leg family values filled from run `34008778001`'s JUnit) refuses to
derive any budget:

```text
4 violation(s):
  [insufficient-runs] ubuntu-latest: 1 green run(s); 3 consecutive are required
  [insufficient-runs] macos-latest: 1 green run(s); 3 consecutive are required
  [insufficient-runs] windows-latest: 1 green run(s); 3 consecutive are required
  [insufficient-runs] wsl2-ubuntu: 1 green run(s); 3 consecutive are required
```

Protocol — fixed by the tool, not by discretion: a budget is
`ceil(worst observed family summed × 1.25)` over **three consecutive green
CI runs per leg**; `kind: local` provenance is refused before any leg is
read, so no budget can be derived from a local run. Missing evidence: two
more green runs of the `03ce3f8c1` source state — only
`gh run rerun 34008778001` can produce them, an operator action this phase
does not take (Phase 9 decides whether to request resampling; Phase 10's
three consecutive candidate runs are judged against whatever budgets are
ratified by then). L3 has no CI route at all — its budget stays pending on
principle, not on sampling.

The observed per-family values the future budgets derive from, beside the
baseline they come from (L1 cells, summed seconds; a family absent from a
leg never ran there; identities from the ubuntu L1 cell):

| Family | Ids | ubuntu | macos | windows | wsl2 | Worst |
|---|---:|---:|---:|---:|---:|---:|
| `lib-unit-compose-core` | 1754 | 211.78 | 121.40 | 237.87 | 370.70 | 370.70 |
| `lib-unit-compose-shell` | 532 | 82.55 | 79.61 | 152.94 | 89.54 | 152.94 |
| `cli-l1-compose-render-graph` | 191 | 74.84 | 48.27 | 134.90 | 85.83 | 134.90 |
| `lib-compose-e2e` | 121 | 67.27 | 34.08 | 99.66 | 91.40 | 99.66 |
| `lib-unit-markdown-rest` | 1438 | 65.59 | 68.75 | 88.32 | 81.08 | 88.32 |
| `lib-unit-compose-cleanup-rendering` | 69 | 58.14 | 32.09 | 110.43 | 63.16 | 110.43 |
| `cli-l1-private-helpers` | 116 | 51.53 | 42.15 | 82.82 | 92.28 | 92.28 |
| `cli-l1-compose-remote-http` | 10 | 27.70 | 15.43 | 48.55 | 32.05 | 48.55 |
| `lib-unit-schemas` | 901 | 25.57 | 26.75 | 37.22 | 31.24 | 37.22 |
| `lib-unit-compose-remote` | 129 | 24.13 | 17.49 | 25.48 | 27.19 | 27.19 |
| `dmls-unit` | 511 | 18.80 | 20.05 | 25.82 | 19.56 | 25.82 |
| `lib-unit-compose-context` | 142 | 18.02 | 9.80 | 16.05 | 48.64 | 48.64 |
| `lib-error-snapshots` | 110 | 17.04 | 13.11 | 26.01 | 2.21 | 26.01 |
| `lib-snapshots-insta` | 52 | 13.73 | 10.82 | 18.79 | 1.57 | 18.79 |
| `cli-l1-clean-hash-frontmatter` | 183 | 13.46 | 21.61 | 23.23 | 16.61 | 23.23 |
| `lib-render-e2e` | 106 | 12.93 | 37.76 | 15.13 | 9.19 | 37.76 |
| `dmls-l1-lsp-session` | 107 | 11.26 | 10.06 | 15.40 | 11.44 | 15.40 |
| `lib-unit-compose-preflight-acceptance` | 12 | 10.86 | 6.90 | 16.02 | 8.51 | 16.02 |
| `lib-override-slow-cleanup` | 1 | 10.47 | 6.05 | 19.87 | 12.35 | 19.87 |
| `cli-l1-compose-shell` | 8 | 9.54 | 6.41 | 11.35 | 17.39 | 17.39 |
| `lib-unit-style` | 337 | 6.89 | 9.49 | 10.34 | 8.69 | 10.34 |
| `lib-override-preflight-proptest` | 1 | 6.74 | 5.46 | 11.72 | 5.99 | 11.72 |
| `lib-unit-layout` | 103 | 6.15 | 10.98 | 7.38 | 6.50 | 10.98 |
| `lib-passive-schema` | 99 | 5.95 | 5.56 | 10.81 | 6.16 | 10.81 |
| `cli-l1-compose-diagnostics` | 4 | 5.15 | 3.47 | 9.35 | 5.41 | 9.35 |
| `lib-shipped-schema-corpus` | 55 | 5.03 | 3.82 | 8.22 | 4.75 | 8.22 |
| `lib-unit-support-modules` | 211 | 4.20 | 6.56 | 6.04 | 11.37 | 11.37 |
| `lib-reference-integration` | 62 | 2.73 | 2.37 | 7.06 | 2.39 | 7.06 |
| `lib-compose-shell-e2e` | 24 | 2.47 | 4.52 | 5.84 | 2.85 | 5.84 |
| `lib-structural-guards` | 13 | 2.46 | 1.99 | 3.60 | 2.31 | 3.60 |
| `cli-unit` | 137 | 1.82 | 4.51 | 3.50 | 2.43 | 4.51 |
| `lib-proptest` | 17 | 1.74 | 1.00 | 2.15 | 1.53 | 2.15 |
| `lib-compose-e2e-env-serial` | 2 | 1.38 | 0.78 | 1.97 | 1.25 | 1.97 |
| `lib-git-context` | 8 | 1.15 | 1.03 | 2.73 | 1.98 | 2.73 |
| `lib-snapshots-layout-env` | 3 | 1.07 | 1.06 | 1.25 | 0.30 | 1.25 |
| `lib-override-ambient-capture` | 2 | 0.92 | 1.28 | 3.47 | 1.17 | 3.47 |
| `cli-l1-inline-spawn-baseline` | 2 | 0.64 | 0.94 | 1.04 | 0.81 | 1.04 |
| `lib-work-counters` | 8 | 0.54 | 0.62 | 0.61 | 0.63 | 0.63 |
| `dmls-l1-workspace-fixtures` | 15 | 0.52 | 0.73 | 0.93 | 0.60 | 0.93 |
| `dmls-l1-passive-proof` | 1 | 0.40 | 0.45 | 0.55 | 0.43 | 0.55 |
| `lib-shipped-example-docs` | 14 | 0.26 | 0.73 | 0.65 | 0.41 | 0.73 |
| `zed-cli-raw-spawn` | 6 | 0.24 | 0.98 | 5.82 | 0.15 | 5.82 |
| `lib-browser-binary-unprefixed` | 1 | 0.23 | 0.68 | 1.86 | 0.12 | 1.86 |
| `cli-l2-harness-integrity` | 4 | 0.17 | 0.15 | 0.47 | 0.61 | 0.61 |
| `zed-cli-unit` | 20 | 0.14 | 0.72 | 1.15 | 0.34 | 1.15 |
| `lib-fixture-tables` | 2 | 0.11 | 0.25 | 0.24 | 0.11 | 0.25 |
| `cli-l1-layout-inprocess` | 10 | 0.09 | 0.17 | 0.20 | 0.11 | 0.20 |
| `lib-effects-integration` | 2 | 0.07 | 0.10 | 0.05 | 0.10 | 0.10 |
| `dmls-bin-args` | 6 | 0.04 | 0.12 | 0.07 | 0.04 | 0.12 |
| `lib-l2-binary-unprefixed` | 1 | 0.04 | 0.09 | 0.03 | 0.03 | 0.09 |
| `dmls-l1-stdio-subprocess` | 1 | 0.03 | 0.26 | 0.07 | 0.04 | 0.26 |
| `dmls-l1-packaging-contracts` | 2 | 0.02 | 0.05 | 0.03 | 0.02 | 0.05 |
| `zed-cli-bin-args` | 1 | 0.01 | 0.02 | 0.12 | 0.01 | 0.12 |
| `lib-compose-e2e-windows-only` | 1 | — | — | 0.27 | — | 0.27 |

L2/browser cells (fewer legs carry them; same pending status): ubuntu L2 —
`cli-l2-render-capture` 3.33 s, `cli-l2-tmux-schema-about` 3.77 s,
`dmls-l2-neovim` 1.67 s; macos L2 — 6.93 / 6.29 / 4.07 s; ubuntu browser —
`lib-browser` 37.26 s, `lib-browser-prefixed-inprocess` 3.32 s. `lib-l3-os-input`
has no CI route and no sample. No local number in this document is a target.

## Family index

Sixty-five families cover all 7,892 current candidate identities exactly once;
the Phase 10 reconciler proves it. *Executed* is what the recorded local gates ran on this host
(L1 local, L2, browser); a shortfall against *Identities* is an `#[ignore]`, a
`cfg`, a feature the local recipe never enables, or a tier the host cannot
run — every shortfall is explained in the family's row. *Summed* is the L1
local run unless marked L2/B (browser). **No family was dispositioned by a
timing threshold.**

| Family | Package | Identities | Executed | Summed | Disposition |
|---|---|---:|---:|---:|---|
| `lib-unit-compose-core` | `darkmatter` | 1754 | 1754 | 183.64 s | remediation in this fix (Phase 7 ambient-capture audit) |
| `lib-unit-compose-shell` | `darkmatter` | 532 | 532 | 123.43 s | remediation in this fix (Phase 8 waits and child ownership) |
| `lib-unit-ignored-host-shell` | `darkmatter` | 1 | 0 | not run | remediation in this fix (Phase 7: reason string or stubbed shell) |
| `lib-unit-compose-remote` | `darkmatter` | 129 | 129 | 191.34 s | remediation in this fix (Phase 8) |
| `lib-unit-compose-context` | `darkmatter` | 143 | 143 | 13.00 s | remediation in this fix (Phase 7) |
| `lib-unit-compose-preflight-acceptance` | `darkmatter` | 12 | 12 | 14.74 s | satisfactory |
| `lib-override-preflight-proptest` | `darkmatter` | 1 | 1 | 6.93 s | follow-up: override review is frozen by assumption 6 |
| `lib-unit-compose-cleanup-rendering` | `darkmatter` | 69 | 69 | 88.25 s | remediation in this fix (Phase 3 attributes, Phase 7 acts) |
| `lib-override-slow-cleanup` | `darkmatter` | 1 | 0 | 7.72 s (include-slow) | satisfactory |
| `lib-unit-schemas` | `darkmatter` | 901 | 901 | 32.05 s | satisfactory |
| `lib-unit-markdown-rest` | `darkmatter` | 1438 | 1438 | 63.99 s | satisfactory |
| `lib-unit-style` | `darkmatter` | 337 | 337 | 11.33 s | satisfactory |
| `lib-unit-layout` | `darkmatter` | 103 | 103 | 9.25 s | satisfactory |
| `lib-unit-support-modules` | `darkmatter` | 211 | 211 | 11.01 s | satisfactory |
| `lib-unit-ignored-perf-harness` | `darkmatter` | 4 | 0 | not run | satisfactory |
| `lib-shipped-schema-corpus` | `darkmatter` | 55 | 55 | 5.27 s | satisfactory |
| `lib-fixture-tables` | `darkmatter` | 2 | 2 | 0.12 s | satisfactory |
| `lib-passive-schema` | `darkmatter` | 99 | 99 | 5.51 s | satisfactory |
| `lib-proptest` | `darkmatter` | 17 | 17 | 1.05 s | satisfactory |
| `lib-snapshots-insta` | `darkmatter` | 52 | 52 | 10.17 s | satisfactory |
| `lib-snapshots-layout-env` | `darkmatter` | 3 | 3 | 0.72 s | satisfactory |
| `lib-error-snapshots` | `darkmatter` | 110 | 110 | 12.60 s | satisfactory |
| `lib-compose-e2e` | `darkmatter` | 121 | 121 | 53.86 s | remediation in this fix (Phase 7 ambient-capture audit) |
| `lib-compose-e2e-env-serial` | `darkmatter` | 2 | 2 | 0.84 s | satisfactory |
| `lib-compose-e2e-windows-only` | `darkmatter` | 0 | 0 | not run | satisfactory |
| `lib-compose-shell-e2e` | `darkmatter` | 24 | 24 | 3.62 s | satisfactory |
| `lib-override-ambient-capture` | `darkmatter` | 2 | 2 | 1.71 s | satisfactory |
| `lib-git-context` | `darkmatter` | 8 | 8 | 1.10 s | satisfactory |
| `lib-reference-integration` | `darkmatter` | 62 | 62 | 3.52 s | satisfactory |
| `lib-work-counters` | `darkmatter` | 8 | 8 | 0.70 s | satisfactory |
| `lib-effects-integration` | `darkmatter` | 2 | 2 | 0.80 s | remediation in this fix (Phase 8) |
| `lib-render-e2e` | `darkmatter` | 106 | 106 | 9.92 s | satisfactory |
| `lib-shipped-example-docs` | `darkmatter` | 14 | 14 | 0.46 s | satisfactory |
| `lib-structural-guards` | `darkmatter` | 13 | 13 | 2.47 s | satisfactory |
| `lib-browser-prefixed-inprocess` | `darkmatter` | 44 | 44 | 1.89 s (B) | remediation in this fix (Phase 7: drop the tier marker) |
| `lib-l2-terminal` | `darkmatter` | 18 | 18 | 16.22 s (L2) | satisfactory |
| `lib-l2-binary-unprefixed` | `darkmatter` | 1 | 1 | current L1 gate | remediation in this fix (Phase 7: give it an L1 route) |
| `lib-l3-os-input` | `darkmatter` | 4 | 0 | not run | satisfactory |
| `lib-browser` | `darkmatter` | 42 | 42 | 29.36 s (B) | satisfactory |
| `lib-browser-binary-unprefixed` | `darkmatter` | 1 | 1 | current browser gate | remediation in this fix (Phase 7: route + no skip-as-pass) |
| `cli-unit` | `darkmatter-cli` | 137 | 137 | 2.83 s | satisfactory |
| `cli-fixture-contract` | `darkmatter-cli` | 17 | 17 | current L1 gate | remediation in this fix (Phases 4–6 fixture contract) |
| `cli-spawn-guard` | `darkmatter-cli` | 18 | 18 | current L1 gate | remediation in this fix (Phase 5 structural gate) |
| `cli-l1-private-helpers` | `darkmatter-cli` | 116 | 116 | 42.35 s | satisfactory after Phase 6A fixture migration |
| `cli-l1-inline-spawn-baseline` | `darkmatter-cli` | 2 | 2 | 0.52 s | satisfactory after Phase 6A fixture migration |
| `cli-l1-clean-hash-frontmatter` | `darkmatter-cli` | 183 | 183 | 14.65 s | satisfactory after Phase 6B fixture migration |
| `cli-l1-compose-render-graph` | `darkmatter-cli` | 191 | 191 | 82.87 s | satisfactory after Phase 6C fixture migration |
| `cli-l1-layout-inprocess` | `darkmatter-cli` | 10 | 10 | 0.23 s | satisfactory |
| `cli-l1-compose-shell` | `darkmatter-cli` | 8 | 8 | 7.71 s | satisfactory after Phase 6C named PATH escapes |
| `cli-l1-compose-remote-http` | `darkmatter-cli` | 12 | 12 | Phase 9 measured cohort | remediation in this fix (Phase 8 + 6C) |
| `cli-l1-compose-diagnostics` | `darkmatter-cli` | 4 | 4 | 6.88 s | satisfactory after Phase 6C fixture migration |
| `cli-l2-render-capture` | `darkmatter-cli` | 66 | 66 | 94.31 s (L2) | remediation in this fix (Phase 8 settle-sleeps) |
| `cli-l2-tmux-schema-about` | `darkmatter-cli` | 3 | 3 | 4.04 s (L2) | remediation in this fix (Phase 8: bare-`md` fallback) |
| `cli-l2-harness-integrity` | `darkmatter-cli` | 4 | 0 | not run | remediation in this fix (Phase 5: give the guard's ancestors a local route) |
| `dmls-unit` | `dmls` | 511 | 511 | 17.54 s | satisfactory |
| `dmls-bin-args` | `dmls` | 6 | 6 | 0.12 s | satisfactory |
| `dmls-l1-lsp-session` | `dmls` | 107 | 107 | 8.64 s | satisfactory |
| `dmls-l1-passive-proof` | `dmls` | 1 | 1 | 0.32 s | satisfactory |
| `dmls-l1-workspace-fixtures` | `dmls` | 15 | 15 | 0.53 s | satisfactory |
| `dmls-l1-stdio-subprocess` | `dmls` | 3 | 3 | current L1 gate | remediation in this fix (Phase 8) |
| `dmls-l1-packaging-contracts` | `dmls` | 2 | 2 | 0.05 s | satisfactory |
| `dmls-l2-neovim` | `dmls` | 3 | 3 | 2.51 s (L2) | remediation in this fix (Phase 8: poll the asserted frame) |
| `zed-cli-unit` | `zed-dmls-cli` | 20 | 20 | 0.45 s | satisfactory |
| `zed-cli-bin-args` | `zed-dmls-cli` | 1 | 1 | 0.02 s | satisfactory |
| `zed-cli-raw-spawn` | `zed-dmls-cli` | 6 | 6 | 1.14 s | satisfactory after Phase 6D package-local fixture migration |

## Families

Fields per row: members; behavior proved and whether the assertions
distinguish a plausible failure; shared setup; external inputs (CWD,
home/config/cache, environment, repository, installed tools, network,
terminal, browser); effect execution; waiting and timing floors; resource
ownership and cleanup; shared-state coordination; tier, features, platforms;
canonical recipe and route; measured cost with provenance; disposition.

A family shares a row only when its members share setup and proof. Where a
member differs it has its own family — that is what the `*-override-*`,
`*-ignored-*`, `*-unprefixed`, `*-env-serial` and `browser-prefixed` carve-outs
are. Costs cite `baseline/local/test-l1-local.log` unless stated.

### `darkmatter` — embedded unit suite (`darkmatter`, 5,678 identities)

#### `lib-unit-compose-core` — 1,754 identities

- **Members** — `markdown::compose::*` minus the carve-outs below. By
  sub-module: `expression` 646, `interpolation` 187, `tests::transclusion_tests`
  63 (of 88; the 25 `remote_transclusion_tests` are carved out),
  `schema_validation` 73, `conditions` 68, `frontmatter_interpolation` 65,
  `toc_linking` 62, `type_tests` 62, `preflight` 58 (of 71; the 13
  `acceptance_tests` are carved out), `file_links` 57, `transclusion` 55,
  `tests::schema` 42, `tests::frontmatter` 40, `page_blocks` 34, `replacement`
  32, `subtree` 22, `directives_api` 21, `block_pairs` 16, `link_resolve` 14,
  `indent` 11, `link_normalization` 10, `parse_utils` 9, `perf` 8, `cache` 97,
  `util` 2.
- **Purpose** — in-process coverage of the twelve compose stages, the
  expression catalog, file-link discovery, the compose cache and preflight.
- **Assertion quality** — sampled; asserts composed output and typed errors.
  Not exhaustively reviewed at this granularity; Phase 7's audit reviews the
  sites it touches.
- **Shared setup** — `tempfile` in 41 compose files; `#[serial]` in 12
  compose modules (intra-binary only under nextest); `std::env::set_var` in
  `conditions.rs`, `expression/ctx.rs`, `link_resolve.rs`,
  `transclusion/resolver.rs` (and three `context` files, carved out).
- **External inputs** — Phase 7 audited the 25 compose files that construct
  `ComposeOptions::new()` or capture context. `ComposeOptions::new()` now
  installs zero-discovery date/time + environment state and upgrades from the
  document's actual `ctx.*` requirements; explicit full captures in this
  family are behavior tests. `CARGO_MANIFEST_DIR`
  is read in 4 compose files. Sleeps: `cache/runtime.rs:1262,1442` (50 ms
  polls), `expression/resolve_ctx.rs:623` (20 ms), `expression/functions/mod.rs:4951`
  (200 ms tokio).
- **Effects** — file writes in temp dirs; no network in this family (network
  modules are carved out).
- **Timing** — summed 183.64 s / 1,754 (105 ms mean), max 3.51 s. No
  intentional floor.
- **Ownership** — temp dirs dropped; no children.
- **Tier / route** — L1, no features, all four legs; `just sanity`, `just test`.
- **Cost provenance** — L1 local log; sanity 140.31 s.
- **Disposition** — satisfactory after Phase 7: the one incidental full
  capture in `expression_regression` now requests only its agent/model group;
  embedded capture sites either use fixed/minimal context or deliberately test
  context capture. Representative E2E capture remains named below.

#### `lib-unit-compose-shell` — 532 identities

- **Members** — `markdown::compose::shell_expansion::*` (296; `resolve_alias_ll`
  carved out), `frontmatter_shell_expansion::*` (115), `shell_blocks::*` (101),
  `tests::shell::*` (20).
- **Purpose** — `$()` expansion, frontmatter shell expansion, `::shell` blocks:
  execution, timeouts, ANSI stripping, `NO_COLOR`, approval policy.
- **Assertion quality** — good; `timeout_kills_long_running_command`,
  `stdin_is_null_command_does_not_hang` and
  `detached_child_output_still_reaches_the_caller` each name the hang or leak
  they distinguish.
- **Shared setup** — stub shells written into temp dirs (`alias.rs`), a Python
  snippet (`execution_tests.rs:331`, `time.sleep(0.01)` inside the child).
- **External inputs** — **spawns real `sh`/`python`/stub shells** through
  `Command::new` (`alias.rs`, `executor.rs`, `shell_expansion/mod.rs`,
  `execution_tests.rs`); needs a working shell on `PATH`. 32 tests are
  `#[cfg(unix)]` (16 `executor`, 4 `alias`, 12 elsewhere) and absent on
  `windows-latest`.
- **Waiting** — `executor.rs:1362` 20 ms poll, `:1627` 400 ms fixed sleep,
  `shell_expansion/mod.rs:3126` 1 ms spin; the alias resolver has a 10 s
  timeout contract.
- **Timing** — summed 123.43 s / 532 (232 ms mean, **2.2× the compose-core
  mean**), max 3.00 s. Child startup is part of the floor.
- **Ownership** — children waited or killed on timeout; `detached_child_…`
  intentionally outlives the shell.
- **Tier / route** — L1, no features; `just sanity`, `just test`.
- **Disposition** — **remediation in this fix (Phase 8)**: classify each sleep
  as timeout contract or readiness wait and bound the latter; confirm every
  child is reaped on failure paths.

#### `lib-unit-ignored-host-shell` — 1 identity

- **Member** — `markdown::compose::shell_expansion::alias::tests::resolve_alias_ll`.
- **Purpose** — resolve the `ll` alias through the developer's real login shell.
- **Assertion quality** — **weak**: asserts a non-empty executable only when the
  alias exists; otherwise prints and passes. Cannot distinguish a broken
  resolver on a host without the alias.
- **External inputs** — the host `$SHELL` and its rc files.
- **Gate / route** — ``#[ignore = "requires an `ll` alias in the user's
  login-shell configuration"]``; manual `--run-ignored` only.
- **Disposition** — satisfactory after Phase 7. Deterministic success and
  negative behavior remain covered by the stub-shell siblings; this host-only
  smoke test now states why it is outside canonical gates.

#### `lib-unit-compose-remote` — 129 identities

- **Members** — `markdown::compose::remote_fetch::*` (23), `remote::*` (52),
  `tests::provider_network::*` (29), `tests::transclusion_tests::remote_transclusion_tests::*` (25).
- **Purpose** — remote read policy, persistent cache freshness/refresh/stale
  fallback, conditional 304/200, concurrency cap, provider (`pr*`/`cicd*`)
  binding through compose, remote transclusion preflight.
- **Assertion quality** — strong; `identical_provider_calls_reach_the_server_exactly_once`
  and the request-count assertions are work-count proofs of the kind RB3 asks for.
- **Shared setup** — `TcpListener` mock servers (`remote_fetch.rs` 24 sites,
  `provider_network.rs` 5, `tests/transclusion.rs` 25) on `127.0.0.1:0`;
  `remote::*` tests are pure (0 servers).
- **External inputs** — loopback network only; cache roots in temp dirs.
- **Waiting** — `remote_fetch.rs`: 200 ms ×3, 400 ms, **1,400 ms**, and 18
  `SETTLE` sleeps (`:1134–1447`) inside the persistent-cache tests; 20 ms poll
  at `:842`.
- **Timing** — **summed 191.34 s / 129 (1.48 s mean) — the most expensive
  family in the area by sum and by mean**; max 5.51 s
  (`preflight_graph_reuse_recurses_to_remote_grandchild`). The `provider_network`
  tests sit at 3.7–4.2 s each, `persistent_cache_tests` at 3.6–3.8 s each.
- **Ownership** — server threads: to be verified joined/shut down in Phase 8
  (the CLI fixture of the same shape is not).
- **Tier / route** — L1, no features, all four legs.
- **Disposition** — **remediation in this fix (Phase 8)**: the `SETTLE` sleeps
  are the first candidates for "poll the final asserted condition"; Phase 3
  attributes how much of the 1.48 s mean is sleep versus work before any
  change.

#### `lib-unit-compose-context` — 143 identities

- **Members** — `markdown::compose::context::*` (capture, catalog, options,
  repository scope, runtime).
- **Purpose** — `ctx.*` capture semantics, catalog projection, options
  identity and cache fingerprints, repository-scope projection.
- **Assertion quality** — good; `capture_shape_matches_projected_type` compares
  every captured value against its projected SimplifiedSchema type.
- **Shared setup** — `set_var` in `capture/agent.rs`, `options.rs`,
  `runtime.rs` (intra-binary `#[serial]`).
- **External inputs** — `capture_shape_matches_projected_type` performs a full
  capture against a disposable `gix` repository boundary rather than the
  monorepo checkout. Two
  `cfg(unix)` non-UTF-8 path tests; one `cfg(target_os = "macos")` symlink
  spelling test.
- **Timing** — summed 13.00 s / 143 (91 ms mean); without the one capture test
  the family mean is ≈55 ms.
- **Tier / route** — L1, all legs (minus the three `cfg` members on Windows).
- **Disposition** — satisfactory after Phase 7: the shape matrix retains its
  full-capture subject at a bounded disposable repository; the broader ambient
  parity case remains in `lib/tests/ambient_ctx_capture.rs`.

#### `lib-unit-compose-preflight-acceptance` — 12 identities

- **Members** — `markdown::compose::preflight::acceptance_tests::*` except the
  override target.
- **Purpose** — the preflight approval graph: execution ⊆ approval across
  states, dead branches approved but not executed, graph reuse to grandchildren,
  span re-anchoring after offset-shifting stages.
- **Assertion quality** — strong; property-shaped acceptance criteria.
- **External inputs** — none beyond temp dirs; no shell executes (the point).
- **Timing** — summed 14.74 s / 12, max 4.46 s (`approval_set_is_loop_stable`).
- **Disposition** — satisfactory; the 4.46 s is Phase 3 attribution input.

#### `lib-override-preflight-proptest` — 1 identity

- **Member** — `markdown::compose::preflight::acceptance_tests::execution_subset_of_approval_across_randomized_conditions`.
- **Purpose** — proptest over randomized compose conditions proving execution
  never exceeds approval.
- **Timing** — 6.93 s (L1), 4.55 s (sanity), 4.31 s (include-slow): the
  override's own "~7 s isolated" matches.
- **Overrides** — `slow-timeout = 30s × 3` in **both** profiles.
- **Disposition** — **follow-up**: re-justifying or narrowing the override is
  a `.config/nextest.toml` change, which assumption 6 freezes for this fix.
  Recorded in the census; Phase 11 files it.

#### `lib-unit-compose-cleanup-rendering` — 69 identities

- **Members** — `markdown::compose::tests::rendering::*` except the `slow_` test.
- **Purpose** — inline cleanup and reflow after composition preserves list
  markers, blockquotes, protected bodies and nested structures.
- **Assertion quality** — good; byte-level expected output per case.
- **External inputs** — in-process; whether each case pays an ambient
  `ComposeOptions::new()` capture is what Phase 3 must attribute.
- **Timing** — summed 88.25 s / 69 (**1.28 s mean**), max **8.96 s**
  (`test_compose_cleanup_preserves_nested_lists_inside_blockquotes`, the
  slowest L1 identity in the area), then 5.69 s and 5.54 s. Under
  `BISCUIT_L1_INCLUDE_SLOW=1` the same family summed 47.23 s (mean 0.69 s) —
  the spread is host contention, which is why these numbers attribute and do
  not budget.
- **Disposition** — **remediation in this fix**: Phase 3 splits the cost into
  composition versus discovery; if ambient capture dominates, Phase 7 supplies
  explicit context. No assertion changes.

#### `lib-override-slow-cleanup` — 1 identity

- **Member** — `markdown::compose::tests::rendering::slow_compose_cleanup_preserves_quoted_marker_looking_indented_code`.
- **Purpose** — a merge-regression test performing several complete compose
  passes.
- **Gate** — `slow_` prefix (excluded from local `just test`/`sanity`, included
  on CI via `BISCUIT_L1_INCLUDE_SLOW=1`) **and** a default-profile
  `slow-timeout = 30s × 3` override.
- **Timing** — 7.72 s in the include-slow run.
- **Disposition** — satisfactory: this is the area's documented local/CI
  cohort split. The override half is in the census follow-up.

#### `lib-unit-schemas` — 901 identities

- **Members** — `markdown::schemas::*` (SimplifiedSchema grammar, convert,
  resolve, coerce, triggers, format, rewrite, clean, source).
- **Purpose** — passive schema parsing, lowering to Draft 2020-12, coercion,
  trigger matching, source projection.
- **Assertion quality** — good; exact lowered fragments and typed problem codes.
- **Shared setup** — `tempfile` in 11 files (trigger discovery over real
  directories); `#[serial]` in 6 modules; `set_var` in `schemas/mod.rs`,
  `resolve.rs`.
- **External inputs** — one `#[cfg(any(unix, windows))]` symlink test; the
  four `coerce` tests the scan misses (see reconciliation).
- **Effects** — none by contract. Phase 7 enables `effects-instrumentation` in
  canonical L1 and `expression_validation_never_evaluates` asserts zero
  `EffectEngine` builds and zero network attempts around the original hostile
  shell/environment and missing-file expressions.
- **Timing** — summed 32.05 s / 901 (36 ms mean), max 0.27 s.
- **Disposition** — satisfactory.

#### `lib-unit-markdown-rest` — 1,438 identities

- **Members** — `markdown::*` outside `compose` and `schemas`, minus 3
  perf-harness tests and 14 `browser_`-prefixed tests: `render_tree` 230,
  `cleanup` 224, `reference` 188, `hash` 113, `output` 91, `highlighting` 88,
  `tests` 51, `language_grammar` 47, `toc` 46, `frontmatter` 44, `inline` 41,
  `yaml_block` 41, `errors` 40, `code_block` 35, `normalize` 34, `dsl` 33,
  `block` 32, `delta` 30, `inline_html` 22, `span` 9, `fs` 1.
- **Purpose** — parser, render tree, cleanup, reference graph, hashing,
  highlighting, TOC, frontmatter, delta.
- **Shared setup** — `#[serial]` + `set_var` in `highlighting/themes.rs`,
  `reference/graph.rs`, `render_tree/code_renderer.rs`, `yaml_block.rs`,
  `code_block.rs` (theme/color env); `tempfile` in `reference` (4 files), `fs.rs`,
  `errors`.
- **External inputs** — `reference` tests build real directories; nothing
  reads the checkout except through `CARGO_MANIFEST_DIR` in `schemas` (carved
  out above). `ComposeContext::capture` appears in 3 `reference` files —
  Phase 7 checks whether those are subject or incidental.
- **Timing** — summed 63.99 s / 1,438 (45 ms mean), max 1.70 s.
- **Disposition** — satisfactory.

#### `lib-unit-style` — 337 identities

- **Members** — `style::*` (apply 63, schema 57, bespoke 49, cli_claims 44,
  parse 39, color 32, length 13, walker 12, descriptor 9, coverage_tests 6,
  alignment/error/warning 4 each, tests 1).
- **Purpose** — style frontmatter parsing, claims merging, lowering.
- **Shared setup** — one `tempfile` use (`bespoke.rs`).
- **Timing** — summed 11.33 s / 337 (34 ms mean), max 0.19 s.
- **Disposition** — satisfactory.

#### `lib-unit-layout` — 103 identities

- **Members** — `layout::*` minus the perf-harness test and the 28
  `browser_`-prefixed tests: `page` 99, `context` 2, `types` 2.
- **Purpose** — `DarkmatterPage` policy resolution and terminal rendering.
- **Shared setup** — `set_var` + `#[serial]` in `layout/page/tests.rs`.
- **Timing** — summed 9.25 s / 103 (90 ms mean), max 1.13 s.
- **Disposition** — satisfactory.

#### `lib-unit-support-modules` — 211 identities

- **Members** — `render` 46, `mermaid` 42, `diff` 35, `terminal` 26,
  `catalog` 22, `editor` 17, `effects` 15, `testing` 8.
- **Purpose** — link/image reference parsing, Mermaid theming, visual diff,
  terminal ANSI, the `ctx` catalog, editor launching, effect verbs.
- **Shared setup** — `set_var` + `#[serial]` in `editor/mod.rs`,
  `terminal/tests.rs`, `render/link.rs`, `render/image_ref.rs`; `tempfile` in
  `effects/*` and `editor`.
- **External inputs** — `editor::*` (6 `cfg(unix)` tests) spawns stub editor
  commands through `Command::new` with `EDITOR`/`VISUAL` set; `effects` writes
  files and logs into temp dirs.
- **Timing** — summed 11.01 s / 211 (52 ms mean), max 2.09 s.
- **Disposition** — satisfactory.

#### `lib-unit-ignored-perf-harness` — 4 identities

- **Members** — the four `*_raw_samples` tests listed in
  [Ignored and slow tests](#ignored-and-slow-tests).
- **Purpose** — retained per-observation vectors for the 2026-07-15
  performance follow-up's findings F23/F25/F35.
- **Gate / route** — `#[ignore = "measurement harness …"]` plus a `DM_PERF_RAW_DIR`
  check that returns early; no canonical recipe; manual invocation documented in
  `lib/src/perf_harness.rs`.
- **Disposition** — satisfactory: intentional, documented, reason strings
  present, zero cost to every gate.

### `darkmatter` — integration binaries (`lib/tests/`, 768 identities)

#### `lib-shipped-schema-corpus` — 55 identities

- **Members** — `base_schema_end_to_end` (15), `meta_schema_phase1` (8),
  `meta_schema_phase4` (6), `meta_schema_phase5` (3), `meta_schema_phase6` (9),
  `meta_schema_repo_schemas` (3), `predict_conflicts` (11).
- **Purpose** — every shipped schema under `darkmatter/docs/schemas/` and the
  repo-root `schemas/` classifies, compiles and validates real documents; the
  expression-function catalog matches its shipped YAML.
- **Assertion quality** — strong; this is the passive shipped-artifact corpus
  coverage RB3 requires kept.
- **External inputs** — reads the checkout via `CARGO_MANIFEST_DIR`
  (`../docs/schemas`, `../../schemas`, `include_str!` of
  `expression-functions.yaml`); `predict_conflicts` has one `cfg(unix)`
  git-access-failure test.
- **Timing** — summed 5.27 s / 55, max 0.63 s.
- **Disposition** — satisfactory. Phase 3 decision 4 measures whether
  `meta_schema_phase4`'s walk and `meta_schema_repo_schemas` duplicate work.

#### `lib-fixture-tables` — 2 identities

- **Members** — `schemas_detect_table::*` (1), `schemas_validate_table::*` (1).
- **Purpose** — table-driven detection (7 cases under `tests/fixtures/detect/`)
  and validation (33 cases under `tests/fixtures/validate/`), one identity each.
- **External inputs** — the committed fixture tree via `CARGO_MANIFEST_DIR`.
- **Timing** — 0.12 s summed.
- **Disposition** — satisfactory; the house shape (one binary, many cases).

#### `lib-passive-schema` — 99 identities

- **Members** — `meta_schema_phase3` (6), `meta_schema_reference_graph` (9),
  `schemas_literal_expression` (44), `schemas_source_projection` (6),
  `suggest_constraint_phase1–4` (12 + 7 + 5 + 10).
- **Purpose** — `literal()` / `expression` / `yaml` / `json` meta-types stay
  passive; `suggest()` constraints; source projection; the schema reference
  graph. Quoted-vs-native YAML forms are asserted as observably different.
- **External inputs** — temp dirs; `suggest_constraint_phase1` `include_str!`s
  the dmls fixtures (shared corpus, read at compile time).
- **Effects** — none by contract. Phase 7 adds counter assertions to
  `expression_validation_never_evaluates` and
  `semantic_types_match_triggers_by_passive_parse`; native, quoted, missing,
  null, malformed, remote-looking, and integer-boundary cases remain at this
  cheap schema boundary.
- **Timing** — summed 5.51 s / 99, max 0.69 s.
- **Disposition** — satisfactory.

#### `lib-proptest` — 17 identities

- **Members** — `schema_quoting_safety` (15: 8 plain + 7 `proptest!`),
  `schemas_grammar_proptest` (2 `proptest!`).
- **Purpose** — quoting idempotence and grammar round-trips over random atoms.
- **Shared setup** — committed `.proptest-regressions` files.
- **Timing** — summed 1.05 s.
- **Disposition** — satisfactory.

#### `lib-snapshots-insta` — 52 identities

- **Members** — `cutover_reference` (6), `horizontal_rule_snapshots` (3),
  `render_tree_hr_snapshots` (3), `schemas_convert_snapshots` (40).
- **Purpose** — `insta` snapshots of terminal/browser output and schema
  lowering under `tests/snapshots/`.
- **Timing** — summed 10.17 s / 52 (196 ms mean).
- **Disposition** — satisfactory.

#### `lib-snapshots-layout-env` — 3 identities

- **Members** — `layout_snapshots::*`.
- **Purpose** — the layout spec's worked example, end to end.
- **Shared setup** — **9 `set_var`/`remove_var` sites** with
  `#[serial(layout_snapshot_env)]`; the tests set the rendering env they
  depend on rather than inheriting it.
- **Timing** — 0.72 s summed.
- **Disposition** — satisfactory (coordination is intra-binary, which is the
  only place it is needed).

#### `lib-error-snapshots` — 110 identities

- **Members** — the 16 modules of `error_snapshots` (`condition`, `ctx_merge`,
  `deferred_set`, `editor`, `file_tree`, `image_ref`, `link`, `markdown_error`,
  `mermaid_theme`, `normalization`, `page_block`, `reference`,
  `shell_expansion`, `stylesheet`, `toc_linking`, `transclusion`).
- **Purpose** — every `BlockError` renders its excerpt, gutter, arms and
  hyperlinks as specified.
- **Shared setup** — `helpers.rs` (`test_ctx`, `render`, `strip_ansi`); a
  fixed display path under `/tmp/test/` that is never touched on disk.
- **Timing** — summed 12.60 s / 110 (115 ms mean).
- **Disposition** — satisfactory.

#### `lib-compose-e2e` — 121 identities

- **Members** — `compose_reuse_phase5` (2), `compose_phase6` (2),
  `expression_regression` (43 of 45), `link_interpolation_integration` (4),
  `more_is_more_literals_and_indexes` (3), `set_overlay_integration` (18),
  `ternary_integration` (11), `yaml_block_parity` (7),
  `disclosure_transclusion_integration` (4), `inline_envelope_prototype` (9),
  `span_compat` (4), `tree_features_characterization` (14).
- **Purpose** — real composition through the public API: expression syntax,
  `--set` overlays, ternaries, YAML blocks, transclusion, inline envelopes.
- **External inputs** — `compose_phase6` reads the committed benchmark fixtures
  under `features/2026-07-15-performance-followup/benchmarks/fixtures/` via
  `CARGO_MANIFEST_DIR`; most members compose with `ComposeOptions::new()`.
- **Timing** — summed 53.86 s / 121 (445 ms mean), max 2.31 s.
- **Disposition** — satisfactory after Phase 7. The agent/model regression now
  captures only the groups named by its original document; real shell,
  interpolation, transclusion, hashing, and persistence composition tests were
  retained unchanged.

#### `lib-compose-e2e-env-serial` — 2 identities

- **Members** — `expression_regression::{regression_ctx_agent_in_interpolation, regression_page_block_with_has_skill}`.
- **Purpose** — `ctx.agent` from the environment; `has_skill` against a
  pinned git root.
- **Shared setup** — `#[serial_test::serial(env_agent_model)]`; the second
  pins a temp dir as the git root deterministically.
- **Timing** — 0.84 s summed.
- **Disposition** — satisfactory: already fixture-anchored.

#### `lib-compose-e2e-windows-only` — 0 identities on this host

- **Members** — `declined_path_transclusion::{transcluded_child_under_a_declined_path_becomes_a_visible_notice, fail_fast_surfaces_the_declined_child_link_as_an_error, transcluded_child_under_an_ordinary_path_still_composes}`.
- **Purpose** — transclusion ordering at the portable-string boundary for
  declined (UNC/long/verbatim) Windows paths.
- **Route** — `#![cfg(windows)]`; `windows-latest` L1 only (`expectEmpty`).
- **Disposition** — satisfactory.

#### `lib-compose-shell-e2e` — 24 identities

- **Members** — `shell_block_integration` (16), `shell_expansion_coordinates`
  (7), `interpolation_literal_pipeline` (1).
- **Purpose** — `::shell` blocks and `$()` expansion through the full
  pipeline, with source coordinates.
- **External inputs** — **real shells**; `interpolation_literal_pipeline`
  spawns the same executable directly to compute its expected output.
- **Timing** — summed 3.62 s / 24, max 1.08 s.
- **Disposition** — satisfactory: the shell is the subject (a genuine tool
  test for Phase 3 decision 2).

#### `lib-override-ambient-capture` — 2 identities

- **Members** — `ambient_ctx_capture::{every_catalog_variable_survives_ambient_options, …}` (2, both `#[serial]`).
- **Purpose** — full `ComposeContext::capture()` parity with ambient
  `ComposeOptions::new()` over every `ctx.*` catalog variable.
- **Shared setup** — a purpose-built two-package fixture repository with a
  commit, staged and dirty files, documents and a skill (`Command::new("git")`),
  precisely so the capture does not walk this checkout.
- **Overrides** — `slow-timeout = 30s × 3` in both profiles on the first test.
- **Timing** — 1.71 s summed, max 1.38 s.
- **Disposition** — satisfactory: Phase 7 verified this prior remediation
  without rewriting it. These two tests are the deliberately retained full
  ambient capture E2E cases. Override handling remains in the census follow-up.

#### `lib-git-context` — 8 identities

- **Members** — `git_context_integration::*`.
- **Purpose** — repository facts (branch, conflicts, staged state) captured
  through `gix` from hand-built `.git` directories.
- **Shared setup** — `init_repo` writes `.git/HEAD`/`config`/index into a temp
  dir; no `git` binary.
- **Timing** — 1.10 s summed.
- **Disposition** — satisfactory. Phase 7 deliberately retains
  `normal_compose_path_renders_all_git_context_values_from_one_snapshot` as the
  public compose-path proof that one captured Git snapshot drives branch,
  worktree, conflict output, and diagnostics.

#### `lib-reference-integration` — 62 identities

- **Members** — `reference_integration::*`.
- **Purpose** — composed reference graph and validation over real temp
  documents.
- **Shared setup** — `tempfile`; one `#[serial]` permission test (`cfg(unix)`).
- **Timing** — 3.52 s summed, max 1.09 s.
- **Disposition** — satisfactory. Phase 7 deliberately retains
  `explicit_context_is_shared_by_enumeration_graph_and_validation` as the
  provenance proof: one disposable repository and one explicit context drive
  relative enumeration, graph construction, and validation consistently.

#### `lib-work-counters` — 8 identities

- **Members** — `clean_counters::*` (all `#[serial]`).
- **Purpose** — `md clean` performance as a correctness property: zero schema
  work without frontmatter, per-run trigger/validator caching.
- **Shared setup** — process-wide counters (hence serial within the binary).
- **Timing** — 0.70 s summed.
- **Disposition** — satisfactory: the counter-assertion model RB3 and RB5 want
  more of.

#### `lib-effects-integration` — 2 identities

- **Members** — `effects_integration::{file_and_dir_verbs, http_post_uses_allowed_host_policy}`.
- **Purpose** — effect verbs write files/logs; `http_post` honors host policy
  and reaches a local server exactly as counted.
- **Shared setup** — `TcpListener` on `127.0.0.1:0` served from a
  **`thread::spawn` whose handle is not retained**.
- **Timing** — 0.80 s summed.
- **Disposition** — **remediation in this fix (Phase 8)**: join the worker and
  bound its lifetime, same contract as the CLI HTTP fixture.

#### `lib-render-e2e` — 106 identities

- **Members** — `render_comparison` (1), `render_invariants` (6),
  `render_tree_roundtrip` (15), `disclosure_render_targets` (12 of 14),
  `horizontal_rule_integration` (27), `html_inversion` (3),
  `blockquote_list_spacing` (1), `prose_wrap_parity` (3), `layout_matrix` (5),
  `style_features_baseline` (5), `style_features_phase5` (13),
  `style_frontmatter_parity` (14), `debug_test` (1).
- **Purpose** — render-tree fold invariants, terminal/HTML parity, layout
  matrix snapshots.
- **External inputs** — `render_tree_roundtrip` reads `tests/fixtures/render_tree`
  via `CARGO_MANIFEST_DIR`; `layout_matrix` uses `layout_matrix_support`.
- **Timing** — 9.92 s summed.
- **Disposition** — satisfactory.

#### `lib-shipped-example-docs` — 14 identities

- **Members** — `style_frontmatter::*`.
- **Purpose** — parse `darkmatter/example-docs/rendering/style-prop.md` and
  assert every spec acceptance field, then lower onto a `DarkmatterPage`.
- **External inputs** — the shipped example document via `CARGO_MANIFEST_DIR`.
- **Timing** — 0.46 s summed.
- **Disposition** — satisfactory; Phase 3 decision 4 asks whether
  `cli/tests/level2_frontmatter_tables.rs` re-reads the same document.

#### `lib-structural-guards` — 13 identities

- **Members** — `as_block_error_registry` (1), `prelude_exports` (8),
  `benchmark_fixtures` (4; 1 `proptest!`).
- **Purpose** — every `BlockError` is registered; the prelude re-exports what
  it promises; the benchmark fixture manifest still matches the committed
  fixture bytes and the TOC `line_at_offset` fast path equals the naive count.
- **External inputs** — read `lib/src` and
  `features/2026-07-15-performance-followup/benchmarks/manifest.yaml` from the
  checkout.
- **Timing** — 2.47 s summed.
- **Disposition** — satisfactory.

#### `lib-browser-prefixed-inprocess` — 44 baseline identities

- **Members** — 28 `layout::page::tests::render_browser_*`, 4
  `markdown::code_block::tests::render_browser_*`, 6
  `markdown::render_tree::code_renderer::tests::render_browser_*`, 4
  `markdown::render_tree::entrypoints::tests::render_browser_*`, and
  `disclosure_render_targets::{render_browser_target_uses_native_details_summary, render_browser_target_renders_nested_disclosures}`.
- **Purpose** — HTML/CSS emission for the browser *target*: class names,
  `max-width`, margins, color CSS, `<details>`/`<summary>`.
- **Assertion quality** — good as unit tests (exact substrings of the emitted
  HTML). **They never touch a browser**: no `ChromeHarness`, no `require_browser`.
- **Baseline route defect** — the old `browser_` leaf prefix made `_tier_filter`
  treat them as Browser-tier, excluding them from L1 on all four CI legs.
- **Disposition** — satisfactory after Phase 7. Renaming only the leaf prefix
  to `render_browser_*` preserves all 44 public-result assertions and routes
  them through L1 on macOS, Linux, Windows, and WSL2. A focused L1 run executed
  all 44 replacements.

#### `lib-l2-terminal` — 18 identities

- **Members** — `level2_render_tree_terminal::{basic_spans (5), code_panel (3), file_links (1), images (2 of 3), layout_policy (5), public_entry_points (2)}`.
- **Purpose** — render-tree output survives a real terminal: SGR, OSC-8, code
  panels, images, page frames.
- **Shared setup** — `support/mod.rs`; `SharedHarness<WezTermHarness>` with
  `atexit` cleanup; `#[serial(level2_terminal)]` on every test; 50 ms sentinel
  poll to a deadline.
- **External inputs** — **WezTerm** with `WEZTERM_UNIX_SOCKET` (this session ran
  inside WezTerm, so they ran). **CI POLICY GAP**: the lib declares
  `l2-backends = ["wezterm"]` and no runner hosts WezTerm, so this family has
  local evidence only; recorded in Phase 1.
- **Overrides** — the census's dead `level2_render_tree_style_in_wezterm` entry
  once pointed here.
- **Timing** — 16.22 s summed (L2 log), max 2.65 s.
- **Disposition** — satisfactory; CI evidence pending by policy, not by defect.

#### `lib-l2-binary-unprefixed` — 1 baseline identity

- **Member** — now `image_pixel_classification::pixel_classification_distinguishes_magenta_from_black`.
- **Purpose** — the PNG pixel classifier used by the image L2/L3 tests.
- **Baseline route defect** — a pure function test inside a `terminal-tests`-gated
  binary with no `level2_` prefix: `test-l2` filters it out, L1 never compiles
  it. Runs only on CI L1 (all four legs). Never executed locally.
- **Disposition** — satisfactory after Phase 7. The classifier and PNG fixture
  helper are shared test support, while this exact black-versus-magenta test is
  in an always-built integration binary and passed under L1. The L2 and L3
  image tests continue to use the same helper.

#### `lib-l3-os-input` — 4 identities

- **Members** — `level3_image_painting::level3_rich_image_node_paints_distinctive_pixels`
  (WezTerm + `screencapture`), `level3_popover::{tab_focuses_anchor_and_reveals_prompt, enter_activates_link, pointer_hover_reveals_prompt}`
  (headed Chrome + `cliclick`).
- **Purpose** — genuine OS-input evidence: pixels actually paint; keyboard and
  pointer reach the popover.
- **Gate** — `#[cfg(target_os = "macos")]`, `require_level!(Level::L3, …)`,
  `RUN_LEVEL3=1`; the recipe refuses unattended without `BISCUIT_L3_TAKE_FOCUS=1`.
- **Waiting** — bounded polling of the final DOM value, accessibility window,
  or captured pixel classification (25–50 ms cadence, 2–5 s deadlines); no
  fixed readiness sleep remains.
- **Route** — `just test-l3` (lib with both features); never CI.
- **Disposition** — satisfactory after Phase 8; compile-checked with both
  terminal/browser features. Runtime evidence remains **pending** because
  `just test-l3` correctly refused unattended execution without
  `BISCUIT_L3_TAKE_FOCUS=1` before opening or focusing a window.

#### `lib-browser` — 43 identities after Phase 7

- **Members** — `browser_render::browser_*`.
- **Purpose** — computed styles, DOM state and geometry of darkmatter HTML in
  headless Chrome.
- **Shared setup** — `biscuit_browser_harness::ChromeHarness`, `#[serial(browser)]`
  (intra-binary; the recipe's `-j 1` is the real serialization), one `CARGO_MANIFEST_DIR`
  fixture read.
- **External inputs** — Chrome/Chromium; skips when absent unless
  `BISCUIT_BROWSER_REQUIRED=1`. Headless, no focus.
- **Timing** — 29.36 s summed (browser log), max 1.84 s.
- **Route** — `just test-browser`; CI browser cell on `ubuntu-latest` (40.6 s).
- **Disposition** — satisfactory. Phase 7 added the real-Mermaid sanitizer
  identity to this route, with its external CLI availability recorded through
  the shared Level-2 gate.

#### `lib-browser-binary-unprefixed` — 1 baseline identity

- **Member** — now
  `browser_render::browser_sanitized_real_mermaid_retains_diagram_geometry`.
- **Purpose** — a real Mermaid render survives the SVG sanitizer with drawable
  geometry and no active markup.
- **Baseline route defect** — no `browser_` prefix inside the `browser-tests` binary:
  `test-browser` filters it out; L1 never compiles it; runs only on CI L1.
  **And** it returns early with a "skipping" message when no Mermaid toolchain
  produces an `<svg>`, so a runner without the toolchain reports a pass that
  proved nothing.
- **Disposition** — satisfactory after Phase 7. The `browser_` prefix gives it
  the canonical browser route. `require_level!` records an explicit skip when
  `mmdc` is unavailable; an installed but broken toolchain now fails the SVG
  assertion instead of passing through an early return.

### `darkmatter-cli`

#### `cli-unit` — 137 identities

- **Members** — the `darkmatter-cli` lib suite: `commands::schema` 32,
  `args::cli` 29, `args::parsers` 28, `commands::compose` 14,
  `commands::code_block` 8, `approval::tests` 7, `render::tests` 7,
  `args::completion` 6 (2 more are `cfg(windows)`), `style_claims::tests` 6.
- **Purpose** — argument parsing, style-claim capture, approval policy,
  render flag lowering.
- **Timing** — 2.83 s summed (21 ms mean).
- **Disposition** — satisfactory.

#### `cli-fixture-contract` — 17 identities

- **Members** — `md_process_fixture::*`.
- **Purpose** — prove the shared command builder pins CWD/home/PATH, scrubs
  hostile inherited state, keeps assert-command and live-child surfaces in
  parity, and rejects checkout-contained ambient workspaces.
- **Route** — ordinary L1 through `darkmatter-cli::md_process_fixture`.
- **Disposition** — satisfactory after Phases 4–6; all 17 passed in the Phase
  10 consolidated gate.

#### `cli-spawn-guard` — 18 identities

- **Members** — `spawn_site_guard::*`.
- **Purpose** — prove every deterministic `md` spawn uses the shared fixture,
  every escape is inventoried, stale inventory entries fail, and the emitted
  census replaces rather than appends prior state.
- **Route** — ordinary L1 through `darkmatter-cli::spawn_site_guard`.
- **Disposition** — satisfactory after Phase 5; all 18 passed in the Phase 10
  consolidated gate.

#### `cli-l1-private-helpers` — 116 identities

- **Members** — `schema_detect` (5), `schema_validate` (36), `compose_schema`
  (21), `compose_schema_file_rewrite` (3), `schema_about` (14), `code_block`
  (30), `schema_triggers` (7) — the seven ★ files, each with a private
  `fn md_cmd() { assert_cmd::Command::cargo_bin("md").unwrap() }`.
- **Purpose** — `md schema detect|validate|about`, schema-driven compose and
  file rewrite, `md code-block`, trigger matching through the real binary.
- **Assertion quality** — good (stdout/stderr predicates, exit codes, rewritten
  files); a handful assert only success.
- **External inputs** — fixture-owned CWD/home/config/cache/temp/PATH/Git and
  rendering inputs; schema/repository topologies live under the fixture.
  Shipped fixtures still enter through `CARGO_MANIFEST_DIR` where their bytes
  are the end-to-end behavior under test.
- **Timing** — summed 42.35 s / 116 (365 ms mean), max 2.96 s.
- **Disposition** — satisfactory after Phase 6A; all seven private helpers were
  deleted and 118/118 focused tests passed through the shared fixture builder.

#### `cli-l1-inline-spawn-baseline` — 2 identities

- **Members** — `schema_validate_baseline::*`.
- **Purpose** — byte-identical `md schema validate` output against committed
  `expected.json`/`expected.pretty` snapshots for the legacy schema cases.
- **External inputs** — fixture-owned command context with each committed case
  copied byte-for-byte into fixture topology; the `{DOC_URL}` placeholder
  normalizes the fixture path.
- **Timing** — 0.52 s summed.
- **Disposition** — satisfactory after Phase 6A; both orphan spawns migrated.

#### `cli-l1-clean-hash-frontmatter` — 183 identities

- **Members** — `clean` (29), `clean_frontmatter` (31), `clean_json` (20),
  `clean_schema` (17), `hash` (10), `hash_directory` (15),
  `hash_kind_save_diff` (22), `get_set_rm` (31), `rm` (8).
- **Purpose** — `md clean`, `md hash` (kinds, save, diff, directories) and
  `md frontmatter get|set|rm` through the real binary, including the
  read/write/read round trips for persisted hashes and frontmatter.
- **External inputs** — per-test `CliProcessFixture`; fixture-owned directories;
  `clean_frontmatter.rs` reads the `features/` directory via
  `CARGO_MANIFEST_DIR`; `hash_kind_save_diff` sets 7 env values.
- **Timing** — summed 14.65 s / 183 (80 ms mean), max 0.67 s.
- **Disposition** — satisfactory after Phase 6B; 202/202 focused family tests
  passed and persisted hash/frontmatter read/write/read checks remain intact.

#### `cli-l1-compose-render-graph` — 191 identities

- **Members** — `compose_base_schema` (9), `compose_basic` (8),
  `compose_interpolation` (7), `compose_layout` (2), `compose_page_blocks`
  (2), `compose_refs_and_missing` (5), `compose_state_set` (26),
  `compose_transclusion` (7), `delta` (16), `graph` (14), `help` (8),
  `render_basic` (17), `toc` (3), `validate_refs` (12), `layout_flags` (29),
  `layout_style_frontmatter` (8), `layout_fill` (18; 4 spawn, the rest use
  `common::layout` in-process).
- **Purpose** — `md compose` (basic, interpolation, `--set`, page blocks,
  transclusion, base schema), `md read`/render flags, `md graph`,
  `md validate refs`, `md toc`, `md delta`, `--help`/completions.
- **External inputs** — per-test `CliProcessFixture`; `graph.rs` and
  `validate_refs.rs` compare against `baseline::` JSON fixtures from the
  checkout; `compose_transclusion.rs` sets one env value; `compose_basic`,
  `compose_page_blocks`, `compose_transclusion`, `compose_state_set` set
  `current_dir` for some tests.
- **Timing** — **summed 82.87 s / 191 (434 ms mean) — the largest CLI family**;
  max 2.54 s.
- **Disposition** — satisfactory after Phase 6C. Relative-reference and
  repository-discovery tests build disposable fixture topology and retain
  relative authored references (RB2); 194/194 focused tests passed.

#### `cli-l1-layout-inprocess` — 10 identities

- **Members** — `layout_alignment::*`.
- **Purpose** — CLI layout flags resolve to the expected `DarkmatterPage`
  alignment policy, in-process via `common::layout::resolved_page`.
- **External inputs** — none (no spawn).
- **Timing** — 0.23 s summed.
- **Disposition** — satisfactory.

#### `cli-l1-compose-shell` — 8 identities

- **Members** — `compose_shell::*`.
- **Purpose** — allow/deny lists, unapproved-command guidance, timeouts,
  discovered-command reporting without execution.
- **External inputs** — a **real shell**; one test already isolates `PATH` so
  a missing executable does not probe WSL interop.
- **Timing** — 7.71 s summed (963 ms mean), max 1.95 s (the timeout tests
  carry a floor).
- **Disposition** — satisfactory after Phase 6C through documented
  `host_path()` escapes for real shell behavior and `fake_only_path()` for
  command-absence behavior.

#### `cli-l1-compose-remote-http` — 12 identities after Phase 8

- **Members** — `compose_remote_caching::*`, including bounded no-request
  teardown and a repeated TTL cache read.
- **Purpose** — `--allow-host` consent, deny-all default, cache refresh
  (`request_count() == 2`), stale-on-failure, invalid freshness flag, rendered
  remote links never fetched.
- **Assertion quality** — request counts are work-count proofs; fetched cases
  also assert GET path and loopback Host content.
- **Shared setup** — `mock_http_server` (10 servers), `--cache-root` temp dirs.
- **External inputs** — loopback only; fixture-owned command context.
- **Timing** — **summed 30.72 s / 10 (3.07 s mean)**, max 6.48 s
  (`test_compose_remote_refresh_revalidates_cached_url`), 6.40 s
  (`…fallback_serves_stale_cache_on_failure`) — the third and fourth slowest L1
  identities in the area.
- **Ownership** — bounded nonblocking accept/read, explicit shutdown, joined
  worker on drop; no public network.
- **Disposition** — satisfactory after Phase 8. Targeted nextest/leak-sweep:
  12/12 passed with no surviving process; Phase 6C spawn migration remains intact.

#### `cli-l1-compose-diagnostics` — 4 identities

- **Members** — `compose_perf::{emits_report_to_stderr, without_perf_no_report}`,
  `compose_terminal_detection::{compose_verbose_perf_performs_single_terminal_detection, compose_redirected_does_not_spawn_appearance_defaults (macOS)}`
  (+1 `cfg(not(macos))` twin off-host).
- **Purpose** — `--perf` report on stderr; exactly one terminal detection per
  compose; no `defaults` spawn when redirected.
- **External inputs** — explicit terminal dimensions/capabilities and a
  fixture-owned `PATH` with a logging `defaults` shim.
- **Timing** — 6.88 s summed, max 2.46 s.
- **Disposition** — satisfactory after Phase 6C; the shim lives in the fixture
  `bin` and the real-tool case uses the documented host-path escape.

#### `cli-l2-render-capture` — 66 identities

- **Members** — `level2_code_block_styling` (10), `level2_disclosure_blocks`
  (4), `level2_errors` (8), `level2_frontmatter_images` (11),
  `level2_frontmatter_tables` (8), `level2_horizontal_rules` (6),
  `level2_layout_dimensions` (8), `level2_ordered_lists` (10),
  `level2_schema_validate` (1).
- **Purpose** — `md` output painted in a real pane: code-block themes,
  disclosure blocks, error blocks, frontmatter tables/images, rules, list
  markers, layout dimensions, schema-validate rendering.
- **Shared setup** — `common/level2.rs`: `md_shim()` (never the host `md`),
  `SHARED_HARNESS` (WezTerm) and `SHARED_TMUX_HARNESS`
  (`level2_code_block_styling` for deterministic `COLORFGBG`),
  `#[serial(level2_terminal)]` throughout.
- **External inputs** — WezTerm socket and/or tmux; `example-docs/rendering/style-prop.md`
  in `level2_frontmatter_tables`.
- **Waiting** — 50 ms completion-sentinel polls, followed by the shared
  `capture_settled()` final-state condition (prompt visible and two identical
  frames); all waits have deadlines and the three fixed 250 ms sleeps are gone.
- **Timing** — **94.31 s summed / 66 (1.43 s mean)** — 80 % of the L2 tier
  (L2 log); max 2.81 s.
- **Route** — `just test-l2`; CI L2 cells on `ubuntu-latest` and `macos-latest`
  with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` (WezTerm-only tests skip there).
- **Disposition** — satisfactory after Phase 8. Canonical `just test-l2`
  passed 69/69 CLI tests on this host without nextest LEAK results.

#### `cli-l2-tmux-schema-about` — 3 identities

- **Members** — `level2_schema_about::*`.
- **Purpose** — the `md schema about` table row reaches a real terminal as a
  painted row.
- **Shared setup** — its own detached tmux session via `Command::new("tmux")`
  (not the shared harness); resolves the binary from `CARGO_BIN_EXE_md`
  **falling back to a bare `"md"`** (`:79`) — exactly the stale-host-binary
  hazard `level2_harness_integrity` exists to prevent.
- **Timing** — 4.04 s summed (L2 log).
- **Disposition** — **remediation in this fix (Phase 8)**: remove the bare
  fallback (fail loudly) and reuse `md_bin()`.

#### `cli-l2-harness-integrity` — 4 identities

- **Members** — `level2_harness_integrity::{md_shim_resolves_to_cargo_built_binary, assert_shim_resolves_to_built_accepts_valid_link, assert_shim_resolves_to_built_rejects_foreign_link, md_shim_path_is_absolute_temp_dir_link}`.
- **Purpose** — the L2 harness cannot silently pass against a stale host `md`
  (symlink/hard-link/copy shim resolves to the cargo-built binary).
- **Route defect** — no `level2_` prefix inside a `terminal-tests` binary:
  never runs locally under any recipe; runs only on CI L1 (all four legs,
  where the CLI builds with `terminal-tests`). Never executed on this host.
- **Disposition** — **remediation in this fix (Phase 5)**: these are the
  spawn-site guard's ancestors and must have a local route — either an L1
  binary without the feature gate or a prefix that the L2 recipe selects.

### `dmls`

#### `dmls-unit` — 511 identities

- **Members** — the `dmls` lib suite: `providers` 170, `overlay` 111, `graph`
  59, `source_map` 42, `wiki` 30, `workspace` 28, `diagnostics` 21,
  `capabilities` 17, `config` 14, `router` 10, `corpus` 5, `bench` 4 (2 more
  are `cfg(windows)`).
- **Purpose** — semantic-token providers, document overlay, workspace graph,
  wiki resolution, configuration, request routing, the bench corpus generator.
- **Shared setup** — `tempfile` in 8 files; no `set_var`, no `#[serial]`, no
  sleeps, no child processes.
- **Timing** — 17.54 s summed / 511 (34 ms mean), max 0.62 s.
- **Disposition** — satisfactory.

#### `dmls-bin-args` — 6 identities

- **Members** — `dmls::bin/dmls::tests::test_parse_*`.
- **Purpose** — the binary's hand-rolled argv parser (`--bench-index`,
  `--gen-corpus`, `--version`, unknown flags).
- **Timing** — 0.12 s summed.
- **Disposition** — satisfactory.

#### `dmls-l1-lsp-session` — 107 identities

- **Members** — `lsp_session` (88), `suggest_constraint_phase1` (19).
- **Purpose** — full LSP conversations (initialize, didOpen, diagnostics,
  completion, hover, semantic tokens, refresh, configuration) over
  `Connection::memory()`; `suggest()` diagnostics and completion.
- **Shared setup** — private `ClientFixture` per binary (server thread outcome
  joined via `mpsc`); per-test `tempfile::tempdir()` workspaces (84 sites in
  `lsp_session`, 18 in `suggest_constraint_phase1`); `include_str!` fixtures.
- **External inputs** — none (in-memory protocol; temp workspaces).
- **Waiting** — synchronizes on protocol responses; no sleeps.
- **Timing** — 8.64 s summed / 107 (81 ms mean), max 0.56 s.
- **Disposition** — satisfactory; the three fixture copies are a Phase 8
  consolidation candidate.

#### `dmls-l1-passive-proof` — 1 identity

- **Member** — `no_side_effects::*`.
- **Purpose** — every read-side request over a document dense with `::shell`,
  `$()`, `predict_conflicts` and remote URLs analyzes them while executing
  **nothing**: a sentinel a shell directive would create never appears, no
  merge is simulated, remote constructs resolve instantly.
- **Assertion quality** — strong; sentinel-based absence proof, the model RB3
  asks for. Phase 7 also asserts zero `EffectEngine` builds and zero network
  attempts around the complete in-memory LSP session under the standard
  `effects-instrumentation` feature.
- **Timing** — 0.32 s.
- **Disposition** — satisfactory.

#### `dmls-l1-workspace-fixtures` — 15 identities

- **Members** — `level1_graph_index` (4), `level1_wiki` (11).
- **Purpose** — discover → index → graph over real temp workspaces; wiki-link
  resolution over an in-memory fixture with fixed absolute paths (so macOS,
  Windows and Linux compute identically).
- **Timing** — 0.53 s summed.
- **Disposition** — satisfactory.

#### `dmls-l1-stdio-subprocess` — 3 identities after Phase 8

- **Members** — the real stdio handshake, the unwind/reap regression, and its
  subprocess-only completion probe.
- **Purpose** — the compiled `dmls` binary speaks LSP over real OS pipes
  through initialize → shutdown → exit.
- **Shared setup** — absolute `bin_exe!("dmls")`, per-test CWD/home/XDG tree,
  piped protocol I/O, RAII child guard, and `wait-timeout` reaping.
- **Timing** — 0.08 s.
- **Disposition** — satisfactory after Phase 8: successful exit is asserted;
  timeout and unwind kill/reap the child. All three passed under nextest and
  the survivor sweep found none.

#### `dmls-l1-packaging-contracts` — 2 identities

- **Members** — `packaging_contract` (1), `zed_extension_contract` (1).
- **Purpose** — `just dist` archive names match the Zed extension's
  `asset_name`; `extension.toml` declares `id = "dmls"` and Markdown.
- **External inputs** — reads `justfile`, `zed-dmls/src`, `zed-dmls/extension.toml`
  from the checkout (passive).
- **Timing** — 0.05 s summed.
- **Disposition** — satisfactory: the in-tree half of the WASM extension's
  route (the other half is `just zed-verify` on CI).

#### `dmls-l2-neovim` — 3 identities

- **Members** — `level2_editor_neovim::*`.
- **Purpose** — Neovim's real LSP client decodes `dmls` semantic tokens onto
  the intended columns, classifies families, excludes fenced code, and
  repaints on `didChangeConfiguration`; in tmux mode the styling recipe
  produces visible SGR.
- **Shared setup** — `nvim --clean --headless -l probe.lua` against
  `bin_exe!("dmls")`; per-test workspace/home/cache/config/data roots; a tmux
  pane for the styling mode; fixtures under `tests/fixtures/editor_neovim/`.
- **Gate** — `require_level!(Level::L2, nvim_available(), "nvim")` — a string
  label, so it contributes no backend-execution evidence; CI provisions
  `neovim` via `runner-tools`.
- **Waiting** — 50 ms capture polling of each final asserted SGR condition,
  bounded by a 30 s deadline; the Lua client synchronizes on LSP attachment,
  tokens, refresh, clear, and repaint conditions.
- **Timing** — 2.51 s summed (L2 log), max 1.63 s.
- **Disposition** — satisfactory after Phase 8. Canonical `just test-l2`
  passed 3/3; the tmux mode names `Backend::Tmux` and recorded execution.

### `zed-dmls-cli`

#### `zed-cli-unit` — 20 identities

- **Members** — the `zed-dmls-cli` lib suite (2 more are `cfg(unix)`, 1
  `cfg(windows)`).
- **Purpose** — staging allowlist and repeatability, registration repair,
  doctor diagnostics, platform data-root discovery, bounded log tail.
- **Shared setup** — `tempfile`; one module shells out (`Command::new` in
  `lib.rs`) for the compiled-version check.
- **Timing** — 0.45 s summed.
- **Disposition** — satisfactory.

#### `zed-cli-bin-args` — 1 identity

- **Member** — `zed-dmls-cli::bin/zed-dmls::tests::parses_global_overrides_before_or_after_subcommand`.
- **Timing** — 0.02 s.
- **Disposition** — satisfactory.

#### `zed-cli-raw-spawn` — 6 identities

- **Members** — `cli::{missing_command_uses_clap_exit_two, doctor_is_hermetic_with_path_overrides_and_plain_output, conditional_doctor_is_silent_when_zed_is_absent, stage_returns_three_when_manual_registration_remains, stage_registers_and_returns_zero_when_zed_is_present, conditional_stage_is_silent_when_zed_is_absent}`.
- **Purpose** — exit codes, `--plain` output, staging and registration through
  the real `zed-dmls` binary.
- **Shared setup** — `ZedDmlsFixture` owns one `TempDir` per test and launches
  all six commands with fixture CWD/home/config/cache/temp, empty `PATH`, and
  scrubbed Git/rendering/application inputs. Every state-touching test passes
  explicit fixture-owned `--staging-dir` / `--zed-data-dir` / `--zed-log`.
  The fixture lives in this package because the Phase 4 `md` fixture is an
  integration-test module in another package and cannot be imported here.
- **Timing** — 1.14 s summed.
- **Disposition** — **satisfactory after Phase 6D migration**. The original six
  exit/output/filesystem assertions pass through the real shipped binary; the
  stage tests additionally assert the staged artifact and registration link.

## Dispositions at a glance

| Disposition | Families | Identities |
|---|---:|---:|
| satisfactory | 45 | 4,968 |
| remediation in this fix | 17 | 2,884 |
| follow-up | 1 | 1 |

Remediation still owned by: Phase 5 (1 family, 4 identities:
`cli-l2-harness-integrity`), Phase 7 (8, 2,134 — the
ambient-capture audit of `lib-unit-compose-core`, `lib-unit-compose-context`,
`lib-unit-compose-cleanup-rendering` and `lib-compose-e2e`, plus the four
route/marker fixes), and Phase 8 (8, 746 — waits, child and server ownership, the two
L2 hazards). Plus the comment-only cleanup of the three stale `#[ignore]` doc
comments. No row was dispositioned by a timing threshold; every fast family
was reviewed for accidental effects, and the two that have them
(`lib-effects-integration`, `dmls-l1-stdio-subprocess`) are remediation rows
despite costing under a second.
