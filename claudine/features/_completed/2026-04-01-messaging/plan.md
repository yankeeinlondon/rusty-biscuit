# Messaging Feature Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add outbound chat notifications (Discord, Slack, Signal, WhatsApp) to Claudine's hook system as a fire-and-forget `Message` action, using the existing `messenger` library.

**Architecture:** A new `claudine::messaging` module handles config types, scope-aware route resolution, secret lookup, and async send. The `HookAction::Message` variant follows the same fire-and-forget pattern as `Speak`. User and repo messaging scopes are preserved separately in `RuntimeConfig` so repo `active = null` doesn't erase user fallback.

**Tech Stack:** Rust, serde, tokio, messenger library (discord/slack/signal/whatsapp features), secrecy crate, tracing

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `claudine/lib/src/messaging/mod.rs` | Module root; re-exports public types and `execute_message()` entry point |
| `claudine/lib/src/messaging/config.rs` | `ScopedMessagingSettings`, `MessagingRouteConfig` enum (serde), default env var name helpers, validation |
| `claudine/lib/src/messaging/resolve.rs` | `RuntimeMessagingSettings`, `ResolvedMessagingRoute`, `MessagingScope`, effective route resolution, secret resolution, Signal recipient parsing, image path resolution |
| `claudine/lib/src/messaging/send.rs` | Build `messenger::Messenger` + `Dispatch` + `Message`, fire-and-forget async send |

### Modified Files

| File | Changes |
|------|---------|
| `claudine/lib/Cargo.toml` | Add `messenger` and `secrecy` dependencies |
| `claudine/lib/src/lib.rs` | Add `pub mod messaging;` |
| `claudine/lib/src/actions/hook_action.rs` | Add `Message` variant, update `type_slug()` and `type_pascal_case()` |
| `claudine/lib/src/events/config.rs` | Add `messaging` field to `GlobalSettings`, import `ScopedMessagingSettings` |
| `claudine/lib/src/dispatch/loader.rs` | Add `messaging` field to `RuntimeConfig`, preserve user/repo scopes separately, add messaging validation, expose `messaging()` accessor |
| `claudine/lib/src/dispatch/runner.rs` | Add `messaging` parameter to `execute_actions()`, add `Message` match arm |
| `claudine/lib/src/dispatch/mod.rs` | Pass `config.messaging()` to `execute_actions()` |

---

## Task 1: Add Dependencies

**Files:**
- Modify: `claudine/lib/Cargo.toml`

- [ ] **Step 1: Add messenger and secrecy to Cargo.toml**

In `claudine/lib/Cargo.toml`, add to the `[dependencies]` section after the `biscuit-speaks` line:

```toml
messenger = { path = "../../messenger/lib", default-features = false, features = ["discord", "slack", "signal", "whatsapp"] }
secrecy = "0.10"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p claudine`
Expected: compiles successfully

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/Cargo.toml
git commit -m "feat(claudine): add messenger and secrecy dependencies"
```

---

## Task 2: Add `HookAction::Message` Variant

**Files:**
- Modify: `claudine/lib/src/actions/hook_action.rs:103-194`
- Test: inline `#[cfg(test)] mod tests` at bottom of same file

- [ ] **Step 1: Write failing tests for Message serde and type helpers**

Add these tests to the existing `mod tests` block in `hook_action.rs`:

```rust
#[test]
fn message_deserializes_with_required_fields() {
    let json = serde_json::json!({
        "type": "message",
        "message": "Deploy complete"
    });

    let action: HookAction = serde_json::from_value(json).unwrap();
    let HookAction::Message { message, image } = action else {
        panic!("expected message");
    };

    assert_eq!(message, "Deploy complete");
    assert!(image.is_none());
}

#[test]
fn message_deserializes_with_image() {
    let json = serde_json::json!({
        "type": "message",
        "message": "Screenshot attached",
        "image": "/tmp/screenshot.png"
    });

    let action: HookAction = serde_json::from_value(json).unwrap();
    let HookAction::Message { message, image } = action else {
        panic!("expected message");
    };

    assert_eq!(message, "Screenshot attached");
    assert_eq!(image.as_deref(), Some("/tmp/screenshot.png"));
}

#[test]
fn message_round_trip() {
    let action = HookAction::Message {
        message: "**build** done".to_string(),
        image: Some("~/artifacts/build.png".to_string()),
    };

    let json = serde_json::to_value(&action).unwrap();
    assert_eq!(json["type"], "message");
    assert_eq!(json["message"], "**build** done");
    assert_eq!(json["image"], "~/artifacts/build.png");

    let back: HookAction = serde_json::from_value(json).unwrap();
    assert_eq!(back, action);
}

#[test]
fn message_type_labels() {
    let action = HookAction::Message {
        message: "test".to_string(),
        image: None,
    };

    assert_eq!(action.type_slug(), "message");
    assert_eq!(action.type_pascal_case(), "Message");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine -- hook_action::tests::message`
Expected: FAIL — no `Message` variant exists

- [ ] **Step 3: Add the Message variant to HookAction**

In `hook_action.rs`, add the new variant to the `HookAction` enum (after the `Report` variant):

```rust
    /// Send a message to the configured messaging destination.
    Message {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image: Option<String>,
    },
```

- [ ] **Step 4: Update type_slug() and type_pascal_case()**

Add the `Message` arm to both match expressions:

In `type_slug()`:
```rust
HookAction::Message { .. } => "message",
```

In `type_pascal_case()`:
```rust
HookAction::Message { .. } => "Message",
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p claudine -- hook_action::tests::message`
Expected: all 4 new tests PASS

- [ ] **Step 6: Fix any exhaustive match warnings**

Run: `cargo check -p claudine 2>&1`

The compiler will flag non-exhaustive match arms in `runner.rs` (the `execute_actions` match block) and possibly `compile_action_mapper`. Add a temporary placeholder arm in `runner.rs`:

```rust
HookAction::Message { .. } => {
    debug!("Message action not yet implemented");
}
```

And if needed in `compile_action_mapper`:
```rust
HookAction::Message { .. } => Ok(None),
```

Run: `cargo check -p claudine`
Expected: compiles clean

- [ ] **Step 7: Commit**

```bash
git add claudine/lib/src/actions/hook_action.rs claudine/lib/src/dispatch/runner.rs claudine/lib/src/dispatch/loader.rs
git commit -m "feat(claudine): add Message variant to HookAction"
```

---

## Task 3: Messaging Config Types (`messaging/config.rs`)

**Files:**
- Create: `claudine/lib/src/messaging/config.rs`
- Create: `claudine/lib/src/messaging/mod.rs`
- Modify: `claudine/lib/src/lib.rs`

- [ ] **Step 1: Create the module root**

Create `claudine/lib/src/messaging/mod.rs`:

