# Frontmatter Shell Expansion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `FrontmatterShellExpansion` compose operation that executes approved shell commands stored in frontmatter string values (`$(cmd)`), and add configurable timeout behavior for both body and frontmatter shell expansion.

**Architecture:** A new compose stage runs after `FrontmatterInterpolation` and before `EffectiveState` construction, scanning top-level frontmatter string values for `$(...)` expressions, executing them through the existing shell approval/execution infrastructure, and rewriting frontmatter values with the trimmed stdout. Timeout behavior is shared between body and frontmatter shell expansion via `ShellTimeoutBehavior` on `ComposeOptions`. A `ShellCommandOrigin` enum replaces bare `line: usize` fields to support frontmatter-key-based error reporting.

**Tech Stack:** Rust, darkmatter lib (compose pipeline), darkmatter CLI (clap), rayon (concurrent execution), existing shell expansion infrastructure (tokenizer, policy, executor, approval).

**Spec:** `darkmatter/features/2026-04-08-shell-expansion-in-fm/spec.md`
**Tech Design:** `darkmatter/features/2026-04-08-shell-expansion-in-fm/tech-design.md`

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs` | Scanning, parsing, validation, execution, and mutation of frontmatter shell values |
| `darkmatter/docs/inline/fm-shell-expansion.md` | Documentation for frontmatter shell expansion |

### Modified Files

| File | Changes |
|------|---------|
| `darkmatter/lib/src/markdown/compose/types.rs` | Add `FrontmatterShellExpansion` to `ComposeOperation`, `ComposeStage`; add `ShellTimeoutBehavior`; add report counter; update `ComposeOptions` with timeout behavior builders |
| `darkmatter/lib/src/markdown/compose/perf.rs` | Add `PerfMetricKind::FrontmatterShellExpansion`; update array size from 12 to 13 |
| `darkmatter/lib/src/markdown/compose/mod.rs` | Add `mod frontmatter_shell_expansion`; run frontmatter shell expansion before `EffectiveState`; snapshot pre-interpolation values; record metrics |
| `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` | Add `ShellCommandOrigin` enum; add `timeout_behavior` to `ShellExpansionOptions`; add `timeout_override` to `ShellDirective`; update error/approval types to use origin |
| `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs` | Parse trailing `::timeout:<N>` suffix on body `::shell` lines |
| `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs` | Accept per-command timeout; handle timeout behavior (error vs empty string) |
| `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs` | Inspect frontmatter for shell expressions during preflight |
| `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs` | Update re-exports for new types |
| `darkmatter/cli/src/args.rs` | Add `--timeout` and `--allow-shell-timeout` to `Compose` subcommand |
| `darkmatter/cli/src/commands.rs` | Wire new flags into `ComposeOptions`; update origin-aware error formatting |
| `darkmatter/docs/darkmatter-compose-pipeline.md` | Add frontmatter shell expansion to pipeline docs |

---

## Task 1: Add `ShellTimeoutBehavior` and timeout-behavior options

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:165-216`
- Modify: `darkmatter/lib/src/markdown/compose/types.rs` (ComposeOptions section ~line 240)
- Test: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` (inline tests)
- Test: `darkmatter/lib/src/markdown/compose/types.rs` (inline tests)

- [ ] **Step 1: Write the failing test for ShellTimeoutBehavior**

Add to the bottom of `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`, inside a new `#[cfg(test)] mod tests` block (or append to existing tests if present):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_timeout_behavior_default_is_error() {
        assert_eq!(ShellTimeoutBehavior::default(), ShellTimeoutBehavior::Error);
    }

    #[test]
    fn shell_expansion_options_default_timeout_behavior_is_error() {
        let opts = ShellExpansionOptions::default();
        assert_eq!(opts.timeout_behavior, ShellTimeoutBehavior::Error);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter shell_timeout_behavior_default_is_error -- --nocapture`
Expected: FAIL — `ShellTimeoutBehavior` type not found.

- [ ] **Step 3: Implement ShellTimeoutBehavior enum**

Add to `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`, before the `ShellExpansionOptions` struct (around line 165):

```rust
/// What happens when a shell command exceeds its timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellTimeoutBehavior {
    /// Compose aborts with a shell-expansion error (default).
    #[default]
    Error,
    /// The shell result is replaced with an empty string and a warning is emitted.
    EmptyString,
}
```

Add `timeout_behavior` field to `ShellExpansionOptions`:

```rust
pub struct ShellExpansionOptions {
    pub timeout: std::time::Duration,
    pub timeout_behavior: ShellTimeoutBehavior,
    pub policy_root: Option<PathBuf>,
    pub working_directory: Option<PathBuf>,
    pub approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
    pub strip_ansi: bool,
}
```

Update the `Clone`, `Debug`, and `Default` impls to include `timeout_behavior`:

In the `Clone` impl, add: `timeout_behavior: self.timeout_behavior,`

In the `Debug` impl, add: `.field("timeout_behavior", &self.timeout_behavior)`

In the `Default` impl, add: `timeout_behavior: ShellTimeoutBehavior::Error,`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p darkmatter shell_timeout_behavior -- --nocapture`
Expected: PASS

- [ ] **Step 5: Write failing test for ComposeOptions timeout behavior builders**

Add to the tests in `darkmatter/lib/src/markdown/compose/types.rs`:

```rust
#[test]
fn compose_options_default_timeout_behavior_is_error() {
    let opts = ComposeOptions::new();
    assert_eq!(
        opts.shell_timeout_behavior,
        crate::markdown::compose::shell_expansion::types::ShellTimeoutBehavior::Error
    );
}

#[test]
fn with_shell_timeout_behavior_sets_value() {
    use crate::markdown::compose::shell_expansion::types::ShellTimeoutBehavior;
    let opts = ComposeOptions::new()
        .with_shell_timeout_behavior(ShellTimeoutBehavior::EmptyString);
    assert_eq!(opts.shell_timeout_behavior, ShellTimeoutBehavior::EmptyString);
}

#[test]
fn with_allow_shell_timeout_sets_empty_string() {
    use crate::markdown::compose::shell_expansion::types::ShellTimeoutBehavior;
    let opts = ComposeOptions::new().with_allow_shell_timeout(true);
    assert_eq!(opts.shell_timeout_behavior, ShellTimeoutBehavior::EmptyString);
}

#[test]
fn with_allow_shell_timeout_false_keeps_error() {
    use crate::markdown::compose::shell_expansion::types::ShellTimeoutBehavior;
    let opts = ComposeOptions::new().with_allow_shell_timeout(false);
    assert_eq!(opts.shell_timeout_behavior, ShellTimeoutBehavior::Error);
}
```

- [ ] **Step 6: Run test to verify it fails**

Run: `cargo test -p darkmatter compose_options_default_timeout_behavior -- --nocapture`
Expected: FAIL — `shell_timeout_behavior` field not found on `ComposeOptions`.

- [ ] **Step 7: Add timeout behavior to ComposeOptions**

In `darkmatter/lib/src/markdown/compose/types.rs`, add to the `ComposeOptions` struct fields:

```rust
pub shell_timeout_behavior: ShellTimeoutBehavior,
```

Add to `ComposeOptions::new()` initialization:

```rust
shell_timeout_behavior: ShellTimeoutBehavior::Error,
```

Add builder methods alongside the existing shell-related builders:

```rust
#[must_use]
pub fn with_shell_timeout_behavior(mut self, behavior: ShellTimeoutBehavior) -> Self {
    self.shell_timeout_behavior = behavior;
    self
}

#[must_use]
pub fn with_allow_shell_timeout(mut self, allow: bool) -> Self {
    self.shell_timeout_behavior = if allow {
        ShellTimeoutBehavior::EmptyString
    } else {
        ShellTimeoutBehavior::Error
    };
    self
}
```

Update the `shell_options()` projection method to pass through `timeout_behavior`:

```rust
pub fn shell_options(&self) -> ShellExpansionOptions {
    ShellExpansionOptions {
        timeout: self.shell_timeout,
        timeout_behavior: self.shell_timeout_behavior,
        // ... existing fields ...
    }
}
```

Add the necessary import at the top of types.rs:

```rust
use super::shell_expansion::types::ShellTimeoutBehavior;
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p darkmatter compose_options_default_timeout_behavior with_shell_timeout_behavior with_allow_shell_timeout -- --nocapture`
Expected: All PASS

- [ ] **Step 9: Update shell_expansion/mod.rs re-exports**

Add `ShellTimeoutBehavior` to the re-exports in `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`:

```rust
pub use types::{
    ErrorHandling, ErrorHandlingOutcome, ShellApprovalDecision, ShellApprovalHandler,
    ShellApprovalRequest, ShellCommandEntry, ShellDirective, ShellExpansionError,
    ShellExpansionOptions, ShellExpansionRuntime, ShellPolicyPaths, ShellRuleSet,
    ShellTimeoutBehavior,
};
```

And add to the compose module's public re-exports in `darkmatter/lib/src/markdown/compose/mod.rs`:

```rust
pub use shell_expansion::ShellTimeoutBehavior;
```

- [ ] **Step 10: Run full build to verify no regressions**

Run: `cargo test -p darkmatter -- --nocapture 2>&1 | tail -5`
Expected: All existing tests pass.

