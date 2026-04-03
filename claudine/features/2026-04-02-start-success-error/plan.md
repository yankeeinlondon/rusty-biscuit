# Composition Lifecycle Notifications Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `start`, `success`, `blocked`, and `failure` lifecycle notification properties to composition frontmatter, with audio-ordered TTS/sound-effect playback, stderr status rendering, and messaging dispatch.

**Architecture:** Parse typed lifecycle configs from effective frontmatter on `PreparedComposition`. A single reusable emitter handles audio ordering (effect-before-speech or speech-before-effect) plus best-effort stderr/messaging fan-out. Integration points are wired into the three composition execution paths: harness loop, direct-without-harness, and inline-without-harness.

**Tech Stack:** Rust, serde/serde_json, biscuit-speaks (TTS), playa (sound effects), biscuit-terminal (Status rendering), messenger (outbound messaging), tokio (async runtime)

---

## File Structure

### New files

| File | Responsibility |
|------|---------------|
| `claudine/lib/src/composition/lifecycle.rs` | Typed parsing, validation, audio-order planning, and best-effort emission API |

### Modified files

| File | Changes |
|------|---------|
| `claudine/lib/src/composition/mod.rs` | Add `pub mod lifecycle;` and re-export public types |
| `claudine/lib/src/composition/types.rs` | Add `lifecycle` field to `PreparedComposition` |
| `claudine/lib/src/composition/error.rs` | Add lifecycle validation error variants |
| `claudine/lib/src/composition/prepare.rs` | Parse lifecycle config during `prepare_direct` and `prepare_inline` |
| `claudine/lib/src/messaging/send.rs` | Add `execute_resolved_message()` that accepts pre-rendered text (no `EventMeta`) |
| `claudine/cli/src/commands/wrap/composition.rs` | Load runtime settings, create lifecycle state, emit `start`/`blocked`/`success`/`failure` in all three paths |
| `claudine/cli/src/commands/wrap/mod.rs` | Accept lifecycle params in `run_harness_loop`, emit lifecycle signals at trigger points |

---

## Task 1: Lifecycle types and parsing

**Files:**
- Create: `claudine/lib/src/composition/lifecycle.rs`
- Modify: `claudine/lib/src/composition/mod.rs`
- Modify: `claudine/lib/src/composition/error.rs`

- [ ] **Step 1: Add error variants for lifecycle validation**

In `claudine/lib/src/composition/error.rs`, add two new variants to `CompositionError`:

```rust
    /// A lifecycle notification property has both `speak` and `speak_first`.
    #[error("lifecycle property `{0}` has both `speak` and `speak_first`; only one is allowed")]
    LifecycleSpeakConflict(String),

    /// A lifecycle notification property references an unknown sound effect.
    #[error("lifecycle property `{0}` references unknown sound effect `{1}`")]
    LifecycleUnknownEffect(String, String),
```

- [ ] **Step 2: Run the build to confirm the new variants compile**

Run: `cargo check -p claudine`
Expected: compiles with warnings about unused variants

- [ ] **Step 3: Create lifecycle.rs with types and parsing**

Create `claudine/lib/src/composition/lifecycle.rs`:

