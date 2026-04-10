# Refactor Config Review 2 Fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all broken functionality, incomplete implementations, and ergonomic issues identified in the refactor-config code review (review-2.md).

**Architecture:** Nine discrete fixes across the CLI handle command, dispatch pipeline, TUI config, init wizard, and bash action executor. Each task is independent except Task 2 (logging) which depends on Task 1 (handle fix) being merged first so the canonical path is exercised.

**Tech Stack:** Rust, ratatui, biscuit-speaks, tokio, serde_json, claudine lib/cli

---

### Task 1: Wire `claudine handle` to the canonical dispatch path

The `handle` command currently calls `claudine::dispatch::dispatch()` which uses the legacy `dispatch_preparsed` → `load_runtime_config` path. This crashes with new `ClaudineConfig` format because `load_runtime_config` expects the old `HookerConfig`.

**Files:**
- Modify: `claudine/cli/src/commands/handle.rs:40`

- [ ] **Step 1: Write a test verifying canonical dispatch is called**

Add a test to `handle.rs` that validates the dispatch path used. Since we can't easily integration-test the full stdin pipeline, we verify by checking that the function signature compiles with the canonical call.

Actually, the existing tests in `handle.rs` only test `resolve_provider_inner` which is unaffected. The fix is a one-line change, so we'll change it and verify by building.

- [ ] **Step 2: Switch dispatch call from legacy to canonical**

In `claudine/cli/src/commands/handle.rs`, change line 40 from:

```rust
    let outcome = claudine::dispatch::dispatch(&raw, provider, &env).await?;
```

to:

```rust
    let outcome = claudine::dispatch::dispatch_canonical(&raw, provider, &env).await?;
```

- [ ] **Step 3: Build and verify**

Run: `cargo build -p claudine-cli`
Expected: Compiles cleanly.

- [ ] **Step 4: Run existing tests**

Run: `cargo test -p claudine-cli`
Expected: All tests pass. The `resolve_provider_*` tests are unaffected.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/handle.rs
git commit -m "fix(claudine-cli): wire handle command to canonical dispatch path

The handle command was calling dispatch() which uses the legacy
HookerConfig/RuntimeConfig path. This crashes when a new ClaudineConfig
is present because load_runtime_config cannot parse the new format.
Switch to dispatch_canonical() which loads and compiles ClaudineConfig."
```

---

### Task 2: Restore JSONL event logging in canonical dispatch

The `HookAction::Log` variant was removed in favor of a `logging: bool` toggle in `ClaudineConfig`, but `dispatch_canonical_with_runtime` never writes JSONL events. This breaks the reporting/analytics pipeline.

**Files:**
- Modify: `claudine/lib/src/dispatch/mod.rs` (the `dispatch_canonical_with_runtime` function, around line 200-347)

- [ ] **Step 1: Write a test for canonical dispatch logging**

Create an integration test in `claudine/lib/src/dispatch/mod.rs` (in the existing `#[cfg(test)]` module if one exists, otherwise add one) that verifies when `logging: true`, `dispatch_canonical_with_runtime` writes a JSONL event.

Since `write_summary_event` writes to a real filesystem path and we don't want to touch `~/.claudine/logs/`, we'll test the logging function separately. The actual integration is a simple conditional call. Write the test for the helper we'll create:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventMeta, Provider};
    use tempfile::TempDir;

    #[test]
    fn log_dispatch_event_writes_jsonl_line() {
        let tmp = TempDir::new().unwrap();
        let log_path = tmp.path().join("test.jsonl");

        let meta = EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::SessionStart,
            timestamp: chrono::Utc::now(),
            session_id: Some("test-sess".into()),
            ..Default::default()
        };

        write_dispatch_event_to(&meta, &log_path).unwrap();

        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("test-sess"));
        assert!(content.ends_with('\n'));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p claudine -- tests::log_dispatch_event_writes_jsonl_line`
Expected: FAIL — `write_dispatch_event_to` does not exist yet.

- [ ] **Step 3: Add the logging helper function**

In `claudine/lib/src/dispatch/mod.rs`, add a helper function that writes an `EventMeta` to a JSONL file. This mirrors `stream::reporting::write_summary_event` but takes an explicit path for testability, and also add a convenience wrapper that resolves the default path:

```rust
use std::io::Write;

