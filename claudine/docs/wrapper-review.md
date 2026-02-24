# Claudine Wrapper Subcommands — Design Review

> Reviewed: 2026-02-23
> Scope: `claudine/cli/src/commands/wrap/` (mod.rs, profile.rs, env.rs, exec.rs) and supporting lib types

---

## Executive Summary

Claudine's wrapper subcommands (`claudine claude`, `claudine codex`, etc.) provide a unified launch surface for 7 agentic CLIs, normalizing YOLO mode, non-interactive mode, and environment sanitization. The architecture is well-structured with clean separation between profile definitions, environment building, and child execution. This review identifies opportunities for greater cross-provider consistency, improved ergonomics, expanded universal CLI surface, and additional security hardening.

---

## 1. Consistency Issues

### 1.1 Roo Code is Correctly Missing from Wrappers Should be Documented

`Provider::RooCode` exists in the lib's `Provider` enum with full event mapping, capability model, and sniff detection, but has no corresponding wrapper subcommand. Every other provider (except Roo) has a wrapper. This is the most visible consistency gap.

**Recommendation:** Roo Code can only be run as a plugin inside of VS Code and therefore was intentionally excluded but this should be documented.

### 1.2 Non-Interactive Mapping Has Three Distinct Strategies

The three `NonInteractiveMapping` variants handle fundamentally different semantics:

| Strategy | Providers | What it does |
|----------|-----------|--------------|
| `AppendFlag` | Claude, Kimi | Adds `--print` — one-shot execution |
| `EnsureEntrypoint` | Codex, OpenCode, Goose | Prepends subcommand (`exec`, `run`) |
| `EnsurePromptMode` | Gemini, Qwen | Validates prompt is present |

This is correct behavior per-provider, but the user experience diverges:

- For `AppendFlag` providers, you can run `claudine claude -n` with no prompt and it works (Claude enters `--print` mode and reads stdin)
- For `EnsurePromptMode` providers, `claudine gemini -n` without a prompt **fails with an error**
- For `EnsureEntrypoint` providers, `claudine goose -n` silently prepends `run` but doesn't validate that a prompt follows

**Recommendation:** Standardize the behavior: when `-n` is passed without a prompt argument, all providers should either (a) attempt stdin, or (b) produce the same style of error. Currently Gemini/Qwen fail while others silently proceed with potentially incomplete invocations. Consider adding prompt validation to `EnsureEntrypoint` providers too.


### 1.6 Typo in Help Message

Line 212 of `mod.rs`: "environement" should be "environment".

---

## 2. Code Ergonomics

### 2.1 Profile Lookup Uses String Matching

`profile_for_wrapper` does a linear scan comparing `profile.wrapper` against a string. The `main.rs` dispatch already knows which provider it's calling. This creates a fragile string-coupling.

**Recommendation:** Pass `Provider` enum directly instead of `&str`. Change the dispatch in `main.rs`:

```rust
Some(Commands::Claude(args)) => wrap::run_provider_wrapper(Provider::Claude, args),
```

Then `profile_for_wrapper` can match on the enum variant — no string matching, no possibility of typos, exhaustive match checking.

### 2.2 `Terminal::new()` is Created Multiple Times

`log_wrapper_summary`, `removed_env_info_message`, `post_env_message`, and `post_env_warning_message` each create a `Terminal::new()`. Since these are all called in sequence during wrapper execution, this could be a single instance passed through.

**Recommendation:** Create one `Terminal` at the top of `run_provider_wrapper_inner` and pass it to all rendering functions. This is a minor efficiency gain but more importantly makes the rendering context explicit.

### 2.3 `env_clear()` + `envs()` Pattern is Correct but Fragile

In `exec.rs`, the child process gets `.env_clear().envs(env)`. This is exactly right for security, but it means the sanitization logic in `env.rs` is the *only* gate. If someone adds a new env injection point and forgets `env.rs`, the variable won't be in the child.

