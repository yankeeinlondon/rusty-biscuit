# Shell Approval Coherence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make shell approval coherent end-to-end: wire interactive approval, preserve preflight state into runtime, and eliminate redundant audit passes.

**Architecture:** The preflight pass in `claudine-lib` becomes the single approval authority. A new `CliShellApprovalHandler` in `claudine-cli` provides interactive prompting. The approval cache (`Arc<Mutex<HashMap<…>>>`) is shared between preflight and the runtime harness loop so that `AllowOnce` decisions survive the handoff.

**Tech Stack:** Rust, darkmatter `ShellApprovalHandler` trait, `biscuit-terminal` Prose rendering, claudine harness/composition modules

**Note on Issue 2 from review4.md:** The parsing-during-approval issue (`parse_harness_plan_with_shell`) is already fixed in the working tree. All parse functions now use `parse_harness_plan` (no shell_options). This plan addresses the two remaining issues.

---

## File Structure

| File | Role |
|------|------|
| Create: `claudine/cli/src/commands/wrap/approval.rs` | `CliShellApprovalHandler` — interactive shell approval for the claudine wrapper |
| Modify: `claudine/cli/src/commands/wrap/mod.rs` | Wire handler into `build_harness_shell_options`, share cache through `CachedHarnessLoopContext`, accept pre-approved set in `run_harness_loop` |
| Modify: `claudine/cli/src/commands/wrap/composition.rs` | Pass pre-approved cache into the harness loop |
| Test: `claudine/cli/src/commands/wrap/approval.rs` (inline `#[cfg(test)]`) | Unit tests for the approval handler |
| Test: `claudine/lib/src/composition/preflight.rs` (extend existing tests) | Test that approval handler is invoked during preflight and cache is shared |

---

### Task 1: Create the CLI Shell Approval Handler

**Files:**
- Create: `claudine/cli/src/commands/wrap/approval.rs`
- Modify: `claudine/cli/src/commands/wrap/mod.rs` (add `mod approval;`)

This handler mirrors `darkmatter/cli/src/approval.rs` but lives in claudine-cli so there is no cross-CLI dependency. It implements `ShellApprovalHandler` by prompting on stderr / reading from stdin.

- [ ] **Step 1: Write the failing test for approval handler**

Create `claudine/cli/src/commands/wrap/approval.rs` with the test module first. The handler struct and `approve_with_io` function do not exist yet, so compilation will fail.

