---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
docs: https://codelabs.developers.google.com/antigravity-cli-hands-on
invocation:
  - command: 'agy -p "prompt"'
    stdin_support: false
    prompt_arg: "-p is a short alias for --print; the prompt is supplied as the flag value."
    notes: "Starts a fresh non-interactive print-mode run and prints only plain text to stdout when successful."
  - command: 'agy --print "prompt"'
    stdin_support: false
    prompt_arg: "--print consumes the following prompt text."
    notes: "Same print mode as -p. No structured stream or session-start event is documented."
  - command: 'agy --prompt "prompt"'
    stdin_support: false
    prompt_arg: "--prompt is an alias for --print."
    notes: "Same print mode as -p."
  - command: 'agy --conversation <ID> -p "prompt"'
    stdin_support: false
    prompt_arg: "--conversation resumes a previous conversation by ID; -p supplies the next prompt."
    notes: "Resume is supported only when the caller already has an ID from another source; print mode does not document an emitted conversation ID."
  - command: 'agy -c -p "prompt"'
    stdin_support: false
    prompt_arg: "-c/--continue resumes the most recent conversation and -p supplies the next prompt."
    notes: "Scriptable but globally ambiguous; unsafe for parallel wrappers."
  - command: 'agy --prompt-interactive "prompt"'
    stdin_support: false
    prompt_arg: "Initial prompt string."
    notes: "Starts the TUI after sending the initial prompt; not a non-interactive wrapper mode."
output_formats:
  - name: "plain print"
    cli_value: "-p / --print / --prompt"
    stream: false
    format: text
    description: "Plain final assistant text on stdout. Errors may also appear as plain text on stdout."
    side_effects: "Does not expose tool calls, tool results, file changes, session metadata, usage, cost, or a terminal event."
  - name: "interactive TUI"
    cli_value: "no -p/--print"
    stream: true
    format: text
    description: "Terminal UI with live agent, tool, permission, artifact, task, and status information."
    side_effects: "Requires a TTY and is not parser-safe for Claudine automation."
  - name: "prompt interactive"
    cli_value: "-i / --prompt-interactive"
    stream: true
    format: text
    description: "Starts the interactive UI with an initial prompt."
    side_effects: "Conflicts with non-interactive operation."
  - name: "status line payload"
    cli_value: "settings.json statusLine.command"
    stream: true
    format: json
    description: "The TUI can pipe current state JSON to a configured status line command."
    side_effects: "Secondary customization callback, not stdout, not documented as print-mode telemetry, and not a complete agent event stream."
schema_sources:
  - url: "https://codelabs.developers.google.com/antigravity-cli-hands-on"
    schema_type: examples
    formal: false
    notes: "Official Google codelab documents -p print mode, --model, settings.json, tool permission modes, and --dangerously-skip-permissions; it does not define a structured event schema."
  - url: "https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md"
    schema_type: examples
    formal: false
    notes: "Versioned official changelog documents print-mode fixes, permission merging, sandbox propagation, quota/status-line features, subagent behavior, and config paths."
  - url: "https://github.com/google-antigravity/antigravity-cli/blob/main/examples/statusline/statusline.sh"
    schema_type: examples
    formal: false
    notes: "Example status-line script shows an informal JSON payload with agent_state, context_window.used_percentage, vcs, sandbox, artifact_count, subagents, task_count, model.display_name, and terminal_width."
  - url: "https://github.com/google-antigravity/antigravity-cli/issues/119"
    schema_type: none
    formal: false
    notes: "Open feature request asks for Gemini-style --output-format stream-json, which is evidence that this format is not currently available."
