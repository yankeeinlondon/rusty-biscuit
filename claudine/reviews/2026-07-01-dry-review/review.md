---
created: '2026-07-01T07:07:08'
updated: '2026-07-16'
agent: 'codex/default'
area: 'claudine'
recommendations: 8
active_recommendations: 8
partially_addressed: 1
critical: 0
---

# DRY Review

Reviewed the Claudine package area for duplicated logic, repeated constants, parallel validation rules, copy/paste setup, and abstractions that would reduce meaningful drift. The original review covered `claudine/lib`, `claudine/cli`, `claudine/contract`, and `claudine/rendezvous` on 2026-07-01.

## 2026-07-16 Reassessment

All eight recommendations remain relevant. Finding 2 was partially addressed when the completion root menu and composition provider switches were derived from the provider catalog on 2026-07-08. The remaining findings were not materially addressed. Findings 3 and 6 now have observable drift, not just a risk of future drift.

| # | Current status | Importance | Effort |
|---|---|---|---|
| 1 | Still valid | IMPORTANT | HIGH |
| 2 | Partially addressed | IMPORTANT | MEDIUM |
| 3 | Still valid; drift confirmed | IMPORTANT | LOW |
| 4 | Still valid; scope has grown | NICE-TO-HAVE | LOW |
| 5 | Still valid; rationale corrected | IMPORTANT | MEDIUM |
| 6 | Still valid; drift confirmed | IMPORTANT | HIGH |
| 7 | Still valid; stronger than originally stated | IMPORTANT | MEDIUM |
| 8 | Still valid; test surface has grown | NICE-TO-HAVE | MEDIUM |

## 1. Consolidate the Two Messaging Configuration Models

- **Status:** STILL VALID
- **Importance:** IMPORTANT
- **Level of Effort:** HIGH
- **Files:** `claudine/lib/src/config/messaging_block.rs`, `claudine/lib/src/messaging/config.rs`, `claudine/lib/src/dispatch/loader.rs`

`MessengerProviderConfig` and `MessagingRouteConfig` still encode the same six route kinds and repeat all eight default environment-variable helpers. Their Discord and Slack webhook branches still duplicate the same "inline URL or nonblank environment variable" rule and call the same URL validators.

The duplication now has a third maintenance seam: `dispatch::loader::bridge_provider_config` exhaustively converts every `MessengerProviderConfig` variant into its corresponding `MessagingRouteConfig`. The differences between the types remain intentional but narrow:

- `MessengerProviderConfig` is the persisted `ClaudineConfig` shape used by init and the config TUI. It allows incomplete bot routes while the TUI is editing them.
- `MessagingRouteConfig` is the richer runtime shape. It supports inline credentials and stricter required-field validation, and it is nested in the user/repo-scoped runtime settings.

Suggested direction: retain the persisted and runtime containers where their policies genuinely differ, but introduce one shared route-kind/default/credential-source core. Centralize webhook validation, default environment-variable names, and provider labels there. Keep WIP-versus-runtime validation as explicit policy layered on the shared rules. If the runtime type does not need to remain a second serializable configuration grammar, make it a resolved runtime representation rather than another public config model. Keep `bridge_provider_config` focused on the intentional shape differences.

## 2. Finish Consolidating Wrapper Inventory and Telemetry Classification

- **Status:** PARTIALLY ADDRESSED
- **Importance:** IMPORTANT
- **Level of Effort:** MEDIUM
- **Files:** `claudine/cli/src/argv/mod.rs`, `claudine/cli/src/completion/root_menu.rs`, `claudine/cli/src/main.rs`, `claudine/cli/src/telemetry.rs`

The completion portion of the original finding is complete. `completion::root_menu::wrapper_tokens()` now derives root-menu wrappers from `PROVIDERS_DISPLAY_ORDER`, and composition provider switches are generated from the same catalog. This correctly picked up Kilo, Pi, and Antigravity.

The rest of the inventory remains hand-maintained:

- `argv::WRAPPER_SUBCOMMANDS` lists all ten wrapper names as string literals.
- `main::wrapper_command` maps ten `Commands` variants to `Provider` variants.
- `telemetry::command_name` and `telemetry::provider_subcommand_name` repeat the wrapper match.
- `telemetry::root_span` has separate wrapper matches for effective-CWD selection and detailed wrapper fields.

There is already drift inside telemetry: `command_name` and `provider_subcommand_name` include Kilo, Pi, and Antigravity, but the effective-CWD and detailed-span matches still cover only the older seven wrappers. Those three providers therefore receive the generic telemetry span and do not use the same repo-root CWD treatment as the older wrappers.