```rust
//! Interactive shell command approval handler for the Claudine CLI.

use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use std::io::{self, BufRead, IsTerminal, Write};
use std::sync::Arc;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;

/// Interactive approval handler that prompts the user on stderr.
pub(crate) struct CliShellApprovalHandler;

impl ShellApprovalHandler for CliShellApprovalHandler {
    fn approve(
        &self,
        request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError> {
        let stderr = io::stderr();
        let stdin = io::stdin();
        let mut stderr = stderr.lock();
        let mut stdin = stdin.lock();
        approve_with_io(&request, &mut stdin, &mut stderr)
    }
}

/// Returns `true` when interactive prompting is safe (both stdin and
/// stderr are terminals).
pub(crate) fn can_prompt_interactively() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

/// Build an optional `Arc<dyn ShellApprovalHandler>` based on interactivity.
pub(crate) fn approval_handler_if_interactive() -> Option<Arc<dyn ShellApprovalHandler>> {
    if can_prompt_interactively() {
        Some(Arc::new(CliShellApprovalHandler))
    } else {
        None
    }
}

/// Escapes angle brackets in user-provided text to prevent Prose tag interpretation.
fn escape_prose(text: &str) -> String {
    text.replace('<', "\\<").replace('>', "\\>")
}

fn approve_with_io<R: BufRead, W: Write>(
    request: &ShellApprovalRequest,
    input: &mut R,
    output: &mut W,
) -> Result<ShellApprovalDecision, ShellExpansionError> {
    let source_desc = match &request.source {
        darkmatter::markdown::compose::ComposeSource::File(p) => p.display().to_string(),
        darkmatter::markdown::compose::ComposeSource::Url(u) => u.to_string(),
        darkmatter::markdown::compose::ComposeSource::Unknown => "<stdin>".to_string(),
    };

    loop {
        write_prompt(output, request, &source_desc)?;

        let mut choice = String::new();
        input
            .read_line(&mut choice)
            .map_err(|source| ShellExpansionError::PolicyIo {
                path: request.whitelist_path.clone(),
                source,
            })?;

        match choice.trim() {
            "1" => return Ok(ShellApprovalDecision::AllowExactPersist),
            "2" => return Ok(ShellApprovalDecision::AllowCommandPersist),
            "3" => return Ok(ShellApprovalDecision::AllowOnce),
            "4" => return Ok(ShellApprovalDecision::Deny),
            "5" => return Ok(ShellApprovalDecision::BlacklistPersist),
            _ => {
                let err_msg = Prose::new("<red>Invalid choice. Please enter 1-5.</red>")
                    .render_optimistic(None);
                writeln!(output, "  {err_msg}").map_err(|source| {
                    ShellExpansionError::PolicyIo {
                        path: request.whitelist_path.clone(),
                        source,
                    }
                })?;
            }
        }
    }
}

fn write_prompt<W: Write>(
    output: &mut W,
    request: &ShellApprovalRequest,
    source_desc: &str,
) -> Result<(), ShellExpansionError> {
    let p = &request.whitelist_path;
    let w = |output: &mut W, args: std::fmt::Arguments| -> Result<(), ShellExpansionError> {
        output
            .write_fmt(args)
            .map_err(|source| ShellExpansionError::PolicyIo {
                path: p.to_path_buf(),
                source,
            })
    };

    let header = Prose::new("\u{26a0}  <bold><yellow>Shell Approval Required</yellow></bold>")
        .render_optimistic(None);
    let source_label = Prose::new("<dim>Source:</dim>").render_optimistic(None);
    let source_value =
        Prose::new(format!("<bold>{source_desc}:{}</bold>", request.line)).render_optimistic(None);
    let cmd_label = Prose::new("<dim>Command:</dim>").render_optimistic(None);
    let cmd_value = Prose::new(format!(
        "<bold><cyan>{}</cyan></bold>",
        escape_prose(&request.raw_command)
    ))
    .render_optimistic(None);

    w(output, format_args!("\n  {header}\n"))?;
    w(output, format_args!("  {source_label}  {source_value}\n"))?;

    if let Some(ref alias_name) = request.alias_name {
        let alias_label = Prose::new("<dim>Alias:</dim>").render_optimistic(None);
        let alias_value = Prose::new(format!(
            "<bold>{}</bold> <dim>\u{2192}</dim> <bold><cyan>{}</cyan></bold>",
            escape_prose(alias_name),
            escape_prose(&request.raw_command)
        ))
        .render_optimistic(None);
        w(output, format_args!("  {alias_label}   {alias_value}\n"))?;
    }

    w(output, format_args!("  {cmd_label} {cmd_value}\n\n"))?;

    let opt1_num = Prose::new("<green>1</green>").render_optimistic(None);
    let opt1_desc = Prose::new(format!(
        "<dim>(persists \"{}\" to whitelist)</dim>",
        escape_prose(&request.raw_command)
    ))
    .render_optimistic(None);
    let opt2_num = Prose::new("<green>2</green>").render_optimistic(None);
    let opt2_desc = Prose::new(format!(
        "<dim>(persists \"{}\" with any args to whitelist)</dim>",
        escape_prose(&request.executable)
    ))
    .render_optimistic(None);
    let opt3_num = Prose::new("<cyan>3</cyan>").render_optimistic(None);
    let opt3_desc = Prose::new("<dim>(this session only)</dim>").render_optimistic(None);
    let opt4_num = Prose::new("<yellow>4</yellow>").render_optimistic(None);
    let opt5_num = Prose::new("<red>5</red>").render_optimistic(None);
    let opt5_desc = Prose::new("<dim>(persists to blacklist)</dim>").render_optimistic(None);

    let sep = Prose::new("<dim>\u{2502}</dim>").render_optimistic(None);

    w(
        output,
        format_args!("  {opt1_num} {sep} Allow exact and save    {opt1_desc}\n"),
    )?;
    w(
        output,
        format_args!("  {opt2_num} {sep} Allow command and save  {opt2_desc}\n"),
    )?;
    w(
        output,
        format_args!("  {opt3_num} {sep} Allow once              {opt3_desc}\n"),
    )?;
    w(output, format_args!("  {opt4_num} {sep} Deny\n"))?;
    w(
        output,
        format_args!("  {opt5_num} {sep} Blacklist and stop      {opt5_desc}\n\n"),
    )?;

    let prompt_arrow = Prose::new("<bold>></bold>").render_optimistic(None);
    w(output, format_args!("  {prompt_arrow} "))?;

    output
        .flush()
        .map_err(|source| ShellExpansionError::PolicyIo {
            path: request.whitelist_path.clone(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::compose::ComposeSource;
    use std::io::Cursor;
    use std::path::PathBuf;

    fn request() -> ShellApprovalRequest {
        ShellApprovalRequest {
            source: ComposeSource::File(PathBuf::from("/tmp/doc.md")),
            line: 12,
            raw_command: "echo hello".to_string(),
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
            normalized_exact: "echo hello".to_string(),
            whitelist_path: PathBuf::from("/tmp/.darkmatter-shell-whitelist"),
            blacklist_path: PathBuf::from("/tmp/.darkmatter-shell-blacklist"),
            alias_name: None,
        }
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for inner in chars.by_ref() {
                    if inner == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn all_five_choices_produce_correct_decisions() {
        let cases = [
            ("1\n", ShellApprovalDecision::AllowExactPersist),
            ("2\n", ShellApprovalDecision::AllowCommandPersist),
            ("3\n", ShellApprovalDecision::AllowOnce),
            ("4\n", ShellApprovalDecision::Deny),
            ("5\n", ShellApprovalDecision::BlacklistPersist),
        ];

        for (input_str, expected) in cases {
            let req = request();
            let mut input = Cursor::new(input_str.as_bytes().to_vec());
            let mut output = Vec::new();

            let decision = approve_with_io(&req, &mut input, &mut output).unwrap();
            assert_eq!(
                decision,
                expected,
                "Input '{}' should produce {:?}",
                input_str.trim(),
                expected
            );
        }
    }

    #[test]
    fn retries_after_invalid_choice() {
        let req = request();
        let mut input = Cursor::new(b"9\n3\n".to_vec());
        let mut output = Vec::new();

        let decision = approve_with_io(&req, &mut input, &mut output).unwrap();
        assert_eq!(decision, ShellApprovalDecision::AllowOnce);

        let prompt = strip_ansi(&String::from_utf8(output).unwrap());
        assert!(prompt.contains("Invalid choice. Please enter 1-5."));
        assert_eq!(prompt.matches("Shell Approval Required").count(), 2);
    }

    #[test]
    fn prompt_shows_source_and_command() {
        let req = request();
        let mut input = Cursor::new(b"1\n".to_vec());
        let mut output = Vec::new();

        approve_with_io(&req, &mut input, &mut output).unwrap();
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());

        assert!(prompt.contains("Source:"));
        assert!(prompt.contains("/tmp/doc.md:12"));
        assert!(prompt.contains("Command:"));
        assert!(prompt.contains("echo hello"));
    }
}
```