cli_params:
  - flag: "-p, --print, --prompt"
    value: "string"
    description: "Run a single prompt non-interactively and print the response."
    example: 'agy -p "What is the gcloud command to deploy to Cloud Run"'
  - flag: "--print-timeout"
    value: "duration"
    description: "Set the print-mode wait timeout; local help shows default 5m0s."
    example: "agy --print-timeout 30s -p prompt"
  - flag: "--conversation"
    value: "conversation ID"
    description: "Resume a previous conversation by ID."
    example: 'agy --conversation 116191af-e6ea-4ba5-aa23-62f995bd068a -p "continue"'
  - flag: "-c, --continue"
    value: "boolean"
    description: "Continue the most recent conversation."
    example: 'agy -c -p "continue"'
  - flag: "--model"
    value: "display model name"
    description: "Select the model for the current CLI session."
    example: 'agy --model "Gemini 3.5 Flash (Low)" -p prompt'
  - flag: "--mode"
    value: "accept-edits | plan"
    description: "Set the agent execution mode for the session; changelog documents default, accept-edits, and plan cycling."
    example: "agy --mode plan -p prompt"
  - flag: "--dangerously-skip-permissions"
    value: "boolean"
    description: "Auto-approve all tool permission requests without prompting."
    example: "agy --dangerously-skip-permissions -p prompt"
  - flag: "--sandbox"
    value: "boolean"
    description: "Run with terminal sandbox restrictions enabled."
    example: "agy --sandbox -p prompt"
  - flag: "--add-dir"
    value: "directory"
    description: "Add a directory to the workspace; repeatable."
    example: "agy --add-dir ../shared -p prompt"
  - flag: "--project"
    value: "project ID"
    description: "Use an existing project for the current session."
    example: "agy --project PROJECT_ID -p prompt"
  - flag: "--new-project"
    value: "project ID or name"
    description: "Create a new project for the current session."
    example: "agy --new-project PROJECT_ID -p prompt"
  - flag: "--log-file"
    value: "path"
    description: "Override the CLI log file path."
    example: "agy --log-file ./agy.log -p prompt"
  - flag: "-i, --prompt-interactive"
    value: "string"
    description: "Run an initial prompt interactively and continue in the TUI; avoid for Claudine non-interactive runs."
    example: 'agy -i "start here"'
config_files:
  - os: macos
    scope: user
    path: "~/.gemini/antigravity-cli/settings.json"
    format: json
    effect: "Stores colorScheme, model, statusLine, trustedWorkspaces, permissions, sandbox, telemetry, credits, and other CLI settings."
    notes: "Official codelab identifies this as the settings file. CLI flags such as --model and --mode affect the current session."
  - os: linux
    scope: user
    path: "~/.gemini/antigravity-cli/settings.json"
    format: json
    effect: "Same user settings behavior as macOS."
    notes: "Path is inferred from the documented Unix-style location and cross-platform CLI install; verify on Linux before relying on it for writes."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\settings.json"
    format: json
    effect: "Same user settings behavior as macOS/Linux."
    notes: "Windows issue reports use the ~/.gemini form; the expanded Windows path should be verified on a Windows host."
  - os: macos
    scope: user
    path: "~/.gemini/config/mcp_config.json"
    format: json
    effect: "Shared MCP server configuration used by Antigravity CLI and related Antigravity surfaces."
    notes: "Changelog records migration from the private antigravity-cli path to the shared config path."
  - os: linux
    scope: user
    path: "~/.gemini/config/mcp_config.json"
    format: json
    effect: "Same MCP configuration behavior as macOS."
    notes: "Supports url in addition to older server URL fields according to the changelog."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\mcp_config.json"
    format: json
    effect: "Same MCP configuration behavior as macOS/Linux."
    notes: "Windows path is inferred from the shared ~/.gemini config root."
  - os: macos
    scope: user
    path: "~/.gemini/config/projects/"
    format: json
    effect: "Project-specific configuration and permissions; changelog says project-specific configuration takes precedence over global settings."
    notes: "Exact per-project file names and merge keys were not verified."
  - os: linux
    scope: user
    path: "~/.gemini/config/projects/"
    format: json
    effect: "Same project-specific configuration behavior as macOS."
    notes: "Exact per-project file names and merge keys were not verified."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\projects\\"
    format: json
    effect: "Same project-specific configuration behavior as macOS/Linux."
    notes: "Exact per-project file names and merge keys were not verified."
  - os: macos
    scope: user
    path: "~/.gemini/antigravity-cli/cache/projects.json"
    format: json
    effect: "Central workspace-to-project mapping cache."
    notes: "Changelog says this replaced local .antigravitycli workspace directories."
  - os: linux
    scope: user
    path: "~/.gemini/antigravity-cli/cache/projects.json"
    format: json
    effect: "Same project cache behavior as macOS."
    notes: "Used for project discovery, not output formatting."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\cache\\projects.json"
    format: json
    effect: "Same project cache behavior as macOS/Linux."
    notes: "Windows path is inferred."
env_vars:
  - name: "AGY_CLI_HIDE_ACCOUNT_INFO"
    effect: "Hides email and plan tier from the interactive header."
    notes: "Useful for screenshots/log hygiene; does not create structured output."
  - name: "AGY_CLI_CMD_OUTPUT_PERCENTAGE"
    effect: "Controls maximum command-output height in the TUI as a percentage of terminal height."
    notes: "Rendering-only; relevant if Claudine ever wraps a PTY/TUI mode."
  - name: "AGY_CLI_DISABLE_LATEX"
    effect: "Disables LaTeX rendering globally."
    notes: "Rendering-only; can reduce terminal output surprises but does not affect print-mode schema."
