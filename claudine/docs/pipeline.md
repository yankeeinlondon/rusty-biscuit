---
title: Compose / Inline-Compose Pipeline
description: |-
    A whole-pipeline enumeration of every step that runs from the moment
    the user presses Enter on `claudine compose` / `claudine inline-compose`
    to the moment the prompt has completed and the process exits. Steps are
    named so they can be discussed, reordered, parallelized, deferred, or
    eliminated in pursuit of an order-of-magnitude speedup of both real and
    perceived latency.
last_updated: 2026-07-23
---

# Compose / Inline-Compose Pipeline

This document is the **flat map** of the composition pipeline. It names
every step (mandatory and conditional) without diving into implementation.
The intent is to enable a holistic conversation about sequencing,
parallelism, and elimination of work — for both real wall-clock latency
and **perceived** latency (time-to-first-feedback on stderr).

The pipeline is divided into seven phases, in order:

1. **A. Pre-clap CLI** — bytes off the user's keyboard up to a parsed `Cli`.
2. **B. Prep** — everything between command dispatch and "we are about to launch the agent".
3. **C. Provider/env wire-up** — assembling the child argv / env / cwd.
4. **D. Preflight gates** — last-mile checks before spawning.
5. **E. Spawn & stream** — the long pole; child process runs, stream is parsed live.
6. **F. Closure / post-process** — assistant-text capture, file rewrite, summary.
7. **G. Termination** — exit-code path and process teardown.

Looping (`loop:` frontmatter) wraps phases B-mid through F per iteration —
described inline at the relevant boundary.

> **Legend.**
> [M] = mandatory · [O-flag] = gated by a CLI flag · [O-fm] = gated by frontmatter · [O-cfg] = gated by config · [O-mode] = differs between compose and inline-compose · [O-prov] = differs by provider.
> Numbers in `compose_prep.*`, `composition_*`, `stream_parse`, etc. are existing tracing spans you can correlate with `RUST_LOG`.

---

## A. Pre-Clap CLI

These steps run on every Claudine invocation, not just compose. They are
in scope here because their cost lands on the same critical path as the
user's first feedback frame. None of them produce visible output today.

| # | Step | Notes |
|---|------|-------|
| A1 | **Process bootstrap** [M] | Rust runtime init, panic hook install, color-eyre install. |
| A2 | **`std::env::args_os()`** [M] | Read the raw argv. |
| A3 | **`argv::normalize`** [M] | Four pre-clap rewrite rules (provider booleans → `--provider`, `--provider` fuzzy match, `--help` hoist, `--` separator insertion before trailing `key=value`). No-op under `COMPLETE`. |
| A4 | **Tracing/log subsystem init** [M] | `tracing-subscriber` install, OTLP exporter wire-up if enabled. |
| A5 | **`Cli::parse_from`** [M] | Strict pass for non-wrapper subcommands; lenient pass with `ignore_errors(true)` for wrapper subcommands. |
| A6 | **Subcommand dispatch** [M] | Reach `run_compose` or `run_inline_compose`. |

---

## B. Prep Phase

The Prep Phase is the **dominant source of perceived slowness today**: it
is the time between the user pressing Enter and the first visible
"Composition · …" execution header on stderr. Unless `--silent`, no
output is emitted before B-end.

### B1. Positional & flag preprocessing

| # | Step | Notes |
|---|------|-------|
| B1.1 | **Parse positionals** [M] | `parse_composition_positionals`: classify each positional as file-ref or `key=value` setter; reject duplicates / empty keys / multi-file. |
| B1.2 | **Install SIGINT guard** [M] | `install_user_interrupt_guard`: process-scoped signal-hook so Ctrl+C during prep emits a single async-signal-safe INFO notice and sets the global `USER_INTERRUPTED` flag. |
| B1.3 | **Validate `--timeout` / `--step-timeout` syntax** [O-flag] | Reuses the harness duration grammar so CLI/frontmatter errors share vocabulary. |
| B1.4 | **Reject `--timeout` / `--step-timeout` + `--interactive`** [O-flag] | Early CLI-only fast feedback. The authoritative resolved-mode check (which also catches `interactive: true` frontmatter and composed/env timeouts) is C5.6. |
| B1.5 | **Merge `--set` JSON5 with shorthand setters** [O-flag] | Shorthand wins on overlapping keys. |
| B1.6 | **Build `SystemPromptArgs`** [M] | Snapshot of `--append-system-prompt` / `--replace-system-prompt`. |