```rust
//! Lifecycle notification configuration and emission for composition workflows.
//!
//! Parses `start`, `success`, `blocked`, and `failure` from effective frontmatter
//! and provides a best-effort emission API with deterministic audio ordering.

use std::path::Path;

use biscuit_speaks::{SpeedLevel, TtsConfig, TtsFailoverStrategy};
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::Renderable;
use biscuit_terminal::terminal::Terminal;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::error::CompositionError;
use crate::events::{GlobalSettings, TtsSettings};
use crate::messaging::RuntimeMessagingSettings;

/// Configuration for a single lifecycle notification (start, success, blocked, or failure).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleNotification {
    /// Text to speak via TTS after the sound effect (default audio order).
    #[serde(default)]
    pub speak: Option<String>,
    /// Text to speak via TTS before the sound effect (reversed audio order).
    #[serde(default)]
    pub speak_first: Option<String>,
    /// Named sound effect to play (must be a valid `playa::SoundEffect` name).
    #[serde(default)]
    pub effect: Option<String>,
    /// Message text to send via configured messaging route.
    #[serde(default)]
    pub message: Option<String>,
    /// Prose markup to render to stderr via `Status`.
    #[serde(default)]
    pub stderr: Option<String>,
}

/// Lifecycle notification configuration parsed from effective frontmatter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleConfig {
    #[serde(default)]
    pub start: Option<LifecycleNotification>,
    #[serde(default)]
    pub success: Option<LifecycleNotification>,
    #[serde(default)]
    pub blocked: Option<LifecycleNotification>,
    #[serde(default)]
    pub failure: Option<LifecycleNotification>,
}

/// Which lifecycle signal to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleSignal {
    Start,
    Success,
    Blocked,
    Failure,
}

impl LifecycleSignal {
    /// The frontmatter property name for this signal.
    fn property_name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Success => "success",
            Self::Blocked => "blocked",
            Self::Failure => "failure",
        }
    }

    /// The `StatusState` used when rendering `stderr` text.
    fn status_state(self) -> StatusState {
        match self {
            Self::Start => StatusState::Info,
            Self::Success => StatusState::Success,
            Self::Blocked => StatusState::Failure,
            Self::Failure => StatusState::Failure,
        }
    }
}

/// Tracks whether the provider has launched (for blocked vs failure classification).
#[derive(Debug, Default)]
pub struct LifecycleRuntimeState {
    pub start_emitted: bool,
    pub provider_launch_started: bool,
}

/// Runtime context needed by the lifecycle emitter.
pub struct LifecycleRuntimeContext<'a> {
    pub settings: &'a GlobalSettings,
    pub messaging: &'a RuntimeMessagingSettings,
    pub term: &'a Terminal,
    pub source_path: &'a Path,
    pub repo_root: Option<&'a Path>,
}

/// Parse lifecycle config from effective frontmatter.
///
/// Extracts `start`, `success`, `blocked`, and `failure` properties from the
/// top-level frontmatter object. Returns `LifecycleConfig::default()` if none
/// are present.
///
/// ## Errors
///
/// Returns `CompositionError` if:
/// - A lifecycle property has both `speak` and `speak_first`
/// - A lifecycle property references an unknown sound effect name
/// - A lifecycle property contains unknown keys
pub fn parse_lifecycle_config(
    frontmatter: &serde_json::Value,
) -> Result<LifecycleConfig, CompositionError> {
    let obj = match frontmatter.as_object() {
        Some(obj) => obj,
        None => return Ok(LifecycleConfig::default()),
    };

    let mut config = LifecycleConfig::default();

    for (key, signal_field) in [
        ("start", &mut config.start),
        ("success", &mut config.success),
        ("blocked", &mut config.blocked),
        ("failure", &mut config.failure),
    ] {
        if let Some(value) = obj.get(key) {
            if value.is_null() {
                continue;
            }
            let notification: LifecycleNotification = serde_json::from_value(value.clone())
                .map_err(|e| {
                    CompositionError::LifecycleSpeakConflict(format!("{key}: {e}"))
                })?;
            validate_notification(key, &notification)?;
            *signal_field = Some(normalize_notification(notification));
        }
    }

    Ok(config)
}

/// Validate a single lifecycle notification.
fn validate_notification(
    property: &str,
    notification: &LifecycleNotification,
) -> Result<(), CompositionError> {
    // speak and speak_first are mutually exclusive
    if notification.speak.is_some() && notification.speak_first.is_some() {
        return Err(CompositionError::LifecycleSpeakConflict(
            property.to_string(),
        ));
    }

    // Validate effect name if present
    if let Some(effect_name) = &notification.effect {
        let trimmed = effect_name.trim();
        if !trimmed.is_empty() && playa::SoundEffect::from_name(trimmed).is_none() {
            return Err(CompositionError::LifecycleUnknownEffect(
                property.to_string(),
                trimmed.to_string(),
            ));
        }
    }

    Ok(())
}

/// Normalize empty strings to `None`.
fn normalize_notification(mut n: LifecycleNotification) -> LifecycleNotification {
    fn normalize_opt(opt: &mut Option<String>) {
        if let Some(s) = opt {
            if s.trim().is_empty() {
                *opt = None;
            }
        }
    }
    normalize_opt(&mut n.speak);
    normalize_opt(&mut n.speak_first);
    normalize_opt(&mut n.effect);
    normalize_opt(&mut n.message);
    normalize_opt(&mut n.stderr);
    n
}

impl LifecycleConfig {
    /// Get the notification config for a specific signal, if defined.
    pub fn get(&self, signal: LifecycleSignal) -> Option<&LifecycleNotification> {
        match signal {
            LifecycleSignal::Start => self.start.as_ref(),
            LifecycleSignal::Success => self.success.as_ref(),
            LifecycleSignal::Blocked => self.blocked.as_ref(),
            LifecycleSignal::Failure => self.failure.as_ref(),
        }
    }

    /// Returns `true` if no lifecycle notifications are configured.
    pub fn is_empty(&self) -> bool {
        self.start.is_none()
            && self.success.is_none()
            && self.blocked.is_none()
            && self.failure.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_valid_lifecycle_config() {
        let fm = json!({
            "start": {
                "speak": "Starting now",
                "effect": "doorbell",
                "stderr": "Starting"
            },
            "success": {
                "speak_first": "Done!",
                "effect": "confirmation",
                "message": "Completed"
            }
        });
        let config = parse_lifecycle_config(&fm).unwrap();
        assert!(config.start.is_some());
        assert!(config.success.is_some());
        assert!(config.blocked.is_none());
        assert!(config.failure.is_none());

        let start = config.start.unwrap();
        assert_eq!(start.speak.as_deref(), Some("Starting now"));
        assert_eq!(start.effect.as_deref(), Some("doorbell"));
        assert_eq!(start.stderr.as_deref(), Some("Starting"));
    }

    #[test]
    fn rejects_both_speak_and_speak_first() {
        let fm = json!({
            "start": {
                "speak": "Hello",
                "speak_first": "Also hello"
            }
        });
        let err = parse_lifecycle_config(&fm).unwrap_err();
        assert!(matches!(err, CompositionError::LifecycleSpeakConflict(_)));
    }

    #[test]
    fn trims_empty_strings_to_none() {
        let fm = json!({
            "start": {
                "speak": "  ",
                "effect": "",
                "message": "  \n  ",
                "stderr": ""
            }
        });
        let config = parse_lifecycle_config(&fm).unwrap();
        let start = config.start.unwrap();
        assert!(start.speak.is_none());
        assert!(start.effect.is_none());
        assert!(start.message.is_none());
        assert!(start.stderr.is_none());
    }

    #[test]
    fn rejects_unknown_keys() {
        let fm = json!({
            "start": {
                "speak": "Hello",
                "unknown_key": "bad"
            }
        });
        let err = parse_lifecycle_config(&fm).unwrap_err();
        // serde deny_unknown_fields triggers this
        assert!(matches!(err, CompositionError::LifecycleSpeakConflict(_)));
    }

    #[test]
    fn rejects_unknown_effect_name() {
        let fm = json!({
            "start": {
                "effect": "nonexistent-sound-that-will-never-exist"
            }
        });
        let err = parse_lifecycle_config(&fm).unwrap_err();
        assert!(matches!(err, CompositionError::LifecycleUnknownEffect(_, _)));
    }

    #[test]
    fn speak_plus_effect_is_valid() {
        let fm = json!({
            "start": {
                "speak": "Hello",
                "effect": "doorbell"
            }
        });
        let config = parse_lifecycle_config(&fm).unwrap();
        let start = config.start.unwrap();
        assert_eq!(start.speak.as_deref(), Some("Hello"));
        assert_eq!(start.effect.as_deref(), Some("doorbell"));
    }

    #[test]
    fn speak_first_plus_effect_is_valid() {
        let fm = json!({
            "start": {
                "speak_first": "Hello",
                "effect": "doorbell"
            }
        });
        let config = parse_lifecycle_config(&fm).unwrap();
        let start = config.start.unwrap();
        assert_eq!(start.speak_first.as_deref(), Some("Hello"));
        assert_eq!(start.effect.as_deref(), Some("doorbell"));
    }

    #[test]
    fn empty_frontmatter_returns_default() {
        let fm = json!({});
        let config = parse_lifecycle_config(&fm).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn non_object_frontmatter_returns_default() {
        let fm = json!("just a string");
        let config = parse_lifecycle_config(&fm).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn null_lifecycle_property_is_skipped() {
        let fm = json!({
            "start": null,
            "success": { "stderr": "Done" }
        });
        let config = parse_lifecycle_config(&fm).unwrap();
        assert!(config.start.is_none());
        assert!(config.success.is_some());
    }

    #[test]
    fn frontmatter_with_non_lifecycle_keys_is_fine() {
        let fm = json!({
            "title": "My Document",
            "agent": "claude",
            "start": { "stderr": "Starting" }
        });
        let config = parse_lifecycle_config(&fm).unwrap();
        assert!(config.start.is_some());
    }
}
```