```rust
mod config;
mod resolve;
mod send;

pub use config::{MessagingRouteConfig, ScopedMessagingSettings};
pub use resolve::{
    MessagingScope, ResolvedMessagingRoute, RuntimeMessagingSettings, resolve_effective_route,
};
pub use send::execute_message;
```

Note: `resolve` and `send` modules will be created in later tasks. For now, comment out the `mod resolve;`, `mod send;`, and their `pub use` lines. We will uncomment them as those modules are created.

Temporary version:

```rust
mod config;
// mod resolve;
// mod send;

pub use config::{MessagingRouteConfig, ScopedMessagingSettings};
// pub use resolve::{
//     MessagingScope, ResolvedMessagingRoute, RuntimeMessagingSettings, resolve_effective_route,
// };
// pub use send::execute_message;
```

- [ ] **Step 2: Register the module**

In `claudine/lib/src/lib.rs`, add:

```rust
pub mod messaging;
```

- [ ] **Step 3: Write failing tests for config serde**

Create `claudine/lib/src/messaging/config.rs` with the test module first:

```rust
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_config_deserializes_with_env_default() {
        let json = serde_json::json!({
            "provider": "discord",
            "channel_id": "123456789012345678"
        });

        let config: MessagingRouteConfig = serde_json::from_value(json).unwrap();
        let MessagingRouteConfig::Discord {
            channel_id,
            bot_token,
            bot_token_env,
        } = config
        else {
            panic!("expected discord");
        };

        assert_eq!(channel_id, "123456789012345678");
        assert!(bot_token.is_none());
        assert_eq!(bot_token_env, "DISCORD_BOT_TOKEN");
    }

    #[test]
    fn slack_config_deserializes_with_inline_token() {
        let json = serde_json::json!({
            "provider": "slack",
            "channel_id": "C012345ABC",
            "bot_token": "xoxb-test-token"
        });

        let config: MessagingRouteConfig = serde_json::from_value(json).unwrap();
        let MessagingRouteConfig::Slack {
            channel_id,
            bot_token,
            bot_token_env,
        } = config
        else {
            panic!("expected slack");
        };

        assert_eq!(channel_id, "C012345ABC");
        assert_eq!(bot_token.as_deref(), Some("xoxb-test-token"));
        assert_eq!(bot_token_env, "SLACK_BOT_TOKEN");
    }

    #[test]
    fn signal_config_deserializes_with_defaults() {
        let json = serde_json::json!({
            "provider": "signal",
            "recipient": "+15551234567"
        });

        let config: MessagingRouteConfig = serde_json::from_value(json).unwrap();
        let MessagingRouteConfig::Signal {
            recipient,
            rpc_url,
            rpc_url_env,
            account,
            account_env,
        } = config
        else {
            panic!("expected signal");
        };

        assert_eq!(recipient, "+15551234567");
        assert!(rpc_url.is_none());
        assert_eq!(rpc_url_env, "SIGNAL_RPC_URL");
        assert!(account.is_none());
        assert_eq!(account_env, "SIGNAL_ACCOUNT");
    }

    #[test]
    fn whatsapp_config_deserializes_with_defaults() {
        let json = serde_json::json!({
            "provider": "whatsapp",
            "recipient": "+15559876543"
        });

        let config: MessagingRouteConfig = serde_json::from_value(json).unwrap();
        let MessagingRouteConfig::WhatsApp {
            recipient,
            access_token,
            access_token_env,
            phone_number_id,
            phone_number_id_env,
        } = config
        else {
            panic!("expected whatsapp");
        };

        assert_eq!(recipient, "+15559876543");
        assert!(access_token.is_none());
        assert_eq!(access_token_env, "WHATSAPP_ACCESS_TOKEN");
        assert!(phone_number_id.is_none());
        assert_eq!(phone_number_id_env, "WHATSAPP_PHONE_NUMBER_ID");
    }

    #[test]
    fn scoped_settings_round_trip() {
        let settings = ScopedMessagingSettings {
            active: Some("work-slack".to_string()),
            configs: HashMap::from([(
                "work-slack".to_string(),
                MessagingRouteConfig::Slack {
                    channel_id: "C012345ABC".to_string(),
                    bot_token: None,
                    bot_token_env: "SLACK_BOT_TOKEN".to_string(),
                },
            )]),
        };

        let json = serde_json::to_value(&settings).unwrap();
        let back: ScopedMessagingSettings = serde_json::from_value(json).unwrap();

        assert_eq!(back.active.as_deref(), Some("work-slack"));
        assert!(back.configs.contains_key("work-slack"));
    }

    #[test]
    fn scoped_settings_deserializes_with_no_active() {
        let json = serde_json::json!({
            "configs": {
                "alerts": {
                    "provider": "discord",
                    "channel_id": "999"
                }
            }
        });

        let settings: ScopedMessagingSettings = serde_json::from_value(json).unwrap();
        assert!(settings.active.is_none());
        assert!(settings.configs.contains_key("alerts"));
    }

    #[test]
    fn validate_rejects_missing_active_route() {
        let settings = ScopedMessagingSettings {
            active: Some("nonexistent".to_string()),
            configs: HashMap::new(),
        };

        let result = settings.validate("user");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent"), "error: {err}");
    }

    #[test]
    fn validate_rejects_blank_channel_id() {
        let settings = ScopedMessagingSettings {
            active: Some("bad".to_string()),
            configs: HashMap::from([(
                "bad".to_string(),
                MessagingRouteConfig::Discord {
                    channel_id: "  ".to_string(),
                    bot_token: None,
                    bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                },
            )]),
        };

        let result = settings.validate("user");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("channel_id"), "error: {err}");
    }

    #[test]
    fn validate_rejects_blank_env_var_name() {
        let settings = ScopedMessagingSettings {
            active: Some("bad".to_string()),
            configs: HashMap::from([(
                "bad".to_string(),
                MessagingRouteConfig::Slack {
                    channel_id: "C123".to_string(),
                    bot_token: None,
                    bot_token_env: "  ".to_string(),
                },
            )]),
        };

        let result = settings.validate("user");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("bot_token_env"), "error: {err}");
    }

    #[test]
    fn validate_rejects_blank_config_name() {
        let settings = ScopedMessagingSettings {
            active: Some("  ".to_string()),
            configs: HashMap::from([(
                "  ".to_string(),
                MessagingRouteConfig::Discord {
                    channel_id: "123".to_string(),
                    bot_token: None,
                    bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                },
            )]),
        };

        let result = settings.validate("user");
        assert!(result.is_err());
    }

    #[test]
    fn validate_accepts_valid_config() {
        let settings = ScopedMessagingSettings {
            active: Some("ops".to_string()),
            configs: HashMap::from([(
                "ops".to_string(),
                MessagingRouteConfig::Slack {
                    channel_id: "C012345ABC".to_string(),
                    bot_token: None,
                    bot_token_env: "SLACK_BOT_TOKEN".to_string(),
                },
            )]),
        };

        assert!(settings.validate("user").is_ok());
    }

    #[test]
    fn validate_accepts_no_active_with_configs() {
        let settings = ScopedMessagingSettings {
            active: None,
            configs: HashMap::from([(
                "standby".to_string(),
                MessagingRouteConfig::Discord {
                    channel_id: "123".to_string(),
                    bot_token: None,
                    bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                },
            )]),
        };

        assert!(settings.validate("repo").is_ok());
    }
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p claudine -- messaging::config::tests`
Expected: FAIL — types don't exist yet

