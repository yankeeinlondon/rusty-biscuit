---
agent: ""
model: ""
ready: false
---

# Feature Review: Centralized Providers

## Findings

### 1. Public compatibility re-exports were removed early

**Severity:** High

The spec/design require deprecated compatibility surfaces for one release cycle: `events::Provider` should remain as a deprecated re-export of `provider::Provider`, and `AgentId` should remain as a deprecated re-export of `Provider`. The current implementation removes both surfaces instead.

- Spec requires `events/provider.rs` to become a thin deprecated re-export: `spec.md:89`.
- Design deprecation timeline keeps `events::Provider` until Phase 8 and `AgentId` until Phase 8: `design.md:297`.
- `events/mod.rs` now only has a comment saying the type moved, with no re-export: `claudine/lib/src/events/mod.rs:28`.
- `agents/mod.rs` exports `parse_agent_id`, but there is no `AgentId` compatibility alias: `claudine/lib/src/agents/mod.rs:13`.

This breaks downstream consumers that still import `claudine::events::Provider`, `claudine::events::provider::Provider`, or `claudine::agents::AgentId`, which the migration plan explicitly promised would continue compiling with deprecation warnings.

**Suggestion:** Restore a deprecated compatibility module/re-export:

- `events::Provider`, `events::PROVIDERS_DISPLAY_ORDER`, and any other previously public provider items that moved.
- `agents::AgentId = Provider` as a deprecated type alias or re-export.
- Add a compile test or simple public API unit/integration test that imports the deprecated paths so this does not regress again.

### 2. Agent discovery still uses a hand-maintained provider table

**Severity:** Medium

`discover_agents_full` still hard-codes provider identity, config path, and sniff binding in `config/mod.rs`:

- Manual table starts at `claudine/lib/src/config/mod.rs:83`.
- It duplicates facts already available in `ProviderInfo`: provider order, `config_paths`, `sniff_binding`, `display_name`, and `binary`.
- The design says `config/` should dispatch through `ConfiguratorBehavior` and should not be edited when adding a new provider: `design.md:359`.

This leaves an important installation/setup path outside the centralized provider catalog. A ninth provider can be added to `provider/<name>.rs` and the central registry, but `claudine init`/agent discovery will still miss it unless this separate table is also updated.

**Suggestion:** Drive `discover_agents_full` from `all_providers()` and `ProviderInfo`. Use `info.sniff_binding`, `info.binary`, and a catalog-backed primary config path derived from `info.config_paths`. Add a test that every `ProviderInfo` entry appears exactly once in discovery and that discovery uses the same sniff binding as `provider_info(provider).sniff_binding`.

### 3. Wrapper registry is still weaker than the designed invariant

**Severity:** Medium

The technical design calls for a CLI-side registry with the same array-backed exhaustiveness property as the lib registry: `design.md:100`. The implementation keeps a `match` with a wildcard fallback:

- `profile_for_provider` matches providers directly: `claudine/cli/src/commands/wrap/profile.rs:711`.
- `Provider::RooCode | _ => None` silently classifies any future provider as having no wrapper: `claudine/cli/src/commands/wrap/profile.rs:722`.

The test iterates `PROVIDERS_DISPLAY_ORDER`, which is useful, but it does not protect against a future `Provider` variant being added without also updating `PROVIDERS_DISPLAY_ORDER`/`PROVIDER_COUNT`. In that case, the wildcard keeps compiling and returns `None`, undercutting the design goal that new providers force registry updates.

**Suggestion:** Replace the wildcard match with a registry structure that cannot silently absorb future variants. If Roo remains intentionally unwrapped, model that explicitly as `Option<&'static dyn WrapperProfile>` in an array indexed by provider, and assert the registry length against the canonical provider count. Avoid `_` in provider registry code.

### 4. The dispatch drift guard is too narrow

**Severity:** Low

The `no_unauthorized_match_provider_in_lib` guard only searches for literal `match provider` text: `claudine/lib/src/provider/tests.rs:319`. It does not catch other provider drift patterns, including:

- Provider arrays like the one in `discover_agents_full`.
- Matches written as `match p`, `match self.provider`, or helper-specific names.
- Direct duplicated provider facts such as `AiCli::*` bindings and config paths.

This allowed the discovery table above to remain despite the feature's stated goal that provider facts flow through the central catalog.

**Suggestion:** Add targeted invariants instead of relying only on source scanning. Useful ones:

- Agent discovery covers exactly `all_providers()` and uses each provider's `sniff_binding`.
- Wrapper registry covers exactly every provider, with only documented no-wrapper exceptions.
- Deprecated public paths compile during the migration window.
- `ProviderInfo.config_paths` has at least one user-level config path for every provider that discovery reports.

## Verification

I ran:

```bash
cargo test -p claudine provider::tests
```

Result: passed, 26 provider tests. This validates the central provider catalog tests that currently exist, but it does not cover the compatibility and discovery gaps above.

## Production Readiness

Not ready. The core catalog is in good shape and the prior compile/migration issues appear to be fixed, but the early removal of promised compatibility APIs is a release blocker for a refactor whose non-goal is user-facing/API regression. The discovery table should also be centralized before calling the provider model drift-proof.
