# Plan: Execution Line First

## Problem

In the **direct wrapper** path (`run_provider_wrapper_inner` in `claudine/cli/src/commands/wrap/mod.rs`), the execution line is emitted at **line 1081** — *after* env plan construction, MCP session composition, system prompt resolution, argument manipulation, and CWD switch. This makes the program feel slow because the user sees no feedback until all that work completes.

The **composition path** already does the right thing: it emits the execution line early (line 184 in `composition.rs`) with the comment "Emit the execution line as early as possible."

The spec also requests explicit preflight status messages to STDERR.

## Current Order (Direct Wrapper)

```
1.  Profile resolution                         (line 645)
2.  Startup detection (sniff scan)             (line 658)
3.  Binary resolution                          (line 669)
4.  Argument parsing + flag extraction         (lines 671–815)
5.  System prompt resolution                   (lines 817–849)
6.  Operation/sandbox flags                    (lines 851–862)
7.  Env plan construction                      (lines 867–878)
8.  MCP session composition                    (lines 892–1044)
9.  Dry-run handling                           (lines 1052–1067)
10. CWD switch                                 (line 1069)
11. **EXECUTION LINE** (late!)                 (lines 1079–1095)
12. Env details + warnings                     (lines 1097–1127)
13. System prompt display                      (lines 1129–1146)
14. Harness preflight                          (lines 1197–1241)
15. Provider execution                         (lines 1246+)
```

## Target Order (Direct Wrapper)

```
1.  Profile resolution                         (unchanged)
2.  Startup detection                          (unchanged)
3.  Binary resolution                          (unchanged)
4.  Argument parsing + flag extraction         (unchanged)
5.  **EXECUTION LINE** (moved early)           ← MOVED HERE
6.  System prompt resolution                   (unchanged)
7.  Operation/sandbox flags                    (unchanged)
8.  Env plan construction                      (unchanged)
9.  **Status: "starting pre-flight checks"**   ← NEW
10. MCP session composition                    (unchanged)
11. Harness preflight (if applicable)          (unchanged)
    - Authorization prompts if needed
12. **Status: "pre-flight checks have passed"** ← NEW
13. Env details + warnings + system prompt     (unchanged)
14. Dry-run handling                           (unchanged)
15. CWD switch                                 (unchanged)
16. Provider execution                         (unchanged)
```

## Changes

### 1. Move execution line early in direct wrapper

**File:** `claudine/cli/src/commands/wrap/mod.rs`

Move the `log_wrapper_header` call from line 1081 to just after argument parsing and before system prompt resolution (~line 816).

Challenge: The header needs `yolo_enabled`, `effective_non_interactive`, `repo_requested`, `detail_requested`, `prompt_display`, `effective_operation`, and `env_plan`. Of these, all except `env_plan` are available after step 4. The header currently receives `&env_plan` but only uses it for env-var diff display — which is not needed for the initial execution line.

Two approaches:
- **A) Build a minimal/default env plan for the header call.** The composition path already passes `&Default::default()` for env_plan in the early header call (composition.rs:196). Do the same here.
- **B) Split the header into two calls.** Not needed — the env plan info is only in the detail section that follows, not in the execution line itself.

**Approach: A** — pass `&Default::default()` for `env_plan` in the early header call (matching the composition pattern), then keep the env detail output at its current location.

### 2. Add "starting pre-flight checks" status

**File:** `claudine/cli/src/commands/wrap/mod.rs`

After env plan construction and before MCP session composition, emit a `Status` message:

```rust
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::Renderable as _;

if !silent_requested && !quiet_requested {
    let status = Status::from_prose("Starting pre-flight checks".to_string())
        .state(StatusState::Info)
        .theme(StatusTheme::Circular);
    crate::log::message(&status.render(&term));
}
```

### 3. Add "pre-flight checks have passed" status

**File:** `claudine/cli/src/commands/wrap/mod.rs`

After harness preflight completes (end of the `wrapper_harness` block, ~line 1241) and before the env details block, emit:

```rust
if !silent_requested && !quiet_requested {
    let status = Status::from_prose("Pre-flight checks have passed".to_string())
        .state(StatusState::Success)
        .theme(StatusTheme::Circular);
    crate::log::message(&status.render(&term));
}
```

### 4. Add preflight status to composition path

**File:** `claudine/cli/src/commands/wrap/composition.rs`

The composition path already has a "preflight-complete" status at lines 676–691. Add a matching "starting pre-flight checks" status before the harness detection and preflight section (~line 566). The existing "shell commands approved" status at line 686 already serves as the "passed" message, so only the "starting" message needs adding.

### 5. Ensure sequence path coverage

**File:** `claudine/cli/src/commands/wrap/sequence.rs`

Review whether the sequence path already has adequate preflight messaging. The spec mentions the execution line should appear before pre-flight work — verify the sequence orchestrator emits its execution line before phase-1 preflight (line 106).

## Files to Modify

| File | Change |
|------|--------|
| `claudine/cli/src/commands/wrap/mod.rs` | Move `log_wrapper_header` call early; add preflight start/passed status messages |
| `claudine/cli/src/commands/wrap/composition.rs` | Add "starting pre-flight checks" status before harness detection |
| `claudine/cli/src/commands/wrap/sequence.rs` | Verify execution line timing; add preflight status if missing |

## Testing

1. `cargo build -p claudine-cli` — verify compilation
2. `cargo test -p claudine-cli` — verify existing tests pass
3. Manual test: `claudine claude "say hello"` — execution line should appear immediately, before env/MCP work
4. Manual test: `claudine opencode "say hello"` — same
5. Manual test with compose: `claudine compose @some/file.md` — should still show execution line first (already works)
6. Verify the two new Status messages appear in the expected order:
   - "Starting pre-flight checks" (Info, circular)
   - "Pre-flight checks have passed" (Success, circular)

## Risks

- **Env plan not available for early header:** Mitigated by using `&Default::default()` (already the composition pattern).
- **Silent/quiet mode:** Both new status messages must respect `silent_requested` and `quiet_requested` flags.
- **Harness preflight requires binary + args:** The harness preflight at line 1224 depends on the full arg pipeline. The "starting" message goes before MCP session composition (~line 892), and the "passed" message goes after the harness block (~line 1241), so all dependencies are satisfied.
