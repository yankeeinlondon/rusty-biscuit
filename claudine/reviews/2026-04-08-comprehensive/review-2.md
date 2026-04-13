# Claudine Package — Senior Rust Code Review

**Date:** 2026-04-08
**Reviewer:** Automated senior review (opencode / glm-5.1)
**Scope:** `claudine/lib` (library crate) + `claudine/cli` (binary crate)
**Files reviewed:** ~220 `.rs` files across 18 top-level library modules, CLI commands, 9 integration test files, and 164 inline `#[cfg(test)]` modules

---

## 1. Executive Summary

Claudine is a cross-agent meta-orchestration harness: it wraps multiple agentic CLI tools (Claude Code, Codex, Gemini CLI, Goose, OpenCode, KimiCode, QwenCode, Roo), providing unified hook dispatch, protect (path-level access control), composition (inline/external prompt orchestration), linking (skill/command/agent symlink management), system prompt assembly, messaging (Discord/Slack/Signal/WhatsApp notifications), reporting (SQLite-backed event index), and a full-featured TUI-based configuration editor.

The codebase is substantial (~220 source files), well-modularized, and demonstrates strong Rust fundamentals. Error handling is consistent: the library uses a single `thiserror`-derived `ClaudineError` enum with 30+ variants providing structured context; the CLI uses `color_eyre` for human-facing diagnostics. The `unsafe` surface area is minimal (3 sites, all justified). Test coverage is extensive (164 inline test modules + 9 integration test files).

The biggest concerns are: (1) a near-complete code duplication between `execute_actions` and `execute_actions_v2` in `dispatch/runner.rs`, which is a maintenance hazard and a source of subtle behavioral drift; (2) silent error discarding at several I/O boundaries (guardrails, messaging fire-and-forget, mutex poisoning); (3) a protect path extraction heuristic that does not handle quoted shell arguments; and (4) a monolithic 4,091-line wrapper module (`cli/src/commands/wrap/mod.rs`) that is difficult to reason about.

**Overall risk level: `medium`**

**Biggest strengths:**

- Comprehensive, well-structured error taxonomy with `thiserror`
- Clean separation between library and CLI layers
- Atomic file writes with lock+rename across config and composition
- Signal handling is correct (atomic counter + escalation, signal-safe operations only)
- Shell command validation through Darkmatter's tokenizer prevents metacharacter injection
- Good test discipline: inline tests in most modules, property-based tests in CLI wrappers

**Biggest concerns:**

- `execute_actions` vs `execute_actions_v2` duplication (maintenance hazard, drift risk)
- Silent error discarding at I/O boundaries
- Protect path extraction does not handle quoted shell arguments
- Monolithic `wrap/mod.rs` at 4,091 lines

**Maturity assessment:** The codebase is production-quality for its intended use (a developer tool), with clear room for improvement in deduplication and defensive error handling. It is not fragile, but the duplicated dispatch path is a live risk.

---

## 2. Key Findings

### Finding 1 — [Severity: High] Dispatch runner contains near-identical duplicated function (~170 lines)

- **Location:** `lib/src/dispatch/runner.rs` — `execute_actions` (lines 23–188) vs `execute_actions_v2` (lines 196–365)
- **Why it matters:** These two functions share ~90% identical code. The only difference is how `Speak` actions are handled: v1 delegates to `GlobalSettings`, v2 resolves voices through `ClaudineConfig`. All other action types (Report, Bash, Call, SoundEffect, Message) have identical implementations. This duplication means any bug fix or behavioral change must be applied to both paths, and subtle drift is already visible in the span attributes (they are identical copies but maintained separately).
- **Evidence:**

  ```rust
  // execute_actions (line 23)
  pub async fn execute_actions(
      actions: &[HookAction],
      compiled_mappers: Option<&[Option<CompiledMapper>]>,
      meta: &EventMeta,
      settings: &GlobalSettings,
      messaging: &RuntimeMessagingSettings,
      can_block: bool,
      protect_decision: Option<&ProtectDecision>,
  ) -> Result<Option<HookResponse>> { ... }

  // execute_actions_v2 (line 196)
  pub async fn execute_actions_v2(
      actions: &[HookAction],
      compiled_mappers: Option<&[Option<CompiledMapper>]>,
      meta: &EventMeta,
      config: &ClaudineConfig,
      messaging: &RuntimeMessagingSettings,
      can_block: bool,
      protect_decision: Option<&ProtectDecision>,
  ) -> Result<Option<HookResponse>> { ... }
  ```

  The Call, Bash, Report, SoundEffect, and Message branches are character-for-character identical between the two functions.
