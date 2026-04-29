# Pre-Flight Shell Approval Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure all shell commands are approved before any provider session begins, preventing hangs and producing clear error messages.

**Architecture:** Darkmatter gets a new `collect_shell_commands()` function that walks the full document graph (with interpolation) and returns every `::shell` directive without executing or approving anything. Darkmatter also gets a `pre_approved_commands` field on `ComposeOptions` that bypasses the approval flow during compose. Claudine gets a new `preflight` module that orchestrates: call Darkmatter's discovery, merge with harness commands from the existing `audit.rs`, check whitelists, prompt the user, and pass the approved set back to Darkmatter.

**Tech Stack:** Rust, darkmatter (compose pipeline), claudine (harness, composition, CLI wrapper)

**Spec:** `docs/superpowers/specs/2026-04-01-preflight-shell-approval-design.md`
**Documentation:** `claudine/docs/topics/pre-flight-checks.md`

---

## File Structure

### Darkmatter Changes

| File | Action | Responsibility |
|------|--------|---------------|
| `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` | Modify | Add `ShellCommandEntry` type |
| `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs` | Create | `collect_shell_commands()` — document graph walking |
| `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs` | Modify | Export new module and types |
| `darkmatter/lib/src/markdown/compose/types.rs` | Modify | Add `pre_approved_commands` field to `ComposeOptions` |
| `darkmatter/lib/src/markdown/compose/mod.rs` | Modify | `run_shell_expansion_stage` pre-approved fast path |

### Claudine Changes

| File | Action | Responsibility |
|------|--------|---------------|
| `claudine/lib/src/composition/preflight.rs` | Create | `resolve_shell_approvals()` orchestration |
| `claudine/lib/src/composition/mod.rs` | Modify | Export preflight module |
| `claudine/lib/src/composition/error.rs` | Modify | Add pre-flight error variants |
| `claudine/lib/src/composition/prepare.rs` | Modify | Accept and set `pre_approved_commands` |
| `claudine/cli/src/commands/compose.rs` | Modify | Wire pre-flight into compose commands |
| `claudine/cli/src/commands/wrap/composition.rs` | Modify | Wire pre-flight into wrapper execution |

---

## Task 1: Add `ShellCommandEntry` Type to Darkmatter

**Files:**

- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`

- [ ] **Step 1: Add `ShellCommandEntry` struct**

In `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`, add after the `ShellDirective` struct (after line 23):

```rust
/// A shell command discovered during document graph analysis.
///
/// Returned by [`collect_shell_commands`] for pre-flight approval.
/// Contains enough information for the caller to check policy and
/// prompt the user without needing to understand the document graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandEntry {
    /// The raw command as written in the directive (e.g., "sniff repo packages").
    pub raw_command: String,
    /// The resolved executable name (e.g., "sniff").
    pub executable: String,
    /// The argument list (e.g., ["repo", "packages"]).
    pub args: Vec<String>,
    /// Normalized form for whitelist matching (e.g., "sniff repo packages").
    pub normalized: String,
    /// Source file where this directive was found.
    pub source_file: std::path::PathBuf,
    /// Line number in the source file.
    pub line: usize,
}
```

- [ ] **Step 2: Export `ShellCommandEntry` from the module**

In `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`, add `ShellCommandEntry` to the `pub use types::` block (around line 43):

```rust
pub use types::{
    ErrorHandling, ErrorHandlingOutcome, ShellApprovalDecision, ShellApprovalHandler,
    ShellApprovalRequest, ShellCommandEntry, ShellDirective, ShellExpansionError,
    ShellExpansionOptions, ShellExpansionRuntime, ShellPolicyPaths, ShellRuleSet,
};
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p darkmatter`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/shell_expansion/types.rs darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
git commit -m "feat(darkmatter): add ShellCommandEntry type for pre-flight discovery

- add ShellCommandEntry struct with raw_command, executable, args, normalized, source_file, line
- export from shell_expansion module"
```

---

## Task 2: Implement `collect_shell_commands` in Darkmatter

**Files:**
- Create: `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs`
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`

This is the core discovery function. It walks the full document graph (following transclusions, running interpolation) and returns every `::shell` directive as a `ShellCommandEntry`. It does NOT execute commands or check policy.

- [ ] **Step 1: Write tests for `collect_shell_commands`**

Create `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs`:

```rust
//! Shell command discovery across the document graph.
//!
//! Walks transclusions and resolves interpolation to find every `::shell`
//! directive that would be executed during composition. Returns entries
//! without executing or approving anything.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::markdown::Markdown;
use crate::markdown::compose::shell_expansion::types::ShellCommandEntry;
use crate::markdown::compose::shell_expansion::{normalize_command, parse_directives};
use crate::markdown::compose::shell_expansion::alias::resolve_alias;
use crate::markdown::compose::{ComposeOperation, ComposeOptions, ComposeSource};
use crate::markdown::types::{MarkdownError, MarkdownResult};

