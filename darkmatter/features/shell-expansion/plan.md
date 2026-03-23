---
feature: shell-expansion
spec: darkmatter/features/shell-expansion/spec.md
tech_design: darkmatter/features/shell-expansion/tech-design.md
created: 2026-03-15
---

# Shell Expansion Implementation Plan

This plan implements the `::shell` directive feature for Darkmatter's Stage 1 compose pipeline. It is organized into three phases matching the tech design's phasing, with each task specifying the exact files to create or modify and the acceptance criteria.

## Dependencies

- **New crate dependency**: `which` (executable lookup) — add to `darkmatter/lib/Cargo.toml`
- **Existing crates used**: `thiserror`, `regex`, `tracing`, `tokio` (for timeout threading), `std::process::Command`

## Phase 1: Types, Parser, Tokenizer, Blacklist, Policy Store

Phase 1 establishes all foundational types and the parsing/validation layer. No commands are executed yet. All tasks in this phase can be implemented and tested independently of the pipeline.

### Task 1.1: Shell Expansion Types

**File**: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` (new)

Create the core type definitions:

```rust
// ShellDirective — parsed directive with raw_command, executable, args, span, line
pub struct ShellDirective {
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub span: std::ops::Range<usize>,
    pub line: usize,
}

// ShellExpansionOptions — timeout, policy_root, working_directory, approval_handler
pub struct ShellExpansionOptions {
    pub timeout: std::time::Duration,
    pub policy_root: Option<std::path::PathBuf>,
    pub working_directory: Option<std::path::PathBuf>,
    pub approval_handler: Option<std::sync::Arc<dyn ShellApprovalHandler>>,
}
// Default: timeout=10s, all others None

// ShellApprovalHandler trait (Send + Sync)
pub trait ShellApprovalHandler: Send + Sync {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError>;
}

// ShellApprovalRequest — source, line, raw_command, executable, args, normalized_exact, whitelist_path, blacklist_path
pub struct ShellApprovalRequest { ... }

// ShellApprovalDecision — AllowExactPersist, AllowCommandPersist, AllowOnce, Deny, BlacklistPersist
pub enum ShellApprovalDecision { ... }

// ShellExpansionError — thiserror enum with variants:
//   ParseDirective, CommandNotFound, Blacklisted, ApprovalRequired, Denied, Timeout, ExecutionFailed, PolicyIo
pub enum ShellExpansionError { ... }

// ShellPolicyPaths — whitelist: PathBuf, blacklist: PathBuf
pub struct ShellPolicyPaths { ... }

// ShellRuleEntry — Exact(String), Prefix(String)
pub enum ShellRuleEntry { ... }

// ShellRuleSet — entries: Vec<ShellRuleEntry>, with match methods
pub struct ShellRuleSet { ... }

// ShellExpansionRuntime — allow_once: HashSet<String>, whitelist: ShellRuleSet, user_blacklist: ShellRuleSet, policy_paths: Option<ShellPolicyPaths>, approvals_used: usize
pub(crate) struct ShellExpansionRuntime { ... }

// PipelineRuntime — transclusion: TransclusionRuntime, shell: ShellExpansionRuntime
pub(crate) struct PipelineRuntime { ... }

// BlacklistRule — Executable, ExecutablePrefix, SubcommandPrefix, ArgExact, ArgPrefix, RawToken
pub enum BlacklistRule { ... }
```

**Acceptance criteria**:
- All types compile
- `ShellExpansionOptions::default()` returns timeout=10s, all others None
- `ShellExpansionError` implements `std::fmt::Display` and `std::error::Error` via thiserror
- `ShellRuleSet` has `matches_exact(&self, normalized: &str) -> bool` and `matches_prefix(&self, executable: &str) -> bool`
- `ShellExpansionRuntime::take_recent_approval_count()` returns count and resets to 0
- `PipelineRuntime::new(max_depth)` creates both sub-runtimes

### Task 1.2: Shell-like Argv Tokenizer

**File**: `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs` (new)

Implement a tokenizer that splits a raw command string into tokens:

**Supported**:
- Unquoted tokens (split on whitespace)
- Single-quoted strings (literal content, no escaping inside)
- Double-quoted strings (backslash escaping for `\`, `"`, and space)
- Backslash escaping outside quotes (for spaces, quotes, backslashes)