- [ ] **Step 5: Implement config types and validation**

Add the implementation above the `#[cfg(test)]` block in `config.rs`:

```rust
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{ClaudineError, Result};

/// Per-scope messaging settings as serialized in config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedMessagingSettings {
    /// Name of the active route in `configs`. `None` disables messaging for this scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,

    /// Named messaging route configurations.
    #[serde(default)]
    pub configs: HashMap<String, MessagingRouteConfig>,
}

/// Provider-specific messaging route configuration.
///
/// Each variant carries the fields needed to build a `messenger` provider
/// and target. Secrets use an inline-or-env pattern: if the inline field
/// is present it wins, otherwise the env var name is looked up at send time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum MessagingRouteConfig {
    Discord {
        channel_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bot_token: Option<String>,
        #[serde(default = "default_discord_token_env")]
        bot_token_env: String,
    },
    Slack {
        channel_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bot_token: Option<String>,
        #[serde(default = "default_slack_token_env")]
        bot_token_env: String,
    },
    Signal {
        recipient: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rpc_url: Option<String>,
        #[serde(default = "default_signal_rpc_url_env")]
        rpc_url_env: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account: Option<String>,
        #[serde(default = "default_signal_account_env")]
        account_env: String,
    },
    WhatsApp {
        recipient: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        access_token: Option<String>,
        #[serde(default = "default_whatsapp_access_token_env")]
        access_token_env: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phone_number_id: Option<String>,
        #[serde(default = "default_whatsapp_phone_number_id_env")]
        phone_number_id_env: String,
    },
}

impl ScopedMessagingSettings {
    /// Validate this scope's messaging configuration.
    ///
    /// `scope_label` is used in error messages (e.g. "user" or "repo").
    pub fn validate(&self, scope_label: &str) -> Result<()> {
        // Validate config names are not blank
        for name in self.configs.keys() {
            if name.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{scope_label} messaging: config name must not be blank"
                )));
            }
        }

        // Validate active references an existing config
        if let Some(active) = &self.active {
            if active.trim().is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{scope_label} messaging: active name must not be blank"
                )));
            }
            if !self.configs.contains_key(active) {
                return Err(ClaudineError::ConfigValidation(format!(
                    "{scope_label} messaging: active route \"{active}\" not found in configs"
                )));
            }
        }

        // Validate each route config
        for (name, config) in &self.configs {
            config.validate(scope_label, name)?;
        }

        Ok(())
    }
}

impl MessagingRouteConfig {
    fn validate(&self, scope_label: &str, name: &str) -> Result<()> {
        let prefix = format!("{scope_label} messaging config \"{name}\"");
        match self {
            MessagingRouteConfig::Discord {
                channel_id,
                bot_token_env,
                ..
            } => {
                if channel_id.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "{prefix}: channel_id must not be blank"
                    )));
                }
                if bot_token_env.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "{prefix}: bot_token_env must not be blank"
                    )));
                }
            }
            MessagingRouteConfig::Slack {
                channel_id,
                bot_token_env,
                ..
            } => {
                if channel_id.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "{prefix}: channel_id must not be blank"
                    )));
                }
                if bot_token_env.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "{prefix}: bot_token_env must not be blank"
                    )));
                }
            }
            MessagingRouteConfig::Signal {
                recipient,
                rpc_url_env,
                account_env,
                ..
            } => {
                if recipient.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "{prefix}: recipient must not be blank"
                    )));
                }
                if rpc_url_env.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "{prefix}: rpc_url_env must not be blank"
                    )));
                }
                if account_env.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "{prefix}: account_env must not be blank"
                    )));
                }
            }
            MessagingRouteConfig::WhatsApp {
                recipient,
                access_token_env,
                phone_number_id_env,
                ..
            } => {
                if recipient.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "{prefix}: recipient must not be blank"
                    )));
                }
                if access_token_env.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "{prefix}: access_token_env must not be blank"
                    )));
                }
                if phone_number_id_env.trim().is_empty() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "{prefix}: phone_number_id_env must not be blank"
                    )));
                }
            }
        }
        Ok(())
    }
}

fn default_discord_token_env() -> String {
    "DISCORD_BOT_TOKEN".to_string()
}

fn default_slack_token_env() -> String {
    "SLACK_BOT_TOKEN".to_string()
}

fn default_signal_rpc_url_env() -> String {
    "SIGNAL_RPC_URL".to_string()
}

fn default_signal_account_env() -> String {
    "SIGNAL_ACCOUNT".to_string()
}

fn default_whatsapp_access_token_env() -> String {
    "WHATSAPP_ACCESS_TOKEN".to_string()
}

fn default_whatsapp_phone_number_id_env() -> String {
    "WHATSAPP_PHONE_NUMBER_ID".to_string()
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p claudine -- messaging::config::tests`
Expected: all 11 tests PASS

- [ ] **Step 7: Commit**

```bash
git add claudine/lib/src/messaging/ claudine/lib/src/lib.rs
git commit -m "feat(claudine): add messaging config types with validation"
```

---

## Task 4: Route Resolution (`messaging/resolve.rs`)

**Files:**
- Create: `claudine/lib/src/messaging/resolve.rs`
- Modify: `claudine/lib/src/messaging/mod.rs` (uncomment resolve)

- [ ] **Step 1: Write failing tests for route resolution and secret lookup**

Create `claudine/lib/src/messaging/resolve.rs` with the test module:

