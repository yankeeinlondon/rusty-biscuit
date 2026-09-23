---
kind: evidence
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 1
generator: baseline/inventory.py
data: baseline/inventory.json
---

# Baseline test-target inventory (before any migration)

Generated from `cargo metadata --no-deps`, each package `Cargo.toml`, and
the macOS before-listings in `baseline/listings/`. `inventory.json` is the
complete record and the future migration manifest's `before` side. This
page summarizes it.

## Workspace context

| Measure | Value |
|---|---:|
| Workspace members (`cargo metadata`) | 74 |
| Members with ≥1 integration-test target | 51 |
| Integration-test targets | 523 |

## Per package

| Package | Top-level `tests/*.rs` | Test targets | Nested `main.rs` roots | Declared `[[test]]` | `mod common;` files | Harness ≠ std | Target-wide keys beyond `required-features` | `[[bench]]` (harness=false) |
|---|---:|---:|---|---:|---:|---:|---|---:|
| `biscuit-terminal` | 38 | 38 | — | 1 | 7 | 0 | none | 2 |
| `darkmatter` | 73 | 74 | `darkmatter/lib/tests/error_snapshots/main.rs` | 4 | 0 | 0 | none | 16 |
| `claudine-cli` | 139 | 139 | — | 37 | 129 | 0 | none | 0 |
| `darkmatter-cli` | 53 | 53 | — | 11 | 51 | 0 | none | 0 |

## Execution contracts (required features × harness × other target-wide keys)

### `biscuit-terminal`

| Contract | Targets | Name markers present |
|---|---:|---|
| `features=- harness=std other-keys=-` | 37 | `(none)`×37 |
| `features=terminal-tests harness=std other-keys=-` | 1 | `level2_`×1 |

### `darkmatter`

| Contract | Targets | Name markers present |
|---|---:|---|
| `features=- harness=std other-keys=-` | 70 | `(none)`×70 |
| `features=browser-tests harness=std other-keys=-` | 2 | `browser_`×1, `level3_`×1 |
| `features=terminal-tests harness=std other-keys=-` | 2 | `level2_`×1, `level3_`×1 |

### `claudine-cli`

| Contract | Targets | Name markers present |
|---|---:|---|
| `features=- harness=std other-keys=-` | 102 | `(none)`×102 |
| `features=real-tests harness=std other-keys=-` | 3 | `real_`×3 |
| `features=terminal-tests harness=std other-keys=-` | 34 | `level2_`×29, `level3_`×5 |

### `darkmatter-cli`

| Contract | Targets | Name markers present |
|---|---:|---|
| `features=- harness=std other-keys=-` | 42 | `(none)`×42 |
| `features=terminal-tests harness=std other-keys=-` | 11 | `level2_`×11 |

## Tier placement today (macOS listing, widest captured feature set per target)

Counts of tests selected by each canonical tier expression. A target
that appears in more than one tier column mixes tiers inside one file;
consolidation keeps that because selection is per test.

- **`biscuit-terminal`**: mixed-tier targets: `progress_parity` (L1:57, browser:4), `prose_cells_parity` (L1:55, browser:3), `section_parity` (L1:48, browser:3), `status_block_parity` (L1:72, browser:1), `text_block_parity` (L1:52, browser:14), `todo_parity` (L1:55, browser:7), `two_column_parity` (L1:71, browser:6). Targets never built by a captured feature set: none.
- **`darkmatter`**: mixed-tier targets: `schema_phase_validation` (L1:18, real:1). Targets never built by a captured feature set: none.
- **`claudine-cli`**: mixed-tier targets: none. Targets never built by a captured feature set: none.
- **`darkmatter-cli`**: mixed-tier targets: none. Targets never built by a captured feature set: none.

## Feature-gated targets: tiers their tests select into

A consolidated target is keyed by `required-features`; tier selection
stays per test. A gated target whose tests select into a tier its
name does not announce keeps that placement under an alias (R2).

