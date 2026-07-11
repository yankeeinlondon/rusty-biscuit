---
created: '2026-07-01T07:07:08'
agent: 'codex/default'
area: 'claudine'
recommendations: 8
critical: 0
---

# DRY Review

Reviewed the Claudine package area for duplicated logic, repeated constants, parallel validation rules, copy/paste setup, and abstractions that would reduce meaningful drift. I focused on `claudine/lib`, `claudine/cli`, `claudine/contract`, and `claudine/rendezvous`.

## 1. Consolidate the Two Messaging Configuration Models

- **Importance:** IMPORTANT
- **Level of Effort:** HIGH
- **Files:** `claudine/lib/src/config/messaging_block.rs`, `claudine/lib/src/messaging/config.rs`

`MessengerProviderConfig` and `MessagingRouteConfig` encode the same provider families, default environment variable names, webhook URL fields, and webhook validation rules in two separate type trees. The duplicated defaults are visible in `config/messaging_block.rs` lines 13-42 and `messaging/config.rs` lines 13-42. The webhook validation branches are also repeated: `config/messaging_block.rs` lines 156-212 and `messaging/config.rs` lines 220-365.

This is more than line-count duplication. The two models already differ in subtle ways: `MessagingRouteConfig` supports inline bot tokens and has stricter required-field checks for bot routes, while `MessengerProviderConfig` intentionally allows WIP bot-token routes and only validates webhook routes. Those differences may be real, but the common provider identity, default env var names, webhook URL validation, and "inline URL or env var" rule should come from one shared route descriptor or shared validation helper.

Suggested direction: keep separate public shapes if compatibility requires it, but introduce a small shared `messaging::route_spec` layer for provider defaults, provider kind labels, webhook URL rules, and `validate_webhook_route(provider, name, inline_url, env_var, error_context)`. Add tests at the shared helper and keep model-specific tests only for policy differences.

## 2. Derive Root Provider Menus and Telemetry Provider Names From the Provider Catalog

- **Importance:** IMPORTANT
- **Level of Effort:** MEDIUM
- **Files:** `claudine/cli/src/argv/mod.rs`, `claudine/cli/src/completion/root_menu.rs`, `claudine/cli/src/telemetry.rs`

The wrapper provider list is repeated in several places:

- `WRAPPER_SUBCOMMANDS` hardcodes seven wrapper names in `argv/mod.rs` lines 66-69.
- Shell completion root menu hardcodes the same names in `completion/root_menu.rs` lines 54-63 and tests repeat the expected order at lines 151-159.
- Telemetry maps each wrapper command to a provider slug in `telemetry.rs` lines 157-166.

This is an "almost duplicate" provider catalog next to the real provider metadata (`Provider`, `ProviderInfo`, display order, aliases). The risk is that adding/removing a wrapper provider requires updating command definitions, completion output, argv pass-through rules, telemetry, and tests independently. `roo` already demonstrates the need for one explicit policy: it exists as a provider, but is intentionally catalog-only and absent from wrapper completion.

Suggested direction: add a single CLI-facing provider list such as `WRAPPER_PROVIDERS: &[Provider]` or `Provider::is_direct_wrapper_supported()`, with a clear exclusion reason for catalog-only providers. Use it to generate `WRAPPER_SUBCOMMANDS`, completion menu provider entries, and telemetry provider slugs. Keep the root menu's non-provider grouping declarative, but avoid duplicating provider names.

## 3. Reuse the Clap-Derived Composition Flag Surface in Completion

- **Importance:** IMPORTANT
- **Level of Effort:** LOW
- **Files:** `claudine/cli/src/argv/mod.rs`, `claudine/cli/src/completion/engine.rs`

`argv/mod.rs` already derives value-bearing composition flags from clap at first use through `collect_composition_value_flags()` (`argv/mod.rs` lines 83-120). The completion engine then keeps a hand-maintained mirror in `is_value_bearing_flag()` (`completion/engine.rs` lines 512-536), with a comment acknowledging the duplication.

This is exactly the kind of CLI/API argument parsing duplication that drifts when a new composition flag is added. A missing update in completion can misclassify the flag's value as a file argument or setter, while argv normalization behaves correctly.

Suggested direction: expose a small predicate from `argv`, for example `composition_flag_takes_value(token: &str)`, backed by the clap-derived list. The completion engine can call that predicate without depending on the private `LazyLock` directly. Add one test that defines expected behavior through clap introspection rather than a fixed duplicated list.

## 4. Centralize Setter and Markdown-Extension Predicates Used by Completion

- **Importance:** NICE-TO-HAVE
- **Level of Effort:** LOW
- **Files:** `claudine/cli/src/completion/engine.rs`, `claudine/cli/src/completion/setter_value.rs`, `claudine/cli/src/completion/frontmatter.rs`, `claudine/cli/src/argv/mod.rs`

Several tiny predicates are duplicated to keep modules self-contained:

- `completion/engine.rs` lines 539-555 duplicates the setter-shape rule from argv normalization.
- `completion/setter_value.rs` lines 231-241 duplicates the Markdown extension predicate from `completion/frontmatter`.

These are small today, so this is not urgent. The risk is semantic drift in completion behavior: if the accepted setter grammar or prompt file extensions change, completion and argv/file scanning can diverge.

Suggested direction: create a narrow `completion::syntax` or `cli_syntax` module for pure predicates: `is_setter_shaped`, `has_markdown_extension`, and maybe `is_flag_token`. Avoid a large abstraction; just move the rules that are explicitly documented as mirrors.