**Rejected** (return error):
- Empty command lines
- Unterminated quotes
- Raw metacharacters outside quotes: `>`, `>>`, `<`, `|`, `;`, `&&`, `||`, `` ` ``, `$(`

**Public API**:
```rust
pub fn tokenize(input: &str) -> Result<Vec<String>, ShellExpansionError>
```

Returns the first token as the executable and remaining as args.

**Acceptance criteria**:
- `tokenize("git status --short")` → `["git", "status", "--short"]`
- `tokenize("rg \"TODO|FIXME\" src")` → `["rg", "TODO|FIXME", "src"]`
- `tokenize("echo 'hello world'")` → `["echo", "hello world"]`
- `tokenize("echo hello\\ world")` → `["echo", "hello world"]`
- `tokenize("")` → error (empty command)
- `tokenize("echo 'unterminated")` → error (unterminated quote)
- `tokenize("echo foo | bar")` → error (pipe metacharacter)
- `tokenize("echo foo > out.txt")` → error (redirect metacharacter)
- `tokenize("echo $(whoami)")` → error (command substitution)
- `tokenize("echo foo; rm -rf /")` → error (semicolon)
- `tokenize("echo foo && echo bar")` → error (`&&`)

### Task 1.3: Directive Parser

**File**: `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs` (new)

Implement directive scanning that finds `::shell` directives in markdown content:

**Algorithm**:
1. Call `parse_utils::find_code_regions(content)` to get excluded byte ranges
2. Scan content line-by-line, tracking byte offsets
3. For each line not inside a code region, check if `line.trim()` starts with `::shell `
4. If matched, extract the remainder as raw command text
5. Tokenize the raw command text into executable and args
6. Build a `ShellDirective` with the span covering the full line (including any trailing newline)

**Public API**:
```rust
pub fn parse_directives(content: &str) -> Result<Vec<ShellDirective>, ShellExpansionError>
```

**Acceptance criteria**:
- Finds directives on plain lines
- Ignores directives inside fenced code blocks (``` and ~~~)
- Handles leading whitespace: `  ::shell git status` is valid
- Captures correct byte spans for replacement
- Returns directives in source order
- Propagates tokenizer errors
- Returns empty vec when no directives found

### Task 1.4: Built-in Blacklist

**File**: `darkmatter/lib/src/markdown/compose/shell_expansion/policy.rs` (new)

Implement the built-in blacklist as a static array of `BlacklistRule` values and a matching function.

**Built-in rules** (from spec, encoded as structured rules):

| Category | Examples | Rule Type |
|----------|----------|-----------|
| Destructive file ops | `rm`, `rmdir`, `shred` | `Executable` |
| Disk format | `mkfs*`, `fdisk`, `parted` | `ExecutablePrefix` / `Executable` |
| Permission/ownership | `chmod`, `chown`, `chgrp` | `Executable` |
| System control | `shutdown`, `reboot`, `halt`, `init`, `systemctl` | `Executable` |
| Package managers | `apt`, `yum`, `dnf`, `brew`, `pacman`, `pip`, `npm`, `cargo` | `Executable` |
| Dangerous git | `git reset`, `git clean`, `git push --force` | `SubcommandPrefix` / `ArgExact` |
| Container ops | `docker system prune`, `docker rm`, `docker rmi` | `SubcommandPrefix` |
| SQL destructive | `psql -c "DROP"`, `mysql -e "DROP"` | `ArgPrefix` |
| Network tools | `nc`, `ncat`, `curl -X DELETE` | `Executable` / `ArgExact` |
| Process control | `kill`, `killall`, `pkill` | `Executable` |
| Disk ops | `dd` | `Executable` |
| Editors (interactive) | `vi`, `vim`, `nano`, `emacs` | `Executable` |
| Shell interpreters | `sh`, `bash`, `zsh`, `fish`, `cmd`, `powershell`, `pwsh` | `Executable` |
| Redirect tokens | `>`, `>>` | `RawToken` |

