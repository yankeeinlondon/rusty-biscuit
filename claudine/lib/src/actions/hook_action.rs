use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::claudine_config::Gender;

/// Logging destination for event log output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum LogTarget {
    /// Append JSONL records to a local file.
    File {
        /// Optional explicit path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<PathBuf>,

        /// Whether default file paths rotate by local day boundary.
        #[serde(default = "default_true")]
        rotate_daily: bool,
    },

    /// POST structured hook records to an HTTP endpoint.
    Server {
        /// Endpoint URL.
        url: String,

        /// Request timeout in milliseconds.
        #[serde(default = "default_log_timeout_ms")]
        timeout_ms: u64,

        /// Optional additional HTTP headers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
}

/// Format options for report output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReportFormat {
    /// Human-readable plain text.
    Text,
    /// Structured JSON object.
    Json,
    /// Compact single-line format: `[EVENT] message`.
    Compact,
}

/// Controls how a `Report` action formats its output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportHandler {
    /// Output format for the report line.
    pub format: ReportFormat,

    /// Optional template string for custom formatting.
    pub template: Option<String>,

    /// When true, include the full event metadata in the report.
    #[serde(default)]
    pub include_metadata: bool,
}

/// Transforms raw command output into a structured `HookResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Mapper {
    /// Parse stdout as JSON and extract a specific field as the decision.
    JsonField {
        /// Dot-separated path into the JSON object.
        field: String,
    },

    /// Parse stdout as JSON and use the entire object as the response.
    JsonObject,

    /// Interpret the exit code as the decision.
    ///
    /// `0` maps to [`HookDecision::Allow`], `2` maps to
    /// [`HookDecision::Deny`], and any other status (including crashes,
    /// `command-not-found` at `127`, or user-defined anomalous codes)
    /// emits **no** decision. The dispatcher logs a `warn!` on
    /// unexpected statuses and falls through to the next action rather
    /// than silently allowing a broken handler.
    ExitCode,

    /// Map stdout lines to specific response fields using named regex groups.
    Regex {
        /// Regex pattern with named capture groups.
        pattern: String,
    },
}

/// Runtime mapper form used by the dispatcher.
#[derive(Debug, Clone)]
pub enum CompiledMapper {
    /// Parse stdout as JSON and extract a specific field.
    JsonField { field: String },
    /// Parse stdout as a full JSON object.
    JsonObject,
    /// Interpret process exit code as decision.
    ExitCode,
    /// Regex mapper with precompiled pattern.
    Regex { pattern: regex::Regex },
}

/// An action to execute when a unified hook fires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum HookAction {
    /// Play an embedded sound effect from playa.
    SoundEffect {
        /// Effect name.
        effect: String,

        /// Playback volume (0.0 to 1.0).
        #[serde(default = "default_volume")]
        volume: f32,

        /// Playback speed multiplier.
        #[serde(default = "default_speed")]
        speed: f32,

        /// Optional Darkmatter boolean condition controlling whether this
        /// action runs. Evaluated against the active [`EventMeta`] before
        /// the action executes; a falsy or invalid expression skips the
        /// action.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<String>,
    },

    /// Speak a message aloud using biscuit-speaks TTS.
    Speak {
        /// Handlebars-style template message.
        message: String,

        /// Optional voice override for this action.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        voice: Option<String>,

        /// Optional gender preference for voice selection.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        gender: Option<Gender>,

        /// Optional Darkmatter boolean condition gating execution.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<String>,
    },

    /// Execute a shell command asynchronously without waiting for a result.
    Bash {
        /// Shell command string.
        command: String,

        /// Template-interpolated parameters appended to the command.
        #[serde(default)]
        params: String,

        /// Optional Darkmatter boolean condition gating execution.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<String>,
    },

    /// Execute a command synchronously and map its output to a hook response.
    Call {
        /// Command name or path to executable.
        command: String,

        /// Optional arguments.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,

        /// Optional timeout in milliseconds.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,

        /// Optional response mapper.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mapper: Option<Mapper>,

        /// Optional Darkmatter boolean condition gating execution. A
        /// skipped `Call` action cannot replace a previously selected
        /// blocking [`HookResponse`].
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<String>,
    },

    /// Report the event into the agent's output stream.
    Report {
        /// Report output handler.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        handler: Option<ReportHandler>,

        /// Optional Darkmatter boolean condition gating execution.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<String>,
    },

    /// Send a message to the configured messaging destination.
    Message {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image: Option<String>,

        /// Optional Darkmatter boolean condition gating execution.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<String>,
    },
}

