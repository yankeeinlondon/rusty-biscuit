# ClaudineConfig Migration Plan

Implements all findings and recommendations from `review.md`. Goal: make `ClaudineConfig` the sole authoritative configuration for runtime behavior, CLI surfaces, and TUI editing. Eliminate dual-config inconsistencies.

## Phase 1 — Quick Fixes (Findings 5, 8, 9)

Low-risk, independent fixes with no cross-cutting dependencies.

### 1.1 Fix `ClaudineConfig::default()` logging (Finding 5)

**File:** `claudine/lib/src/config/claudine_config.rs:271`

Change `logging: false` to `logging: true`. The init wizard already sets `true`, and the user-facing message says "both are enabled by default". The default must agree.

### 1.2 Fix actions TUI invalid sound effect (Finding 9)

**File:** `claudine/cli/src/commands/config_tui/tabs/actions.rs:544`

Replace `"attention".to_string()` with a valid playa sound effect name. Use `claudine::events::init_defaults::recommended_sound(&event)` to pick the right sound for the event being configured, rather than hardcoding any single name. This makes the default contextually appropriate.

### 1.3 Fix TTS tab placeholder voices (Finding 8)

**File:** `claudine/cli/src/commands/config_tui/tabs/tts.rs:480-491`

The `format!("{} default", cfg.provider)` placeholders are serialized as real voice IDs. Instead, when the user selects the first gendered voice, set the opposite gender to `VoiceSelection::Single(voice_name)` (keeping it simple) and only create a `Gendered` entry when both voices have been explicitly selected. Alternatively, use an empty string and add validation to reject it — but the simpler path is to use `Single` until both are set.

**Concrete change:** Replace the two placeholder arms with:

```rust
(_, GenderTab::Female) => {
    cfg.voice = Some(VoiceSelection::Single(voice_name));
}
(_, GenderTab::Male) => {
    cfg.voice = Some(VoiceSelection::Single(voice_name));
}
```

This is safe because the user can always come back and set the other gender, at which point the existing `Gendered { male, .. }` / `Gendered { female, .. }` arms handle promotion to `Gendered`.

## Phase 2 — Messenger & Config Validation (Findings 6, 7)

### 2.1 Strengthen messenger validation (Finding 7)

**File:** `claudine/lib/src/config/claudine_config.rs` — `ClaudineMessengerConfig::validate()` and `ClaudineConfig::validate()`

Add field-level validation to `MessengerProviderConfig`. Each variant has required destination fields (`channel_id` or `recipient`) — validate they are non-empty:

```rust
impl MessengerProviderConfig {
    fn validate(&self, name: &str) -> Result<()> {
        match self {
            Self::Discord { channel_id, .. } | Self::Slack { channel_id, .. } => {
                if channel_id.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(
                        format!("messenger.configurations.{name}: channel_id cannot be blank")
                    ));
                }
            }
            Self::Signal { recipient, .. } | Self::Whatsapp { recipient, .. } => {
                if recipient.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(
                        format!("messenger.configurations.{name}: recipient cannot be blank")
                    ));
                }
            }
        }
        Ok(())
    }
}
```

Call from `ClaudineMessengerConfig::validate()`:

```rust
for (name, config) in &self.configurations {
    config.validate(name)?;
}
```