io_contract:
  stdout: text_only
  stderr: diagnostics_only
  stdin: ignored
  framing: text
  noise_handling: "Treat stdout as human text, not parseable events. Also inspect stdout for plain-text errors because a local auth-timeout smoke test printed the error on stdout with no stderr."
  notes: "No documented JSON/JSONL/SSE/JSON-RPC stdout mode exists for agy print mode as of 1.1.0."
stream_contract:
  discriminator: "none"
  event_ordering: "No structured event stream is exposed in print mode."
  correlation_fields: []
  terminal_event: "none"
  partial_message_events: false
  unknown_event_policy: "Not applicable until Antigravity adds a structured stream."
  notes: "Status-line JSON is a customization callback payload, not the print-mode output stream."
session_metadata:
  session_id: "Not emitted in print-mode stdout/stderr; --conversation can consume an existing ID."
  cwd: "Not emitted in print mode; status-line payload can include workspace.current_dir in TUI mode."
  model: "Requested by --model or settings.json; status-line payload can include model.display_name in TUI mode."
  provider: "Antigravity/agy implied by executable; not emitted as a field."
  auth: "Authentication failures are visible as plain text; auth source is not emitted."
  version: "agy --version prints the CLI version outside a run; not emitted in print mode."
  mcp_servers: "Configured by mcp_config.json; not emitted in print mode."
  permission_mode: "Configured by settings and flags; not emitted in print mode."
  notes: "The TUI status-line payload exposes useful runtime metadata, but only to configured customization commands."
stream_events:
  - event: "plain assistant text"
    category: assistant
    fields: []
    notes: "Final response text only; no event envelope."
  - event: "plain error text"
    category: error
    fields: []
    notes: "Observed locally for auth timeout as stdout text: Error: authentication failed or timed out."
  - event: "statusLine payload"
    category: other
    fields: ["agent_state", "context_window.used_percentage", "vcs.branch", "vcs.dirty", "sandbox.enabled", "artifact_count", "subagents", "task_count", "model.display_name", "terminal_width"]
    notes: "Informal TUI customization payload from official example script; not a complete event stream."
tools:
  - name: "filesystem tools"
    call_visible: false
    result_visible: false
    metadata: []
    notes: "Interactive examples show Read/ListDir/Create-style tool renderings, but print mode does not expose structured tool events."
  - name: "terminal commands"
    call_visible: false
    result_visible: false
    metadata: []
    notes: "Permissions and sandboxing affect execution, but command stdout/stderr/exit status are not surfaced structurally in print mode."
  - name: "web/read URL/search tools"
    call_visible: false
    result_visible: false
    metadata: []
    notes: "Interactive examples show ReadURL/WebSearch; no print-mode structured events."
  - name: "MCP tools"
    call_visible: false
    result_visible: false
    metadata: []
    notes: "MCP servers are configurable, including URL-based servers, but MCP calls are not distinguishable in print-mode output."
completion:
  success_event: "none"
  failure_event: "none"
  exit_code_reliable: false
  result_fields: []
  cost_fields: []
  usage_fields: []
  notes: "Use process exit plus non-empty stdout heuristics. A local auth-timeout smoke test exited through the outer timeout with a plain stdout error and no stderr; historical Windows issue #76 reported exit 0 with empty output before a later fix."
blocking_behavior:
  permissions: configurable
  questions: hang
  tool_approvals: configurable
  notes: "Default request-review mode is interactive. --dangerously-skip-permissions auto-approves tool permissions; proceed-in-sandbox and always-proceed can be configured, but artifact review and model questions have no documented programmable answer channel in print mode."
subagents:
  supported: true
  start_visible: false
  stop_visible: false
  nested_events_visible: false
  prompt_injection_supported: false
  metadata_fields: ["statusLine.subagents"]
  notes: "CLI supports subagents and status-line payload can include a subagents array/count, but print mode does not expose subagent lifecycle or nested tool events."
use_cases:
  - name: "plan_cap_approaching"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "No print-mode hook/log parity verified."
    notes: "Quota appears in TUI status surfaces, not print-mode stdout."
  - name: "plan_capped"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "No print-mode hook/log parity verified."
    notes: "G1 credits and quota panels exist, but structured cap events are not exposed."
  - name: "no_funds"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "No print-mode hook/log parity verified."
    notes: "Could appear as plain text; no stable field path."
  - name: "auth"
    detectable: true
    event_types: ["plain error text"]
    fields: []
    hook_parity: "No hook parity verified."
    notes: "Observed stdout text for auth timeout; classify by conservative text matching until structured output exists."
  - name: "permission_read_denied"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "No print-mode hook/log parity verified."
    notes: "Permission prompts are UI behavior, not structured print-mode events."
  - name: "permission_write_denied"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "No print-mode hook/log parity verified."
    notes: "Permission prompts are UI behavior, not structured print-mode events."
  - name: "tokens_consumed"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "Near miss: interactive transcript examples show thought token counts, but print mode has no field."
    notes: "Status-line context percentage is not token usage."
  - name: "model_used"
    detectable: false
    event_types: ["statusLine payload"]
    fields: ["model.display_name"]
    hook_parity: "Status-line callback can see it in TUI mode."
    notes: "Print mode does not emit the field; callers can know requested --model but not resolved backend identity."
  - name: "model_fallback"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "No hook parity verified."
    notes: "No fallback event or resolved-model field found."
  - name: "human_in_loop"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "No print-mode hook/log parity verified."
    notes: "Default permission and artifact flows can need humans, but print mode exposes no structured question event."
  - name: "session_resumable"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "No hook parity verified."
    notes: "--conversation resumes by ID, but print mode does not emit the ID."
  - name: "subagent_prompt_injection"
    detectable: false
    event_types: []
    fields: []
    hook_parity: "No hook parity verified."
    notes: "No documented way to inject non-interactive instructions into subagent prompts from CLI flags."