impl HookAction {
    /// Returns the canonical snake_case action type used in config serialization.
    pub const fn type_slug(&self) -> &'static str {
        match self {
            HookAction::SoundEffect { .. } => "sound_effect",
            HookAction::Speak { .. } => "speak",
            HookAction::Bash { .. } => "bash",
            HookAction::Call { .. } => "call",
            HookAction::Report { .. } => "report",
            HookAction::Message { .. } => "message",
        }
    }

    /// Returns a PascalCase action type label for terminal presentation.
    pub const fn type_pascal_case(&self) -> &'static str {
        match self {
            HookAction::SoundEffect { .. } => "SoundEffect",
            HookAction::Speak { .. } => "Speak",
            HookAction::Bash { .. } => "Bash",
            HookAction::Call { .. } => "Call",
            HookAction::Report { .. } => "Report",
            HookAction::Message { .. } => "Message",
        }
    }

    /// Returns the optional Darkmatter boolean condition gating this
    /// action's execution.
    ///
    /// When `Some`, the runner evaluates the expression against the
    /// active [`EventMeta`] before running the action. A falsy result or
    /// a parse/evaluation error causes the action to be skipped without
    /// short-circuiting the rest of the binding.
    pub fn when(&self) -> Option<&str> {
        match self {
            HookAction::SoundEffect { when, .. }
            | HookAction::Speak { when, .. }
            | HookAction::Bash { when, .. }
            | HookAction::Call { when, .. }
            | HookAction::Report { when, .. }
            | HookAction::Message { when, .. } => when.as_deref(),
        }
    }
}

fn default_volume() -> f32 {
    1.0
}

