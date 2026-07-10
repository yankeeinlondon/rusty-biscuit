---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-09
agent: codex
model: default
docs: https://antigravity.google/docs/cli-statusline
records:
  - id: exit-auth_invalid-models-signin
    signal: auth_invalid
    source: exit
    locator: "exit_code=1"
    detection: declarative
    priority: 10
    match_path: stdout_tail
    match_op: substring_ci
    match_value: "Please sign in to view available models"
    distinguish: "The `models` subcommand fails before listing models when no Antigravity credentials are available. This is an authentication preflight, not a model-catalog failure."
    vocabulary: ["Please sign in to view available models"]
    since: "1.0.5"
    confidence: observed
    evidence: ./fixtures/antigravity/exit-auth-invalid-models.json
    notes: "Reconfirmed against installed `agy` 1.1.0 on 2026-07-08."
  - id: exit-auth_invalid-print-timeout
    signal: auth_invalid
    source: exit
    locator: "exit_code=1"
    detection: declarative
    priority: 20
    match_path: stdout_tail
    match_op: substring_ci
    match_value: "authentication failed or timed out"
    distinguish: "Headless print mode can initiate OAuth and then fail after the authentication wait expires. This is auth state, not model timeout or generation timeout, because no generation starts."
    vocabulary: ["authentication failed or timed out"]
    confidence: observed
    evidence: ./fixtures/antigravity/exit-auth-invalid-print.json
    notes: "Captured from installed `agy` 1.1.0 on 2026-07-08 with `--print-timeout 15s`; the OAuth wait in the app log was 30s."
extractions:
  - record: exit-auth_invalid-models-signin
    field: message
    path: stdout_tail
  - record: exit-auth_invalid-print-timeout
    field: message
    path: stdout_tail
gaps:
  - "Antigravity app logs (`--log-file`) carry a glog-style `Language server version:` line (`provider_version`), documented and backed by a scrubbed fixture at `./fixtures/antigravity/app-log-auth-version.txt`. Claudine has no runtime app-log ingestion path for Antigravity yet, so this is not compiled as a detection record. Adding it requires a bespoke glog-line classifier fed from the wrapper reading `--log-file`."
  - "Antigravity app logs (`--log-file`) carry clustered `You are not logged into Antigravity` token-source failures (`auth_invalid`), documented and backed by the same scrubbed fixture at `./fixtures/antigravity/app-log-auth-version.txt`. Claudine has no runtime app-log ingestion path for Antigravity yet, so this is not compiled as a detection record. Adding it requires a bespoke glog-line classifier that deduplicates the repeated cache/poll failures within one startup window."
  - "The public `google-antigravity/antigravity-cli` repository at tag `1.1.0` (`ee2382093ac06d9d68fc88e822713357c2401a78`) contains README, changelog, examples, and media, but no implementation source. Source-code-first inspection therefore found no event enums, stream schemas, error enums, or log-entry types to cite."
  - "The official statusline documentation URL is the closest signal/event documentation found, but the public page is rendered as a client-side app and the repository example gives only consumer code, not a verbatim payload fixture. No official machine-readable stream/event contract for `agy` was found."
  - "The glog-style app-log timestamp (`I0708 11:17:34.547650`) carries month/day and local wall-clock time but no year or timezone. No timestamp extraction is recorded; any future timestamp payload should use `zone: unspecified` unless source confirms otherwise."
  - "Print mode app logs show OAuth prompts and a 30-second authentication wait, but the only structured fixture-backed record emitted here is the final exit auth failure. A future classifier could add `human_input_requested` if Claudine intentionally promotes OAuth prompt lines from app logs."
  - "Changelog entries document Models & Quota UI/statusline quota display, `/usage` and `/quota`, G1 credits, automatic client-side retries, transient errors, Ctrl+C interruption, SQLite conversations, `/resume`, permission prompts, and background task logs, but no verbatim event/log payloads for those surfaces were obtainable from source or official docs."
  - "The statusline example documents fields such as `context_window.used_percentage`, `model.display_name`, `agent_state`, `subagents`, and `task_count`, but the repository provides the consumer script rather than verbatim statusline JSON payload bytes. No `tokens_consumed`, `usage_cap_approaching`, or `model_resolved` record is emitted from that surface until a captured or documented payload fixture exists."
  - "No fixture-backed records were found for `usage_cap_approaching`, `usage_capped`, `rate_limited`, `provider_overloaded`, `retries_exhausted`, `no_funds`, `permission_denied_read`, `permission_denied_write`, `model_fallback`, `generation_retried`, `interrupted`, `session_resumable`, or `human_input_requested`."