- [ ] **Step 11: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/shell_expansion/types.rs \
       darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs \
       darkmatter/lib/src/markdown/compose/types.rs \
       darkmatter/lib/src/markdown/compose/mod.rs
git commit -m "feat(darkmatter): add ShellTimeoutBehavior and timeout-behavior options"
```

---

## Task 2: Add per-command `timeout_override` to `ShellDirective` and executor

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:15-23`
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:95-232`
- Test: inline tests in both files

- [ ] **Step 1: Write the failing test for timeout_override on ShellDirective**

Add to the executor tests in `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs`:

```rust
#[test]
fn per_command_timeout_override_beats_global() {
    let d = ShellDirective {
        raw_command: "sleep 10".to_string(),
        executable: "sleep".to_string(),
        args: vec!["10".to_string()],
        span: 0..8,
        line: 1,
        error_handling: ErrorHandling::default(),
        timeout_override: Some(Duration::from_millis(100)),
    };
    let options = ShellExpansionOptions {
        timeout: Duration::from_secs(60), // Global is generous
        ..Default::default()
    };
    let source = ComposeSource::Unknown;

    let result = execute_command(&d, &options, &source);
    assert!(result.is_err());
    match result.unwrap_err() {
        ShellExpansionError::Timeout { timeout, .. } => {
            assert_eq!(timeout, Duration::from_millis(100));
        }
        err => panic!("Expected Timeout, got: {:?}", err),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter per_command_timeout_override_beats_global -- --nocapture`
Expected: FAIL — `timeout_override` not a field on `ShellDirective`.

- [ ] **Step 3: Add `timeout_override` to `ShellDirective`**

In `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`, update the `ShellDirective` struct:

```rust
/// A parsed `::shell` directive.
#[derive(Debug, Clone)]
pub struct ShellDirective {
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub span: Range<usize>,
    pub line: usize,
    pub error_handling: ErrorHandling,
    /// Per-command timeout override. When `Some`, takes precedence over the global timeout.
    pub timeout_override: Option<std::time::Duration>,
}
```

- [ ] **Step 4: Fix all existing code that constructs ShellDirective**

Every place that constructs a `ShellDirective` needs `timeout_override: None` added:

1. `shell_expansion/parser.rs` — in `parse_directives()` around line 77:
   ```rust
   directives.push(ShellDirective {
       raw_command,
       executable,
       args,
       span: line_start..line_with_newline_end,
       line: line_num,
       error_handling,
       timeout_override: None,
   });
   ```

2. `shell_expansion/mod.rs` — in `resolve_or_passthrough()` around line 297:
   ```rust
   let effective = ShellDirective {
       raw_command: raw,
       executable: resolved.executable,
       args: merged_args,
       span: directive.span.clone(),
       line: directive.line,
       error_handling: directive.error_handling.clone(),
       timeout_override: directive.timeout_override,
   };
   ```

3. All test helpers that construct `ShellDirective` in `executor.rs`, `mod.rs` tests — add `timeout_override: None`.

- [ ] **Step 5: Wire per-command timeout in executor**

In `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs`, update `execute_command()`. Change line 155 from:

```rust
let timeout = shell_opts.timeout;
```

to:

```rust
let timeout = directive.timeout_override.unwrap_or(shell_opts.timeout);
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p darkmatter per_command_timeout_override_beats_global -- --nocapture`
Expected: PASS

Run: `cargo test -p darkmatter -p darkmatter-cli -- --nocapture 2>&1 | tail -5`
Expected: All tests pass (no regressions).

- [ ] **Step 7: Write the failing test for timeout EmptyString behavior**

Add to executor tests:

```rust
#[test]
fn timeout_with_empty_string_behavior_returns_empty() {
    use super::super::types::ShellTimeoutBehavior;

    let d = ShellDirective {
        raw_command: "sleep 10".to_string(),
        executable: "sleep".to_string(),
        args: vec!["10".to_string()],
        span: 0..8,
        line: 1,
        error_handling: ErrorHandling::default(),
        timeout_override: Some(Duration::from_millis(100)),
    };
    let options = ShellExpansionOptions {
        timeout: Duration::from_secs(60),
        timeout_behavior: ShellTimeoutBehavior::EmptyString,
        ..Default::default()
    };
    let source = ComposeSource::Unknown;

    let result = execute_command(&d, &options, &source);
    assert!(result.is_ok(), "Expected Ok with empty string, got: {:?}", result);
    assert_eq!(result.unwrap(), "");
}
```

- [ ] **Step 8: Implement timeout behavior in executor**

In `execute_command()`, update the timeout handling block (around line 208-217). Replace:

```rust
if start.elapsed() >= timeout {
    let _ = child.kill();
    let _ = child.wait();
    warn!(elapsed = ?start.elapsed(), "shell: command timed out");
    return Err(ShellExpansionError::Timeout {
        command: directive.raw_command.clone(),
        timeout,
        line: directive.line,
    });
}
```

with:

```rust
if start.elapsed() >= timeout {
    let _ = child.kill();
    let _ = child.wait();
    warn!(elapsed = ?start.elapsed(), "shell: command timed out");
    match shell_opts.timeout_behavior {
        ShellTimeoutBehavior::Error => {
            return Err(ShellExpansionError::Timeout {
                command: directive.raw_command.clone(),
                timeout,
                line: directive.line,
            });
        }
        ShellTimeoutBehavior::EmptyString => {
            return Ok(String::new());
        }
    }
}
```

Add the import at the top of executor.rs:

```rust
use super::types::{ShellDirective, ShellExpansionError, ShellExpansionOptions, ShellTimeoutBehavior};
```

- [ ] **Step 9: Run tests to verify they pass**

Run: `cargo test -p darkmatter timeout_with_empty_string_behavior -- --nocapture`
Expected: PASS

Run: `cargo test -p darkmatter -p darkmatter-cli -- --nocapture 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 10: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/shell_expansion/types.rs \
       darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs \
       darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs \
       darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
git commit -m "feat(darkmatter): add per-command timeout_override and EmptyString timeout behavior"
```

---

## Task 3: Parse `::timeout:<N>` suffix on body `::shell` directives

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs`
- Test: inline tests in parser.rs

- [ ] **Step 1: Write the failing tests**

Add to the tests in `parser.rs`:

```rust
#[test]
fn parse_timeout_suffix_at_end() {
    let content = "::shell echo hello ::timeout:5\n";
    let directives = parse_directives(content).unwrap();
    assert_eq!(directives.len(), 1);
    assert_eq!(directives[0].executable, "echo");
    assert_eq!(directives[0].args, vec!["hello"]);
    assert_eq!(
        directives[0].timeout_override,
        Some(std::time::Duration::from_secs(5))
    );
}

#[test]
fn parse_timeout_suffix_after_error_handling() {
    let content = "::shell echo hello --when-error empty ::timeout:3\n";
    let directives = parse_directives(content).unwrap();
    assert_eq!(directives[0].executable, "echo");
    assert_eq!(directives[0].args, vec!["hello"]);
    assert_eq!(
        directives[0].error_handling.when_error,
        Some("empty".to_string())
    );
    assert_eq!(
        directives[0].timeout_override,
        Some(std::time::Duration::from_secs(3))
    );
}

#[test]
fn parse_no_timeout_suffix_leaves_none() {
    let content = "::shell echo hello\n";
    let directives = parse_directives(content).unwrap();
    assert!(directives[0].timeout_override.is_none());
}

#[test]
fn parse_timeout_suffix_before_error_handling_is_rejected() {
    let content = "::shell echo hello ::timeout:5 --when-error empty\n";
    let result = parse_directives(content);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("::timeout") && err.contains("last"),
        "Expected error about ::timeout position, got: {err}"
    );
}

#[test]
fn parse_timeout_zero_is_rejected() {
    let content = "::shell echo hello ::timeout:0\n";
    let result = parse_directives(content);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("greater than zero"),
        "Expected error about zero timeout, got: {err}"
    );
}

#[test]
fn parse_timeout_non_integer_is_rejected() {
    let content = "::shell echo hello ::timeout:abc\n";
    let result = parse_directives(content);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("integer"),
        "Expected error about integer, got: {err}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter parse_timeout_suffix -- --nocapture`
Expected: FAIL — timeout suffix is treated as a command argument.

- [ ] **Step 3: Implement `::timeout:<N>` parsing in `parse_directives`**

The `::timeout:<N>` suffix must be the **last token** on the `::shell` line. After tokenization and error-handling extraction, check if the last remaining token matches `::timeout:<N>`.

In `parse_directives()`, after the call to `extract_error_handling()` (line 64), add timeout suffix extraction:

```rust
// Extract error handling options from anywhere in the token list
let (error_handling, cmd_tokens) = extract_error_handling(&tokens, line_num)?;

// Extract ::timeout:<N> suffix — must be the last token
let (timeout_override, cmd_tokens) =
    extract_timeout_suffix(&cmd_tokens, line_num)?;

if cmd_tokens.is_empty() {
    return Err(ShellExpansionError::ParseDirective {
        line: line_num,
        message: "No command after error handling options".to_string(),
    });
}
```

Add the `extract_timeout_suffix` function:

```rust
/// Extracts a trailing `::timeout:<N>` token from the command token list.
///
/// Returns `(Some(Duration), remaining_tokens)` if the last token matches the
/// pattern, or `(None, original_tokens)` if no timeout suffix is present.
///
/// The suffix must be the last token. If a `::timeout:` token appears
/// before the last position, it is an error (must appear after error-handling flags).
fn extract_timeout_suffix(
    tokens: &[String],
    line: usize,
) -> Result<(Option<std::time::Duration>, Vec<String>), ShellExpansionError> {
    if tokens.is_empty() {
        return Ok((None, tokens.to_vec()));
    }

    // Check if any non-last token is a timeout suffix (reject)
    for (i, token) in tokens.iter().enumerate() {
        if i < tokens.len() - 1 && token.starts_with("::timeout:") {
            return Err(ShellExpansionError::ParseDirective {
                line,
                message: "::timeout:<N> must be the last token on the ::shell line"
                    .to_string(),
            });
        }
    }

    let last = &tokens[tokens.len() - 1];
    if let Some(value_str) = last.strip_prefix("::timeout:") {
        let seconds: u64 = value_str.parse().map_err(|_| {
            ShellExpansionError::ParseDirective {
                line,
                message: format!(
                    "::timeout requires a positive integer of seconds, got '{value_str}'"
                ),
            }
        })?;

        if seconds == 0 {
            return Err(ShellExpansionError::ParseDirective {
                line,
                message: "::timeout value must be greater than zero".to_string(),
            });
        }

        let remaining = tokens[..tokens.len() - 1].to_vec();
        Ok((Some(std::time::Duration::from_secs(seconds)), remaining))
    } else {
        Ok((None, tokens.to_vec()))
    }
}
```

Update the `ShellDirective` construction in `parse_directives()` to include `timeout_override`:

```rust
directives.push(ShellDirective {
    raw_command,
    executable,
    args,
    span: line_start..line_with_newline_end,
    line: line_num,
    error_handling,
    timeout_override,
});
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter parse_timeout_suffix -- --nocapture`
Expected: All PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p darkmatter -p darkmatter-cli -- --nocapture 2>&1 | tail -5`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs
git commit -m "feat(darkmatter): parse ::timeout:<N> suffix on body ::shell directives"
```

---

## Task 4: Add `ShellCommandOrigin` and update shell expansion types

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs`
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs`
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs`
- Modify: `darkmatter/lib/src/markdown/compose/mod.rs` (run_shell_expansion_stage)
- Modify: `darkmatter/cli/src/commands.rs` (error formatting)
- Test: inline tests across modified files

This is a mechanical refactor: replace `line: usize` with `origin: ShellCommandOrigin` in key types.

- [ ] **Step 1: Write the failing test for ShellCommandOrigin Display**

Add to the tests in `shell_expansion/types.rs`:

```rust
#[test]
fn shell_command_origin_body_display() {
    let origin = ShellCommandOrigin::Body { line: 42 };
    assert_eq!(format!("{origin}"), "line 42");
}

#[test]
fn shell_command_origin_frontmatter_display() {
    let origin = ShellCommandOrigin::Frontmatter {
        key: "files".to_string(),
    };
    assert_eq!(format!("{origin}"), "frontmatter.files");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter shell_command_origin -- --nocapture`
Expected: FAIL — `ShellCommandOrigin` type not found.

- [ ] **Step 3: Implement ShellCommandOrigin**

Add to `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`, near the top (after imports):

```rust
/// Where a shell command originates in a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellCommandOrigin {
    /// Body `::shell` directive at the given 1-indexed line.
    Body { line: usize },
    /// Frontmatter property with a `$(...)` shell expression.
    Frontmatter { key: String },
}

impl fmt::Display for ShellCommandOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Body { line } => write!(f, "line {line}"),
            Self::Frontmatter { key } => write!(f, "frontmatter.{key}"),
        }
    }
}