fn default_speed() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_log_timeout_ms() -> u64 {
    10_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_effect_deserializes_defaults() {
        let json = serde_json::json!({
            "type": "sound_effect",
            "effect": "success"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::SoundEffect {
            effect,
            volume,
            speed,
            when,
        } = action
        else {
            panic!("expected sound_effect");
        };

        assert_eq!(effect, "success");
        assert_eq!(volume, 1.0);
        assert_eq!(speed, 1.0);
        assert!(when.is_none());
    }

    #[test]
    fn sound_effect_with_effect_field() {
        let json = serde_json::json!({
            "type": "sound_effect",
            "effect": "ding",
            "volume": 0.5,
            "speed": 1.5
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::SoundEffect {
            effect,
            volume,
            speed,
            when,
        } = action
        else {
            panic!("expected sound_effect");
        };

        assert_eq!(effect, "ding");
        assert_eq!(volume, 0.5);
        assert_eq!(speed, 1.5);
        assert!(when.is_none());
    }

    #[test]
    fn speak_with_voice_and_gender() {
        let json = serde_json::json!({
            "type": "speak",
            "message": "Hello world",
            "voice": "Samantha",
            "gender": "female"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::Speak {
            message,
            voice,
            gender,
            when,
        } = action
        else {
            panic!("expected speak");
        };

        assert_eq!(message, "Hello world");
        assert_eq!(voice.as_deref(), Some("Samantha"));
        assert_eq!(gender, Some(Gender::Female));
        assert!(when.is_none());
    }

    #[test]
    fn speak_minimal() {
        let json = serde_json::json!({
            "type": "speak",
            "message": "Hello"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::Speak {
            message,
            voice,
            gender,
            when,
        } = action
        else {
            panic!("expected speak");
        };

        assert_eq!(message, "Hello");
        assert!(voice.is_none());
        assert!(gender.is_none());
        assert!(when.is_none());
    }

    #[test]
    fn speak_skips_serializing_none_fields() {
        let action = HookAction::Speak {
            message: "test".to_string(),
            voice: None,
            gender: None,
            when: None,
        };

        let json = serde_json::to_value(&action).unwrap();
        assert!(json.get("voice").is_none());
        assert!(json.get("gender").is_none());
        assert!(json.get("when").is_none());
    }

    #[test]
    fn bash_deserializes() {
        let json = serde_json::json!({
            "type": "bash",
            "command": "notify-send",
            "params": "{{tool_name}}"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::Bash {
            command,
            params,
            when,
        } = action
        else {
            panic!("expected bash");
        };

        assert_eq!(command, "notify-send");
        assert_eq!(params, "{{tool_name}}");
        assert!(when.is_none());
    }

    #[test]
    fn bash_default_params() {
        let json = serde_json::json!({
            "type": "bash",
            "command": "echo hello"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::Bash {
            command,
            params,
            when,
        } = action
        else {
            panic!("expected bash");
        };

        assert_eq!(command, "echo hello");
        assert_eq!(params, "");
        assert!(when.is_none());
    }

    #[test]
    fn bash_type_labels() {
        let action = HookAction::Bash {
            command: "echo".to_string(),
            params: String::new(),
            when: None,
        };

        assert_eq!(action.type_slug(), "bash");
        assert_eq!(action.type_pascal_case(), "Bash");
    }

    #[test]
    fn call_round_trip() {
        let action = HookAction::Call {
            command: "security-check".to_string(),
            args: Some(vec!["--quick".to_string()]),
            timeout_ms: Some(4_000),
            mapper: Some(Mapper::JsonField {
                field: "decision".to_string(),
            }),
            when: None,
        };

        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "call");

        let back: HookAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn message_deserializes_with_required_fields() {
        let json = serde_json::json!({
            "type": "message",
            "message": "Deploy complete"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::Message {
            message,
            image,
            when,
        } = action
        else {
            panic!("expected message");
        };

        assert_eq!(message, "Deploy complete");
        assert!(image.is_none());
        assert!(when.is_none());
    }

    #[test]
    fn message_deserializes_with_image() {
        let json = serde_json::json!({
            "type": "message",
            "message": "Screenshot attached",
            "image": "/tmp/screenshot.png"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::Message {
            message,
            image,
            when,
        } = action
        else {
            panic!("expected message");
        };

        assert_eq!(message, "Screenshot attached");
        assert_eq!(image.as_deref(), Some("/tmp/screenshot.png"));
        assert!(when.is_none());
    }

    #[test]
    fn message_round_trip() {
        let action = HookAction::Message {
            message: "**build** done".to_string(),
            image: Some("~/artifacts/build.png".to_string()),
            when: None,
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
            when: None,
        };

        assert_eq!(action.type_slug(), "message");
        assert_eq!(action.type_pascal_case(), "Message");
    }

    // =========================================================================
    // `when` round-trip and accessor tests (Phase 3 of leverage-dm-parser)
    // =========================================================================

    #[test]
    fn old_action_configs_without_when_still_deserialize() {
        let json = serde_json::json!({
            "type": "bash",
            "command": "echo hi"
        });
        let action: HookAction = serde_json::from_value(json).unwrap();
        assert!(action.when().is_none());
    }

    #[test]
    fn sound_effect_round_trips_with_when() {
        let action = HookAction::SoundEffect {
            effect: "ding".to_string(),
            volume: 1.0,
            speed: 1.0,
            when: Some("tool_name == 'Bash'".to_string()),
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["when"], "tool_name == 'Bash'");
        let back: HookAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
        assert_eq!(back.when(), Some("tool_name == 'Bash'"));
    }

    #[test]
    fn speak_round_trips_with_when() {
        let action = HookAction::Speak {
            message: "ready".to_string(),
            voice: None,
            gender: None,
            when: Some("provider == 'claude'".to_string()),
        };
        let json = serde_json::to_value(&action).unwrap();
        let back: HookAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
        assert_eq!(back.when(), Some("provider == 'claude'"));
    }

    #[test]
    fn bash_round_trips_with_when() {
        let action = HookAction::Bash {
            command: "echo".to_string(),
            params: String::new(),
            when: Some("git.is_dirty".to_string()),
        };
        let json = serde_json::to_value(&action).unwrap();
        let back: HookAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn call_round_trips_with_when() {
        let action = HookAction::Call {
            command: "security-check".to_string(),
            args: None,
            timeout_ms: None,
            mapper: None,
            when: Some("tool_name == 'Bash'".to_string()),
        };
        let json = serde_json::to_value(&action).unwrap();
        let back: HookAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn report_round_trips_with_when() {
        let action = HookAction::Report {
            handler: None,
            when: Some("event == 'tool_error'".to_string()),
        };
        let json = serde_json::to_value(&action).unwrap();
        let back: HookAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn message_round_trips_with_when() {
        let action = HookAction::Message {
            message: "alert".to_string(),
            image: None,
            when: Some("error".to_string()),
        };
        let json = serde_json::to_value(&action).unwrap();
        let back: HookAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn when_field_omitted_from_serialization_when_none() {
        let action = HookAction::Bash {
            command: "echo".to_string(),
            params: String::new(),
            when: None,
        };
        let json = serde_json::to_value(&action).unwrap();
        assert!(json.get("when").is_none());
    }
}