Suggested direction: give `Commands` one borrowed wrapper-classification method returning the provider plus `WrapperArgs`, and use it in dispatch and telemetry. Derive string inventories from the provider catalog where a runtime collection is acceptable. Clap's enum variants will remain explicit, so add one coverage test that compares the classified wrapper providers and subcommand tokens with `PROVIDERS_DISPLAY_ORDER`. Do not undo the already-catalog-derived completion menu.

## 3. Reuse the Clap-Derived Composition Flag Surface in Completion

- **Status:** STILL VALID; DRIFT CONFIRMED
- **Importance:** IMPORTANT
- **Level of Effort:** LOW
- **Files:** `claudine/cli/src/argv/partition.rs`, `claudine/cli/src/completion/engine/tokens.rs`, `claudine/cli/src/completion/engine/mod.rs`

The original `collect_composition_value_flags()` implementation was replaced by a stronger abstraction: `argv::partition::OwnedFlags::for_composition()` derives both value-taking and boolean flags from `Cli`, `ComposeArgs`, and `SequenceArgs`. The completion engine, however, still uses a hand-maintained `is_value_bearing_flag()` match.

The completion mirror has already drifted. It does not include these current value-taking `SharedComposeArgs` flags:

- `--step-timeout`
- `--stall-timeout`
- `--max-iterations`
- `--on-rate-limit`

When completion scans an argv containing one of these flags in space-separated form, it can mistake the flag's value for the composition file or a setter candidate.

Suggested direction: expose a narrow read-only query from `OwnedFlags`, such as `composition_flag_consumes_next(token)`, and use it from completion. Cache the derived surface if repeated clap introspection is measurable, but keep clap as the source of truth. Add a drift test that walks every clap-derived value-taking composition flag and asserts that completion skips its following value; do not replace this with another fixed expected list.

## 4. Centralize Setter and Markdown-Extension Syntax Predicates

- **Status:** STILL VALID; SCOPE HAS GROWN
- **Importance:** NICE-TO-HAVE
- **Level of Effort:** LOW
- **Files:** `claudine/cli/src/argv/mod.rs`, `claudine/cli/src/commands/compose/setters.rs`, `claudine/cli/src/completion/engine/tokens.rs`, `claudine/cli/src/completion/frontmatter.rs`, `claudine/cli/src/completion/setter_value.rs`, `claudine/cli/src/completion/default_glob.rs`, `claudine/cli/src/commands/schema_interactive/mod.rs`

The setter grammar is now implemented independently by `argv::looks_like_setter`, `compose::setters::parse_compose_setter`, `completion::engine::tokens::split_setter`, and `completion::engine::tokens::is_setter_shaped`. They all intend to recognize `^[A-Za-z_][A-Za-z0-9_-]*=`.

The case-insensitive `.md`/`.markdown` test is now repeated in four places: completion frontmatter inspection, setter-value completion, default-glob completion, and schema-interactive file detail extraction.

Suggested direction: add a small CLI syntax module containing a setter splitter/classifier and `has_markdown_extension`. Let the value-parsing layer remain responsible for JSON/JSON5 coercion after the shared setter classifier returns the key and raw value. This should remain a small pure-helper extraction, not a general parsing framework.

## 5. Put Git Repository-Root Discovery Behind One Helper

- **Status:** STILL VALID; ORIGINAL RATIONALE CORRECTED
- **Importance:** IMPORTANT
- **Level of Effort:** MEDIUM
- **Files:** `claudine/lib/src/composition/prepare.rs`, `claudine/lib/src/linking/paths.rs`, `claudine/cli/src/completion/scopes.rs`, `claudine/cli/src/completion/engine/mod.rs`, `claudine/cli/src/commands/wrap/harness_orch/shell_options.rs`, `claudine/cli/src/telemetry.rs`

Upward repository-root walks remain duplicated in composition preparation, completion scope discovery, completion's repo-config check, harness shell-policy selection, and telemetry. Other paths already use shared discovery: composition resolution uses `biscuit_file::find_git_root`, while linking's `resolve_repo_root` uses Sniff.

The original review overstated one behavioral difference. `path.join(".git").exists()` recognizes both normal `.git` directories and linked-worktree `.git` files, just as the explicit `is_dir() || is_file()` checks do. Worktree pointer support is therefore not the reason to consolidate these loops.

The remaining differences are still meaningful: callers disagree about whether the start is a file or directory, whether failure returns `None` or falls back to the CWD/source directory, whether repo-config lookup is folded into discovery, and whether a non-Git repository root is acceptable.

Suggested direction: add one cheap Git-root-only helper with `Option<PathBuf>` semantics using Sniff's Tier-3 repository discovery (`sniff::filesystem::git::GitRepo::discover`) and layer caller-specific fallback/config behavior above it. Keep broader repository/package discovery separate. `linking::resolve_repo_root` can then become a fallback wrapper over the same primitive instead of the only Sniff-backed implementation.

