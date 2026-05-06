---
phases: 5
created: 2026-05-05
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - baseline.md
  - reproduce.sh
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-terminal/lib/src/terminal.rs
  - biscuit-terminal/lib/src/components/status.rs
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/components/mermaid.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
  - biscuit-terminal/lib/src/components/graph_expression.rs
  - biscuit-terminal/lib/src/discovery/osc_queries/mod.rs
  - biscuit-terminal/cli/src/commands/shared.rs
  - biscuit-terminal/cli/src/output.rs
  - biscuit-terminal/lib/tests/integration.rs
  - biscuit-terminal/lib/examples/terminal_info.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/output/terminal.rs
  - darkmatter/lib/src/markdown/yaml_block.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - biscuit-terminal
  - biscuit-terminal-cli
  - darkmatter
  - darkmatter-cli
---

# Execution Plan: Fix Terminal Escape Code Bleed in Non-Interactive Sessions

## Problem Summary

In non-interactive sessions (where a prompt is provided via stdin/args but stdout is connected to a TTY/PTY), terminal OSC 11
color query responses bleed into output as literal characters. The pattern `^[]11;rgb:1a1a/1b1b/2626^[\` appears before each
tool call icon and persists once it starts.

**Root cause**: `biscuit_terminal::discovery::detection::color_mode()` sends live OSC 11 queries to stdout on every
invocation. Multiple hot paths call this repeatedly:

1. `Status::to_terminal()` calls the static `Terminal::color_mode()` on every status render
2. `TerminalOptions::default()` in darkmatter triggers `detect_color_mode()` on every instantiation
3. `Terminal` struct does not cache `color_mode`
4. Non-interactive sessions with TTY-connected stdout still trigger OSC queries

## Phase 1: Diagnosis & Baseline

**Goal**: Confirm the exact code paths and establish a reproduction.

### Step 1.1: Trace exact call paths
**Observable**: Document the complete call chain from claudine output → OSC query.

- Verify `wrap_terminal()` → `crate::log::terminal()` → `Terminal::new()` path
- Verify `Status::to_terminal()` line 508 calls `Terminal::color_mode()` (static)
- Verify `TerminalOptions::default()` line 764 calls `detect_color_mode()` → `bg_color()`
- Verify `query_osc_actual()` sends `\x1b]11;?\x07` to stdout

**Files to inspect**:
- `biscuit-terminal/lib/src/discovery/osc_queries.rs:597` (query_osc_actual)
- `biscuit-terminal/lib/src/discovery/detection.rs:292` (color_mode)
- `biscuit-terminal/lib/src/components/status.rs:508` (Status::to_terminal)
- `darkmatter/lib/src/markdown/output/terminal.rs:764` (TerminalOptions::default)
- `claudine/cli/src/log.rs:50` (log::terminal)
- `claudine/cli/src/commands/wrap/mod.rs:190` (wrap_terminal)

### Step 1.2: Create reproduction script
**Observable**: A script that demonstrates the bleed in a controlled environment.

Create a minimal reproduction that:
1. Runs `claudine` in non-interactive mode with a prompt
2. Captures stdout to show OSC responses appearing as literal text
3. Can be run before/after fixes to verify resolution

**Parallelizable**: No (depends on 1.1)

### Step 1.3: Document baseline behavior
**Observable**: Written record of current behavior for comparison.

- Screenshot or text capture of bleed pattern
- Count of OSC queries issued per typical claudine session (instrument if needed)
- Note any environment factors (terminal app, TTY/PTY setup)

**Validation checkpoint**: Baseline documented and reproducible.

## Phase 2: biscuit-terminal Core Fixes

**Goal**: Eliminate repeated OSC queries by caching `color_mode` at the Terminal instance and process level.

### Step 2.1: Add `color_mode` field to `Terminal` struct
**Observable**: `Terminal` struct has a `color_mode: ColorMode` field.

**File**: `biscuit-terminal/lib/src/terminal.rs:166`

Add field:

```rust
pub color_mode: ColorMode,
```

**Dependencies**: None

### Step 2.2: Cache `color_mode` in `new_terminal()`
**Observable**: `Terminal::new()` sets `color_mode` once via `color_mode()`.

**File**: `biscuit-terminal/lib/src/terminal.rs:41`

In `new_terminal()`, add:

```rust
color_mode: color_mode(),
```

**Dependencies**: Step 2.1

### Step 2.3: Add `color_mode` to `TerminalBuilder`
**Observable**: Builder supports `.color_mode(ColorMode::Dark)`.

**File**: `biscuit-terminal/lib/src/terminal.rs:571`

1. Add `color_mode: Option<ColorMode>` to `TerminalBuilder` fields
2. Add builder method:

```rust
pub fn color_mode(mut self, value: ColorMode) -> Self {
    self.color_mode = Some(value);
    self
}
```

3. In `TerminalBuilder::build()`, use `self.color_mode.unwrap_or_else(color_mode)`

**Dependencies**: Step 2.1
**Parallelizable with**: Step 2.2

### Step 2.4: Change `Terminal::color_mode()` from static to instance method
**Observable**: `term.color_mode()` returns cached value; static method removed or deprecated.

**File**: `biscuit-terminal/lib/src/terminal.rs:492`

Change:

```rust
// FROM:
pub fn color_mode() -> ColorMode {
    color_mode()
}