/// Walks the full document graph and returns every `::shell` directive found.
///
/// Runs interpolation using the provided `ComposeOptions` state so that
/// template variables and dynamic transclusion paths resolve identically
/// to how they would during `compose_with()`.
///
/// No approval checks, no whitelist lookups, no execution.
///
/// ## Errors
///
/// Returns `MarkdownError` if interpolation or transclusion resolution fails.
pub fn collect_shell_commands(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> MarkdownResult<Vec<ShellCommandEntry>> {
    // Compose with only interpolation + transclusion enabled (no shell execution).
    // This resolves the full document graph so we can parse ::shell directives
    // from the final interpolated content of every file in the tree.
    let discovery_options = options
        .clone()
        .only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
            ComposeOperation::BlockTransclusion,
            ComposeOperation::FrontmatterTransclusion,
        ]);

    // Clone the markdown since compose_with consumes it. The clone is cheap
    // relative to the I/O and process spawning that shell execution would do.
    let discovery_md = markdown.clone();

    // Compose to get the fully-resolved document (interpolation + transclusion
    // but NO shell expansion). This gives us the content after all {{variables}}
    // and ::file references are resolved, with ::shell directives still as text.
    let (composed, _report) = discovery_md.compose_with(discovery_options)?;

    // Parse ::shell directives from the composed content.
    let directives = parse_directives(composed.content())?;

    // Resolve source file for entries.
    let source_file = match &options.source {
        ComposeSource::File(p) => p.clone(),
        _ => PathBuf::from("<unknown>"),
    };

    // Build entries, resolving aliases and deduplicating by normalized form.
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    for directive in directives {
        // Resolve alias if executable is not on PATH
        let (executable, args) = if which::which(&directive.executable).is_ok() {
            (directive.executable.clone(), directive.args.clone())
        } else if let Some(resolved) = resolve_alias(&directive.executable) {
            let mut merged_args = resolved.args;
            merged_args.extend_from_slice(&directive.args);
            (resolved.executable, merged_args)
        } else {
            (directive.executable.clone(), directive.args.clone())
        };

        let normalized = normalize_command(&executable, &args);

        if seen.insert(normalized.clone()) {
            entries.push(ShellCommandEntry {
                raw_command: directive.raw_command,
                executable,
                args,
                normalized,
                source_file: source_file.clone(),
                line: directive.line,
            });
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;
    use tempfile::TempDir;

    #[test]
    fn discovers_shell_directives_in_simple_document() {
        let content = "# Test\n::shell echo hello\nSome text\n::shell ls -la\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].executable, "echo");
        assert_eq!(entries[0].args, vec!["hello"]);
        assert_eq!(entries[0].normalized, "echo hello");
        assert_eq!(entries[1].executable, "ls");
    }

    #[test]
    fn discovers_directives_in_transcluded_files() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path().join("root.md");
        let child_path = temp_dir.path().join("child.md");

        std::fs::write(&root_path, "# Root\n::shell echo root\n::file ./child.md\n").unwrap();
        std::fs::write(&child_path, "## Child\n::shell echo child\n").unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();
        // Should find both "echo root" from root.md and "echo child" from child.md
        let commands: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(commands.contains(&"echo root"));
        assert!(commands.contains(&"echo child"));
    }

    #[test]
    fn deduplicates_by_normalized_form() {
        let content = "::shell echo hello\n::shell echo hello\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn resolves_interpolated_variables_before_scanning() {
        let content = "---\ncmd_arg: world\n---\n::shell echo {{ cmd_arg }}\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo world");
    }

    #[test]
    fn ignores_directives_in_code_blocks() {
        let content = "# Test\n::shell echo outside\n```\n::shell echo inside\n```\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo outside");
    }

    #[test]
    fn empty_document_returns_empty_vec() {
        let md: Markdown = "# No shell commands\n".into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn respects_set_overrides_during_interpolation() {
        let content = "---\ntarget: default\n---\n::shell echo {{ target }}\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new()
            .with_set_overrides(serde_json::json!({"target": "overridden"}));

        let entries = collect_shell_commands(&md, &options).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo overridden");
    }
}
```

- [ ] **Step 2: Export the discovery module**

In `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`, add:

After `pub mod types;` (line 34):
```rust
pub mod discovery;
```

Add to the `pub use` section:
```rust
pub use discovery::collect_shell_commands;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p darkmatter -- shell_expansion::discovery --nocapture`
Expected: All tests pass. The interpolation test may need adjustment if Darkmatter's interpolation stage requires frontmatter keys to be in the state — verify and fix as needed.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
git commit -m "feat(darkmatter): add collect_shell_commands for pre-flight discovery

- walk full document graph with interpolation and transclusion
- return ShellCommandEntry for every ::shell directive found
- deduplicate by normalized form
- resolve aliases to report actual commands
- no execution, no policy checks"
```

