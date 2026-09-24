---
kind: spike
feature: 2026-09-22-consolidated-test-binaries-wave-2
spike: S2
created: 2026-09-23
rev: 208051f753aa9ce0fcc19622f64464d11d3570d2
scanner: spikes/s2-scan.py
raw_output: spikes/s2-scan-output.txt
---

# S2 — Crate-global, path, and identity hazard scan (wave 2)

## Summary

This scan is read-only. It covers all 144 `.rs` files under the ten packages'
`tests/` trees: 136 former target files and 8 helper files (fixture data is
excluded). It uses `consolidation.py`'s own detector tables and parsers
(`CRATE_GLOBAL_DETECTORS`, `IDENTITY_DETECTORS`, `PATH_DETECTORS`,
`CRATE_PATH`, `leading_inner_attributes`, `module_declarations`), plus a
repo-wide `git grep` for consumers that name these tests by path or target.

**No package has a crate-global construct.** Across all ten there are zero
`#[macro_export]`, `#[global_allocator]`, `#[no_mangle]`, `#[export_name]`,
`#[link_section]`, `#[used]`, `#[link(...)]`, `ctor`/`dtor`, `#[panic_handler]`,
`extern "C" fn`, and `#[macro_use]`. There are also zero crate-root-only inner
attributes. No finding forces a separate target.

The scan found:

- 13 inner `#![cfg]` in target files, all legal in a module and all moving to
  the `mod` declaration;
- 1 inner lint attribute in a target file;
- 95 path-sensitive sites needing repair: 52 `include_str!`/`include_bytes!`,
  12 `#[path]` (3 of them spell `common/mod.rs`), 1 bare `mod fixtures;`, and
  30 bare `mod common;`;
- 2 self-exec `--exact` identity strings (claudine-gen and dmls);
- 2 nextest exact-name overrides (3 profile entries);
- 3 genuine `crate::` paths (biscuit-tui-cli) and 3 `crate::` false positives
  inside string literals (schematic-gen ×2, claudine-gen ×1), which will trip
  `check-attributes` unless dispositioned;
- 19 Insta snapshot files in 2 packages;
- 1 proptest regression file;
- 2 path-keyed guards that need updating: sniff-cli's `spawn_site_guard`, and
  a cross-package assertion in `tools/test-toolkit`;
- 2 recipes that name a `--test` target (4 lines), plus doc and comment drift.

**Three items need a ruling:**

- **H-P1:** the proptest regression-file location. Wave 1's darkmatter move
  is silently broken.
- **H-G2:** the test-toolkit guard reads `windows_captured_stdout.rs` by path
  and requires its inner `#![cfg(windows)]`.
- **H-R1:** the `--test <former target>` recipe lines.

`claudine`'s `boundary_lint` is **not** a path-keyed guard over test files. It
reads production sources relative to the crate directory, so no change is
needed. The duplicate crate-root names are harmless in every package, because
each former file becomes its own module namespace.

Nothing at all was found in these categories: `biscuit-file`, `claudine`,
`sniff`, and `tree-hugger` have no identity-string hazards;
`tree-hugger`, `claudine`, `sniff`, `biscuit-file`, `schematic-gen`,
`claudine-gen`, and `dmls` have no snapshots; and only `biscuit-file` has
proptest persistence files.

## Cross-cutting findings

### H-P1 — proptest regression files do not live "beside the module" (needs ruling)

Proptest 1.11.0 (the locked version) persists failures with the default
`FileFailurePersistence::SourceParallel("proptest-regressions")`
(`proptest-1.11.0/src/test_runner/failure_persistence/file.rs:326-380`). It
walks up from `file!()` to the **nearest ancestor containing `lib.rs` or
`main.rs`**:

- **If no such ancestor is found**, it falls back to
  `<source>.proptest-regressions` beside the file. That is today's layout,
  because `tests/` has no `main.rs`.
- **If one is found**, it uses
  `<parent of that dir>/proptest-regressions/<relative path>.txt`.