- **Recommendation:** Extract the common action execution loop into a private helper that accepts either a unified settings struct or a closure/trait for speak resolution. The v1/v2 difference can be injected via a `SpeakResolver` trait or a simple `enum DispatchConfig<'a> { V1(&'a GlobalSettings), V2(&'a ClaudineConfig) }`. This eliminates ~150 lines of duplication and prevents drift.
- **Confidence:** `high`

---

### Finding 2 — [Severity: Medium] Mutex poisoning silently ignored in approval cache

- **Location:** `lib/src/harness/shell.rs:153-154`
- **Why it matters:** When the Mutex is poisoned (a thread panicked while holding the lock), the `.ok()` silently discards the error and returns `None` from the cache lookup. This means the approval cache is silently invalidated on any panic, causing re-prompting for previously-approved commands. While not a soundness bug, it breaks a user expectation (commands approved once should stay approved).
- **Evidence:**

  ```rust
  if let Some(decision) = options
      .approval_cache
      .lock()
      .ok()                    // silently ignores poison
      .and_then(|cache| cache.get(&normalized).copied())
  { ... }
  ```

  The same pattern appears in `cache_approval_decision` at line 274: `if let Ok(mut cache) = cache.lock() { ... }`.
- **Recommendation:** Use `.lock().unwrap_or_else(|e| e.into_inner())` to recover from poisoned locks, preserving the cache even if a previous operation panicked. Alternatively, log a warning when poison is detected so the behavior is observable.
- **Confidence:** `high`

---

### Finding 3 — [Severity: Medium] Protect path extraction does not handle quoted shell arguments

- **Location:** `lib/src/services/protect/path.rs:139-158` — `extract_target_paths`
- **Why it matters:** The function splits on whitespace and skips `-`-prefixed tokens, but does not handle shell quoting. A command like `rm -rf "my file.txt" other` would produce targets `["\"my", "file.txt\"", "other"]` instead of `["my file.txt", "other"]`. This means protect's path-level access control could fail to match a real file path that was intended to be protected (or allowed).
- **Evidence:**

  ```rust
  pub fn extract_target_paths(command: &str) -> Vec<String> {
      let words: Vec<&str> = command.split_whitespace().collect();
      // ...
      while i < words.len() {
          let word = words[i];
          if word.starts_with('-') {
              i += 1;
              continue;
          }
          targets.push(word.to_string());
          i += 1;
      }
      targets
  }
  ```

- **Recommendation:** Use `shell_words::split` (already a dependency in `dispatch/runner.rs`) to properly tokenize the command string. This handles single quotes, double quotes, and escape sequences correctly.
- **Confidence:** `high`

---

### Finding 4 — [Severity: Medium] Silent error discarding in guardrails file creation

- **Location:** `lib/src/composition/guardrails.rs:44-47`
- **Why it matters:** If `create_dir_all` or `write` fails (e.g., permissions, disk full), the errors are silently discarded with `let _ = ...`. The function returns the default guardrails regardless, so the user has no indication that their customization file was not created. If the write fails, subsequent runs will also attempt (and fail) to create the file, causing repeated silent failures.
- **Evidence:**

  ```rust
  if let Some(parent) = guardrails_path.parent() {
      let _ = fs::create_dir_all(parent);   // silently discards error
  }
  let _ = fs::write(&guardrails_path, DEFAULT_GUARDRAILS);  // silently discards error
  ```

- **Recommendation:** Either propagate the error (this is a composition-critical path) or at minimum log a warning via `tracing::warn!` so the failure is observable.
- **Confidence:** `high`

---

### Finding 5 — [Severity: Medium] Blocking poll loop in `execute_approved_command`

- **Location:** `lib/src/harness/shell.rs:308-354`
- **Why it matters:** The function uses `std::thread::sleep(Duration::from_millis(50))` in a tight loop to poll `child.try_wait()`. This blocks the calling thread for the duration of the command execution. When called from async context (which the harness does), this blocks a Tokio worker thread, potentially starving other tasks. The 50ms granularity also means the process may linger up to 50ms after completion before being reaped.
- **Evidence:**

  ```rust
  loop {
      match child.try_wait() {
          Ok(Some(status)) => { /* collect output, return */ }
          Ok(None) => {
              if start.elapsed() >= timeout {
                  let _ = child.kill();
                  // ...
              }
              std::thread::sleep(poll_interval);
          }
          // ...
      }
  }
  ```