- [ ] **Step 2: Register the module**

In `claudine/cli/src/commands/wrap/mod.rs`, add the module declaration near the top with the other modules:

```rust
mod approval;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p claudine-cli -- approval`
Expected: 3 tests pass

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/wrap/approval.rs claudine/cli/src/commands/wrap/mod.rs
git commit -m "feat(claudine): add CLI shell approval handler for interactive prompting"
```

---

### Task 2: Wire the Approval Handler into Shell Options

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs:170-179` (`build_harness_shell_options`)
- Modify: `claudine/cli/src/commands/wrap/mod.rs:269-304` (`CachedHarnessLoopContext`)

The approval handler must be available both during preflight (before the harness loop) and during the runtime audit (inside the harness loop). We achieve this by:

1. Making `build_harness_shell_options` accept an optional handler
2. Making `CachedHarnessLoopContext` accept and preserve a shared `approval_cache`

- [ ] **Step 1: Update `build_harness_shell_options` to accept handler and cache**

In `claudine/cli/src/commands/wrap/mod.rs`, change the function signature and body:

```rust
pub(crate) fn build_harness_shell_options(
    source_path: &Path,
    repo_root: Option<&Path>,
    approval_handler: Option<Arc<dyn darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler>>,
    approval_cache: Option<Arc<std::sync::Mutex<std::collections::HashMap<String, claudine::harness::shell::CachedApprovalDecision>>>>,
) -> claudine::harness::ShellApprovalOptions {
    claudine::harness::ShellApprovalOptions {
        policy_root: harness_policy_root(source_path, repo_root),
        approval_handler,
        approval_cache: approval_cache.unwrap_or_default(),
    }
}
```