```rust
use std::collections::HashMap;

use tracing::{debug, warn};

use super::config::{MessagingRouteConfig, ScopedMessagingSettings};

#[cfg(test)]
mod tests {
    use super::*;

    fn slack_config() -> MessagingRouteConfig {
        MessagingRouteConfig::Slack {
            channel_id: "C123".to_string(),
            bot_token: None,
            bot_token_env: "SLACK_BOT_TOKEN".to_string(),
        }
    }

    fn discord_config() -> MessagingRouteConfig {
        MessagingRouteConfig::Discord {
            channel_id: "999".to_string(),
            bot_token: None,
            bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
        }
    }

    // --- Route resolution ---

    #[test]
    fn repo_active_beats_user_active() {
        let messaging = RuntimeMessagingSettings {
            user: Some(ScopedMessagingSettings {
                active: Some("user-slack".to_string()),
                configs: HashMap::from([("user-slack".to_string(), slack_config())]),
            }),
            repo: Some(ScopedMessagingSettings {
                active: Some("repo-discord".to_string()),
                configs: HashMap::from([("repo-discord".to_string(), discord_config())]),
            }),
        };

        let resolved = resolve_effective_route(&messaging).unwrap();
        assert!(matches!(resolved.scope, MessagingScope::Repo));
        assert_eq!(resolved.name, "repo-discord");
    }

    #[test]
    fn repo_inactive_falls_back_to_user() {
        let messaging = RuntimeMessagingSettings {
            user: Some(ScopedMessagingSettings {
                active: Some("user-slack".to_string()),
                configs: HashMap::from([("user-slack".to_string(), slack_config())]),
            }),
            repo: Some(ScopedMessagingSettings {
                active: None,
                configs: HashMap::from([("repo-discord".to_string(), discord_config())]),
            }),
        };

        let resolved = resolve_effective_route(&messaging).unwrap();
        assert!(matches!(resolved.scope, MessagingScope::User));
        assert_eq!(resolved.name, "user-slack");
    }

    #[test]
    fn both_inactive_returns_none() {
        let messaging = RuntimeMessagingSettings {
            user: Some(ScopedMessagingSettings {
                active: None,
                configs: HashMap::new(),
            }),
            repo: Some(ScopedMessagingSettings {
                active: None,
                configs: HashMap::new(),
            }),
        };

        assert!(resolve_effective_route(&messaging).is_none());
    }

    #[test]
    fn no_scopes_returns_none() {
        let messaging = RuntimeMessagingSettings {
            user: None,
            repo: None,
        };

        assert!(resolve_effective_route(&messaging).is_none());
    }

    #[test]
    fn user_only_scope() {
        let messaging = RuntimeMessagingSettings {
            user: Some(ScopedMessagingSettings {
                active: Some("my-slack".to_string()),
                configs: HashMap::from([("my-slack".to_string(), slack_config())]),
            }),
            repo: None,
        };

        let resolved = resolve_effective_route(&messaging).unwrap();
        assert!(matches!(resolved.scope, MessagingScope::User));
        assert_eq!(resolved.name, "my-slack");
    }

    // --- Secret resolution ---

    #[test]
    fn inline_secret_wins_over_env() {
        let result = resolve_secret(Some("inline-value"), "UNUSED_ENV");
        assert_eq!(result.unwrap(), "inline-value");
    }

    #[test]
    fn env_fallback_works() {
        std::env::set_var("TEST_MSG_SECRET_1", "from-env");
        let result = resolve_secret(None, "TEST_MSG_SECRET_1");
        std::env::remove_var("TEST_MSG_SECRET_1");
        assert_eq!(result.unwrap(), "from-env");
    }

    #[test]
    fn missing_env_returns_error() {
        std::env::remove_var("NONEXISTENT_MSG_SECRET");
        let result = resolve_secret(None, "NONEXISTENT_MSG_SECRET");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("NONEXISTENT_MSG_SECRET"), "error: {err}");
    }

    // --- Signal recipient parsing ---

    #[test]
    fn signal_phone_recipient() {
        assert!(matches!(
            parse_signal_recipient("+15551234567"),
            SignalRecipient::Phone(p) if p == "+15551234567"
        ));
    }

    #[test]
    fn signal_group_recipient() {
        assert!(matches!(
            parse_signal_recipient("group-id-base64"),
            SignalRecipient::Group(g) if g == "group-id-base64"
        ));
    }

    // --- Image path resolution ---

    #[test]
    fn absolute_path_unchanged() {
        let resolved = resolve_image_path("/absolute/path.png", None, None);
        assert_eq!(resolved.to_str().unwrap(), "/absolute/path.png");
    }

    #[test]
    fn tilde_path_expands() {
        let resolved = resolve_image_path("~/images/shot.png", None, None);
        let home = dirs::home_dir().unwrap();
        assert_eq!(resolved, home.join("images/shot.png"));
    }

    #[test]
    fn relative_path_uses_cwd() {
        let resolved = resolve_image_path(
            "artifacts/shot.png",
            Some("/workspace/project"),
            None,
        );
        assert_eq!(
            resolved.to_str().unwrap(),
            "/workspace/project/artifacts/shot.png"
        );
    }

    #[test]
    fn relative_path_falls_back_to_repo_root() {
        let resolved = resolve_image_path(
            "artifacts/shot.png",
            None,
            Some("/repo/root"),
        );
        assert_eq!(
            resolved.to_str().unwrap(),
            "/repo/root/artifacts/shot.png"
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine -- messaging::resolve::tests`
Expected: FAIL — types and functions don't exist

- [ ] **Step 3: Implement resolve types and functions**

Add the implementation above the `#[cfg(test)]` block:

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::{debug, warn};

use super::config::{MessagingRouteConfig, ScopedMessagingSettings};

/// Which scope a resolved route came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagingScope {
    User,
    Repo,
}

/// A fully resolved messaging route with provenance.
#[derive(Debug, Clone)]
pub struct ResolvedMessagingRoute {
    pub scope: MessagingScope,
    pub name: String,
    pub config: MessagingRouteConfig,
}

/// Preserved user and repo messaging scopes for runtime resolution.
#[derive(Debug, Clone, Default)]
pub struct RuntimeMessagingSettings {
    pub user: Option<ScopedMessagingSettings>,
    pub repo: Option<ScopedMessagingSettings>,
}

/// Signal recipient type after parsing the recipient string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalRecipient {
    Phone(String),
    Group(String),
}

/// Resolve the effective messaging route from user and repo scopes.
///
/// 1. If repo scope has an active route, use it.
/// 2. Otherwise, if user scope has an active route, use it.
/// 3. Otherwise, return `None` (messaging disabled).
pub fn resolve_effective_route(messaging: &RuntimeMessagingSettings) -> Option<ResolvedMessagingRoute> {
    if let Some(route) = resolve_scope(&messaging.repo, MessagingScope::Repo) {
        return Some(route);
    }

    if let Some(route) = resolve_scope(&messaging.user, MessagingScope::User) {
        return Some(route);
    }

    debug!("No active messaging route in any scope");
    None
}

fn resolve_scope(
    scope: &Option<ScopedMessagingSettings>,
    scope_kind: MessagingScope,
) -> Option<ResolvedMessagingRoute> {
    let settings = scope.as_ref()?;
    let active = settings.active.as_ref()?;

    let config = settings.configs.get(active)?;

    Some(ResolvedMessagingRoute {
        scope: scope_kind,
        name: active.clone(),
        config: config.clone(),
    })
}

/// Resolve a secret from an inline value or an environment variable.
///
/// Returns the secret string or an error message naming the missing env var.
pub fn resolve_secret(inline: Option<&str>, env_name: &str) -> std::result::Result<String, String> {
    if let Some(value) = inline {
        if !value.is_empty() {
            return Ok(value.to_string());
        }
    }

    std::env::var(env_name).map_err(|_| {
        format!("missing secret: neither inline value nor env var {env_name} is set")
    })
}

