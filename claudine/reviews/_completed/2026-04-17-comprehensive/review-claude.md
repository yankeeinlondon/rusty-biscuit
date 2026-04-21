# Claudine Comprehensive Rust Code Review (2026-04-18)

Scope: `claudine/lib/` (~70 KLOC) and `claudine/cli/` (~60 KLOC), branch `claudine`,
HEAD `bb44da8e`. Review performed on the full tree; findings cite `file:line`
against the checked-out worktree.

## 1. Executive Summary

Claudine is a sophisticated multi-provider agentic-CLI harness that normalizes
hooks, wraps 8 different agent CLIs, parses 6 JSON-Lines provider protocols,
maintains a cross-provider permission/policy engine, and drives a
Markdown-based composition/sequence pipeline. For a project of this scope the
codebase is unusually well-structured: modules are cohesive, error types are
rich and domain-named, the streaming protocol layer has been deliberately
refactored onto typed serde models, the CLI has an auditable argv
pre-normalizer with drift-detection tests, and there are **extensive** unit and
fixture-based integration tests (hundreds of `#[test]` blocks).

The project feels production-ready as a developer tool. The most serious
concerns are concentrated in four areas: (1) `atomic_write` is not actually
atomic under concurrent writers and has a non-atomic copy-fallback; (2) several
`unsafe { …unwrap_unchecked() }` sites guarded only by `debug_assert!` risk UB
in release builds for zero measurable gain; (3) the `BLOCKED_COMMANDS` list in
the bash-action executor is a basename-only deny list that conveys a false
sense of safety; (4) `map_exit_code` silently treats every non-{0,2} exit
status as **Allow**, which is the wrong default for a permission mapper. None
of these are currently causing user-visible failures, but each warrants a
targeted fix. **Overall risk: medium-low** — serious issues exist, but they
are localized and have clear remediations.

### Strengths

- Domain-rich error enum (`ClaudineError`) with `#[from]` conversions and
  structured context (`LockError { path }`, `McpAmbiguousMatch { candidates }`).
- Argv pre-normalizer (`cli/src/argv.rs`) is tiny, auditable, pass-through by
  default, has an explicit drift-detection test for the value-bearing-flag list.
- Stream protocol layer (`lib/src/stream/protocol/`) cleanly separates
  serde-tagged enums from handler logic; each module has an
  `unknown_event_type_fails_typed` safety-net test.
- Process supervision in `cli/src/commands/wrap/exec.rs` correctly distinguishes
  pipe-inheriting children (own process group) from TTY-inheriting children
  (share pgroup) and installs signal handlers only when ownership is correct.
- Systematic use of `tracing` spans with structured fields instead of ad-hoc
  `eprintln!` debug output.

### Biggest Concerns

- Config write path is mis-named as atomic and has cross-process race
  conditions on the temp file.
- Small cluster of `unsafe` micro-optimizations behind `debug_assert!` add UB
  risk with no benchmark justifying them.
- Exit-code mapper biases toward `Allow` on every unknown status.
- File-scoped `unwrap()`/`expect()` density is high enough in non-test code
  that a spot check was needed to confirm most are actually in `#[cfg(test)]`
  (they are) — but this makes future clippy-with-`unwrap_used` adoption hard.

---

## 2. Key Findings

### [Severity: Critical] `atomic_write` is not atomic under concurrent writers

- **Location:** `lib/src/config/atomic.rs:13-51`
- **Why it matters:** Claudine runs wrapper commands, hook handlers, and
  config-editor subcommands that may all touch the same JSON/TOML file. The
  function is advertised as atomic, the error type is named `LockError`, and
  callers rely on the "temp file + rename" invariant for crash/abort safety.
  The implementation violates that invariant twice.