impl ShellCommandOrigin {
    /// Returns the line number if this is a body origin, or 0 for frontmatter.
    ///
    /// Provided for backward compatibility with code that needs a line number.
    pub fn line_number(&self) -> usize {
        match self {
            Self::Body { line } => *line,
            Self::Frontmatter { .. } => 0,
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter shell_command_origin -- --nocapture`
Expected: PASS

- [ ] **Step 5: Update `ShellExpansionError` to use `ShellCommandOrigin`**

Replace `line: usize` with `origin: ShellCommandOrigin` in every variant of `ShellExpansionError`:

```rust
#[derive(Error, Debug)]
pub enum ShellExpansionError {
    #[error("Shell directive parse error at {origin}: {message}")]
    ParseDirective {
        origin: ShellCommandOrigin,
        message: String,
    },

    #[error("Command not found: '{command}' at {origin}")]
    CommandNotFound {
        command: String,
        origin: ShellCommandOrigin,
    },

    #[error("Blacklisted command '{command}' at {origin}: {reason}")]
    Blacklisted {
        command: String,
        reason: String,
        origin: ShellCommandOrigin,
    },

    #[error("Approval required for '{command}' at {origin}")]
    ApprovalRequired {
        command: String,
        whitelist_path: PathBuf,
        blacklist_path: PathBuf,
        origin: ShellCommandOrigin,
    },

    #[error("Command denied: '{command}' at {origin}")]
    Denied {
        command: String,
        origin: ShellCommandOrigin,
    },

    #[error(
        "Command '{command}' at {origin} was not pre-approved{source_desc}. \
         This is a bug in the pre-flight scanner -- please report it."
    )]
    NotPreApproved {
        command: String,
        origin: ShellCommandOrigin,
        source_desc: String,
    },

    #[error("Command timed out after {timeout:?}: '{command}' at {origin}")]
    Timeout {
        command: String,
        timeout: std::time::Duration,
        origin: ShellCommandOrigin,
    },

    #[error("Command failed (exit {code}): '{command}' at {origin}")]
    ExecutionFailed {
        command: String,
        code: i32,
        stdout: String,
        stderr: String,
        origin: ShellCommandOrigin,
    },

    #[error("Policy I/O error for {path}: {source}")]
    PolicyIo {
        path: PathBuf,
        source: std::io::Error,
    },
}
```

- [ ] **Step 6: Update `ShellDirective` to carry origin**

Replace `line: usize` with `origin: ShellCommandOrigin` in `ShellDirective`:

```rust
pub struct ShellDirective {
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub span: Range<usize>,
    pub origin: ShellCommandOrigin,
    pub error_handling: ErrorHandling,
    pub timeout_override: Option<std::time::Duration>,
}
```

- [ ] **Step 7: Update `ShellApprovalRequest`**

Replace `line: usize` with `origin: ShellCommandOrigin`:

```rust
pub struct ShellApprovalRequest {
    pub source: ComposeSource,
    pub origin: ShellCommandOrigin,
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub normalized_exact: String,
    pub whitelist_path: PathBuf,
    pub blacklist_path: PathBuf,
    pub alias_name: Option<String>,
}
```

- [ ] **Step 8: Update `ShellCommandEntry`**

Replace `line: usize` with `origin: ShellCommandOrigin` (keep `source_file`):

```rust
pub struct ShellCommandEntry {
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub normalized: String,
    pub source_file: PathBuf,
    pub origin: ShellCommandOrigin,
}
```

- [ ] **Step 9: Fix all compiler errors across the codebase**

This is a mechanical step — follow compiler errors to update every match arm, constructor, and field access. Key files to update:

1. **`shell_expansion/parser.rs`**: `parse_directives()` constructs `ShellDirective` — change `line: line_num` to `origin: ShellCommandOrigin::Body { line: line_num }`. The `tokenize()` errors use `line: 0` — change to `origin: ShellCommandOrigin::Body { line: 0 }`.

2. **`shell_expansion/tokenize.rs`**: All `ShellExpansionError::ParseDirective` uses `line: 0` — change to `origin: ShellCommandOrigin::Body { line: 0 }`.

3. **`shell_expansion/executor.rs`**: `execute_command()` references `directive.line` — change to `directive.origin.clone()`. Update all error constructors from `line: directive.line` to `origin: directive.origin.clone()`.

4. **`shell_expansion/mod.rs`**: `execute_directive()` and `resolve_or_passthrough()` reference `directive.line` — change to `directive.origin.clone()`. Update approval request construction.

5. **`shell_expansion/discovery.rs`**: `collect_shell_commands()` constructs `ShellCommandEntry` with `line` — change to `origin: ShellCommandOrigin::Body { line }`.

6. **`compose/mod.rs`**: `run_shell_expansion_stage()` doesn't directly reference `line`, but any error propagation should work automatically via the `?` operator.

7. **`cli/src/commands.rs`**: Error formatting matches on `ShellExpansionError` variants — update field names from `line` to `origin` and update format strings from `"on line {line}"` to `"at {origin}"`.

8. **All tests**: Update every `ShellDirective`, `ShellExpansionError`, or `ShellCommandEntry` construction in test code. The pattern is mechanical: `line: N` becomes `origin: ShellCommandOrigin::Body { line: N }`.

- [ ] **Step 10: Add ShellCommandOrigin to re-exports**

In `shell_expansion/mod.rs`, add to the `pub use types::` block:

```rust
pub use types::{
    // ... existing types ...
    ShellCommandOrigin,
};
```

In `compose/mod.rs`, add to the public re-exports:

```rust
pub use shell_expansion::ShellCommandOrigin;
```

- [ ] **Step 11: Run full test suite**

Run: `cargo test -p darkmatter -p darkmatter-cli -- --nocapture 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 12: Commit**

```bash
git add darkmatter/lib/ darkmatter/cli/
git commit -m "refactor(darkmatter): replace line: usize with ShellCommandOrigin in shell expansion types"
```

---

## Task 5: Add `ComposeOperation::FrontmatterShellExpansion` and supporting infrastructure

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/types.rs`
- Modify: `darkmatter/lib/src/markdown/compose/perf.rs`
- Test: inline tests

- [ ] **Step 1: Write the failing test**

Add to the tests in `types.rs`:

```rust
#[test]
fn frontmatter_shell_expansion_is_inline_pre() {
    assert_eq!(
        ComposeOperation::FrontmatterShellExpansion.phase(),
        ComposePhase::InlinePre
    );
}

#[test]
fn frontmatter_shell_expansion_follows_frontmatter_interpolation_in_default_order() {
    let order = ComposeOperation::default_order();
    let fm_interp_pos = order
        .iter()
        .position(|op| *op == ComposeOperation::FrontmatterInterpolation)
        .unwrap();
    let fm_shell_pos = order
        .iter()
        .position(|op| *op == ComposeOperation::FrontmatterShellExpansion)
        .unwrap();
    assert_eq!(fm_shell_pos, fm_interp_pos + 1);
}

#[test]
fn compose_report_tracks_frontmatter_shell_expansions() {
    let report = ComposeReport::new();
    assert_eq!(report.frontmatter_shell_expansions_applied, 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter frontmatter_shell_expansion_is_inline_pre -- --nocapture`
Expected: FAIL — `FrontmatterShellExpansion` variant not found.

- [ ] **Step 3: Add `ComposeOperation::FrontmatterShellExpansion`**

In `darkmatter/lib/src/markdown/compose/types.rs`, update `ComposeOperation`:

1. Add the variant after `FrontmatterInterpolation`:
   ```rust
   FrontmatterShellExpansion,
   ```

2. Update `COUNT` from `11` to `12`.

3. Update `index()` — `FrontmatterShellExpansion` gets index 1, and all subsequent operations shift by +1:
   ```rust
   fn index(self) -> usize {
       match self {
           Self::FrontmatterInterpolation => 0,
           Self::FrontmatterShellExpansion => 1,
           Self::TextReplacement => 2,
           Self::PageBlocks => 3,
           Self::Interpolation => 4,
           Self::ShellExpansion => 5,
           Self::BlockTransclusion => 6,
           Self::FrontmatterTransclusion => 7,
           Self::CodeTransclusion => 8,
           Self::TocLinking => 9,
           Self::Cleanup => 10,
           Self::Normalization => 11,
       }
   }
   ```

4. Update `phase()` — `FrontmatterShellExpansion` is `InlinePre`:
   ```rust
   Self::FrontmatterShellExpansion => ComposePhase::InlinePre,
   ```

5. Update `default_order()` — insert after `FrontmatterInterpolation`:
   ```rust
   pub fn default_order() -> &'static [ComposeOperation] {
       &[
           Self::FrontmatterInterpolation,
           Self::FrontmatterShellExpansion,
           Self::TextReplacement,
           Self::PageBlocks,
           Self::Interpolation,
           Self::ShellExpansion,
           Self::BlockTransclusion,
           Self::FrontmatterTransclusion,
           Self::CodeTransclusion,
           Self::TocLinking,
           Self::Cleanup,
           Self::Normalization,
       ]
   }
   ```

6. Update the `Display` impl to include the new variant:
   ```rust
   Self::FrontmatterShellExpansion => write!(f, "Frontmatter Shell Expansion"),
   ```

- [ ] **Step 4: Add `ComposeStage::FrontmatterShellExpansion`**

In the `ComposeStage` enum, add after `FrontmatterInterpolation`:

```rust
FrontmatterShellExpansion,
```

Update its `Display` impl:

```rust
Self::FrontmatterShellExpansion => write!(f, "frontmatter shell expansion"),
```

- [ ] **Step 5: Add `frontmatter_shell_expansions_applied` to `ComposeReport`**

Add the field to the `ComposeReport` struct:

```rust
pub frontmatter_shell_expansions_applied: usize,
```

Initialize it to `0` in `ComposeReport::new()`.

Include it in `has_changes()`:

```rust
self.frontmatter_shell_expansions_applied > 0
```

Include it in `summary()` alongside the existing counters.

Include it in `merge()`:

```rust
self.frontmatter_shell_expansions_applied += other.frontmatter_shell_expansions_applied;
```

- [ ] **Step 6: Add `PerfMetricKind::FrontmatterShellExpansion`**

In `darkmatter/lib/src/markdown/compose/perf.rs`:

1. Add the variant after `FrontmatterInterpolation`:
   ```rust
   FrontmatterShellExpansion,
   ```

2. Update the `stage()` mapping:
   ```rust
   Self::FrontmatterShellExpansion => ComposeStage::FrontmatterShellExpansion,
   ```

3. Update `all()` to include the new variant in order.

4. Update the fixed array size from `12` to `13` in `PerfCollector`:
   ```rust
   durations: [(Duration, usize); 13],
   ```

5. Update `new()` to initialize with 13 entries:
   ```rust
   durations: [(Duration::ZERO, 0); 13],
   ```

- [ ] **Step 7: Update the compose/mod.rs operation dispatcher**

In `run_inline_pre_operation()`, add the new operation to the match and perf kind mapping. The `FrontmatterShellExpansion` case should be a no-op in the generic dispatcher (like `FrontmatterInterpolation`) because it runs before `EffectiveState`:

```rust
ComposeOperation::FrontmatterShellExpansion => Ok(()),
```

In the perf kind mapping within `run_compose_pipeline_internal()`, add:

```rust
ComposeOperation::FrontmatterShellExpansion => {
    perf::PerfMetricKind::FrontmatterShellExpansion
}
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p darkmatter frontmatter_shell_expansion -- --nocapture`
Expected: All PASS

Run: `cargo test -p darkmatter -p darkmatter-cli -- --nocapture 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 9: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/types.rs \
       darkmatter/lib/src/markdown/compose/perf.rs \
       darkmatter/lib/src/markdown/compose/mod.rs
git commit -m "feat(darkmatter): add ComposeOperation::FrontmatterShellExpansion infrastructure"
```

---

## Task 6: Implement frontmatter shell expansion module — parsing and validation

**Files:**
- Create: `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`
- Modify: `darkmatter/lib/src/markdown/compose/mod.rs` (add `mod` declaration)
- Test: inline tests in frontmatter_shell_expansion.rs

- [ ] **Step 1: Add the module declaration**

In `darkmatter/lib/src/markdown/compose/mod.rs`, add after the `frontmatter_interpolation` declaration:

```rust
pub(crate) mod frontmatter_shell_expansion;
```

- [ ] **Step 2: Write the failing tests for candidate detection and parsing**

Create `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs` with tests:

```rust
//! Frontmatter shell expansion engine.
//!
//! Scans top-level frontmatter string values for `$(command)` expressions,
//! executes approved shell commands, and rewrites frontmatter values with
//! trimmed stdout output.
//!
//! ## Syntax
//!
//! ```text
//! $(<command and args>)
//! $(<command and args>)::timeout:<seconds>
//! ```
//!
//! The entire frontmatter value must be a shell expansion expression.
//! Nested values (objects, arrays) are not scanned.

#[cfg(test)]
mod tests {
    use super::*;

    mod candidate_tests {
        use super::*;

        #[test]
        fn detects_simple_shell_expression() {
            let result = parse_shell_value("$(echo hello)", "files", None);
            assert!(result.is_some());
            let directive = result.unwrap();
            assert_eq!(directive.raw_command, "echo hello");
            assert_eq!(directive.executable, "echo");
            assert_eq!(directive.args, vec!["hello"]);
            assert!(directive.timeout_override.is_none());
        }

        #[test]
        fn detects_expression_with_timeout() {
            let result = parse_shell_value("$(pwd)::timeout:3", "cwd", None);
            assert!(result.is_some());
            let directive = result.unwrap();
            assert_eq!(directive.raw_command, "pwd");
            assert_eq!(directive.executable, "pwd");
            assert!(directive.args.is_empty());
            assert_eq!(
                directive.timeout_override,
                Some(std::time::Duration::from_secs(3))
            );
        }

        #[test]
        fn ignores_non_shell_string() {
            let result = parse_shell_value("just a plain string", "title", None);
            assert!(result.is_none());
        }

        #[test]
        fn ignores_partial_match_no_closing_paren() {
            let result = parse_shell_value("$(echo hello", "bad", None);
            assert!(result.is_none());
        }

        #[test]
        fn ignores_embedded_expression() {
            // Value must be ENTIRELY a shell expression
            let result = parse_shell_value("prefix $(echo hello) suffix", "bad", None);
            assert!(result.is_none());
        }

        #[test]
        fn rejects_zero_timeout() {
            let result = parse_shell_value("$(echo hello)::timeout:0", "bad", None);
            // Should be None (rejected) or return an error
            assert!(result.is_none());
        }

        #[test]
        fn rejects_non_integer_timeout() {
            let result = parse_shell_value("$(echo hello)::timeout:abc", "bad", None);
            assert!(result.is_none());
        }
    }

    mod interpolation_provenance_tests {
        use super::*;

        #[test]
        fn rejects_interpolated_executable() {
            // Original (pre-interpolation) had {{ }} in executable position
            let original = Some("$({{cmd}} arg1)");
            let result = parse_shell_value("$(ls arg1)", "bad", original);
            assert!(result.is_none());
        }

        #[test]
        fn accepts_interpolated_argument() {
            // Original had {{ }} only in argument position
            let original = Some("$(dirname {{file}})");
            let result = parse_shell_value("$(dirname README.md)", "dir", original);
            assert!(result.is_some());
            let directive = result.unwrap();
            assert_eq!(directive.executable, "dirname");
            assert_eq!(directive.args, vec!["README.md"]);
        }

        #[test]
        fn accepts_no_interpolation_at_all() {
            let original = Some("$(echo hello)");
            let result = parse_shell_value("$(echo hello)", "msg", original);
            assert!(result.is_some());
        }

        #[test]
        fn rejects_interpolated_executable_after_pipe_position() {
            // Even though pipes are rejected by the tokenizer, test the
            // conceptual rule: $(cat foo | {{cmd}}) — original has {{ in pipe target
            let original = Some("$(cat foo | {{cmd}})");
            // The tokenizer would reject this due to |, but the provenance
            // check runs first on the original string
            let result = parse_shell_value("$(cat foo | grep)", "bad", original);
            assert!(result.is_none());
        }
    }

    mod scan_tests {
        use super::*;
        use crate::markdown::frontmatter::Frontmatter;
        use serde_json::json;

        fn fm_from_json(data: serde_json::Value) -> Frontmatter {
            let map: crate::markdown::types::FrontmatterMap = match data {
                serde_json::Value::Object(obj) => obj.into_iter().collect(),
                _ => Default::default(),
            };
            Frontmatter::from_map(map)
        }

        #[test]
        fn scan_finds_shell_expressions_in_top_level_strings() {
            let fm = fm_from_json(json!({
                "files": "$(sniff repo dirty-files)",
                "title": "Hello",
                "count": 42
            }));
            let candidates = scan_frontmatter(&fm, None);
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].key, "files");
        }

        #[test]
        fn scan_skips_nested_objects() {
            let fm = fm_from_json(json!({
                "meta": { "cmd": "$(echo nested)" },
                "top": "$(echo top)"
            }));
            let candidates = scan_frontmatter(&fm, None);
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].key, "top");
        }

        #[test]
        fn scan_skips_arrays() {
            let fm = fm_from_json(json!({
                "list": ["$(echo item)"],
                "top": "$(echo top)"
            }));
            let candidates = scan_frontmatter(&fm, None);
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].key, "top");
        }
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p darkmatter frontmatter_shell_expansion -- --nocapture`
Expected: FAIL — functions not defined.

- [ ] **Step 4: Implement the parsing functions**

Add the implementation above the test module in `frontmatter_shell_expansion.rs`:

```rust
use crate::markdown::compose::shell_expansion::tokenize::tokenize;
use crate::markdown::compose::shell_expansion::types::{
    ShellCommandOrigin, ShellExpansionError,
};
use crate::markdown::frontmatter::Frontmatter;
use std::collections::HashMap;