changes:
  - "Refreshed Antigravity research against tag `1.1.0` and installed `agy` 1.1.0 on 2026-07-08."
  - "Confirmed the public repository still lacks implementation source, so no `source_code` confidence signal records can be produced."
  - "Added a separate fixture-backed `exit-auth_invalid-print-timeout` record for unauthenticated headless print mode."
  - "Updated version-drift notes: launcher and language-server version now both report 1.1.0 in the local capture."
  - "Moved the two app-log observations (`provider_version`, `auth_invalid`) from detection records to `gaps`; there is no runtime app-log ingestion path yet. Removed the retired `documentation` detection mode."
requires_claudine_update: true
reason: "Antigravity's generated signal tables now carry the two fixture-backed `exit` auth records; the app-log `provider_version` / `auth_invalid` observations remain future work in `gaps` pending a bespoke app-log classifier fed from the wrapper reading `--log-file`."
---

# Antigravity CLI Signal Detection

## Overview

Antigravity CLI currently exposes Claudine-relevant signals through ordinary command output, a configurable plaintext app log (`--log-file`), interactive TUI/statusline state, SQLite-backed conversation history, and UI panels such as Models & Quota, `/usage`, `/quota`, `/resume`, `/permissions`, and `/tasks`. The only fixture-backed machine surfaces verified in this pass are exit/stdout payloads from unauthenticated non-interactive commands and glog-style app-log lines captured from the same installed `agy` 1.1.0 binary.

The public `google-antigravity/antigravity-cli` repository is not source-complete as of tag `1.1.0` (`ee2382093ac06d9d68fc88e822713357c2401a78`); the repository listing contains README, changelog, examples, and media, but no implementation files. That makes this provider materially different from source-available providers such as Gemini CLI or Qwen Code: enum vocabularies, stream envelopes, and error classifiers could not be extracted from source code. Records below are therefore `observed` where they come from local `agy` bytes and `documented` only in body prose where the changelog, README, or example files establish a surface without payload bytes.

## Signal Surfaces

### Exit Output

`agy models` is a one-shot subcommand that lists available models when authenticated. In an unsigned-in environment it exits with code `1` and writes a plain error line to stdout:

```text
Error: Please sign in to view available models. Launch the CLI without arguments to sign in.
```

Headless print mode (`agy --print`) has a separate unauthenticated failure shape. The app log shows it starts silent auth, triggers OAuth, waits for authentication, and times out; stdout only carries the final plain error:

```text
Error: authentication failed or timed out
```

Claudine should treat wrapper-synthesized `{exit_code, stdout_tail, stderr_tail}` payloads as the structured source for this surface. The captured fixtures `./fixtures/antigravity/exit-auth-invalid-models.json` and `./fixtures/antigravity/exit-auth-invalid-print.json` record the two exit code and stdout-tail shapes.

### App Logs

The CLI accepts `--log-file` and writes glog-style plaintext lines to the chosen file. A startup run of `agy models` without credentials produced boot metadata and repeated auth/cache diagnostics:

```text
I0708 11:17:34.547650 12345 server.go:1380] Language server version: 1.1.0
E0708 11:17:34.648690 12345 log.go:398] Failed to poll FetchAvailableModels: failed to get load code assist response: error getting token source: You are not logged into Antigravity.
I0708 11:17:34.656522 12345 quota_manager.go:63] quotaRefreshLoop: skipped (not logged in)
```

This is a diagnostic side-channel rather than a stable JSON contract. It is still operationally useful because it carries provider-version and auth-state signals even when stdout contains only a user-facing error. Those two app-log observations (`provider_version`, `auth_invalid`) are documented here and a scrubbed fixture is retained, but they are NOT emitted as detection records: Claudine has no runtime app-log ingestion path for Antigravity yet, so they live in `gaps` as future work pending a bespoke glog-line classifier.

### Statusline Payload

The repository ships `examples/statusline/statusline.sh`, which reads a JSON payload from stdin and extracts `agent_state`, `context_window.used_percentage`, `vcs.branch`, `vcs.dirty`, `sandbox.enabled`, `artifact_count`, `subagents`, `task_count`, `model.display_name`, and `terminal_width`. The accompanying README links `https://antigravity.google/docs/cli-statusline` as the official public documentation for the statusline surface. This establishes a structured statusline hook payload that can carry context usage and model display information, but the example is a consumer script rather than a verbatim payload fixture. No frontmatter records are emitted for `tokens_consumed`, `usage_cap_approaching`, or `model_resolved` from this surface.