| Package | Target | Required features | Selected per tier (feature set that builds it) |
|---|---|---|---|
| `biscuit-terminal` | `level2_terminal_osc_wezterm` | `terminal-tests` | L2:2 |
| `darkmatter` | `browser_render` | `browser-tests` | browser:43 |
| `darkmatter` | `level2_render_tree_terminal` | `terminal-tests` | L2:18 |
| `darkmatter` | `level3_image_painting` | `terminal-tests` | L3:1 |
| `darkmatter` | `level3_popover` | `browser-tests` | L3:3 |
| `claudine-cli` | `level2_auto_complete_chooser` | `terminal-tests` | L2:6 |
| `claudine-cli` | `level2_auto_complete_operation_file` | `terminal-tests` | L2:19 |
| `claudine-cli` | `level2_context_capture` | `terminal-tests` | L2:19 |
| `claudine-cli` | `level2_dry_run_approval_capture` | `terminal-tests` | L2:1 |
| `claudine-cli` | `level2_dry_run_metadata_capture` | `terminal-tests` | L2:7 |
| `claudine-cli` | `level2_explicit_operation_file_miss` | `terminal-tests` | L2:2 |
| `claudine-cli` | `level2_file_resolution_capture` | `terminal-tests` | L2:7 |
| `claudine-cli` | `level2_incomplete_subagents_capture` | `terminal-tests` | L2:1 |
| `claudine-cli` | `level2_initialize_generated_transclusion` | `terminal-tests` | L2:4 |
| `claudine-cli` | `level2_inline_compose_mismatch_capture` | `terminal-tests` | L2:2 |
| `claudine-cli` | `level2_interrupt_feedback_capture` | `terminal-tests` | L2:1 |
| `claudine-cli` | `level2_invalid_file_reference_capture` | `terminal-tests` | L2:4 |
| `claudine-cli` | `level2_lifecycle_action_forms` | `terminal-tests` | L2:3 |
| `claudine-cli` | `level2_lifecycle_control` | `terminal-tests` | L2:97 |
| `claudine-cli` | `level2_lifecycle_dispatch` | `terminal-tests` | L2:11 |
| `claudine-cli` | `level2_lifecycle_loop` | `terminal-tests` | L2:4 |
| `claudine-cli` | `level2_malformed_frontmatter_capture` | `terminal-tests` | L2:1 |
| `claudine-cli` | `level2_perf_capture` | `terminal-tests` | L2:3 |
| `claudine-cli` | `level2_prompt_reporting_capture` | `terminal-tests` | L2:4 |
| `claudine-cli` | `level2_provided_partial_file_capture` | `terminal-tests` | L2:2 |
| `claudine-cli` | `level2_provider_overlay_capture` | `terminal-tests` | L2:1 |
| `claudine-cli` | `level2_removed_validation_key_capture` | `terminal-tests` | L2:2 |
| `claudine-cli` | `level2_schema_parse_capture` | `terminal-tests` | L2:1 |
| `claudine-cli` | `level2_sequence_task_stream_capture` | `terminal-tests` | L2:10 |
| `claudine-cli` | `level2_stalled_generation_capture` | `terminal-tests` | L2:2 |
| `claudine-cli` | `level2_typed_error_render_capture` | `terminal-tests` | L2:14 |
| `claudine-cli` | `level2_windows_provided_partial_file_capture` | `terminal-tests` | none selected on macOS (tests cfg-absent here, or all ignored) |
| `claudine-cli` | `level2_wrap_ctrl_c_loop_wedge_tmux` | `terminal-tests` | L2:1 |
| `claudine-cli` | `level2_wrap_ctrl_c_tmux` | `terminal-tests` | L2:2 |
| `claudine-cli` | `level3_auto_complete_chooser` | `terminal-tests` | L3:1 |
| `claudine-cli` | `level3_linux_sequence_ctrl_c` | `terminal-tests` | none selected on macOS (tests cfg-absent here, or all ignored) |
| `claudine-cli` | `level3_sequence_ctrl_c` | `terminal-tests` | L3:1 |
| `claudine-cli` | `level3_windows_sequence_ctrl_c` | `terminal-tests` | none selected on macOS (tests cfg-absent here, or all ignored) |
| `claudine-cli` | `level3_wrap_ctrl_c` | `terminal-tests` | L3:2 |
| `claudine-cli` | `real_inline_write_grant` | `real-tests` | real:2 |
| `claudine-cli` | `real_opencode_yolo_subagent` | `real-tests` | real:1 |
| `claudine-cli` | `real_pi_steering` | `real-tests` | none selected on macOS (tests cfg-absent here, or all ignored) |
| `darkmatter-cli` | `level2_code_block_styling` | `terminal-tests` | L2:10 |
| `darkmatter-cli` | `level2_disclosure_blocks` | `terminal-tests` | L2:4 |
| `darkmatter-cli` | `level2_errors` | `terminal-tests` | L2:8 |
| `darkmatter-cli` | `level2_frontmatter_images` | `terminal-tests` | L2:11 |
| `darkmatter-cli` | `level2_frontmatter_tables` | `terminal-tests` | L2:8 |
| `darkmatter-cli` | `level2_harness_integrity` | `terminal-tests` | L1:4 |
| `darkmatter-cli` | `level2_horizontal_rules` | `terminal-tests` | L2:6 |
| `darkmatter-cli` | `level2_layout_dimensions` | `terminal-tests` | L2:8 |
| `darkmatter-cli` | `level2_ordered_lists` | `terminal-tests` | L2:10 |
| `darkmatter-cli` | `level2_schema_about` | `terminal-tests` | L2:3 |
| `darkmatter-cli` | `level2_schema_validate` | `terminal-tests` | L2:1 |

## Module-name hazard (R2 neutral-alias rule, S1 §2)

A target whose **name** carries a tier marker keeps its tests' tiers as a
module name only if every one of its tests already carries that marker
in its own path. Targets whose name has no marker are always safe.

| Package | Marker-named targets | Alias required (a test lacks the marker) | Undetermined |
|---|---:|---|---:|
| `biscuit-terminal` | 1 | none | 0 |
| `darkmatter` | 4 | none | 0 |
| `claudine-cli` | 37 | none | 0 (source-scan only: `level2_windows_provided_partial_file_capture`, `level3_linux_sequence_ctrl_c`, `level3_windows_sequence_ctrl_c`) |
| `darkmatter-cli` | 11 | `level2_harness_integrity` | 0 |

## Inner `#![cfg]` crate conditions (must become module-declaration conditions)

- **`biscuit-terminal`**: `cfg(feature = "image")`×3; `cfg(unix)`×3
- **`darkmatter`**: `cfg(target_os = "macos")`×2; `cfg(windows)`×1
- **`claudine-cli`**: `cfg(target_os = "linux")`×1; `cfg(target_os = "macos")`×2; `cfg(unix)`×76; `cfg(windows)`×3
- **`darkmatter-cli`**: none