- [ ] **Step 4: Wire the module into mod.rs**

In `claudine/lib/src/composition/mod.rs`, add:

```rust
pub mod lifecycle;
```

after the existing `pub mod closure;` line. Then add to the re-exports:

```rust
pub use lifecycle::{
    LifecycleConfig, LifecycleNotification, LifecycleRuntimeContext, LifecycleRuntimeState,
    LifecycleSignal, parse_lifecycle_config,
};
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p claudine --lib composition::lifecycle`
Expected: all 10 tests pass

- [ ] **Step 6: Commit**

```bash
git add claudine/lib/src/composition/lifecycle.rs claudine/lib/src/composition/mod.rs claudine/lib/src/composition/error.rs
git commit -m "feat(claudine): add lifecycle notification types and parsing

Add LifecycleConfig, LifecycleNotification, and parse_lifecycle_config()
with validation for speak/speak_first mutual exclusivity, effect name
validation, empty string normalization, and unknown key rejection."
```

---

## Task 2: Wire lifecycle config into PreparedComposition

**Files:**
- Modify: `claudine/lib/src/composition/types.rs`
- Modify: `claudine/lib/src/composition/prepare.rs`

- [ ] **Step 1: Write failing test in prepare.rs**

Add this test to the existing `#[cfg(test)] mod tests` block in `claudine/lib/src/composition/prepare.rs`:

```rust
    #[test]
    fn direct_composition_parses_lifecycle_config() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("title", json!("Test")),
                (
                    "start",
                    json!({"stderr": "Starting", "effect": "doorbell"}),
                ),
                ("success", json!({"speak": "All done"})),
            ],
            "Do the work.",
        );

        let prepared = prepare_direct(&source, None, None).unwrap();
        assert!(prepared.lifecycle.start.is_some());
        assert!(prepared.lifecycle.success.is_some());
        assert!(prepared.lifecycle.blocked.is_none());
        assert!(prepared.lifecycle.failure.is_none());
    }

    #[test]
    fn inline_composition_parses_lifecycle_config() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("prompt", json!("Write something")),
                ("failure", json!({"stderr": "Failed"})),
            ],
            "Old content",
        );

        let prepared = prepare_inline(&source, None, None).unwrap();
        assert!(prepared.lifecycle.failure.is_some());
        assert!(prepared.lifecycle.start.is_none());
    }

    #[test]
    fn invalid_lifecycle_config_fails_preparation() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("title", json!("Test")),
                (
                    "start",
                    json!({"speak": "Hello", "speak_first": "Also hello"}),
                ),
            ],
            "Content",
        );

        let err = prepare_direct(&source, None, None).unwrap_err();
        assert!(matches!(err, CompositionError::LifecycleSpeakConflict(_)));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine --lib composition::prepare`
Expected: FAIL — `PreparedComposition` has no `lifecycle` field

- [ ] **Step 3: Add lifecycle field to PreparedComposition**

In `claudine/lib/src/composition/types.rs`, add the import and field:

Add to imports at the top:

```rust
use super::lifecycle::LifecycleConfig;
```

Add to `PreparedComposition` struct, after the `closure` field:

```rust
    /// Parsed lifecycle notification config from effective frontmatter.
    pub lifecycle: LifecycleConfig,
```

- [ ] **Step 4: Update prepare_direct to parse lifecycle**

In `claudine/lib/src/composition/prepare.rs`, add the import:

```rust
use super::lifecycle::parse_lifecycle_config;
```

In `prepare_direct()`, after `let effective_agent_hint = ...` (line 50), add:

```rust
    let lifecycle = parse_lifecycle_config(&effective_frontmatter)?;
```

Then add `lifecycle` to the `PreparedComposition` construction:

