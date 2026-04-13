# Config Refactor Review 3 Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement all 6 findings from `claudine/features/2026-04-7-refactor-config/review-3.md` to fix service/action coupling, add repo messenger overrides, fix modal staging, constrain headless init, document the bash escaping contract, and wire default_sounds into runtime.

**Architecture:** The dispatch pipeline is restructured so services (logging, protect_post) run independently of action bindings. The TUI gains staged modal state for cancelable dialogs. RepoOverrideConfig grows a single `active_messenger` field. The bash action contract is documented and tested. Default sounds are wired into dispatch outcomes.

**Tech Stack:** Rust, ratatui (TUI), serde, playa (sound effects), tokio, shell_words

---

## File Map

| File | Change | Purpose |
|------|--------|---------|
| `claudine/lib/src/dispatch/mod.rs` | Modify | Decouple logging/protect_post from binding lookup |
| `claudine/lib/src/config/claudine_config.rs` | Modify | Add `active_messenger` to `RepoOverrideConfig` |
| `claudine/lib/src/dispatch/loader.rs` | Modify | Merge repo `active_messenger` override |
| `claudine/cli/src/commands/config_tui/app.rs` | Modify | Add staged rules to `ProtectRules` modal variant |
| `claudine/cli/src/commands/config_tui/tabs/services.rs` | Modify | Stage rule toggles, commit on Enter, discard on Esc |
| `claudine/cli/src/commands/config_tui/tabs/messenger.rs` | Modify | Show repo messenger override when in repo mode |
| `claudine/cli/src/commands/init_wizard.rs` | Modify | Remove hook registration from headless path |
| `claudine/lib/src/dispatch/runner.rs` | Modify | Document bash escaping contract, add `execute_default_sound` |
| `claudine/lib/src/actions/bash_executor.rs` | Modify | Add doc comment clarifying raw interpolation contract |
| `claudine/lib/tests/canonical_dispatch.rs` | Modify | Add tests for decoupled services and default sounds |

---

### Task 1: Decouple logging from action binding lookup

**Files:**
- Modify: `claudine/lib/src/dispatch/mod.rs:289-380`
- Modify: `claudine/lib/tests/canonical_dispatch.rs`
- Modify: `claudine/lib/src/dispatch/mod.rs:1446-1472` (fix existing test)

- [ ] **Step 1: Write the failing test — logging fires with no binding**

In `claudine/lib/tests/canonical_dispatch.rs`, add:

```rust
/// Logging should fire even when there is no action binding for the event.
#[tokio::test]
async fn dispatch_logs_event_when_logging_enabled_and_no_binding() {
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let log_path = tmp.path().join("test.jsonl");

    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.logging = true;
    // No actions configured — no bindings at all.

    let runtime = compile_canonical_runtime(config, None).unwrap();
    let meta = EventMeta::new(Provider::Claude, AgenticEvent::SessionStart);

    // Set the log path env so `resolve_file_log_path` picks it up
    std::env::set_var("CLAUDINE_LOG_FILE", log_path.display().to_string());

    let outcome = dispatch_canonical_with_runtime(
        Provider::Claude,
        AgenticEvent::SessionStart,
        meta,
        &runtime,
    )
    .await
    .unwrap();

    std::env::remove_var("CLAUDINE_LOG_FILE");

    // The outcome should not be the bare default — logging ran.
    // But the response may still be None since there are no actions.
    // The key assertion: the log file was written to.
    let log_content = std::fs::read_to_string(&log_path).unwrap_or_default();
    assert!(
        log_content.contains("SessionStart"),
        "JSONL log should contain the dispatched event even without an action binding"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine -- dispatch_logs_event_when_logging_enabled_and_no_binding --nocapture`

Expected: FAIL — the current code returns early at line 293 when no binding exists, so logging never runs.

- [ ] **Step 3: Restructure `dispatch_canonical_with_runtime` to decouple services from binding**

In `claudine/lib/src/dispatch/mod.rs`, replace lines 289-380 (from `// --- Binding lookup` through end of function) with:

