---
kind: spike-record
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 1
status: complete
scanner: spikes/s2-scan.py
raw_output: spikes/s2-scan-output.txt
---

# S2 — Crate-global and consolidation-sensitive construct scan

This is the spec §3 pre-migration scan over every `.rs` file under the four
packages' `tests/` trees (claudine-cli 148, darkmatter 100, darkmatter-cli
58, biscuit-terminal 42). Run it from the repository root with
`python3 features/2026-09-21-consolidated-test-binaries/spikes/s2-scan.py`.
`--json` gives the machine form. The complete hit list is in
`s2-scan-output.txt`. This page gives each hit class a **disposition**:

- **safe** — it survives modularization unchanged.
- **structural edit** — it needs one of the §3 permitted edits.
- **forces a separate target** — it cannot live in a shared crate.
- **blocks migration** — it needs a behavior change, which is a separate
  defect.

The detector list at the top of `s2-scan.py` is the seed for Phase 2's
`check-attributes`. Findings from S1 §7 and §9 (module-file resolution) are
folded in.

## Crate-global constructs (the ones that could force separate targets)

| Construct | Hits in the four trees | Disposition |
|---|---|---|
| `#[macro_export]` | 0 | — |
| `#[global_allocator]` | 0 | — |
| `#[no_mangle]` / `#[export_name]` / `#[link_section]` / `#[used]` | 0 | — |
| `#[ctor]`/`#[dtor]`-style startup constructors, `#[panic_handler]` | 0 | — |
| `extern "C" fn` definitions (exported symbols) | 0 | — |
| `#[link(...)]` | 1: `claudine/cli/tests/sequence_ctrl_c_windows.rs:82` | **safe**: an `extern` *import* block (Windows console API), not an export. Two modules importing the same symbol cannot collide at link time. |
| crate-root-only inner attributes (`recursion_limit`, `no_std`, `no_main`, `feature`, `windows_subsystem`, `type_length_limit`, `crate_type`, `crate_name`, `test_runner`, …) | 0 | — (S1 §7: they would degrade to a warning, so `check-attributes` must keep detecting them) |
| `fn main` | 4 textual hits: `level2_provider_overlay_capture.rs:323`, `sequence_budget.rs:39`, `sequence_cli.rs:731`, `wrap_ctrl_c_windows.rs:56` (claudine-cli) | **safe**: all four are inside raw-string fixture programs (`r#"…"#`, `r##"…"##`) that the tests compile or write out. None is a crate entry point. |
| `#[macro_use]` | 0 | — |
| `macro_rules!` | `claudine/cli/tests/wrap_compose_agent.rs:300`; `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs:442,459,483` | **safe**: textually scoped to their own module and used only there. None is `#[macro_export]`, so none can collide at the new crate root. |
| `crate::` paths inside test files | 0 | — (a moved file's `crate::` would silently re-point to the consolidated root; there are none today) |

**Conclusion: no hit forces a separate target, and none blocks
migration.** R3's expected outcome, "nothing beyond `harness = false`",
holds at the source level. The inventory confirms it at the manifest level
(`baseline/inventory.md`: zero non-standard harnesses and no target-wide keys
beyond `required-features`). The spec's remaining concern is link-time
collisions that no source scan can see. Those are covered by compiling the
consolidated targets on all three OSes (Phase 3 Wave 3), not by this scan.

## Inner `#![cfg]` conditions: structural edit (move to the module declaration)

| Package | Conditions (files) |
|---|---|
| claudine-cli | `#![cfg(unix)]` ×76, `#![cfg(windows)]` ×3, `#![cfg(target_os = "macos")]` ×2, `#![cfg(target_os = "linux")]` ×1 |
| darkmatter | `#![cfg(target_os = "macos")]` ×2 (`level3_image_painting.rs`, `level3_popover.rs`), `#![cfg(windows)]` ×1 (`declined_path_transclusion.rs`) |
| darkmatter-cli | none |
| biscuit-terminal | `#![cfg(unix)]` ×3, `#![cfg(feature = "image")]` ×3 |

S1 §7 proved that an inner `#![cfg]` still works inside a module file.
Moving it to `#[cfg(...)] mod x;` in `main.rs` is required by acceptance 4,
so reviewers see each module's reachability in one place. It is not required
for correctness. Three claudine files carry the `cfg` below a long module doc
(`level2_windows_provided_partial_file_capture.rs:50`,
`level3_windows_sequence_ctrl_c.rs:87`, `sequence_ctrl_c_windows.rs:61`).
`check-attributes` must find inner attributes anywhere in the leading
attribute/doc block, not only on line 1.

biscuit-terminal's `#![cfg(feature = "image")]` is a **feature** condition on
a module, not a `required-features` entry. It stays a module condition
inside the L1 binary, which keeps today's behavior: the tests are
compiled out unless `image` is on. It must not be promoted to a separate
target (that would add a contract the manifest never declared).

Other inner attributes are lint levels (`#![allow(dead_code)]`,
`#![allow(deprecated)]`), which are **safe**: they are module-scoped in a
module file. Darkmatter's three `#![proptest_config(...)]` hits sit inside
`proptest!` blocks and are not file attributes.

## Module-resolution hazards: structural edit (path repair)

A file that stops being a crate root changes how its relative paths resolve.
Both rules below were measured on the S1 scratch crate:

- **Bare `mod x;`** resolves beside a crate root (`tests/x.rs`). From a
  non-root module file `tests/l1/f.rs`, it resolves under
  **`tests/l1/f/x.rs`**.
- **`#[path = "…"]` and `include_str!`/`include_bytes!`** resolve relative to
  the directory of the file that contains them, so they break when the file
  moves one directory deeper.
- `#[path = "../common/mod.rs"] mod common;` from `tests/l1/main.rs` loads
  `common/mod.rs` as a mod-rs file, so `common`'s own children (`pub mod
  child;`) keep resolving inside `tests/common/`. This was verified, and it
  means `common/` does not move.

| Package | Files needing a path repair when moved |
|---|---|
| claudine-cli | `#[path = "common/…"] mod …;` in `error_guards.rs` (+ its `error_guards/source_scan.rs` subtree), `spawn_site_guard.rs`, `system_prompt_perf_bench.rs`, `test_placement.rs`; relative `include_str!` in `compose_caller_file_provenance.rs` (10 sites), `sequence_schema.rs`, `shipped_prompt_contract.rs` (2), `wrap_compose_validation.rs` (3); 129 `mod common;` declarations become `crate::common` imports |
| darkmatter | `#[path]` in `image_pixel_classification.rs`, `level2_render_tree_terminal.rs` (7 submodules), `level3_image_painting.rs`; bare `mod` in `layout_matrix.rs`, `render_comparison.rs`; the nested root `error_snapshots/main.rs` (17 `mod` children, which keep resolving if it becomes `error_snapshots/mod.rs`); relative `include_str!` in `as_block_error_registry.rs`, `inline_document_text.rs` (4), `predict_conflicts.rs`, `schema_phase_validation.rs`, `suggest_constraint_phase1.rs` (2) |
| darkmatter-cli | `#[path = "common/…"]` ×2 in `spawn_site_guard.rs`; 51 `mod common;` declarations |
| biscuit-terminal | bare `mod <x>_support;`/`mod common;` in 21 top-level files (`*_parity.rs`, `*_matrix.rs`, `render_comparison.rs`, …) that point at sibling helper directories; 7 `mod common;` declarations |

Fixture reads through `biscuit_test_harness::manifest_dir!()` /
`CARGO_MANIFEST_DIR` are relative to the **crate directory**, not the source
file, so they are unaffected. No test uses `file!()`, `module_path!()`, or
`CARGO_CRATE_NAME` directly.

## Identity-sensitive runtime behavior (new; not in the plan's detector list)

These constructs compile fine and cannot be seen by `cfg` or attribute
checks. They change behavior because the **test path or the binary** changes:

| Site | What it depends on | Disposition |
|---|---|---|
| `darkmatter/lib/tests/level2_render_tree_terminal/support/mod.rs:432` re-runs `current_exe()` with `--exact public_entry_points::level2_render_probe_entrypoint` | the probe's **test path inside its binary**. After the move it becomes `<module>::public_entry_points::level2_render_probe_entrypoint`, so `--exact` would select nothing, and the L2 run would render nothing. | **structural edit, needs R9**: the identity string must gain the module prefix. It is an identity repair caused by the move, analogous to a path repair. The exact form is recorded in the migration manifest. |
| `darkmatter/lib/tests/interpolation_literal_pipeline.rs:9` runs `current_exe() --list` and embeds its stdout in YAML front matter | the **size and content of the whole binary's test list**. Today that is one file's tests; after consolidation it is every L1 test in the package (thousands of lines). | **safe in principle** (the expected value is computed from the same command), but a behavioral sensitivity to verify in Phase 4. If the larger list breaks shell or YAML handling, that is a halt-and-file defect under §3, never a test edit. |
| `darkmatter/lib/tests/shell_expansion_coordinates.rs:42` runs `current_exe()` with an invalid libtest option | only that the executable exists and fails | **safe** |
| claudine `claudine_bin()` (`level2_auto_complete_*`), `biscuit-terminal` `discovery_probe_path()`, `biscuit-test-harness` `bin_dir` | the `deps/` directory layout, which consolidation does not change | **safe** |
| Nextest exact-name overrides in `.config/nextest.toml`: `test(=compose_loop_rate_limit_pause_waits_then_continues)` (claudine-cli `loop_cli.rs`), `test(=every_catalog_variable_survives_ambient_options)` (darkmatter `ambient_ctx_capture.rs`), each in `default` and `ci` profiles | the **top-level test path**. After the move the `=` match no longer matches, and the extended `slow-timeout` silently stops applying. | **structural edit, needs R5 amendment**: the filter must be updated in the same package migration, and the comparator must prove it (see R5). |
| Nextest regex overrides `test(/level2_/)`, `test(/browser_/)` (unanchored) and `package(claudine) & !test(/(^|::)level2_/)…` | any module segment containing the substring | **safe today**: no in-scope target name contains `level2_`, `level3_`, `browser_`, `real_`, or `slow_` other than as a prefix (`level2_perf_capture` already starts with `level2_`; `system_prompt_perf_bench`'s `_perf_` is not matched by the anchored `perf_` filter, which applies only to `worktree-cli`). The alias rule (R2) must evaluate these override filters as well as `_tier_filter`. |

No test or harness crate derives a name from `NEXTEST_TEST_NAME`,
`NEXTEST_BINARY_ID`, `thread::current().name()`, or `CARGO_BIN_NAME`, so
temp directories, sessions, and backend-proof records are not keyed on the
changing identity.

## Process-global state (§7 boundary)

Statics such as `SHARED_HARNESS` (`OnceLock`) in darkmatter's L2 support
module are per-process. Under Nextest's process-per-test execution,
consolidation does not make two tests share them. There is no crate-global
code that runs before libtest selects a test (no constructors, no global
allocator), so the §7 "code before case selection" boundary has nothing in it.

## Method, for `check-attributes`

1. Walk every `.rs` under the package `tests/`.
2. Inner attributes: match `#![name…]` anywhere in the leading attribute or doc
   block. Classify `crate-root-only` (list in `s2-scan.py`), `cfg`, `cfg_attr`,
   and lint levels. Any `crate-root-only` attribute in a file that becomes a
   module fails.
3. Crate-global detectors: `macro_export`, `global_allocator`, `no_mangle`,
   `export_name`, `link_section`, `used`, `ctor`/`dtor`, `panic_handler`,
   exported `extern "C" fn`. Any hit fails unless dispositioned in the
   manifest.
4. For each moved file, every former inner `#![cfg(...)]` must appear, token
   for token, as an outer `#[cfg(...)]` on its `mod` declaration.
5. Path-sensitive detectors (`mod x;`, `#[path]`, `include_*!`) are reported
   with the file's old and new directory, so a reviewer can confirm each
   repair. They do not fail by themselves.
6. Identity-sensitive detectors (a string passed after `--exact`, and
   `current_exe()` followed by `--list`) are reported for manual
   disposition. The literal `--exact <path>` form is the one to match. A
   quoted `"--exact"` argument never occurs today.