```rust
    Ok(PreparedComposition {
        mode: CompositionMode::ChainedDocument,
        resolved_path: source.resolved_path.clone(),
        source_repo_root,
        prompt: composed.content().to_string(),
        effective_frontmatter,
        effective_agent_hint,
        closure: CompositionClosurePlan::Direct,
        lifecycle,
    })
```

- [ ] **Step 5: Update prepare_inline to parse lifecycle**

In `prepare_inline()`, after `let effective_agent_hint = ...` (line 105), add:

```rust
    let lifecycle = parse_lifecycle_config(&effective_frontmatter)?;
```

Then add `lifecycle` to the `PreparedComposition` construction:

```rust
    Ok(PreparedComposition {
        mode: CompositionMode::InlineFrontmatterPrompt,
        resolved_path: source.resolved_path.clone(),
        source_repo_root,
        prompt,
        effective_frontmatter,
        effective_agent_hint,
        closure: CompositionClosurePlan::Inline(InlineClosurePlan {
            original_document_text: source.original_text.clone(),
            original_body_hash,
        }),
        lifecycle,
    })
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p claudine --lib composition::prepare`
Expected: all tests pass (existing + 3 new)

- [ ] **Step 7: Commit**

```bash
git add claudine/lib/src/composition/types.rs claudine/lib/src/composition/prepare.rs
git commit -m "feat(claudine): parse lifecycle config into PreparedComposition

Lifecycle config is now parsed from effective frontmatter during both
prepare_direct and prepare_inline, ensuring compose and inline-compose
paths have consistent lifecycle notification behavior."
```

---

## Task 3: Add resolved-message helper to messaging

**Files:**
- Modify: `claudine/lib/src/messaging/send.rs`

- [ ] **Step 1: Write the failing test**

Add a `#[cfg(test)]` module at the end of `claudine/lib/src/messaging/send.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_resolved_message_with_no_route_is_noop() {
        // Empty messaging settings → no route → should not panic
        let messaging = RuntimeMessagingSettings {
            user: None,
            repo: None,
        };
        execute_resolved_message("Hello world", None, None, None, &messaging);
        // If we get here without panic, the no-op path works
    }
}
```

- [ ] **Step 2: Run the test to see it fail**

Run: `cargo test -p claudine --lib messaging::send`
Expected: FAIL — `execute_resolved_message` does not exist

- [ ] **Step 3: Implement execute_resolved_message**

Add this function to `claudine/lib/src/messaging/send.rs`, after the existing `execute_message` function:

```rust
/// Send a pre-rendered message without template interpolation.
///
/// Unlike [`execute_message`], this function accepts already-resolved text
/// and does not require an [`EventMeta`]. Designed for lifecycle notifications
/// where the message text is a fixed string from frontmatter.
///
/// Follows the same fire-and-forget pattern: spawns an async task and returns
/// immediately. Missing routes are a no-op.
pub fn execute_resolved_message(
    text: &str,
    image: Option<&str>,
    cwd: Option<&Path>,
    repo_root: Option<&Path>,
    messaging: &RuntimeMessagingSettings,
) {
    if text.trim().is_empty() && image.is_none() {
        return;
    }

    let Some(route) = resolve_effective_route(messaging) else {
        return;
    };

    let cwd_str = cwd.and_then(|p| p.to_str());
    let repo_str = repo_root.and_then(|p| p.to_str());

    let Some(payload) = build_payload(
        &route,
        text.to_string(),
        image.map(|s| s.to_string()),
        cwd_str,
        repo_str,
    ) else {
        return;
    };

    tokio::spawn(async move {
        if let Err(e) = send_payload(&route, payload).await {
            warn!(
                route = route.name,
                provider = provider_kind_label_from_config(&route.config),
                error = %e,
                "Failed to send lifecycle message"
            );
        }
    });
}
```

Also add this import at the top of the file if not already present:

```rust
use std::path::Path;
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p claudine --lib messaging::send`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add claudine/lib/src/messaging/send.rs
git commit -m "feat(claudine): add execute_resolved_message for lifecycle notifications

Provides a messaging send path that accepts pre-rendered text without
requiring EventMeta or template interpolation. Missing routes are a
silent no-op."
```

---

## Task 4: Lifecycle emitter with audio ordering

**Files:**
- Modify: `claudine/lib/src/composition/lifecycle.rs`

- [ ] **Step 1: Write the audio-order unit tests**

Add these tests to the existing `#[cfg(test)] mod tests` in `lifecycle.rs`:

```rust
    #[test]
    fn audio_order_speak_plus_effect() {
        let n = LifecycleNotification {
            speak: Some("Hello".into()),
            effect: Some("doorbell".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 2);
        assert!(matches!(phases[0], AudioPhase::Effect(_)));
        assert!(matches!(phases[1], AudioPhase::Speak(_)));
    }

    #[test]
    fn audio_order_speak_first_plus_effect() {
        let n = LifecycleNotification {
            speak_first: Some("Hello".into()),
            effect: Some("doorbell".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 2);
        assert!(matches!(phases[0], AudioPhase::Speak(_)));
        assert!(matches!(phases[1], AudioPhase::Effect(_)));
    }

    #[test]
    fn audio_order_speech_only() {
        let n = LifecycleNotification {
            speak: Some("Hello".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 1);
        assert!(matches!(phases[0], AudioPhase::Speak(_)));
    }

    #[test]
    fn audio_order_effect_only() {
        let n = LifecycleNotification {
            effect: Some("doorbell".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert_eq!(phases.len(), 1);
        assert!(matches!(phases[0], AudioPhase::Effect(_)));
    }

    #[test]
    fn audio_order_no_audio() {
        let n = LifecycleNotification {
            stderr: Some("Status only".into()),
            ..Default::default()
        };
        let phases = audio_phases(&n);
        assert!(phases.is_empty());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claudine --lib composition::lifecycle`