headless_constraints:
  - constraint: "No structured output format."
    mitigation: "Do not build a Claudine parser from print-mode prose; wait for a JSON/JSONL/protocol surface or use Antigravity SDK if it exposes a programmable stream."
    notes: "Issue #119 explicitly asks for Gemini-style stream-json parity."
  - constraint: "Default permission mode can prompt."
    mitigation: "Use --dangerously-skip-permissions only inside an external sandbox, or configure proceed-in-sandbox/always-proceed where acceptable."
    notes: "The official codelab says request-review pauses for approval."
  - constraint: "Conversation ID is not emitted by print mode."
    mitigation: "Avoid -c/--continue in parallel wrappers; use --conversation only when an ID was obtained from another trusted source."
    notes: "Issue #7 requests conversation ID emission for headless callers."
  - constraint: "Subcommands may require TTY even for --help."
    mitigation: "Avoid invoking subcommands for discovery from non-interactive wrappers unless tested with the exact version."
    notes: "Local 1.1.0 `agy models --help`, `plugin --help`, and `changelog --help` failed opening /dev/tty."
  - constraint: "Errors are plain text and may be on stdout."
    mitigation: "Classify known text errors conservatively and treat empty stdout with zero exit as ambiguous."
    notes: "Local auth timeout wrote stdout; historical Windows bug reported empty stdout/stderr with exit 0."
quirks:
  - "Antigravity CLI 1.1.0 has no `--output-format`, `--json`, or `stream-json` flag in local `agy --help`."
  - "The public GitHub repository contains README, changelog, and examples, but not CLI source types for event schemas."
  - "Status-line JSON is useful telemetry but is pushed into a user command, not emitted to the wrapper."
  - "Local subcommand help attempted to open a TTY, which makes some discovery commands unsafe in automation."
  - "Print-mode auth failure can appear as plain stdout, so stderr-only diagnostics assumptions are unsafe."
gaps:
  - "No formal JSON Schema, OpenAPI, protobuf, TypeScript type, Rust type, or Go type for print-mode output was found."
  - "No verified stdin prompt support for `agy -p` was found."
  - "No verified print-mode behavior for permission prompts, artifact review prompts, or model questions was captured with a fully authenticated account."
  - "Exact Windows and Linux config paths should be verified on those operating systems."
  - "Exact project-specific config file names under ~/.gemini/config/projects/ were not verified."
  - "CLI log file structure was not researched deeply enough to treat it as a secondary stream."
claudine_strategy:
  preferred_invocation: 'agy --print-timeout 5m --sandbox -p "<prompt>"'
  required_flags: ["-p/--print", "--print-timeout", "--sandbox or --dangerously-skip-permissions depending on external isolation"]
  conflicting_flags: ["--prompt-interactive", "interactive slash-command workflows", "subcommands that open Bubble Tea without a TTY"]
  parser_notes: "There is no structured parser target. Treat stdout as human text, classify a small set of plain-text failures, and report missing telemetry rather than pretending Antigravity has JSON events."
  wrapper_notes: "Antigravity is currently a weak fit for Claudine's non-interactive wrapper goals. Prefer providers with JSONL streams until Antigravity adds a documented structured output mode."
data_format: text
changes: []
requires_claudine_update: true
reason: "Antigravity is a new researched provider with only plain print-mode output; Claudine would need provider metadata that marks structured non-interactive telemetry unsupported and avoids JSON parser generation."
---

# Antigravity CLI Non-Interactive Sessions

## Summary

Antigravity CLI can run without opening its TUI by using print mode: `agy -p`, `agy --print`, or `agy --prompt`. That mode is automation-oriented in the narrow sense that the prompt is supplied on the command line and the interactive terminal UI does not open. It is not wrapper-grade structured output. As of local `agy 1.1.0` inspection and the public docs/changelog, there is no documented `--json`, `--output-format`, `stream-json`, SSE, JSON-RPC, or bidirectional wire mode for CLI runs.

