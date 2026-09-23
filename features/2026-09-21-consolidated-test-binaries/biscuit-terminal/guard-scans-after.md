---
kind: evidence
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-22
plan_phase: 6
package: biscuit-terminal
host: macOS aarch64-apple-darwin
---

# Guard after-scans — `biscuit-terminal`

`biscuit-terminal/lib/tests/` has no spawn-site or isolation guard. The one
path-sensitive guard that scans it is the repository archive-path guard
(`tools/test-toolkit/tests/archive_path_guard.rs`), compared here by path with
`../baseline/guard-scans.md` (acceptance 7). Each before path was mapped through
`../biscuit-terminal-migration.json`. No helper directory moved:
`common/`, `layout_matrix_support/`, and `inline_content_matrix_support/` stay
at `tests/`. Produced the Phase 1 way: a temporary probe test appended to the
guard's file called `test_toolkit::archive_guard::{collect_rust_files,
is_eligible}`, ran once, and the file was restored and verified clean.

| Guard | Before | After | Extra after mapping | Missing after mapping | Eligibility changed |
|---|---:|---:|---|---|---|
| archive-path guard, `biscuit-terminal/lib/tests/**` | 42 | 45 | `l1/main.rs`, `l1/test_layout.rs`, `level2/main.rs` | none | none (all `true`) |

The guard's summary line reads `mode=full-tree files-checked=3494 ineligible=0
violations=5`: Phase 5's 3,491 plus these three files. The 5 violations are the
baseline's 5 allow-listed live sites.

Path-keyed code inside the moved tests was searched before the move
(`starts_with`, `file_name`, `read_dir`, `current_exe`, relative-path string
literals). None keys on a test file's location: fixture paths go through
`manifest_dir!()`, and `level2_terminal_osc_wezterm`'s `discovery_probe_path()`
climbs from the test executable in `target/<profile>/deps/`, which the move does
not change.

## archive-path guard walked (45, repository-relative, `path<TAB>eligible`)

```text
biscuit-terminal/lib/tests/common/mod.rs	true
biscuit-terminal/lib/tests/common/pty.rs	true
biscuit-terminal/lib/tests/inline_content_matrix_support/mod.rs	true
biscuit-terminal/lib/tests/l1/compose_parity.rs	true
biscuit-terminal/lib/tests/l1/filesystem_parity.rs	true
biscuit-terminal/lib/tests/l1/graph_expression_parity.rs	true
biscuit-terminal/lib/tests/l1/horizontal_rule_parity.rs	true
biscuit-terminal/lib/tests/l1/html_page_example.rs	true
biscuit-terminal/lib/tests/l1/inline_content_matrix.rs	true
biscuit-terminal/lib/tests/l1/integration.rs	true
biscuit-terminal/lib/tests/l1/layout_matrix.rs	true
biscuit-terminal/lib/tests/l1/level1_apple_terminal_prose.rs	true
biscuit-terminal/lib/tests/l1/level1_clipboard.rs	true
biscuit-terminal/lib/tests/l1/level1_cursor.rs	true
biscuit-terminal/lib/tests/l1/level1_mode_2027.rs	true
biscuit-terminal/lib/tests/l1/level1_osc_queries.rs	true
biscuit-terminal/lib/tests/l1/level1_terminal_init.rs	true
biscuit-terminal/lib/tests/l1/level1_terminal_osc_cache.rs	true
biscuit-terminal/lib/tests/l1/list_parity.rs	true
biscuit-terminal/lib/tests/l1/main.rs	true
biscuit-terminal/lib/tests/l1/mermaid_parity.rs	true
biscuit-terminal/lib/tests/l1/metrics_tree_parity.rs	true
biscuit-terminal/lib/tests/l1/ordered_list_parity.rs	true
biscuit-terminal/lib/tests/l1/parity_helpers.rs	true
biscuit-terminal/lib/tests/l1/perf_gate.rs	true
biscuit-terminal/lib/tests/l1/prelude_exports.rs	true
biscuit-terminal/lib/tests/l1/progress_parity.rs	true
biscuit-terminal/lib/tests/l1/prose_cells_parity.rs	true
biscuit-terminal/lib/tests/l1/render_comparison.rs	true
biscuit-terminal/lib/tests/l1/render_tree_code_context.rs	true
biscuit-terminal/lib/tests/l1/render_tree_component_parity.rs	true
biscuit-terminal/lib/tests/l1/section_parity.rs	true
biscuit-terminal/lib/tests/l1/status_block_parity.rs	true
biscuit-terminal/lib/tests/l1/status_parity.rs	true
biscuit-terminal/lib/tests/l1/table_parity.rs	true
biscuit-terminal/lib/tests/l1/terminal_image_parity.rs	true
biscuit-terminal/lib/tests/l1/test_layout.rs	true
biscuit-terminal/lib/tests/l1/text_block_parity.rs	true
biscuit-terminal/lib/tests/l1/todo_parity.rs	true
biscuit-terminal/lib/tests/l1/tree_layout.rs	true
biscuit-terminal/lib/tests/l1/two_column_parity.rs	true
biscuit-terminal/lib/tests/l1/unordered_list_parity.rs	true
biscuit-terminal/lib/tests/layout_matrix_support/mod.rs	true
biscuit-terminal/lib/tests/level2/level2_terminal_osc_wezterm.rs	true
biscuit-terminal/lib/tests/level2/main.rs	true
```