### SQLite Conversation Stores

The changelog states that SQLite (`.db`) conversation support was added in 1.0.4 and that `/resume` later gained SQLite scanning and persistent metadata caching. A 1.1.0 unauthenticated startup log also reports creation of a store manager with a proto store and SQLite store. The schema, tables, and row payloads are not documented in the public repo, and no local Antigravity conversation database was present in the workspace inspected during this pass. Treat SQLite as a likely future source for `session_resumable`, token history, and model changes, but not yet recordable.

### TUI Panels and Slash Commands

The changelog documents Models & Quota, `/usage`, `/quota`, `/credits`, `/permissions`, `/resume`, `/tasks`, `/hooks`, `/settings`, and `/help` surfaces. These are user-facing TUI surfaces rather than exposed event contracts in the available evidence. They are useful for manual diagnosis but should not become Claudine records until a structured export, log line, or replayable fixture is captured.

## Usage and Rate Limits

No fixture-backed usage-limit or rate-limit error envelope was found. The changelog documents quota surfaces: 1.0.8 redesigned the "Models & Quota" page and added quota usage to the status line; 1.0.1 improved `/usage` and `/quota` by forcing a real-time reload; 1.0.3 added G1 credits and a `/credits` panel.

The local unauthenticated capture did include `quotaRefreshLoop: skipped (not logged in)`, but this is an auth-state diagnostic rather than `usage_cap_approaching`, `usage_capped`, `rate_limited`, `no_funds`, or `tokens_consumed`. The statusline example's `context_window.used_percentage` is context-window occupancy, not billable token consumption, and the unit is percent.

## Authentication and Authorization

The README states that Antigravity CLI authenticates through the system keyring and falls back to Google Sign-In when no active session exists. In the local capture, `agy models` failed with a stdout error asking the user to sign in. The app log from the same run emitted repeated token-source failures with the message `You are not logged into Antigravity`.

The `models` exit/stdout shape is declarative: match `stdout_tail` for `Please sign in to view available models`. The `--print` exit/stdout shape is also declarative: match `stdout_tail` for `authentication failed or timed out`. The app-log auth shape is documented but NOT emitted as a detection record — it lives in `gaps` — because there is no runtime app-log ingestion path yet; a future classifier would parse glog metadata, classify the message cluster once, and attach the surrounding source site (`FetchAvailableModels`, `availableModels`, `userInfo`, `quotaRefreshLoop`, or `printmode.go`) as diagnostic context.

Permission behavior is documented but not recordable from this pass. The changelog documents `request-review` mode, `/permissions`, project/user/CLI permission merging, sandbox auto-approval, and fixes for read/write workspace checks. No stable payload was found that distinguishes `permission_denied_read` from `permission_denied_write`.

## Model Resolution

`agy --help` documents `--model`, and the changelog says `--model` plus the `models` subcommand were added in 1.0.5. The statusline example reads `.model.display_name` from the statusline JSON payload. No verbatim statusline payload or stream event was available, and the unauthenticated `models` run never reached model listing.

The model-related classification risk from this pass is negative: the `models` failure is `auth_invalid`, not `model_resolved` or model-catalog drift, because the failure occurs before model data is available.

## Provider Version

`agy --version` printed `1.1.0`, and the configured app log for `agy models` reported `Language server version: 1.1.0`. The public `1.1.0` tag is `ee2382093ac06d9d68fc88e822713357c2401a78`.

The app-log language-server line is documented as a `provider_version` observation but is NOT emitted as a detection record — it lives in `gaps` — because it requires plaintext log parsing and Claudine has no runtime app-log ingestion path yet. A future Antigravity adapter should still decide whether to expose the launcher version, embedded language-server version, or both as separate metadata fields; earlier local research observed a launcher/server mismatch, while this refresh did not.

## Token Metering

No billable token count payload was found. The statusline example exposes `context_window.used_percentage`, which is a percentage occupancy measure. That could be useful as a future `usage_cap_approaching` advisory if Antigravity documents threshold semantics, but it is not a `tokens_consumed` payload by itself.

## Retries and Transient Failures

The 1.0.16 changelog documents automatic client-side retries for transient model generation errors. The public repo does not contain retry enums or log payload examples, and the local capture did not trigger retry behavior. No `generation_retried`, `retries_exhausted`, `provider_overloaded`, or `repeated_stream_error` record is emitted.