Claudine should therefore not prefer Antigravity for structured non-interactive supervision today. The only parser target is plain text on stdout, with no session-start event, tool-call events, file-change events, usage fields, cost fields, or terminal result record. The most important caveats are that default permissions can still need a human unless preconfigured or bypassed, print mode does not emit a conversation ID, and some operational failures can appear as plain stdout rather than stderr.

## Non-Interactive Entry Points

The official codelab describes non-interactive mode as directly providing the prompt so the interactive terminal does not open, and shows `agy -p "What is the gcloud command to deploy to Cloud Run"` as the launch form. Local `agy --help` on version `1.1.0` confirms that `-p` is a short alias for `--print`, and that `--prompt` is also an alias for `--print`.

Print mode is fresh-session oriented unless the caller also supplies resume flags. `--conversation` resumes a previous conversation by ID, and `-c`/`--continue` resumes the most recent conversation. For a wrapper, `--conversation` is safer than `--continue`, but Antigravity does not document any way for a print-mode caller to capture the newly created conversation ID. GitHub issue #7 is an open request for that exact capability, which makes resume support incomplete for parallel Claudine-style automation.

The CLI accepts session-shaping flags that matter to automation:

| Flag | Use in headless runs | Wrapper risk |
| --- | --- | --- |
| `-p`, `--print`, `--prompt` | Supplies the prompt and avoids the TUI. | Plain text only. |
| `--print-timeout` | Bounds print-mode wait time; local help shows default `5m0s`. | Timeout result is not structured. |
| `--model` | Selects the model for the session. | Resolved backend model is not emitted. |
| `--mode` | Sets execution mode; help lists `accept-edits` and `plan`, while the changelog also describes the public cycle `default -> accept-edits -> plan`. | Mode is not emitted in print output. |
| `--sandbox` | Enables terminal restrictions. | Changelog records a past fix to ensure propagation into print mode, so older versions are risky. |
| `--dangerously-skip-permissions` | Auto-approves tool permission requests. | Useful for deterministic automation only inside an external sandbox. |
| `--add-dir` | Adds workspace roots. | Roots are not emitted in print output. |
| `--project`, `--new-project` | Selects or creates Antigravity project context. | Project identity is not emitted in print output. |
| `-i`, `--prompt-interactive` | Sends an initial prompt and continues interactively. | Conflicts with non-interactive operation. |

Prompt input appears to be argv-based. I found no documented `-` stdin prompt convention, JSON request body, file prompt flag, or line protocol for print mode.

## Output Formats

Antigravity exposes several user-visible output surfaces, but only one is a non-interactive CLI launch form:

| Surface | Selector | Format | Streams live? | Claudine preference |
| --- | --- | --- | --- | --- |
| Print mode | `-p`, `--print`, `--prompt` | Plain text | No structured stream | Best available, but weak. |
| Interactive TUI | `agy` | Terminal UI text | Yes, for humans | Do not parse. |
| Prompt-interactive TUI | `-i`, `--prompt-interactive` | Terminal UI text | Yes, for humans | Avoid. |
| Status line callback | `settings.json` `statusLine.command` | JSON payload to the configured command's stdin | Repeated TUI callback | Not a complete agent stream. |

Claudine should prefer print mode only if Antigravity must be supported at all, because it is the only mode intended to run without opening the TUI. That recommendation is a fallback, not an endorsement of the format. Unlike Gemini `stream-json` or Codex `exec --json`, print mode does not give Claudine incremental assistant deltas, tool start/result records, usage, or terminal status. It cannot support clean live progress rendering without scraping prose or adding a custom side channel.

The open feature request for Gemini-style `--output-format stream-json` parity is significant: it shows the gap is visible to wrapper authors and was still open in the public issue tracker. Local `agy --help` for 1.1.0 also contains no `--json` or `--output-format` flag.

The status-line callback is the one structured-looking stream in the CLI ecosystem. The official example script reads a JSON object from stdin and extracts fields such as `agent_state`, `context_window.used_percentage`, `vcs.branch`, `sandbox.enabled`, `artifact_count`, `subagents`, `task_count`, `model.display_name`, and `terminal_width`. This is useful situational telemetry for the TUI, but it is not a stdout event stream, not documented for print mode, and not sufficient to reconstruct tool calls, results, or completion.

## Schema Sources

There is no formal schema for non-interactive print output. The public GitHub repository is not the CLI source tree; it contains README, changelog, images, and examples. That means there are no Go structs, TypeScript unions, Rust Serde enums, JSON Schema files, OpenAPI documents, or protobuf definitions available for print-mode output.