### B2. Source resolution & shared discovery

| # | Step | Notes |
|---|------|-------|
| B2.1 | **`resolve_composition_source`** [M] | `biscuit-file`-driven `FileReference` resolution: parse the user's argv argument (`@prompts/x.md`, relative path, absolute path), open the file, parse frontmatter + body via Darkmatter into a `Markdown`. Touches disk. |
| B2.2 | **Inline-only: pre-validate `prompt` frontmatter property** [O-mode] | Inline-compose only. Reads `prompt` from the raw frontmatter and emits an INFO/WARN check via `report_prompt_property`. |
| B2.3 | **`CompositionPrepContext::new`** [M] | One-shot prep context: `compose_prep.prep_context` span. Owns the next four steps. |
| B2.3.a | **Shared `sniff::detect_with_plan` scan** [M] | `compose_prep.shared_sniff`. Single rooted scan at launch CWD covering git summary + repo structure (no os/hw/net). Replaces what used to be two redundant scans (`LaunchContext::from_cwd` and `detect_environment_fast`). Yields a cached `LaunchContext` and a cached `EnvironmentContext`. |
| B2.3.b | **Resolve source repo root** [M] | `compose_prep.source_repo_root`. Fast path when source's parent dir is inside the launch repo (reuse launch root). Slow path: `detect_git` probe on the source's parent dir. |
| B2.3.c | **Load selection config** [M] | `compose_prep.selection_config`. Reads `~/.claudine/config.json` and (when present) `<repo>/.claudine/config.json` for `favorite_agent` and `model_overrides`. |
| B2.3.d | **Build installed-provider snapshot** [M] | `compose_prep.installed_clients`. PATH scan via `sniff::programs::InstalledAiClients` for the supported provider CLIs, filtered by `--exclude`. Supports automatic selection and the dry-run resolution breakdown; a dry-run never treats the selected executable's absence as an error. Also populates `binary_paths` used by live execution at C1.4 (W2). |

### B3. Eager target resolution

This phase decides **which provider and which model** will run, before any
template is composed. Done eagerly so `{{env.AGENT}}` in the body resolves
correctly.

| # | Step | Notes |
|---|------|-------|
| B3.1 | **Parse selection hints from raw frontmatter** [M] | `agent:` / `model:` (no compose). Treats template-shaped values as absent. |
| B3.2 | **Eager target resolution** [M] | `compose_prep.eager_target`. Branches on TTY vs non-TTY: |
| B3.2.tty | **TTY: explicit-flag wins; otherwise picker** [O-flag] | If `--<provider>` set, pick directly. Otherwise build a picker plan, render a one-shot `biscuit-tui::ChooseOne` picker. |
| B3.2.nontty | **Non-TTY: strict resolution chain** [M] | `--<provider>` > singular `agent:` > list `agent:` > `favorite_agent` > hard error. |
| B3.3 | **Probe model resolution (no catalog)** [M] | First pass to learn the `ModelResolutionReason`. |
| B3.4 | **`compose_prep.model_catalog` refresh** [O-fm] | Skipped when `model:` is absent **or** the resolved reason is `ExplicitCli` / `ProviderEnv` / `GenericEnv`. Provider-scoped: Claude/Codex are O(1) static, OpenCode/Qwen shell out to `<provider> models`, Gemini/Kimi/Goose are user-overrides only. |
| B3.5 | **Final model resolution** [M] | Re-run model resolution with the (potentially refreshed) catalog. |
| B3.6 | **Install `AGENT` env var** [M] | Both into the parent process env (`set_var`) and into `env_overrides` for the child. |

### B4. Shell preflight (template commands)

The composition pipeline pre-approves shell-expansion commands found in
the template body so Darkmatter can run them non-interactively.