**Also implement interpreter-wrapper rejection** (from tech design):
- `sh -c`, `bash -c`, `bash -lc`, `zsh -c`, `fish -c`, `cmd /C`, `powershell -Command`, `pwsh -Command`

**Public API**:
```rust
pub fn check_builtin_blacklist(executable: &str, args: &[String]) -> Option<String>
// Returns Some(reason) if blacklisted, None if allowed

pub fn check_user_blacklist(ruleset: &ShellRuleSet, executable: &str, args: &[String], normalized: &str) -> bool
// Returns true if blacklisted

pub fn check_whitelist(ruleset: &ShellRuleSet, executable: &str, normalized: &str) -> bool
// Returns true if whitelisted

pub fn normalize_command(executable: &str, args: &[String]) -> String
// Produces a stable normalized string for exact matching
```

**Acceptance criteria**:
- `rm` is blacklisted
- `rm -rf /` is blacklisted (executable match)
- `git reset --hard` is blacklisted
- `git status` is NOT blacklisted
- `docker system prune -af` is blacklisted
- `find . -delete` is blacklisted
- `bash -c 'echo hello'` is blacklisted (interpreter wrapper)
- `bash --version` is blacklisted (shell executable)
- `echo hello` is NOT blacklisted
- `rg TODO src` is NOT blacklisted
- Raw `>` token detected in args

### Task 1.5: Policy File Store

**File**: `darkmatter/lib/src/markdown/compose/shell_expansion/store.rs` (new)

Implement policy file discovery, loading, and persistence.

**Discovery** (`resolve_policy_paths`):
1. Use `options.shell.policy_root` if set
2. Otherwise use `ComposeSource::File` parent directory if available
3. Otherwise use `std::env::current_dir()`
4. If path is inside a git repo, walk up to find `.git` and use that directory
5. Otherwise use `${HOME}`
6. Return `ShellPolicyPaths { whitelist: root/.darkmatter-shell-whitelist, blacklist: root/.darkmatter-shell-blacklist }`

**Loading** (`load_ruleset`):
- Parse line-oriented format: `exact <command>` and `prefix <command>`
- Skip blank lines and `#` comments
- Deduplicate entries in memory
- Return `ShellRuleSet`

**Persistence** (`append_entry`):
- Append a single normalized line to the file
- Create the file if it doesn't exist
- Always write a trailing newline

**Normalization**:
- Use executable basename (not resolved path)
- Quote args only when they contain whitespace or special characters
- Use the same tokenizer for round-trip safety

**Public API**:
```rust
pub fn resolve_policy_paths(
    shell_opts: &ShellExpansionOptions,
    source: &ComposeSource,
) -> Result<ShellPolicyPaths, ShellExpansionError>

pub fn load_ruleset(path: &Path) -> Result<ShellRuleSet, ShellExpansionError>

pub fn append_whitelist_exact(paths: &ShellPolicyPaths, normalized: &str) -> Result<(), ShellExpansionError>
pub fn append_whitelist_prefix(paths: &ShellPolicyPaths, executable: &str) -> Result<(), ShellExpansionError>
pub fn append_blacklist_exact(paths: &ShellPolicyPaths, normalized: &str) -> Result<(), ShellExpansionError>
```

**Acceptance criteria**:
- Loads `exact` and `prefix` entries correctly
- Ignores comments and blank lines
- Deduplicates entries
- Appends entries with correct format
- Creates file on first append
- Git root detection works (walk up looking for `.git`)
- Falls back to HOME when not in a git repo
- Uses `policy_root` override when set
- Round-trips through tokenizer (appended entries can be loaded back)

### Task 1.6: Module Scaffold and Type Exports

