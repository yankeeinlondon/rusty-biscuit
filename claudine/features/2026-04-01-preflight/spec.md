# Pre-Flight Shell Command Approval

## Problem

When Claudine launches a non-interactive provider session (via `claudine compose`, `claudine claude`, or any wrapper command), shell commands embedded in the prompt template, harness pre/post checks, and handlers may require user approval. Currently, approval is attempted at execution time — deep inside Darkmatter's compose pipeline or during harness validation. In a non-interactive context, there is no opportunity to prompt the user, causing the process to hang indefinitely or produce misleading error messages.

The observed failure: `just commit` invoked `claudine compose "@prompts/commit.md"`, which called Darkmatter's `compose_with()`. The template contained `::shell sniff repo packages`. The process appeared hung, and only after pressing CTRL+C did a misleading "Command timed out after 10s" error appear — when the real issue was that approval was never resolved.

## Root Cause

Two independent approval systems exist:

1. **Darkmatter's** — built into `compose_with()`, uses `ShellApprovalHandler` trait, whitelist files at the git root or HOME directory, and an interactive approval callback.
2. **Claudine's harness** — in `harness/shell.rs`, uses its own `ShellApprovalOptions` with a separate cache and approval handler.

When Claudine calls `prepare_direct()` → `compose_with()`, it passes `ComposeOptions` with no approval handler. Darkmatter runs its own approval flow and, finding no handler for unapproved commands, either returns `ApprovalRequired` or blocks. Claudine has no visibility into what Darkmatter is doing, and the user sees a hung process.

## Design Principles

1. **Claudine owns all shell approval.** Darkmatter discovers commands; Claudine decides what's allowed.
2. **All approvals happen before any provider session begins.** Once a session starts, no further user interaction is needed for shell commands.
3. **Never block.** If a command can't be approved, fail immediately with a clear error.
4. **Clear error messages.** Every error identifies what happened, where, and why. No nested chains like "compose failed: Shell expansion failed: Command timed out after 10s."

## Design

### Darkmatter Library Changes

#### New Public Function: `collect_shell_commands`

```rust
/// Walks the full document graph (following transclusions) and returns
/// every `::shell` directive found, with source file and line info.
///
/// Runs interpolation using the provided `ComposeOptions` state (external_state,
/// set_overrides, context) so that dynamic paths and template variables resolve
/// identically to how they would during `compose_with()`.
///
/// No approval checks, no whitelist lookups, no execution.
/// Pure document analysis.
pub fn collect_shell_commands(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> Result<Vec<ShellCommandEntry>, MarkdownError>
```

Location: `darkmatter::markdown::compose::shell_expansion`

**How it works internally:**

1. Runs the interpolation stage using the provided state so `{{variable}}` references in `::shell` directives and `::file` transclusion paths resolve correctly.
2. Walks the transclusion graph (following `::file` references) to discover shell commands in child documents.
3. Parses `::shell` directives using the existing parser.
4. Resolves aliases (so the entry reflects the actual command that would execute).
5. Deduplicates by normalized form (same command in multiple files only needs one approval).
6. Does NOT execute anything. Does NOT check any policy files.

**The caller must pass the same `ComposeOptions` state** (external_state, set_overrides, source) that will later be used for `compose_with()`. This ensures the pre-flight scan sees exactly the same resolved commands.

#### New Type: `ShellCommandEntry`

```rust
/// A shell command discovered during document graph analysis.
pub struct ShellCommandEntry {
    /// The raw command as written in the directive (e.g., "sniff repo packages").
    pub raw_command: String,
    /// The resolved executable name (e.g., "sniff").
    pub executable: String,
    /// The argument list (e.g., ["repo", "packages"]).
    pub args: Vec<String>,
    /// Normalized form for whitelist matching.
    pub normalized: String,
    /// Source file where this directive was found.
    pub source_file: PathBuf,
    /// Line number in the source file.
    pub line: usize,
}
```

Location: `darkmatter::markdown::compose::shell_expansion::types`

#### New Field on `ComposeOptions`: `pre_approved_commands`

```rust
/// Pre-approved shell commands (normalized forms).
///
/// When set, the shell expansion stage skips the entire approval flow
/// (no whitelist check, no blacklist check, no approval handler).
/// Each directive's normalized command is checked against this set:
/// - Found → execute immediately (still subject to timeout)
/// - Not found → immediate Denied error
///
/// Mutually exclusive with `shell_approval_handler`. When this field
/// is `Some`, the approval handler is ignored.
pub pre_approved_commands: Option<HashSet<String>>
```

Location: `darkmatter::markdown::compose::types::ComposeOptions`

#### Compose-Time Behavior

When `pre_approved_commands` is `Some`:

- The shell expansion stage skips whitelist, blacklist, and approval handler entirely.
- For each `::shell` directive, checks the normalized command against the pre-approved set.
- If found: execute (still subject to timeout, stdout/stderr capture).
- If not found: immediate `Denied` error with a message indicating a pre-flight scanner bug.
- The `shell_approval_handler` field is ignored.