Expected: FAIL — `audio_phases` and `AudioPhase` do not exist

- [ ] **Step 3: Implement audio phase planning**

Add these types and function to `lifecycle.rs`, after the `normalize_notification` function and before the `impl LifecycleConfig` block:

```rust
/// A single audio playback phase.
#[derive(Debug)]
enum AudioPhase {
    Speak(String),
    Effect(String),
}

/// Compute the ordered audio phases for a notification.
///
/// When both speech and effect are present:
/// - `speak` + `effect` → effect first, then speech
/// - `speak_first` + `effect` → speech first, then effect
///
/// When only one audio output is present, it is the sole phase.
fn audio_phases(n: &LifecycleNotification) -> Vec<AudioPhase> {
    let speech_text = n
        .speak
        .as_deref()
        .or(n.speak_first.as_deref())
        .filter(|s| !s.is_empty());
    let effect_name = n.effect.as_deref().filter(|s| !s.is_empty());
    let speech_first = n.speak_first.is_some();

    match (speech_text, effect_name) {
        (Some(text), Some(effect)) if speech_first => {
            vec![
                AudioPhase::Speak(text.to_string()),
                AudioPhase::Effect(effect.to_string()),
            ]
        }
        (Some(text), Some(effect)) => {
            vec![
                AudioPhase::Effect(effect.to_string()),
                AudioPhase::Speak(text.to_string()),
            ]
        }
        (Some(text), None) => vec![AudioPhase::Speak(text.to_string())],
        (None, Some(effect)) => vec![AudioPhase::Effect(effect.to_string())],
        (None, None) => vec![],
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p claudine --lib composition::lifecycle`
Expected: all tests pass (10 parsing + 5 audio order)

- [ ] **Step 5: Implement the emit function**

Add this public function to `lifecycle.rs`:

```rust
/// Build a `TtsConfig` from global settings (mirrors dispatch/runner.rs logic).
fn tts_config_from_settings(tts: Option<&TtsSettings>) -> TtsConfig {
    let mut config = TtsConfig::new();
    let Some(settings) = tts else {
        return config;
    };

    if let Some(voice) = settings.voice.as_deref() {
        config = config.with_voice(voice);
    }
    if let Some(rate) = settings.rate {
        config = config.with_speed(SpeedLevel::Explicit(rate));
    }
    if let Some(provider) = settings.provider.as_deref() {
        if let Some(provider) = biscuit_speaks::parse_provider_name(provider) {
            config = config.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
        } else {
            warn!(
                provider,
                "Unknown TTS provider in settings; using automatic selection"
            );
        }
    }
    config
}

/// Play a sound effect synchronously (blocking).
fn play_effect_blocking(name: &str) {
    let Some(effect) = playa::SoundEffect::from_name(name) else {
        warn!(%name, "Unknown sound effect in lifecycle notification");
        return;
    };
    match playa::Playa::from_bytes(effect.bytes().to_vec()) {
        Ok(player) => {
            if let Err(e) = player.play() {
                warn!(%e, "Lifecycle sound effect playback failed");
            }
        }
        Err(e) => warn!(%e, "Failed to construct sound effect player"),
    }
}

/// Speak text synchronously using the Tokio runtime.
fn speak_blocking(text: &str, config: TtsConfig) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let text = text.to_string();
        // Block on the async speak to ensure ordering
        let _ = handle.block_on(async move {
            if let Err(e) = biscuit_speaks::Speak::new(text).with_config(config).play().await {
                warn!(%e, "Lifecycle TTS playback failed");
            }
        });
    } else {
        warn!("No Tokio runtime available for lifecycle TTS");
    }
}

/// Emit a lifecycle signal with deterministic audio ordering.
///
/// Dispatches non-audio targets (stderr, message) immediately, then
/// plays audio phases in order. All errors are logged as warnings and
/// never propagated.
pub fn emit_lifecycle_signal(
    config: &LifecycleConfig,
    signal: LifecycleSignal,
    ctx: &LifecycleRuntimeContext<'_>,
) {
    let Some(notification) = config.get(signal) else {
        return;
    };

    // --- Non-audio fan-out (immediate) ---

    // stderr
    if let Some(stderr_text) = &notification.stderr {
        let rendered = Status::from_prose(stderr_text)
            .state(signal.status_state())
            .theme(StatusTheme::Circular)
            .render(ctx.term);
        eprintln!("{rendered}");
    }

    // message
    if let Some(message_text) = &notification.message {
        crate::messaging::execute_resolved_message(
            message_text,
            None,
            Some(ctx.source_path),
            ctx.repo_root,
            ctx.messaging,
        );
    }

    // --- Audio phases (sequential, blocking) ---

    let phases = audio_phases(notification);
    let tts_config = tts_config_from_settings(ctx.settings.tts.as_ref());

    for phase in phases {
        match phase {
            AudioPhase::Speak(text) => speak_blocking(&text, tts_config.clone()),
            AudioPhase::Effect(name) => play_effect_blocking(&name),
        }
    }
}
```

- [ ] **Step 6: Run full check**

Run: `cargo check -p claudine`
Expected: compiles cleanly

- [ ] **Step 7: Run all lifecycle tests**

Run: `cargo test -p claudine --lib composition::lifecycle`
Expected: all 15 tests pass

- [ ] **Step 8: Commit**

```bash
git add claudine/lib/src/composition/lifecycle.rs
git commit -m "feat(claudine): implement lifecycle emitter with audio ordering

Adds emit_lifecycle_signal() with deterministic audio phase ordering:
effect-before-speech for 'speak', speech-before-effect for 'speak_first'.
Non-audio targets (stderr, message) dispatch immediately before audio.
All notification errors are best-effort warnings."
```