```rust
    // --- Binding lookup by canonical event only ---
    let binding = runtime.get_binding(&event);

    // --- Execute actions if a binding exists and is enabled ---
    let action_response = if let Some(binding) = binding {
        if !binding.enabled() {
            debug!(%event, "Canonical binding disabled, skipping actions");
            None
        } else if !matcher::matches_with_regex(binding.matcher(), &meta) {
            debug!(%event, "Matcher did not match in canonical binding, skipping actions");
            None
        } else {
            let resolved_hook = ResolvedHook {
                event,
                meta: meta.clone(),
                provider,
                actions: binding.actions().to_vec(),
                can_block,
            };

            info!(
                event = %resolved_hook.event,
                provider = %resolved_hook.provider,
                tool_name = resolved_hook.meta.tool_name.as_deref().unwrap_or(""),
                tool_detail = tool_detail.as_deref().unwrap_or(""),
                action_count = resolved_hook.actions.len(),
                can_block = resolved_hook.can_block,
                "Executing resolved canonical hook"
            );

            runner::execute_actions_v2(
                &resolved_hook.actions,
                Some(binding.compiled_mappers()),
                &resolved_hook.meta,
                runtime.config(),
                runtime.messaging(),
                resolved_hook.can_block,
                protect_pre.as_ref(),
            )
            .await?
        }
    } else {
        debug!(%event, "No canonical binding found for event");
        None
    };

    // --- JSONL event logging (independent of binding) ---
    if runtime.config().logging {
        log_dispatch_event(&meta);
    }

    // --- Protect post-evaluation (independent of binding) ---
    let protect_post = protect_service.and_then(|service| {
        if !matches!(
            event,
            AgenticEvent::AfterTool | AgenticEvent::TurnComplete | AgenticEvent::SubagentStop
        ) {
            return None;
        }
        let request = extract_protect_request(&event, &meta)?;
        let decision = service.evaluate(&request);
        if decision.is_blocked() {
            Some(decision)
        } else {
            None
        }
    });

    let action_response = if let Some(ref decision) = protect_post {
        Some(map_protect_block(decision))
    } else {
        action_response
    };

    finalize_response(
        adapter,
        &event,
        can_block,
        action_response,
        protect_pre,
        protect_post,
    )
```

Note: `meta` must be cloned for the `ResolvedHook` since we now use it again after action execution. The `meta.clone()` is needed because `ResolvedHook` takes ownership. Also `resolved_hook.event` and `resolved_hook.meta` references are no longer available after the `if let`, so use the outer `event` and `meta` in logging and protect_post sections.

- [ ] **Step 4: Run the new test to verify it passes**

Run: `cargo test -p claudine -- dispatch_logs_event_when_logging_enabled_and_no_binding --nocapture`

Expected: PASS

- [ ] **Step 5: Fix the existing test that asserts wrong behavior**

In `claudine/lib/src/dispatch/mod.rs`, update the test `canonical_dispatch_returns_default_when_no_binding` (around line 1446). The outcome is no longer `DispatchOutcome::default()` when logging is enabled because finalize_response runs. Update the test to disable logging:

```rust
    #[tokio::test]
    async fn canonical_dispatch_returns_default_when_no_binding() {
        use crate::config::claudine_config::{ClaudineConfig, DefaultSounds};

        let mut config = ClaudineConfig::default();
        config.protect.enabled = false;
        config.logging = false;
        config.default_sounds = DefaultSounds::default();

        let runtime = loader::compile_canonical_runtime(config, None).unwrap();

        let mut meta = EventMeta::new(Provider::Claude, AgenticEvent::SessionStart);
        meta.env = EnvironmentContext::default();

        let outcome = dispatch_canonical_with_runtime(
            Provider::Claude,
            AgenticEvent::SessionStart,
            meta,
            &runtime,
        )
        .await
        .unwrap();

        // With logging off and no binding, outcome goes through finalize_response
        // which for a non-blocking event returns the adapter's non_blocking_ack.
        // Claude adapter returns {} for non-blocking events.
        assert!(
            outcome.protect_pre.is_none(),
            "no protect evaluation without binding"
        );
        assert!(
            outcome.protect_post.is_none(),
            "no protect_post for SessionStart"
        );
    }
```