| # | Step | Notes |
|---|------|-------|
| B4.1 | **Build `ComposeOptions`** [M] | Captures source path + `--set` overrides. |
| B4.2 | **Allocate shared approval cache** [M] | `Arc<Mutex<HashMap<...>>>` shared across all loop iterations and harness re-attempts. |
| B4.3 | **Build harness shell options** [M] | Combines repo-root scoped allow/deny lists and the cache. |
| B4.4 | **`compose_prep.shell_preflight`** [M] | `composition::resolve_shell_approvals` walks every `$(…)` / shell directive in the source template, asks the user (TTY) or applies allowlists (non-TTY), and populates the cache. |
| B4.5 | **Post-prep interrupt checkpoint** [M] | If SIGINT was observed at any point, short-circuit with exit 130. |

### B5. Loop entry decision

| # | Step | Notes |
|---|------|-------|
| B5.1 | **Resolve `LoopExecutionOptions`** [M] | `--max-iterations` flag > `loop.max` frontmatter > `CLAUDINE_MAX_ITERATIONS` env. |
| B5.2 | **`resolve_loop_config(source)`** [M] | Returns `Some` when `loop:` frontmatter is present. |
| B5.3 | **Branch: loop vs single-shot** [M] | When looping, every iteration re-runs B6 → F. When single-shot, B6 → F runs once. |

### B6. Per-iteration template composition

This is where Darkmatter actually runs. **Per iteration** when looping.

| # | Step | Notes |
|---|------|-------|
| B6.1 | **Compose: `prepare_direct`** [O-mode, compose] | `compose_prep.prepare_direct`. Runs the full Darkmatter compose pass on the document body: `{{ }}` expression-engine interpolation (including `env.*` access), `$(cmd)` shell expansion (against the cache), TOC linking, `@file` includes, expression evaluation, frontmatter merging. Produces `PreparedComposition.prompt`. |
| B6.2 | **Inline-compose: `prepare_inline`** [O-mode, inline] | `compose_prep.prepare_inline`. Same Darkmatter pass, but composes the `prompt` frontmatter property as the body, then appends inline guardrails (closure markers). Body is unread; only frontmatter `prompt` matters. |
| B6.3 | **Build `CompositionExecutionRequest`** [M] | Carries every flag, `prepared`, `resolved_target`, the cached prep context. |

---

## C. Provider / Env Wire-up (executor inner)

Everything below runs inside `execute_composition_request_inner` under
the `composition_prepare` span.

### C0. Dry-run short-circuit

| # | Step | Notes |
|---|------|-------|
| C0.1 | **`--dry-run`** [O-flag] | Emit the composed body/frontmatter and return before every launch-wiring step below. Provider/model identity was resolved in B3; selected-executable availability/path resolution, MCP, argv, system-prompt delivery, lifecycle runtime, and spawn do not run. The selected provider need not be installed. |

### C1. Provider/target re-use or re-resolve