The strongest evidence is therefore operational and example-based:

| Source | What it proves | Confidence |
| --- | --- | --- |
| Official Google codelab | `-p` print mode exists; `--model`, `--dangerously-skip-permissions`, settings, and permission modes exist. | High for documented user behavior. |
| Local `agy --help` on 1.1.0 | Current CLI flags include print mode and lack JSON output flags. | High for the installed version. |
| Official changelog | Versioned fixes for print mode, sandbox propagation, permissions, status line, subagents, quotas, and config paths. | High for release history. |
| Official status-line example | Informal shape of the TUI status-line JSON payload. | Medium; useful but not the CLI run stream. |
| GitHub issues #7, #76, #119 | Wrapper pain points around missing session ID, historical non-TTY stdout loss, and missing stream-json parity. | Medium; issue reports are not specs. |

Because the only output is plain text, any Claudine parser would be heuristic. The frontmatter marks schema evidence as informal examples or absent rather than pretending there is a stable contract.

## IO Contract

In print mode, stdout is final text, not parse-only JSON. A local smoke test on macOS ran:

```bash
AGY_CLI_HIDE_ACCOUNT_INFO=1 timeout 25s agy --print-timeout 20s --sandbox -p 'Reply with exactly PONG.'
```

With the local account unauthenticated or unavailable, the process produced this plain stdout text and no stderr:

```text
Error: authentication failed or timed out
```

The outer `timeout` command exited `124`, so this is not a clean provider exit-code fixture. It still proves an important IO point: Claudine cannot assume Antigravity errors are stderr-only. Historical issue #76 reported an earlier Windows bug where `agy --print` under non-TTY stdout completed with exit code `0`, zero stdout, and zero stderr; the changelog later says Windows print mode and other non-TUI command outputs were fixed in 1.0.15. Wrappers should still treat empty stdout with success exit as ambiguous unless they have version-specific confidence.

No stdin prompt contract was found. The TUI status-line and title customization commands receive JSON on their stdin, but that is a callback from Antigravity to a configured script, not stdin into `agy -p`.

## Stream Contract

There is no stream contract for print mode. There is no top-level discriminator, no nested subtype, no event ordering guarantee, no correlation ID, no terminal event, and no schema version marker. Assistant content is a final plain-text blob rather than deltas or completed message events.

The status-line JSON payload has an informal object shape, but it should not be treated as the agent event protocol. It is a snapshot for UI customization. It can show current state and counters, but it does not identify tool call IDs, tool results, file edits, command stdout/stderr, or final success/failure.

If Antigravity later adds structured output, Claudine should treat it as a new contract and require fixture capture before enabling parsing. Unknown events should then be skipped and logged, but there are no current events to apply that policy to.

## Session Metadata

Print mode emits almost no machine-readable metadata. The executable identity and version can be obtained out of band with `agy --version`, and the requested model can be known if Claudine passes `--model`, but print output itself does not carry session ID, cwd, project ID, roots, model, auth kind, permission mode, sandbox mode, MCP servers, or CLI version.

The TUI status-line payload is a near miss. Official examples show:

| Field | Meaning | Print-mode availability |
| --- | --- | --- |
| `agent_state` | State such as `idle`, `thinking`, `working`, or `tool_use`. | Not emitted. |
| `workspace.current_dir` | Current workspace directory. | Not emitted. |
| `context_window.used_percentage` | Context-window percentage for display. | Not emitted. |
| `vcs.branch`, `vcs.dirty` | Git status display. | Not emitted. |
| `sandbox.enabled` | Sandbox badge. | Not emitted. |
| `artifact_count` | Artifact count. | Not emitted. |
| `subagents` | Array used to count active subagents. | Not emitted. |
| `task_count` | Background task count. | Not emitted. |
| `model.display_name` | Human display name for the model. | Not emitted. |
| `terminal_width` | Width used by status-line layout. | Not emitted. |

These fields are useful if Antigravity ever lets a wrapper subscribe to the same state snapshots in print mode. Today they are not enough for Claudine's non-interactive stream parser.

## Event Families

Print mode exposes only two practical families: final assistant text and plain error text. Everything else is absent or only visible in the interactive UI.

| Family | Print-mode visibility | Notes |
| --- | --- | --- |
| Session start/end | Not visible | No ID or terminal event. |
| Assistant text | Final text only | No deltas or envelope. |
| Reasoning | Not structured | Interactive examples show thought summaries and token counts, but print mode has no fields. |
| Tool calls/results | Not structured | Interactive UI shows tool-like lines such as `ListDir`, `ReadURL`, and `Create`. |
| File changes | Not structured | Changes may happen, but no dedicated event or file list is emitted. |
| Plans/artifacts | Not structured | Artifacts are central in the TUI, but not a stream family in print mode. |
| Usage/cost/quota | Not structured | Quota and G1 credits are TUI/status features. |
| Permissions | Not structured | Default flow can pause for approvals. |
| Subagents/tasks | Not structured | TUI can show active subagents and background tasks. |
| Errors | Plain text | May appear on stdout. |