Note: check whether `CachedApprovalDecision` is public. If not, the cache type is accessible via `ShellApprovalOptions::default().approval_cache` — just use `Arc::new(Mutex::new(HashMap::new()))` directly.

- [ ] **Step 2: Verify `CachedApprovalDecision` visibility**

Run: `grep -n 'pub.*enum CachedApprovalDecision' claudine/lib/src/harness/shell.rs`

If it's not `pub`, make it `pub` and re-export from `claudine/lib/src/harness/mod.rs`. The cache must be constructable by the CLI.

- [ ] **Step 3: Update `CachedHarnessLoopContext` to preserve shared state**

In `claudine/cli/src/commands/wrap/mod.rs`, modify the struct and its methods:

```rust
pub(crate) struct CachedHarnessLoopContext {
    source_path: PathBuf,
    repo_root: Option<PathBuf>,
    shell_options: claudine::harness::ShellApprovalOptions,
    /// Shared approval handler — preserved across source-path refreshes.
    approval_handler: Option<Arc<dyn darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler>>,
    /// Shared approval cache — preserved across source-path refreshes so that
    /// AllowOnce decisions from preflight survive into runtime.
    shared_cache: Arc<std::sync::Mutex<std::collections::HashMap<String, claudine::harness::shell::CachedApprovalDecision>>>,
}

impl CachedHarnessLoopContext {
    fn new(
        source_path: &Path,
        repo_root: Option<&Path>,
        approval_handler: Option<Arc<dyn darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler>>,
        shared_cache: Arc<std::sync::Mutex<std::collections::HashMap<String, claudine::harness::shell::CachedApprovalDecision>>>,
    ) -> Self {
        let shell_options = build_harness_shell_options(
            source_path,
            repo_root,
            approval_handler.clone(),
            Some(Arc::clone(&shared_cache)),
        );
        Self {
            source_path: source_path.to_path_buf(),
            repo_root: repo_root.map(Path::to_path_buf),
            shell_options,
            approval_handler,
            shared_cache,
        }
    }

    fn refresh(&mut self, source_path: &Path, repo_root: Option<&Path>) {
        let repo_root = repo_root.map(Path::to_path_buf);
        if self.source_path != source_path || self.repo_root != repo_root {
            self.source_path = source_path.to_path_buf();
            self.repo_root = repo_root;
            // Rebuild with new policy root but same handler + cache
            self.shell_options = build_harness_shell_options(
                &self.source_path,
                self.repo_root.as_deref(),
                self.approval_handler.clone(),
                Some(Arc::clone(&self.shared_cache)),
            );
        }
    }

    fn resolve_context(&self) -> claudine::harness::HarnessResolutionContext<'_> {
        claudine::harness::HarnessResolutionContext {
            source_path: &self.source_path,
            repo_root: self.repo_root.as_deref(),
        }
    }

    fn shell_options(&self) -> &claudine::harness::ShellApprovalOptions {
        &self.shell_options
    }
}
```

