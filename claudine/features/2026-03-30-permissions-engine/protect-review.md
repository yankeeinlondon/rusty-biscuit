# Protect Review

Review target: the refactored Protect service against `protect-final-design.md`.

Checks run:

- `cargo test -p claudine protect -- --nocapture` (passes)

## Findings

### P1: MCP redaction plans are generated but not reliably enforced

- Dispatch applies post-action redaction first and skips protect short-circuiting when a redaction plan exists (`claudine/lib/src/dispatch/mod.rs:247`).
- `apply_redaction()` only mutates `HookResponse.updated_input` / `additional_context` or clears them for `BlockPayload`, but it never sets a deny/ask decision (`claudine/lib/src/dispatch/mod.rs:328`).
- Those fields exist on `HookResponse` (`claudine/lib/src/actions/hook_response.rs:7`), but the adapter formatters do not consume them; they format decisions/reasons only.
- Result: `ReplaceText` / `ReplaceJson` redactions are likely dropped during provider formatting, and `BlockPayload` does not actually block the payload.

Suggested fix:

- Make post-action redaction part of provider formatting, or convert `ProtectRedactionPlan` into provider-native raw responses before `format_response()`.
- For `BlockPayload`, force an enforceable deny/stop response instead of only clearing `HookResponse` fields.
- Add dispatch integration tests that assert `ReplaceJson`, `ReplaceText`, and `BlockPayload` survive all the way to formatted provider output.

### P1: Protect never supplies trust context to PolicyEngine

- `PolicyContext` explicitly carries trust state and provider backends use it to decide whether repo-scoped policy may load (`claudine/lib/src/permissions/context.rs:11`).
- `build_session_context()` only sets `cwd`, `repo_root`, `home_dir`, wrapper flags, and session ID; it never calls `with_trust(...)` (`claudine/lib/src/dispatch/mod.rs:279`).
- Codex and Gemini both gate repo config on `ctx.trust.is_trusted == Some(true)` and emit warnings when trust is unknown (`claudine/lib/src/permissions/providers/codex.rs:102`, `claudine/lib/src/permissions/providers/codex.rs:545`, `claudine/lib/src/permissions/providers/gemini.rs:148`, `claudine/lib/src/permissions/providers/gemini.rs:512`).

Impact:

- Any provider with trust-gated repo policy will be evaluated as `Unknown` / warning-prone even when wrapper/runtime state could have supplied trust.
- This directly undermines the final design’s “effective vs configured” behavior.

Suggested fix:

- Extend session-context construction to derive trust from `EventMeta.extra` / wrapper state and pass it via `PolicyContext::with_trust(...)`.
- Add integration tests for trusted vs untrusted repo policy resolution through Protect dispatch.

### P1: Provider-aware observation is only implemented for Claude

- The trait default is still `default_observe_protect(...)` (`claudine/lib/src/adapters/mod.rs:91`).
- The only adapter overriding `observe_protect()` is Claude (`claudine/lib/src/adapters/claude.rs:84`).
- The default observer extracts only a narrow subset of intents: `ExecuteCommand`, `ReadPath` / `WritePath`, `UseMcpServer`, `SpawnSubagent`, and `CompletionOutputScan` (`claudine/lib/src/services/protect/observe.rs:37`).

Impact:

- Most of the final design’s intent surface is effectively unreachable in non-Claude providers: `UseMcpTool`, `AccessDomain`, `TraversePath`, `SwitchMode`, and `ModifyProviderConfig`.
- Protect can only ask PolicyEngine the right question if the adapter derives the right intent. Right now that is mostly not happening.

Suggested fix:

- Add provider fixtures and `observe_protect()` overrides for at least Codex, Gemini, OpenCode, Qwen, and Roo.
- Make `UseMcpTool` / `AccessDomain` extraction first-class for MCP/tool-call payloads across providers.

### P2: Completion scanning is incomplete and several completion config knobs are dead

- `CompletionPolicy` still exposes `check_commands` and `secret_scan` (`claudine/lib/src/services/protect/config.rs:514`).
- `completion_scan_result()` only checks `contains_instruction_payload(text)` and otherwise returns allow (`claudine/lib/src/services/protect/evaluate.rs:608`).
- It does not consult `completion.secret_scan`, `completion.check_commands`, or Protect secret patterns.

Impact:

- The implemented behavior is instruction-injection detection, not the “secret scan and loop detection” described in the design.
- Existing knobs suggest functionality that is not actually wired.