/// Write a dispatch event to the daily-rotated JSONL log.
///
/// Called by canonical dispatch when `config.logging` is enabled.
fn log_dispatch_event(meta: &EventMeta) {
    if let Err(e) = log_dispatch_event_inner(meta) {
        tracing::warn!(%e, "Failed to write dispatch event log");
    }
}

fn log_dispatch_event_inner(meta: &EventMeta) -> std::result::Result<(), std::io::Error> {
    let path = crate::reporting::paths::resolve_file_log_path(None, true)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    write_dispatch_event_to(meta, &path)
}

fn write_dispatch_event_to(
    meta: &EventMeta,
    path: &std::path::Path,
) -> std::result::Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(meta)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p claudine -- tests::log_dispatch_event_writes_jsonl_line`
Expected: PASS.

- [ ] **Step 5: Wire logging into dispatch_canonical_with_runtime**

In the `dispatch_canonical_with_runtime` function, add a logging call after the action execution block (around line 314, after `execute_actions_v2` returns). Insert before the protect post-evaluation:

```rust
    // --- JSONL event logging ---
    if runtime.config().logging {
        log_dispatch_event(&resolved_hook.meta);
    }
```

- [ ] **Step 6: Build and test**

Run: `cargo build -p claudine && cargo test -p claudine`
Expected: All pass.

- [ ] **Step 7: Commit**

```bash
git add claudine/lib/src/dispatch/mod.rs
git commit -m "fix(claudine): restore JSONL event logging in canonical dispatch