## 5. Put Git/Repository Root Detection Behind One Helper

- **Importance:** IMPORTANT
- **Level of Effort:** MEDIUM
- **Files:** `claudine/lib/src/composition/prepare.rs`, `claudine/lib/src/composition/resolve.rs`, `claudine/cli/src/completion/scopes.rs`, `claudine/cli/src/completion/engine.rs`, `claudine/cli/src/commands/wrap/harness_orch/shell_options.rs`, `claudine/cli/src/telemetry.rs`

There are multiple local implementations of "walk upward to find the repository root":

- `composition/prepare.rs` lines 102-112.
- `completion/scopes.rs` lines 237-250.
- `completion/engine.rs` lines 592-606.
- `wrap/harness_orch/shell_options.rs` lines 23-36.
- `telemetry.rs` lines 170-178.

Some variants use `path.join(".git").exists()`, while completion explicitly checks `is_dir() || is_file()` for worktree pointer support. On macOS/Linux/Windows, git worktrees commonly use a `.git` file, so duplicated root detection can produce inconsistent behavior between completion, prompt resolution, shell policy roots, and telemetry.

Suggested direction: add a shared non-IO-heavy helper in the library, or use `sniff` where richer discovery is already appropriate. The helper should handle `.git` directories and `.git` files consistently, accept a starting file or directory, and document whether it is "git-root only" or "workspace/package-aware." Then replace local loops with the shared helper.

## 6. Model Provider Resource Directories Once

- **Importance:** IMPORTANT
- **Level of Effort:** MEDIUM
- **Files:** `claudine/cli/src/completion/scopes.rs`, `claudine/lib/src/protect/path.rs`, `claudine/cli/src/commands/wrap/repo_home.rs`, `claudine/lib/src/provider/*`

Provider resource directories and offsets appear in several separate literal lists:

- Completion's skill peer directories are hardcoded in `completion/scopes.rs` lines 253-266.
- Sensitive home prefixes include provider directories in `protect/path.rs` lines 33-51.
- Shadow-home repo exclusions are hardcoded by `agent_offset` in `repo_home.rs` lines 140-158.
- Provider metadata already carries `agent_offset`, resource support, and per-provider paths in the provider modules.

Some duplication is domain-specific, but these lists encode the same provider directory universe (`.claude`, `.codex`, `.gemini`, `.opencode`, `.goose`, `.qwen`, `.kimi`, `.roo`) with different omissions. That makes provider rollout brittle: adding a provider or changing resource support requires searching for all hardcoded directory lists.

Suggested direction: introduce a small provider-resource descriptor derived from `ProviderInfo` and resource capabilities, for example `Provider::agent_offset()` plus `ProviderResourceSupport { skills, commands, agents, hooks, prompts }`. Use policy-specific filters where needed (`protect` may still include sensitive dirs even when linking/completion does not), but make omissions explicit instead of implicit list drift.

## 7. Extract Shared MCP Export/Injection Entry Builders

- **Importance:** NICE-TO-HAVE
- **Level of Effort:** MEDIUM
- **Files:** `claudine/lib/src/mcp/export.rs`, `claudine/lib/src/mcp/inject.rs`

MCP export and runtime injection build provider-specific JSON/TOML entries by repeatedly copying common fields such as command, args, env, URL, headers, enabled/disabled tools, and provider overrides. Examples include Codex export in `mcp/export.rs` lines 363-388, Gemini export in lines 420-447, OpenCode export in lines 480-511, and Gemini runtime injection in `mcp/inject.rs` lines 281-310.

The providers do have real schema differences, so a large generic serializer would likely hurt readability. The duplication worth removing is the common transformation from `McpServer`/`ExportServer` into small reusable field fragments: command+args, env maps, header maps, tool filters, and provider override extraction.

Suggested direction: keep provider writer functions, but add focused helpers like `insert_json_if_non_empty`, `insert_sorted_map_table`, `stdio_command_array`, and typed provider override accessors grouped per target schema. Tests should cover each helper once and retain provider-specific golden tests for schema shape.

## 8. Add Integration-Test Builders for Common Fake Provider Runs

- **Importance:** NICE-TO-HAVE
- **Level of Effort:** MEDIUM
- **Files:** `claudine/cli/tests/common`, `claudine/cli/tests/wrap_inline_compose.rs`, `claudine/cli/tests/wrap_opencode.rs`, `claudine/cli/tests/compose_schema_cli.rs`

There is already a useful `cli/tests/common` module with `TestWorkspace`, `write`, `write_executable`, `augmented_path`, and wrap fixtures (`common/mod.rs` lines 30-91, `common/wrap.rs` lines 197-200). However, many integration tests still repeat the same setup pattern:

- Create temp workspace and `bin` dir.
- Seed minimal Claudine config.
- Write a fake provider executable.
- Run `cargo_bin_cmd!("claudine")` with `NO_COLOR`, `HOME`, and `PATH`.
- Capture args/env/sentinel files.

Examples are `wrap_inline_compose.rs` lines 47-76 and 188-220, `wrap_opencode.rs` lines 17-44 and 117-143, and `compose_schema_cli.rs` lines 35-70 and 100-140.

Suggested direction: add a small builder in `cli/tests/common/wrap.rs`, for example `FakeProviderRun::new("opencode").with_model_env(...).capture_args().capture_env().run(args)`. Keep it intentionally narrow and Unix-gated where shell scripts are used. This would reduce test noise and make future behavioral assertions less likely to forget required environment isolation.