---

## Task 5: Wire lifecycle into execute_composition_request

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition.rs`

This task adds lifecycle state tracking and runtime context loading to the main composition executor, then wires `start`/`blocked`/`success`/`failure` signals into the non-harness paths.

- [ ] **Step 1: Add imports at the top of composition.rs**

Add these imports to the existing import block in `claudine/cli/src/commands/wrap/composition.rs`:

```rust
use claudine::composition::lifecycle::{
    LifecycleConfig, LifecycleRuntimeContext, LifecycleRuntimeState, LifecycleSignal,
    emit_lifecycle_signal,
};
use claudine::messaging::RuntimeMessagingSettings;
use claudine::events::GlobalSettings;
```

- [ ] **Step 2: Run check to confirm imports resolve**

Run: `cargo check -p claudine-cli`
Expected: compiles (imports unused but valid)

- [ ] **Step 3: Load runtime settings in execute_composition_request**

In `execute_composition_request()`, after the header emission section (around line 536, before the three-way dispatch at line 542), add:

```rust
    // --- Lifecycle setup ---
    let lifecycle = &request.prepared.lifecycle;
    let mut lifecycle_state = LifecycleRuntimeState::default();

    let (lifecycle_settings, lifecycle_messaging) = match claudine::dispatch::loader::load_runtime_config(None, effective_repo_root) {
        Ok(config) => (config.settings().clone(), config.messaging().clone()),
        Err(_) => (GlobalSettings::default(), RuntimeMessagingSettings { user: None, repo: None }),
    };

    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &lifecycle_settings,
        messaging: &lifecycle_messaging,
        term: &term,
        source_path: &request.prepared.resolved_path,
        repo_root: effective_repo_root,
    };
```

- [ ] **Step 4: Wire lifecycle into the non-harness inline writability failure**

Find the non-harness inline writability check (around line 482 — the `check_write_permission` call that returns `Err`). Wrap the error return to emit `blocked` first:

Replace the existing `.map_err(|reason| eyre!("{reason}"))?;` with:

```rust
    .map_err(|reason| {
        emit_lifecycle_signal(lifecycle, LifecycleSignal::Blocked, &lifecycle_ctx);
        eyre!("{reason}")
    })?;
```

- [ ] **Step 5: Wire lifecycle into execute_direct_without_harness call**

Change the `execute_direct_without_harness(...)` call (around line 619) to include lifecycle emission. Before the call, emit `start`:

```rust
    // Emit start before provider launch
    emit_lifecycle_signal(lifecycle, LifecycleSignal::Start, &lifecycle_ctx);
    lifecycle_state.start_emitted = true;
    lifecycle_state.provider_launch_started = true;

    let exit_code = execute_direct_without_harness(
        ...existing args...
    )?;

    // Emit terminal lifecycle signal
    if exit_code == 0 {
        emit_lifecycle_signal(lifecycle, LifecycleSignal::Success, &lifecycle_ctx);
    } else {
        emit_lifecycle_signal(lifecycle, LifecycleSignal::Failure, &lifecycle_ctx);
    }

    Ok(exit_code)
```

Note: The existing code returns the result of `execute_direct_without_harness(...)` directly. You need to capture it in a variable, emit the lifecycle signal, then return.

- [ ] **Step 6: Wire lifecycle into execute_inline_without_harness call**

Similarly for the `execute_inline_without_harness(...)` call (around line 596). Before the call, emit `start`:

```rust
    // Emit start before provider launch
    emit_lifecycle_signal(lifecycle, LifecycleSignal::Start, &lifecycle_ctx);
    lifecycle_state.start_emitted = true;
    lifecycle_state.provider_launch_started = true;

    let exit_code = execute_inline_without_harness(
        ...existing args...
    )?;

    // Emit terminal lifecycle signal
    if exit_code == 0 {
        emit_lifecycle_signal(lifecycle, LifecycleSignal::Success, &lifecycle_ctx);
    } else {
        emit_lifecycle_signal(lifecycle, LifecycleSignal::Failure, &lifecycle_ctx);
    }

    Ok(exit_code)
```

- [ ] **Step 7: Run check**

Run: `cargo check -p claudine-cli`
Expected: compiles cleanly

- [ ] **Step 8: Commit**

```bash
git add claudine/cli/src/commands/wrap/composition.rs
git commit -m "feat(claudine): wire lifecycle signals into non-harness composition paths

Emits start before provider launch, success/failure based on exit code,
and blocked for pre-launch writability failures in non-harness inline.
Runtime settings are loaded once and lifecycle context is shared across
all paths."
```

---

## Task 6: Wire lifecycle into run_harness_loop

**Files:**
- Modify: `claudine/cli/src/commands/wrap/mod.rs`

This is the most complex integration. The harness loop has 9 terminal exit points and 5 handler-recovery continue points. Lifecycle signals must be emitted at the right moments.

- [ ] **Step 1: Add lifecycle parameters to run_harness_loop signature**

In `claudine/cli/src/commands/wrap/mod.rs`, add three new parameters to `run_harness_loop()` at the end of the parameter list (before the closing parenthesis):

```rust
    lifecycle: &LifecycleConfig,
    lifecycle_state: &mut LifecycleRuntimeState,
    lifecycle_ctx: &LifecycleRuntimeContext<'_>,