---

## Task 3: Add `pre_approved_commands` to `ComposeOptions`

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/types.rs`

- [ ] **Step 1: Add the field to `ComposeOptions`**

In `darkmatter/lib/src/markdown/compose/types.rs`, add after the `shell_approval_handler` field (after line 342):

```rust
    /// Pre-approved shell commands (normalized forms).
    ///
    /// When set, the shell expansion stage skips the entire approval flow
    /// (no whitelist check, no blacklist check, no approval handler).
    /// Each directive's normalized command is checked against this set:
    /// - Found: execute immediately (still subject to timeout)
    /// - Not found: immediate Denied error
    ///
    /// Mutually exclusive with `shell_approval_handler`. When this field
    /// is `Some`, the approval handler is ignored.
    pub pre_approved_commands: Option<std::collections::HashSet<String>>,
```

- [ ] **Step 2: Add default in constructor**

In the `new_with_context` method (around line 480), add after `shell_approval_handler: None,`:

```rust
            pre_approved_commands: None,
```

- [ ] **Step 3: Add builder method**

After `with_shell_approval_handler` (around line 653), add:

```rust
    /// Sets the pre-approved shell commands.
    ///
    /// When set, the shell expansion stage bypasses all approval logic
    /// and checks commands against this set instead. Commands not in the
    /// set produce an immediate Denied error.
    #[must_use]
    pub fn with_pre_approved_commands(mut self, commands: std::collections::HashSet<String>) -> Self {
        self.pre_approved_commands = Some(commands);
        self
    }
```

- [ ] **Step 4: Add to Debug impl**

In the manual `Debug` impl (around line 420), add after the `shell_approval_handler` field:

```rust
            .field(
                "pre_approved_commands",
                &self.pre_approved_commands.as_ref().map(|s| format!("{} commands", s.len())),
            )
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p darkmatter`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/types.rs
git commit -m "feat(darkmatter): add pre_approved_commands field to ComposeOptions

- when set, shell expansion bypasses approval flow entirely
- commands checked against pre-approved set instead of whitelist/handler
- not in set produces immediate Denied error
- mutually exclusive with shell_approval_handler"
```

---

## Task 4: Implement Pre-Approved Fast Path in Shell Expansion

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`

- [ ] **Step 1: Add `NotPreApproved` error variant**

In `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`, add to `ShellExpansionError` (after the `Denied` variant, around line 251):

```rust
    #[error(
        "Command '{command}' on line {line} was not pre-approved. \
         This is a bug in the pre-flight scanner -- please report it."
    )]
    NotPreApproved { command: String, line: usize },
