# Preflight Improvements Design

Implements all findings and improvement ideas from `claudine/features/2026-04-01-preflight/review.md`.

## Context

The preflight shell approval system works but has structural gaps identified during review:

- **3 High findings**: No interactive approval handler wired, harness parsing couples discovery with authorization, preflight approvals discarded before runtime
- **4 Medium findings**: Wrong transclusion provenance, discovery skips compose stages, interactive wrappers bypass harness preflight, flat error reporting
- **4 Improvement ideas**: Shared command type, reuse CLI handler, discovery-only harness parsing, cached document graph

The improvement ideas are the natural fixes for their corresponding findings, so they are merged rather than layered separately.

## Approach

Component-layered, bottom-up by package dependency:

1. Darkmatter foundation (types, discovery pipeline, provenance)
2. Claudine-lib (approval authority, cache propagation)
3. Claudine-cli (handler wiring, interactive preflight, error reporting)
4. Tests across both packages

## Group 1: Darkmatter Foundation

Addresses: Finding 4, Finding 5, Improvement Idea 1.

### 1a. Align discovery pipeline with compose order (Finding 5)

`collect_shell_commands()` currently runs composition with:

```
FrontmatterInterpolation, Interpolation, BlockTransclusion, FrontmatterTransclusion
```

The real compose order also includes `TextReplacement` and `PageBlocks` before `ShellExpansion`. Discovery must mirror this to avoid collecting commands from conditionally removed blocks or missing commands introduced by replacement.

**Change:** Update the `.only()` list in `collect_shell_commands()` to:

```
FrontmatterInterpolation, TextReplacement, PageBlocks, Interpolation, BlockTransclusion, FrontmatterTransclusion
```

This matches the real pipeline order up to (but not including) `ShellExpansion`. `PageBlocks` conditional evaluation is valid because the frontmatter/state context is passed through `ComposeOptions`.

**File:** `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs`

### 1b. Source map for transclusion provenance (Finding 4)

The composed output is a flat string with no per-line origin tracking. When `BlockTransclusion` inserts content from another file, the originating file is lost. Discovery then attributes every command to the root document.

**Change:** Extend `ComposeReport` with a source map:

```rust
pub struct SourceRange {
    pub byte_start: usize,
    pub byte_end: usize,
    pub source_file: PathBuf,
    pub source_start_line: usize,
}
```

`BlockTransclusion` already knows the source file and insertion byte span when it replaces content. It records a `SourceRange` entry in the report for each transclusion insertion.

**Change:** `collect_shell_commands()` receives the `ComposeReport` from `compose_with()` and uses the source map to look up each `::shell` directive's byte position, attributing it to the correct originating file and line rather than the root document. Commands not covered by any source range default to the root document (existing behavior).

**Files:**
- `darkmatter/lib/src/markdown/compose/types.rs` (add `SourceRange` to `ComposeReport`)
- `darkmatter/lib/src/markdown/compose/mod.rs` (BlockTransclusion records source ranges)
- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs` (use source map for attribution)

### 1c. Preserve provenance through to claudine (Improvement Idea 1)

Currently claudine's `resolve_shell_approvals()` reduces each `ShellCommandEntry` to `entry.normalized`, discarding `source_file` and `line`. The `ShellCommandEntry` already carries the right fields — claudine just needs to stop stripping them.

**Change:** When building `ShellApprovalRequest` in `resolve_shell_approvals()`, populate `source` and `line` from the `ShellCommandEntry` rather than defaulting. Error messages and approval prompts then show the real source location.

**File:** `claudine/lib/src/composition/preflight.rs`

## Group 2: Claudine-lib Approval Authority

Addresses: Finding 1, Finding 2, Finding 3, Improvement Ideas 2 and 3.

### 2a. Make harness parsing discovery-only (Finding 2, Improvement 3)

`parse_harness_plan_with_shell()` accepts `Option<&ShellApprovalOptions>` and calls `validate_and_approve_command_parts()` during parsing. This means harness command approval happens at parse time, before `resolve_shell_approvals()` runs — breaking the design that preflight is the single approval authority.

**Change:**

- Remove the `shell_options` parameter from `parse_harness_plan_with_shell()`. Revert the signature to plain `parse_harness_plan()`.
- Harness parsing only tokenizes commands (executable + args) without policy checks. The existing `None` branch already does this via `tokenize_to_approved_command()` — make that the only path.
- Remove `parse_runtime_command()` and `parse_runtime_command_parts()` helper functions that branch on `shell_options`. Replace with direct tokenization.
- All approval decisions happen in `resolve_shell_approvals()`, which already collects harness commands via `collect_auditable_commands()`.

**Files:**
- `claudine/lib/src/harness/parse.rs` (remove shell_options parameter, simplify to tokenize-only)
- All call sites of `parse_harness_plan_with_shell` (update to new signature)

### 2b. Carry preflight approvals through to runtime (Finding 3)

Currently the `PreFlightResult` is logged and discarded. `AllowOnce` decisions are lost.

The fix relies on how `resolve_shell_approvals()` already works: it calls `validate_and_approve_command_parts()` which populates the `approval_cache` on the shared `ShellApprovalOptions` as a side effect. The same `ShellApprovalOptions` instance (with its warm cache) must be passed through to the harness runtime loop.

**Change:**

- In `wrap/composition.rs`: pass the same `shell_options` that preflight used into the harness execution path, so the warm `approval_cache` carries `AllowOnce` and other session-local decisions.
- In `wrap/mod.rs`: same — the `shell_options` used for preflight must be the same instance passed to `run_harness_loop()`.
- For template commands: the `approved_commands: HashSet<String>` from `PreFlightResult` is already passed into `ComposeOptions::pre_approved_commands` for the real compose pass in the composition path. Verify this is wired correctly in the passthrough path as well.

**Files:**
- `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