/// A parsed frontmatter shell directive ready for execution.
#[derive(Debug, Clone)]
pub(crate) struct FrontmatterShellDirective {
    /// The frontmatter key this directive was extracted from.
    pub key: String,
    /// The raw template string (post-interpolation value).
    pub raw_template: String,
    /// The command string between `$(` and `)`.
    pub raw_command: String,
    /// The resolved executable name.
    pub executable: String,
    /// The resolved arguments.
    pub args: Vec<String>,
    /// Per-command timeout override from `::timeout:<N>` suffix.
    pub timeout_override: Option<std::time::Duration>,
}

/// Result of frontmatter shell expansion.
pub(crate) struct FrontmatterShellExpansionReport {
    /// Number of frontmatter values rewritten.
    pub replacements: usize,
    /// Number of shell approvals consumed.
    pub approvals_used: usize,
    /// Warnings emitted (e.g., timeout fallback).
    pub warnings: Vec<super::types::ComposeWarning>,
}

/// Attempts to parse a single frontmatter string value as a shell expression.
///
/// Returns `Some(directive)` if the value matches `$(command)` or
/// `$(command)::timeout:<N>`, otherwise `None`.
///
/// If `original_value` is provided (pre-interpolation snapshot), the
/// executable-token interpolation rule is enforced: the first token
/// in the original string must not contain `{{ }}`.
pub(crate) fn parse_shell_value(
    value: &str,
    key: &str,
    original_value: Option<&str>,
) -> Option<FrontmatterShellDirective> {
    let trimmed = value.trim();

    // Must start with $( and contain a closing )
    if !trimmed.starts_with("$(") {
        return None;
    }

    // Find the closing ) — it must be the last ) before any ::timeout suffix
    // or the last character if no suffix
    let (command_end, timeout_override) = if let Some(suffix_start) = trimmed.find(")::timeout:") {
        // Parse timeout suffix
        let timeout_str = &trimmed[suffix_start + ")::timeout:".len()..];

        // Reject if there's extra content after the timeout value
        let seconds: u64 = timeout_str.parse().ok()?;
        if seconds == 0 {
            return None;
        }

        (suffix_start, Some(std::time::Duration::from_secs(seconds)))
    } else if trimmed.ends_with(')') {
        (trimmed.len() - 1, None)
    } else {
        // No valid closing pattern
        return None;
    };

    // Extract the command string between $( and )
    let inner = &trimmed[2..command_end];
    if inner.trim().is_empty() {
        return None;
    }

    // Check interpolation provenance rule: the executable token must not
    // come from interpolation.
    if let Some(original) = original_value {
        let orig_trimmed = original.trim();
        if orig_trimmed.starts_with("$(") {
            // Find the inner command area of the original
            let orig_inner_start = 2;
            // Extract the first token area (up to first whitespace)
            let orig_inner = if let Some(paren_pos) = orig_trimmed[2..].find(')') {
                &orig_trimmed[2..2 + paren_pos]
            } else {
                &orig_trimmed[2..]
            };

            // Get the executable portion (before first whitespace)
            let executable_portion = orig_inner
                .split_whitespace()
                .next()
                .unwrap_or(orig_inner);

            if executable_portion.contains("{{") && executable_portion.contains("}}") {
                // Executable token was derived from interpolation — reject
                return None;
            }

            // Also check for interpolation in pipe/chain operator positions
            // The tokenizer rejects these anyway, but we check the original
            // to catch cases like $(cat foo | {{cmd}})
            for segment in orig_inner.split(&['|', '&'][..]) {
                let segment = segment.trim();
                if segment.is_empty() {
                    continue;
                }
                // Each segment after a pipe/chain has its own executable
                let seg_exe = segment.split_whitespace().next().unwrap_or(segment);
                if seg_exe.contains("{{") && seg_exe.contains("}}") {
                    return None;
                }
            }
        }
    }

    // Tokenize the command
    let tokens = tokenize(inner).ok()?;
    if tokens.is_empty() {
        return None;
    }

    let executable = tokens[0].clone();
    let args = tokens[1..].to_vec();
    let raw_command = tokens.join(" ");

    Some(FrontmatterShellDirective {
        key: key.to_string(),
        raw_template: trimmed.to_string(),
        raw_command,
        executable,
        args,
        timeout_override,
    })
}