/// Parse a Signal recipient string into phone or group.
///
/// Strings starting with `+` are treated as phone numbers.
/// Everything else is treated as a Signal group ID.
pub fn parse_signal_recipient(recipient: &str) -> SignalRecipient {
    if recipient.starts_with('+') {
        SignalRecipient::Phone(recipient.to_string())
    } else {
        SignalRecipient::Group(recipient.to_string())
    }
}

/// Resolve an image path to an absolute path.
///
/// 1. Absolute paths remain absolute.
/// 2. `~/...` expands via `dirs::home_dir()`.
/// 3. Relative paths resolve from `cwd` when available.
/// 4. Otherwise resolve from `repo_root`, then current working directory.
pub fn resolve_image_path(
    raw: &str,
    cwd: Option<&str>,
    repo_root: Option<&str>,
) -> PathBuf {
    let path = Path::new(raw);

    if path.is_absolute() {
        return path.to_path_buf();
    }

    if raw.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&raw[2..]);
        }
    }

    if let Some(cwd) = cwd {
        return PathBuf::from(cwd).join(raw);
    }

    if let Some(root) = repo_root {
        return PathBuf::from(root).join(raw);
    }

    // Final fallback: current working directory
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(raw)
}
```

- [ ] **Step 4: Uncomment resolve in mod.rs**

In `claudine/lib/src/messaging/mod.rs`, uncomment:

```rust
mod resolve;

pub use resolve::{
    MessagingScope, ResolvedMessagingRoute, RuntimeMessagingSettings, resolve_effective_route,
};
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p claudine -- messaging::resolve::tests`
Expected: all 11 tests PASS

- [ ] **Step 6: Commit**

```bash
git add claudine/lib/src/messaging/
git commit -m "feat(claudine): add messaging route resolution and secret lookup"
```

---

## Task 5: Send Helper (`messaging/send.rs`)

**Files:**
- Create: `claudine/lib/src/messaging/send.rs`
- Modify: `claudine/lib/src/messaging/mod.rs` (uncomment send)

- [ ] **Step 1: Write the send module**

Create `claudine/lib/src/messaging/send.rs`:

```rust
use messenger::provider::ProviderKind;
use secrecy::SecretString;
use tracing::{debug, warn};

use super::resolve::{
    ResolvedMessagingRoute, RuntimeMessagingSettings, SignalRecipient, parse_signal_recipient,
    resolve_effective_route, resolve_image_path, resolve_secret,
};
use super::config::MessagingRouteConfig;
use crate::dispatch::template::interpolate;
use crate::events::EventMeta;

/// Execute a messaging action (fire-and-forget).
///
/// Interpolates the message and image templates, resolves the active route,
/// builds the messenger payload, and spawns an async send task.
/// Errors are logged as warnings and never propagate to the caller.
pub fn execute_message(
    message_template: &str,
    image_template: Option<&str>,
    meta: &EventMeta,
    messaging: &RuntimeMessagingSettings,
) {
    let text = interpolate(message_template, meta);
    let image = image_template
        .map(|raw| interpolate(raw, meta))
        .filter(|value| !value.trim().is_empty());

    let Some(route) = resolve_effective_route(messaging) else {
        debug!("No active messaging route, skipping message action");
        return;
    };

    if text.trim().is_empty() && image.is_none() {
        debug!("Empty interpolated message and no image, skipping");
        return;
    }

    let cwd = meta.cwd.as_deref();
    let repo_root = meta.env.repo.root.as_deref();

    let payload = match build_payload(&route, &text, image.as_deref(), cwd, repo_root) {
        Some(payload) => payload,
        None => return,
    };

    tokio::spawn(async move {
        if let Err(error) = send_payload(route, payload).await {
            warn!(%error, "Messaging send failed");
        }
    });
}

/// Everything needed to send a message, built before entering the async task.
struct MessagePayload {
    message: messenger::Message,
    target: messenger::Target,
    provider_kind: ProviderKind,
    route_config: MessagingRouteConfig,
}

fn build_payload(
    route: &ResolvedMessagingRoute,
    text: &str,
    image: Option<&str>,
    cwd: Option<&str>,
    repo_root: Option<&str>,
) -> Option<MessagePayload> {
    let (target, provider_kind) = match &route.config {
        MessagingRouteConfig::Discord { channel_id, .. } => (
            messenger::Target::discord_channel(channel_id),
            ProviderKind::Discord,
        ),
        MessagingRouteConfig::Slack { channel_id, .. } => (
            messenger::Target::slack_channel(channel_id),
            ProviderKind::Slack,
        ),
        MessagingRouteConfig::Signal { recipient, .. } => {
            let target = match parse_signal_recipient(recipient) {
                SignalRecipient::Phone(phone) => {
                    messenger::Target::signal_user(messenger::target::SignalAddress::Phone(phone))
                }
                SignalRecipient::Group(group_id) => {
                    messenger::Target::signal_group(group_id)
                }
            };
            (target, ProviderKind::Signal)
        }
        MessagingRouteConfig::WhatsApp { recipient, .. } => (
            messenger::Target::whatsapp_recipient(recipient),
            ProviderKind::WhatsApp,
        ),
    };

    let mut msg = if text.trim().is_empty() {
        messenger::Message::text("")
    } else {
        messenger::Message::markdown(text)
    };

    // Only Discord supports image attachments in v1
    if let Some(image_path) = image {
        if provider_kind == ProviderKind::Discord {
            let resolved = resolve_image_path(image_path, cwd, repo_root);
            msg = msg.image(resolved);
        } else {
            warn!(
                provider = %provider_kind_label(provider_kind),
                "Image attachments not supported for this provider, sending text only"
            );
        }
    }

    Some(MessagePayload {
        message: msg,
        target,
        provider_kind,
        route_config: route.config.clone(),
    })
}