## Interruption and Recovery

The 1.0.11 changelog documents `ctrl+c` behavior: first press cancels active agent operations such as streaming responses, while double press triggers exit. The 1.0.6 changelog fixed a bug where typing after `Esc` interrupt could be swallowed. These are native interruption behaviors, but the available evidence is changelog prose rather than a capture of the resulting exit code, log line, or session record.

Antigravity has persistent history and `/resume`; the README also says terminal sessions can be exported to Antigravity 2.0 GUI. The changelog documents SQLite conversation support and `/resume` metadata caching. This is strong evidence that sessions can be resumed, but no fixture-backed event or database row was available for a `session_resumable` record.

## Human Input Requests

Unauthenticated print mode surfaces an OAuth flow in app logs, including an authentication URL, a prompt to paste an authorization code, and an authentication wait. That is a native human-input request, but this pass does not promote it into frontmatter because the captured structured surface is only the final exit failure and the app-log prompt contains one-time OAuth details that require careful scrubbing. A future Antigravity app-log classifier could emit `human_input_requested` from scrubbed OAuth prompt lines if Claudine decides to observe that reserved signal.

## Version Drift

The current latest public release inspected here is 1.1.0 on 2026-07-08. The installed local `agy --version` output and app-log language-server version both reported 1.1.0. Prior research observed a local launcher/server mismatch (`agy --version` 1.0.6 with language server 1.1.0), so wrappers should keep launcher and embedded-server version fields conceptually separate even though they matched in this refresh.

Documented surface drift:

| Version | Drift |
| --- | --- |
| 1.0.1 | `/usage` and `/quota` gained real-time reload; OAuth persistence/authentication hangs fixed; Windows log redirection fixed. |
| 1.0.3 | G1 credits and `/credits` panel added. |
| 1.0.4 | SQLite conversation support added and named as the CLI conversation format. |
| 1.0.5 | `--model`, `models`, `/permissions`, and SQLite scanning for `/resume` added. |
| 1.0.8 | Models & Quota page redesigned; quota usage and execution mode added to the status line. |
| 1.0.11 | `ctrl+c` interruption and exit behavior added. |
| 1.0.16 | Automatic client-side retries for transient generation errors added. |
| 1.1.0 | `request-review` write-review mode became the default execution behavior. |

## Quirks and Gaps

The provider appears to have rich internal state, but most of it is currently exposed through TUI panels, diagnostics, or docs/changelog prose rather than stable event schemas. Claudine should avoid broad substring classifiers for quota/rate/funds until real payloads are captured, because auth, quota, available-models, and cache refresh failures can co-occur in the same startup window.

The public repository should not be treated as an OSS source tree for signal semantics today. It is useful for documentation, changelog, and examples, but not for source-code-first enum extraction.

Statusline paths are promising but under-evidenced: `context_window.used_percentage` has a clear percent unit and `model.display_name` has an obvious model display meaning, but the consumer script is not a payload fixture. The correct next evidence step is a scrubbed live statusline payload capture or an official docs payload example.

## Changelog

- 2026-07-08: Refreshed against public tag `1.1.0` and installed `agy` 1.1.0. Confirmed the public repository still contains no implementation source. Added the unauthenticated headless print-mode auth failure record and fixture. Updated notes that launcher and language-server versions now both reported 1.1.0 in local capture.
- 2026-07-08: Initial Antigravity signal research document created.

## Sources

- [Antigravity CLI README at `1.1.0`](https://github.com/google-antigravity/antigravity-cli/blob/1.1.0/README.md#L1-L85)
- [Antigravity CLI changelog at `1.1.0`](https://github.com/google-antigravity/antigravity-cli/blob/1.1.0/CHANGELOG.md#L1-L208)
- [Statusline example README at `1.1.0`](https://github.com/google-antigravity/antigravity-cli/blob/1.1.0/examples/statusline/README.md#L1-L25)
- [Statusline example script at `1.1.0`](https://github.com/google-antigravity/antigravity-cli/blob/1.1.0/examples/statusline/statusline.sh#L32-L58)
- [Antigravity CLI overview](https://antigravity.google/docs/cli/overview)
- [Antigravity CLI statusline documentation](https://antigravity.google/docs/cli-statusline)
- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [GitHub release 1.1.0](https://github.com/google-antigravity/antigravity-cli/releases/tag/1.1.0)