## Tools

Antigravity's interactive examples show native tool families for filesystem operations, URL/web access, terminal commands, artifacts, tasks, and MCP-backed tools. The CLI changelog also references terminal command permission checks, PowerShell handling, sandbox execution, MCP server startup, URL-based MCP configuration, and a maximum tool-call limit for Gemini models.

None of that is represented structurally in print mode. Claudine cannot see call start, tool input, progress, result, error, stdout, stderr, file changes, attachments, or final tool status. Permission policy affects whether tools run, but the output contract does not tell Claudine which policy decision happened.

This means Antigravity print mode is unsuitable for Claudine features that depend on tool-level lifecycle reporting, such as live command rendering, write-denial classification by path, or file-change reports.

## Completion and Exit Status

Normal completion is inferred from process exit plus stdout content. There is no `result`, `session.complete`, `turn.completed`, or equivalent terminal event. Final answer text is the stdout body.

Failures are not normalized. The local auth-timeout fixture produced plain stdout text. The historical Windows issue reported exit `0` with no output for a completed model round trip before the changelog fix. Because of these behaviors, process exit code is useful but not sufficient. Claudine should classify:

| Condition | Suggested classification |
| --- | --- |
| Non-zero exit with stdout/stderr error text | Failure, with text classification where possible. |
| Zero exit with non-empty stdout | Success only if stdout does not match known error text. |
| Zero exit with empty stdout | Ambiguous failure for wrapper purposes. |
| Timeout killed by Claudine | Timeout/cancellation controlled by Claudine, not Antigravity. |

No usage, cost, token, or quota fields are emitted at completion.

## Blocking Behavior

Antigravity's default tool permission mode is `request-review`. The codelab explains that this pauses before terminal commands, file operations, or external-service calls that have not been pre-approved. That is fundamentally interactive. The CLI offers `--dangerously-skip-permissions`, which local help says auto-approves all tool permission requests, and the codelab describes as avoiding permission prompts. The settings UI also exposes permission modes including `proceed-in-sandbox`, `always-proceed`, and `strict`.

For Claudine, the deterministic choices are:

1. Use `--sandbox` plus a permission mode that avoids prompts, if the task can run under Antigravity's sandbox.
2. Use `--dangerously-skip-permissions` only when Claudine or the caller has already supplied an external sandbox.
3. Avoid tasks likely to require artifact review, human clarification, OAuth, or setup prompts, because print mode has no programmable question/answer protocol.

Authentication can block automation. The README says the CLI authenticates through the system keyring and falls back to Google Sign-In, opening a browser locally or printing an authorization URL for SSH. In non-interactive automation, Claudine should assume missing or expired auth can fail or time out unless credentials are already established.

## Subagents

Subagents are supported by Antigravity CLI, but not in a print-mode stream. The product page and changelog describe subagents, the `/agents` panel, background tasks, and a status indicator for active subagents and background tasks. Version 1.0.14 also enabled an "always proceeds" mode for subagents to auto-approve artifacts, preventing a parent from hanging when blocked.

The parent print-mode stdout does not expose subagent start, stop, model, session IDs, nested tool calls, results, errors, or prompts. The status-line example counts `subagents`, but that is TUI telemetry. I found no CLI flag that injects non-interactive instructions into subagent prompts.

## Use Case Detection

Most Claudine normalized use cases are not detectable from Antigravity print mode:

| Use case | Detectable? | Evidence and extraction |
| --- | --- | --- |
| `plan_cap_approaching` | No | Quota displays exist, but no print event or field. |
| `plan_capped` | No | G1 credits and `/credits` exist, but no structured cap event. |
| `no_funds` | No | Could appear as prose; no stable field. |
| `auth` | Partially | Plain text such as `Error: authentication failed or timed out`; no auth kind. |
| `permission_read_denied` | No | Permission prompts are not emitted structurally. |
| `permission_write_denied` | No | Same. |
| `tokens_consumed` | No | Interactive examples show thought token counts, but print mode lacks fields. |
| `model_used` | Requested only | Claudine can know `--model`; TUI status-line has `model.display_name`, but print mode does not. |
| `model_fallback` | No | No resolved model or fallback event. |
| `human_in_loop` | No | Prompt need can manifest as hang or prose; no question event. |
| `session_resumable` | No | `--conversation` consumes IDs but print mode does not emit one. |
| `subagent_prompt_injection` | No | No flag or stream evidence found. |

