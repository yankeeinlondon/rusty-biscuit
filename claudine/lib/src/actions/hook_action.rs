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
    },

    /// Execute a shell command asynchronously without waiting for a result.
    Bash {
        /// Shell command string.
        command: String,

        /// Template-interpolated parameters appended to the command.
        #[serde(default)]
        params: String,
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
    },

    /// Report the event into the agent's output stream.
    Report {
        /// Report output handler.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        handler: Option<ReportHandler>,
    },

    /// Send a message to the configured messaging destination.
    Message {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image: Option<String>,
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
        } = action
        else {
            panic!("expected sound_effect");
        };

        assert_eq!(effect, "success");
        assert_eq!(volume, 1.0);
        assert_eq!(speed, 1.0);
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
        } = action
        else {
            panic!("expected sound_effect");
        };

        assert_eq!(effect, "ding");
        assert_eq!(volume, 0.5);
        assert_eq!(speed, 1.5);
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
        } = action
        else {
            panic!("expected speak");
        };

        assert_eq!(message, "Hello world");
        assert_eq!(voice.as_deref(), Some("Samantha"));
        assert_eq!(gender, Some(Gender::Female));
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
        } = action
        else {
            panic!("expected speak");
        };

        assert_eq!(message, "Hello");
        assert!(voice.is_none());
        assert!(gender.is_none());
    }

    #[test]
    fn speak_skips_serializing_none_fields() {
        let action = HookAction::Speak {
            message: "test".to_string(),
            voice: None,
            gender: None,
        };

        let json = serde_json::to_value(&action).unwrap();
        assert!(json.get("voice").is_none());
        assert!(json.get("gender").is_none());
    }

    #[test]
    fn bash_deserializes() {
        let json = serde_json::json!({
            "type": "bash",
            "command": "notify-send",
            "params": "{{tool_name}}"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::Bash { command, params } = action else {
            panic!("expected bash");
        };

        assert_eq!(command, "notify-send");
        assert_eq!(params, "{{tool_name}}");
    }

    #[test]
    fn bash_default_params() {
        let json = serde_json::json!({
            "type": "bash",
            "command": "echo hello"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::Bash { command, params } = action else {
            panic!("expected bash");
        };

        assert_eq!(command, "echo hello");
        assert_eq!(params, "");
    }

    #[test]
    fn bash_type_labels() {
        let action = HookAction::Bash {
            command: "echo".to_string(),
            params: String::new(),
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
}