```

- [ ] **Step 2: Write test for pre-approved fast path**

In `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`, add to `integration_tests` module:

```rust
    #[test]
    fn pre_approved_commands_bypass_approval_flow() {
        let content = "# Test\n::shell echo hello\n::shell echo world\n";
        let md: Markdown = content.into();

        let mut approved = std::collections::HashSet::new();
        approved.insert("echo hello".to_string());
        approved.insert("echo world".to_string());

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_pre_approved_commands(approved);

        // No approval handler, no whitelist — should succeed via pre-approved set
        let (composed, report) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("hello"));
        assert!(composed.content().contains("world"));
        assert_eq!(report.shell_expansions_applied, 2);
        assert_eq!(report.shell_approvals_used, 0);
    }

    #[test]
    fn pre_approved_rejects_unknown_commands() {
        let content = "# Test\n::shell echo hello\n::shell echo sneaky\n";
        let md: Markdown = content.into();

        let mut approved = std::collections::HashSet::new();
        approved.insert("echo hello".to_string());
        // "echo sneaky" is NOT pre-approved

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_pre_approved_commands(approved);

        let err = md.compose_with(options).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not pre-approved"), "got: {msg}");
        assert!(msg.contains("echo sneaky"), "got: {msg}");
    }
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p darkmatter -- integration_tests::pre_approved --nocapture`
Expected: FAIL — pre-approved path not implemented yet

- [ ] **Step 4: Implement the pre-approved fast path in `execute_directive`**

In `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`, modify `execute_directive` (starting at line 87). Insert the pre-approved check as the very first step, before the existing blacklist/whitelist/approval flow:

```rust
pub fn execute_directive(
    directive: &ShellDirective,
    options: &ComposeOptions,
    policy_paths: &ShellPolicyPaths,
    shell_runtime: &mut ShellExpansionRuntime,
) -> Result<String, ShellExpansionError> {
    // Resolve alias if the executable is not found on PATH
    let (effective, alias_name) = resolve_or_passthrough(directive);

    let normalized = normalize_command(&effective.executable, &effective.args);

    // ── Pre-approved fast path ───────────────────────────────────────
    // When a pre-approved set is provided, skip the entire approval flow.
    // The caller (Claudine) has already verified every command.
    if let Some(ref approved) = options.pre_approved_commands {
        if approved.contains(&normalized) {
            return execute_and_handle_errors(&effective, options, &directive.error_handling);
        } else {
            return Err(ShellExpansionError::NotPreApproved {
                command: display_command(directive, alias_name.as_deref()),
                line: directive.line,
            });
        }
    }

    // ── Standard approval flow (unchanged) ───────────────────────────
    let runtime_snapshot = shell_runtime.snapshot();

    // 1. Check built-in blacklist (against resolved command)
    if let Some(reason) = check_builtin_blacklist(&effective.executable, &effective.args) {
```

Note: the existing code from line 99 onward (`let runtime_snapshot = shell_runtime.snapshot();` through the end of the function) is unchanged. The only addition is the `pre_approved_commands` block before it.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p darkmatter -- integration_tests::pre_approved --nocapture`
Expected: Both tests pass

- [ ] **Step 6: Run full darkmatter test suite**

Run: `cargo test -p darkmatter`
Expected: All existing tests still pass

- [ ] **Step 7: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
git commit -m "feat(darkmatter): implement pre-approved command fast path in shell expansion

- when pre_approved_commands is set, skip blacklist/whitelist/handler entirely
- commands in the set execute immediately (still subject to timeout)
- commands not in the set produce immediate NotPreApproved error
- existing approval flow unchanged when pre_approved_commands is None"
```

---

## Task 5: Add Pre-Flight Error Variants to Claudine

**Files:**
- Modify: `claudine/lib/src/composition/error.rs`

- [ ] **Step 1: Add new error variants**

In `claudine/lib/src/composition/error.rs`, add these variants to `CompositionError`:

```rust
    #[error("Pre-flight shell approval failed: {0}")]
    PreFlightFailed(String),

    #[error(
        "Aborted: shell command '{command}' was denied during pre-flight approval. \
         No provider session was started."
    )]
    ShellCommandDenied { command: String },

    #[error("Pre-flight discovery failed: {0}")]
    PreFlightDiscoveryFailed(String),
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p claudine`
Expected: compiles (possibly with unused variant warnings, which is fine)

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/composition/error.rs
git commit -m "feat(claudine): add pre-flight shell approval error variants

- ShellCommandDenied for user denial during pre-flight
- PreFlightFailed for general pre-flight failures
- PreFlightDiscoveryFailed for Darkmatter discovery errors"
```

---

## Task 6: Implement `resolve_shell_approvals` in Claudine

**Files:**
- Create: `claudine/lib/src/composition/preflight.rs`
- Modify: `claudine/lib/src/composition/mod.rs`

This is the orchestration function that ties everything together.

- [ ] **Step 1: Create the preflight module with tests**

Create `claudine/lib/src/composition/preflight.rs`:

```rust
//! Pre-flight shell command approval for provider sessions.
//!
//! Scans all sources of shell commands (template directives, harness
//! pre/post checks, handlers), checks them against the whitelist,
//! prompts the user for any that need approval, and returns the full
//! pre-approved set.

use std::collections::HashSet;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::compose::shell_expansion::{
    ShellCommandEntry, collect_shell_commands, normalize_command,
};

use crate::composition::error::CompositionError;
use crate::harness::audit::collect_auditable_commands;
use crate::harness::model::HarnessPlan;
use crate::harness::shell::ShellApprovalOptions;

/// Result of pre-flight shell command approval.
///
/// Contains the full set of normalized commands that are authorized
/// for execution during the session.
pub struct PreFlightResult {
    /// Normalized command strings that are approved for execution.
    pub approved_commands: HashSet<String>,
    /// Total number of commands discovered across all sources.
    pub total_discovered: usize,
    /// Number of commands that were already whitelisted.
    pub already_whitelisted: usize,
    /// Number of commands that the user approved interactively.
    pub user_approved: usize,
}

/// Scans all sources of shell commands for a provider session,
/// checks them against the whitelist, prompts the user for any that
/// need approval, and returns the full pre-approved set.
///
/// ## Sources
///
/// 1. Template `::shell` directives (via Darkmatter's document graph walker)
/// 2. Harness pre_checks / post_checks `ShellCommand` validations
/// 3. Harness handlers (`Deviate` actions and programmatic `handle`)
///
/// ## Errors
///
/// - `ShellCommandDenied` if the user denies any command
/// - `PreFlightDiscoveryFailed` if Darkmatter's document graph walk fails
/// - `PreFlightFailed` for other pre-flight errors
pub fn resolve_shell_approvals(
    markdown: Option<&Markdown>,
    compose_options: Option<&ComposeOptions>,
    harness_plan: Option<&HarnessPlan>,
    approval_options: &ShellApprovalOptions,
) -> Result<PreFlightResult, CompositionError> {
    let mut all_normalized: Vec<String> = Vec::new();

    // ── Source 1: Template ::shell directives ─────────────────────────
    if let (Some(md), Some(opts)) = (markdown, compose_options) {
        let entries = collect_shell_commands(md, opts)
            .map_err(|e| CompositionError::PreFlightDiscoveryFailed(e.to_string()))?;

        for entry in &entries {
            all_normalized.push(entry.normalized.clone());
        }
    }

    // ── Source 2 & 3: Harness commands ───────────────────────────────
    if let Some(plan) = harness_plan {
        let auditable = collect_auditable_commands(plan, None)
            .map_err(|e| CompositionError::PreFlightFailed(e.to_string()))?;

        for cmd in &auditable {
            let normalized = normalize_command(&cmd.executable, &cmd.args);
            all_normalized.push(normalized);
        }
    }

    // ── Deduplicate ──────────────────────────────────────────────────
    let unique: Vec<String> = {
        let mut seen = HashSet::new();
        all_normalized
            .into_iter()
            .filter(|n| seen.insert(n.clone()))
            .collect()
    };

    let total_discovered = unique.len();

    // ── Check each against whitelist and prompt if needed ─────────────
    let mut approved = HashSet::new();
    let mut already_whitelisted = 0usize;
    let mut user_approved = 0usize;

    for normalized in &unique {
        // Build parts for the existing validate_and_approve_command_parts
        let parts: Vec<String> = shell_lex::split(normalized)
            .unwrap_or_else(|_| vec![normalized.clone()]);

        match crate::harness::shell::validate_and_approve_command_parts(
            &parts,
            approval_options,
        ) {
            Ok(_) => {
                // Command passed policy — it's either whitelisted or was just approved
                // by the handler. We can't easily distinguish, but the approval_cache
                // in ShellApprovalOptions tracks new interactive approvals.
                approved.insert(normalized.clone());
                already_whitelisted += 1;
            }
            Err(crate::harness::error::HarnessError::ShellCommandDenied { command }) => {
                // If there's no approval handler, this means it needs approval
                // but we can't get it. If there IS a handler and user denied,
                // we abort.
                if approval_options.approval_handler.is_some() {
                    return Err(CompositionError::ShellCommandDenied { command });
                }
                // No handler — treat as needing approval but nobody to ask.
                return Err(CompositionError::PreFlightFailed(format!(
                    "Shell command '{command}' requires approval but no approval handler \
                     is available. Add to whitelist or run interactively."
                )));
            }
            Err(crate::harness::error::HarnessError::ShellCommandBlacklisted {
                command,
                reason,
            }) => {
                return Err(CompositionError::PreFlightFailed(format!(
                    "Shell command '{command}' is blacklisted: {reason}"
                )));
            }
            Err(e) => {
                return Err(CompositionError::PreFlightFailed(e.to_string()));
            }
        }
    }

    Ok(PreFlightResult {
        approved_commands: approved,
        total_discovered,
        already_whitelisted,
        user_approved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::model::{
        ApprovedRuntimeCommand, HandlerTable, HarnessPlan, ValidationEvent, ValidationKind,
        ValidationPhase, ValidationRule, ValidationRuleId,
    };
    use std::path::PathBuf;

    fn empty_plan() -> HarnessPlan {
        HarnessPlan {
            source_path: PathBuf::from("/tmp/test.md"),
            timeout: None,
            pre_checks: Vec::new(),
            post_checks: Vec::new(),
            handlers: HandlerTable::default(),
            programmatic_handler: None,
        }
    }

    fn default_approval_options() -> ShellApprovalOptions {
        let dir = tempfile::TempDir::new().unwrap();
        ShellApprovalOptions {
            policy_root: Some(dir.into_path()),
            approval_handler: None,
            ..Default::default()
        }
    }

    #[test]
    fn empty_sources_returns_empty_approved_set() {
        let options = default_approval_options();
        let result = resolve_shell_approvals(None, None, None, &options).unwrap();
        assert!(result.approved_commands.is_empty());
        assert_eq!(result.total_discovered, 0);
    }

    #[test]
    fn discovers_commands_from_template() {
        let content = "# Test\n::shell echo hello\n";
        let md: Markdown = content.into();
        let compose_opts = ComposeOptions::new();
        let options = default_approval_options();

        // echo should pass built-in policy (not blacklisted, no whitelist needed)
        let result =
            resolve_shell_approvals(Some(&md), Some(&compose_opts), None, &options).unwrap();
        assert_eq!(result.total_discovered, 1);
        assert!(result.approved_commands.contains("echo hello"));
    }

    #[test]
    fn discovers_commands_from_harness_plan() {
        let mut plan = empty_plan();
        plan.pre_checks.push(ValidationRule {
            id: ValidationRuleId(0),
            event: ValidationEvent::ShellCommand,
            phase: ValidationPhase::Both,
            kind: ValidationKind::ShellCommand {
                command: ApprovedRuntimeCommand {
                    raw: "echo check".to_string(),
                    executable: "echo".to_string(),
                    args: vec!["check".to_string()],
                },
                show_stdout: false,
                show_stderr: false,
            },
            message_template: None,
            subject_key: None,
        });

        let options = default_approval_options();
        let result =
            resolve_shell_approvals(None, None, Some(&plan), &options).unwrap();
        assert_eq!(result.total_discovered, 1);
        assert!(result.approved_commands.contains("echo check"));
    }

    #[test]
    fn blacklisted_command_returns_error() {
        let content = "# Test\n::shell rm -rf /\n";
        let md: Markdown = content.into();
        let compose_opts = ComposeOptions::new();
        let options = default_approval_options();

        let err = resolve_shell_approvals(Some(&md), Some(&compose_opts), None, &options);
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("blacklisted"), "got: {msg}");
    }

    #[test]
    fn deduplicates_commands_across_sources() {
        let content = "# Test\n::shell echo hello\n";
        let md: Markdown = content.into();
        let compose_opts = ComposeOptions::new();

        let mut plan = empty_plan();
        plan.pre_checks.push(ValidationRule {
            id: ValidationRuleId(0),
            event: ValidationEvent::ShellCommand,
            phase: ValidationPhase::Both,
            kind: ValidationKind::ShellCommand {
                command: ApprovedRuntimeCommand {
                    raw: "echo hello".to_string(),
                    executable: "echo".to_string(),
                    args: vec!["hello".to_string()],
                },
                show_stdout: false,
                show_stderr: false,
            },
            message_template: None,
            subject_key: None,
        });

        let options = default_approval_options();
        let result = resolve_shell_approvals(
            Some(&md),
            Some(&compose_opts),
            Some(&plan),
            &options,
        )
        .unwrap();
        // Same command from both sources should count as 1
        assert_eq!(result.total_discovered, 1);
    }
}
```