async fn send_payload(
    route: ResolvedMessagingRoute,
    payload: MessagePayload,
) -> std::result::Result<(), String> {
    let provider: Box<dyn messenger::Provider> = match &payload.route_config {
        MessagingRouteConfig::Discord { bot_token, bot_token_env, .. } => {
            let token = resolve_secret(bot_token.as_deref(), bot_token_env)
                .map_err(|e| format!("discord: {e}"))?;
            Box::new(messenger::provider::discord::DiscordProvider::new(
                messenger::provider::discord::DiscordConfig {
                    bot_token: SecretString::from(token),
                },
            ))
        }
        MessagingRouteConfig::Slack { bot_token, bot_token_env, .. } => {
            let token = resolve_secret(bot_token.as_deref(), bot_token_env)
                .map_err(|e| format!("slack: {e}"))?;
            Box::new(messenger::provider::slack::SlackProvider::new(
                messenger::provider::slack::SlackConfig {
                    bot_token: SecretString::from(token),
                    api_base_url: None,
                },
            ))
        }
        MessagingRouteConfig::Signal {
            rpc_url, rpc_url_env, account, account_env, ..
        } => {
            let url = resolve_secret(rpc_url.as_deref(), rpc_url_env)
                .map_err(|e| format!("signal: {e}"))?;
            let acct = resolve_secret(account.as_deref(), account_env)
                .map_err(|e| format!("signal: {e}"))?;
            Box::new(messenger::provider::signal::SignalProvider::new(
                messenger::provider::signal::SignalConfig {
                    rpc_url: url,
                    account: acct,
                },
            ))
        }
        MessagingRouteConfig::WhatsApp {
            access_token, access_token_env, phone_number_id, phone_number_id_env, ..
        } => {
            let token = resolve_secret(access_token.as_deref(), access_token_env)
                .map_err(|e| format!("whatsapp: {e}"))?;
            let phone_id = resolve_secret(phone_number_id.as_deref(), phone_number_id_env)
                .map_err(|e| format!("whatsapp: {e}"))?;
            Box::new(messenger::provider::whatsapp::WhatsAppProvider::new(
                messenger::provider::whatsapp::WhatsAppConfig {
                    access_token: SecretString::from(token),
                    phone_number_id: phone_id,
                    api_version: None,
                    api_base_url: None,
                },
            ))
        }
    };

    let mut messenger_instance = messenger::Messenger::new();
    messenger_instance.register(provider);

    let dispatch = messenger::Dispatch::to(payload.target);

    let plan = messenger_instance
        .plan_send(dispatch, &payload.message)
        .map_err(|e| format!("plan_send failed: {e}"))?;

    for warning in &plan.warnings {
        warn!(%warning, "Messaging compatibility warning");
    }

    messenger_instance
        .send_planned(plan)
        .await
        .map_err(|e| format!("send failed: {e}"))?;

    debug!(
        route = %route.name,
        scope = ?route.scope,
        "Message sent successfully"
    );

    Ok(())
}

fn provider_kind_label(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Discord => "discord",
        ProviderKind::Slack => "slack",
        ProviderKind::Signal => "signal",
        ProviderKind::WhatsApp => "whatsapp",
        ProviderKind::Telegram => "telegram",
    }
}
```

- [ ] **Step 2: Uncomment send in mod.rs**

In `claudine/lib/src/messaging/mod.rs`, uncomment the send lines. The final `mod.rs` should be:

```rust
mod config;
mod resolve;
mod send;

pub use config::{MessagingRouteConfig, ScopedMessagingSettings};
pub use resolve::{
    MessagingScope, ResolvedMessagingRoute, RuntimeMessagingSettings, resolve_effective_route,
};
pub use send::execute_message;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p claudine`
Expected: compiles (may need minor adjustments to messenger import paths if provider modules have different visibility — follow compiler guidance)

- [ ] **Step 4: Commit**

```bash
git add claudine/lib/src/messaging/
git commit -m "feat(claudine): add messaging send helper with provider building"
```

---

## Task 6: Wire `GlobalSettings` and Config Validation

**Files:**
- Modify: `claudine/lib/src/events/config.rs:40-61` (GlobalSettings)
- Modify: `claudine/lib/src/events/config.rs:137-162` (validate)

- [ ] **Step 1: Write failing test for GlobalSettings messaging field**

Add to the existing test module in `config.rs`:

```rust
#[test]
fn global_settings_with_messaging() {
    let json = serde_json::json!({
        "messaging": {
            "active": "ops",
            "configs": {
                "ops": {
                    "provider": "slack",
                    "channel_id": "C123"
                }
            }
        }
    });

    let settings: GlobalSettings = serde_json::from_value(json).unwrap();
    let messaging = settings.messaging.unwrap();
    assert_eq!(messaging.active.as_deref(), Some("ops"));
}

