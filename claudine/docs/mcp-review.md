# MCP Review

Reviewed against:

- `claudine/prompts/archive/mcp-support.md`
- `claudine/plans/2026-03-08.mcp-support-plan.md`

## Findings

1. `P0` Sync/export ownership is not implemented strongly enough to satisfy the design's coexistence rules.
   Evidence:
   `claudine/lib/src/mcp/state.rs:67` defines `record_managed`, but it is never called outside state tests.
   `claudine/lib/src/mcp/export.rs:61-66` only consults managed entries for removals, so stale managed entries are never tracked in real usage.
   `claudine/lib/src/mcp/export.rs:69` and the provider writers in `claudine/lib/src/mcp/export.rs:215-242`, `claudine/lib/src/mcp/export.rs:274-299`, `claudine/lib/src/mcp/export.rs:340-363`, and `claudine/lib/src/mcp/export.rs:393-419` always write `server.id` as the native name.
   Impact:
   The same-fingerprint/different-native-name merge path cannot round-trip safely. A provider-native `calendar` entry deduped into catalog ID `google-calendar` will be re-exported as a second entry instead of updating the original native name, and sync cannot reliably remove only Claudine-managed entries later.
   Recommendation:
   Record managed ownership on successful `mcp sync --apply`, use provider-state `native_name` as the write/remove key for previously known entries, and remove/update provider-state when entries are deleted or renamed.

2. `P0` `claudine gemini --mcp` is not actually session-scoped unless some other path already created a shadow home.
   Evidence:
   `claudine/cli/src/commands/wrap/mod.rs:212` passes `env_plan.shadow_home_path` into the injector.
   `claudine/cli/src/commands/wrap/env.rs:90-108` only creates a shadow home for `--repo` or Codex prompt overlay cases.
   `claudine/lib/src/mcp/inject.rs:215-246` falls back to `Path::new(".")` and writes `./.gemini/settings.json` when no shadow home is present.
   Impact:
   `claudine gemini --mcp` can write a file in the current working directory that Gemini will not use, while the child process still runs against the original `HOME`.
   Recommendation:
   Make Gemini runtime MCP imply a shadow-home overlay, and mirror the sidecar state Gemini expects (`mcp-server-enablement.json`, `mcp-oauth-tokens.json`) into that shadow home when needed.

3. `P1` Launch-time `#tag` activation was implemented in the library but never wired into the wrapper.
   Evidence:
   `claudine/lib/src/mcp/session.rs:123-172` implements `extract_tags`.
   `claudine/cli/src/commands/wrap/mod.rs:203` calls `compute_session_set(..., &[])`, so no prompt tags ever reach session composition.
   `claudine/cli/src/output.rs:101-165` also has no MCP-specific dry-run output even though the plan called for it.
   Impact:
   One of the design's explicit user-facing activation paths is effectively missing, and dry-run cannot show what would be injected.
   Recommendation:
   Parse prompt-bearing passthrough args in non-interactive wrapper mode, feed extracted tags into `compute_session_set`, forward `cleaned_prompt`, and include the resolved MCP set in dry-run and summary output.

4. `P1` The Codex and Gemini runtime injectors replace config files instead of overlaying only the MCP-related section.
   Evidence:
   `claudine/lib/src/mcp/inject.rs:140-175` creates a brand-new Codex TOML document containing only `[mcp_servers]`.
   `claudine/lib/src/mcp/inject.rs:219-246` writes a Gemini JSON document containing only `mcpServers`.
   Impact:
   Even when the shadow-home path is correct, the wrapped session loses unrelated provider settings that were supposed to remain intact for that temporary run.
   Recommendation:
   Start from the existing shadow-home config and mutate only the MCP section plus any explicitly required allow-list flags.

5. `P1` Repo-scoped MCP commands use `current_dir()` directly instead of resolving the repository root.
   Evidence:
   `claudine/cli/src/commands/mcp.rs:168-170`, `claudine/cli/src/commands/mcp.rs:273-276`, and `claudine/cli/src/commands/mcp.rs:320-335`.
   Impact:
   Running `claudine mcp init`, `claudine mcp default --repo`, or `claudine mcp sync --scope repo` from a nested package directory can miss the actual repo config files or write nested `.claudine/mcp.json` files in the wrong place.
   Recommendation:
   Resolve repo scope through the existing repo-root helper once and reuse it across import/default/sync.

6. `P1` The shipped command surface is still short of the design and plan.
   Evidence:
   `claudine/cli/src/commands/mcp.rs:116-160` lists only ID, transport, command/url, and aliases, but not defaults or provider presence.
   `claudine/cli/src/commands/mcp.rs:225-230` emits raw server JSON, including env/header secrets, and does not include provider-state provenance.
   `claudine/cli/src/commands/mcp.rs:304-309` removes catalog entries without the confirmation step called for in the plan.
   `claudine/cli/src/commands/mcp.rs:337-340` silently drops unresolved defaults during sync.
   `claudine/lib/src/mcp/export.rs:68-111` only populates `removed` and `preserved` during dry-run.
   Recommendation:
   Finish the planned command behavior: defaults/provider-presence in `mcp` list, provenance in `mcp show`, redaction in all default output modes, confirmation on remove, explicit unresolved-ID reporting, and complete sync reports for both dry-run and apply.

