# Claudine Config Review

## Scope

Reviewed the current Claudine configuration implementation against:

- `claudine/features/2026-04-7-refactor-config/spec.md`
- `claudine/features/2026-04-7-refactor-config/tech-design.md`

Assumption for this review: `ClaudineConfig` is intended to be the only active configuration model, and `HookerConfig` should no longer be part of normal runtime or public API behavior.

## Findings

1. **The migration is not complete: `HookerConfig` is still an active runtime and public API, not just dead compatibility code.**
   `HookerConfig` still exists as a public type in `claudine/lib/src/events/config.rs:19` and is re-exported from `claudine/lib/src/events/mod.rs:11-14`. Active CLI flows still load it directly via `load_config()`, including `claudine actions` (`claudine/cli/src/commands/actions.rs:19-26`), `claudine sync` (`claudine/cli/src/commands/sync.rs:182-186`), `claudine hooks` (`claudine/cli/src/commands/hooks.rs:628-639`), and canonical-provider display helpers (`claudine/cli/src/commands/link_display.rs:29-68`). The old `init` implementation is also still compiled in `claudine/cli/src/commands/mod.rs:9`. This means `ClaudineConfig` has not actually replaced `HookerConfig`; the codebase currently has two live config systems.

2. **`preferred_agent` in `ClaudineConfig` is not the source of truth for composition selection.**
   Composition still resolves the favorite provider from legacy linking preferences in `claudine/cli/src/commands/wrap/composition.rs:1464-1470`, and the composition docs still describe that same legacy contract in `claudine/lib/src/composition/select.rs:23-30` and `claudine/lib/src/composition/types.rs:45-53`. The new TUI edits `config.preferred_agent`, but compose/inline-compose/sequence do not consume that field. This is a direct user-visible regression.

3. **The new `canonical_provider` field is disconnected from the existing linking/canonical-provider flow.**
   The TUI edits `ClaudineConfig.canonical_provider`, but the CLI display path for canonical providers still reads the legacy `settings.linking.canonical_provider` structure from `HookerConfig` in `claudine/cli/src/commands/link_display.rs:34-68`. The design simplified canonical provider down to a single field, but the rest of the linking subsystem still assumes the older per-resource/per-scope model. At minimum this causes display drift; depending on the linking entrypoint, it likely also means the new field is ignored by behavior outside the config TUI.

4. **Lifecycle notifications in composition still depend on the legacy runtime config, so new TTS and messenger settings are not authoritative.**
   Composition lifecycle setup still loads `load_runtime_config()` and consumes `GlobalSettings` + legacy messaging in `claudine/cli/src/commands/wrap/composition.rs:540-558`. That means the new TTS and messenger settings in `ClaudineConfig` are not consistently applied across the product, even though the config TUI presents them as the main configuration surface.

5. **`ClaudineConfig::default()` contradicts the spec and the init wizard by disabling logging.**
   `ClaudineConfig::default()` sets `logging: false` in `claudine/lib/src/config/claudine_config.rs:266-278`, while the spec says logging is on by default and the init wizard writes `logging: true`. This creates inconsistent behavior for programmatic defaults, tests, and any call site that instantiates the config without going through the wizard.

6. **Repo-scoped migration/error handling is incomplete.**
   `load_claudine_config()` only runs old-format detection/backup for the primary file path at `claudine/lib/src/dispatch/loader.rs:660-698`; it does not perform the same migration check for the merged repo config at `claudine/lib/src/dispatch/loader.rs:684-694`. Separately, the TUI silently drops repo-config load failures with `.ok()` in `claudine/cli/src/commands/config_tui/mod.rs:40-45`. In practice, an invalid or still-legacy repo config can disappear from the editor instead of being migrated or surfaced clearly.

7. **The messenger TUI can write non-functional configs, and validation does not catch them.**
   The TUI can only create skeletal messenger entries with empty required fields such as `channel_id` or `recipient` in `claudine/cli/src/commands/config_tui/tabs/messenger.rs:242-293`. There is no edit flow for those values in this tab, and `ClaudineMessengerConfig::validate()` only checks `active_config` membership in `claudine/lib/src/config/claudine_config.rs:195-206`, while `ClaudineConfig::validate()` only delegates that same shallow check in `claudine/lib/src/config/claudine_config.rs:292-306`. So the TUI can save a “valid” messenger config that cannot actually send anything.