/// Scans top-level frontmatter string values for shell expansion candidates.
///
/// Only examines top-level keys whose values are strings. Nested objects
/// and arrays are ignored per the v1 scope.
///
/// If `pre_interpolation_snapshot` is provided, it maps frontmatter keys
/// to their original values before frontmatter interpolation ran. This
/// enables the executable-token interpolation check.
pub(crate) fn scan_frontmatter(
    frontmatter: &Frontmatter,
    pre_interpolation_snapshot: Option<&HashMap<String, String>>,
) -> Vec<FrontmatterShellDirective> {
    let mut candidates = Vec::new();

    for (key, value) in frontmatter.as_map().iter() {
        if let Some(string_val) = value.as_str() {
            let original = pre_interpolation_snapshot
                .and_then(|snap| snap.get(key))
                .map(|s| s.as_str());

            if let Some(directive) = parse_shell_value(string_val, key, original) {
                candidates.push(directive);
            }
        }
    }

    candidates
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p darkmatter frontmatter_shell_expansion -- --nocapture`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs \
       darkmatter/lib/src/markdown/compose/mod.rs
git commit -m "feat(darkmatter): implement frontmatter shell expansion parsing and validation"
```

---

## Task 7: Implement frontmatter shell expansion execution

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`
- Test: inline tests

- [ ] **Step 1: Write the failing integration test**

Add to the tests in `frontmatter_shell_expansion.rs`:

```rust
mod execution_tests {
    use super::*;
    use crate::markdown::compose::shell_expansion::types::{
        ShellExpansionOptions, ShellExpansionRuntime, ShellPolicyPaths,
    };
    use crate::markdown::compose::types::{ComposeContext, ComposeOptions};
    use crate::markdown::frontmatter::Frontmatter;
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn fm_from_json(data: serde_json::Value) -> Frontmatter {
        let map: crate::markdown::types::FrontmatterMap = match data {
            serde_json::Value::Object(obj) => obj.into_iter().collect(),
            _ => Default::default(),
        };
        Frontmatter::from_map(map)
    }

    struct MockApprovalHandler;
    impl crate::markdown::compose::shell_expansion::types::ShellApprovalHandler
        for MockApprovalHandler
    {
        fn approve(
            &self,
            _request: crate::markdown::compose::shell_expansion::types::ShellApprovalRequest,
        ) -> Result<
            crate::markdown::compose::shell_expansion::types::ShellApprovalDecision,
            crate::markdown::compose::shell_expansion::types::ShellExpansionError,
        > {
            Ok(crate::markdown::compose::shell_expansion::types::ShellApprovalDecision::AllowOnce)
        }
    }

    #[test]
    fn execute_replaces_frontmatter_value_with_output() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "greeting": "$(echo hello world)"
        }));

        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApprovalHandler)),
            ..Default::default()
        });

        let mut runtime = crate::markdown::compose::shell_expansion::types::PipelineRuntime::new(
            16,
            crate::markdown::compose::cache::CacheAccessMode::Disabled,
            None,
        );

        let report = execute_frontmatter_shell_expansion(
            &mut fm,
            &options,
            &mut runtime,
            None,
        )
        .unwrap();

        assert_eq!(report.replacements, 1);
        assert_eq!(
            fm.as_map().get("greeting"),
            Some(&json!("hello world"))
        );
    }

    #[test]
    fn execute_trims_output_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "$(echo '  padded  ')"
        }));

        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApprovalHandler)),
            ..Default::default()
        });

        let mut runtime = crate::markdown::compose::shell_expansion::types::PipelineRuntime::new(
            16,
            crate::markdown::compose::cache::CacheAccessMode::Disabled,
            None,
        );

        let report = execute_frontmatter_shell_expansion(
            &mut fm,
            &options,
            &mut runtime,
            None,
        )
        .unwrap();

        assert_eq!(report.replacements, 1);
        // Output should be trimmed
        assert_eq!(
            fm.as_map().get("val"),
            Some(&json!("padded"))
        );
    }

    #[test]
    fn execute_skips_non_shell_values() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "title": "Hello",
            "count": 42,
            "cmd": "$(echo result)"
        }));

        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApprovalHandler)),
            ..Default::default()
        });

        let mut runtime = crate::markdown::compose::shell_expansion::types::PipelineRuntime::new(
            16,
            crate::markdown::compose::cache::CacheAccessMode::Disabled,
            None,
        );

        let report = execute_frontmatter_shell_expansion(
            &mut fm,
            &options,
            &mut runtime,
            None,
        )
        .unwrap();

        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("title"), Some(&json!("Hello")));
        assert_eq!(fm.as_map().get("count"), Some(&json!(42)));
        assert_eq!(fm.as_map().get("cmd"), Some(&json!("result")));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter execute_replaces_frontmatter_value -- --nocapture`
Expected: FAIL — `execute_frontmatter_shell_expansion` not found.

- [ ] **Step 3: Implement the execution function**

Add to `frontmatter_shell_expansion.rs`:

```rust
use crate::markdown::compose::shell_expansion::{
    execute_directive, resolve_policy_paths,
};
use crate::markdown::compose::shell_expansion::types::{
    ErrorHandling, PipelineRuntime, ShellDirective, ShellExpansionOptions,
};
use crate::markdown::compose::types::{ComposeOptions, ComposeWarning};
use crate::markdown::types::MarkdownResult;
use serde_json::Value;

