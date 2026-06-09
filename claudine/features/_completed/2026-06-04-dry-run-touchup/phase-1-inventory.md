# Phase 1 Inventory — `--dry-run` Agent Resolution Touch-up

## Agent Frontmatter Parsing (fatal invalid values today)

- **`claudine/lib/src/composition/prepare.rs:281-315`** (`parse_agent_hint`)
  - Single string path (lines 283-286): unknown provider → `CompositionError::AgentHintInvalid`
  - Array path (lines 288-308): unknown list entry → same hard error at line 293-294
  - Callers: `prepare_direct` (109-113), `prepare_inline` (189-193), `parse_selection_hints_from_frontmatter` (270)

## Provider Selection & Dry-Run Seam

- **Picker location**: `claudine/cli/src/commands/wrap/composition/mod.rs:734` (`prompt_one_shot_provider`)
- **Dry-run seam**: `mod.rs:1455-1479` — returns AFTER provider selection (lines 671-794)
- **Problem**: under `--dry-run`, the picker fires before the seam is reached; must short-circuit

## Installed Provider Representation

- `InstalledProviderSnapshot` in `claudine/lib/src/composition/types.rs:238-256`
  - `runnable`: installed + not excluded
  - `all_installed`: every installed provider
  - `binary_paths`: resolved binaries
- Validity vs installation: `Provider::fuzzy_match_cli_name` = known catalog; snapshot membership = installed

## TTY / Prompt Channel

- Current TTY gate (`mod.rs:700`): `stdin().is_terminal() && stdout().is_terminal()`
- `choose_one` (`biscuit-tui standalone`): backend writes to **stdout**, alternate screen
- Styled messages / dry-run table: routed to **stderr** via `crate::log::message`
- **Required change**: agent re-prompt gate must key off the prompting channel; spec says stderr

## Table Component Newline Confirmation

- **Confirmed**: `biscuit-terminal` `Table` preserves embedded newlines inside a single cell
- Evidence: `table.rs` doc example (line 114), `calculate_row_heights_for_plan` (line 854), `render_content` multi-line row rendering (line 912+)
- Two-column alignment and the `1ch` left margin (`table_utils.rs:7`) survive multi-line cells

## Call Sites That Will Change

| File | Lines | What |
|------|-------|------|
| `claudine/lib/src/composition/prepare.rs` | 281-315 | `parse_agent_hint` — make invalid single + list entries non-fatal |
| `claudine/lib/src/composition/types.rs` | — | Add `AgentResolutionState` enum |
| `claudine/cli/src/commands/wrap/composition/dry_run.rs` | 30-185 | Replace `Option<Provider>` with `AgentResolutionState`; render all variants |
| `claudine/cli/src/commands/wrap/composition/mod.rs` | 671-794, 1455 | Move dry-run seam before provider selection; implement TTY-only live gate |
| `claudine/lib/src/composition/select.rs` | — | Add pure classification helpers |

## Test Files That Will Cover Changes

| File | Level | Coverage |
|------|-------|----------|
| `claudine/cli/src/commands/wrap/composition/dry_run.rs` (unit) | L1 | Renderer for every `AgentResolutionState` variant |
| `claudine/cli/tests/wrap_commands.rs` | L1 | Live-path TTY/no-TTY behavior, `--silent` invariance |
| `claudine/cli/tests/level2_dry_run_metadata_capture.rs` | L2 | Red/yellow/dim SGR assertions, multi-line cell alignment |
| `claudine/lib/src/composition/prepare.rs` (unit) | L1 | Non-fatal invalid agent parsing |
| `claudine/lib/src/composition/select.rs` (unit) | L1 | Classification helper correctness |