After a file moves to `tests/l1/yaml_mutation.rs`, `tests/l1/main.rs`
exists. So proptest reads and writes
**`tests/proptest-regressions/yaml_mutation.txt`**, not
`tests/l1/yaml_mutation.proptest-regressions`.

- **biscuit-file:** `tests/yaml_mutation.proptest-regressions` has 2 `cc`
  seeds. The planned disposition is a byte-identical rename to
  `biscuit-file/lib/tests/proptest-regressions/yaml_mutation.txt`.
- **claudine:** `tests/typed_stream_protocols.rs` uses `proptest!` but has no
  committed regressions file. There is nothing to move. Future failures will
  persist to `claudine/lib/tests/proptest-regressions/typed_stream_protocols.txt`.
- **Wave-1 defect (out of scope here; file it):** commit `ba9697fb3` moved
  darkmatter's two files to
  `darkmatter/lib/tests/l1/{schema_quoting_safety,schemas_grammar_proptest}.proptest-regressions`
  (5 seeds). The implementation log says "Proptest resolves them relative to
  the source file". By the code above, those seeds are no longer replayed.
  The correct location is
  `darkmatter/lib/tests/proptest-regressions/<name>.txt`.

**Ruling needed:** adopt the `tests/proptest-regressions/<name>.txt` rule
(derived from the source above, not measured). Add it to the mechanical-move
rules beside snapshots, and file the darkmatter fix as a separate defect.
Proving the rule takes one scratch-crate run with a deliberately failing
property. That run is allowed locally and is not a CI cell.

### H-C1 — `crate::` detector false positives