The strongest near miss is the status-line payload. It can report state, model display name, context percentage, sandbox flag, artifact count, and subagent/task counts in the TUI. It does not have hook parity with print mode.

## Headless Constraints

The first constraint is the lack of structured output. This alone prevents Claudine from giving high-quality live supervision without scraping. It also prevents reliable final-state classification beyond process and text heuristics.

The second constraint is human-in-the-loop behavior. Default `request-review` pauses for approvals. Artifact review and implementation-plan approval are core Antigravity workflows, and print mode does not publish a protocol for answering those requests. `--dangerously-skip-permissions` can remove tool approval prompts, but it should only be used when an outer sandbox and policy are already in place.

The third constraint is discoverability in non-TTY contexts. Local `agy --help` worked, but `agy models --help`, `agy changelog --help`, `agy plugin --help`, and `agy install --help` all failed with a Bubble Tea `/dev/tty` error. Claudine should not rely on subcommand help or model listing as a safe non-interactive runtime step until that behavior is fixed or version-gated.

## Timeline

| Version/date | Non-interactive relevance |
| --- | --- |
| 1.0.0 | Initial public Antigravity CLI release. |
| 1.0.5 | Added `--model`, `models`, `/permissions`; integrated permission merging across project, user, and CLI settings; fixed `-p` metadata location. |
| 1.0.6 | Fixed `--sandbox` propagation in headless print mode. |
| 1.0.8 | Added quota usage and execution mode in the status line. |
| 1.0.9 | Fixed headless print-mode resumption with `--conversation`/`-c` plus `-p`. |
| 1.0.14 | Enabled subagent "always proceeds" mode to avoid artifact-approval hangs. |
| 1.0.15 | Fixed Windows print mode and other non-TUI outputs being discarded in non-TTY environments. |
| 1.1.0 | Made agent execution mode cycling public and added `request-review` as the default execution behavior with interactive diff review. |

## Quirks and Gaps

The main parser footgun is that Antigravity looks agentic and telemetry-rich in the TUI, but the non-interactive surface discards nearly all of that structure. The status line proves the CLI has internal structured state, yet it is exposed only as a customization callback.

Another footgun is stdout classification. Plain errors can appear on stdout, so a wrapper that treats stdout as final answer text will misclassify auth failures. Conversely, a wrapper that expects stderr diagnostics may miss the error entirely.

The biggest gaps are the absence of source-level schema evidence, unverified Linux/Windows config paths, and unverified behavior for authenticated print-mode runs that encounter permission prompts, artifact review, model questions, quota caps, or MCP OAuth.

## Claudine Integration Notes

Recommended fallback command shape:

```bash
agy --print-timeout 5m --sandbox -p "$PROMPT"
```

For trusted, externally sandboxed automation that must make edits, Claudine could add `--dangerously-skip-permissions`, but that should be an explicit policy decision. Do not use `--continue` for parallel sessions. Do not assume `--conversation` can create or reveal a conversation ID. Do not run TUI subcommands for runtime discovery unless the exact version has been tested in a non-TTY subprocess.

Claudine should parse stdout as text, not events. It should maintain a small set of text classifiers for known failures such as authentication timeout, and it should report missing telemetry explicitly: no live tool events, no file-change list, no token usage, no cost, no session ID, and no structured terminal status. If Antigravity adds `stream-json` or another structured mode, that should be researched as a new parser contract with captured fixtures.

## Sources

- [Hands-on with Antigravity CLI](https://codelabs.developers.google.com/antigravity-cli-hands-on) documents settings, permission modes, non-interactive `-p`, `--model`, and `--dangerously-skip-permissions`.
- [Antigravity CLI repository](https://github.com/google-antigravity/antigravity-cli) provides the README, changelog, and examples but not CLI source code.
- [Antigravity CLI changelog](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md) provides versioned behavior around print mode, sandbox propagation, permissions, status line, subagents, quota, and config paths.
- [Status-line example script](https://github.com/google-antigravity/antigravity-cli/blob/main/examples/statusline/statusline.sh) shows the informal TUI status-line JSON payload.
- [Title example script](https://github.com/google-antigravity/antigravity-cli/blob/main/examples/title/title.sh) shows `agent_state` and `workspace.current_dir` in another customization payload.
- [Issue #119](https://github.com/google-antigravity/antigravity-cli/issues/119) requests Gemini-style `--output-format stream-json` parity.
- [Issue #7](https://github.com/google-antigravity/antigravity-cli/issues/7) requests print-mode conversation ID emission for headless callers.
- [Issue #76](https://github.com/google-antigravity/antigravity-cli/issues/76) records the historical Windows non-TTY print-output loss that the changelog later says was fixed.