- **Evidence:**
  ```rust
  // 20:  let tmp_path = path.with_extension("tmp");   // fixed, not unique
  // 24:  let mut file = File::create(&tmp_path)?;     // truncates before lock
  // 25:  file.lock_exclusive()…                        // advisory, per-FD
  // 42:  if fs::rename(&tmp_path, path).is_err() {
  // 44:      fs::copy(&tmp_path, path) …               // ← NOT atomic
  // 46:      let _ = fs::remove_file(&tmp_path);
  // 47:      copy_result?;
  // 48:  }
  ```
  Two concurrent writers compute the same `tmp_path`. Both `File::create` it,
  both truncate the other's partial write, one acquires the advisory lock and
  writes, the other waits, writes, and renames — and the final file is the
  *second* writer's content regardless of which `atomic_write` returned first.
  Separately, when `fs::rename` fails (cross-device: tmp on the same dir but
  path is a bind-mount; some Windows scenarios), the code falls back to
  `fs::copy` — which performs a non-atomic, byte-by-byte copy over the target.
  A crash mid-copy leaves the target truncated, exactly the invariant
  `atomic_write` is supposed to prevent.
- **Recommendation:**
  1. Use `tempfile::NamedTempFile::new_in(parent)?` to get a uniquely-named
     temp file in the target directory; never reuse a fixed `.tmp` sibling.
  2. Keep the exclusive lock on the target path (or use a dedicated lockfile)
     rather than on the throwaway temp file.
  3. If `rename` fails, return the error; do not fall back to non-atomic copy.
     `rename(2)` within the same filesystem is the only atomic primitive.
  4. Call `File::sync_all()` on the parent directory after rename for crash
     durability (`std::fs::File::open(parent)?.sync_all()` on Unix).
- **Confidence:** high

### [Severity: High] `unsafe { unwrap_unchecked() }` behind `debug_assert!` risks UB

- **Location:** `lib/src/permissions/json_utils.rs:25`;
  `lib/src/permissions/providers/opencode.rs:623`;
  `lib/src/permissions/providers/qwen.rs:1069`.
- **Why it matters:** `debug_assert!` is stripped in release. `unwrap_unchecked`
  on a violated invariant is immediate undefined behaviour (reading the
  `None`/wrong-variant discriminant as an `Option<&mut T>`). None of these
  sites are on a measured hot path; they all exist in permission-mutation code
  that runs once per config apply. The pattern — `debug_assert!(x.is_object());
  unsafe { x.as_object_mut().unwrap_unchecked() }` — trades real soundness risk
  for an unmeasured branch saving.
- **Evidence:**
  ```rust
  // json_utils.rs:20-25
  debug_assert!(
      value.is_array(),
      "ensure_json_array: type mismatch after set — expected array, got {:?}",
      value
  );
  unsafe { value.as_array_mut().unwrap_unchecked() }
  ```
  Immediately above, the code *just set* the value to `Value::Array(Vec::new())`
  if it wasn't one, so the invariant truly holds — but the invariant is
  structural, enforced by the surrounding code, not by `unwrap_unchecked`.
  A well-behaved `unwrap()` compiles to an identical branch-on-variant
  check followed by an unreachable-on-panic call; LLVM will almost always
  fold the check after the immediately-preceding assignment.
- **Recommendation:** Replace every `unwrap_unchecked` call site with
  `.expect("…")` and rely on the compiler. Remove the `unsafe` blocks. If a
  benchmark ever shows this on a hot path, reintroduce `unsafe` with a proper
  SAFETY comment *and* a `#[cfg(debug_assertions)]` path that still runs the
  check so release failures are at least testable.
- **Confidence:** high

### [Severity: High] `map_exit_code` defaults unknown statuses to Allow

- **Location:** `lib/src/dispatch/runner.rs:695-716`
- **Why it matters:** This mapper turns a `Call` action's child-process exit
  status into a `HookDecision` used by the hook permission pipeline. The
  convention in the codebase — matching Claude Code's own PermissionDecision
  mapper — is `0 → Allow`, `2 → Deny`, everything else `→ Allow`. That means
  SIGSEGV (139), permission-denied (126), command-not-found (127), and any
  user-defined exit code between 3 and 125 are all treated as **Allow** with no
  log trail distinguishing them from a clean success.