**Note:** The `shell_lex` crate is used for splitting normalized command strings back into parts. If this crate is not available in the workspace, use `darkmatter::markdown::compose::shell_expansion::tokenize::tokenize` instead — adjust the split call accordingly. Check `Cargo.toml` during implementation.

- [ ] **Step 2: Export the preflight module**

In `claudine/lib/src/composition/mod.rs`, add:

```rust
pub mod preflight;
```

And add to the `pub use` block:

```rust
pub use preflight::{PreFlightResult, resolve_shell_approvals};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p claudine -- composition::preflight --nocapture`
Expected: All tests pass. Fix any import issues — the `shell_lex` split may need to be replaced with Darkmatter's tokenizer.

- [ ] **Step 4: Commit**

```bash
git add claudine/lib/src/composition/preflight.rs claudine/lib/src/composition/mod.rs
git commit -m "feat(claudine): add preflight module for pre-flight shell approval

- resolve_shell_approvals collects from template + harness sources
- deduplicates by normalized form
- checks whitelist, prompts user, aborts on denial
- returns full approved set for downstream use"
```

---

## Task 7: Wire Pre-Flight into Compose Commands

**Files:**
- Modify: `claudine/lib/src/composition/prepare.rs`
- Modify: `claudine/cli/src/commands/compose.rs`

- [ ] **Step 1: Update `prepare_direct` and `prepare_inline` to accept pre-approved commands**

In `claudine/lib/src/composition/prepare.rs`, modify `prepare_direct`:

```rust
pub fn prepare_direct(
    source: &ResolvedCompositionSource,
    set_overrides: Option<serde_json::Value>,
    pre_approved_commands: Option<std::collections::HashSet<String>>,
) -> Result<PreparedComposition, CompositionError> {
    let mut options = ComposeOptions::new().with_source_file(&source.resolved_path);
    if let Some(overrides) = set_overrides {
        options = options.with_set_overrides(overrides);
    }
    if let Some(approved) = pre_approved_commands {
        options = options.with_pre_approved_commands(approved);
    }
    let (composed, _report) = source
        .markdown
        .compose_with(options)
        .map_err(|e| CompositionError::ComposeFailed(e.to_string()))?;
```

Apply the same pattern to `prepare_inline` — add `pre_approved_commands: Option<HashSet<String>>` parameter and set it on the options.

- [ ] **Step 2: Update CLI compose commands to run pre-flight**

In `claudine/cli/src/commands/compose.rs`, modify `run_compose_inner` (around line 263). Add pre-flight between source resolution and prepare:

```rust
fn run_compose_inner(args: ComposeArgs, verbose: u8) -> Result<i32> {
    let excluded = parse_excluded(&args.exclude, args.silent || args.quiet);
    let explicit_provider = args.provider.resolve();
    let set_overrides = parse_set_json(args.set.as_deref())?;

    let source = composition::resolve_composition_source(&args.file).map_err(|e| eyre!("{e}"))?;

    // ── Pre-flight shell approval ────────────────────────────────────
    let compose_options = {
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new()
            .with_source_file(&source.resolved_path);
        if let Some(ref overrides) = set_overrides {
            opts = opts.with_set_overrides(overrides.clone());
        }
        opts
    };

    let approval_options = crate::commands::wrap::build_shell_approval_options(
        source.resolved_path.parent(),
    );

    let preflight = composition::resolve_shell_approvals(
        Some(&source.markdown),
        Some(&compose_options),
        None, // harness plan parsed later from effective frontmatter
        &approval_options,
    )
    .map_err(|e| eyre!("{e}"))?;

    let prepared = composition::prepare_direct(
        &source,
        set_overrides,
        Some(preflight.approved_commands),
    )
    .map_err(|e| eyre!("{e}"))?;
```

Apply the same pattern to `run_inline_compose_inner`.

**Note:** The `build_shell_approval_options` helper may need to be extracted from the wrap module or created. Check the existing code in `commands/wrap/mod.rs` for how `ShellApprovalOptions` is constructed — look for `build_harness_shell_options` or similar. During implementation, find the exact function name and import path.

- [ ] **Step 3: Fix compilation errors**

Run: `cargo check -p claudine-cli`

Fix any call sites that pass the old 2-argument form of `prepare_direct` or `prepare_inline`. Search for all callers:

Run (in editor/grep): search for `prepare_direct(` and `prepare_inline(` across the codebase

Update each call site to pass `None` for `pre_approved_commands` if it doesn't have a pre-flight result (e.g., in tests).

- [ ] **Step 4: Run tests**

Run: `cargo test -p claudine`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/composition/prepare.rs claudine/cli/src/commands/compose.rs
git commit -m "feat(claudine): wire pre-flight shell approval into compose commands

- prepare_direct and prepare_inline accept pre_approved_commands
- run_compose_inner runs pre-flight before composition
- run_inline_compose_inner runs pre-flight before composition
- pre-approved set passed through to Darkmatter compose"
```

---

## Task 8: Wire Pre-Flight into Wrapper Commands

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition.rs`
- Modify: `claudine/cli/src/commands/wrap/mod.rs`

The wrapper execution path (`execute_composition_request` and `run_provider_wrapper_inner`) also needs pre-flight. The harness loop already has shell audit logic — the pre-flight should run before it, and the existing audit should be updated to use the pre-approved set.

- [ ] **Step 1: Add pre-flight to `execute_composition_request`**

In `claudine/cli/src/commands/wrap/composition.rs`, the harness detection block (around line 414-462) parses the harness plan from effective frontmatter. The pre-flight for harness commands should run after the plan is parsed but before the provider launches.

After the harness plan is parsed and before the execution branch (around line 519), add:

```rust
    // ── Pre-flight shell approval for harness commands ───────────────
    if let Some(ref plan) = harness_plan {
        let harness_preflight = claudine::composition::resolve_shell_approvals(
            None,  // template commands already approved during compose
            None,
            Some(plan),
            &shell_approval_options,
        )
        .map_err(|e| eyre!("{e}"))?;

        // Merge harness approvals into the compose pre-approved set
        // so the harness runtime can check commands against it.
        // Store for use in run_harness_loop.
        harness_approved_commands = Some(harness_preflight.approved_commands);
    }
```

**Note:** The exact integration point depends on the current code structure. During implementation, read the full `execute_composition_request` function and find the right spot between harness plan parsing and provider launch.

- [ ] **Step 2: Add pre-flight to direct wrapper commands**

In `claudine/cli/src/commands/wrap/mod.rs`, the `run_provider_wrapper_inner` function handles direct wrapper commands like `claudine claude "prompt"`. If the prompt is a Markdown file with `::shell` directives or harness properties, pre-flight should run.

Find the prompt resolution point (where the prompt text is determined) and add pre-flight there. The exact location depends on the current code — search for where `MaterializedHarnessPrompt` or prompt text is built.

- [ ] **Step 3: Run full test suite**

Run: `cargo test -p claudine-cli`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/wrap/composition.rs claudine/cli/src/commands/wrap/mod.rs
git commit -m "feat(claudine): wire pre-flight into wrapper execution paths