## 6. Derive Provider Resource Paths From the Generated Catalog

- **Status:** STILL VALID; DRIFT CONFIRMED
- **Importance:** IMPORTANT
- **Level of Effort:** HIGH
- **Files:** `claudine/cli/src/completion/scopes.rs`, `claudine/lib/src/protect/path.rs`, `claudine/cli/src/commands/wrap/repo_home.rs`, `claudine/lib/src/provider/*/data.rs`

This finding is more important after the provider-metadata work completed. Provider `data.rs` files are now generated and expose `agent_offset`, `supports_skills`, config paths, and a typed `ProviderCapabilities` resource descriptor with repo/user paths. The three consumers in this finding still maintain separate literal policies.

Concrete drift is visible today:

- Completion's `SKILL_PEER_DIRS` still lists only the older seven provider directories, so it omits catalog-supported Kilo (`.kilo/skills`), Pi (`.pi/skills`), and Antigravity (`.agents/skills`) roots.
- Protect's sensitive home prefixes omit `.kimi`, `.kilo`, `.pi`, and `.agents`, while retaining the removed `.roo` entry.
- `RepoHomeManager::excluded_dirs` has custom branches only for the older seven offsets; Kilo, Pi, and Antigravity fall through to a generic exclusion set even though the generated resource catalog now describes their actual layouts.

These consumers do not need one identical list. Completion needs supported repo skill paths (including documented alternate read paths), protect needs a conservative security policy, and shadow-HOME isolation needs provider-specific exclusion behavior.

Suggested direction: derive completion paths directly from `ProviderCapabilities`. For protect, combine provider-derived agent/config roots with the non-provider sensitive-path catalog, keeping an explicit test that every compiled provider contributes its relevant home roots; remove Roo unless a non-provider compatibility requirement is documented. For shadow HOME, add a typed policy descriptor only if `ProviderCapabilities` cannot express the exclusions—do not infer behavior from string-matching `agent_offset`. Because `data.rs` is generated, any new metadata field must flow through facts/overrides and `claudine-gen`, not be edited into generated files.

## 7. Extract Shared MCP Export/Injection Entry Builders

- **Status:** STILL VALID; STRONGER THAN ORIGINALLY STATED
- **Importance:** IMPORTANT
- **Level of Effort:** MEDIUM
- **Files:** `claudine/lib/src/mcp/export.rs`, `claudine/lib/src/mcp/inject.rs`, `claudine/lib/src/mcp/types.rs`

The duplication is not limited to small field fragments. Export and runtime injection independently build essentially the same complete provider entry for Codex, Gemini, and OpenCode. They also duplicate the full family of typed provider-override readers (`value`, `bool`, `i64`, `string`, string array, and string map). For Gemini, the command/args/env/URL/tool-filter/override assembly is effectively repeated wholesale; Codex and OpenCode have the same pattern.

Document loading, managed-entry replacement, native-name versus catalog-ID selection, shadow-HOME placement, and injection result construction are legitimately different. Provider entry serialization is not.

Suggested direction: create one typed entry builder per provider schema and reuse it from both export and injection. Keep document mutation and file placement in the existing writer/injector functions. Move typed override accessors to `McpServer` or one shared MCP helper module. Retain provider-specific golden tests for the final JSON/TOML documents and add parity tests proving export and injection serialize the same server fields.

## 8. Add Integration-Test Builders for Common Fake Provider Runs

- **Status:** STILL VALID; TEST SURFACE HAS GROWN
- **Importance:** NICE-TO-HAVE
- **Level of Effort:** MEDIUM
- **Files:** `claudine/cli/tests/common`, `claudine/cli/tests/wrap_*.rs`, `claudine/cli/tests/compose_schema_cli.rs`, `claudine/cli/tests/sequence_*.rs`

`cli/tests/common` still provides useful primitives such as `TestWorkspace`, `write`, `write_executable`, `augmented_path`, catalog seeders, and `seed_minimal_config`, but it still has no fake-provider run builder. The repeated setup has spread well beyond the three files named in the original review: 49 current integration-test files invoke `cargo_bin_cmd!("claudine")`, and the wrapper, composition, and sequence suites repeatedly assemble the same HOME/bin/config/PATH/capture environment.

Suggested direction: add a deliberately narrow fixture that owns a temporary home, fake bin directory, minimal config, capture paths, and a preconfigured Claudine command. Let individual tests provide the provider script and assertions. Use `std::env::join_paths` through the existing `augmented_path` helper. Keep shell-script conveniences under `#[cfg(unix)]`; if cross-platform fake execution is needed, give the fixture a Windows `.cmd` path rather than embedding Unix assumptions in the common core.