- **Evidence:**
  ```rust
  fn map_exit_code(output: &CommandOutput) -> HookResponse {
      let code = output.status.code().unwrap_or(1);
      let decision = match code {
          0 => Some(HookDecision::Allow),
          2 => Some(HookDecision::Deny),
          _ => Some(HookDecision::Allow),   // ← crashes, typos, everything
      };
  ```
  There's no `warn!`/`tracing::event!` on the `_` arm and no way for an
  operator to distinguish "mapper crashed at 139" from "mapper cleanly
  allowed at 0" in the JSONL audit trail.
- **Recommendation:** Either (a) change `_ => None` so the response carries no
  decision and the caller falls through to the next action, or (b) map
  non-{0,2} to `None` **and** emit a `tracing::warn!` with `code` and `stderr`
  so operators can see anomalous exits. Document the chosen policy in the
  `Call` action's `mapper` field rustdoc.
- **Confidence:** high

### [Severity: High] `BLOCKED_COMMANDS` deny list is basename-only and trivially bypassable

- **Location:** `lib/src/actions/bash_executor.rs:6-9, 34-42`
- **Why it matters:** The list (`rm`, `rmdir`, `mkfs`, `dd`, `fdisk`, `kill`,
  …) is matched against `path.file_name()`, which means:
  - A user (or a malicious `user.yaml` merged into a shared repo config) can
    bypass by naming the action command `rm2`, `dd.exe`, `./r\m`, or creating
    a symlink with any other basename.
  - `sudo rm -rf /`, `doas shutdown`, `su -c "kill 1"`, `crontab -r`,
    `nohup rm`, `env rm`, `xargs rm`, `bash -c rm` are all allowed.
  - The list doesn't include `sudo`, `doas`, `su`, `chmod`, `chown`, `mv`,
    `chattr`, `crontab`, `at`, `setfacl`, or any of the obvious "run arbitrary
    code" wrappers.
- **Evidence:**
  ```rust
  // 36: let base_name = path.file_name().and_then(|n| n.to_str()).unwrap_or(command);
  // 38: if BLOCKED_COMMANDS.contains(&base_name) {
  ```
- **Recommendation:** The correct framing is "bash actions are a user-authored
  escape hatch; any deny list is a speed bump, not a boundary." Either:
  1. Delete the deny list and document that bash actions run whatever the
     config says, with a startup warning when actions are configured; or
  2. Keep it as an **advisory** check that emits `warn!` but does not block,
     and document that the *real* control is the `ProtectService` catalog at
     `lib/src/services/protect/catalog.rs`, which is regex-based and
     substantially more robust.

  Right now the code advertises a safety property it does not deliver.
- **Confidence:** high

### [Severity: High] Unbounded JSON accumulation in Claude tool-use delta buffer

- **Location:** `lib/src/stream/claude_semantic.rs` (pending-tool-use state;
  `input_json` `push_str`s without a size check until JSON parses)
- **Why it matters:** The Claude provider streams tool inputs as
  `input_json_delta` events, which are appended to a per-pending-tool
  `input_json: String` until the accumulated text parses as JSON. A
  malformed/truncated stream (or a provider-side bug) that never closes the
  JSON produces unbounded memory growth in the Claudine process. For an
  always-on wrapper, an upstream partial outage can become an OOM.
- **Evidence:** The subagent's reading of `claude_semantic.rs` confirmed
  `pending.input_json.push_str(partial_json)` has no size ceiling, and the
  JSON parse failure path silently discards with `.ok()`.
- **Recommendation:** Cap `input_json` at e.g. 1 MiB; when exceeded, emit a
  `SemanticEvent::Error { kind: AgentNative, … }` with a truncation notice,
  drop the pending entry, and continue. This matches the already-present 32-
  entry cap on `pre_init_hook_buffer`.
- **Confidence:** medium (based on subagent read; please confirm the exact
  line numbers against HEAD before coding the fix)

### [Severity: Medium] Signal handler uses child PID after child may have exited