- execute_composition_request runs pre-flight for harness commands
- run_provider_wrapper_inner runs pre-flight for direct wrapper commands
- pre-approved set available to harness runtime during execution"
```

---

## Task 9: Integration Test — End-to-End Pre-Flight

**Files:**
- Tests in existing test files or a new integration test

- [ ] **Step 1: Write integration test for pre-flight with compose**

This test verifies the full flow: template with `::shell` directives → pre-flight discovery → pre-approved set → compose succeeds without approval handler.

Add to `claudine/lib/src/composition/preflight.rs` tests:

```rust
    #[test]
    fn full_flow_template_with_whitelisted_commands() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "# Test\n::shell echo hello\n").unwrap();

        // Create whitelist with echo prefix
        let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
        std::fs::write(&whitelist_path, "prefix echo\n").unwrap();

        let md = Markdown::try_from(file_path.as_path()).unwrap();
        let compose_opts = ComposeOptions::new().with_source_file(&file_path);

        let options = ShellApprovalOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };

        // Pre-flight should discover and approve "echo hello"
        let result =
            resolve_shell_approvals(Some(&md), Some(&compose_opts), None, &options).unwrap();
        assert!(result.approved_commands.contains("echo hello"));

        // Now compose with the pre-approved set — should succeed
        let compose_with_approval = compose_opts
            .with_pre_approved_commands(result.approved_commands);
        let (composed, _) = md.compose_with(compose_with_approval).unwrap();
        assert!(composed.content().contains("hello"));
    }
```

- [ ] **Step 2: Write integration test for denied command**

```rust
    #[test]
    fn full_flow_blacklisted_command_aborts_preflight() {
        let content = "# Test\n::shell rm -rf /\n";
        let md: Markdown = content.into();
        let compose_opts = ComposeOptions::new();

        let temp_dir = tempfile::TempDir::new().unwrap();
        let options = ShellApprovalOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };

        let err = resolve_shell_approvals(Some(&md), Some(&compose_opts), None, &options);
        assert!(err.is_err());
        // Compose was never called — session never started
    }
```

- [ ] **Step 3: Run all tests**

Run: `cargo test -p claudine -- preflight --nocapture`
Expected: All pass

Run: `cargo test -p darkmatter -- discovery --nocapture`
Expected: All pass

- [ ] **Step 4: Run full workspace tests for both packages**

Run: `just test` (from darkmatter/ and claudine/ directories)
Expected: All pass

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/composition/preflight.rs
git commit -m "test(claudine): add integration tests for pre-flight shell approval

- full flow: discovery → approval → compose with pre-approved set
- blacklisted command aborts before compose
- deduplication across template and harness sources"
```

---

## Task 10: Update Documentation

**Files:**
- Modify: `claudine/docs/topics/composition.md`
- Modify: `claudine/docs/topics/validations-and-handlers.md`

- [ ] **Step 1: Add pre-flight section to composition.md**

In `claudine/docs/topics/composition.md`, in the Architecture section (around line 163), add a reference to the pre-flight step in the pipeline:

```markdown
Both commands follow the same six-stage pipeline:

```
Resolve → Pre-Flight → Prepare → Select Provider → Launch → Closure
```

- **Resolve**: `composition::resolve_composition_source()` loads the Markdown file
- **Pre-Flight**: `composition::resolve_shell_approvals()` discovers all `::shell` commands in the document graph and harness plan, checks whitelists, and prompts the user to approve any unapproved commands before proceeding (see [Pre-Flight Shell Approval](pre-flight-checks.md))
- **Prepare**: `composition::prepare_direct()` or `composition::prepare_inline()` composes through Darkmatter with the pre-approved command set
```

Update the existing description to match (replace the old five-stage pipeline text).

- [ ] **Step 2: Update shell policy section in composition.md**

Replace the "Shell Policy" section (around line 141-143) with a reference to the pre-flight doc:

```markdown
### Shell Policy

Shell commands in `::shell` directives, `shell_command` validations, and `deviate`/`handle` declarations are all approved upfront during the pre-flight phase — before the provider session starts. See [Pre-Flight Shell Approval](pre-flight-checks.md) for the full flow.
```

- [ ] **Step 3: Add pre-flight reference to validations-and-handlers.md**

In the "Before The Provider Starts" section (around line 47-51), add a note about pre-flight:

```markdown
Before the pre-check phase, Claudine runs a pre-flight scan that discovers and approves all shell commands that might be executed during the session — including those in pre-checks, post-checks, and handlers. See [Pre-Flight Shell Approval](pre-flight-checks.md) for details.
```

- [ ] **Step 4: Commit**

```bash
git add claudine/docs/topics/composition.md claudine/docs/topics/validations-and-handlers.md
git commit -m "docs(claudine): add pre-flight references to composition and validation docs

- update pipeline from 5-stage to 6-stage (adds Pre-Flight step)
- update shell policy section to reference pre-flight doc
- add pre-flight note to validations-and-handlers"
```