Suggested fix:

- Either remove the unused config knobs or implement them fully.
- Reuse the same secret-pattern scanning logic used by MCP redaction so completion scans can catch real secrets, not just prompt-injection phrases.

### P2: Runtime guard coverage is still minimal

- `apply_runtime_guards()` currently implements only “root without sandbox” (`claudine/lib/src/services/protect/evaluate.rs:293`).
- The final design called out additional runtime-only guards such as unknown-write/unknown-command escalation and provider-config mutation escalation.
- Existing config still exposes privilege knobs like `require_ask_for_network_writes` and `require_ask_for_broad_fs_writes` (`claudine/lib/src/services/protect/config.rs:640`), but they are not referenced by evaluation.

Impact:

- The runtime-guard stage is much thinner than designed.
- Some configuration is currently misleading because it looks supported but has no effect on decisions.

Suggested fix:

- Implement the missing runtime-only guardrails or remove the dead knobs.
- Add focused tests for each guard so the runtime-vs-policy boundary stays explicit.

### P2: ProtectConfig still duplicates permission truth instead of deferring to PolicyEngine

- `ProtectConfig` still owns `rules`, `mcp.allowlist` / `denylist`, and `subagents` (`claudine/lib/src/services/protect/config.rs:76`, `claudine/lib/src/services/protect/config.rs:486`, `claudine/lib/src/services/protect/config.rs:557`, `claudine/lib/src/services/protect/config.rs:596`).
- Validation and merge logic still treat those as active config rather than deprecated fields (`claudine/lib/src/services/protect/config.rs:143`).
- The integration test fixture in `claudine/cli/tests/protect_cli.rs:51` still configures the old `rules` shape.

Impact:

- The migration described in the final design did not land.
- Protect still carries a second permission-ish config surface, which increases drift risk and makes the ownership boundary with PolicyEngine less clear.

Suggested fix:

- Replace these with runtime-only knobs and targeted validation errors for removed fields.
- Keep `secret_patterns` only if it is explicitly part of a runtime redaction policy, not a general permission-rules bucket.

## Testing Gaps

- There is no integration test proving that post-action `ProtectRedactionPlan` survives adapter formatting and changes the actual provider response.
- There are no Protect dispatch tests for trusted vs unknown-trust repo policy resolution.
- There are no adapter observation tests beyond Claude-style behavior. I did not find provider-specific `observe_protect()` fixture coverage for Codex, Gemini, OpenCode, Goose, Kimi, Qwen, or Roo.
- CLI-level coverage is shallow: `claudine/cli/tests/protect_cli.rs` only checks that quick init writes defaults and that `handle --json` includes a `protect_pre` field.
- I did not find coverage for `AGENT_PARAMS` affecting `ProtectPolicyMode::Effective` through the full dispatch path.
- I did not find coverage for completion scan behavior tied to `CompletionPolicy.secret_scan` / `check_commands`.

## Ergonomics And Performance

### Add snapshot caching

- The final design called for caching snapshots by provider/cwd/repo_root/trust/CLI fingerprint.
- `ProtectService` currently stores only config, profiles, state, and `last_evaluation`; there is no snapshot cache (`claudine/lib/src/services/protect/service.rs:28`).
- `evaluate_structured()` resolves policy every time (`claudine/lib/src/services/protect/service.rs:87`), so pre-action and post-action evaluation in the same dispatch path repeat the same work.

Suggested improvement:

- Cache configured/effective snapshots inside `ProtectService` using the design key.
- Invalidate only when the session fingerprint changes.

### Remove Protect’s internal provider capability registry

- The final design said adapter capability metadata should remain, but `ProviderProtectProfiles` should no longer be a service-internal registry.
- `ProtectService` still owns `profiles` and dispatch still creates a profile set just to insert the active adapter’s capability (`claudine/lib/src/services/protect/service.rs:28`, `claudine/lib/src/dispatch/mod.rs:155`).

Suggested improvement:

- Pass `ProviderProtectCapabilities` directly into evaluation/downgrade instead of maintaining a parallel map.
- This will simplify service construction and reduce drift between adapters and Protect defaults.

### Tighten the observation/evaluation seam

- The observation layer is now the critical correctness boundary.
- Right now it is half generic, half provider-specific, and its coverage is uneven.

Suggested improvement:

- Treat `observe_protect()` as a first-class contract with per-provider fixtures.
- Consider making observation output more explicit in test helpers so new providers cannot silently fall back to weak generic extraction.
