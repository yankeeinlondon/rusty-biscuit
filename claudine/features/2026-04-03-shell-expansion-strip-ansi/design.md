# Design Document: Shell Expansion ANSI Stripping

## Overview
This document outlines the design for adding ANSI escape code stripping to the `::shell` directive expansion feature within the `darkmatter` library. The goal is to prevent terminal escape sequences (such as color codes and cursor movements) from leaking into rendered markdown or HTML outputs when shell commands are executed during markdown composition.

## Context
The `darkmatter` library supports dynamic document composition using `::shell {cmd}` directives. Many CLI tools assume they are running in a TTY and output ANSI escape sequences for syntax highlighting, progress bars, or error formatting. When this output is captured and injected into markdown, the raw escape codes (e.g., `\x1b[31m`) corrupt the document structure and are rendered as garbage text in HTML or PDF exports.

## Proposed Solution
We will implement a multi-layered approach to handle ANSI escape codes safely and efficiently, utilizing existing monorepo capabilities.

### 1. Proactive Prevention (`NO_COLOR=1`)
Before executing the command, we will inject the standard `NO_COLOR=1` environment variable. This politely requests compliant CLI tools (e.g., `git`, `ripgrep`, `sniff`) to disable colored output, reducing the amount of raw data processed.

### 2. Reactive Post-processing (`strip_escape_codes`)
Because many tools ignore `NO_COLOR=1`, we will apply a regex-based cleanup pass on the captured stdout/stderr. We will reuse `biscuit_terminal::prelude::strip_escape_codes`, which is the established terminal authority within the `rusty-biscuit` monorepo.

### 3. Configuration via `ShellExpansionOptions`
The behavior will be configurable via a new boolean flag in the `ShellExpansionOptions` struct, allowing consumers of the `darkmatter` library to opt-in or opt-out depending on their rendering target (e.g., a terminal renderer might *want* the colors, whereas an HTML renderer does not).

## Implementation Details

### 1. Update `ShellExpansionOptions`
File: `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`

```rust
pub struct ShellExpansionOptions {
    pub timeout: std::time::Duration,
    pub policy_root: Option<PathBuf>,
    pub working_directory: Option<PathBuf>,
    pub approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
    // NEW FIELD
    pub strip_ansi: bool, 
}

impl Default for ShellExpansionOptions {
    fn default() -> Self {
        Self {
            // ... existing defaults
            strip_ansi: true, // Defaulting to true as it's the safest for markdown injection
        }
    }
}
```

### 2. Update `executor.rs`
File: `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs`

- Add the `NO_COLOR` environment variable to the `Command` builder if `strip_ansi` is true.
- After combining the stdout and stderr, pass the resulting string through `biscuit_terminal::prelude::strip_escape_codes` if `strip_ansi` is true.

```rust
// ... setup cmd ...
if shell_opts.strip_ansi {
    cmd.env("NO_COLOR", "1");
}

// ... spawn and wait ...
if status.success() {
    let mut output = stdout_str;
    if !stderr_str.is_empty() {
        if !output.is_empty() { output.push('\n'); }
        output.push_str(&stderr_str);
    }
    
    if shell_opts.strip_ansi {
        output = biscuit_terminal::prelude::strip_escape_codes(output);
    }
    
    return Ok(output);
}
```

### 3. (Optional) Update `parser.rs` for Inline Control
File: `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs`

To provide authors fine-grained control, we could parse a `--strip-ansi` or `--keep-ansi` flag from the `::shell` directive itself, overriding the global `ShellExpansionOptions` on a per-command basis. This would require adding fields to the `ErrorHandling` struct (or creating a new `ExecutionOptions` struct) and updating the tokenization logic.

## Alternatives Considered
*   **Adding an external crate (e.g., `strip-ansi-escapes`)**: Rejected to minimize dependency bloat. `biscuit-terminal` already exists in the monorepo and exposes robust regex-based stripping functions.
*   **Only using `NO_COLOR=1`**: Rejected because non-compliant tools would still corrupt the markdown.
*   **Always stripping without an option to disable**: Rejected because some rendering pipelines (like a terminal-native markdown viewer) might support rendering raw ANSI codes directly to the screen.