**Validation scope:** Only validate destination fields on configurations that are referenced by `active_config`. Non-active configs may have empty fields (they're work-in-progress). This means the validation loop becomes:

```rust
if let Some(active) = &self.active_config {
    if let Some(config) = self.configurations.get(active) {
        config.validate(active)?;
    }
}
```

**TUI impact:** The messenger tab currently inserts skeleton configs with empty fields and immediately sets them as `active_config`. After this change, saving that config will fail validation because the active config has blank required fields. Two options:

- **(A) Don't set `active_config` on create** — let the user set it after filling in fields. This is the minimal change.
- **(B) Prompt for required fields inline** — more complex, deferred to Phase 6 reducer work.

Recommend **(A)** for now: remove the `messenger.active_config = Some(name)` line at `messenger.rs:289` when creating a new config. The user must explicitly activate it after editing.

### 2.2 Fix repo config error swallowing (Finding 6 — partial)

**File:** `claudine/cli/src/commands/config_tui/mod.rs:44`

Replace `.ok()` with a `match` that warns the user:

```rust
let repo_cfg = if repo_cfg_path.exists() {
    match claudine::dispatch::loader::load_claudine_config(Some(&repo_cfg_path), None) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!("Warning: repo config at {} could not be loaded: {}", repo_cfg_path.display(), e);
            None
        }
    }
} else {
    None
};
```

### 2.3 Add old-format detection for repo config (Finding 6 — partial)

**File:** `claudine/lib/src/dispatch/loader.rs:686-694`

The repo config merge block does not check `migration::is_old_format()`. Add the same detection that the user config path gets:

```rust
if repo_path.is_file() {
    let repo_raw = std::fs::read_to_string(&repo_path)?;
    let repo_value = parse_json5_to_value(&repo_raw)?;
    if migration::is_old_format(&repo_value) {
        migration::backup_old_config(&repo_path)?;
        debug!(?repo_path, "Backed up old-format repo config");
        // Skip merge — repo config was legacy format
    } else {
        let repo_config: ClaudineConfig =
            serde_json::from_value(repo_value).map_err(ClaudineError::JsonParse)?;
        debug!(?repo_path, "Loaded ClaudineConfig (repo)");
        merge_claudine_configs(&mut config, &repo_config);
    }
}
```

## Phase 3 — Wire `ClaudineConfig` Into Composition & Linking (Findings 2, 3, 4)

The core architectural change. Replace all composition and linking reads from `HookerConfig` with `ClaudineConfig`.

### 3.1 Replace `load_config_favorite()` with `ClaudineConfig` (Finding 2)

**File:** `claudine/cli/src/commands/wrap/composition.rs:1464-1471`

Current code loads `HookerConfig` and reads `settings.linking.preference[0]`. Replace with:

```rust
fn load_config_favorite(cwd: &Path) -> Option<Provider> {
    let repo_root = sniff::filesystem::git::detect_git(cwd, false, 1)
        .ok()
        .flatten()
        .map(|info| info.repo_root);
    let config = claudine::dispatch::loader::load_claudine_config(None, repo_root.as_deref()).ok()?;
    Some(config.preferred_agent)
}
```

This makes `preferred_agent` the source of truth for composition provider selection.

### 3.2 Replace lifecycle config loading with `ClaudineConfig` (Finding 4)

**File:** `claudine/cli/src/commands/wrap/composition.rs:540-560`

The lifecycle setup currently loads `load_runtime_config()` (legacy) to get `GlobalSettings` and `RuntimeMessagingSettings`. These need to come from `ClaudineConfig` instead.

**Strategy:** Add a bridge function in the loader that converts `ClaudineConfig` TTS and messenger settings into the types `LifecycleRuntimeContext` expects.

**File:** `claudine/lib/src/dispatch/loader.rs` — add:

```rust
/// Bridge `ClaudineConfig` TTS settings to legacy `GlobalSettings`.
///
/// Constructs a minimal `GlobalSettings` containing only the TTS
/// configuration, suitable for `LifecycleRuntimeContext`.
pub fn bridge_tts_settings(config: &ClaudineConfig) -> GlobalSettings {
    let tts = match &config.tts {
        TtsValue::Boolean(false) => None,
        TtsValue::Boolean(true) => Some(TtsSettings::default()),
        TtsValue::Config(cfg) => {
            let voice = match &cfg.voice {
                Some(VoiceSelection::Single(v)) => Some(v.clone()),
                Some(VoiceSelection::Gendered { male, female }) => {
                    // Use gender-appropriate voice
                    match cfg.gender {
                        Gender::Male => Some(male.clone()),
                        Gender::Female => Some(female.clone()),
                    }
                }
                None => None,
            };
            Some(TtsSettings {
                provider: Some(cfg.provider.clone()),
                voice,
                rate: None,
            })
        }
    };
    GlobalSettings {
        tts,
        ..GlobalSettings::default()
    }
}

/// Bridge `ClaudineConfig` messenger settings to `RuntimeMessagingSettings`.
///
/// Reuses `bridge_messenger_to_runtime` from `compile_canonical_runtime`.
pub fn bridge_messaging_settings(config: &ClaudineConfig) -> RuntimeMessagingSettings {
    config.messenger.as_ref()
        .map(bridge_messenger_to_runtime)
        .unwrap_or_default()
}
```

The `bridge_messenger_to_runtime()` function already exists at `loader.rs:468` and is used by `compile_canonical_runtime()`. The new public wrapper just exposes it for composition's lifecycle path.

Then update composition.rs:540-560:

```rust
let (lifecycle_settings, lifecycle_messaging) = if lifecycle.is_empty() {
    (GlobalSettings::default(), RuntimeMessagingSettings { user: None, repo: None })
} else {
    match claudine::dispatch::loader::load_claudine_config(None, effective_repo_root) {
        Ok(config) => (
            claudine::dispatch::loader::bridge_tts_settings(&config),
            claudine::dispatch::loader::bridge_messaging_settings(&config),
        ),
        Err(_) => (GlobalSettings::default(), RuntimeMessagingSettings { user: None, repo: None }),
    }
};
```

### 3.3 Replace `link_display.rs` canonical provider display (Finding 3)

**File:** `claudine/cli/src/commands/link_display.rs:29-97`

Replace the two functions that read `HookerConfig` with `ClaudineConfig`:

**`repo_canonical_needs_init`:** Check whether `canonical_provider` is `None` for the repo scope. With `ClaudineConfig`, canonical provider is a single `Option<Provider>` (not per-resource), so this simplifies to checking whether a repo-scoped config has `canonical_provider` set.

```rust
pub(crate) fn repo_canonical_needs_init(
    paths: &ProviderSkillPaths,
    _resource: LinkableResource,
) -> bool {
    let repo_root = Some(paths.repo_root());
    let repo_path = paths.repo_root().join(".claudine").join("config.json");
    if !repo_path.exists() {
        return true;
    }
    match claudine::dispatch::loader::load_claudine_config(Some(&repo_path), None) {
        Ok(config) => config.canonical_provider.is_none(),
        Err(_) => true,
    }
}
```

**`render_canonical_providers`:** Read from `ClaudineConfig` instead of `HookerConfig`:

```rust
pub(crate) fn render_canonical_providers(
    term: &Terminal,
    paths: &ProviderSkillPaths,
    is_git_repo: bool,
    _resource: LinkableResource,
) {
    let user_config = claudine::dispatch::loader::load_claudine_config(None, None).ok();
    let user_canonical = user_config.as_ref().and_then(|c| c.canonical_provider);

    let not_configured = "<i><red>not configured</red></i>";
    let user_part = match user_canonical {
        Some(p) => format!("user: <b>{p}</b>"),
        None => format!("user: {not_configured}"),
    };

    let line = if is_git_repo {
        let repo_path = paths.repo_root().join(".claudine").join("config.json");
        let repo_config = claudine::dispatch::loader::load_claudine_config(Some(&repo_path), None).ok();
        let repo_canonical = repo_config.as_ref().and_then(|c| c.canonical_provider);
        let repo_part = match repo_canonical {
            Some(p) => format!("repo: <b>{p}</b>"),
            None => format!("repo: {not_configured}"),
        };
        format!("<blue><b>Canonical Provider:</b></blue> {user_part}, {repo_part}")
    } else {
        format!("<blue><b>Canonical Provider:</b></blue> {user_part}")
    };

    log::data(&Prose::new(line).render(term));
    log::data("");
}
```

Note the label changes from plural "Canonical Providers" to singular since `ClaudineConfig` uses a single provider, not per-resource slots.

## Phase 4 — Cut Legacy Consumers (Finding 1)

Switch remaining CLI commands from `load_config()` to `load_claudine_config()` or `compile_canonical_runtime()`.

### 4.1 `actions.rs` — switch to `ClaudineConfig`

**File:** `claudine/cli/src/commands/actions.rs:20`

Replace `load_config(None, None)` with `load_claudine_config(None, None)`. The `actions.rs` display renders events and their actions — `ClaudineConfig.actions` is a `HashMap<AgenticEvent, Vec<HookAction>>` which is actually simpler to iterate than the per-provider structure.

The `run_verbose` and `run_simple` functions will need adjustment to iterate `config.actions` instead of `config.providers[*].events[*]`. The output format becomes simpler since actions are event-centric, not provider-centric.

### 4.2 `sync.rs` — switch to `ClaudineConfig`

**File:** `claudine/cli/src/commands/sync.rs:182`

`sync` needs provider event bindings to know which hooks to register. With `ClaudineConfig`, the `actions` map is provider-agnostic (every event applies to all providers). So sync should:

1. Load `ClaudineConfig` via `load_claudine_config()`
2. Use `compile_canonical_runtime()` to get compiled event bindings
3. Register hooks for each installed provider based on the canonical runtime

### 4.3 `hooks.rs` — switch to `ClaudineConfig`

**File:** `claudine/cli/src/commands/hooks.rs:629`

Similar to `sync.rs`. The `hooks` command displays registered hooks and validates sound effects. Switch to `load_claudine_config()` and adjust the rendering to iterate the flat actions map.

### 4.4 Remove dead `init` import

**File:** `claudine/cli/src/commands/mod.rs:9`

Remove the old `init` module if it's no longer used. The init wizard has replaced it.

## Phase 5 — Tests (Finding 10)

### 5.1 Unit tests for TUI write-path fixes

Add tests in `claudine/cli/src/commands/config_tui/` (or a test module alongside the tabs) that verify:

- Adding a sound effect action produces a valid `playa::SoundEffect` name
- Setting a single gendered voice produces `VoiceSelection::Single`, not a placeholder
- Creating a messenger config without filling required fields does NOT set `active_config`
- The `ClaudineConfig` produced by these operations passes `validate()`

These can be pure unit tests on the config data structures — they don't need the full TUI/terminal.

### 5.2 Integration test: `preferred_agent` honored by composition

Add a test in `claudine/cli/tests/` that:

1. Writes a `ClaudineConfig` with `preferred_agent: Provider::Codex` to a temp path
2. Calls the provider selection logic with no explicit provider
3. Asserts that `Codex` is selected as the favorite

### 5.3 Integration test: repo config migration

Add a test that:

1. Writes an old-format config to a temp repo `.claudine/config.json`
2. Calls `load_claudine_config()` with that repo root
3. Asserts the old config was backed up to `.bak`
4. Asserts the load falls back cleanly (no panic, returns valid config or expected error)

### 5.4 Validation round-trip test

Add a test that constructs a `ClaudineConfig` with each type of invalid data (empty channel_id, placeholder voice, invalid sound name) and asserts that `validate()` rejects each one.

## Phase 6 — Design Recommendations (Review Recommendations)

### 6.1 Extract TUI reducers

Move state-mutating logic out of the modal key handlers into pure functions:

```rust
// Pure function: returns the new action to insert
fn create_default_sound_action(event: AgenticEvent) -> HookAction { ... }

// Pure function: returns the new voice selection
fn apply_voice_selection(current: Option<&VoiceSelection>, gender: GenderTab, voice: String) -> VoiceSelection { ... }

// Pure function: returns the new messenger config entry
fn create_messenger_config(provider: &str) -> Option<MessengerProviderConfig> { ... }
```

These are trivially testable without any App/terminal state.

### 6.2 Narrow configurator type

Replace `HookerConfig` usage in the provider configurator layer with a focused type:

```rust
pub struct ProviderHookPlan {
    pub events: Vec<AgenticEvent>,
    pub canonical_for: Option<ResourceScope>,
}
```

This captures only what configurators need: which events to register and whether this provider is canonical. No global settings, no linking preferences.

### 6.3 Cache provider discovery in TUI

In `App::new()`, call `discover_agents_full()` once and store the result. Pass it through to tabs that need provider lists instead of recomputing.

## Execution Order Summary

| Phase | Findings | Risk | Dependencies |
|-------|----------|------|--------------|
| 1     | 5, 8, 9  | Low  | None         |
| 2     | 6, 7     | Low  | None         |
| 3     | 2, 3, 4  | Medium | Phases 1-2 |
| 4     | 1        | Medium | Phase 3    |
| 5     | 10       | Low  | Phases 1-4   |
| 6     | Recs     | Low  | Phases 1-4   |

Phases 1 and 2 are fully independent and can be done in parallel. Phase 3 is the core migration. Phase 4 extends it. Phases 5 and 6 can be parallelized.