```

Add the necessary imports at the top of the file:

```rust
use claudine::composition::lifecycle::{
    LifecycleConfig, LifecycleRuntimeContext, LifecycleRuntimeState, LifecycleSignal,
    emit_lifecycle_signal,
};
```

- [ ] **Step 2: Update the call site in composition.rs**

In the harness branch of `execute_composition_request()` (around line 566), update the `run_harness_loop(...)` call to pass the three new lifecycle arguments at the end:

```rust
    run_harness_loop(
        ...existing args...,
        lifecycle,
        &mut lifecycle_state,
        &lifecycle_ctx,
    )
```

- [ ] **Step 3: Run check to confirm signature change compiles**

Run: `cargo check -p claudine-cli`
Expected: compiles

- [ ] **Step 4: Emit `start` before first provider launch**

In `run_harness_loop()`, find the `build_harness_launch()` call (around line 2431). Immediately before it, add:

```rust
        // Emit start lifecycle signal before the first provider launch
        if !lifecycle_state.start_emitted {
            emit_lifecycle_signal(lifecycle, LifecycleSignal::Start, lifecycle_ctx);
            lifecycle_state.start_emitted = true;
        }
        lifecycle_state.provider_launch_started = true;
```

- [ ] **Step 5: Emit `blocked` at pre-launch terminal failures**

There are four pre-launch failure return points. At each one, emit `blocked` before returning.

**5a. Source file missing** (around line 2271-2274):

Before the `return Err(...)`, add:

```rust
            if !lifecycle_state.provider_launch_started {
                emit_lifecycle_signal(lifecycle, LifecycleSignal::Blocked, lifecycle_ctx);
            }
```

**5b. Shell audit failure — source page ::shell** (around line 2327-2330):

Before the `return Err(...)`, add the same guard:

```rust
            if !lifecycle_state.provider_launch_started {
                emit_lifecycle_signal(lifecycle, LifecycleSignal::Blocked, lifecycle_ctx);
            }
```

**5c. Shell audit failure — handler exhausted** (around line 2364):

Before the `return Err(...)`, add the same guard:

```rust
            if !lifecycle_state.provider_launch_started {
                emit_lifecycle_signal(lifecycle, LifecycleSignal::Blocked, lifecycle_ctx);
            }
```

**5d. Pre-check failure — handler exhausted** (around line 2426):

Before the `return Err(...)`, add the same guard:

```rust
            if !lifecycle_state.provider_launch_started {
                emit_lifecycle_signal(lifecycle, LifecycleSignal::Blocked, lifecycle_ctx);
            }
```

- [ ] **Step 6: Emit `failure` at post-launch terminal failures**

There are four post-launch failure return points.

**6a. Interrupted** (around line 2467):

Before `return Ok(outcome.exit_code)`, add:

```rust
            emit_lifecycle_signal(lifecycle, LifecycleSignal::Failure, lifecycle_ctx);
```

**6b. Agent failure — handler exhausted** (around line 2511):

Before the `return Err(...)`, add:

```rust
            emit_lifecycle_signal(lifecycle, LifecycleSignal::Failure, lifecycle_ctx);
```

**6c. Inline closure failure — handler exhausted** (around line 2565):

Before the `return Err(...)`, add:

```rust
            emit_lifecycle_signal(lifecycle, LifecycleSignal::Failure, lifecycle_ctx);
```

**6d. Post-check failure — handler exhausted** (around line 2627):

Before the `return Err(...)`, add:

```rust
            emit_lifecycle_signal(lifecycle, LifecycleSignal::Failure, lifecycle_ctx);
```

- [ ] **Step 7: Emit `success` at the success path**

At the success return point (around line 2587, where `post_report.all_passed()` is true), before `return Ok(outcome.exit_code)`, add:

```rust
            emit_lifecycle_signal(lifecycle, LifecycleSignal::Success, lifecycle_ctx);
```

- [ ] **Step 8: Run check**

Run: `cargo check -p claudine-cli`
Expected: compiles cleanly

- [ ] **Step 9: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs claudine/cli/src/commands/wrap/composition.rs
git commit -m "feat(claudine): wire lifecycle signals into harness loop

Emits start before first provider launch (once, even across retries),
blocked for terminal pre-launch failures, failure for post-launch
terminal errors, and success when post-checks pass. Handler recovery
that reaches launch suppresses blocked."
```

---

## Task 7: Handle harness-path blocked and success in composition.rs

**Files:**
- Modify: `claudine/cli/src/commands/wrap/composition.rs`

The harness path returns a `Result<i32>` from `run_harness_loop`. If the harness itself returned `Err`, the lifecycle signal was already emitted inside the loop. But if it returned `Ok(exit_code)`, `success` was already emitted inside the loop too. For the harness path, no additional lifecycle emission is needed in the outer `execute_composition_request` — the harness loop handles it internally.

However, there are harness-path failures that happen *before* `run_harness_loop` is called — specifically the harness plan parsing failure (around line 440).

- [ ] **Step 1: Wire blocked into harness plan parse failure**

Find the harness plan parsing failure (around line 440 — where `harness::parse_plan(...)` or similar returns an error before `run_harness_loop` is called). If it fails after `PreparedComposition` exists but before launch:

```rust
    // Before the return Err for harness plan parse failure:
    emit_lifecycle_signal(lifecycle, LifecycleSignal::Blocked, &lifecycle_ctx);
```

Note: Only add this if the failure point is reachable after the lifecycle context has been constructed (Step 3 of Task 5). If it occurs before, this is one of the early-error cases from spec section 4 where no lifecycle signal is emitted. Verify the line numbers against the actual code order before adding.

- [ ] **Step 2: Run check**

Run: `cargo check -p claudine-cli`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add claudine/cli/src/commands/wrap/composition.rs
git commit -m "feat(claudine): emit blocked for harness plan parse failures