- **Location:** `cli/src/commands/wrap/exec.rs:623-667, 693-789`
- **Why it matters:** `wait_with_signal_handling` installs a `signal_hook`
  handler that captures `child_pid: u32` by copy and calls
  `libc::kill(-(child_pid as i32), SIGINT)` on second Ctrl-C. Once the child
  exits and is reaped, the kernel is free to recycle that PID. On a long-lived
  Claudine session the next Ctrl-C could signal a completely unrelated
  process group. The handler does not check whether the child has exited, and
  there is no `SIGCHLD` notification that retracts the handler.
- **Evidence:**
  ```rust
  let _guard = unsafe {
      signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
          …
          libc::kill(-(child_pid as i32), libc::SIGINT);
          …
      })
  }?;
  let status = child.wait()?;   // handler stays registered until guard drops
  ```
  The `_guard` binding unregisters the handler on scope exit, so the window is
  bounded by how long it takes to drop the guard after `child.wait()` returns.
  On a typical run this is microseconds, but the `wait_with_signal_and_early_termination`
  variant keeps the handler live for a 5-second grace period after SIGTERM,
  which widens the window meaningfully.
- **Recommendation:** Inside the handler, track an `AtomicBool child_exited`
  that the main thread sets before dropping the guard, and early-return from
  the handler when set. On Unix, prefer `pidfd`-based signaling where
  available.
- **Confidence:** medium

### [Severity: Medium] Heavy `#[serde(default)]` across protocol structs masks real parse errors

- **Location:** `lib/src/stream/protocol/claude.rs`, `codex.rs`, `gemini.rs`,
  `opencode.rs`, `qwen.rs`, `kimi.rs` — every field of every struct.
