---
date: 2026-04-21
fixed: 2026-04-21
agent: "${env.AGENT}"
---

## Problem Statement

Running specialized `sniff` commands (e.g., `sniff repo package-area --perf`) produced no performance metrics. The `--perf` flag was inconsistently implemented across the CLI, hindering the ability to diagnose slow-running commands.

## Requirements

### 1. Global Performance Collection

- **Scope of `--perf`:** A global `CliPerf` is initialized at the start of CLI execution.
- **Universal Coverage:** The standard performance report is appended to the output of **every** command when `--perf` is active. This includes specialized commands that bypass the main detection pipeline, such as:

    - `programs`
    - `just`
    - `docs`
    - `blast-radius`
    - `services`
    - `repo packages`
    - `repo package-area`
    - `repo root`
    - `repo recent-commits`
    - `repo dirty-source-code` / `staged-source-code` / `unstaged-source-code`
    - `repo staged-files` / `unstaged-files` / `untracked-files` / `dirty-files`
    - `repo hash`
    - `repo remote`
    - `repo has-merge-conflict`
    - `repo package` / `repo package-area`
    - `topics`

##### 2. Output Destination and Scriptability

- **Standard Output:** By default, performance metrics are appended to the command's output.
- **Scripting Mode:** For scriptable commands (e.g., `sniff repo package-area`) that emit machine-readable or pipable text, the performance breakdown is emitted to `stderr` when in text mode. This ensures `stdout` remains clean for shell pipelines while still providing profiling data to the developer.

##### 3. Performance Baseline and SLA

- **Threshold:** Any CLI command taking longer than **1 second** is considered a performance bug and must be optimized.
- **Environment Baseline:** This threshold is defined against the `rusty-biscuit` monorepo (48 members) with a **warm disk cache** on a standard developer machine (e.g., Apple Silicon or equivalent).

## Implementation

### Changes

1. **`cli/src/commands.rs`**: Added `CliPerf` struct that tracks wall-clock time from CLI start. Threaded through every command path. Rich terminal commands emit perf to stdout; scriptable text commands emit to stderr.

2. **`cli/src/output/mod.rs`**: Made `render_performance_section` public. Removed perf from `render_text()` so emission is controlled centrally in `commands.rs`.

3. **`cli/src/output/recent_commits.rs`**: Added `&CliPerf` parameter to `handle_recent_commits_command`. Emits perf to stderr (scriptable command).

4. **`cli/README.md`**: Added "Performance Reporting" section documenting `--perf` behavior, stdout vs stderr destination, and JSON embedding.

### Scriptable vs Rich Classification

**Rich terminal output (perf → stdout):**
- `programs`, `editors`, `utilities`, `agents`, etc.
- `just`
- `docs`
- `services`
- `repo remote`, `repo hash`
- `repo structure`, `repo git-status`, `repo deps`
- `topics`

**Scriptable text output (perf → stderr in text mode):**
- `repo packages`, `repo package`, `repo package-area`
- `repo root`, `repo package-root`, `repo package-area-root`
- `repo dirty-packages`, `repo dirty-package-areas`, `repo staged-*`, `repo unstaged-*`
- `repo staged-files`, `repo unstaged-files`, `repo untracked-files`
- `repo dirty-source-code`, `repo staged-source-code`, `repo unstaged-source-code`, `repo dirty-files`
- `repo recent-commits`, `repo source-code-changes`, `repo documentation-changes`
- `repo is-current-package-area-dirty`, `repo package-area-has-source-code-changes`
- `repo has-merge-conflict`
- `blast-radius`