### 2c. Wire interactive approval handler (Finding 1, Improvement 2)

`build_harness_shell_options()` sets `approval_handler: None`, so any unapproved command hard-fails instead of prompting.

**Change:**

- For interactive sessions: set `approval_handler: Some(Arc::new(CliShellApprovalHandler::new()))` in `build_harness_shell_options()`. The handler already exists in `darkmatter/cli/src/approval.rs`.
- For non-interactive sessions: keep `approval_handler: None` — unapproved commands should hard-fail since there's no terminal to prompt on.
- Add `darkmatter-cli` as a dependency of `claudine-cli` (the handler lives there, not in darkmatter-lib).
- Update `build_harness_shell_options()` signature to accept an `interactive: bool` parameter (or equivalent) to decide whether to attach the handler.

**Files:**
- `claudine/cli/src/commands/wrap/mod.rs` (update builder, add handler)
- `claudine/cli/Cargo.toml` (add darkmatter-cli dependency)

## Group 3: Claudine-cli Wiring

Addresses: Finding 6, Finding 7.

### 3a. Enable harness preflight for interactive wrappers (Finding 6)

The passthrough wrapper path only runs harness preflight when `effective_non_interactive` is true. The design says preflight should run for all wrapper commands because shell commands execute before the provider session begins.

**Change:**

- Remove the `effective_non_interactive` guard around harness preflight in `wrap/mod.rs`.
- Preflight runs unconditionally when a harness is detected in frontmatter.
- Interactive sessions prompt via `CliShellApprovalHandler` (from 2c). Non-interactive sessions hard-fail on unapproved commands.

**File:** `claudine/cli/src/commands/wrap/mod.rs`

### 3b. Improve error reporting (Finding 7)

Errors are currently flat strings. The design calls for richer, preflight-specific framing.

**Change:**

- Enrich `ShellCommandDenied` error variant to include `source_file`, `line`, and `working_directory` fields.
- Enrich `PreFlightFailed` to include the specific command, source location, and reason (blacklisted vs. no handler available).
- For the `NotPreApproved` runtime error (the "scanner bug" case), include the source file and line from `ShellCommandEntry` provenance (now available from Group 1).
- Format these errors with structured prose following claudine's existing CLI error formatting patterns.

**Files:**
- `claudine/lib/src/composition/error.rs` (enrich error variants)
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` (enrich `NotPreApproved`)
- `claudine/lib/src/composition/preflight.rs` (pass enriched context into errors)

## Group 4: Tests

### 4a. Darkmatter discovery tests

**Transclusion provenance fidelity:**
- A root document transcludes a child document containing `::shell` directives. After discovery, the `source_file` on those entries should be the child document's path, not the root.
- Line numbers should refer to the child document's lines, not composed output lines.

**PageBlocks/TextReplacement interactions:**
- A `::shell` directive inside a `::block when="false"` block should be excluded from discovery results.
- A `::shell` directive introduced by `TextReplacement` should appear in discovery results.

**File:** `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs` (tests module)

### 4b. Claudine-lib preflight tests

**All 5 approval decisions:**
- Mock handler returning each of `AllowExactPersist`, `AllowCommandPersist`, `AllowOnce`, `Deny`, `BlacklistPersist`.
- Verify that `Deny` and `BlacklistPersist` produce `ShellCommandDenied` errors.
- Verify that `AllowOnce` populates the cache but does not persist to whitelist file.
- Verify that `AllowExactPersist` and `AllowCommandPersist` persist to whitelist file.

**Warm cache propagation:**
- Run preflight with a mock handler that approves via `AllowOnce`.
- Pass the same `ShellApprovalOptions` to a second validation call.
- Verify the second call does not invoke the handler (cache hit).

**File:** `claudine/lib/src/composition/preflight.rs` (tests module)

### 4c. Claudine-cli integration tests

**Preflight prompting:**
- End-to-end test using a test handler that verifies approval prompts are shown for unapproved commands.

**Interactive wrapper preflight:**
- End-to-end test that an interactive wrapper session runs harness preflight (previously skipped).

**Enriched error context:**
- Test that preflight error messages include source file, line, and working directory.

**File:** `claudine/cli/tests/wrap_commands.rs`

## Deferred

- Retry/redirect/deviate flows preserving preflight shell approvals across harness attempts. This is a deeper harness lifecycle concern outside the scope of these findings.
- Cache/share the resolved document-graph analysis between `collect_shell_commands()` and the subsequent compose pass (Improvement Idea 4). This is a performance optimization gated on observed startup latency, not a correctness fix.

## Dependency Order

```
Group 1a (discovery pipeline) ─┐
Group 1b (source map)         ─┼─→ Group 2a (discovery-only parsing) ─→ Group 2b (cache propagation)
Group 1c (provenance)         ─┘                                        Group 2c (handler wiring)
                                                                              │
                                                                              ↓
                                                                     Group 3a (interactive preflight)
                                                                     Group 3b (error reporting)
                                                                              │
                                                                              ↓
                                                                     Group 4 (tests)
```