When `pre_approved_commands` is `None` (default):

- Existing behavior preserved. Whitelist checks, approval handler callbacks, `ApprovalRequired` when no handler is set. Darkmatter's standalone usability is unchanged.

### Claudine Library Changes

#### New Module: `composition::preflight`

```rust
/// Scans all sources of shell commands for a provider session,
/// checks them against the whitelist, prompts the user for any that
/// need approval, and returns the full pre-approved set.
///
/// `compose_options` is required when the session involves template
/// composition (compose/inline-compose). For direct wrapper commands
/// (e.g., `claudine claude "prompt"`) where no template composition
/// occurs, pass `None` — only harness sources are scanned.
///
/// Returns Err immediately if the user denies any command.
pub fn resolve_shell_approvals(
    source: Option<&ResolvedCompositionSource>,
    compose_options: Option<&ComposeOptions>,
    harness_plan: Option<&HarnessPlan>,
    approval_options: &ShellApprovalOptions,
) -> Result<HashSet<String>, CompositionError>
```

**Three command sources collected:**

1. **Template `::shell` directives** — when `source` and `compose_options` are provided, calls Darkmatter's `collect_shell_commands()` with the same `ComposeOptions` that will be used for compose, getting all commands from the full document graph. Skipped for direct wrapper commands with no template.
2. **Harness pre_checks / post_checks** — iterates `ValidationRule::ShellCommand` variants from the `HarnessPlan`.
3. **Harness handlers** — iterates handler definitions that contain shell commands. These are conditional (may not fire at runtime) but we still need approval upfront since they might.

**Approval flow:**

1. Collect all commands from the three sources.
2. Deduplicate by normalized form.
3. Check each against Claudine's whitelist (existing `check_whitelist` + `check_builtin_blacklist` in `harness/shell.rs`).
4. For any not covered: present to user sequentially via the existing `ShellApprovalHandler` trait (AllowOnce, AllowExactPersist, AllowCommandPersist, Deny, BlacklistPersist).
5. If user denies any command: return `Err(CompositionError::ShellCommandDenied { ... })` immediately. Session aborts.
6. Return the full set (whitelisted + newly approved) as `HashSet<String>` of normalized commands.

#### Updated Error Types

Three distinct error variants in `CompositionError`:

1. **Pre-flight denial** — user denied a command during pre-flight:
   ```
   Aborted: shell command 'rm -rf /' was denied during pre-flight approval.
   No provider session was started.
   ```

2. **Runtime unapproved** (safety net — should never happen):
   ```
   Shell command 'sniff repo packages' was not pre-approved and cannot
   be approved during an active session. This is a bug in the pre-flight
   scanner -- please report it.

   Source: prompts/commit.md:23
   ```

3. **Genuine execution failure** (command runs but fails or times out):
   ```
   Shell command 'sniff repo packages' failed after 10s (timeout).

   Source: prompts/commit.md:23
   Working directory: /path/to/repo

   This command was approved and executed but did not complete within
   the timeout. If this command normally completes quickly, it may be
   blocked by another process or waiting on a resource.
   ```

### Claudine CLI Changes

**All wrapper commands** call `resolve_shell_approvals()` after prompt resolution but before provider launch. This applies to:

- `claudine compose <file>` and `claudine inline-compose <file>`
- `claudine claude`, `claudine codex`, `claudine gemini`, `claudine opencode`, `claudine qwen`, `claudine goose`, `claudine kimi`
- Any wrapper command that launches a provider session with a prompt

The approved set flows into:

- `ComposeOptions::pre_approved_commands` for compose/inline-compose paths
- The harness runtime for pre/post check and handler shell command execution

The approval handler is constructed from the CLI's terminal context (prompting on stderr since stdout may be piped). It is only used during pre-flight — it is never passed to Darkmatter.

## What Doesn't Change

- Darkmatter's standalone behavior when `pre_approved_commands` is `None`
- Whitelist/blacklist file format (`.darkmatter-shell-whitelist`, `.darkmatter-shell-blacklist`)
- The `ShellApprovalHandler` trait interface
- Harness validation and handler architecture (pre_checks, post_checks, handlers)
- The existing `ShellExpansionRuntime` and `PipelineRuntime` types

## Testing Strategy

- **Darkmatter unit tests**: `collect_shell_commands` returns correct entries for documents with transclusions, interpolated variables, and aliases. Pre-approved compose skips approval flow. Missing pre-approval produces immediate `Denied`.
- **Claudine unit tests**: `resolve_shell_approvals` collects from all three sources, deduplicates, respects whitelist, calls handler for unapproved commands, aborts on denial.
- **Integration tests**: End-to-end flow with a template containing `::shell` directives, verifying pre-flight runs before compose and the approved set is passed through.