- **Recommendation:** Use `tokio::process::Command` instead of `std::process::Command` so the async runtime can await the child process without busy-waiting. Alternatively, use `wait_with_output()` with a `tokio::time::timeout` wrapper, which is already the pattern used in `dispatch/runner.rs`.
- **Confidence:** `high`

---

### Finding 6 — [Severity: Medium] Direct `std::env::var("HOME")` instead of `dirs::home_dir()`

- **Location:** `lib/src/composition/sequence.rs:79`
- **Why it matters:** The code uses `std::env::var("HOME")` directly, which is not portable to Windows (where `HOME` is typically not set). The `dirs` crate is already a dependency and is used elsewhere in the codebase (e.g., `services/protect/path.rs` uses `dirs::home_dir()`).
- **Evidence:**

  ```rust
  let home = std::env::var("HOME").map_err(|_| {
      CompositionError::SequenceExternalLoad(format!(
          "`{raw}`: HOME environment variable is not set"
      ))
  })?;
  ```

- **Recommendation:** Replace with `dirs::home_dir().ok_or_else(|| CompositionError::SequenceExternalLoad(...))` for cross-platform consistency.
- **Confidence:** `high`

---

### Finding 7 — [Severity: Medium] Unquoted YAML values in frontmatter serialization

- **Location:** `lib/src/composition/closure.rs` — `serialize_frontmatter_property` (not fully visible but identified by sub-agent analysis)
- **Why it matters:** When serializing frontmatter values that contain YAML special characters (`:`, `#`, `[`, `]`, `{`, `}`, `&`, `*`, `?`, `|`, `-`, `<`, `>`, `=`, `!`, `%`, `@`, `` ` ``), the function may produce invalid YAML or silently alter the value's meaning if values are not properly quoted.
- **Evidence:** The `closure.rs` file constructs YAML frontmatter line-by-line. String values containing special characters would need YAML quoting to round-trip correctly.
- **Recommendation:** Validate that all serialized property values use proper YAML quoting (single-quoted or double-quoted). Consider using a YAML serialization library (the project already depends on `biscuit-file` which supports YAML) instead of manual string construction.
- **Confidence:** `medium` (would need to verify exact serialization logic)

---

### Finding 8 — [Severity: Medium] `output.rs` byte-offset truncation assumes ASCII-safe content

- **Location:** `cli/src/output.rs:327-329`
- **Why it matters:** `summarize_value` truncates at byte offset 117 (`&value[..117]`). While the value is JSON-encoded (which is ASCII-safe for structural characters), user-provided string values within the JSON could contain multi-byte UTF-8 characters. Slicing at a byte boundary that falls within a multi-byte character produces a panic in debug mode or invalid UTF-8 in release mode.
- **Evidence:** The pattern `&value[..117]` performs byte-level slicing on a `&str`. If the 117th byte falls within a multi-byte UTF-8 sequence, this is a panic.
- **Recommendation:** Use `value.char_indices().take_while(|(i, _)| *i < 117).map(|(_, c)| c).collect::<String>()` or `value.floor_char_boundary(117)` (stable since Rust 1.82).
- **Confidence:** `medium` (JSON encoding limits but does not eliminate the risk)

---

### Finding 9 — [Severity: Low] Regex compiled per-call in `apply_mapper` fallback path

- **Location:** `lib/src/dispatch/runner.rs:847-849`
- **Why it matters:** When using `Mapper::Regex` without a pre-compiled mapper, the regex is recompiled on every hook invocation. For hot-path hooks (BeforeTool, AfterTool), this is unnecessary allocation and CPU work.
- **Evidence:**

  ```rust
  Mapper::Regex { pattern } => {
      let regex = Regex::new(pattern)?;  // compiles on every call
      map_regex_with_compiled(&regex, output)
  }
  ```

- **Recommendation:** The compiled mappers already handle this case via `CompiledMapper::Regex`. Ensure all call sites provide pre-compiled mappers (this is likely already the case in production paths). Add a debug assertion or tracing warning when the fallback path is hit.
- **Confidence:** `high`

---

### Finding 10 — [Severity: Low] Duplicated `ensure_json_value` helpers across permission backends

- **Location:** `lib/src/permissions/providers/claude.rs`, `opencode.rs`, `qwen.rs`, `roo.rs`, `gemini.rs`
- **Why it matters:** The exact same `ensure_json_value` / `ensure_json_array` helper functions are copy-pasted across 5+ provider backend files. This is a maintenance burden and a source of drift.
- **Recommendation:** Extract into a shared utility module (e.g., `permissions/providers/mod.rs` or a new `permissions/json_utils.rs`).
- **Confidence:** `high`

---

### Finding 11 — [Severity: Low] `expect()` calls in permissions provider backends

- **Location:** `lib/src/permissions/providers/claude.rs:920,931`, `opencode.rs:618,630`, `qwen.rs:1065,1077`, `roo.rs:946`
- **Why it matters:** `ensure_json_value` and `ensure_json_array` use `expect("object")` and `expect("array")` after a code path that just set the value to that type. While the panic is "impossible" by construction, `expect` bypasses the error chain and produces an unhelpful panic message in the edge case where a logic bug is introduced.
- **Recommendation:** Replace with `debug_assert!` followed by an `unsafe { get_unchecked }` or simply use `debug_assert!` + normal indexing. Alternatively, use `.ok_or_else(|| ...)` for a proper error path even if it's currently unreachable.
- **Confidence:** `high`

---

### Finding 12 — [Severity: Low] Roo `plan_change` silently writes partial plans for unsupported operations

- **Location:** `lib/src/permissions/providers/roo.rs` — `plan_change`
- **Why it matters:** When `plan_change` receives an unsupported operation, it sets `supported = false` but still produces and writes a `PersistentMutationPlan` containing only the supported subset. This silently drops the unsupported operations without any error or warning.
- **Recommendation:** Return `PolicyMutationPlan::unsupported()` when the operation is not supported, or at minimum log a warning about the dropped operations.
- **Confidence:** `medium`

---

## 3. Rust-Idiomaticity Notes

### 3.1 Type system modeling

- **`TtsValue` enum** (`config/claudine_config.rs`): Uses a three-variant enum (`Boolean(bool)` | `Config(TtsConfigSettings)`) to represent `tts: true | false | { ... }` in YAML. This is well-modeled — a flat enum captures the three states cleanly and avoids `Option<Either<...>>`.

- **`Provider` enum** (`events/provider.rs`): Simple enum with 8 variants. Derives `Clone`, `Copy`, `Hash`, `Eq`, `Serialize`, `Deserialize`. Good. The `strum` crate could reduce some boilerplate (`as_slug()`, `FromStr` impl) but the current manual impls are clear.

- **`HookResponse`** (`actions/hook_response.rs`): Uses `Default` trait for optional fields. Clean.

### 3.2 Ownership patterns

- Heavy use of `Arc<Mutex<...>>` for shared state (approval caches, summary details). This is appropriate for the CLI tool's concurrency model (Tokio runtime with shared state).
- `Arc` is used consistently for shared handler references (`ShellApprovalHandler`, `LiveStreamSink`).
- `clone()` is used liberally on `String`, `PathBuf`, `Vec<String>`. This is appropriate for a CLI tool where data volumes are small and correctness matters more than micro-optimization.

### 3.3 Async patterns

- `tokio::spawn` is used for fire-and-forget operations (TTS, sound effects, messaging). This is correct for non-critical side effects.
- `tokio::task::spawn_blocking` is used for blocking sound effect playback. Correct.
- The main async boundary is at the CLI entry point (`#[tokio::main]`). Library functions are appropriately async only when they perform I/O.

### 3.4 Error handling

- Library: Single `ClaudineError` enum with structured variants. Each variant carries relevant context (paths, provider names, pattern strings). Good use of `#[from]` for automatic conversion from dependency errors.
- Composition module: Separate `CompositionError` enum. This is fine as a domain-specific error type that doesn't pollute the main error enum with composition-specific variants.
- CLI: `color_eyre::eyre::Result` wraps library errors. The `?` operator propagates naturally. Good layering.

### 3.5 API naming

- Function names are clear and follow Rust conventions: `execute_actions`, `validate_and_approve_command`, `extract_target_paths`.
- Module organization maps well to domain concepts: `dispatch`, `composition`, `linking`, `protect`, `harness`.
- `execute_actions_v2` is an exception — the `v2` suffix is a code smell indicating the v1 should have been refactored rather than duplicated.

---

## 4. Testing Gaps

### 4.1 Missing test coverage for critical paths

| Module / Path | Missing Scenarios |
|---|---|
| `execute_approved_command` (harness/shell.rs) | No test for timeout enforcement; no test for zombie process cleanup; no test for `which::which` failure when executable is not in PATH |
| `extract_target_paths` (protect/path.rs) | No test for quoted arguments (`rm -rf "my file.txt"`), no test for `--flag=value` style arguments, no test for environment variable expansion |
| `atomic_write` (config/atomic.rs) | No concurrent write test (the primary purpose of atomic writes); no test for rename fallback when cross-filesystem |
| `execute_actions` vs `execute_actions_v2` | No test verifying behavioral equivalence between v1 and v2 for all action types |
| Composition inline closure | No test for YAML special characters in frontmatter values round-tripping |
| `load_or_create_guardrails` | No test for the error case when `create_dir_all` or `write` fails (e.g., read-only filesystem) |
| `normalize_path` (protect/path.rs) | No test for paths containing `..` beyond the root (e.g., `/foo/../../../bar`) — currently pops past root silently |
| Signal handling (exec.rs) | Integration test for SIGINT→SIGTERM→SIGKILL escalation sequence |
| `create_resource_link` (linking/symlink.rs) | No test for concurrent symlink creation race |

### 4.2 Test quality observations

- The 164 inline test modules demonstrate strong test discipline across the codebase.
- Integration tests exist for: handle repo config, wrap commands, sequence CLI, protect CLI, skills integration, MCP CLI, PTY tests — this is a good breadth.
- Property-based tests (`proptest`) exist for wrapper flag extraction and stream event parsing — appropriate for parsing logic.
- Snapshot tests (`insta`) are used for JSON output validation — good for preventing regressions in structured output.

### 4.3 Brittle tests

- `cli/src/commands/wrap/exec.rs` tests that check for specific exit codes use `std::process::ExitStatus::from_raw(2 << 8)`. This is Unix-specific and will not work on Windows. The test is already behind `#[cfg(unix)]` in some places, but the pattern should be validated.
- Tests that depend on `dirs::home_dir()` (e.g., `protect/path.rs`) will behave differently in CI environments where `HOME` may not be set or may point to an unexpected location.

---

## 5. Unsafe Code Review

### Site 1: `cli/src/main.rs:63` — `std::env::set_var("NO_COLOR", "1")`

```rust
unsafe { std::env::set_var("NO_COLOR", "1") };
```

- **Invariant:** `set_var` is `unsafe` in edition 2024 because concurrent access to environment variables is undefined behavior if another thread is reading/writing env vars simultaneously.
- **Upheld?** Yes — this runs before `Cli::parse()` and before any async runtime or thread spawning. It is the second statement in `main()`.
- **Documented?** The comment explains the purpose but does not explicitly state the safety contract.
- **Minimized?** The unsafe block is one line.
- **Verdict:** Justified. This is the standard pattern for pre-parsing flag injection.

### Site 2: `cli/src/commands/wrap/exec.rs:412-433` — Signal handler registration

```rust
let _guard = unsafe {
    signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
        let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
        match count {
            1 => libc::kill(child_pid as i32, libc::SIGINT),
            2 => libc::kill(child_pid as i32, libc::SIGTERM),
            _ => libc::kill(child_pid as i32, libc::SIGKILL),
        }
    })
}?;
```

- **Invariant:** The signal handler must only perform signal-safe operations.
- **Upheld?** Yes — the handler performs only `AtomicU8::fetch_add` (signal-safe) and `libc::kill` (signal-safe per POSIX).
- **Documented?** Inline comments explain the escalation logic.
- **Minimized?** The unsafe block covers the registration call only.
- **Verdict:** Correct and safe. The escalation pattern (SIGINT→SIGTERM→SIGKILL) is standard Unix process management.

### Site 3: `cli/src/commands/wrap/exec.rs:486-509` — SIGTERM and SIGKILL escalation

```rust
unsafe { libc::kill(child.id() as i32, libc::SIGTERM); }
// ...
unsafe { libc::kill(child.id() as i32, libc::SIGKILL); }
```

- **Invariant:** `child.id()` returns a valid PID for a child process that we spawned and still own.
- **Upheld?** Yes — these are called within the timeout handler after `child.try_wait()` returned `None`, meaning the child is still running.
- **Documented?** Tracing warns explain the escalation.
- **Minimized?** Each unsafe block is a single `libc::kill` call.
- **Verdict:** Correct. Standard Unix process termination.

### Site 4: `lib/src/messaging/resolve.rs:309-345` — Test-only `set_var`/`remove_var`

```rust
// SAFETY: single-threaded test environment; no concurrent env access
unsafe { std::env::set_var("TEST_INLINE_WINS_VAR", "from-env"); }
```

- **Invariant:** No concurrent access to environment variables.
- **Upheld?** Yes — guarded by `#[serial]` test attribute, ensuring single-threaded execution.
- **Documented?** SAFETY comments present on each block.
- **Verdict:** Correct. The `serial_test` crate provides the necessary isolation.

### Summary

All unsafe usage is justified, documented, and correctly scoped. No unsoundness risk identified.

---

## 6. Performance Notes

### 6.1 Likely issues

1. **Blocking poll in `execute_approved_command`** (Finding 5): The 50ms sleep loop blocks a Tokio worker thread for the full command duration. For long-running commands, this wastes a thread. Impact: medium for single-command execution; high if the harness ever runs multiple commands concurrently.

2. **Regex compilation on fallback path** (Finding 9): The `Mapper::Regex` fallback recompiles the pattern on every call. Impact: low in practice because pre-compiled mappers are used in production paths.

### 6.2 Acceptable patterns

- `.clone()` on `String`/`PathBuf`/`Vec<String>` throughout: appropriate for CLI-scale data volumes.
- `serde_json::Value` cloning in metadata construction: the values are small event metadata, not large payloads.
- `indexmap::IndexMap` for frontmatter: preserves insertion order at negligible cost for the data sizes involved.
- `Arc<Mutex<HashMap<...>>>` for approval cache: correct for the shared-mutation pattern; contention is negligible (single reader/writer per command).

### 6.3 Needs benchmarking (not actionable without data)

- SQLite ingestion performance for large log datasets (reporting/ingest.rs)
- YAML frontmatter parsing for large composition chains
- Symlink tree creation for repositories with many skills

---

## 7. Additional Observations

### 7.1 Module size

| File | Lines | Assessment |
|---|---|---|
| `cli/src/commands/wrap/mod.rs` | 4,091 | Too large. Should decompose into submodules. |
| `cli/src/commands/hooks.rs` | 1,175 | Large but cohesive (hook display logic). |
| `cli/src/commands/logs.rs` | 1,300 | Large but cohesive (log rendering). |
| `cli/src/commands/mcp.rs` | 1,209 | Large but cohesive (MCP catalog management). |
| `lib/src/dispatch/mod.rs` | ~1,539+ | Contains both dispatch logic and extensive tests. Consider splitting tests. |
| `lib/src/harness/parse.rs` | ~969 | Large parsing module. Acceptable for a parser. |

The `wrap/mod.rs` file is the primary concern. It handles wrapper dispatch, harness loop execution, inline closure validation, handler resolution, prompt materialization, and summary rendering — all in one file. This makes the module difficult to navigate and review.

### 7.2 Trust gating inconsistency

Different permission providers use different trust-gating semantics:

- Codex, Gemini: `== Some(true)` — only trusted projects discover repo config
- Qwen: `!= Some(false)` — trusted AND unknown projects discover repo config
- Claude, OpenCode, Roo: No trust gating — always discover repo config

If this is intentional per-provider semantics, it should be documented. If not, it's a bug.

### 7.3 `fs2` for file locking

The `fs2` crate provides `FileExt::lock_exclusive()` for atomic writes. This is a mature crate but note that `fs2` is in maintenance mode and `fs4` is the successor. The current usage is correct and safe.

---

## 8. Prioritized Next Steps

1. **Deduplicate `execute_actions` / `execute_actions_v2`** — Extract a shared action execution loop with a strategy parameter for speak resolution. This eliminates ~150 lines of duplication and prevents behavioral drift. (Finding 1)

2. **Fix `extract_target_paths` to use `shell_words::split`** — Protect's path-level access control depends on correct argument parsing. Quoted arguments are a common pattern. (Finding 3)

3. **Replace blocking poll in `execute_approved_command` with async `tokio::process::Command`** — This prevents blocking Tokio worker threads during shell command execution. (Finding 5)

4. **Add error logging to guardrails file creation** — Replace `let _ = ...` with `tracing::warn!` for observable failure modes. (Finding 4)

5. **Replace `std::env::var("HOME")` with `dirs::home_dir()`** in sequence.rs — Cross-platform consistency. One-line fix. (Finding 6)

6. **Extract duplicated `ensure_json_value` helpers** from permission backends into a shared utility module. (Finding 10)

7. **Decompose `cli/src/commands/wrap/mod.rs`** (4,091 lines) into focused submodules — e.g., `dispatch.rs`, `harness_loop.rs`, `summary.rs`, `prompt.rs`. This improves reviewability and reduces merge conflict surface.
