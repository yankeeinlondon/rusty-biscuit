# Specification: Better Shell Command Parsing

This document defines the technical strategy for enhancing shell expansion parsing in `darkmatter`. It enables support for common redirection patterns and logical command chaining while maintaining security through explicit Rust-side orchestration.

## 1. Problem Statement

The current tokenizer in `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs` unconditionally rejects most shell metacharacters (`>`, `<`, `|`, `;`, `&&`, `||`, etc.). While this was intended for security—preventing accidental shell-injection-like behavior when using `std::process::Command` directly—it severely limits the utility of `::shell` directives and `$(...)` frontmatter expansions.

Users frequently need to:
- Suppress output (e.g., `> /dev/null`).
- Chain commands based on success or failure (e.g., `npm install && npm test`).
- Redirect error streams (e.g., `2>&1`).

## 2. Updated Requirements

### 2.1 Redirection Support
The system must support a curated set of redirection patterns by manually configuring the spawned process in Rust, rather than relying on a system shell.
- **Allowed Patterns:**
    - `> /dev/null` (STDOUT to null)
    - `2> /dev/null` (STDERR to null)
    - `2>&1` (STDERR to STDOUT)
    - `>&2` (STDOUT to STDERR)
- **Constraint:** Redirection to arbitrary files remains disallowed to prevent side effects and simplify security audits.

### 2.2 Command Chaining

The system must support logical AND (`&&`) and OR (`||`) operators to allow multi-command pipelines.
- **Orchestration:** Chaining is managed by a Rust-side state machine.
- **Conditional Logic:** 
    - `A && B`: Execute `B` only if `A` returns exit code 0.
    - `A || B`: Execute `B` only if `A` returns a non-zero exit code.

### 2.3 Preflight Validation & Approval

Security is maintained by validating the *entire* pipeline before any command is executed.
- **Policy Engine:** Every command in the chain must be checked against the policy engine.
- **Block Approval:** If any command in the chain requires user intervention (e.g., is not on the whitelist), the user must be presented with the **entire chain** of commands for upfront approval.
- **Atomic Failure:** If the user rejects the chain or any command fails the strict policy check (where "ask" is not an option), the entire pipeline is aborted.

### 2.4 Backtick Handling
Backticks (`` ` ``) should be allowed as literal characters within arguments but are not currently supported for subshell expansion.

## 3. Technical Design

### 3.1 Tokenizer & Parser (`tokenize.rs`)
The tokenizer will be enhanced to recognize `&&`, `||`, and the specified redirection tokens. It will transform the raw string into a structured `ShellPipeline`.

```rust
pub struct ShellPipeline {
    pub chains: Vec<CommandChain>,
}

pub struct CommandChain {
    pub command: CommandAction,
    pub next: Option<(ChainOperator, Box<CommandChain>)>,
}

pub struct CommandAction {
    pub executable: String,
    pub args: Vec<String>,
    pub redirection: RedirectionConfig,
}

pub enum ChainOperator {
    And, // &&
    Or,  // ||
}

#[derive(Default)]
pub struct RedirectionConfig {
    pub stdout: StdoutTarget,
    pub stderr: StderrTarget,
}
```

### 3.2 Executor (`executor.rs`)
The executor will iterate through the `ShellPipeline`, managing the lifecycle of each process.

- **Process Configuration:** For each `CommandAction`, `std::process::Command` will be configured using `.stdout()` and `.stderr()` based on the `RedirectionConfig`.
    - `> /dev/null` -> `Stdio::null()`
    - `2>&1` -> Requires capturing the stream and merging, or using specific crate support for stream piping.
- **State Machine:** The executor maintains the "exit status" of the previous command to determine whether to proceed with the `next` link in the chain based on the `ChainOperator`.

### 3.3 Validation Logic
Before passing the `ShellPipeline` to the executor:
1. Flatten the pipeline into a list of all executables and their arguments.
2. Run each through the policy engine.
3. If any require approval, aggregate the full command string for the user prompt.

## 4. Acceptance Criteria

- [ ] A command like `ls > /dev/null` executes successfully with no output captured.
- [ ] `cmd1 && cmd2` executes `cmd2` only if `cmd1` succeeds.
- [ ] `cmd1 || cmd2` executes `cmd2` only if `cmd1` fails.
- [ ] Complex chains like `npm install && npm run build || echo "Build failed"` are handled correctly.
- [ ] If `npm run build` is not whitelisted, the user is prompted to approve the *entire* chain `npm install && npm run build || echo "Build failed"`.
- [ ] Unsupported characters (e.g., `;`, `<`, `|`) still trigger a "not supported" error during tokenization.
- [ ] Literal backticks in strings are preserved.