- [ ] **Step 6: Run all dispatch tests**

Run: `cargo test -p claudine -- canonical_dispatch --nocapture`

Expected: All pass.

- [ ] **Step 7: Commit**

```
feat(claudine): decouple logging and protect_post from action binding lookup

Services (logging, protect post-scan) now run for every dispatched
canonical event when enabled, regardless of whether the event has a
configured action binding.
```

---

### Task 2: Add protect_post test for events without bindings

**Files:**
- Modify: `claudine/lib/tests/canonical_dispatch.rs`

- [ ] **Step 1: Write the test — protect_post evaluates with no binding**

In `claudine/lib/tests/canonical_dispatch.rs`, add:

```rust
/// Protect post-evaluation should run even when an event has no action binding.
#[tokio::test]
async fn dispatch_protect_post_evaluates_without_binding() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = true;
    config.default_sounds = DefaultSounds::default();
    config.logging = false;
    // No actions configured — protect should still evaluate for AfterTool.

    let runtime = compile_canonical_runtime(config, None).unwrap();

    let mut meta = EventMeta::new(Provider::Claude, AgenticEvent::AfterTool);
    meta.tool_name = Some("Write".to_string());
    meta.tool_input = Some(serde_json::json!({"file_path": "/etc/passwd", "content": "pwned"}));
    meta.env = claudine::events::EnvironmentContext::default();

    let outcome = dispatch_canonical_with_runtime(
        Provider::Claude,
        AgenticEvent::AfterTool,
        meta,
        &runtime,
    )
    .await
    .unwrap();

    // AfterTool with a sensitive path should trigger protect_post even
    // without an action binding.
    assert!(
        outcome.protect_post.as_ref().map_or(false, |d| d.is_blocked()),
        "protect_post should evaluate and block /etc/passwd write even without an action binding"
    );
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p claudine -- dispatch_protect_post_evaluates_without_binding --nocapture`

Expected: PASS (task 1 already restructured the dispatch).

- [ ] **Step 3: Commit**

```
test(claudine): add protect_post test for events without action bindings
```

---

### Task 3: Add repo-scoped messenger override to RepoOverrideConfig

**Files:**
- Modify: `claudine/lib/src/config/claudine_config.rs:280-290`
- Modify: `claudine/lib/src/dispatch/loader.rs:857-872`

- [ ] **Step 1: Write the failing test — repo messenger override round-trips**

In `claudine/lib/src/config/claudine_config.rs`, add to the test module:

```rust
    #[test]
    fn repo_override_with_active_messenger_round_trips() {
        let json = r#"{ "active_messenger": "work-slack" }"#;
        let repo: RepoOverrideConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            repo.active_messenger,
            Some(Some("work-slack".to_string())),
            "should parse active_messenger override"
        );

        let serialized = serde_json::to_string(&repo).unwrap();
        assert!(serialized.contains("work-slack"));
    }

    #[test]
    fn repo_override_with_null_active_messenger_disables() {
        let json = r#"{ "active_messenger": null }"#;
        let repo: RepoOverrideConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            repo.active_messenger,
            Some(None),
            "null should mean 'disable messenger for this repo'"
        );
    }

    #[test]
    fn repo_override_without_active_messenger_is_no_override() {
        let json = r#"{}"#;
        let repo: RepoOverrideConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            repo.active_messenger, None,
            "absent field means no override"
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine -- repo_override_with_active_messenger --nocapture`

Expected: FAIL — `active_messenger` field doesn't exist yet.

- [ ] **Step 3: Add `active_messenger` to `RepoOverrideConfig`**