#[test]
fn global_settings_without_messaging() {
    let json = serde_json::json!({});
    let settings: GlobalSettings = serde_json::from_value(json).unwrap();
    assert!(settings.messaging.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine -- events::config::tests::global_settings_with_messaging events::config::tests::global_settings_without_messaging`
Expected: FAIL — no `messaging` field on `GlobalSettings`

- [ ] **Step 3: Add messaging field to GlobalSettings**

In `claudine/lib/src/events/config.rs`, add the import at the top:

```rust
use crate::messaging::ScopedMessagingSettings;
```

Add the field to `GlobalSettings`:

```rust
    /// Messaging destination settings for `Message` actions.
    #[serde(default)]
    pub messaging: Option<ScopedMessagingSettings>,
```

- [ ] **Step 4: Add messaging validation to HookerConfig::validate()**

In the `validate()` method, add after the protect validation block:

```rust
        if let Some(messaging) = self.settings.messaging.as_ref() {
            messaging.validate("config").map_err(|error| {
                ClaudineError::ConfigValidation(format!("invalid settings.messaging: {error}"))
            })?;
        }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p claudine -- events::config::tests`
Expected: all tests PASS (including existing tests)

- [ ] **Step 6: Fix merge_configs for messaging**

In `claudine/lib/src/dispatch/loader.rs`, update the `merge_configs` function to handle the new field. In the `GlobalSettings` construction block, add:

```rust
        messaging: repo.settings.messaging.or(user.settings.messaging),
```

Note: This merged field is used for non-messaging callers. The scope-aware resolution in Task 7 will bypass this.

- [ ] **Step 7: Verify full build**

Run: `cargo check -p claudine`
Expected: compiles clean

- [ ] **Step 8: Commit**

```bash
git add claudine/lib/src/events/config.rs claudine/lib/src/dispatch/loader.rs
git commit -m "feat(claudine): add messaging to GlobalSettings and config validation"
```

---

## Task 7: Wire Loader for Scope-Aware Messaging

**Files:**
- Modify: `claudine/lib/src/dispatch/loader.rs:22-58` (RuntimeConfig) and `load_config`/`load_runtime_config`

- [ ] **Step 1: Write failing test for scope-aware config loading**

Add to the `mod tests` in `loader.rs`:

```rust
#[test]
fn runtime_config_preserves_messaging_scopes() {
    let user_config = HookerConfig {
        version: "1.0".to_string(),
        settings: GlobalSettings {
            messaging: Some(crate::messaging::ScopedMessagingSettings {
                active: Some("my-slack".to_string()),
                configs: std::collections::HashMap::from([(
                    "my-slack".to_string(),
                    crate::messaging::MessagingRouteConfig::Slack {
                        channel_id: "C123".to_string(),
                        bot_token: None,
                        bot_token_env: "SLACK_BOT_TOKEN".to_string(),
                    },
                )]),
            }),
            ..Default::default()
        },
        providers: std::collections::HashMap::new(),
    };

    let repo_config = HookerConfig {
        version: "1.0".to_string(),
        settings: GlobalSettings {
            messaging: Some(crate::messaging::ScopedMessagingSettings {
                active: Some("alerts".to_string()),
                configs: std::collections::HashMap::from([(
                    "alerts".to_string(),
                    crate::messaging::MessagingRouteConfig::Discord {
                        channel_id: "999".to_string(),
                        bot_token: None,
                        bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                    },
                )]),
            }),
            ..Default::default()
        },
        providers: std::collections::HashMap::new(),
    };

    let merged = merge_configs(user_config.clone(), repo_config.clone());
    let runtime = compile_runtime_config_with_messaging(
        merged,
        user_config.settings.messaging,
        repo_config.settings.messaging,
    )
    .unwrap();

    let messaging = runtime.messaging();
    assert!(messaging.user.is_some());
    assert!(messaging.repo.is_some());
    assert_eq!(
        messaging.repo.as_ref().unwrap().active.as_deref(),
        Some("alerts")
    );
    assert_eq!(
        messaging.user.as_ref().unwrap().active.as_deref(),
        Some("my-slack")
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p claudine -- dispatch::loader::tests::runtime_config_preserves_messaging_scopes`
Expected: FAIL — no `messaging` field on `RuntimeConfig`, no `compile_runtime_config_with_messaging`

- [ ] **Step 3: Add messaging to RuntimeConfig**

In `loader.rs`, modify the `RuntimeConfig` struct:

```rust
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    settings: GlobalSettings,
    messaging: RuntimeMessagingSettings,
    providers: HashMap<Provider, RuntimeProviderConfig>,
}
```

Add the import at the top of `loader.rs`:

```rust
use crate::messaging::RuntimeMessagingSettings;
```

Add the accessor method to `impl RuntimeConfig`:

```rust
    pub fn messaging(&self) -> &RuntimeMessagingSettings {
        &self.messaging
    }
```

- [ ] **Step 4: Update compile_runtime_config to accept messaging scopes**

Rename the existing `compile_runtime_config` to a private helper and create a new flow. Replace the function:

```rust
fn compile_runtime_config(config: HookerConfig) -> Result<RuntimeConfig> {
    compile_runtime_config_with_messaging(config, None, None)
}

fn compile_runtime_config_with_messaging(
    config: HookerConfig,
    user_messaging: Option<crate::messaging::ScopedMessagingSettings>,
    repo_messaging: Option<crate::messaging::ScopedMessagingSettings>,
) -> Result<RuntimeConfig> {
    let HookerConfig {
        version: _,
        settings,
        providers,
    } = config;

    let mut runtime_providers = HashMap::new();

    for (provider, provider_config) in providers {
        let mut runtime_events = HashMap::new();

        for (event, binding) in provider_config.events {
            let matcher = binding
                .matcher
                .as_deref()
                .map(|pattern| {
                    Regex::new(pattern).map_err(|error| {
                        ClaudineError::TemplateError(format!(
                            "invalid matcher regex for provider={provider} event={event}: {error} ({pattern})"
                        ))
                    })
                })
                .transpose()?;

            let compiled_mappers = binding
                .actions
                .iter()
                .map(|action| compile_action_mapper(action, provider, event))
                .collect::<Result<Vec<_>>>()?;

            runtime_events.insert(
                event,
                RuntimeEventBinding {
                    enabled: binding.enabled,
                    actions: binding.actions,
                    matcher,
                    compiled_mappers,
                },
            );
        }

        runtime_providers.insert(
            provider,
            RuntimeProviderConfig {
                events: runtime_events,
            },
        );
    }

    Ok(RuntimeConfig {
        settings,
        messaging: RuntimeMessagingSettings {
            user: user_messaging,
            repo: repo_messaging,
        },
        providers: runtime_providers,
    })
}
```

- [ ] **Step 5: Update load_runtime_config to preserve messaging scopes**

Replace `load_runtime_config`:

```rust
pub fn load_runtime_config(user: Option<&Path>, repo_root: Option<&Path>) -> Result<RuntimeConfig> {
    let user_config = load_user_config(user)?;
    let repo_config = load_repo_config(repo_root)?;

    // Capture messaging scopes before merging
    let user_messaging = user_config
        .as_ref()
        .and_then(|c| c.settings.messaging.clone());
    let repo_messaging = repo_config
        .as_ref()
        .and_then(|c| c.settings.messaging.clone());

    // Validate messaging per-scope before merging
    if let Some(ref messaging) = user_messaging {
        messaging.validate("user")?;
    }
    if let Some(ref messaging) = repo_messaging {
        messaging.validate("repo")?;
    }

    let config = match (user_config, repo_config) {
        (Some(user_cfg), Some(repo_cfg)) => {
            debug!("Merging user and repo configurations");
            merge_configs(user_cfg, repo_cfg)
        }
        (Some(cfg), None) => cfg,
        (None, Some(cfg)) => cfg,
        (None, None) => {
            let path = user.map(PathBuf::from).unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("~"))
                    .join(USER_CONFIG_NAMES[0])
            });
            return Err(ClaudineError::ConfigNotFound(path));
        }
    };

    config.validate()?;
    compile_runtime_config_with_messaging(config, user_messaging, repo_messaging)
}
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p claudine -- dispatch::loader::tests::runtime_config_preserves_messaging_scopes`
Expected: PASS

- [ ] **Step 7: Run full test suite to check for regressions**

Run: `cargo test -p claudine`
Expected: all existing tests PASS

- [ ] **Step 8: Commit**

```bash
git add claudine/lib/src/dispatch/loader.rs
git commit -m "feat(claudine): preserve messaging scopes separately in RuntimeConfig"
```

---

## Task 8: Wire Dispatch Runner

**Files:**
- Modify: `claudine/lib/src/dispatch/runner.rs:23-30` (execute_actions signature)
- Modify: `claudine/lib/src/dispatch/mod.rs:214-222` (call site)

- [ ] **Step 1: Write failing test for Message action dispatch**

Add to the `mod tests` in `runner.rs`:

```rust
#[tokio::test]
async fn message_action_skipped_when_no_route() {
    let actions = vec![HookAction::Message {
        message: "test notification".to_string(),
        image: None,
    }];

    let messaging = crate::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };

    // Should not error — message is silently skipped when no route configured
    let result = execute_actions(
        &actions,
        None,
        &meta(),
        &GlobalSettings::default(),
        &messaging,
        false,
        None,
    )
    .await
    .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn message_action_does_not_block() {
    // A Message action should never produce a HookResponse, even with can_block = true
    let actions = vec![
        HookAction::Message {
            message: "notify".to_string(),
            image: None,
        },
        HookAction::Log {
            target: LogTarget::File {
                path: None,
                rotate_daily: false,
            },
        },
    ];

    let messaging = crate::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };

    // This should complete without error (message silently skipped, log executes)
    let result = execute_actions(
        &actions,
        None,
        &meta(),
        &GlobalSettings::default(),
        &messaging,
        true,
        None,
    )
    .await;

    assert!(result.is_ok());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine -- dispatch::runner::tests::message_action`
Expected: FAIL — `execute_actions` doesn't have `messaging` parameter

- [ ] **Step 3: Add messaging parameter to execute_actions**

Update the function signature in `runner.rs`:

```rust
pub async fn execute_actions(
    actions: &[HookAction],
    compiled_mappers: Option<&[Option<CompiledMapper>]>,
    meta: &EventMeta,
    settings: &GlobalSettings,
    messaging: &crate::messaging::RuntimeMessagingSettings,
    can_block: bool,
    protect_decision: Option<&ProtectDecision>,
) -> Result<Option<HookResponse>>
```

- [ ] **Step 4: Replace the temporary Message match arm**

In the match block inside `execute_actions`, replace the debug placeholder with:

```rust
HookAction::Message { message, image } => {
    crate::messaging::execute_message(
        message,
        image.as_deref(),
        meta,
        messaging,
    );
}
```

- [ ] **Step 5: Update the call site in dispatch/mod.rs**

In `claudine/lib/src/dispatch/mod.rs`, update the `execute_actions` call (around line 214):

```rust
    let action_response = runner::execute_actions(
        &resolved_hook.actions,
        Some(binding.compiled_mappers()),
        &resolved_hook.meta,
        config.settings(),
        config.messaging(),
        resolved_hook.can_block,
        protect_pre_decision.as_ref(),
    )
    .await?;