| # | Step | Notes |
|---|------|-------|
| C1.1 | **Reuse eager target** [M] | When `request.resolved_target` is `Some` (compose's normal path), reuse it and skip the entire C1 fallback below. Saves a duplicate `InstalledAiClients` PATH scan + `load_selection_config` + catalog build. |
| C1.2 | **Fallback re-resolution** [O] | Only triggered by callers that don't pre-resolve (legacy library callers). Mirrors B2.3 + B3 inline. |
| C1.3 | **Look up `WrapperProfile`** [M] | `profile_for_provider(provider)`. |
| C1.4 | **Resolve binary path** [M, live only] | `resolve_binary_path_direct(profile)` — consults the `InstalledProviderSnapshot.binary_paths` map built during B2.3.d; falls back to `which::which` only for legacy callers. Dry-run returned at C0. **Optimized by W2.** |
| C1.5 | **Inline+interactive support check** [O-mode] | Hard-fail when inline closure is requested with an interactive provider that doesn't support it. |
| C1.6 | **Compute `effective_non_interactive`** [M] | `!session_interactive`. Drives every downstream branching decision. |

### C2. Early header (first user-visible feedback)

| # | Step | Notes |
|---|------|-------|
| C2.1 | **Build header `EnvPlan`** [M] | Lightweight env plan for the header only; package context comes from `request.prep_launch_workspace` (cached in B2.3.a via W0). No redundant filesystem scans. |
| C2.2 | **Emit execution header** [M, unless `--silent`] | `crate::output::log_wrapper_header` — first line on stderr. **This is the perceived-latency line.** Everything before this is invisible to the user. |

### C3. MCP composition (optional)

| # | Step | Notes |
|---|------|-------|
| C3.1 | **Decide MCP shadow-HOME need** [M] | Codex/Gemini + (`--mcp` or `--use`). |
| C3.2 | **Decide repo shadow-HOME need** [O-flag] | `--repo`. |
| C3.3 | **Build child env (`env::build_child_env_with_launch`)** [M] | Uses the pre-computed `LaunchWorkspaceContext` from `request.prep_launch_workspace` (W0). No redundant `resolve_launch_workspace_context` call. |
| C3.4 | **Apply `--operation` env override** [O-flag] | |
| C3.5 | **Apply request-level env overrides** [M] | E.g., `FAIL_FAST` from sequence. |
| C3.6 | **MCP: bootstrap state** [O-flag] | `bootstrap_mcp_state`. |
| C3.7 | **MCP: load catalog** [O-flag] | `McpCatalogStore::load`. |
| C3.8 | **MCP: lex `#tags` from prompt** [O-flag] | |
| C3.9 | **MCP: `compute_session_set`** [O-flag] | Resolve tags → server set; ambiguity prompts in TTY+interactive. |
| C3.10 | **MCP: handle missing/ambiguous tags** [O-flag] | `--strict` makes them fatal. |
| C3.11 | **MCP: provider injector** [O-flag, O-prov] | Codex/Gemini/OpenCode get runtime injection (writes shadow config or sets `OPENCODE_CONFIG_CONTENT`); other providers hard-fail with a hint to use `claudine mcp export`. |

### C4. Provider argv assembly

| # | Step | Notes |
|---|------|-------|
| C4.1 | **`--yolo`** [O-flag] | `apply_yolo_for_mode`. May warn. |
| C4.2 | **`apply_entrypoint`** [M, O-prov] | Provider-specific entrypoint flags. |
| C4.3 | **`apply_non_interactive_flags`** [M, O-prov] | When `effective_non_interactive`. |
| C4.4 | **OpenCode model resolution** [O-prov] | Provider-specific model wiring (`apply_opencode_model_resolution`). |
| C4.5 | **Universal `--model` flag** [O-prov] | Non-OpenCode (and OpenCode interactive) fall through to `profile.apply_model`. |
| C4.6 | **Validate non-interactive requirements** [M, O-prov] | E.g., a prompt is required for non-interactive. |
| C4.7 | **Universal `--output` flag** [O-flag] | `profile.apply_output_format`. |

### C5. System prompt

| # | Step | Notes |
|---|------|-------|
| C5.1 | **Enforce `--repo` launch detection** [O-flag] | Hard-fail when prep-time sniff failed and `--repo` is set. |
| C5.2 | **Reuse cached `LaunchContext`** [M] | Otherwise (legacy path) call `LaunchContext::from_cwd`. |
| C5.3 | **`resolve_and_prepare_for_session`** [M] | Reads CLI args, walks launch-context hierarchy for `system-prompt.md` discovery, runs Darkmatter prep. |
| C5.4 | **`profile.apply_system_prompt`** [O-prov] | Provider-specific delivery: Claude `--append-system-prompt` flag, Codex shadow `instructions.md` file, Gemini `OPENCODE_CONFIG_CONTENT`-style env, etc. May produce `sp_artifacts` + warnings. |
| C5.5 | **Apply `--sandbox`** [O-flag, O-prov] | |
| C5.6 | **Resolved-mode timeout conflict** [O-mode] | Authoritative check: when `session_interactive` is true, an explicit `timeout` / `step_timeout` from any source (CLI, composed frontmatter, env) hard-fails. The built-in 30m `step_timeout` default is excluded. Error names the resolved source (`--interactive` vs `frontmatter`). |
| C5.7 | **Append MCP extra args** [O-flag] | |

### C6. Structured streaming decision

| # | Step | Notes |
|---|------|-------|
| C6.1 | **Decide `use_structured`** [M] | `profile.supports_structured_stream() && effective_non_interactive`. |
| C6.2 | **Apply structured-stream flags** [O-prov] | `--output-format stream-json`, etc. |
| C6.3 | **Codex captured-output prep** [O-prov] | When `provider == Codex` and either structured or interactive-inline, allocates an output capture file and adds `--output-last-message` to argv. |

### C7. Prompt delivery wiring

| # | Step | Notes |
|---|------|-------|
| C7.1 | **Snapshot `args_before_prompt`** [M] | The harness loop needs a prompt-free base. |
| C7.2 | **`profile.prompt_delivery`** [M, O-prov] | Decide: stdin seed, positional argv, `--prompt` flag, JSON-RPC wire (Kimi). |
| C7.3 | **Apply delivery to argv** [M] | May append the prompt to argv or capture it for stdin. |
| C7.4 | **Validate prompt presence** [M] | `require_prompt_present`. |
| C7.5 | **Validate argv flags before separator** [O, debug-only] | Warn-level lint. |
| C7.6 | **Mark env setup complete (perf)** [O-flag] | |

### C9. Switch process CWD

| # | Step | Notes |
|---|------|-------|
| C9.1 | **`switch_process_cwd(child_cwd)`** [M] | The wrapper deliberately mutates parent CWD to the chosen child CWD (repo root for permission/trust scope). Documented invariant — see CLAUDE.md memory. |

---

## D. Preflight Gates

Spans `composition_preflight`. Last-mile checks before spawn.

| # | Step | Notes |
|---|------|-------|
| D1 | **"Starting pre-flight checks" status** [M, unless `--silent`/`--quiet`] | Second user-visible feedback line. |
| D2 | **Detect harness-enabled** [M] | `harness::has_harness_properties(effective_frontmatter)`. |
| D3 | **Build harness shell options** [M] | Reuses the shared approval cache from B4.2. |
| D4 | **Lifecycle config load** [O-fm] | Skipped when `lifecycle:` frontmatter is empty. Otherwise loads dispatch config + bridges TTS / messaging settings. |
| D5 | **Build `LifecycleRunGuard`** [M] | Will emit `start` / `success` / `failure` / `blocked` events. |
| D6.harness | **Parse `HarnessPlan`** [O-fm, harness-enabled] | `harness::parse_harness_plan` validates the harness frontmatter. |
| D6.inline-harness | **Prepend writability pre-check** [O-mode, inline + harness-enabled] | Allows handler-driven recovery on permission failure. |
| D6.preflight | **Harness shell preflight** [O-fm] | `resolve_shell_approvals` for harness commands. |
| D6.inline | **Inline writability pre-check** [O-mode, inline] | Injected as the first `pre_check` by `finalize_effective_plan`; enforced inside the harness loop. Hard-fail because no handler exists to recover. |
| D7 | **Preflight-complete status** [M, unless `--silent`/`--quiet`/`--sequence`] | "Preflight: shell commands approved …" |
| D8 | **`compose_prep.environment` (re-detect or reuse)** [M] | Reuses cached `EnvironmentContext` from B2.3.a when `env_detect_root` matches; otherwise calls `detect_environment_fast`. |
| D9 | **Render env details** [O-flag] | `--quiet` suppresses; otherwise shown when interactive or `-v`. |
| D10 | **Render system prompt** [O-flag] | `log_system_prompt`. |
| D11 | **Render compose prompt** [O-mode, compose-only when non-interactive] | `log_compose_prompt` echoes the composed prompt body to stderr. |

---

## E. Spawn & Stream

Spans `composition_execute`. The long pole.

### E1. Composition execution path

| # | Step | Notes |
|---|------|-------|
| E1.0 | **Build `dispatch_context`** [M] | `composition_dispatch_context`. |
| E1.1 | **`run_harness_loop`** [M] | Per-attempt: re-parse plan, run pre-checks, spawn child, stream, run post-checks, on failure invoke handler recovery, possibly retry. Bare documents yield the empty plan and still execute through the loop. |

### E2. Sink / parser construction

(Per attempt inside the harness loop.)

| # | Step | Notes |
|---|------|-------|
| E2.1 | **Allocate `Arc<Mutex<StructuredSummaryDetails>>`** [M] | |
| E2.2 | **Build `LiveSemanticSink`** [M, structured-only] | Wires section tracker, watchdog state, live metrics, stream output, dispatch-context extras. |
| E2.3 | **Build `ParserConfig`** [M] | |
| E2.4 | **`build_structured_plumbing`** [M] | Pairs the parser builder with the stderr bridge. |
| E2.5 | **Preload `DispatchRuntimeContext`** [O-prov] | Wire (Kimi) only. |

### E3. Child process spawn

| # | Step | Notes |
|---|------|-------|
| E3.1 | **`Command::new(binary)` + `env_clear()` + `envs(env)`** [M] | The only env gate. |
| E3.2 | **Decide `isolate_process_group`** [M] | True when stdin/stdout/stderr is piped. |
| E3.3 | **Apply stdin/stdout/stderr stdio** [M] | Pipe vs inherit decisions per provider. |
| E3.4 | **`spawn()`** [M] | Mark `child_spawned = true`. |
| E3.5 | **Mark provider-launched on lifecycle guard** [M] | So later failures classify as `Failure` not `Blocked`. |
| E3.6 | **Send stdin seed** [O-prov] | When `stdin_seed` is `Some`. |
| E3.7 | **Wire JSON-RPC** [O-prov, Kimi] | `run_kimi_wire_session`: `initialize` request, `prompt/send`, etc. |

### E4. Live streaming

| # | Step | Notes |
|---|------|-------|
| E4.1 | **Spawn stdout reader thread** [M] | Read line-by-line; per-line spans wrapped in `stream_parse`. |
| E4.2 | **Per-line: parse JSON Value** [M] | Two-pass dispatch; `serde_json::Value` first. |
| E4.3 | **Per-line: typed deserialize into `*Event`** [M] | Provider-specific tagged enum. |
| E4.4 | **Per-line: feed `LiveSemanticSink`** [M] | Tool calls, reasoning, assistant text, errors, metadata. |
| E4.5 | **Emit `→ Name(summary)` / `← Name(slot)`** [M] | Tool call rendering. |
| E4.6 | **Emit `Section::Thinking` / reasoning** [O-prov] | Claude/Codex/OpenCode/Gemini/Qwen. |
| E4.7 | **Spawn stderr bridge thread** [M] | Filter noise prefixes; classify errors. |
| E4.8 | **Spawn `flush_if_idle` ticker** [M] | 30-second silence → flush dangling block; emit at most one `⏳ Awaiting subagent` line per active subagent per silence window. |
| E4.9 | **Spawn prompt-timing ticker** [M] | 10-minute interval `t=0` / `t=10m` headers. |
| E4.10 | **Spawn timeout watchdog ticker** [O-flag/fm] | `wall_clock` (`timeout`) and `step_silence` (`step_timeout`). On breach: render `Agent Error` block with stuck-subagent detail and synthesize `error_kind`. |
| E4.11 | **Update `WatchdogState` on `SubagentStart`/`SubagentStop`** [M] | |

### E5. Wait + termination escalation

| # | Step | Notes |
|---|------|-------|
| E5.1 | **`wait_with_signal_and_early_termination`** [M] | Cooperative wait that respects SIGINT escalation, watchdog channel, and EOF. |
| E5.2 | **SIGINT escalation** [O] | Second Ctrl+C → SIGTERM; third → SIGKILL. |
| E5.3 | **Timeout escalation** [O-flag/fm] | `CLAUDINE_KILL_GRACE` (default 10s) between SIGTERM and SIGKILL. |
| E5.4 | **Reap & collect telemetry** [M] | `ProcessTelemetry`. |

---

## F. Closure / Post-process

Spans `composition_postprocess`.

### F1. Codex post-hoc capture

| # | Step | Notes |
|---|------|-------|
| F1.1 | **`StructuredCodexOutput::apply_to_summary`** [O-prov, Codex] | Reads the captured-output file and patches `summary.assistant_text` when Codex didn't stream it. |

### F2. Compose-mode post-process

| # | Step | Notes |
|---|------|-------|
| F2.1 | **Render assistant markdown to stdout** [M, when no live streaming] | Section-stream-aware so the trailer matches. Markdown rendering only when `stdout` is a TTY. |
| F2.2 | **`emit_composition_summary`** [M, structured] | Trailer block (model, duration, tokens, cost). |
| F2.3 | **`emit_stream_summary`** [M] | Trailer block (model, duration, tokens, cost) emitted by the harness loop for both structured and captured/non-structured attempts. |

### F3. Inline-compose post-process

| # | Step | Notes |
|---|------|-------|
| F3.1 | **Validate closure plan** [M] | Closure markers present in agent response. |
| F3.2 | **`split_frontmatter_and_body`** [M] | Pull the new body out of the response. |
| F3.3 | **Markdown cleanup of new body** [M] | `cleanup_content` runs inside `apply_inline_closure` so the cleaned body is hashed and written. |
| F3.4 | **Update `last_updated` frontmatter** [M] | |
| F3.5 | **Atomic write of target file** [M] | Single write; the stamped `hash:` describes the cleaned body. |
| F3.6 | **Deferred `emit_composition_summary`** [M] | Emitted *after* closure validation messages so the section separator does not split the block. |

### F4. Lifecycle terminal signal

| # | Step | Notes |
|---|------|-------|
| F4.1 | **`guard.emit_terminal(Success | Failure)`** [M] | Fires `composition_success` / `composition_failure` lifecycle events → TTS / messaging / sound effects per dispatch config. |

### F5. Loop iteration close (when looping)

| # | Step | Notes |
|---|------|-------|
| F5.1 | **Apply loop actions** | `increment` / `decrement` / `set` / `append` / `prepend` / `merge`. |
| F5.2 | **Evaluate loop condition** | Darkmatter expression against ambient + frontmatter variables. |
| F5.3 | **Lookahead `is_last`** | |
| F5.4 | **Loop back to B6 or break** | |

---

## G. Termination

| # | Step | Notes |
|---|------|-------|
| G1 | **`--perf` report** [O-flag] | Always emitted to stderr when set; overrides `--silent`/`--quiet`. |
| G2 | **Drop SIGINT guard** [M] | RAII restores prior handler. |
| G3 | **Switch back to launch CWD** [O] | Wrapper does *not* restore — see project memory note about intentional CWD switch. |
| G4 | **`std::process::exit(code)`** [M] | Top-level walker translates errors into `BlockError` reports first. |

---

## Where the Time Goes Today

(Indicative, not measured here. Use `RUST_LOG=info,compose_prep=trace` +
`--perf` to get real numbers in your environment.)

| Hot region | Why it can be slow |
|---|---|
| **B2.3.a `compose_prep.shared_sniff`** | Filesystem walk for git + repo structure. Now also builds the `LaunchWorkspaceContext` reused in C2.1 and C3.3 (W0). |
| **B2.3.b `compose_prep.source_repo_root`** | When the source lives outside the launch repo, full `detect_git` probe (HEAD + branch + upstream + commit summary). |
| **B2.3.d `compose_prep.installed_clients`** | Full PATH scan for *all* eight provider binaries every invocation. |
| **B3.4 `compose_prep.model_catalog`** | OpenCode and Qwen shell out to `<provider> models` synchronously. |
| **B4.4 `compose_prep.shell_preflight`** | Each `$(…)` in the template runs synchronously. |
| **B6.1 / B6.2 `prepare_direct` / `prepare_inline`** | Full Darkmatter compose pass; for big templates this is non-trivial. |
| **C1.4 `resolve_binary_path_direct`** | Consults `InstalledProviderSnapshot.binary_paths` built during B2.3.d; no fresh PATH scan in the common path. **Optimized by W2.** |
| **C5.3 `resolve_and_prepare_for_session`** | Walks launch-context hierarchy + Darkmatter prep on `system-prompt.md`. |
| **D8 `compose_prep.environment`** | Cache hit is common after Phase 2 fixes, but the legacy fallback re-runs `detect_environment_fast`. |
| **E3.4 `spawn()`** | Provider startup is the largest unavoidable single cost (Node/Python/Go warmup). |
| **E4.* live streaming** | The agent's real work — outside Claudine's control. |

---

## Perceived-Latency Anchors

In order of when they appear on stderr today:

1. **W1 — receipt banner** (`→ Composing <file>…`) — first visible byte within ~50ms of process start (Phase A/B boundary). **Added by W1.**
2. **C2.2 — execution header** (provider, mode, yolo, etc.). Follows after prep completes.
3. **D1 — "Starting pre-flight checks"** status.
4. **D7 — "Preflight: shell commands approved"** status.
5. **D9 / D10 / D11** — env / system-prompt / compose-prompt blocks (verbose-gated).
6. **E4.* — live tool calls & assistant text streaming.**
7. **F2.2 / F3.6 — trailer summary.**

The **time from Enter to step 1** is the dominant perceived-latency win
available, because today the user stares at a blank screen through the
entire Prep Phase. Even partial progress (e.g., a "Resolving prompt…"
status before B2.1, or streaming the prep checks as they run) would
massively change the felt experience without changing real wall-clock at
all.

---

## Optionality Cheatsheet

For quick scanning, the steps that are **never run** under common flag
combos:

- `--dry-run`: returns at C0 and skips C1 onward, including selected-executable validation/path resolution, MCP/argv/CWD setup, lifecycle, spawn, and closure.
- `--silent`: skips B2.2, all status banners (C2.2, D1, D7, D9-D11), warnings.
- `--quiet`: skips C2.2's verbose details, D9, optional warnings.
- No `loop:` frontmatter: skips F5 entirely; B6 runs once.
- No `lifecycle:` frontmatter: skips D4 (`load_claudine_config` for runtime config).
- No `--mcp` / `--use`: skips C3.6-C3.11.
- No `--repo` and no MCP: skips shadow-HOME setup in C3.3.
- No harness frontmatter: skips D6.harness/preflight shell-approval differences; still runs through E1.1 with the bare plan.
- TTY + explicit `--<provider>`: skips picker in B3.2.tty.
- `--<provider>` flag: skips all picker UI.
- Catalog refresh skipped (B3.4) when `model:` absent or model came from CLI/env.
- Fast path for B2.3.b: source inside launch repo → reuse launch root, no `detect_git`.
- Cache hit for D8: `env_detect_root` matches the prep-time scan root → no re-scan.

---

## Trace Span Index (existing)

Spans you can already filter on with `RUST_LOG`:

- `compose` / `inline_compose` (top-level)
- `compose_prep.prep_context`
- `compose_prep.shared_sniff`
- `compose_prep.source_repo_root`
- `compose_prep.selection_config`
- `compose_prep.installed_clients`
- `compose_prep.eager_target`
- `compose_prep.model_catalog`
- `compose_prep.shell_preflight`
- `compose_prep.prepare_direct` / `compose_prep.prepare_inline`
- `composition_prepare`
- `composition_preflight`
- `compose_prep.environment`
- `composition_execute`
- `stream_parse`
- `composition_postprocess`
- `kimi_wire_session` / `kimi_wire_stdout` / `kimi_wire_initialize` / `kimi_wire_prompt_send` / `kimi_wire_cancel`

---

## Adjacent References

- [`composition.md`](topics/composition.md) — narrative description of the same pipeline with frontmatter / flag impact.
- [`execution-flow.md`](topics/execution-flow.md) — older walk-through across compose / inline-compose / sequence.
- [`pre-flight-checks.md`](topics/pre-flight-checks.md) — preflight semantics in detail.
- [`signal-handling.md`](topics/signal-handling.md) — SIGINT / SIGTERM / SIGKILL escalation.
- [`timeouts.md`](topics/timeouts.md) — timeout / step_timeout precedence and watchdog.
- [`stream-parsing.md`](topics/stream-parsing.md) — typed protocol models and parser dispatch.
- [`performance-testing.md`](topics/performance-testing.md) — how to measure.