7. `P1` Normalization and round-tripping are still lossy for provider-native fields the design expected Claudine to preserve or park in `provider_overrides`.
   Evidence:
   OpenCode research documents local commands as arrays in `claudine/docs/mcp/opencode.md:48-60`, but `claudine/lib/src/mcp/import.rs:581-592` reads `command` as a string and discards args, and `claudine/lib/src/mcp/export.rs:343-347` writes a string back out.
   Codex research documents `env_vars`, `bearer_token_env_var`, `env_http_headers`, `enabled`, `required`, `startup_timeout_sec`, `tool_timeout_sec`, `enabled_tools`, and `disabled_tools` in `claudine/docs/mcp/codex.md:74-104`, but the parser/exporter only handle a small subset in `claudine/lib/src/mcp/import.rs:429-495` and `claudine/lib/src/mcp/export.rs:273-299`.
   Claude research calls out local/plugin/managed scope precedence in `claudine/docs/mcp/claude.md:25-40`, but import discovery only models user and repo in `claudine/lib/src/mcp/import.rs:250-264`.
   Gemini research calls out enablement and OAuth sidecars in `claudine/docs/mcp/gemini.md:26-31`, but runtime/session code does not model them.
   Recommendation:
   Move the provider edges to typed per-provider config structs, preserve unsupported-but-known fields in `provider_overrides`, and add round-trip tests for every provider-specific field Claudine claims to normalize.

8. `P2` Test coverage is strong for library primitives but still thin for real workflows.
   Evidence:
   There are no dedicated CLI tests for `claudine mcp` handlers in `claudine/cli/src/commands/mcp.rs:115-360`, and no wrapper MCP integration tests for `claudine <provider> --mcp` in `claudine/cli/src/commands/wrap/mod.rs:194-238`.
   Import coverage is parser-heavy, but discovery/orchestration/conflict paths are not covered end-to-end in `claudine/lib/src/mcp/import.rs:88-239`.
   Export coverage only exercises low-level writers, not `sync_provider` or CLI sync behavior in `claudine/lib/src/mcp/export.rs:51-111` and `claudine/cli/src/commands/mcp.rs:313-360`.
   Recommendation:
   Add the CLI and integration cases called out in plan Phase 11:
   `claudine mcp` list/init/show/default/alias/remove/sync
   `--json` branches
   wrapper `--mcp`/`--use` dry runs for Codex, Gemini, and OpenCode
   unsupported-provider guidance
   repo-root resolution from nested directories
   same-fingerprint/different-native-name sync round-trips
   backup creation and managed-entry removal behavior

9. `P2` Documentation drift is real, and the review target asked for it explicitly.
   Evidence:
   `claudine/README.md` still omits the `claudine mcp` command family and still describes configuration as only `~/.claudine/config.json` or `<repo>/.claudine/config.json`.
   `claudine/lib/README.md` still describes the library as nine top-level modules and does not document the exported `mcp` module at all.
   `claudine/cli/README.md` omits both the `claudine mcp` commands and the wrapper `--mcp` / `--use` flags plus their provider limitations.
   `.claude/skills/claudine/SKILL.md` still describes the older hook/linking-centric module breakdown and does not teach the MCP command surface or runtime session composition behavior.
   The plan also called for a new `claudine/docs/mcp-support.md` in `claudine/plans/2026-03-08.mcp-support-plan.md:604-621`, and that file does not exist.
   Recommendation:
   Update the READMEs and skill file together with the missing architecture doc so the documented command surface, storage model, provider rollout, and wrapper limitations match what actually shipped.

## Idiomatic And Performance Suggestions

- Return a typed `ResolvedMatch { server, tier }` from catalog resolution instead of resolving once and then re-scanning to infer `MatchTier` in `claudine/lib/src/mcp/session.rs:175-200`.
- Use a `HashSet` or `IndexSet` for session dedupe instead of repeated `Vec::contains` checks in `claudine/lib/src/mcp/session.rs:64-113`.
- Replace the current ad hoc `serde_json::Value` and `toml_edit` field picking in import/export/inject with typed provider structs plus small conversion traits. That will make drift against provider research much easier to spot and reduce duplicated mapping logic across `import.rs`, `export.rs`, and `inject.rs`.
- Route `claudine mcp` terminal output through the existing output/logging helpers instead of raw `println!` so redaction, tables, JSON branching, and styling follow the rest of the CLI.

## Verification

- Ran `just test` in `claudine` on 2026-03-09.
- The library suite passed, including the current MCP unit tests.
- The CLI suite is not green locally because `claudine/cli/tests/pty_tests.rs:17` and `claudine/cli/tests/pty_tests.rs:39` timed out with `ExpectTimeout`. Those failures are not MCP-specific, but they mean the wrapper integration layer is already brittle.
- The package-area `just test` recipe currently reports success even when the CLI leg fails because `claudine/justfile:167-169` does not propagate the non-zero exit from `cargo test -p "claudine-cli"`. That makes manual verification less trustworthy than it should be.