- [ ] **Step 4: Run `cargo check -p claudine-cli`**

Expected: Compilation errors at call sites of `build_harness_shell_options` and `CachedHarnessLoopContext::new` — these will be fixed in the next task.

- [ ] **Step 5: Commit (partial — call sites break is OK at this step)**

Do not commit yet — move to Task 3 to fix the callers.

---

### Task 3: Update All Call Sites in the Plain Wrapper

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs:1124-1141` (plain wrapper preflight)
- Modify: `claudine/cli/src/commands/wrap/mod.rs:2180-2183` (harness loop creation)

The plain wrapper path builds shell options for preflight and again for the harness loop. We must:
1. Create the handler + cache once
2. Use the same cache for both preflight and runtime

- [ ] **Step 1: Create shared handler and cache in plain wrapper preflight**

In `claudine/cli/src/commands/wrap/mod.rs`, around lines 1109-1153, replace the preflight section:

```rust
    let wrapper_harness = if effective_non_interactive {
        let base_prompt =
            extract_prompt_from_child_args(provider, &child_args, stdin_seed.as_deref());
        let harness_source = base_prompt.as_ref().and_then(|_| {
            // ... existing source resolution logic unchanged ...
        });

        if let (Some(base_prompt), Some(source_path)) = (base_prompt, harness_source) {
            let seed = materialize_passthrough_harness_seed(&source_path, base_prompt.clone())?;
            let harness_enabled = claudine::harness::has_harness_properties(&seed.frontmatter);
            if harness_enabled {
                let resolve_ctx = claudine::harness::HarnessResolutionContext {
                    source_path: &source_path,
                    repo_root: env_plan.repo_root.as_deref(),
                };
                let approval_handler = approval::approval_handler_if_interactive();
                let approval_cache = Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
                let shell_options = build_harness_shell_options(
                    &source_path,
                    env_plan.repo_root.as_deref(),
                    approval_handler.clone(),
                    Some(Arc::clone(&approval_cache)),
                );
                let plan = claudine::harness::parse_harness_plan(
                    &seed.frontmatter,
                    &source_path,
                    &resolve_ctx,
                )
                .map_err(|e| eyre!("{e}"))?;

                // Pre-flight harness shell commands — results stay in the cache
                let _harness_preflight = claudine::composition::resolve_shell_approvals(
                    None,
                    None,
                    Some(&plan),
                    &shell_options,
                )
                .map_err(|e| eyre!("{e}"))?;

                drop(plan);

                Some((source_path, base_prompt, seed, approval_handler, approval_cache))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
```

- [ ] **Step 2: Update the harness loop call to pass shared state**

In the same file, update the `run_harness_loop` call site (around line 1158-1200). The tuple now has 5 elements:

```rust
    let exit_code = if let Some((source_path, base_prompt, initial_materialized, approval_handler, approval_cache)) = wrapper_harness
    {
        let mut prompt_state = HarnessPromptState {
            mode: HarnessPromptMode::Passthrough,
            original_ref: source_path.display().to_string(),
            source_path,
            base_prompt: Some(base_prompt),
            overlay: indexmap::IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
        };

        let mut harness_base_args = child_args.clone();
        strip_prompt_from_args(provider, &mut harness_base_args);
        if !use_structured {
            profile.prepare_captured_output(&mut harness_base_args);
        }

        run_harness_loop(
            provider,
            profile,
            binary_path.as_path(),
            child_cwd,
            effective_non_interactive,
            args.timeout,
            &harness_base_args,
            &env_plan.env,
            &mut prompt_state,
            env_plan.repo_root.as_deref(),
            use_structured,
            structured_codex_output.as_ref(),
            stdout_noise,
            stderr_noise,
            profile.suppress_structured_stderr_on_success(),
            !silent_requested,
            stream_verbosity,
            verbose_requested,
            &env_context,
            &dispatch_context,
            Some(initial_materialized),
            approval_handler,
            approval_cache,
            &term,
        )?
    } else if use_structured {
```

- [ ] **Step 3: Update `run_harness_loop` signature to accept shared state**

Add two parameters to `run_harness_loop`:

```rust
pub(crate) fn run_harness_loop(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    effective_non_interactive: bool,
    cli_timeout: Option<u64>,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    prompt_state: &mut HarnessPromptState,
    repo_root: Option<&Path>,
    use_structured: bool,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    suppress_stderr_on_success: bool,
    show_checks: bool,
    stream_verbosity: Verbosity,
    verbose_requested: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    initial_materialized: Option<MaterializedHarnessPrompt>,
    approval_handler: Option<Arc<dyn darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler>>,
    approval_cache: Arc<std::sync::Mutex<std::collections::HashMap<String, claudine::harness::shell::CachedApprovalDecision>>>,
    term: &Terminal,
) -> Result<i32> {
```

- [ ] **Step 4: Wire shared state into CachedHarnessLoopContext inside run_harness_loop**

Replace the `CachedHarnessLoopContext::new` call at the top of `run_harness_loop`:

```rust
    let mut harness_context = CachedHarnessLoopContext::new(
        &prompt_state.source_path,
        repo_root,
        approval_handler,
        approval_cache,
    );
```

- [ ] **Step 5: Run `cargo check -p claudine-cli`**

Expected: Compilation error only in `composition.rs` — fixed in Task 4.

---

### Task 4: Update Call Sites in the Composition Wrapper

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition.rs:426-463` (composition preflight)
- Modify: `claudine/cli/src/commands/wrap/composition.rs:558-581` (harness loop call)

The composition wrapper also calls `build_harness_shell_options` and `run_harness_loop`. Apply the same shared-state pattern.

- [ ] **Step 1: Create shared handler and cache in composition preflight**

In `claudine/cli/src/commands/wrap/composition.rs`, update the harness preflight section (around lines 421-463):

```rust
    if harness_enabled {
        let resolve_ctx = claudine::harness::HarnessResolutionContext {
            source_path: &request.prepared.resolved_path,
            repo_root: effective_repo_root,
        };
        let approval_handler = approval::approval_handler_if_interactive();
        let approval_cache = Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        let shell_options = build_harness_shell_options(
            &request.prepared.resolved_path,
            effective_repo_root,
            approval_handler.clone(),
            Some(Arc::clone(&approval_cache)),
        );
        let mut plan = claudine::harness::parse_harness_plan(
            &request.prepared.effective_frontmatter,
            &request.prepared.resolved_path,
            &resolve_ctx,
        )
        .map_err(|e| eyre!("{e}"))?;

        if is_inline {
            plan.pre_checks.insert(
                0,
                claudine::harness::inline_writability_pre_check(&request.prepared.resolved_path),
            );
        }

        let harness_preflight = claudine::composition::resolve_shell_approvals(
            None,
            None,
            Some(&plan),
            &shell_options,
        )
        .map_err(|e| eyre!("{e}"))?;

        if !request.quiet && !request.silent && harness_preflight.total_discovered > 0 {
            log::info(&format!(
                "Pre-flight: {} harness shell command(s) approved",
                harness_preflight.total_discovered,
            ));
        }

        drop(plan);
        // approval_handler and approval_cache are moved to the harness loop below
    }
```

Note: `approval_handler` and `approval_cache` must be visible at the `run_harness_loop` call site. They need to be declared outside the `if harness_enabled` block. Use `Option` wrapping or declare before the block and conditionally populate.

The exact approach depends on the existing control flow. The implementer should declare `approval_handler` and `approval_cache` at the function scope before the `if harness_enabled` block:

```rust
    let approval_handler = approval::approval_handler_if_interactive();
    let approval_cache: Arc<std::sync::Mutex<std::collections::HashMap<String, claudine::harness::shell::CachedApprovalDecision>>> =
        Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
```

Then use them in both the preflight block and the `run_harness_loop` call.

- [ ] **Step 2: Pass shared state to `run_harness_loop` in composition**

Update the `run_harness_loop` call in `composition.rs` (around line 558) to pass the two new parameters before `&term`:

```rust
        run_harness_loop(
            provider,
            profile,
            // ... existing args ...
            Some(materialized_harness_prompt_from_prepared(&request.prepared)),
            approval_handler,      // new
            approval_cache,        // new
            &term,
        )
```

- [ ] **Step 3: Add necessary imports to composition.rs**

Add `use std::sync::Arc;` and the `use super::approval;` import at the top of `composition.rs` if not already present.

- [ ] **Step 4: Run `cargo check -p claudine-cli`**

Expected: Clean compilation

- [ ] **Step 5: Run all existing tests**

Run: `just test` from `claudine/`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs claudine/cli/src/commands/wrap/composition.rs claudine/lib/src/harness/shell.rs
git commit -m "feat(claudine): wire interactive shell approval and share cache between preflight and runtime"
```

---

### Task 5: Remove Redundant Runtime Audit Re-discovery

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs:2232-2260` (runtime shell audit in harness loop)

The harness loop currently re-reads source text and re-runs full audit from scratch. With the shared cache, pre-approved commands will hit the cache and pass. However, the loop still reads the source file for `collect_auditable_commands(plan, source_text)` which discovers `ComposeSourceLine` commands — these were already approved during the compose stage.

The key change: since the approval cache is now shared, the runtime audit's `validate_and_approve_command_parts` calls will find cached `Allowed` entries for commands that preflight approved. No code change is needed in the audit infrastructure — the cache sharing is sufficient.

However, we should verify this works and clean up the unnecessary `_harness_preflight` binding.

- [ ] **Step 1: Write a test proving cache sharing works end-to-end**

In `claudine/lib/src/composition/preflight.rs`, add a test to the existing `tests` module:

```rust
    #[test]
    fn preflight_populates_cache_for_runtime_reuse() {
        use claudine::harness::shell::{CachedApprovalDecision, ShellApprovalOptions};
        use darkmatter::markdown::compose::shell_expansion::{
            ShellApprovalDecision as Decision,
            ShellApprovalHandler,
            ShellApprovalRequest,
            ShellExpansionError,
        };
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        struct AllowOnceHandler {
            call_count: AtomicUsize,
        }
        impl ShellApprovalHandler for AllowOnceHandler {
            fn approve(
                &self,
                _request: ShellApprovalRequest,
            ) -> Result<Decision, ShellExpansionError> {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Ok(Decision::AllowOnce)
            }
        }

        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(AllowOnceHandler {
            call_count: AtomicUsize::new(0),
        });
        let cache = Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            approval_cache: Arc::clone(&cache),
        };

        // Build a plan with a shell_command pre-check
        let plan = HarnessPlan {
            source_path: dir.path().join("test.md"),
            timeout: None,
            pre_checks: vec![ValidationRule {
                id: ValidationRuleId(0),
                event: ValidationEvent::ShellCommand,
                phase: ValidationPhase::Both,
                kind: ValidationKind::ShellCommand {
                    command: ApprovedRuntimeCommand {
                        raw: "echo hello".to_string(),
                        executable: "echo".to_string(),
                        args: vec!["hello".to_string()],
                    },
                    show_stdout: true,
                    show_stderr: true,
                },
                message_template: None,
                subject_key: None,
            }],
            post_checks: vec![],
            handlers: HandlerTable::default(),
            programmatic_handler: None,
        };

        // First call: preflight should invoke the handler
        let result = resolve_shell_approvals(None, None, Some(&plan), &options).unwrap();
        assert_eq!(result.total_discovered, 1);
        assert_eq!(handler.call_count.load(Ordering::SeqCst), 1);

        // Second call: same cache — handler should NOT be invoked again
        let result2 = resolve_shell_approvals(None, None, Some(&plan), &options).unwrap();
        assert_eq!(result2.total_discovered, 1);
        assert_eq!(
            handler.call_count.load(Ordering::SeqCst),
            1,
            "handler should not be called again — cache should provide the answer"
        );

        // Verify cache contains the approved command
        let cache_lock = cache.lock().unwrap();
        assert!(
            cache_lock.values().any(|d| matches!(d, CachedApprovalDecision::Allowed)),
            "cache should contain at least one Allowed entry"
        );
    }
```

- [ ] **Step 2: Run the new test**

Run: `cargo test -p claudine -- preflight_populates_cache_for_runtime_reuse`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/composition/preflight.rs
git commit -m "test(claudine): verify preflight cache is shared with runtime audit"
```

---

### Task 6: Integration Test — Approval Flow End-to-End

**Files:**
- Test: `claudine/cli/tests/wrap_commands.rs` (extend existing)

Add a test that proves: a harness source with a non-whitelisted shell command triggers the approval handler, and the approval persists into the runtime harness loop without double-prompting.

- [ ] **Step 1: Write the integration test**

In `claudine/cli/tests/wrap_commands.rs`, add a test that uses a harness source file with a `shell_command` pre-check pointing to a non-whitelisted command, run with `--dry-run` or a mock provider, and verify:
- The approval prompt is shown exactly once
- The shell audit passes on the runtime side

This test requires careful setup because the approval handler reads from stdin. The test should provide simulated input via stdin piping.

```rust
#[test]
fn shell_approval_prompted_once_for_non_whitelisted_command() {
    // This test verifies that:
    // 1. A non-whitelisted shell_command triggers the interactive prompt
    // 2. The prompt appears only once (not during both preflight and runtime)
    //
    // Setup: create a harness source file with a shell_command pre-check
    // that points to a non-whitelisted command, run the wrapper with
    // simulated stdin input ("3\n" = AllowOnce), and verify the output
    // shows exactly one "Shell Approval Required" prompt.

    let dir = tempdir().unwrap();
    let source = dir.path().join("test.md");
    fs::write(
        &source,
        r#"---
pre_checks:
  - shell_command: "echo verification-check"
---
# Test prompt
Tell me hello.
"#,
    )
    .unwrap();

    // Create an isolated policy directory with no whitelist
    let policy_dir = dir.path().join(".darkmatter-shell-whitelist");
    // Intentionally do NOT create this file — command is not whitelisted

    let mut cmd = cargo_bin_cmd("claudine");
    cmd.args(["wrap", "echo", "--prompt-file", source.to_str().unwrap()])
        .env("HOME", dir.path())
        .write_stdin("3\n"); // AllowOnce

    // The command will fail because "echo" is not a real provider,
    // but we can verify the approval flow from stderr
    let output = cmd.output().expect("failed to execute");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Count approval prompts — should appear at most once
    let approval_count = stderr.matches("Shell Approval Required").count();
    assert!(
        approval_count <= 1,
        "Expected at most 1 approval prompt, got {approval_count}.\nStderr:\n{stderr}"
    );
}
```

Note: This test's exact shape depends on how the wrapper handles unknown providers and dry-run modes. The implementer should adjust the provider arg, flags, and assertions based on the actual CLI behavior. The key assertion is that the approval prompt count is ≤ 1.

- [ ] **Step 2: Run the integration test**

Run: `cargo test -p claudine-cli --test wrap_commands -- shell_approval_prompted_once`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add claudine/cli/tests/wrap_commands.rs
git commit -m "test(claudine): integration test for single-prompt shell approval flow"
```

---

## Verification Checklist

After all tasks are complete, verify:

- [ ] `cargo check -p claudine -p claudine-cli` — clean compilation
- [ ] `just test` from `claudine/` — all tests pass
- [ ] `just lint` from `claudine/` — no new warnings
- [ ] Approval handler unit tests pass: `cargo test -p claudine-cli -- approval`
- [ ] Cache sharing test passes: `cargo test -p claudine -- preflight_populates_cache`
- [ ] Integration test passes: `cargo test -p claudine-cli --test wrap_commands`