- **Why it matters:** This is deliberate ("format evolution never breaks
  deserialization" per the skill docs) and the tradeoff is reasonable, but the
  consequences are not consistently handled downstream. A corrupt stream line
  where `session_id` was lost mid-transport parses as `ClaudeInit { session_id:
  None, … }` and flows into the semantic layer indistinguishable from a
  provider that legitimately chose not to emit a session id. The error-
  classification logic (`SemanticErrorKind`) can't tell "missing because
  omitted" from "missing because corrupt".
- **Evidence:** Every `#[derive(… Deserialize)]` struct in `stream/protocol/`
  annotates every field `#[serde(default)]`.
- **Recommendation:** Keep the pattern, but add a strict "envelope" struct per
  provider that requires the one or two fields that identify the event kind
  (e.g. `type` + `id`/`session_id`), and run a cheap second-pass validation
  against it when the semantic layer needs to act on an event. Alternatively,
  emit a `tracing::warn!` the first time a struct deserializes with every
  optional field `None`, since that strongly suggests corruption.
- **Confidence:** medium

### [Severity: Medium] Bash-action template interpolation can split values on whitespace

- **Location:** `lib/src/dispatch/runner.rs:439-495` (`execute_bash`)
- **Why it matters:** The contract — documented in the function rustdoc — is
  "author quotes placeholders in params, we `shell_words::split`, then
  `Command::args`." The flow is correct *provided* the template is
  well-formed. In practice, a user's template like
  `params: "--message {{tool_name}}"` with `tool_name = "my tool"` splits into
  `["--message", "my", "tool"]`, producing a silently-wrong invocation. The
  failure is invisible (no split error), it just runs the wrong command.
- **Evidence:**
  ```rust
  let rendered_params = interpolate(params, meta);  // "--msg my tool"
  let param_args = shell_words::split(&rendered_params)?;  // ["--msg","my","tool"]
  Command::new(cmd).args(&param_args)
  ```
- **Recommendation:** Two options:
  1. **Preferred:** change the template-params schema to an array of strings
     (`params: ["--message", "{{tool_name}}"]`) and interpolate per-item, so
     each placeholder resolves to exactly one argv slot, no matter its content.
  2. **Fallback:** keep the string form but emit a `tracing::warn!` once per
     template when the rendered `params` differs in token count from a
     pre-rendered parse of the template with placeholders replaced by a unique
     sentinel. This at least makes silent splitting diagnosable.
- **Confidence:** high

### [Severity: Medium] Sequence renderer re-compiles a regex on every render

- **Location:** `lib/src/composition/sequence.rs:341`
- **Why it matters:** `render_simple_template` compiles its placeholder regex
  via `Regex::new(…).unwrap()` inside the function. For a 100-step sequence
  with a 5-field template this is 500 regex compilations per run, each
  allocating and building an NFA. The file already imports `regex::Regex`;
  neighbouring code (`dispatch/template.rs:13,17`) uses `LazyLock` correctly.
- **Evidence:**
  ```rust
  fn render_simple_template(…) -> String {
      …
      let re = regex::Regex::new(r"\{\{\s*…\}\}").unwrap();   // every call
  ```
- **Recommendation:** Move to a `static PLACEHOLDER_RE: LazyLock<Regex>` at the
  module level, matching `dispatch/template.rs`. Replace `.unwrap()` with
  `.expect("placeholder regex is valid")` so the panic message is helpful if
  the literal is ever broken.
- **Confidence:** high

### [Severity: Medium] Silent failures on `tokio::spawn`'d bash actions

- **Location:** `lib/src/dispatch/runner.rs:468-495`
- **Why it matters:** `execute_bash` spawns a tokio task that `await`s the
  child and `warn!`s on failure. The spawn itself, however, is fire-and-forget
  — if the entire Claudine process is about to exit (common when Claudine is
  invoked as a `handle` hook under the 5-second deadline), the task is
  dropped before it runs, and no log line is emitted. There is no join or
  completion signal.
- **Evidence:**
  ```rust
  tokio::spawn(async move {
      let result = …;   // warn! on error
  });
  // return immediately; caller has no handle
  ```
- **Recommendation:** On the `handle` hook path (bounded by
  `CLAUDINE_HANDLE_DEADLINE_SECONDS`), run bash actions inline with a shorter
  timeout instead of spawning. On long-lived wrapper paths, track spawned
  tasks in a `JoinSet` so graceful shutdown can await them briefly before
  abort.
- **Confidence:** medium

### [Severity: Medium] Non-UTF-8 argv tokens silently bypass all normalization rules

- **Location:** `cli/src/argv.rs:127-184, 419-421`
- **Why it matters:** The normalizer is documented as "deliberately skip
  non-UTF-8 tokens" and the tests confirm pass-through. But the state machine
  in `apply_composition_separator` treats a non-UTF-8 token as "opaque" and
  **does not advance the positional/flag tracking** for it. A user with
  non-UTF-8 filenames on Unix can construct an argv where the positional file
  reference is a non-UTF-8 path, and Rule 3's `--` insertion will misfire
  (inserting too early, or not at all). This is unlikely in practice but the
  behaviour is under-documented.
- **Evidence:** `argv.rs:227-233` skips non-UTF-8 without incrementing the
  state machine's `saw_positional` flag, so non-UTF-8 positionals are invisible
  to Rule 3.
- **Recommendation:** Decide the policy explicitly and document it:
  - either treat non-UTF-8 tokens as positionals for state-machine purposes
    (most correct on Unix, where paths are bytes), or
  - keep the skip but add a `tracing::debug!` noting that Rule 3 was bypassed
    due to a non-UTF-8 token.
- **Confidence:** medium

### [Severity: Low] `terminal_meta_json` swallows serialization errors

- **Location:** `lib/src/dispatch/runner.rs:517-586`
- **Why it matters:** `serde_json::to_string(&value).unwrap_or_else(|_| "{}"
  .to_string())` silently replaces serialization failures with an empty
  object. The metadata object is constructed from `EventMeta`; the only ways
  `to_string` can fail are (a) a custom `Serialize` impl returns `Err`, (b)
  non-UTF-8 map keys (impossible for `serde_json::Map<String, Value>`). In
  practice this never fires, but the pattern appears several times — notably
  in `extra` and `env` — and if `EnvironmentContext` ever grows a custom
  `Serialize`, the fallback would silently hide bugs.
- **Evidence:** See lines 520, 536, 578, 582.
- **Recommendation:** Replace `unwrap_or_else(|_| "{}".to_string())` with
  `unwrap_or_else(|err| { warn!(%err, "serializing … failed"); Value::Null })`
  or similar. Losing the value is fine; silently losing it without a log is
  not.
- **Confidence:** medium

### [Severity: Low] `#[allow(dead_code)]` on `StreamThinkingRenderer`

- **Location:** `cli/src/commands/wrap/exec.rs:295-361`
- **Why it matters:** 60+ lines of unused code retained "for potential future
  use." The comment at line 288-294 explicitly says it is no longer wired in.
  Dead code drifts: its assumptions about `crate::log::terminal()` and Prose
  markup will silently rot. Either revive it or delete it.
- **Evidence:** `#[allow(dead_code)] struct StreamThinkingRenderer { … }`
  with all four `impl` methods similarly annotated.
- **Recommendation:** Delete. If it's ever needed, `git log` has it.
- **Confidence:** high

### [Severity: Low] `fs::rename` without `fsync` on the parent directory

- **Location:** `lib/src/config/atomic.rs:42`
- **Why it matters:** Standard crash-durability gap: the rename is a metadata
  change on the parent directory, and until the directory inode is flushed
  to disk, a power loss can lose the new file even though the write returned.
  For a developer tool this is rarely user-visible, but the function
  advertises atomicity.
- **Recommendation:** After `fs::rename`, `std::fs::File::open(parent)?.
  sync_all()?;` on Unix. On Windows the guarantee is platform-defined; leave
  cfg-gated.
- **Confidence:** medium

### [Severity: Low] `expect("table")` / `expect("stdout was set to piped")` messages

- **Location:** `lib/src/permissions/providers/codex.rs:998`;
  `cli/src/commands/wrap/exec.rs:458, 486, 1044, 1067, 1429, 1486, 2049`
- **Why it matters:** All of these are technically correct — the preceding
  code guarantees the invariant — but the `expect` messages are debug-grade
  and will leak into a user-visible panic if the invariant is ever violated.
  `"table"` and `"stdout was set to piped"` don't name the caller or the
  expected pre-condition.
- **Recommendation:** Either (a) prefer `if let …` / `let Some(…) else`
  destructuring so the error path is explicit, or (b) make the expect
  messages full sentences that name the function and invariant, e.g.
  `"ensure_table invariant: current[segment] was set to Item::Table above"`.
- **Confidence:** medium

---

## 3. Rust-Idiomaticity Notes

- **Error type** (`lib/src/error.rs`) is excellent — `thiserror` used well,
  per-variant domain context, `#[from]` for all wrapped error types. The
  explicit `LockError { path: PathBuf }` variant is a good example of
  *structured* errors rather than stringly-typed ones.

- **`Provider` enum** (`lib/src/events.rs`) and the `PROVIDERS_DISPLAY_ORDER`
  constant give you enum-driven iteration for free and avoid matching order
  drift across the 8 wrappers. Good use of the type system.

- **`ClaudineConfig` merge semantics** (asymmetric: repo fully replaces user
  provider configs, globals merge field-by-field) is surprising. Consider
  documenting the rationale directly on the `merge` function rustdoc rather
  than only in the skill docs, because future maintainers will reach for
  `cargo doc` first.

- **Composition unification** (2026-04-16 fix) folded three almost-identical
  `compose` / `inline-compose` / `sequence` paths into one `execute_without_harness`
  with a `CompositionExecutionMode` enum. This is exemplary refactoring —
  the enum makes the behavioural differences type-enforced instead of
  implicit. Apply the same pattern to the five near-duplicate provider
  semantic parsers (`*_semantic.rs`) where feasible.

- **`#[allow(dead_code)]` clusters:** grep shows 15+ sites. Dead code should
  either be feature-gated (`#[cfg(feature = "…")]`) behind a real-use path or
  deleted. Retention-for-future-use is how codebases slowly become
  unreviewable.

- **`.expect("…")` in non-test code** (spot count from `grep`): ~40 sites,
  mostly in the CLI `wrap/*` paths and `permissions/providers/*`. Many are
  defensible ("stdout was set to piped three lines above"); a handful
  (`permissions/providers/codex.rs:998 "table"`) deserve better messages.

- **`LazyLock<Regex>`** is used correctly in `dispatch/template.rs` and
  `stream/logs/*`. `composition/sequence.rs:341` is the only outlier.

- **`From<SemanticErrorKind> for AgentErrorCategory`** alignment across
  live-sink rendering and end-of-run report (introduced 2026-04-16) is a nice
  example of letting the type system enforce consistency.

---

## 4. Testing Gaps

High-signal scenarios that would meaningfully improve confidence:

1. **`atomic_write` under contention.** Spawn N threads that call
   `atomic_write` on the same path, assert (a) the final content is *exactly*
   one of the N payloads, not a blend, and (b) no `.tmp` file remains. This
   test will fail with the current implementation and drive the fix in
   Finding #1.
2. **`map_exit_code` on non-{0,2} codes.** `tests::mapper_exit_code_unknown`
   returning code 139 or 127 should assert the chosen policy (`None` or
   explicit warn-and-Allow). Currently only code 0 and 2 are tested.
3. **Bash-action argument splitting.** A test for `execute_bash` with
   `params = "{{tool_name}}"` and `tool_name = "my tool"`, asserting the
   spawned command observes either `["my tool"]` (with the array schema fix)
   or emits a diagnostic (with the warn fix).
4. **Claude stream tool-use DoS.** A fixture stream with a single unterminated
   `input_json_delta` that repeats 100 MiB of garbage, asserting the parser
   bounds memory and emits an error event.
5. **Signal handler PID-reuse race.** Hard to test deterministically, but a
   property-style test that checks the handler early-returns when the
   tracked `child_exited` flag is set would at least exercise the contract.
6. **Argv normalization with non-UTF-8 positionals on Unix.** The current
   `normalize_preserves_non_utf8_tokens` is pass-through only; add a test for
   `compose <non-utf-8-path> name=Ken --foo bar key=val` to pin down Rule 3
   behaviour.
7. **Kimi and Qwen protocol parsers.** No fixture-based integration tests
   exist (per skill docs: "Minimal (unit tests only)"). Add at least one
   real provider-output fixture per protocol.
8. **Error classification across all providers.** Only Claude has extensive
   `classify_error` coverage; `gemini_semantic.rs`, `codex_semantic.rs`,
   `opencode_semantic.rs`, `kimi_semantic.rs`, `qwen_semantic.rs` need
   analogous test blocks.
9. **MCP state validation after parse.** A test that feeds malformed
   `provider-state.json` (valid JSON, invalid semantics — e.g. catalog_id
   referring to a nonexistent server) and asserts the loader rejects it.
10. **Concurrent `claudine handle` invocations.** Two concurrent hook
    deliveries from the same provider session sharing the same config file.
    Would exercise both the atomic-write finding and the config-load code
    path simultaneously.

---

## 5. Unsafe Code Review

Grepping for `unsafe ` in `lib/src` + `cli/src` yields **51 sites**, broken
down as follows:

### Unjustified (remove)

- **`lib/src/permissions/json_utils.rs:25`**
  - **Invariant:** preceding code guarantees `value.is_array()`.
  - **Minimized:** yes (single expression).
  - **Documented:** only via `debug_assert!`, which is stripped in release.
  - **Verdict:** Remove. Replace with `.expect("ensure_json_array: just-set
    value must be Array")`. See Finding #2.

- **`lib/src/permissions/providers/opencode.rs:623`** — same pattern on
  `as_object_mut`. Same remediation.

- **`lib/src/permissions/providers/qwen.rs:1069`** — same pattern on
  `as_array_mut`. Same remediation.

### Justified, correct

- **`cli/src/main.rs:125`** — `std::env::set_var("NO_COLOR", "1")`
  - **Invariant (2024 edition):** no other thread may read `environ`
    concurrently.
  - **Verdict:** safe; runs before `tokio::main` spawns worker threads, before
    any `color_eyre::install()` returns. Would be safer still to pass a
    `NoColor` flag through rather than mutate global state, but acceptable.

- **`cli/src/commands/wrap/exec.rs:554-563`** — `libc::kill(-pid, SIG{TERM,KILL})`
  in `kill_process_group`.
  - **Invariant:** `pid` is a valid PID of a child we spawned with
    `process_group(0)`.
  - **Verdict:** Correct, though see Finding #6 for the PID-reuse window
    concern.

- **`cli/src/commands/wrap/exec.rs:635, 709`** — `signal_hook::low_level::
  register(…)` with a closure calling `libc::kill`.
  - **Invariant:** the closure only uses async-signal-safe syscalls.
    `libc::kill` is on the POSIX async-signal-safe list; `AtomicU8::
    fetch_add` with SeqCst compiles to a lock-free instruction that is
    signal-safe.
  - **Verdict:** Correct, with the PID-reuse caveat.

- **`cli/src/commands/wrap/exec.rs:758-760, 783-785, 906-907, 928-929`** —
  further `libc::kill` calls on timeout/early-termination paths. All use
  cached `child.id()` obtained before the last reap, on a Child the caller
  still owns. Safe.

### Test-only env mutation (benign with `#[serial]`)

- **`lib/src/composition/sequence.rs:853-920`** — six `set_var`/`remove_var`
  pairs inside `#[test]` functions gated with `#[serial]`.
- **`lib/src/system_prompt/{resolve.rs,prepare.rs}`** — same pattern.
- **`lib/src/messaging/resolve.rs:309-344`** — same.
- **`cli/src/log.rs:142-190`** — same.
- **`cli/src/commands/wrap/{exec.rs,live_semantic_sink.rs,sequence.rs}`** — same.
- **`cli/src/commands/wrap/exec.rs:1945-1978`** — same.

All are the Edition-2024 `unsafe { set_var }` pattern, guarded by
`#[serial]` / `#[serial_test::serial(…)]`, and none are reachable from
release builds. The `SAFETY:` comments on these blocks are appropriate.
**Verdict: acceptable.** (`grep "unsafe " | grep test` + `grep "#\[serial"`
confirm this.)

**Bottom line:** The only soundness-relevant `unsafe` sites are the three
`unwrap_unchecked` calls in `permissions/`, and those should simply be
removed.

---

## 6. Prioritized Next Steps

1. **Fix `atomic_write`** (Finding #1): switch to `tempfile::NamedTempFile::
   new_in(parent)`, drop the copy-fallback, add directory fsync. Add a
   contention test. This is the single most impactful change.
2. **Delete the three `unwrap_unchecked` sites** (Finding #2): replace with
   `.expect(…)`. One-line changes, zero measurable perf impact, meaningful
   soundness gain.
3. **Fix `map_exit_code`** (Finding #3): decide the policy (I recommend `_ =>
   None` + `warn!` with code/stderr), update tests, document in the `Call`
   action rustdoc.
4. **Reframe `BLOCKED_COMMANDS`** (Finding #4): delete it or explicitly mark
   it advisory; point users at `ProtectService` for the real control plane.
   Update the `bash_executor` rustdoc.
5. **Bound the Claude tool-use input buffer** (Finding #5): add a size ceiling
   mirroring the `MAX_PRE_INIT_HOOK_EVENTS` constant. Emit a terminal
   `SemanticEvent::Error` on overflow.
6. **Move `composition/sequence.rs:341` regex to `LazyLock`** (Finding #9):
   one-line fix, matches the rest of the codebase.
7. **Structured bash-action params** (Finding #8): migrate the schema from
   `params: string` to `params: Vec<String>` with per-item interpolation.
   Breaking change for configs, so schedule behind a deprecation window, but
   the silent-splitting failure mode is not fixable inside the string form.

Remaining Low / Medium findings are worth a pass but do not block shipping.

---

*Review produced by reading the HEAD tree directly plus targeted subagent
exploration of dispatch/actions/composition, stream/protocol, and
config/permissions/mcp/harness. Specific `file:line` references cite the
worktree state at `bb44da8e`.*