Pre-launch harness plan parsing errors now emit the blocked lifecycle
signal before propagating the error."
```

---

## Task 8: Export execute_resolved_message from messaging module

**Files:**
- Modify: `claudine/lib/src/messaging/mod.rs` (or whichever file re-exports messaging publics)

- [ ] **Step 1: Find the messaging module's public exports**

Check `claudine/lib/src/messaging/mod.rs` for existing exports.

- [ ] **Step 2: Add the re-export**

Add `execute_resolved_message` to the public exports:

```rust
pub use send::execute_resolved_message;
```

- [ ] **Step 3: Run check**

Run: `cargo check -p claudine`
Expected: compiles

- [ ] **Step 4: Commit**

```bash
git add claudine/lib/src/messaging/mod.rs
git commit -m "feat(claudine): export execute_resolved_message from messaging module"
```

---

## Task 9: Integration tests for stderr lifecycle output

**Files:**
- Create: `claudine/lib/src/composition/lifecycle_integration_tests.rs` (or add to existing test infrastructure)

These tests validate the observable behavior of lifecycle signals — specifically the stderr output which is the most testable aspect.

- [ ] **Step 1: Add stderr capture tests to lifecycle.rs**

Add these integration-style tests to the existing `#[cfg(test)]` module in `lifecycle.rs`. These test the `emit_lifecycle_signal` function's stderr output by testing the Status rendering directly (since we can't easily capture stderr in unit tests, we test the components):

```rust
    #[test]
    fn status_state_mapping() {
        assert_eq!(LifecycleSignal::Start.status_state(), StatusState::Info);
        assert_eq!(LifecycleSignal::Success.status_state(), StatusState::Success);
        assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Failure);
        assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Failure);
    }

    #[test]
    fn property_names() {
        assert_eq!(LifecycleSignal::Start.property_name(), "start");
        assert_eq!(LifecycleSignal::Success.property_name(), "success");
        assert_eq!(LifecycleSignal::Blocked.property_name(), "blocked");
        assert_eq!(LifecycleSignal::Failure.property_name(), "failure");
    }

    #[test]
    fn lifecycle_config_get() {
        let fm = json!({
            "start": { "stderr": "Starting" },
            "failure": { "stderr": "Failed" }
        });
        let config = parse_lifecycle_config(&fm).unwrap();
        assert!(config.get(LifecycleSignal::Start).is_some());
        assert!(config.get(LifecycleSignal::Success).is_none());
        assert!(config.get(LifecycleSignal::Blocked).is_none());
        assert!(config.get(LifecycleSignal::Failure).is_some());
    }

    #[test]
    fn lifecycle_config_is_empty() {
        let empty = LifecycleConfig::default();
        assert!(empty.is_empty());

        let fm = json!({ "start": { "stderr": "Go" } });
        let non_empty = parse_lifecycle_config(&fm).unwrap();
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn lifecycle_runtime_state_defaults() {
        let state = LifecycleRuntimeState::default();
        assert!(!state.start_emitted);
        assert!(!state.provider_launch_started);
    }
```

- [ ] **Step 2: Run all lifecycle tests**

Run: `cargo test -p claudine --lib composition::lifecycle`
Expected: all 20 tests pass

- [ ] **Step 3: Commit**

```bash
git add claudine/lib/src/composition/lifecycle.rs
git commit -m "test(claudine): add lifecycle signal mapping and config accessor tests

Tests status state mapping, property names, config.get(), is_empty(),
and runtime state defaults."
```

---

## Task 10: Full build verification and cleanup

**Files:**
- All modified files

- [ ] **Step 1: Run full library tests**

Run: `cargo test -p claudine`
Expected: all tests pass

- [ ] **Step 2: Run CLI build check**

Run: `cargo check -p claudine-cli`
Expected: compiles cleanly

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p claudine -p claudine-cli -- -D warnings`
Expected: no warnings

- [ ] **Step 4: Run rustfmt**

Run: `cargo fmt --check`
Expected: no formatting issues (or run `cargo fmt` to fix)

- [ ] **Step 5: Fix any issues found in steps 1-4**

Address compiler errors, clippy warnings, or formatting issues.

- [ ] **Step 6: Run the full test suite**

Run: `just test` from the `claudine/` directory
Expected: all tests pass

- [ ] **Step 7: Commit any cleanup**

```bash
git add -u
git commit -m "chore(claudine): fix lint and formatting for lifecycle feature"
```

---

## Summary of changes by file

| File | Action | Purpose |
|------|--------|---------|
| `claudine/lib/src/composition/lifecycle.rs` | CREATE | Types, parsing, validation, audio ordering, emitter |
| `claudine/lib/src/composition/mod.rs` | MODIFY | Add `pub mod lifecycle` + re-exports |
| `claudine/lib/src/composition/types.rs` | MODIFY | Add `lifecycle: LifecycleConfig` to `PreparedComposition` |
| `claudine/lib/src/composition/error.rs` | MODIFY | Add `LifecycleSpeakConflict` and `LifecycleUnknownEffect` variants |
| `claudine/lib/src/composition/prepare.rs` | MODIFY | Parse lifecycle config in `prepare_direct` and `prepare_inline` |
| `claudine/lib/src/messaging/send.rs` | MODIFY | Add `execute_resolved_message()` |
| `claudine/lib/src/messaging/mod.rs` | MODIFY | Re-export `execute_resolved_message` |
| `claudine/cli/src/commands/wrap/composition.rs` | MODIFY | Load runtime config, create lifecycle context, emit signals in non-harness paths + pre-harness blocked |
| `claudine/cli/src/commands/wrap/mod.rs` | MODIFY | Accept lifecycle params in `run_harness_loop`, emit signals at 9 trigger points |