**Files modified**:
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs` (new) — module declarations, re-exports
- `darkmatter/lib/src/markdown/compose/mod.rs` — add `pub mod shell_expansion;` and re-exports
- `darkmatter/lib/src/markdown/compose/types.rs` — add `shell_expansion: bool` to `Stage1Stages`, add `shell: ShellExpansionOptions` to `ComposeOptions`, add report fields
- `darkmatter/lib/src/markdown/types.rs` — add `ShellExpansion` variant to `MarkdownError`
- `darkmatter/lib/Cargo.toml` — add `which = "7"` dependency

**Changes to `Stage1Stages`**:
```rust
pub struct Stage1Stages {
    pub replacement: bool,
    pub interpolation: bool,
    pub toc_linking: bool,
    pub shell_expansion: bool,  // NEW — default true
    pub cleanup: bool,
    pub normalization: bool,
}
```
- Update `Default`, `none()`, and add `only_shell_expansion()`
- Update existing `only_*` methods to set `shell_expansion: false`

**Changes to `ComposeOptions`**:
```rust
pub struct ComposeOptions {
    // ... existing fields ...
    pub shell: ShellExpansionOptions,  // NEW
}
```
- Update `new()` to initialize `shell: ShellExpansionOptions::default()`
- Add `with_shell(mut self, shell: ShellExpansionOptions) -> Self` builder

**Changes to `ComposeReport`**:
```rust
pub struct ComposeReport {
    // ... existing fields ...
    pub shell_expansions_applied: usize,   // NEW
    pub shell_approvals_used: usize,       // NEW
}
```
- Update `has_changes()` to include `shell_expansions_applied > 0`
- Update `summary()` to include shell expansion and approval counts

**Changes to `MarkdownError`**:
```rust
pub enum MarkdownError {
    // ... existing variants ...
    #[error("Shell expansion failed: {0}")]
    ShellExpansion(#[from] ShellExpansionError),
}
```

**Acceptance criteria**:
- All existing tests pass (no regressions from type changes)
- New `shell_expansion` field defaults to `true` in `Stage1Stages`
- `ComposeOptions::new()` includes default `ShellExpansionOptions`
- `ComposeReport::has_changes()` returns true when `shell_expansions_applied > 0`
- `ComposeReport::summary()` includes shell expansion counts
- `which` crate is available as a dependency
- `ShellExpansionError` converts into `MarkdownError` via `From`

### Task 1.7: Phase 1 Tests

**File**: `darkmatter/lib/src/markdown/compose/shell_expansion/` — tests within each module file

Write unit tests for all Phase 1 components:

**Tokenizer tests** (in `tokenize.rs`):
- All acceptance criteria from Task 1.2

**Parser tests** (in `parser.rs`):
- All acceptance criteria from Task 1.3

**Blacklist tests** (in `policy.rs`):
- All acceptance criteria from Task 1.4

**Store tests** (in `store.rs`):
- All acceptance criteria from Task 1.5
- Use `tempfile` for file I/O tests
- Use `serial_test` for tests that manipulate cwd or env vars

**Type tests** (in `types.rs`):
- `ShellRuleSet` matching behavior
- `ShellExpansionRuntime::take_recent_approval_count()`
- `normalize_command` round-trip stability

**Acceptance criteria**:
- All tests pass with `just test` from the darkmatter directory
- No existing tests regress

---

## Phase 2: Executor, Pipeline Runtime, Pipeline Integration

Phase 2 connects the parsing and validation layer to actual command execution and wires it into the compose pipeline.

### Task 2.1: Command Executor

**File**: `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs` (new)

Implement command execution with timeout and output capture.

**Algorithm**:
1. Resolve executable path with `which::which(executable)`
2. Build `std::process::Command` with resolved path, args, working_dir, stdin=null, stdout=piped, stderr=piped
3. Spawn the child process
4. Spawn threads to drain stdout and stderr concurrently
5. Poll `try_wait()` in a loop with small sleeps until completion or timeout
6. On timeout: kill child, wait, return `ShellExpansionError::Timeout`
7. On completion: join output threads, check exit code
8. Exit code 0: return combined output (stdout followed by stderr, both trimmed of trailing newline only if output would otherwise end with double newline)
9. Exit code non-zero: return `ShellExpansionError::ExecutionFailed` with stdout, stderr, code
10. Decode all bytes with `String::from_utf8_lossy()`

**Working directory resolution**:
1. `options.shell.working_directory` if set
2. Source file's parent directory if `ComposeSource::File`
3. `options.shell.policy_root` if set
4. `std::env::current_dir()`

**Public API**:
```rust
pub fn resolve_working_directory(
    shell_opts: &ShellExpansionOptions,
    source: &ComposeSource,
) -> PathBuf

pub fn execute_command(
    directive: &ShellDirective,
    shell_opts: &ShellExpansionOptions,
    source: &ComposeSource,
) -> Result<String, ShellExpansionError>
```

**Acceptance criteria**:
- `echo hello` returns `"hello\n"`
- `printf ""` (or equivalent empty-output command) returns `""`
- A command that writes to stderr only still captures that output
- Non-zero exit produces `ExecutionFailed` with captured streams
- Timeout kills the child (test with `sleep 60` and 1s timeout override)
- `CommandNotFound` for non-existent executables
- stdin is null (no hanging on interactive commands)
- Working directory resolution follows the documented priority

### Task 2.2: Pipeline Runtime Refactor

**Files modified**:
- `darkmatter/lib/src/markdown/compose/mod.rs` — replace `TransclusionRuntime` with `PipelineRuntime`
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs` — `PipelineRuntime` already defined in Task 1.1

**Changes to `run_compose_pipeline`**:
```rust
fn run_compose_pipeline(&mut self, options: ComposeOptions) -> MarkdownResult<ComposeReport> {
    let mut runtime = PipelineRuntime::new(options.transclusion.max_depth);
    self.run_compose_pipeline_internal(options, &mut runtime)
}
```

**Changes to `run_compose_pipeline_internal`**:
```rust
pub(crate) fn run_compose_pipeline_internal(
    &mut self,
    options: ComposeOptions,
    runtime: &mut PipelineRuntime,
) -> MarkdownResult<ComposeReport> {
    // ... existing source_id logic using runtime.transclusion ...
    // ... existing stages ...
    // NEW: shell expansion stage between TOC linking and cleanup
    // ... existing Stage 2 using runtime.transclusion ...
}
```

**Key requirement**: All existing call sites that pass `&mut TransclusionRuntime` must now pass `&mut PipelineRuntime`. The transclusion runtime is accessed via `runtime.transclusion`. This includes:
- `run_block_transclusion_stage` and its recursive calls
- `run_frontmatter_transclusion_stage`

**Acceptance criteria**:
- All existing compose tests pass without modification
- All existing transclusion tests pass
- `PipelineRuntime` is created once at the root and threaded through recursion
- The transclusion runtime behavior is identical (just accessed through `.transclusion`)

### Task 2.3: Wire Shell Expansion Stage into Pipeline

**File modified**: `darkmatter/lib/src/markdown/compose/mod.rs`

Insert the shell expansion stage call between TOC linking and cleanup:

```rust
// Stage 1: Shell Expansion (after TOC linking, before cleanup)
if options.stages.shell_expansion {
    self.run_shell_expansion_stage(&options, runtime, &mut report)?;
}
```

**Implement `run_shell_expansion_stage`** as a method on `Markdown`:
```rust
fn run_shell_expansion_stage(
    &mut self,
    options: &ComposeOptions,
    runtime: &mut PipelineRuntime,
    report: &mut ComposeReport,
) -> MarkdownResult<()> {
    let directives = shell_expansion::parse_directives(&self.content)?;
    if directives.is_empty() {
        return Ok(());
    }

    let policy_paths = shell_expansion::resolve_policy_paths(
        &options.shell,
        &options.transclusion.source,
    )?;
    runtime.shell.ensure_loaded(&policy_paths)?;

    let mut replacements = Vec::new();

    for directive in directives {
        // 1. Check builtin blacklist
        // 2. Check user blacklist
        // 3. Check whitelist (exact then prefix)
        // 4. Check runtime allow_once
        // 5. Request approval or return ApprovalRequired
        // 6. Handle approval decision (persist, allow-once, deny, blacklist)
        // 7. Execute command
        // 8. Collect replacement

        let replacement = shell_expansion::execute_directive(
            &directive,
            options,
            &policy_paths,
            &mut runtime.shell,
        )?;
        replacements.push((directive.span.clone(), replacement));
        report.shell_expansions_applied += 1;
    }

    shell_expansion::apply_replacements_in_reverse(&mut self.content, replacements);
    report.shell_approvals_used += runtime.shell.take_recent_approval_count();
    Ok(())
}
```

**Implement `execute_directive`** in `shell_expansion/mod.rs`:
This is the core orchestration function that runs the validation model steps 1-9 from the tech design, then calls the executor.

**Implement `apply_replacements_in_reverse`** in `shell_expansion/mod.rs`:
Sort replacements by span start descending, then apply `content.replace_range(span, &replacement)` for each.

**Acceptance criteria**:
- `::shell echo hello` in a document is replaced with `hello\n`
- `::shell echo hello` inside a fenced code block is NOT replaced
- Blacklisted commands fail the pipeline
- Whitelisted commands execute without approval
- Commands not in whitelist fail with `ApprovalRequired` when no handler is set
- Empty output removes the directive line
- Non-zero exit fails the pipeline with captured output
- Report counts are updated correctly
- Shell expansion runs after interpolation (so `{{ var }}` in command args works)
- Shell expansion runs before cleanup (so output gets cleaned up)

### Task 2.4: Phase 2 Tests

**Files**: tests within module files + integration tests

**Unit tests for executor** (in `executor.rs`):
- All acceptance criteria from Task 2.1

**Integration tests for pipeline** (in `darkmatter/lib/tests/` or inline):
- Interpolation feeds into shell expansion: `::shell echo {{ var }}` with frontmatter `var: hello` → `hello\n`
- Cleanup runs after shell expansion
- Blacklisted command in document fails pipeline
- Whitelisted command succeeds
- Empty output removes directive
- Non-zero exit fails with error details
- Multiple directives in one document all execute
- Directives in code blocks are skipped
- Report counts are correct
- `fail_fast` does NOT affect shell errors (always hard failure)

**Runtime sharing tests**:
- Allow-once persists across documents in the same compose run (requires a transclusion test with child documents containing `::shell`)

**Acceptance criteria**:
- All Phase 1 tests still pass
- All new tests pass
- No regressions in existing compose/transclusion tests

---

## Phase 3: CLI Approval Handler, CLI Integration, Documentation

Phase 3 adds the interactive CLI experience and finalizes documentation.

### Task 3.1: CLI Approval Handler

**File**: `darkmatter/cli/src/commands/compose/approval.rs` (new, or inline in compose command)

Implement `CliShellApprovalHandler` that implements `ShellApprovalHandler`:

**Prompt format** (on stderr):
```text
Unapproved shell command in {source}:{line}
  {raw_command}

1. allow exact and save
2. allow command and save
3. allow once
4. deny
5. blacklist and stop
```

**Prompting conditions** (all must be true):
1. Input is a file path, not stdin
2. stdin is a terminal (`std::io::stdin().is_terminal()`)
3. stderr is a terminal (`std::io::stderr().is_terminal()`)

**Non-interactive error guidance**:
When prompting is not possible and a command needs approval, the CLI error message should include:
- The command
- The whitelist file path
- The exact `exact <normalized>` and `prefix <executable>` entries the user can add manually

**Public API**:
```rust
pub struct CliShellApprovalHandler;

impl ShellApprovalHandler for CliShellApprovalHandler {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError>;
}
```

**Acceptance criteria**:
- Prompts on stderr, reads from stdin
- Returns correct `ShellApprovalDecision` variant for each choice
- Invalid input re-prompts
- Works correctly when stdin is a terminal

### Task 3.2: CLI Compose Integration

**File modified**: `darkmatter/cli/src/commands.rs` (or `commands/compose.rs`)

Update `run_compose()`:
1. Detect if prompting is safe (file input + terminal stdin + terminal stderr)
2. If safe: set `approval_handler = Some(Arc::new(CliShellApprovalHandler))`
3. If not safe: leave `approval_handler = None`
4. Build `ShellExpansionOptions` with appropriate defaults
5. Pass shell options into `ComposeOptions` via `.with_shell(shell_opts)`

**Acceptance criteria**:
- `md compose file.md` with a `::shell` directive and no whitelist entry prompts the user
- `md compose -` with a `::shell` directive and no whitelist entry fails with manual guidance
- Persisted approvals work on subsequent runs
- Shell expansion can be disabled via Stage1Stages toggle (future CLI flag if needed)

### Task 3.3: CLI Integration Tests

**File**: `darkmatter/cli/tests/cli.rs` (or new test file)

**Test cases**:
1. `md compose file.md` with whitelisted command succeeds
2. `md compose file.md` with blacklisted command fails with blacklist error
3. `md compose -` (stdin) with unapproved command fails with manual whitelist guidance in error message
4. Persisted approval in `.darkmatter-shell-whitelist` is reused on later runs
5. `md compose file.md` with non-existent command fails with "does not exist" error

Use `tempfile` for isolated test directories with `.darkmatter-shell-whitelist` files.
Use `assert_cmd` for CLI invocation testing.

**Acceptance criteria**:
- All CLI test cases pass
- Tests use isolated temp directories to avoid side effects

### Task 3.4: Documentation Updates

**Files modified**:
- `darkmatter/docs/dependencies.md` — add `which` crate entry
- `darkmatter/lib/README.md` — mention shell expansion in feature list if applicable
- `darkmatter/docs/darkmatter-compose-pipeline.md` — update Stage 1 order to include shell expansion

**Acceptance criteria**:
- `which` crate documented with version and purpose
- Pipeline documentation reflects the new stage ordering

---

## Cross-Cutting Concerns

### Error Handling
- All shell expansion errors are **hard failures** — they never degrade to warnings even when `fail_fast = false`
- `ShellExpansionError` converts to `MarkdownError::ShellExpansion` via `From`
- Error messages include the line number and command for debugging

### Tracing
- Add `tracing::debug!` for directive discovery
- Add `tracing::info!` for command execution (executable, args, working dir)
- Add `tracing::warn!` for blacklist rejections and approval requests
- Follow existing darkmatter tracing patterns

### Security
- Never invoke shell interpreters (no `sh -c`, `bash -c`, etc.)
- Reject shell metacharacters at the tokenizer level
- Blacklist checks always run before approval checks
- stdin is always null for spawned processes
- Kill child processes on timeout

### Backward Compatibility
- `shell_expansion` defaults to `true` in `Stage1Stages`, but no existing documents use `::shell` directives, so this is safe
- `PipelineRuntime` replaces `TransclusionRuntime` in the internal API, but the public API is unchanged
- All existing builder patterns continue to work because new fields have defaults

---

## Task Dependency Graph

```
Phase 1 (no external dependencies between tasks):
  1.1 Types
  1.2 Tokenizer (depends on 1.1 for error type)
  1.3 Parser (depends on 1.1, 1.2)
  1.4 Blacklist/Policy (depends on 1.1, 1.2)
  1.5 Store (depends on 1.1, 1.2, 1.4)
  1.6 Module Scaffold (depends on 1.1-1.5)
  1.7 Phase 1 Tests (depends on 1.1-1.6)

Phase 2 (depends on Phase 1):
  2.1 Executor (depends on 1.1)
  2.2 Pipeline Runtime Refactor (depends on 1.1, 1.6)
  2.3 Wire Stage (depends on 2.1, 2.2, 1.3, 1.4, 1.5)
  2.4 Phase 2 Tests (depends on 2.1-2.3)

Phase 3 (depends on Phase 2):
  3.1 CLI Approval Handler (depends on 1.1)
  3.2 CLI Compose Integration (depends on 2.3, 3.1)
  3.3 CLI Tests (depends on 3.2)
  3.4 Documentation (depends on all)
```

## Execution Strategy

- **Phase 1** tasks 1.1-1.5 can be implemented in sequence within a single agent since they build on each other incrementally
- **Phase 1** task 1.6 (scaffold) ties everything together and should be done after 1.1-1.5
- **Phase 2** task 2.2 (runtime refactor) is the highest-risk change because it touches recursive pipeline plumbing — test thoroughly against existing tests before proceeding
- **Phase 3** is the lowest-risk phase since it only adds CLI-layer code on top of a working library