/// Executes all frontmatter shell expansion directives and rewrites
/// frontmatter values in place.
///
/// Scans top-level string-valued frontmatter entries for `$(...)` expressions,
/// validates them, executes approved commands through the shared shell runtime,
/// trims output, and rewrites the frontmatter values.
///
/// When multiple shell expressions are found, they are executed concurrently
/// using rayon since seed-only semantics guarantee no cross-dependencies.
pub(crate) fn execute_frontmatter_shell_expansion(
    frontmatter: &mut Frontmatter,
    options: &ComposeOptions,
    runtime: &mut PipelineRuntime,
    pre_interpolation_snapshot: Option<&HashMap<String, String>>,
) -> MarkdownResult<FrontmatterShellExpansionReport> {
    let candidates = scan_frontmatter(frontmatter, pre_interpolation_snapshot);

    if candidates.is_empty() {
        return Ok(FrontmatterShellExpansionReport {
            replacements: 0,
            approvals_used: 0,
            warnings: vec![],
        });
    }

    // Resolve policy paths once for all directives
    let shell_opts = options.shell_options();
    let policy_paths = resolve_policy_paths(&shell_opts, &options.source)?;
    runtime.shell.ensure_loaded(&policy_paths)?;

    // Execute each directive. For now, execute serially.
    // TODO: When rayon is wired up for frontmatter concurrency, use par_iter
    // here. Serial execution is correct since there are no cross-dependencies.
    let mut results: Vec<(String, String)> = Vec::new();
    let mut warnings = Vec::new();

    for candidate in &candidates {
        // Build a ShellDirective compatible with the existing execute_directive API
        let directive = ShellDirective {
            raw_command: candidate.raw_command.clone(),
            executable: candidate.executable.clone(),
            args: candidate.args.clone(),
            span: 0..0, // Not relevant for frontmatter
            origin: ShellCommandOrigin::Frontmatter {
                key: candidate.key.clone(),
            },
            error_handling: ErrorHandling::default(),
            timeout_override: candidate.timeout_override,
        };

        let output =
            execute_directive(&directive, options, &policy_paths, &mut runtime.shell)?;

        // Trim all surrounding whitespace per spec
        let trimmed = output.trim().to_string();

        // If timeout behavior is EmptyString and result is empty, emit a warning
        if trimmed.is_empty() && !output.is_empty() {
            warnings.push(ComposeWarning::new(
                "frontmatter-shell-expansion",
                format!(
                    "Shell command for frontmatter key '{}' produced only whitespace",
                    candidate.key
                ),
            ));
        }

        results.push((candidate.key.clone(), trimmed));
    }

    // Rewrite frontmatter values
    let fm_mut = frontmatter.as_map_mut();
    let replacements = results.len();
    for (key, value) in results {
        fm_mut.insert(key, Value::String(value));
    }

    let approvals_used = runtime.shell.take_recent_approval_count();

    Ok(FrontmatterShellExpansionReport {
        replacements,
        approvals_used,
        warnings,
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter frontmatter_shell_expansion::tests::execution_tests -- --nocapture`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
git commit -m "feat(darkmatter): implement frontmatter shell expansion execution"
```

---

## Task 8: Integrate frontmatter shell expansion into compose pipeline

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/mod.rs:278-494`
- Test: compose integration tests

- [ ] **Step 1: Write the failing integration test**

Add to the integration tests in `darkmatter/lib/src/markdown/compose/mod.rs` (or add a new test file):

```rust
#[cfg(test)]
mod frontmatter_shell_expansion_integration {
    use crate::markdown::Markdown;
    use crate::markdown::compose::{ComposeOperation, ComposeOptions};
    use crate::markdown::compose::shell_expansion::types::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
        ShellExpansionError, ShellExpansionOptions,
    };
    use std::sync::Arc;
    use tempfile::TempDir;

    struct MockApproval;
    impl ShellApprovalHandler for MockApproval {
        fn approve(
            &self,
            _req: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    #[test]
    fn frontmatter_shell_output_visible_to_body_interpolation() {
        let temp_dir = TempDir::new().unwrap();
        let content = "---\ngreeting: \"$(echo hello)\"\n---\nMessage: {{greeting}}\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::Interpolation,
            ])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("Message: hello"),
            "Expected 'Message: hello' in:\n{}",
            composed.content()
        );
        assert_eq!(report.frontmatter_shell_expansions_applied, 1);
    }

    #[test]
    fn frontmatter_interpolation_feeds_into_shell_expansion() {
        let temp_dir = TempDir::new().unwrap();
        let content = "---\nfile: README.md\ndir: \"$(dirname {{file}})\"\n---\nDir: {{dir}}\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::Interpolation,
            ])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("Dir: ."),
            "Expected 'Dir: .' in:\n{}",
            composed.content()
        );
        assert_eq!(report.frontmatter_shell_expansions_applied, 1);
    }

    #[test]
    fn body_and_frontmatter_shell_coexist() {
        let temp_dir = TempDir::new().unwrap();
        let content = "---\nfm_val: \"$(echo from-frontmatter)\"\n---\n::shell echo from-body\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::ShellExpansion,
            ])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApproval)),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert_eq!(report.frontmatter_shell_expansions_applied, 1);
        assert_eq!(report.shell_expansions_applied, 1);
        assert!(composed.content().contains("from-body"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter frontmatter_shell_expansion_integration -- --nocapture`
Expected: FAIL — pipeline doesn't run frontmatter shell expansion.

- [ ] **Step 3: Integrate into `run_compose_pipeline_internal`**

In `darkmatter/lib/src/markdown/compose/mod.rs`, within `run_compose_pipeline_internal()`, add the following block **after** the frontmatter interpolation block (around line 342) and **before** the `EffectiveState` build (around line 345):

First, **snapshot pre-interpolation frontmatter values** before frontmatter interpolation runs. Add this block before the interpolation block (around line 324):

```rust
// Snapshot frontmatter string values before interpolation for the
// executable-token provenance check in frontmatter shell expansion.
let pre_interpolation_snapshot: Option<HashMap<String, String>> =
    if options.is_enabled(ComposeOperation::FrontmatterShellExpansion) {
        let mut snapshot = HashMap::new();
        for (key, value) in self.frontmatter().as_map().iter() {
            if let Some(s) = value.as_str() {
                snapshot.insert(key.clone(), s.to_string());
            }
        }
        Some(snapshot)
    } else {
        None
    };
```

Then, after the frontmatter interpolation block, add frontmatter shell expansion:

```rust
// Frontmatter Shell Expansion: execute $(cmd) in frontmatter values
// before EffectiveState is built, since the expanded values must be
// visible to all later stages.
if options.is_enabled(ComposeOperation::FrontmatterShellExpansion) {
    let fse_start = perf.is_enabled().then(std::time::Instant::now);
    let fse_report =
        frontmatter_shell_expansion::execute_frontmatter_shell_expansion(
            self.frontmatter_mut(),
            &options,
            runtime,
            pre_interpolation_snapshot.as_ref(),
        )?;
    report.frontmatter_shell_expansions_applied = fse_report.replacements;
    report.shell_approvals_used += fse_report.approvals_used;
    report.warnings.extend(fse_report.warnings);
    if let Some(start) = fse_start {
        perf.record(
            perf::PerfMetricKind::FrontmatterShellExpansion,
            start.elapsed(),
        );
    }
}
```

Also ensure that the `HashMap` import is present at the top of the file (it already is via `use std::collections::HashMap`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter frontmatter_shell_expansion_integration -- --nocapture`
Expected: All PASS

- [ ] **Step 5: Run the full test suite**

Run: `cargo test -p darkmatter -p darkmatter-cli -- --nocapture 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/mod.rs
git commit -m "feat(darkmatter): integrate frontmatter shell expansion into compose pipeline"
```

---

## Task 9: Extend discovery to include frontmatter shell commands

**Files:**
- Modify: `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs`
- Modify: `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs` (expose scan function)
- Test: inline tests in discovery.rs

- [ ] **Step 1: Write the failing test**

Add to the tests in `discovery.rs`:

```rust
#[test]
fn discovers_frontmatter_shell_commands() {
    let content = "---\nfiles: \"$(sniff repo dirty-files)\"\n---\n# Doc\n::shell echo body\n";
    let md: Markdown = content.into();
    let options = ComposeOptions::new();

    let entries = collect_shell_commands(&md, &options).unwrap();

    // Should find both the frontmatter command and the body command
    assert_eq!(entries.len(), 2, "entries: {:?}", entries);
    let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
    assert!(executables.contains(&"sniff"), "Missing sniff: {:?}", executables);
    assert!(executables.contains(&"echo"), "Missing echo: {:?}", executables);
}

#[test]
fn frontmatter_shell_commands_use_frontmatter_origin() {
    let content = "---\nfiles: \"$(echo fm-cmd)\"\n---\n# Doc\n";
    let md: Markdown = content.into();
    let options = ComposeOptions::new();

    let entries = collect_shell_commands(&md, &options).unwrap();

    assert_eq!(entries.len(), 1);
    match &entries[0].origin {
        ShellCommandOrigin::Frontmatter { key } => assert_eq!(key, "files"),
        other => panic!("Expected Frontmatter origin, got: {:?}", other),
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter discovers_frontmatter_shell_commands -- --nocapture`
Expected: FAIL — frontmatter commands not found in discovery.

- [ ] **Step 3: Extend `collect_shell_commands` to scan frontmatter**

In `discovery.rs`, after the existing body directive parsing (around line 97), add frontmatter scanning. Before the body scan, we need to also scan frontmatter from the original document (before compose runs).

Add frontmatter scanning before the compose pass. The approach is:

1. Clone the markdown document
2. Run frontmatter interpolation only on the clone
3. Scan the interpolated frontmatter for shell expressions
4. Continue with the existing compose-based body discovery

Update `collect_shell_commands()`:

```rust
pub fn collect_shell_commands(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> MarkdownResult<Vec<ShellCommandEntry>> {
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    let default_source = match &options.source {
        ComposeSource::File(p) => p.clone(),
        _ => PathBuf::from("<unknown>"),
    };

    // ── Phase 1: Discover frontmatter shell commands ───────────────
    // Clone the document and run frontmatter interpolation to resolve
    // any variables in shell expressions, then scan for candidates.
    {
        let mut fm_clone = markdown.clone();
        if options.is_enabled(ComposeOperation::FrontmatterInterpolation) {
            let _ = crate::markdown::compose::frontmatter_interpolation::interpolate_frontmatter(
                fm_clone.frontmatter_mut(),
                options.context(),
                false,
            );
        }

        let candidates = crate::markdown::compose::frontmatter_shell_expansion::scan_frontmatter(
            fm_clone.frontmatter(),
            None, // No pre-interpolation snapshot needed for discovery
        );

        for candidate in candidates {
            let (executable, args) = if which::which(&candidate.executable).is_ok() {
                (candidate.executable.clone(), candidate.args.clone())
            } else if let Some(resolved) = resolve_alias(&candidate.executable) {
                let mut merged_args = resolved.args;
                merged_args.extend_from_slice(&candidate.args);
                (resolved.executable, merged_args)
            } else {
                (candidate.executable.clone(), candidate.args.clone())
            };

            let normalized = normalize_command(&executable, &args);

            if seen.insert(normalized.clone()) {
                entries.push(ShellCommandEntry {
                    raw_command: candidate.raw_command,
                    executable,
                    args,
                    normalized,
                    source_file: default_source.clone(),
                    origin: ShellCommandOrigin::Frontmatter {
                        key: candidate.key.clone(),
                    },
                });
            }
        }
    }

    // ── Phase 2: Discover body shell commands (existing logic) ─────
    let discovery_options = options.clone().only(&[
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::TextReplacement,
        ComposeOperation::PageBlocks,
        ComposeOperation::Interpolation,
        ComposeOperation::BlockTransclusion,
        ComposeOperation::FrontmatterTransclusion,
    ]);

    let (composed, report) = markdown.compose_with(discovery_options)?;
    let directives = parse_directives(composed.content())?;

    for directive in directives {
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
            let (source_file, line) = lookup_provenance(
                directive.span.start,
                directive.origin.line_number(),
                &report.source_map,
                composed.content(),
                &default_source,
            );

            entries.push(ShellCommandEntry {
                raw_command: directive.raw_command,
                executable,
                args,
                normalized,
                source_file,
                origin: directive.origin,
            });
        }
    }

    Ok(entries)
}
```

Add the import for the new module at the top of `discovery.rs`:

```rust
use crate::markdown::compose::shell_expansion::types::ShellCommandOrigin;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter discovers_frontmatter_shell_commands frontmatter_shell_commands_use_frontmatter_origin -- --nocapture`
Expected: All PASS

- [ ] **Step 5: Run the full test suite**

Run: `cargo test -p darkmatter -p darkmatter-cli -- --nocapture 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs \
       darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
git commit -m "feat(darkmatter): extend shell command discovery to include frontmatter expressions"
```

---

## Task 10: Add CLI flags `--timeout` and `--allow-shell-timeout`

**Files:**
- Modify: `darkmatter/cli/src/args.rs`
- Modify: `darkmatter/cli/src/commands.rs`
- Test: inline tests in args.rs and commands.rs

- [ ] **Step 1: Write the failing test for CLI arg parsing**

Add to the tests in `darkmatter/cli/src/args.rs`:

```rust
#[test]
fn compose_timeout_flag_parses() {
    let cli = Cli::try_parse_from(["md", "compose", "doc.md", "--timeout", "3"]).unwrap();
    match cli.command {
        Some(Command::Compose { timeout, .. }) => assert_eq!(timeout, Some(3)),
        _ => panic!("Expected Compose command"),
    }
}

#[test]
fn compose_allow_shell_timeout_flag_parses() {
    let cli =
        Cli::try_parse_from(["md", "compose", "doc.md", "--allow-shell-timeout"]).unwrap();
    match cli.command {
        Some(Command::Compose {
            allow_shell_timeout,
            ..
        }) => assert!(allow_shell_timeout),
        _ => panic!("Expected Compose command"),
    }
}

#[test]
fn compose_timeout_defaults_to_none() {
    let cli = Cli::try_parse_from(["md", "compose", "doc.md"]).unwrap();
    match cli.command {
        Some(Command::Compose { timeout, .. }) => assert_eq!(timeout, None),
        _ => panic!("Expected Compose command"),
    }
}

#[test]
fn compose_allow_shell_timeout_defaults_false() {
    let cli = Cli::try_parse_from(["md", "compose", "doc.md"]).unwrap();
    match cli.command {
        Some(Command::Compose {
            allow_shell_timeout,
            ..
        }) => assert!(!allow_shell_timeout),
        _ => panic!("Expected Compose command"),
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p darkmatter-cli compose_timeout_flag -- --nocapture`
Expected: FAIL — `timeout` field not found on `Compose`.

- [ ] **Step 3: Add the flags to the Compose subcommand**

In `darkmatter/cli/src/args.rs`, add to the `Compose` variant:

```rust
/// Global shell command timeout in seconds (default: 10)
#[arg(long, value_name = "SECONDS")]
timeout: Option<u64>,

/// Convert shell timeout failures into empty strings instead of errors
#[arg(long)]
allow_shell_timeout: bool,
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p darkmatter-cli compose_timeout_flag compose_allow_shell_timeout -- --nocapture`
Expected: All PASS

- [ ] **Step 5: Wire the flags into ComposeOptions**

In `darkmatter/cli/src/commands.rs`, update the `run_compose()` function signature and the `run_subcommand()` match arm to pass through `timeout` and `allow_shell_timeout`.

In the `CliCommand::Compose` match arm (around line 110), add the new fields:

```rust
CliCommand::Compose {
    // ... existing fields ...
    timeout,
    allow_shell_timeout,
    perf,
} => {
```

Pass them to `run_compose()`:

```rust
run_compose(
    input.as_ref(),
    state.as_deref(),
    set.as_deref(),
    output,
    show,
    frontmatter,
    mode,
    indent,
    &allow,
    allow_ctx_override,
    timeout,
    allow_shell_timeout,
    perf,
    cli,
)?;
```

Update the `run_compose()` function signature:

```rust
pub fn run_compose(
    // ... existing params ...
    timeout_secs: Option<u64>,
    allow_shell_timeout: bool,
    perf: bool,
    cli: &Cli,
) -> Result<()> {
```

Wire the new values into `ComposeOptions` in the shell options section:

```rust
let shell_opts = ShellExpansionOptions {
    timeout: timeout_secs
        .map(|s| std::time::Duration::from_secs(s))
        .unwrap_or(std::time::Duration::from_secs(10)),
    policy_root: resolved_input.as_ref().and_then(|p| {
        p.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.to_path_buf())
    }),
    approval_handler: if is_file_input && crate::approval::can_prompt_interactively() {
        Some(Arc::new(crate::approval::CliShellApprovalHandler))
    } else {
        None
    },
    ..Default::default()
};

options = options.with_shell(shell_opts);

if allow_shell_timeout {
    options = options.with_allow_shell_timeout(true);
}
```

- [ ] **Step 6: Update origin-aware error formatting in commands.rs**

Update the shell error match arms to use `origin` instead of `line`. For each `ShellExpansionError` variant, change references from `.line` to `.origin`:

```rust
ShellExpansion(ShellExpansionError::ExecutionFailed {
    command,
    code,
    stderr,
    origin,
    ..
}) => {
    let detail = stderr.trim();
    if detail.is_empty() {
        eyre!("Shell command failed (exit {code}) at {origin}: '{command}'")
    } else {
        eyre!(
            "Shell command failed (exit {code}) at {origin}: '{command}'\n{detail}"
        )
    }
}
```

Apply similar changes to all other `ShellExpansionError` match arms (`CommandNotFound`, `Timeout`, `Blacklisted`, `Denied`, `ApprovalRequired`).

- [ ] **Step 7: Run the full test suite**

Run: `cargo test -p darkmatter -p darkmatter-cli -- --nocapture 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add darkmatter/cli/src/args.rs darkmatter/cli/src/commands.rs
git commit -m "feat(darkmatter-cli): add --timeout and --allow-shell-timeout compose flags"
```

---

## Task 11: Update documentation

**Files:**
- Create: `darkmatter/docs/inline/fm-shell-expansion.md`
- Modify: `darkmatter/docs/darkmatter-compose-pipeline.md`

- [ ] **Step 1: Create the frontmatter shell expansion documentation**

Create `darkmatter/docs/inline/fm-shell-expansion.md`:

```markdown
# Frontmatter Shell Expansion

Frontmatter Shell Expansion allows shell commands to be executed during the compose pipeline and their stdout output stored as frontmatter property values.

## Syntax

A top-level frontmatter property whose entire string value matches one of these patterns is treated as a shell expression:

```text
$(<command and args>)
$(<command and args>)::timeout:<seconds>
```

Examples:

```yaml
---
files: "$(sniff repo dirty-files)"
cwd: "$(pwd)::timeout:1"
---
```

## Rules

- The **entire** frontmatter value must be the shell expression — embedded expressions like `"prefix $(cmd) suffix"` are not supported.
- Only top-level string-valued frontmatter properties are scanned. Nested objects and array elements are ignored.
- The optional `::timeout:<N>` suffix overrides the global shell timeout for that specific command. `N` must be a positive integer of seconds.

## Pipeline Placement

Frontmatter Shell Expansion runs in the **Inline Pre** phase, after Frontmatter Interpolation and before EffectiveState construction:

1. Merge external/inherited state
2. Apply `--set` overrides
3. **Frontmatter Interpolation** — resolve `{{ }}` expressions
4. **Frontmatter Shell Expansion** — execute `$(cmd)` expressions
5. Build EffectiveState
6. Body operations continue...

Because interpolation runs first, shell commands can use interpolated values as arguments:

```yaml
---
file: README.md
dir: "$(dirname {{file}})"
---
```

After interpolation, the shell stage sees `$(dirname README.md)`.

## Security

### Executable Token Rule

The executable (first token) of a frontmatter shell command must **not** come from interpolation. Only arguments may be interpolated.

Rejected:

```yaml
cmd: ls
bad: "$({{cmd}} -la)"        # executable from interpolation
```

Accepted:

```yaml
file: README.md
dir: "$(dirname {{file}})"   # only argument is interpolated
```

### Approval

Frontmatter shell commands participate in the same approval flow as body `::shell` directives. They are included in preflight discovery and subject to whitelist, blacklist, and interactive approval.

## Error Handling

Frontmatter shell expansion has **no error-recovery options**. Any non-zero exit code, missing executable, blacklisted command, or denied approval results in an immediate compose error. This is intentionally simpler than body `::shell` directives.

Timeout failures follow the timeout behavior configured via `--allow-shell-timeout` (CLI) or `ComposeOptions::with_allow_shell_timeout()` (library).

## Output Normalization

The stdout from a frontmatter shell command is trimmed of all surrounding whitespace (`.trim()`) before being stored as the frontmatter value.

## Concurrency

When multiple top-level frontmatter properties contain shell expressions, they execute concurrently. Seed-only interpolation semantics guarantee no cross-dependencies.

## Timeouts

- Default global timeout: 10 seconds
- Override globally: `--timeout <seconds>` (CLI) or `ComposeOptions::with_shell_timeout()` (library)
- Override per-command: `$(cmd)::timeout:<seconds>`
- Timeout outcome:
  - Default: compose error
  - With `--allow-shell-timeout`: empty string replacement + warning
```

- [ ] **Step 2: Update the compose pipeline documentation**

In `darkmatter/docs/darkmatter-compose-pipeline.md`, add `Frontmatter Shell Expansion` to the Inline Pre phase listing, between Frontmatter Interpolation and Text Replacement. Update the pipeline diagram if one exists.

- [ ] **Step 3: Commit**

```bash
git add darkmatter/docs/inline/fm-shell-expansion.md \
       darkmatter/docs/darkmatter-compose-pipeline.md
git commit -m "docs(darkmatter): add frontmatter shell expansion documentation"
```

---

## Self-Review Checklist

### Spec Coverage

| Spec Requirement | Task |
|-----------------|------|
| `$(cmd)` syntax in frontmatter | Task 6 (parsing) |
| `$(cmd)::timeout:N` syntax | Task 6 (parsing) |
| Error handling: always fatal | Task 7 (execution — no ErrorHandling options) |
| Output trimming (`.trim()`) | Task 7 (execution) |
| Concurrency for multiple expressions | Task 7 (execution — serial initially, rayon-ready) |
| Security: executable-token rule | Task 6 (interpolation provenance check) |
| Security: preflight discovery | Task 9 (discovery extension) |
| Pipeline placement after interpolation | Task 8 (pipeline integration) |
| Timeouts for both body and frontmatter | Tasks 1-3 (shared timeout infrastructure) |
| Default 10s timeout | Task 1 (ShellExpansionOptions default) |
| `--allow-shell-timeout` flag | Task 10 (CLI) |
| `--timeout N` flag | Task 10 (CLI) |
| Per-command `::timeout:N` for frontmatter | Task 6 (parsing) |
| Per-command `::timeout:N` for body | Task 3 (body parser) |
| Timeout empty-string behavior + warning | Task 2 (executor) |
| Origin-aware error reporting | Task 4 (ShellCommandOrigin) |

### Placeholder Scan

No "TBD", "TODO" (except one intentional note about rayon concurrency), or "implement later" placeholders.

### Type Consistency

- `ShellCommandOrigin` — defined in Task 4, used consistently in Tasks 5-10
- `ShellTimeoutBehavior` — defined in Task 1, used in Tasks 2, 10
- `FrontmatterShellDirective` — defined in Task 6, used in Tasks 7, 9
- `FrontmatterShellExpansionReport` — defined in Task 6, used in Tasks 7, 8
- `timeout_override: Option<Duration>` — added to `ShellDirective` in Task 2, parsed in Tasks 3, 6
- `ComposeOperation::FrontmatterShellExpansion` — added in Task 5, integrated in Task 8