8. **The TTS voice editor can persist placeholder strings that are not real voices.**
   On first gendered voice selection, the TUI writes `"{provider} default"` into the opposite gender slot in `claudine/cli/src/commands/config_tui/tabs/tts.rs:480-491`. That is a display placeholder being serialized as if it were a real voice identifier. The config layer does not validate TTS providers or voice IDs, so this can persist garbage into the config and defer failure until runtime.

9. **The Actions TUI creates an invalid sound effect by default.**
   Adding a `SoundEffect` action inserts `effect: "attention"` in `claudine/cli/src/commands/config_tui/tabs/actions.rs:541-547`. That is not a real `playa` sound name; the config validator checks sound names via `SoundEffect::from_name` in `claudine/lib/src/config/claudine_config.rs:301-320`. This makes the Actions tab capable of generating a config that should fail validation on save/load.

10. **The new TUI has almost no direct test coverage.**
    There are no unit tests under `claudine/cli/src/commands/config_tui/`, and the existing CLI tests only cover repo-scoped `handle` behavior (`claudine/cli/tests/handle_repo_config.rs`). There are good loader/config tests in `claudine/lib/src/config/claudine_config.rs` and `claudine/lib/src/dispatch/loader.rs`, but almost nothing verifies TUI state transitions, modal flows, repo/user save semantics, or that TUI edits round-trip into valid serialized config.

## Design and Ergonomics Recommendations

- Replace `HookerConfig` in the configurator layer with a narrow registration-only type such as `ProviderHookPlan` or `RegistrationConfig`.
  The provider configurators do not need full legacy global settings; they mainly need to know which canonical events should be registered per provider.

- Add a dedicated repo overlay type instead of reusing full `ClaudineConfig` for repo writes.
  The current repo editor path creates `ClaudineConfig::default()` in `claudine/cli/src/commands/config_tui/tabs/preferences.rs:363-371`, which serializes unrelated defaults into repo scope. A `RepoClaudineConfig` or explicit overlay struct would be clearer and safer.

- Move TUI mutations into pure reducers/helpers and test those directly.
  The current modal handlers mutate `App` in place, which makes behavior harder to reason about and harder to test. A reducer-style layer would make the TUI far easier to verify.

- Strengthen validation for TTS and messenger.
  At minimum:
  - reject blank messenger destination fields
  - validate `tts.provider` against supported provider names
  - reject obviously placeholder voice names before save

- Cache host/provider discovery in the TUI state.
  Functions like `discover_agents_full()` and provider list lookups are cheap today, but the TUI repeatedly recomputes them instead of treating them as session state.

- Remove or quarantine dead legacy docs and modules once the runtime migration is complete.
  The library README still documents `HookerConfig` as the config model in `claudine/lib/README.md:57-64`, which will keep confusing future work.

## Testing Assessment

Focused tests I ran:

- `cargo test -p claudine --lib claudine_config`
- `cargo test -p claudine --lib loader`
- `cargo test -p claudine --test canonical_dispatch`
- `cargo test -p claudine-cli --test handle_repo_config`

All of those passed.

What is still missing:

- direct unit tests for `config_tui` tab handlers and modal flows
- tests that TUI edits serialize to valid `ClaudineConfig`
- tests that `preferred_agent` is honored by composition selection
- tests that repo-scoped config migration/error handling works for the new format and old-format backups
- tests covering messenger/TTS invalid input generated through the TUI

## Suggested Fix Order

1. Make `ClaudineConfig` the only active config source for runtime behavior and CLI surfaces.
2. Wire `preferred_agent` and `canonical_provider` through the composition/linking paths that still read legacy linking settings.
3. Fix the TUI write-path bugs (`attention` sound, fake TTS default voices, messenger skeleton configs).
4. Add reducer-level and integration tests for the TUI before further expanding the editor.