In `claudine/lib/src/config/claudine_config.rs`, update the `RepoOverrideConfig` struct:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoOverrideConfig {
    /// Override the canonical provider for this repo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_provider: Option<Provider>,

    /// Override or extend actions for this repo (per-event replacement).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub actions: HashMap<AgenticEvent, Vec<HookAction>>,

    /// Override the active messenger configuration key for this repo.
    ///
    /// - `None` (absent): no override, inherit from user config.
    /// - `Some(None)` (JSON `null`): disable messenger for this repo.
    /// - `Some(Some(key))`: use the named configuration for this repo.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_messenger_override",
    )]
    pub active_messenger: Option<Option<String>>,
}
```

Add the custom deserializer right after the struct (before `impl Default for ClaudineConfig`):

```rust
/// Deserialize the double-Option messenger override.
///
/// - JSON key absent → outer `None` (serde default)
/// - JSON `"active_messenger": null` → `Some(None)` (disable)
/// - JSON `"active_messenger": "key"` → `Some(Some("key"))`
fn deserialize_optional_messenger_override<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // When serde calls this, the key IS present in the JSON.
    // A JSON null arrives as Option::None, a string as Option::Some.
    let value: Option<String> = serde::Deserialize::deserialize(deserializer)?;
    Ok(Some(value))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claudine -- repo_override --nocapture`

Expected: All three new tests pass.

- [ ] **Step 5: Update merge_repo_override in loader.rs**

In `claudine/lib/src/dispatch/loader.rs`, add to `merge_repo_override`:

```rust
fn merge_repo_override(user: &mut ClaudineConfig, repo: &RepoOverrideConfig) {
    // canonical_provider: repo overrides user if set
    if repo.canonical_provider.is_some() {
        user.canonical_provider = repo.canonical_provider;
    }

    // actions: per-event replacement
    for (event, repo_actions) in &repo.actions {
        user.actions.insert(*event, repo_actions.clone());
    }

    // active_messenger: repo overrides the active config key only
    if let Some(ref override_value) = repo.active_messenger {
        if let Some(ref mut messenger) = user.messenger {
            messenger.active_config = override_value.clone();
        }
    }
}
```

- [ ] **Step 6: Add merge test in loader.rs tests**

Add a test to the existing loader test module:

```rust
    #[test]
    fn merge_repo_override_applies_active_messenger() {
        let mut user = ClaudineConfig::default();
        user.messenger = Some(ClaudineMessengerConfig {
            active_config: Some("personal".to_string()),
            configurations: {
                let mut m = HashMap::new();
                m.insert(
                    "personal".to_string(),
                    MessengerProviderConfig::Discord {
                        channel_id: "123".to_string(),
                        bot_token_env_var: "TOKEN".to_string(),
                    },
                );
                m.insert(
                    "work".to_string(),
                    MessengerProviderConfig::Slack {
                        webhook_url_env_var: "SLACK_URL".to_string(),
                    },
                );
                m
            },
        });

        let repo = RepoOverrideConfig {
            active_messenger: Some(Some("work".to_string())),
            ..Default::default()
        };

        merge_repo_override(&mut user, &repo);

        assert_eq!(
            user.messenger.as_ref().unwrap().active_config.as_deref(),
            Some("work"),
            "repo should override active messenger key"
        );
    }

    #[test]
    fn merge_repo_override_disables_messenger_with_null() {
        let mut user = ClaudineConfig::default();
        user.messenger = Some(ClaudineMessengerConfig {
            active_config: Some("personal".to_string()),
            configurations: HashMap::new(),
        });

        let repo = RepoOverrideConfig {
            active_messenger: Some(None),
            ..Default::default()
        };

        merge_repo_override(&mut user, &repo);

        assert_eq!(
            user.messenger.as_ref().unwrap().active_config,
            None,
            "null override should disable active messenger"
        );
    }
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p claudine -- merge_repo_override --nocapture`

Expected: PASS.

- [ ] **Step 8: Commit**

```
feat(claudine): add repo-scoped active_messenger override to RepoOverrideConfig