`check-attributes` fails any former file whose comment-stripped text matches
`crate::`. String literals are **not** stripped, so these three sites need a
manifest disposition (`crate_path`: "inside a string literal naming generated
code; not a path"):

- `schematic/gen/tests/e2e_generation.rs:214` (`"use crate::shared::{…}"`)
- `schematic/gen/tests/http_client.rs:471` (`"crate::shared::RequestBody::Json(json)"`)
- `claudine/gen/tests/pipeline.rs:166` (`"pub(in crate::provider) static …"`)

### H-O1 — nextest exact-name overrides (spec hazard 5, confirmed)

| Filter | Profiles | Test | New filter |
|---|---|---|---|
| `test(=test_detect_completes_in_reasonable_time)` | `ci` (`.config/nextest.toml:229`) | `sniff/lib/tests/integration.rs:73` | `test(=integration::test_detect_completes_in_reasonable_time)` |
| `test(=level2_render_tree_style_in_wezterm)` | `default` (:142), `ci` (:264) | `biscuit-terminal/cli/tests/level2_render_tree_style.rs:1064` | `test(=level2_render_tree_style::level2_render_tree_style_in_wezterm)` |

The package-level filters (`package(claudine) & !test(/(^|::)level2_/)…` →
`claudine-l1`, and `package(sniff) + package(sniff-cli)` →
`sniff-windows-l1`) are unaffected. No `claudine` lib stem begins with a tier
marker.

### H-R1 — recipes and docs that name a former `--test` target (needs ruling on scope)

These **executable** lines break when their target disappears:

- `schematic/justfile:271` `test-e2e`:
  `cargo test -p schematic-gen --test e2e_generation -- --ignored`. It must
  become `--test l1 -- --ignored e2e_generation::`. Without the filter it
  would run every ignored test in `l1`.
- `biscuit-tui/justfile:127-129` `test-pty`:
  - `--test keyboard_protocol` becomes `--test l1 keyboard_protocol::`;
  - `--test completions_shell` becomes `--test l1 completions_shell::`;
  - `--test choose_cli pty::` becomes `--test l1 choose_cli::pty::`.
- `schematic/justfile:149` (`cargo test -p schematic-gen artifact_drift -- --ignored`)
  still works, because the substring filter matches `artifact_drift::…`.

The following are doc and comment drift, updated in the same package commit
per the `CLAUDE.md` drift rules:

- `sniff/lib/tests/remote_providers.rs:11`
- `schematic/gen/tests/artifact_drift.rs:11`
- `schematic/gen/tests/postman_golden.rs:18`
- `.claude/skills/schematic-define/SKILL.md:173-175`
- `.claude/skills/biscuit-test-harness/SKILL.md:166,174`
- `biscuit-test-harness/README.md:391,400`
- `biscuit-tui/cli/README.md:367-378`
- `biscuit-tui/cli/tests/choose_cli.rs:782`
- `biscuit-tui/cli/tests/completions_shell.rs:12`

Path mentions in comments also drift:

- `sniff/lib/benches/support/bench_ids.rs:7,11,74`
- `claudine/gen/Cargo.toml:67`
- `darkmatter/dmls/Cargo.toml:72`
- `biscuit-tui/cli/Cargo.toml:45`
- `darkmatter/dmls/README.md:25,147-148`
- `darkmatter/dmls/docs/…` (smoke-checklist, features, hover)
- `tree-hugger/lib/README.md:144,154`
- `biscuit-test-harness/src/bin/broker.rs:187`
- `renderable/docs/layout-and-style.md:405`
- `.claude/skills/os/windows.md:219`
- `.claude/skills/rust-testing/SKILL.md:140`
- `docs/comment-quality.md:293`
- `biscuit-tui/lib/README.md:201`

Historical `reviews/_completed`, `docs/plans`, and `.ai/plans` mentions are
records and stay as they are. `.github/ci/ci-baseline.toml:74-79` names an
in-scope package only in a commented example. No live skip entry exists for
any of the ten packages.

### H-G2 — cross-package guard reads a moved test by path (needs ruling)

`tools/test-toolkit/tests/ci_workflow_contracts.rs:7053`
(`the_windows_captured_stdout_test_is_discoverable_as_ordinary_l1`) reads
`biscuit-tui/cli/tests/windows_captured_stdout.rs`. It asserts
`test.contains("#![cfg(windows)]")`.

After the move:

- the path must become `biscuit-tui/cli/tests/level2/windows_captured_stdout.rs`
  (per F5/R4);
- the migration must **keep** the inner `#![cfg(windows)]` in the module file
  as well as copying it to `#[cfg(windows)] mod windows_captured_stdout;`.
  `check-attributes` accepts that as `inner-cfg-duplicated` (review only).
  Otherwise the assertion text must change to look at the declaration.

This edit lands in `test-toolkit`, a package outside the ten. That schedules
`test-toolkit` in the biscuit-tui-cli commit. It still counts as a
structural consumer edit.

### Other consumers checked and found safe

- **`claudine/cli/tests/l1/test_placement.rs`** scans `lib/tests` and
  `gen/tests` (`TEST_ROOTS`, :38-45) for focus-stealing harness APIs. It
  classifies files by **file name** (`is_level3_file`, :460), and no
  `claudine`/`claudine-gen` file is `level3_*`, so it is unaffected. Its layout
  gate (:505 onward) is rooted at the CLI crate only (`cli_root`). Extending it
  to `claudine/lib` and `claudine/gen`, as plan Wave 2 anticipates, is a
  separate edit.
- **`tools/test-audit/fixtures/claudine-compat/families.json`** and
  `junit/nextest-l1-excerpt.xml` are frozen fixtures of the test-audit tool,
  not live identity stores.
- **`current_exe()` + `--list`** has zero sites in all ten packages. Every
  `--list` hit is a CLI flag under test:
  - `sniff/cli/tests/cli.rs` ×33;
  - `biscuit-terminal/cli/tests/integration_test.rs:2478`;
  - `biscuit-tui/cli/tests/{choose_one_output,choose_many_output}.rs:13`;
  - `biscuit-tui/cli/tests/completions_shell.rs:191,506`.
- **`file!()`, `module_path!()`, `CARGO_CRATE_NAME`, `NEXTEST_TEST_NAME`,
  `NEXTEST_BINARY_ID`, and `CARGO_BIN_NAME`** have zero sites.
- **`fn main` textual hits** are 6, all inside raw-string Rust fixtures:
  `tree-hugger/lib/tests/lint_diagnostics.rs:273,360,782,1584,1641` and
  `phase1_diagnostics.rs:77`. They are safe.
- **Fixture reads through `manifest_dir!()`/`CARGO_MANIFEST_DIR`** (43 sites
  in 37 files) resolve from the crate directory, not the source file, so they
  are unaffected. Every relative `"tests/…"` and `"../…"` string literal not
  inside `include_*!`/`#[path]` was checked. Each is either a `manifest_dir`
  join or reference-grammar test data (biscuit-file, `lsp_session.rs`'s
  `"./schema.yaml"`), so all are safe.
- **`#[serial]`** (serial_test) is used in 22 files. Under nextest's
  process-per-test model it is unchanged. Under a single-process `cargo test`
  of a consolidated binary, its key scope widens to the whole binary. That
  means more exclusion, never less, so it is safe.
- **Per-file statics** (e.g. `SHARED_WEZTERM`/`SHARED_KITTY`/`SHARED_TMUX` in
  10 biscuit-terminal-cli files) become 10 module-scoped statics. They are
  per process, which is the wave-1 §7 boundary: safe under nextest.

## Per-package hazards

Dispositions use wave 1's vocabulary: **safe**, **structural edit**, **needs
ruling**. "Root" means the consolidated `tests/<target>/main.rs`. Per R2, each
root that needs `common` declares `#[path = "../common/mod.rs"] mod common;`
once, and the files' own `mod common;` becomes `use crate::common;`.

### tree-hugger (10 files → `l1`)

| Hazard | Where | Disposition |
|---|---|---|
| `include_str!("../queries/…")` ×32 | `query_compile.rs:84-336` | structural edit: becomes `"../../queries/…"` |
| `include_str!("../queries/…")` ×9 | `phase6_neovim_query_reuse.rs:396,419,423,427,431,435,454,458,462` | structural edit: becomes `"../../queries/…"` |
| duplicate `fn create_temp_file` | `cache_tests.rs:9`, `lint_diagnostics.rs:8`, `phase1_diagnostics.rs:8`, `tree_file.rs:131` | safe (separate modules) |
| `fn main` textual | `lint_diagnostics.rs` ×5, `phase1_diagnostics.rs:77` | safe (raw-string fixtures) |

There are no inner attributes, no identity strings, no snapshots, and no
guards.

### claudine (15 files → `l1`)

| Hazard | Where | Disposition |
|---|---|---|
| `#[path = "../../../darkmatter/features/2026-09-22-lifecycle-events/spike/model.rs"] mod model;` | `lifecycle_control_flow_spike.rs:2-3` | structural edit: one more `../` (`"../../../../darkmatter/…"`). Note: the path spells an **active feature directory**. When that spec moves to `_completed`, the test breaks (pre-existing; flag to the author) |
| `include_str!("fixtures/logs/opencode-subagent-lifecycle.txt")` | `opencode_stderr_lifecycle.rs:33` | structural edit: becomes `"../fixtures/logs/…"` |
| inner `#![allow(deprecated)]` (below the module doc) | `deprecated_compatibility.rs:14` | safe: stays inner in the module file (wave-1 precedent) |
| `boundary_lint` guard | `boundary_lint.rs` | **safe, not path-keyed on tests**: it reads `src/…`, `../cli/src/…`, and `../../darkmatter/lib/src/…` via `manifest_dir!()`. No path list changes |
| proptest without a regressions file | `typed_stream_protocols.rs` | see H-P1 (nothing to move) |
| duplicate `fn fixture_path`, `struct Recording` | `kimi_wire.rs`/`protocol_fixture_replay.rs`/`semantic_fidelity.rs` | safe |
| scanned by `claudine-cli`'s `test_placement` | `TEST_ROOTS` `lib/tests` | safe (file-name keyed) |

There are no identity strings and no snapshots. The two `*_spike` stems carry
no tier marker.

### sniff (20 files → `l1`; `fixtures.rs` is a helper, not a module)

| Hazard | Where | Disposition |
|---|---|---|
| inner `#![cfg(feature = "remote")]` | `focused_provider.rs:1`, `remote_providers.rs:14` (below the doc) | structural edit: copy to `#[cfg(feature = "remote")] mod …;` (legal to keep inner too) |
| inner `#![cfg(feature = "network")]` | `remote_observation.rs:1` | same |
| inner `#![cfg(target_os = "windows")]` | `windows_app_paths_orphan.rs:7`, `windows_find_program_priority.rs:6` (below the doc) | same. Must be shown run on Windows (plan Wave 3) |
| `#[path = "../benches/support/X.rs"]` ×7 | `bench_fixtures.rs:10`, `bench_ids_sync.rs:11`, `bench_plans.rs:6`, `benchmark_workloads.rs:3,5`, `git_parity.rs:11` | structural edit: becomes `"../../benches/support/X.rs"`. `builder.rs` is included by 3 modules. Each keeps its own copy, as today |
| bare `mod fixtures;` → `tests/fixtures.rs` | `integration.rs:9` | structural edit. `tests/fixtures.rs` is today **both** an autotest target (0 tests) **and** `integration`'s child. Under `autotests = false` it is a helper used by one file. Per R2 it either moves beside it (`tests/l1/integration/fixtures.rs`, bare `mod fixtures;` unchanged) or stays and gets `#[path = "../fixtures.rs"]`. **Do not** declare it as a root module: the former `fixtures` binary lists 0 tests, so dropping it loses no identity. The data dir `tests/fixtures/remote/` has no `mod.rs`, so there is no E0761 clash |
| nextest `test(=test_detect_completes_in_reasonable_time)` | H-O1 | structural edit (R10) |
| `macro_rules! roundtrip` | `program_serialization.rs:23` (inside a fn body) | safe: function-scoped, with no textual-order dependency |
| `"tests/basic.rs"` literal | `bench_fixtures.rs:70` | safe (synthetic repo file list) |
| duplicate `mod builder` ×3, `mod fixtures` ×2 (different files: `benches/support/fixtures.rs` vs `tests/fixtures.rs`), `fn repository`, `struct Fixture` | — | safe: each is scoped to its own module |
| comment drift | `benches/support/bench_ids.rs:7,11,74`; `remote_providers.rs:11` (`--test remote_providers`) | update in the same commit |

There are no identity strings and no snapshots.

### biscuit-file (15 files → `l1`, `l1-fetch`)

| Hazard | Where | Disposition |
|---|---|---|
| proptest regressions `tests/yaml_mutation.proptest-regressions` (2 seeds) | `yaml_mutation.rs` | **needs ruling (H-P1)**: move byte-identical to `tests/proptest-regressions/yaml_mutation.txt` |
| `#![proptest_config(…)]` | `yaml_mutation.rs:204` | safe (inside `proptest!`, not a file attribute) |
| `tests/corpus/yaml_corpus.json` | `yaml_corpus.rs:67-71` | safe (`manifest_dir` join) |
| duplicate helpers (`canonical`, `git_init`, `ctx_with_repo`, …) | 7 files | safe |

There are no inner attributes, path repairs, identity strings, snapshots, or
guards. `fetch_integration.rs` carries no inner `cfg`; its contract is
manifest-only (`required-features = ["fetch"]`).

### schematic-gen (14 files → `l1`, `level2`)

| Hazard | Where | Disposition |
|---|---|---|
| `include_bytes!("fixtures/postman/v2.1.0-collection.schema.json")` | `postman_artifact_validation.rs:20`, `postman_golden.rs:40`, `postman_schema.rs:28` | structural edit: becomes `"../fixtures/postman/…"` |
| golden files (`postman_golden`, `artifact_drift`) | `postman_golden.rs:67-72`, `artifact_drift.rs:35` | safe (`manifest_dir` join, so no path repair). Good test-input probe candidates |
| `crate::` inside string literals | `e2e_generation.rs:214`, `http_client.rs:471` | disposition needed (H-C1) |
| `just test-e2e` `--test e2e_generation` | `schematic/justfile:271` | structural edit (H-R1) |
| doc `--test artifact_drift` / `--test postman_golden` | `artifact_drift.rs:11`, `postman_golden.rs:18`, schematic-define skill | drift update |
| duplicate `POSTMAN_SCHEMA_BYTES`, `postman_validator`, `format_tokens`, … | 3–5 files | safe |

There are no inner attributes. `terminal_capture` has no inner `cfg`; its
contract is manifest-only. There are no identity strings and no snapshots.

### biscuit-terminal-cli (14 files + `common/` → `l1`, `level2`)

| Hazard | Where | Disposition |
|---|---|---|
| `mod common;` ×9 | `level2_{apple_terminal_prose:35, container_fenced_code:16, cursor_and_hygiene:14, diagrams:37, image:27, layout:20, prose_styling:13, status_block:16, style_everywhere_matrix:30}` | structural edit: the `level2` root declares `common` once (all 9 users are `level2`), and the files `use crate::common;`. `common/mod.rs`'s `pub mod pane_geometry;` keeps resolving in `tests/common/` |
| Insta ×7 (auto-named) | `integration_test.rs:1482,1498,2130,2148,2163,2177,2191` | structural edit: `tests/snapshots/integration_test__<name>.snap` → `tests/l1/snapshots/l1__integration_test__<name>.snap` (7 files: columns, list, padleft, padright, prose, prose_styled, quote), byte-identical, via `check-snapshots`. There are no Insta settings overrides |
| nextest `test(=level2_render_tree_style_in_wezterm)` | H-O1 (both profiles) | structural edit (R10) |
| `#![allow(dead_code)]` | `common/mod.rs:16`, `common/pane_geometry.rs:8` | safe (common does not move) |
| `manifest_dir.join("../README.md")` etc. | `integration_test.rs:2206-2209` | safe (crate-dir relative) |
| 10 `SHARED_*` statics, `BORDER_GLYPH`, `capture_bt`, … | level2 files | safe (module-scoped, per process) |
| doc `--test level2_prose_styling` | biscuit-test-harness skill and README | drift update |

There are no crate-global constructs and no identity strings. F3's aliases
(`prose_cells`, `diagrams`) are a tier matter covered by the projection, not
by this scan.

### claudine-gen (11 files → `l1`, `level2`)

| Hazard | Where | Disposition |
|---|---|---|
| **self-exec `--exact level2_report_probe`** | `level2_report_terminal.rs:160` (code), `:16` (module doc) | **structural edit, R9 identity repair**: becomes `--exact level2_report_terminal::level2_report_probe`. The doc line is updated in the same edit. A missed repair makes the probe print nothing, so the assertions fail loudly when tmux is present but skip quietly without it. Prove it on a tmux host |
| inner `#![cfg(unix)]` (below the long doc) | `level2_report_terminal.rs:45` | structural edit: `#[cfg(unix)] mod level2_report_terminal;` |
| `crate::` inside a string literal | `pipeline.rs:166` | disposition needed (H-C1) |
| repository reads (`drift`, `fixtures_provenance`, `signals_sidecar_mirror`, `steering_check`) | `drift.rs:38`, `steering_check.rs:17`, … | safe (`manifest_dir` joins). Test-input probe candidates |
| duplicate `fn area`, `fn real_area`, `struct Fixture`, `fn new` | — | safe (`real_area` is a helper fn, not a test, so no `real_` tier effect) |
| comment drift | `claudine/gen/Cargo.toml:67` | update |

There are no include/`#[path]` repairs and no snapshots.

### dmls (11 files + `common/` → `l1`, `level2`)

| Hazard | Where | Disposition |
|---|---|---|
| **self-exec `["--exact", "child_guard_cancellation_probe"]`** | `stdio_subprocess.rs:84-85` | **structural edit, R9 identity repair**: the arg becomes `"stdio_subprocess::child_guard_cancellation_probe"`. A missed repair fails loudly ("cancellation probe did not start") |
| `include_str!("fixtures/suggest_constraint/…")` ×7 | `suggest_constraint_phase1.rs:12-18` | structural edit: becomes `"../fixtures/suggest_constraint/…"` |
| `mod common;` ×4 | `lsp_session.rs:8`, `no_side_effects.rs:13`, `strict_mode_recovery_spike.rs:4`, `suggest_constraint_phase1.rs:7` | structural edit: the `l1` root declares `common` once |
| `#![allow(dead_code)]` | `common/mod.rs:29` | safe |
| `../justfile`, `../docs/…` | `packaging_contract.rs:33`, `mapping_only_corpus.rs:62`, `lsp_session.rs:5463,5495,6029` | safe (`manifest_dir` joins) |
| comment drift | `Cargo.toml:72`, `README.md:25,147-148`, `docs/editors/smoke-checklist.md`, `docs/features.md:38`, `docs/hover.md:113`, `src/overlay/expressions.rs:1566` | update (README links `tests/no_side_effects.rs` directly) |

There are no inner `cfg` attributes. `level2_editor_neovim` is manifest-gated
only. There are no snapshots.

### sniff-cli (11 files + `common/` → `l1`, `level2`; rulings R14 folds `level2-fixtures` into `level2`)

| Hazard | Where | Disposition |
|---|---|---|
| **`spawn_site_guard` self-exclusion is path-keyed** | `spawn_site_guard.rs:44` `relative == "spawn_site_guard.rs"` | **structural edit (path key)**: after the move the relative path is `l1/spawn_site_guard.rs`. Without an update the guard scans itself. The key becomes `"l1/spawn_site_guard.rs"` (wave-1 precedent: claudine-cli asserts on `"l1/spawn_site_guard.rs"`). The `common/` prefix and the `level2_`/`real_` **file-name** exclusions keep working. New `*/main.rs` roots get scanned but contain no spawns. The `Site.file` values in the burn-down report gain a `<target>/` prefix; `SPAWN_ALLOWLIST` is empty. Record the scan diff in `guard-scans-after.md` |
| `#[path = "common/source_scan.rs"] mod source_scan;` | `spawn_site_guard.rs:4` | structural edit: becomes `"../common/source_scan.rs"` (only user) |
| `mod common;` ×10 (`tty.rs:20` carries `#[cfg(unix)]`) | cli, cli_process_fixture, install_interview_cli, install_plan, level2_* ×4, snapshots, tty | structural edit: each of the three roots declares `common`. `tty`'s `use crate::common;` keeps `#[cfg(unix)]` |
| inner `#![cfg(feature = "test-fixtures")]` | `level2_cicd_styling.rs:18`, `level2_git_status_styling.rs:23`, `level2_perf_tree_rendering.rs:45`, **`level2_recent_commits_rendering.rs:22`** | structural edit: copy to the declarations. **Note for spec hazard 3:** `level2_recent_commits_rendering` has no manifest `required-features`, but its source is `#![cfg(feature = "test-fixtures")]`. Without the feature, its target compiles to zero tests. Keeping it in a feature-less `level2` target with the module `cfg` preserves that exactly. Merging it into `level2-fixtures` would also change no identity. The spec's separation is conservative, not identity-forced (inform R-ruling; no change proposed) |
| Insta ×12 (named) | `snapshots.rs:153-849` | structural edit: `tests/snapshots/snapshots__<name>.snap` → `tests/l1/snapshots/l1__snapshots__<name>.snap` (12 files), byte-identical. There are no settings overrides. The module file `tests/l1/snapshots.rs` coexists with the directory `tests/l1/snapshots/`, which is legal while no `snapshots/mod.rs` exists |
| `#![allow(dead_code)]` | `common/mod.rs:10` | safe |
| duplicate `RENDER_DEADLINE`, `PANE_ROWS`, … | level2 files | safe |

There are no self-exec identity strings (33 `--list` hits are CLI flags).
Package-level `sniff-windows-l1` is unaffected.

### biscuit-tui-cli (15 files + `common/` → `l1`, `level2`, `level3`)

| Hazard | Where | Disposition |
|---|---|---|
| `mod common;` ×7 and `#[path = "common/mod.rs"] mod common;` ×3 | `boolean_switch_output`, `choose_many_output`, `choose_one_output`, `exit_codes`, `input_table_output`, `text_area_input_output`, `text_input_output` (bare); `choose_cli.rs:29`, `keyboard_protocol.rs:21`, `real_terminal_render.rs:57` (`#[path]`) | structural edit: the `l1` and `level2` roots declare `common` (`level3_chord_select` does not use it). `common/mod.rs`'s `#[cfg(unix)] pub mod pty;` and `#[cfg(all(unix, feature = "terminal-tests"))] pub mod real_terminal;` keep resolving in `tests/common/` |
| **`crate::common::pty::answer_cursor_position_request`** | `choose_cli.rs:831`, `keyboard_protocol.rs:81,171` | structural edit plus a `crate_path` disposition: today `crate` is the file itself. After the move `crate::common` resolves to the root's `common`, which is equivalent because `pty` is `pub`. The files' own `mod common;` must be removed (not kept alongside), or `crate::common` and the local `common` become two copies |
| inner `#![cfg(target_os = "macos")]` | `level3_chord_select.rs:63` (below the doc) | structural edit: `#[cfg(target_os = "macos")] mod level3_chord_select;` |
| inner `#![cfg(unix)]` | `real_terminal_render.rs:55` | structural edit: `#[cfg(unix)] mod terminal_render;` (F4 alias) |
| inner `#![cfg(windows)]` | `windows_captured_stdout.rs:72` | structural edit: `#[cfg(windows)] mod windows_captured_stdout;` in `level2`. **Keep the inner attribute too** (H-G2) |
| inner `#![cfg(unix)]`, `#![allow(dead_code)]` | `common/pty.rs:17,21`, `common/mod.rs:8`, `common/real_terminal/mod.rs:7` | safe (common does not move) |
| test-toolkit guard reads `tests/windows_captured_stdout.rs` | `tools/test-toolkit/tests/ci_workflow_contracts.rs:7053` | **needs ruling (H-G2)** |
| `just test-pty` `--test keyboard_protocol` / `completions_shell` / `choose_cli pty::` | `biscuit-tui/justfile:127-129` | structural edit (H-R1) |
| doc `--test …` | `cli/README.md:367-378`, `choose_cli.rs:782`, `completions_shell.rs:12` | drift update |
| duplicate `question_binary`, `skip_if_not_enabled`, `mod skip_unix_only`, `QUESTION_RENDER_MS` | — | safe |

There are no self-exec identity strings (the `--list` hits are CLI flags) and
no snapshots.

## Method

`python3 features/2026-09-22-consolidated-test-binaries-wave-2/spikes/s2-scan.py`
(`--json` gives the machine form) imports `scripts/ci/consolidation.py` and
runs:

- its `CRATE_GLOBAL_DETECTORS`, `IDENTITY_DETECTORS`, `PATH_DETECTORS`,
  `CRATE_PATH`, `CURRENT_EXE`+`LIST_ARG`, `leading_inner_attributes`, and
  `module_declarations`;
- wave-1's extra detectors (`link_attr`, `macro_use`, `macro_rules`,
  `fn_main`, Insta assertions and settings);
- wave-2 additions: `manifest_dir` anchoring, `"tests/…"` and `"../…"`
  string literals, top-level item names per target file, and any
  non-leading `#![`.

Every hit was then read in context. The consumer search was a `git grep`
over `scripts/`, `.github/`, `.config/`, `tools/`, all justfiles, skills,
and docs for each package's target stems and `--test <stem>`. Proptest's
persistence rule was read from the locked `proptest-1.11.0` source.