**Recommendation:** Add a doc comment on `run_child` emphasizing that `env` must be the complete environment. Consider adding a debug assertion that critical vars (`PATH`, `HOME`) are present.

### 2.4 Flag Extraction Could Miss Provider-Native Aliases

`extract_wrapper_flags_from_passthrough` only extracts `-y`, `--yolo`, `-n`, `--non-interactive`, `--ni`. But Codex natively supports `--yolo` as an alias for its YOLO flag. If a user writes:

```
claudine codex --yolo -- exec "task"
```

The `--yolo` is extracted by Claudine (correct). But:

```
claudine codex exec --yolo "task"
```

It's unclear why this was raised as a concern!

THE CORRECT BEHAVIOR:

- `--yolo` and `-y` is ALWAYS extracted as a claudine switch
- it is then mapped to the provider's variant for  YOLO mode (including using `--yolo` if that's what the provider uses)
- if a user calls a provider via claudine wrapper then non `--yolo` and `-y` representations of yolo mode should be rejected with a well phrased error encouraging the user to use the "universal" nomenclature of `--yolo` and `-y`

### 2.5 The `WrapperArgs` Struct Could Carry the Provider

Currently `WrapperArgs` is generic across all providers. Adding a `provider: Provider` field (set by clap's subcommand dispatch) would eliminate the `wrapper: &str` parameter threading.

---

## 3. Universal CLI Surface Expansion

### 3.1 Current Universal Flags

The wrapper currently normalizes three concepts:

| Claudine Flag | Semantics |
|---------------|-----------|
| `--yolo` / `-y` | Auto-approve all tool use |
| `--non-interactive` / `-n` / `--ni` | One-shot execution, no interactive prompt |
| `--include <ENV>` | Whitelist sensitive env vars |

### 3.2 Proposed Additions

#### `--model <MODEL>` / `-m`

Currently only OpenCode gets model injection. Many users want to control which model the agent uses. Most providers support this natively:

| Provider | Native Flag | Notes |
|----------|-------------|-------|
| Claude | `--model` | Supported |
| Codex | `--model` | Supported |
| Gemini | `--model` | Supported |
| Kimi | `--model` | Supported |
| Qwen | `--model` / `-m` | Supported |
| OpenCode | `--model` / `-m` | Supported |
| Goose | `--model` | Via `GOOSE_MODEL` env or profile |

NOTE: the reason OpenCode was treated different is that because when a non-interactive session is started with OpenCode it REQUIRES that a model be provided explicitly and has no concept of a default model. The other providers do not need this special treatment.

**Recommendation:** Add `--model` as a universal wrapper flag. 

#### `--output <FORMAT>` / `-o`

Several providers support structured output in non-interactive mode:

| Provider | Formats |
|----------|---------|
| Claude | `--output-format json\|stream-json\|text` |
| Codex | `--json` flag |
| Gemini | `--output json\|text` |
| OpenCode | `--output-format json` |

**Recommendation:** Add `--output <json|text|stream>` as a universal flag. Map to provider-native format flags. Default to `text`. This is high value for scripting and CI/CD pipelines.

#### `--system-prompt <FILE>` / `-s`

Most providers support supplementing or overriding the system prompt. Claudine could provide a universal flag that maps to `--system-prompt`, `--append-system-prompt`, `SYSTEM_PROMPT` env, etc.

**Recommendation:** Add `--system-prompt <prompt | file>` as a universal flag. If the underlying provider we'll just add a `- Warning: XXX provider does not support setting the system prompt so this was skipped`. The parameter should be able to be either a file reference or a string prompt.

#### `--timeout <SECONDS>` / `-t`

No provider natively supports execution timeouts, but Claudine controls the child process and could enforce one.

**Recommendation:** Add `--timeout` that sends SIGTERM to the child after N seconds, then SIGKILL after a grace period. Valuable for CI/CD and batch execution. Inject `TIMEOUT` env var so hooks can observe it. This should only be allowed when the user specifies the prompt in a "non interactive" mode!

#### `--dry-run`

Show what would be executed without launching the child. Currently the summary output partially serves this purpose but still launches.

**Recommendation:** Add `--dry-run` that prints the full command line, environment changes, and exits 0. Useful for debugging wrapper behavior.

#### `--quiet` / `-q`

Suppress the Claudine preflight summary banner. Some users pipe output and don't want the Environment Variables block on stderr.

**Recommendation:** Add `--quiet` that suppresses all `log::message` output. Errors should still go to stderr.

**IMPORTANT:** the default behavior should always be to print the summary information to STDERR and the output from the AGENT to STDOUT so the need for this switch is dubious but it causes no harm so long as we're correctly directing the outputs to the right output streams.


## 4. Security Enhancements

### 4.1 Current Security Model — Strengths

The current implementation has solid fundamentals:

- **Environment sanitization**: Strips `API_KEY`, `TOKEN`, `PASSWORD`, `SECRET` patterns
- **Include whitelist validation**: Strict env name format (`^[A-Za-z_][A-Za-z0-9_]*$`)
- **YOLO flag interception**: Prevents users from bypassing Claudine's YOLO tracking by passing native flags directly
- **`env_clear()`**: Child gets only explicitly approved environment — no leaks
- **Exit code propagation**: No masking of child failures

### 4.2 Additional Sensitive Patterns

The `is_sensitive_key` function checks for `API_KEY`, `TOKEN`, `PASSWORD`, `SECRET`. Consider adding:

| Pattern | Rationale |
|---------|-----------|
| `PRIVATE_KEY` | SSH and signing keys |
| `CREDENTIAL` | Generic credential storage |
| `AUTH` | Authentication tokens (e.g. `GITHUB_AUTH`) |
| `PASSPHRASE` | Key passphrases |
| `ACCESS_KEY` | AWS-style access keys (currently caught by `KEY` in `API_KEY` but `AWS_ACCESS_KEY_ID` would not be caught) |

**Note:** `AWS_ACCESS_KEY_ID` is NOT currently caught because `is_sensitive_key` checks for `API_KEY` (requires the underscore grouping), `TOKEN`, `PASSWORD`, `SECRET`. The string `ACCESS_KEY` doesn't match any of these.

**Recommendation:** Add `PRIVATE_KEY`, `CREDENTIAL`, `ACCESS_KEY` to the sensitive patterns. Be cautious with `AUTH` as it would catch `OAUTH_REDIRECT_URI` and similar non-secret values. Consider a two-tier system: high-confidence patterns (always strip) and medium-confidence patterns (warn but keep).

### 4.3 Working Directory Restriction

Currently the wrapper runs the child in `cwd`. For YOLO mode especially, consider offering a `--sandbox-dir` flag that restricts the working directory.

**Recommendation:** Most providers have their own sandboxing (Codex has `--sandbox`). But for providers without sandboxing (Goose, Kimi, Qwen), Claudine could provide cgroup/namespace isolation on Linux. Start with documentation of each provider's native sandboxing capabilities. Then implement a universal `--sandbox` switch and for provider's which do not support it simply log a warning: `- Warning: the XXX provider does not provide sandboxing so the sandbox functionality will be skipped.`

**CORRECTION:** Qwen CLI does provide a `--sandbox` flag as well as `--sandbox-image`!


### 4.6 Sensitive Arg Redaction in AGENT_PARAMS

`AGENT_PARAMS` contains a JSON-encoded copy of the original CLI args. If the user passes something like `--api-key=sk-...` in passthrough args, it's stored in `AGENT_PARAMS` and visible to the child process and any hooks that read that env var.

**Recommendation:** Apply the same sensitive-pattern matching to individual args in `AGENT_PARAMS`. Redact values that look like they contain secrets (e.g., `--api-key=****`). Alternatively, only store non-sensitive flags and a hash of the full args for correlation.

### 4.7 Child Process Signal Handling

`exec.rs` spawns the child and waits for its exit. If Claudine receives SIGINT (Ctrl+C), the signal propagates to the child process group (standard Unix behavior). But if the child ignores SIGINT, Claudine's wait blocks indefinitely.

**Recommendation:** Install a signal handler that, on second SIGINT, sends SIGTERM to the child. On third SIGINT, send SIGKILL. This prevents stuck wrapper processes.

---

## 5. Messaging Improvements

### 5.2 Error Message Consistency

Error messages use different styles:

- `reject_direct_yolo_passthrough`: Uses raw ANSI `\x1b[34m` inline
- `removed_env_info_message`: Uses `Prose` with `<blue>` tags
- `opencode_non_interactive_model_hint`: Uses `<bold><blue>` tags
- `log::error`: Uses `\x1b[31m` directly

**Recommendation:** Standardize all error/warning/info messages through `Prose` rendering. Remove raw ANSI from `reject_direct_yolo_passthrough` and use the same `<blue>` tag pattern. This ensures consistent behavior with `NO_COLOR` and terminal capability detection.

### 5.3 Missing Provider Name in Output

The summary header shows badges but not *which provider* is being wrapped. A user running `claudine codex -y` sees "Claudine YOLO" but not "Codex". The provider name only appears as `AGENT=codex` in the env var list.

**Recommendation:** Add the provider display name to the header: `Claudine ▸ Codex [YOLO]`. This is the most important piece of context — which agent am I about to launch?

### 5.4 Verbose Mode Integration

The wrapper currently has no interaction with `-v`/`-vv` verbosity. All output goes through `log::message` which always prints.

**Recommendation:**

- Default: Show header + badges only (one line)
- `-v`: Show header + environment changes + warnings
- `-vv`: Show header + environment changes + full command + all debug info
- `--quiet`: Show nothing (errors only)

---

## 6. Structural Recommendations

### 6.1 Profile as a Trait Instead of a Struct

The current `ProviderProfile` struct with enum-based mappings works but requires match arms that grow with each new mapping type. Consider refactoring to a trait:

```rust
trait WrapperProfile {
    fn provider(&self) -> Provider;
    fn binary(&self) -> &str;
    fn apply_yolo(&self, args: &mut Vec<String>, env: &mut Vec<(String, String)>) -> Result<Option<String>>;
    fn apply_non_interactive(&self, args: &mut Vec<String>) -> Result<()>;
    fn apply_defaults(&self, args: &mut Vec<String>);
    fn reject_direct_yolo(&self, args: &[String]) -> Result<()>;
}
```

Each provider implements the trait. This moves provider-specific logic (like the Qwen `--approval-mode` check and Opencode model injection) into the provider's own implementation instead of generic match arms.

**Trade-off:** More files, but each provider's wrapper logic is self-contained. The current approach is fine for 7-8 providers but will get unwieldy at 12+.

**Recommendation:** Implement the suggested trait based approach.

### 6.2 Test Coverage Gaps

The integration tests cover the core scenarios well, but miss:

- Gemini wrapper behavior (only Codex, Claude, and OpenCode have integration tests)
- Goose YOLO via env var injection
- Kimi `--print` in non-interactive mode
- Qwen `--approval-mode` conflict detection
- Signal handling (SIGINT, SIGTERM propagation)
- `--include` with multiple vars
- Edge case: empty passthrough args

**Recommendation:** Add at minimum Gemini and Goose integration tests. These two have the most distinct mapping strategies (FlagValue and EnvVar respectively) and are untested at the integration level.

**IMPORTANT:** Goose is not installed on this host, the others are. Are our tests mocking or do they require the host have the underlying provider? If the later then make sure tests are conditional on the host platform having the agentic CLI installed.

### 6.3 Consider `exec` Replacement Instead of `spawn + wait`

On Unix, the wrapper could use `exec` (process replacement) instead of spawning a child. This would eliminate the signal-forwarding problem entirely — Claudine replaces itself with the provider binary.

**Trade-off:** No post-execution cleanup is possible (no audit logging, no exit code interception for reporting).

**Recommendation:** Use `spawn + wait`.