Repos can now override the active messenger config key without
duplicating full provider configurations into repo-level state.
```

---

### Task 4: Implement staged state for Protect rules modal

**Files:**
- Modify: `claudine/cli/src/commands/config_tui/app.rs:87-89`
- Modify: `claudine/cli/src/commands/config_tui/tabs/services.rs:375-411`

- [ ] **Step 1: Update ModalState::ProtectRules to carry staged state**

In `claudine/cli/src/commands/config_tui/app.rs`, change the `ProtectRules` variant:

```rust
    ProtectRules {
        highlighted: usize,
        staged_rules: claudine::services::protect::config::ProtectRuleToggles,
    },
```

- [ ] **Step 2: Update all pattern matches for ProtectRules**

In `app.rs`, update `modal_highlighted` (line 325):

```rust
            Some(ModalState::ProtectRules { highlighted, .. }) => *highlighted,
```

In `app.rs`, update `set_modal_highlighted` (line 350):

```rust
                ModalState::ProtectRules { highlighted, .. } => *highlighted = new_idx,
```

- [ ] **Step 3: Update modal open in services.rs to clone current rules**

In `claudine/cli/src/commands/config_tui/tabs/services.rs`, update the line that opens the modal (around line 377):

```rust
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.modal = Some(ModalState::ProtectRules {
                highlighted: 0,
                staged_rules: app.config.protect.rules.clone(),
            });
        }