The HookAction::Log variant was removed in favor of a logging: bool
toggle in ClaudineConfig, but dispatch_canonical_with_runtime never
wrote events. Add log_dispatch_event() that writes EventMeta to
daily-rotated JSONL files in ~/.claudine/logs/ when logging is enabled."
```

---

### Task 3: Add repo-scoped configuration support to the TUI

The config TUI only loads/saves user-scoped config (`~/.claudine/config.json`). It needs to track repo-scoped config separately when `is_in_repo` is true, and save repo-scoped changes to the local `.claudine/config.json`.

**Files:**
- Modify: `claudine/cli/src/commands/config_tui/mod.rs:27-62`
- Modify: `claudine/cli/src/commands/config_tui/app.rs:51-61,146-157`
- Modify: `claudine/cli/src/commands/config_tui/tabs/preferences.rs:243-278`

- [ ] **Step 1: Add repo config fields to App state**

In `claudine/cli/src/commands/config_tui/app.rs`, add fields to track repo-scoped config:

```rust
pub struct App {
    pub mode: AppMode,
    pub focused_tab: Tab,
    pub selected_tab: Option<Tab>,
    pub config: ClaudineConfig,           // user-scope config
    pub repo_config: Option<ClaudineConfig>, // repo-scope config (None if not in repo)
    pub repo_config_path: Option<std::path::PathBuf>, // path to repo .claudine/config.json
    pub is_in_repo: bool,
    pub should_quit: bool,
    pub dirty: bool,
    pub repo_dirty: bool,                // repo config modified
    pub list_index: usize,
    pub modal: Option<ModalState>,
}
```

Update `App::new` to accept optional repo config:

```rust
pub fn new(
    config: ClaudineConfig,
    repo_config: Option<ClaudineConfig>,
    repo_config_path: Option<std::path::PathBuf>,
    is_in_repo: bool,
) -> Self {
    Self {
        mode: AppMode::Overview,
        focused_tab: Tab::Preferences,
        selected_tab: None,
        config,
        repo_config,
        repo_config_path,
        is_in_repo,
        should_quit: false,
        dirty: false,
        repo_dirty: false,
        list_index: 0,
        modal: None,
    }
}
```

- [ ] **Step 2: Update TUI entry point to load repo config**

In `claudine/cli/src/commands/config_tui/mod.rs`, update the `run` function to detect repo root, load repo config, and save both on exit:

```rust
pub async fn run(_args: ConfigArgs) -> color_eyre::Result<()> {
    let config_path = claudine::dispatch::loader::user_config_path();
    let config = claudine::dispatch::loader::load_claudine_config(Some(&config_path), None)?;

    let cwd = std::env::current_dir()?;
    let git_info = sniff::filesystem::git::detect_git(&cwd, false, 1).ok().flatten();
    let is_in_repo = git_info.is_some();

    let (repo_config, repo_config_path) = if let Some(ref git) = git_info {
        let repo_root = &git.repo_root;
        let repo_cfg_path = repo_root.join(".claudine").join("config.json");
        let repo_cfg = if repo_cfg_path.exists() {
            claudine::dispatch::loader::load_claudine_config(Some(&repo_cfg_path), None).ok()
        } else {
            None
        };
        (repo_cfg, Some(repo_cfg_path))
    } else {
        (None, None)
    };

    let mut app = App::new(config, repo_config, repo_config_path.clone(), is_in_repo);

    // ... terminal setup unchanged ...

    if app.dirty {
        claudine::dispatch::loader::save_claudine_config(&app.config, &config_path)?;
        eprintln!("Configuration saved to {}", config_path.display());
    }
    if app.repo_dirty {
        if let Some(ref path) = repo_config_path {
            if let Some(ref repo_cfg) = app.repo_config {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                claudine::dispatch::loader::save_claudine_config(repo_cfg, path)?;
                eprintln!("Repo configuration saved to {}", path.display());
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Fix repo provider modal to write to repo config**

In `claudine/cli/src/commands/config_tui/tabs/preferences.rs`, the `handle_repo_provider_modal` currently delegates to `handle_user_provider_modal`, which writes to `app.config.canonical_provider` (user scope). Fix it to write to repo config instead:

```rust
pub fn handle_repo_provider_modal(app: &mut App, key: KeyEvent) {
    let providers = super::super::get_provider_list();
    let count = providers.len() + 1;
    match key.code {
        KeyCode::Up => {
            let idx = app.modal_highlighted();
            if idx > 0 {
                app.set_modal_highlighted(idx - 1);
            }
        }
        KeyCode::Down => {
            let idx = app.modal_highlighted();
            if idx + 1 < count {
                app.set_modal_highlighted(idx + 1);
            }
        }
        KeyCode::Enter => {
            let idx = app.modal_highlighted();
            let selected = if idx == 0 {
                None
            } else {
                providers.get(idx - 1).copied()
            };
            // Ensure repo config exists
            if app.repo_config.is_none() {
                app.repo_config = Some(ClaudineConfig::default());
            }
            if let Some(ref mut repo_cfg) = app.repo_config {
                repo_cfg.canonical_provider = selected;
            }
            app.repo_dirty = true;
            app.modal = None;
        }
        KeyCode::Esc => {
            app.modal = None;
        }
        _ => {}
    }
}
```

Also update the render for Repo Provider in `preferences.rs` to show the repo config value:

Replace the repo_line rendering block (lines 64-79) to read from `app.repo_config`:

```rust
    if app.is_in_repo {
        let repo_provider = app
            .repo_config
            .as_ref()
            .and_then(|c| c.canonical_provider)
            .map(|p| p.to_string())
            .unwrap_or_else(|| "(not set)".to_string());
        let repo_line = Paragraph::new(Line::from(vec![
            Span::styled(
                "Repo Provider",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(": "),
            Span::styled(
                format!("[{repo_provider}]"),
                if is_detail {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                },
            ),
        ]));
        frame.render_widget(repo_line, chunks[2]);
    } else {
        // ... unchanged ...
    }
```

- [ ] **Step 4: Update all App::new call sites**

The `App::new()` signature changed. Update:
- `config_tui/mod.rs` — already handled in step 2
- `messenger.rs` tests `test_app()` function (line 480):

```rust
fn test_app() -> App {
    let mut app = App::new(ClaudineConfig::default(), None, None, false);
    app.mode = AppMode::Detail;
    app
}
```

- [ ] **Step 5: Build and test**

Run: `cargo build -p claudine-cli && cargo test -p claudine-cli`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add claudine/cli/src/commands/config_tui/
git commit -m "fix(claudine-cli): add repo-scoped config support to TUI

The config TUI previously only loaded/saved user-scoped config. Now
it loads repo config separately when in a git repo, renders repo
provider from the repo config, and saves repo changes to the local
.claudine/config.json file."
```

---

### Task 4: Implement TTS voice listing in TUI

`get_tts_voices()` returns an empty vec. It should query available voices from the selected TTS provider.

**Files:**
- Modify: `claudine/cli/src/commands/config_tui/mod.rs:148-150`
- Modify: `claudine/cli/src/commands/config_tui/app.rs` (add cached_voices field)
- Modify: `claudine/cli/src/commands/config_tui/tabs/tts.rs:88,181`

- [ ] **Step 1: Add cached voices to App state**

In `app.rs`, add a field to cache voice names:

```rust
pub struct App {
    // ... existing fields ...
    pub cached_voices: Vec<String>,
}
```

Initialize to empty in `App::new`:
```rust
cached_voices: Vec::new(),
```

- [ ] **Step 2: Implement voice listing by provider**

In `config_tui/mod.rs`, replace the stub `get_tts_voices()` with a function that queries the system based on the selected provider. Since the TUI is synchronous, use `std::process::Command`:

```rust
fn query_voices_for_provider(provider: &str) -> Vec<String> {
    match provider {
        "say" => query_say_voices(),
        "espeak-ng" => query_espeak_voices("espeak-ng"),
        "espeak" => query_espeak_voices("espeak"),
        _ => vec![],
    }
}

fn query_say_voices() -> Vec<String> {
    let output = match std::process::Command::new("say").arg("-v").arg("?").output() {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    if !output.status.success() {
        return vec![];
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let name = line.split_whitespace().next()?;
            Some(name.to_string())
        })
        .collect()
}

fn query_espeak_voices(binary: &str) -> Vec<String> {
    let output = match std::process::Command::new(binary).arg("--voices").output() {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    if !output.status.success() {
        return vec![];
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .skip(1) // header line
        .filter_map(|line| {
            // Format: Pty Language Age/Gender VoiceName ...
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.get(3).map(|s| s.to_string())
        })
        .collect()
}
```

- [ ] **Step 3: Populate cached voices when opening voice selector modal**

In `tts.rs`, when the user presses `f` or `m` to open the voice selector, populate the cache:

```rust
        KeyCode::Char('f') => {
            let provider = match &app.config.tts {
                TtsValue::Config(cfg) => cfg.provider.clone(),
                _ => "auto".to_string(),
            };
            app.cached_voices = super::super::query_voices_for_provider(&provider);
            app.modal = Some(ModalState::VoiceSelector {
                gender: GenderTab::Female,
                highlighted: 0,
            });
        }
        KeyCode::Char('m') => {
            let provider = match &app.config.tts {
                TtsValue::Config(cfg) => cfg.provider.clone(),
                _ => "auto".to_string(),
            };
            app.cached_voices = super::super::query_voices_for_provider(&provider);
            app.modal = Some(ModalState::VoiceSelector {
                gender: GenderTab::Male,
                highlighted: 0,
            });
        }
```

- [ ] **Step 4: Use cached voices in render and handler**

In `tts.rs`, update the voice selector modal rendering (around line 88) to use `app.cached_voices` instead of `get_tts_voices()`:

```rust
    if let Some(ModalState::VoiceSelector { gender: _, highlighted }) = &app.modal {
        let mut items = vec!["(auto)".to_string()];
        items.extend(app.cached_voices.iter().cloned());
        super::super::widgets::modal::render_list_modal(
            frame, area, "Select Voice", &items, *highlighted,
        );
    }
```

Similarly, update `handle_voice_selector_modal` (around line 181) to use `app.cached_voices`:

```rust
    let voices = &app.cached_voices;
    let count = voices.len() + 1;
```

And update the Enter handler to use `voices[idx - 1].clone()` instead of `voices[idx - 1].to_string()`.

- [ ] **Step 5: Remove the old get_tts_voices function**

Delete the now-unused `get_tts_voices()` function from `config_tui/mod.rs`.

- [ ] **Step 6: Build and test**

Run: `cargo build -p claudine-cli && cargo test -p claudine-cli`
Expected: All pass.

- [ ] **Step 7: Commit**

```bash
git add claudine/cli/src/commands/config_tui/
git commit -m "feat(claudine-cli): implement TTS voice listing in config TUI

Replace the stubbed get_tts_voices() with query_voices_for_provider()
that queries say/espeak-ng/espeak for available voices. Voices are
cached in App state when the voice selector modal opens."
```

---

### Task 5: Add messenger configuration to initialization wizard

The init wizard skips the messenger configuration step entirely.

**Files:**
- Modify: `claudine/cli/src/commands/init_wizard.rs:40-81`

- [ ] **Step 1: Add configure_messenger function**

Add a messenger configuration step to `init_wizard.rs`:

```rust
fn configure_messenger() -> Result<Option<ClaudineMessengerConfig>> {
    log::message("");
    log::message("  Messenger");
    log::message("  Claudine can send notifications via Discord, Slack, Signal, or WhatsApp.");
    log::message("");

    let setup_now = inquire::Confirm::new("  Would you like to configure a messenger now?")
        .with_default(false)
        .prompt()?;

    if !setup_now {
        return Ok(None);
    }

    let providers = ["Discord", "Slack", "Signal", "WhatsApp"];
    let selection = inquire::Select::new("  Select messenger provider:", providers.to_vec()).prompt()?;

    let (name, config) = match selection {
        "Discord" => (
            "discord".to_string(),
            MessengerProviderConfig::Discord {
                channel_id: inquire::Text::new("  Discord channel ID:").prompt()?,
                bot_token_env: inquire::Text::new("  Bot token env var:")
                    .with_default("DISCORD_BOT_TOKEN")
                    .prompt()?,
            },
        ),
        "Slack" => (
            "slack".to_string(),
            MessengerProviderConfig::Slack {
                channel_id: inquire::Text::new("  Slack channel ID:").prompt()?,
                bot_token_env: inquire::Text::new("  Bot token env var:")
                    .with_default("SLACK_BOT_TOKEN")
                    .prompt()?,
            },
        ),
        "Signal" => (
            "signal".to_string(),
            MessengerProviderConfig::Signal {
                recipient: inquire::Text::new("  Signal recipient:").prompt()?,
                rpc_url_env: inquire::Text::new("  RPC URL env var:")
                    .with_default("SIGNAL_RPC_URL")
                    .prompt()?,
                account_env: inquire::Text::new("  Account env var:")
                    .with_default("SIGNAL_ACCOUNT")
                    .prompt()?,
            },
        ),
        "WhatsApp" => (
            "whatsapp".to_string(),
            MessengerProviderConfig::Whatsapp {
                recipient: inquire::Text::new("  WhatsApp recipient:").prompt()?,
                access_token_env: inquire::Text::new("  Access token env var:")
                    .with_default("WHATSAPP_ACCESS_TOKEN")
                    .prompt()?,
                phone_number_id_env: inquire::Text::new("  Phone number ID env var:")
                    .with_default("WHATSAPP_PHONE_NUMBER_ID")
                    .prompt()?,
            },
        ),
        _ => return Ok(None),
    };

    let mut configurations = std::collections::HashMap::new();
    configurations.insert(name.clone(), config);

    Ok(Some(ClaudineMessengerConfig {
        active_config: Some(name),
        configurations,
    }))
}
```

- [ ] **Step 2: Add the necessary import**

Add the import at the top of `init_wizard.rs`:

```rust
use claudine::config::claudine_config::{
    ClaudineConfig, ClaudineMessengerConfig, DefaultSounds, MessengerProviderConfig, TtsValue,
};
```

- [ ] **Step 3: Wire messenger into the interactive flow**

In `run_interactive_initialization`, add the messenger step after the services explanation and before the actions section. Insert after the `Press Enter to continue` confirm (around line 59):

```rust
    let messenger = configure_messenger()?;
```

Then pass it to `build_config`:

```rust
    let config = build_config(tts, preferred_agent, messenger);
```

- [ ] **Step 4: Update build_config to accept messenger**

```rust
fn build_config(tts: TtsValue, preferred_agent: Provider, messenger: Option<ClaudineMessengerConfig>) -> ClaudineConfig {
    // ... existing actions setup ...

    ClaudineConfig {
        tts,
        messenger,
        logging: true,
        // ... rest unchanged ...
    }
}
```

Also update `build_default_config` to pass `None`:

```rust
fn build_default_config() -> ClaudineConfig {
    // ... existing agent detection ...
    build_config(TtsValue::Boolean(has_tts), preferred_agent, None)
}
```

- [ ] **Step 5: Build and test**

Run: `cargo build -p claudine-cli && cargo test -p claudine-cli`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add claudine/cli/src/commands/init_wizard.rs
git commit -m "feat(claudine-cli): add messenger configuration to init wizard

The initialization wizard now offers messenger setup (Discord, Slack,
Signal, WhatsApp) as part of the interactive flow, matching the spec."
```

---

### Task 6: Filter TUI provider lists by installed agents

The Preferences tab's user-scoped canonical provider modal shows all 8 providers. It should only show agents that are actually installed.

**Files:**
- Modify: `claudine/cli/src/commands/config_tui/mod.rs:126-128`
- Modify: `claudine/cli/src/commands/config_tui/tabs/preferences.rs:131-141,243-273`

- [ ] **Step 1: Add a filtered provider list function**

In `config_tui/mod.rs`, add a function that filters providers by availability:

```rust
fn get_available_providers() -> Vec<claudine::events::Provider> {
    let agents = claudine::config::discover_agents_full();
    agents
        .iter()
        .filter(|a| a.is_available())
        .map(|a| a.provider)
        .collect()
}
```

- [ ] **Step 2: Use filtered list in user provider modal**

In `preferences.rs`, update `handle_user_provider_modal` to use `get_available_providers()` instead of `get_provider_list()`:

```rust
pub fn handle_user_provider_modal(app: &mut App, key: KeyEvent) {
    let providers = super::super::get_available_providers();
    // ... rest unchanged ...
}
```

Also update the render for `UserProviderSelector` modal:

```rust
    if let Some(ModalState::UserProviderSelector { highlighted }) = &app.modal {
        let providers = super::super::get_available_providers();
        // ... rest unchanged ...
    }
```

- [ ] **Step 3: Build and test**

Run: `cargo build -p claudine-cli && cargo test -p claudine-cli`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add claudine/cli/src/commands/config_tui/
git commit -m "fix(claudine-cli): filter provider list by installed agents

The user-scoped canonical provider selector now only shows providers
that are actually installed on the host, using discover_agents_full()
filtered by is_available()."
```

---

### Task 7: Fix Bash action parameter interpolation

`execute_bash` splits interpolated params by whitespace then individually escapes each piece. This breaks single arguments that contain spaces (e.g., a git commit message).

**Files:**
- Modify: `claudine/lib/src/dispatch/runner.rs:560-605`

- [ ] **Step 1: Write a test for params with spaces**

Add a test in `runner.rs` (or a new test module) that verifies params containing spaces are preserved as single arguments. Since `execute_bash` is a private function that spawns a process, we'll test the escaping logic directly:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_params_preserves_spaces() {
        let params = "hello world with spaces";
        let escaped = bash_executor::shell_escape(params);
        assert_eq!(escaped, "'hello world with spaces'");
    }
}
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `cargo test -p claudine -- dispatch::runner::tests::escape_params_preserves_spaces`
Expected: PASS — `shell_escape` already handles this correctly. The bug is in the *splitting* before escaping.

- [ ] **Step 3: Fix execute_bash to escape the whole param string**

In `runner.rs`, change the `execute_bash` function's parameter handling. Replace the whitespace-splitting logic (lines 572-580):

```rust
    let escaped_params = if rendered_params.is_empty() {
        String::new()
    } else {
        rendered_params
            .split_whitespace()
            .map(bash_executor::shell_escape)
            .collect::<Vec<_>>()
            .join(" ")
    };
```

With whole-string escaping:

```rust
    let escaped_params = if rendered_params.is_empty() {
        String::new()
    } else {
        bash_executor::shell_escape(&rendered_params)
    };
```

This treats the entire interpolated `params` field as a single argument, which is the correct behavior since `params` is defined as a single string in the config schema.

- [ ] **Step 4: Build and test**

Run: `cargo build -p claudine && cargo test -p claudine`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/dispatch/runner.rs
git commit -m "fix(claudine): preserve spaces in Bash action params

execute_bash was splitting interpolated params by whitespace before
shell-escaping each piece, which broke single arguments containing
spaces (e.g., git commit messages). Now the entire params string is
escaped as one argument."
```

---

### Task 8: Fix Messenger tab keyboard navigation

The Messenger tab uses custom keybindings (`s`, `a`, `e`) instead of `Tab`/`Shift-Tab` for focus management.

**Files:**
- Modify: `claudine/cli/src/commands/config_tui/tabs/messenger.rs:60-63,138-161`
- Modify: `claudine/cli/src/commands/config_tui/app.rs` (add focus_index to App or Messenger state)

- [ ] **Step 1: Add focus index for Messenger tab**

In `app.rs`, add a field for messenger focus tracking:

```rust
pub struct App {
    // ... existing fields ...
    pub messenger_focus: usize,  // 0 = select active, 1 = add new, 2 = edit active
}
```

Initialize to 0 in `App::new`.

- [ ] **Step 2: Update messenger key handler for Tab/Shift-Tab**

In `messenger.rs`, replace the `handle_key` function to use `Tab`/`Shift-Tab` for focus cycling and `Enter` to activate the focused item:

```rust
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            app.messenger_focus = (app.messenger_focus + 1) % 3;
        }
        KeyCode::BackTab => {
            app.messenger_focus = (app.messenger_focus + 2) % 3; // wraps backwards
        }
        KeyCode::Enter => {
            match app.messenger_focus {
                0 => {
                    app.modal = Some(ModalState::MessengerSelect { highlighted: 0 });
                }
                1 => {
                    app.modal = Some(ModalState::MessengerAdd { highlighted: 0 });
                }
                2 => {
                    if let Some(name) = app
                        .config
                        .messenger
                        .as_ref()
                        .and_then(|m| m.active_config.clone())
                    {
                        app.modal = Some(ModalState::MessengerEdit {
                            config_name: name,
                            field_index: 0,
                        });
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 3: Update messenger render to show focus state**

In the `render` function of `messenger.rs`, update the styling to highlight the focused element. Replace the `select_line` construction (lines 27-47) to highlight based on `app.messenger_focus`:

```rust
    let select_style = if is_detail && app.messenger_focus == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else if is_detail {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let add_style = if is_detail && app.messenger_focus == 1 {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else if is_detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let select_line = Line::from(vec![
        Span::styled("Active", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": "),
        Span::styled(format!("[{active_name}]"), select_style),
        Span::raw("  "),
        Span::styled("[+ Add]", add_style),
    ]);
```

Update the help line to show the new keybindings:

```rust
    if is_detail && app.modal.is_none() {
        let edit_label = if app.messenger_focus == 2 { "> Edit" } else { "  Edit" };
        let help = Paragraph::new(format!(
            " Tab/Shift-Tab: focus | Enter: activate | {edit_label} active config"
        ))
        .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[2]);
    }
```

- [ ] **Step 4: Build and test**

Run: `cargo build -p claudine-cli && cargo test -p claudine-cli`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add claudine/cli/src/commands/config_tui/
git commit -m "fix(claudine-cli): use Tab/Shift-Tab navigation in Messenger tab

Replace custom s/a/e keybindings with standard Tab/Shift-Tab focus
cycling and Enter to activate the focused element (select, add, edit)."
```

---

### Task 9: Add tests for reporting ingestion and canonical dispatch

The review flagged zero unit tests in `reporting/ingest.rs` and severe lack of canonical dispatch integration tests.

**Files:**
- Modify: `claudine/lib/src/reporting/ingest.rs`
- Create: `claudine/lib/tests/canonical_dispatch.rs` (integration test)

- [ ] **Step 1: Add ingestion unit tests**

In `claudine/lib/src/reporting/ingest.rs`, add a `#[cfg(test)]` module with tests for the core ingestion logic. Check what functions are already tested and what's missing. Add tests for:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_event_line(session_id: &str, event: &str) -> String {
        serde_json::json!({
            "provider": "claude",
            "event": event,
            "timestamp": "2026-04-07T12:00:00Z",
            "session_id": session_id,
            "extra": {}
        })
        .to_string()
    }

    #[test]
    fn ingest_single_event_from_jsonl() {
        let tmp = TempDir::new().unwrap();
        let log_path = tmp.path().join("2026-04-07.jsonl");
        let db_path = tmp.path().join("metrics.db");

        std::fs::write(&log_path, make_test_event_line("sess-1", "session_start") + "\n").unwrap();

        let store = crate::reporting::ReportingStore::open(&db_path).unwrap();
        store.ingest_logs(tmp.path()).unwrap();

        let events = store.query_events_today().unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn ingest_incremental_skips_already_processed_lines() {
        let tmp = TempDir::new().unwrap();
        let log_path = tmp.path().join("2026-04-07.jsonl");
        let db_path = tmp.path().join("metrics.db");

        // Write one event and ingest
        std::fs::write(
            &log_path,
            make_test_event_line("sess-1", "session_start") + "\n",
        )
        .unwrap();
        let store = crate::reporting::ReportingStore::open(&db_path).unwrap();
        store.ingest_logs(tmp.path()).unwrap();

        // Append a second event and re-ingest
        let mut f = std::fs::OpenOptions::new().append(true).open(&log_path).unwrap();
        use std::io::Write;
        writeln!(f, "{}", make_test_event_line("sess-1", "session_end")).unwrap();
        store.ingest_logs(tmp.path()).unwrap();

        let events = store.query_events_today().unwrap();
        assert_eq!(events.len(), 2);
    }
}
```

Note: The exact test code will depend on the public API of `ReportingStore`. Adapt the function names and assertions to match what's actually available. If `query_events_today` doesn't exist, use whatever query method the store exposes.

- [ ] **Step 2: Run ingestion tests**

Run: `cargo test -p claudine -- reporting::ingest::tests`
Expected: PASS (adjust function names if needed).

- [ ] **Step 3: Add canonical dispatch integration test**

Create `claudine/lib/tests/canonical_dispatch.rs`:

```rust
use std::collections::HashMap;

use claudine::actions::HookAction;
use claudine::config::claudine_config::{ClaudineConfig, DefaultSounds, TtsValue};
use claudine::dispatch::loader::{compile_canonical_runtime, CanonicalRuntimeConfig};
use claudine::dispatch::{dispatch_canonical_with_runtime, DispatchOutcome};
use claudine::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};
use claudine::services::protect::config::ProtectConfig;

fn make_config_with_action(event: AgenticEvent, action: HookAction) -> CanonicalRuntimeConfig {
    let mut actions = HashMap::new();
    actions.insert(event, vec![action]);

    let config = ClaudineConfig {
        tts: TtsValue::Boolean(false),
        messenger: None,
        logging: false,
        protect: ProtectConfig::default(),
        actions,
        preferred_agent: Provider::Claude,
        canonical_provider: None,
        default_sounds: DefaultSounds::default(),
    };

    compile_canonical_runtime(config, None).unwrap()
}

fn make_meta(event: AgenticEvent) -> EventMeta {
    EventMeta {
        provider: Provider::Claude,
        event,
        timestamp: chrono::Utc::now(),
        ..Default::default()
    }
}

#[tokio::test]
async fn dispatch_sound_effect_action() {
    let runtime = make_config_with_action(
        AgenticEvent::HumanInTheLoop,
        HookAction::SoundEffect {
            effect: "confirmation".to_string(),
            volume: 0.0, // silent for tests
            speed: 1.0,
        },
    );

    let outcome = dispatch_canonical_with_runtime(
        Provider::Claude,
        AgenticEvent::HumanInTheLoop,
        make_meta(AgenticEvent::HumanInTheLoop),
        &runtime,
    )
    .await
    .unwrap();

    // SoundEffect is fire-and-forget, no blocking response
    assert_eq!(outcome.response, None);
    assert_eq!(outcome.exit_code, None);
}

#[tokio::test]
async fn dispatch_no_binding_returns_default_outcome() {
    let runtime = make_config_with_action(
        AgenticEvent::HumanInTheLoop,
        HookAction::SoundEffect {
            effect: "confirmation".to_string(),
            volume: 1.0,
            speed: 1.0,
        },
    );

    // Dispatch an event with no configured binding
    let outcome = dispatch_canonical_with_runtime(
        Provider::Claude,
        AgenticEvent::SessionStart,
        make_meta(AgenticEvent::SessionStart),
        &runtime,
    )
    .await
    .unwrap();

    assert_eq!(outcome, DispatchOutcome::default());
}
```

- [ ] **Step 4: Run integration tests**

Run: `cargo test -p claudine --test canonical_dispatch`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/reporting/ingest.rs claudine/lib/tests/canonical_dispatch.rs
git commit -m "test(claudine): add reporting ingestion and canonical dispatch tests

Add unit tests for JSONL ingestion covering single-event and
incremental processing. Add integration tests for canonical dispatch
verifying sound effect execution and missing-binding handling."
```