// TO:
pub fn color_mode(&self) -> ColorMode {
    self.color_mode
}
```

**Dependencies**: Steps 2.1, 2.2

### Step 2.5: Fix `Status::to_terminal()` to use instance method
**Observable**: `Status::to_terminal()` calls `term.color_mode()` instead of `Terminal::color_mode()`.

**File**: `biscuit-terminal/lib/src/components/status.rs:508`

Change:

```rust
// FROM:
let tw_color = match (Terminal::color_mode(), icon_def.color_alt) {

// TO:
let tw_color = match (term.color_mode(), icon_def.color_alt) {
```

**Dependencies**: Step 2.4

### Step 2.6: Fix other static `Terminal::color_mode()` call sites
**Observable**: All call sites use instance method or cached value.

**Files to update**:
- `biscuit-terminal/lib/src/components/table/table.rs:1982,1987,2005,2010`
    - These are in `render()` methods that have `&self` or `term` available
    - Change `Terminal::color_mode()` to use the terminal instance
- `biscuit-terminal/lib/src/components/mermaid.rs:142`
    - Has access to `term` parameter; change to `term.color_mode()`
- `biscuit-terminal/lib/src/components/horizontal_rule.rs:385`
    - Has access to `term` parameter; change to `term.color_mode()`
- `biscuit-terminal/lib/src/components/graph_expression.rs:284`
    - Has access to `term` parameter; change to `term.color_mode()`

**Dependencies**: Step 2.4
**Parallelizable with**: Step 2.5 (same phase, independent files)

### Step 2.7: Add process-level cache for `bg_color()` / OSC queries
**Observable**: `bg_color()` returns cached result after first call per process.

**File**: `biscuit-terminal/lib/src/discovery/osc_queries.rs`

Add a `std::sync::OnceLock` cache:

```rust
static BG_COLOR_CACHE: std::sync::OnceLock<Option<RgbValue>> = std::sync::OnceLock::new();
```

In `bg_color()` (or `query_osc_color`), use:

```rust
BG_COLOR_CACHE.get_or_init(|| {
    // existing query logic
}).clone()
```

This ensures even if code still calls the free `color_mode()` function, the expensive OSC query only runs once per process.

**Dependencies**: None (independent improvement)
**Parallelizable with**: Steps 2.1-2.6

### Step 2.8: Run biscuit-terminal tests
**Observable**: All tests pass.

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/biscuit-terminal/lib
cargo test
```

**Dependencies**: Steps 2.1-2.7

## Phase 3: darkmatter Fixes

**Goal**: Prevent `TerminalOptions::default()` from triggering repeated color mode detection.

### Step 3.1: Cache color_mode in `TerminalOptions`
**Observable**: `TerminalOptions::default()` uses cached `color_mode` detection.

**File**: `darkmatter/lib/src/markdown/output/terminal.rs:756`

Options:
1. Add a `static COLOR_MODE_CACHE: OnceLock<ColorMode>` and use it in `default()`
2. Or accept `color_mode` as a parameter and move detection responsibility to callers

Recommended approach 1 (minimal change):

```rust
static DETECTED_COLOR_MODE: std::sync::OnceLock<ColorMode> = std::sync::OnceLock::new();

impl Default for TerminalOptions {
    fn default() -> Self {
        let color_mode = DETECTED_COLOR_MODE.get_or_init(|| detect_color_mode()).clone();
        // ... rest of default
    }
}
```

**Dependencies**: Phase 2 (for testing coordination, but technically independent)

### Step 3.2: Run darkmatter tests
**Observable**: All tests pass.

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib
cargo test
```

**Dependencies**: Step 3.1

## Phase 4: claudine Integration

**Goal**: Ensure non-interactive mode suppresses terminal queries and reuses terminal instances.

### Step 4.1: Update `log::terminal()` for non-interactive mode
**Observable**: `log::terminal()` returns terminal with explicit `color_mode` in non-interactive contexts.

**File**: `claudine/cli/src/log.rs:50`

In `terminal()`, when not plain mode and not force_color, set an explicit color_mode to avoid triggering detection:

```rust
pub fn terminal() -> Terminal {
    if colors_disabled() {
        Terminal::builder()
            .is_tty(false)
            .color_depth(ColorDepth::None)
            .color_mode(ColorMode::Dark) // Avoid OSC query
            .build()
    } else if force_color_enabled() {
        Terminal::new_optimistic(forced_width(80))
    } else {
        // Normal mode - let Terminal::new() detect, but it will now cache
        Terminal::new()
    }
}
```

Also update `optimistic_terminal()` to set `color_mode`.

**Dependencies**: Phase 2

### Step 4.2: Cache terminal instances in hot paths
**Observable**: `wrap_terminal()` and `StreamTextRenderer` do not recreate `Terminal` on every call.

**Files**:
- `claudine/cli/src/commands/wrap/mod.rs:190` - `wrap_terminal()` already calls `crate::log::terminal()`; with caching this is
now safe
- `claudine/cli/src/commands/wrap/exec/mod.rs:88` - `StreamTextRenderer::new()` creates terminal once; verify this is
sufficient
- `claudine/cli/src/commands/wrap/exec/spawn.rs:740,878` - Check if these create new terminals per render; if so, cache them

**Dependencies**: Step 4.1

### Step 4.3: Run claudine tests
**Observable**: All tests pass.

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli
cargo test
```

**Dependencies**: Steps 4.1, 4.2

## Phase 5: Validation

**Goal**: Confirm fix resolves bleed without regressions.

### Step 5.1: Run full test suite for affected crates
**Observable**: All tests pass across biscuit-terminal, darkmatter, and claudine.

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine
# biscuit-terminal
cargo test -p biscuit-terminal
# darkmatter
cargo test -p darkmatter
# claudine CLI
cargo test -p claudine-cli
```

**Dependencies**: Phases 2-4

### Step 5.2: Verify reproduction no longer shows bleed
**Observable**: Reproduction script from Step 1.2 runs without OSC code literals in output.

Run the reproduction script and confirm:
- No `^[]11;rgb:` patterns appear in stdout
- Tool call icons render correctly
- Output remains clean throughout session

**Dependencies**: Step 5.1

### Step 5.3: Check for regressions in interactive mode
**Observable**: Interactive claudine session renders correctly with proper colors.

Test scenarios:
- Interactive session (no prompt arg, TTY stdin): colors and icons render correctly
- Dark mode terminal: dark theme applied
- Light mode terminal: light theme applied
- Status icons: proper Tailwind colors based on terminal background

**Dependencies**: Step 5.2
**Parallelizable with**: Step 5.2 (different test environments)

## Dependency Graph

```
Phase 1
    ├── Step 1.1
    ├── Step 1.2 (depends on 1.1)
    └── Step 1.3 (depends on 1.2)

Phase 2
    ├── Step 2.1
    ├── Step 2.2 (depends on 2.1)
    ├── Step 2.3 (depends on 2.1) [parallel with 2.2]
    ├── Step 2.4 (depends on 2.1, 2.2)
    ├── Step 2.5 (depends on 2.4)
    ├── Step 2.6 (depends on 2.4) [parallel with 2.5]
    ├── Step 2.7 (independent) [parallel with all 2.x]
    └── Step 2.8 (depends on 2.1-2.7)

Phase 3
    ├── Step 3.1 [parallel with Phase 2 after 2.1]
    └── Step 3.2 (depends on 3.1)

Phase 4
    ├── Step 4.1 (depends on Phase 2)
    ├── Step 4.2 (depends on 4.1)
    └── Step 4.3 (depends on 4.1, 4.2)

Phase 5
    ├── Step 5.1 (depends on Phases 2-4)
    ├── Step 5.2 (depends on 5.1)
    └── Step 5.3 (depends on 5.1) [parallel with 5.2]
```

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Changing `Terminal::color_mode()` from static to instance breaks external consumers | Search all workspace crates for
`Terminal::color_mode()` calls; update all call sites in Phase 2.6 |
| `OnceLock` cache causes issues in tests | Use `std::sync::OnceLock` which is thread-safe and lazy; tests can override via
builder |
| Non-interactive detection misses some edge cases | The fix is defensive: caching + explicit color_mode in non-interactive
paths; even if detection is wrong, query only runs once |
| Performance regression from caching | Caching reduces work; no performance concern |
| Different behavior in multiplexers (tmux, screen) | `query_osc_actual` already checks for multiplexers and returns early;
caching does not change this |

## Parallelization Summary

**Within Phase 2**:
- Steps 2.2 and 2.3 can be done in parallel after 2.1
- Step 2.7 (process-level cache) can be done in parallel with all other Phase 2 steps
- Steps 2.5 and 2.6 can be done in parallel after 2.4

**Across Phases**:
- Phase 3 can start after Phase 2 Step 2.1 (independent crate)
- Phase 4 must wait for Phase 2 completion
- Phase 5 must wait for Phases 2-4

**Maximum parallel paths**:
1. Phase 2 Steps 2.2, 2.3, 2.7 (after 2.1)
2. Phase 3 Step 3.1 (after Phase 2 starts)
3. Phase 5 Steps 5.2, 5.3 (after 5.1)
)
 ← Read(successful, /Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/plan.md)

Plan saved to claudine/plan.md.

5 phases, 18 steps covering:

1. Diagnosis & Baseline — Confirm code paths and create reproduction
2. biscuit-terminal Core — Cache color_mode in Terminal struct, add process-level OnceLock for OSC queries, fix
Status::to_terminal() and other call sites
3. darkmatter — Cache color_mode in TerminalOptions::default()
4. claudine Integration — Set explicit color_mode in non-interactive paths, cache terminal instances
5. Validation — Full test suite + reproduction verification + interactive regression check

Key parallel paths:
- Phase 2 Steps 2.2, 2.3, 2.7 can run concurrently after 2.1
- Phase 3 can start alongside Phase 2
- Phase 5 Steps 5.2 and 5.3 can run in parallel