```

- [ ] **Step 6: Fix all existing execute_actions callers**

Search for other call sites of `execute_actions` that need the new parameter. Each existing test call needs the messaging parameter added. Update them all to pass `&crate::messaging::RuntimeMessagingSettings::default()` (or `&messaging` if a local is defined).

For example, in `runner.rs` tests, the existing `log_file_writes_jsonl` test becomes:

```rust
execute_actions(
    &actions,
    None,
    &meta(),
    &GlobalSettings::default(),
    &crate::messaging::RuntimeMessagingSettings::default(),
    false,
    None,
)
.await
.unwrap();
```

Apply this pattern to all existing test calls of `execute_actions` in the runner tests.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p claudine`
Expected: all tests PASS

- [ ] **Step 8: Commit**

```bash
git add claudine/lib/src/dispatch/runner.rs claudine/lib/src/dispatch/mod.rs
git commit -m "feat(claudine): wire Message action into dispatch runner"
```

---

## Task 9: Documentation Updates

**Files:**
- Modify: `claudine/docs/topics/configuring-actions.md` (if it exists)
- Modify: `claudine/features/2026-04-01-messaging/spec.md`

- [ ] **Step 1: Check if configuring-actions.md exists**

Run: `ls claudine/docs/topics/configuring-actions.md 2>/dev/null && echo EXISTS || echo MISSING`

If EXISTS, add the `message` action documentation following the existing patterns for other actions. The entry should include:

```markdown
### Message

Sends a message to the configured messaging destination (Slack, Discord, Signal, or WhatsApp).

| Field     | Type   | Required | Description |
|-----------|--------|----------|-------------|
| `message` | string | yes      | Template message with `{{variable}}` placeholders |
| `image`   | string | no       | File path to a raster image (Discord only in v1) |

```json
{
  "type": "message",
  "message": "**{{provider}}** `{{event}}` in `{{cwd}}`",
  "image": "{{cwd}}/.claudine/artifacts/last-run.png"
}
```

Messaging must be configured in `settings.messaging` (see below).
```

Also add a section documenting the `settings.messaging` configuration schema with the provider config examples from the tech design.

- [ ] **Step 2: Update the spec to match implementation**

In `claudine/features/2026-04-01-messaging/spec.md`, replace the webhook-based Slack/Discord examples (lines 31-46) with the bot-token-based config that matches the `messenger` library:

```json
"messaging": {
  "active": "work-slack",
  "configs": {
    "work-slack": {
      "provider": "slack",
      "channel_id": "C012345ABC",
      "bot_token_env": "SLACK_BOT_TOKEN"
    },
    "personal-discord": {
      "provider": "discord",
      "channel_id": "123456789012345678",
      "bot_token_env": "DISCORD_BOT_TOKEN"
    }
  }
}
```

Add a note about env-backed secrets:

> Secrets can be provided inline (`bot_token`) or via environment variable name (`bot_token_env`). Inline values take precedence. Default env var names follow the `messenger` CLI conventions.

- [ ] **Step 3: Commit**

```bash
git add claudine/docs/ claudine/features/2026-04-01-messaging/spec.md
git commit -m "docs(claudine): add messaging action and config documentation"
```

---

## Task 10: Final Verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test -p claudine`
Expected: all tests PASS

- [ ] **Step 2: Run lints**

Run: `cargo clippy -p claudine -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Run full build**

Run: `cargo build -p claudine`
Expected: builds clean

- [ ] **Step 4: Verify config round-trip with sample JSON**

Create a quick manual test by adding a temporary test (or running in the test suite) that parses the full example config from the tech design:

```rust
#[test]
fn full_config_example_parses() {
    let json = serde_json::json!({
        "version": "1.0",
        "settings": {
            "messaging": {
                "active": "work-slack",
                "configs": {
                    "work-slack": {
                        "provider": "slack",
                        "channel_id": "C012345ABC",
                        "bot_token_env": "SLACK_BOT_TOKEN"
                    },
                    "personal-discord": {
                        "provider": "discord",
                        "channel_id": "123456789012345678",
                        "bot_token_env": "DISCORD_BOT_TOKEN"
                    }
                }
            }
        },
        "providers": {}
    });

    let config: crate::events::HookerConfig = serde_json::from_value(json).unwrap();
    let messaging = config.settings.messaging.unwrap();
    assert_eq!(messaging.active.as_deref(), Some("work-slack"));
    assert_eq!(messaging.configs.len(), 2);
    config.validate().unwrap();
}
```

Run: `cargo test -p claudine -- full_config_example_parses`
Expected: PASS

- [ ] **Step 5: Commit final test**

```bash
git add claudine/
git commit -m "test(claudine): add full config example parse test for messaging"
```

---

## Summary

| Task | Description | New Files | Tests |
|------|-------------|-----------|-------|
| 1 | Add dependencies | — | build check |
| 2 | `HookAction::Message` variant | — | 4 serde/type tests |
| 3 | Config types (`messaging/config.rs`) | `messaging/mod.rs`, `messaging/config.rs` | 11 serde/validation tests |
| 4 | Route resolution (`messaging/resolve.rs`) | `messaging/resolve.rs` | 11 resolution/secret/path tests |
| 5 | Send helper (`messaging/send.rs`) | `messaging/send.rs` | build check |
| 6 | GlobalSettings + validation wiring | — | 2 settings tests |
| 7 | Loader scope preservation | — | 1 loader test |
| 8 | Dispatch runner wiring | — | 2 dispatch tests |
| 9 | Documentation | — | — |
| 10 | Final verification | — | 1 integration test |

**Total: 32+ tests, 4 new files, 7 modified files, 10 commits**