```

- [ ] **Step 4: Update `handle_protect_rules_modal` to use staged state**

Replace the entire `handle_protect_rules_modal` function in `services.rs`:

```rust
pub fn handle_protect_rules_modal(app: &mut App, key: KeyEvent) {
    let rule_names = super::super::get_protect_rule_names();
    let count = rule_names.len();
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
        KeyCode::Char(' ') => {
            let idx = app.modal_highlighted();
            if let Some(name) = rule_names.get(idx) {
                if let Some(ModalState::ProtectRules { staged_rules, .. }) = &mut app.modal {
                    super::super::toggle_protect_rule(staged_rules, name);
                }
            }
        }
        KeyCode::Enter => {
            // Commit staged rules to config
            if let Some(ModalState::ProtectRules { staged_rules, .. }) = app.modal.take() {
                app.config.protect.rules = staged_rules;
                app.dirty = true;
            }
        }
        KeyCode::Esc => {
            // Discard staged changes
            app.modal = None;
        }
        _ => {}
    }
}
```

- [ ] **Step 5: Update the Protect rules modal renderer to use staged state**

In `services.rs`, find the rendering code that reads `app.config.protect.rules` inside the modal and change it to read from the staged rules. Look for the modal rendering block (around line 131) that checks `ModalState::ProtectRules`. Update the rule-enabled check to read from `staged_rules`:

```rust
    if let Some(ModalState::ProtectRules { highlighted, staged_rules }) = &app.modal {
```

And where rule enabled status is checked for display, use `staged_rules` instead of `app.config.protect.rules`. Find the call to `is_protect_rule_enabled` and pass `staged_rules` instead of `&app.config.protect.rules`.

- [ ] **Step 6: Build and verify**

Run: `cargo build -p claudine-cli`

Expected: Compiles successfully.

- [ ] **Step 7: Commit**

```
fix(claudine): stage protect rule toggles in modal, commit on Enter, discard on Esc

The protect rules modal now clones rules into modal-local state on open.
Space toggles the staged copy. Enter commits staged rules to config.
Esc discards all staged changes.
```

---

### Task 5: Add repo messenger override to TUI

**Files:**
- Modify: `claudine/cli/src/commands/config_tui/tabs/messenger.rs`

- [ ] **Step 1: Add repo messenger override display**

In `messenger.rs`, in the `render` function, after the active config line, add a repo override line when `app.is_in_repo` is true:

```rust
    // Show repo override if in a repo
    if app.is_in_repo {
        let repo_active = app
            .repo_config
            .as_ref()
            .and_then(|rc| rc.active_messenger.as_ref())
            .map(|opt| {
                opt.as_deref().unwrap_or("(disabled)")
            });
        let repo_label = match repo_active {
            Some(name) => format!("  Repo override: {name}"),
            None => "  Repo override: (none — inherits user)".to_string(),
        };
        // Render repo_label as a styled line below the active config
    }
```

- [ ] **Step 2: Add 'R' key handler for repo messenger override**

In `handle_key`, add a handler for 'R' to open a messenger select modal that targets the repo config:

```rust
        KeyCode::Char('r') | KeyCode::Char('R') if app.is_in_repo => {
            let has_configs = app
                .config
                .messenger
                .as_ref()
                .map(|m| !m.configurations.is_empty())
                .unwrap_or(false);
            if has_configs {
                let configs: Vec<String> = app
                    .config
                    .messenger
                    .as_ref()
                    .map(|m| m.configurations.keys().cloned().collect())
                    .unwrap_or_default();
                let repo_active = app
                    .repo_config
                    .as_ref()
                    .and_then(|rc| rc.active_messenger.as_ref())
                    .and_then(|opt| opt.as_deref());
                let highlighted = repo_active
                    .and_then(|name| configs.iter().position(|k| k == name))
                    .map(|i| i + 2) // +1 for "(inherit)", +1 for "(disabled)"
                    .unwrap_or(0);
                app.modal = Some(ModalState::MessengerSelect { highlighted });
                // Mark this as a repo-targeted selection (see step 3)
            }
        }
```

Note: The exact modal handling for repo vs user messenger selection may require a new modal variant or a flag. Evaluate during implementation whether to add a `repo_target: bool` field to `MessengerSelect` or use a separate variant.

- [ ] **Step 3: Build and verify**

Run: `cargo build -p claudine-cli`

Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```
feat(claudine): add repo messenger override to TUI messenger tab

Users can now press R in the Messenger tab to set a repo-scoped
override for the active messenger configuration key.
```

---

### Task 6: Remove hook registration from headless init

**Files:**
- Modify: `claudine/cli/src/commands/init_wizard.rs:31-55`

- [ ] **Step 1: Write the test expectation**

This is a behavioral change verified by code inspection. The headless path should only write config. There's no easy way to unit test "hooks were NOT registered" without mocking the filesystem, so we verify by reading the code.

- [ ] **Step 2: Remove hook registration from headless path**

In `claudine/cli/src/commands/init_wizard.rs`, replace `run_headless_initialization`:

```rust
async fn run_headless_initialization() -> Result<()> {
    let config = build_headless_config();
    let path = claudine::dispatch::loader::user_config_path();
    claudine::dispatch::loader::save_claudine_config(&config, &path)?;
    Ok(())
}
```

Remove the `register_hooks_all_providers_headless` function entirely (lines 40-55).

- [ ] **Step 3: Build and verify**

Run: `cargo build -p claudine-cli`

Expected: Compiles (may warn about unused `discover_agents_full` import if only used by headless — check and clean up).

- [ ] **Step 4: Clean up unused imports**

If `discover_agents_full`, `provider_hook_events`, `ProviderHookPlan`, or `get_configurator` were only used by the removed function, remove those imports.

- [ ] **Step 5: Commit**

```
fix(claudine): headless init only writes default config, no hook registration

The non-interactive/CI init path no longer mutates provider state by
registering hooks. Users must run interactive init or `claudine sync`
to register hooks explicitly.
```

---

### Task 7: Document bash interpolation contract and add tests

**Files:**
- Modify: `claudine/lib/src/actions/bash_executor.rs:144-157`
- Modify: `claudine/lib/src/dispatch/runner.rs:562-606`
- Modify: `claudine/lib/src/dispatch/runner.rs` (test section)

- [ ] **Step 1: Document the escaping contract on `shell_escape`**

In `claudine/lib/src/actions/bash_executor.rs`, update the doc comment on `shell_escape`:

```rust
/// Wraps a value in single quotes, escaping any embedded single quotes.
///
/// **Note:** This function is provided for callers that need explicit
/// shell escaping (e.g., building `sh -c` strings). The standard dispatch
/// path does NOT use this function because it passes interpolated values
/// through `shell_words::split` and then supplies them as discrete `argv`
/// entries via `Command::args()`, which preserves variable boundaries
/// without shell interpretation.
///
/// ## Examples
///
/// ```
/// use claudine::actions::bash_executor::shell_escape;
///
/// assert_eq!(shell_escape("hello"), "'hello'");
/// assert_eq!(shell_escape("it's"), "'it'\\''s'");
/// ```
pub fn shell_escape(value: &str) -> String {
    let escaped = value.replace('\'', "'\\''");
    format!("'{escaped}'")
}
```

- [ ] **Step 2: Document the contract on `execute_bash`**

In `claudine/lib/src/dispatch/runner.rs`, update the doc comment on `execute_bash`:

```rust
/// Execute a validated command asynchronously using direct spawning.
///
/// ## Interpolation Contract
///
/// Template placeholders in `command` and `params` are expanded by
/// [`interpolate`] as raw string substitution. The rendered `params`
/// string is then split by [`shell_words::split`] into discrete `argv`
/// entries. This means:
///
/// - Interpolated values that contain spaces **will** split into
///   multiple arguments unless the config author quoted them in the
///   `params` template (e.g., `--message '{{tool_name}}'`).
/// - No shell metacharacter interpretation occurs because the command
///   is spawned directly via `Command::new().args()`, not through
///   `sh -c`.
/// - The `shell_escape()` helper in `bash_executor` is intentionally
///   not used here — it is for callers that build `sh -c` strings.
```

- [ ] **Step 3: Write argv preservation tests**

In `claudine/lib/src/dispatch/runner.rs`, add to the test module:

```rust
    #[test]
    fn interpolate_preserves_spaces_in_quoted_params() {
        // Simulates what happens when a user writes:
        //   params: "--message '{{tool_name}}'"
        // and tool_name = "my tool"
        let raw = "--message 'my tool'";
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--message", "my tool"]);
    }

    #[test]
    fn interpolate_splits_unquoted_spaces() {
        // Without quotes, an interpolated value with spaces splits.
        let raw = "--message my tool";
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--message", "my", "tool"]);
    }

    #[test]
    fn interpolate_handles_shell_metacharacters_safely() {
        // Metacharacters are literal in shell_words::split (no shell expansion).
        let raw = "--path /tmp/$(whoami)";
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--path", "/tmp/$(whoami)"]);
    }

    #[test]
    fn interpolate_handles_quotes_in_values() {
        let raw = r#"--message "it's a test""#;
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--message", "it's a test"]);
    }

    #[test]
    fn interpolate_empty_params_produces_no_args() {
        let raw = "";
        let args: Vec<String> = if raw.is_empty() {
            vec![]
        } else {
            shell_words::split(raw).unwrap()
        };
        assert!(args.is_empty());
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p claudine -- interpolate_ --nocapture`

Expected: All pass. These test the documented contract.

- [ ] **Step 5: Commit**

```
docs(claudine): document bash interpolation contract and add argv tests

Clarifies that the dispatch bash action uses shell_words::split on
raw-interpolated params and spawns directly via Command::args — no
shell interpretation occurs. The shell_escape helper is documented as
being for callers that need sh -c strings.
```

---

### Task 8: Wire default_sounds into dispatch outcomes

**Files:**
- Modify: `claudine/lib/src/dispatch/mod.rs`
- Modify: `claudine/lib/src/dispatch/runner.rs`
- Modify: `claudine/lib/tests/canonical_dispatch.rs`

- [ ] **Step 1: Add a function to play default sounds based on event category**

In `claudine/lib/src/dispatch/runner.rs`, add after `execute_sound_effect`:

```rust
/// Play the appropriate default sound for an event, if configured.
///
/// Maps canonical events to sound categories:
/// - `success`: SessionEnd, TurnComplete
/// - `attention`: HumanInTheLoop
/// - `error`: (triggered by protect blocks)
pub fn play_default_sound_for_event(
    event: &AgenticEvent,
    config: &ClaudineConfig,
    was_blocked: bool,
) {
    let sound_name = if was_blocked {
        config.default_sounds.error.as_deref()
    } else {
        match event {
            AgenticEvent::SessionEnd | AgenticEvent::TurnComplete => {
                config.default_sounds.success.as_deref()
            }
            AgenticEvent::HumanInTheLoop => config.default_sounds.attention.as_deref(),
            _ => None,
        }
    };

    if let Some(name) = sound_name {
        execute_sound_effect(name, 1.0, 1.0);
    }
}
```

- [ ] **Step 2: Call `play_default_sound_for_event` from dispatch**

In `claudine/lib/src/dispatch/mod.rs`, after the protect_post evaluation block and before `finalize_response`, add:

```rust
    // --- Default sounds ---
    let was_blocked = protect_pre.is_some() || protect_post.is_some();
    runner::play_default_sound_for_event(&event, runtime.config(), was_blocked);
```

Note: `play_default_sound_for_event` needs to be `pub(crate)` not just `pub` — adjust visibility as needed.

- [ ] **Step 3: Write test**

In `claudine/lib/tests/canonical_dispatch.rs`, add:

```rust
/// Default sounds config is accessible via the runtime and the function
/// correctly maps events to sound categories.
#[test]
fn default_sound_maps_event_to_category() {
    use claudine::events::AgenticEvent;
    use claudine::config::claudine_config::{ClaudineConfig, DefaultSounds};

    let config = ClaudineConfig {
        default_sounds: DefaultSounds {
            success: Some("doorbell".to_string()),
            attention: Some("bong".to_string()),
            error: Some("space-alarm".to_string()),
        },
        ..ClaudineConfig::default()
    };

    // Success events
    assert_eq!(
        default_sound_for_event(&AgenticEvent::TurnComplete, &config, false),
        Some("doorbell"),
    );
    assert_eq!(
        default_sound_for_event(&AgenticEvent::SessionEnd, &config, false),
        Some("doorbell"),
    );

    // Attention events
    assert_eq!(
        default_sound_for_event(&AgenticEvent::HumanInTheLoop, &config, false),
        Some("bong"),
    );

    // Error (blocked)
    assert_eq!(
        default_sound_for_event(&AgenticEvent::BeforeTool, &config, true),
        Some("space-alarm"),
    );

    // No sound for unmapped events
    assert_eq!(
        default_sound_for_event(&AgenticEvent::SessionStart, &config, false),
        None,
    );
}
```

For this test to work, extract the sound selection logic into a pure function `default_sound_for_event` that returns `Option<&str>` — this keeps the logic testable without needing audio hardware:

```rust
/// Determine which default sound (if any) should play for the given event.
///
/// Returns the sound name from config, or `None` if no sound is configured
/// for this event/state combination.
pub fn default_sound_for_event<'a>(
    event: &AgenticEvent,
    config: &'a ClaudineConfig,
    was_blocked: bool,
) -> Option<&'a str> {
    if was_blocked {
        return config.default_sounds.error.as_deref();
    }
    match event {
        AgenticEvent::SessionEnd | AgenticEvent::TurnComplete => {
            config.default_sounds.success.as_deref()
        }
        AgenticEvent::HumanInTheLoop => config.default_sounds.attention.as_deref(),
        _ => None,
    }
}
```

Put this in `runner.rs` and make it `pub`. Update `play_default_sound_for_event` to call it.

- [ ] **Step 4: Run tests**

Run: `cargo test -p claudine -- default_sound --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```
feat(claudine): wire default_sounds into dispatch lifecycle

Default sounds are now played during canonical dispatch based on event
category: success sounds for SessionEnd/TurnComplete, attention for
HumanInTheLoop, and error when protect blocks a request.
```

---

### Task 9: Final integration verification

**Files:** None (verification only)

- [ ] **Step 1: Run the full claudine test suite**

Run: `cargo test -p claudine --nocapture`

Expected: All tests pass.

- [ ] **Step 2: Run the claudine-cli build**

Run: `cargo build -p claudine-cli`

Expected: Compiles cleanly.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p claudine -p claudine-cli -- -D warnings`

Expected: No warnings.

- [ ] **Step 4: Run fmt check**

Run: `cargo fmt --package claudine --package claudine-cli -- --check`

Expected: No formatting issues.

- [ ] **Step 5: Final commit if any fixups needed**

Only if clippy/fmt required changes.
