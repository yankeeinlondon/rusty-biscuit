use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Logging destination for `HookAction::Log`.
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
        name: String,

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
    },

    /// Write the event to a configured log target.
    Log {
        /// Destination target.
        #[serde(default = "default_log_target")]
        target: LogTarget,
    },

    /// Execute a command asynchronously without waiting for a result.
    FireAndForget {
        /// Command name or path to executable.
        command: String,

        /// Optional arguments.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
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

fn default_log_target() -> LogTarget {
    LogTarget::File {
        path: None,
        rotate_daily: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_effect_deserializes_defaults() {
        let json = serde_json::json!({
            "type": "sound_effect",
            "name": "success"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::SoundEffect {
            name,
            volume,
            speed,
        } = action
        else {
            panic!("expected sound_effect");
        };

        assert_eq!(name, "success");
        assert_eq!(volume, 1.0);
        assert_eq!(speed, 1.0);
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
    fn log_defaults_to_file_target() {
        let json = serde_json::json!({
            "type": "log"
        });

        let action: HookAction = serde_json::from_value(json).unwrap();
        let HookAction::Log { target } = action else {
            panic!("expected log");
        };

        assert_eq!(
            target,
            LogTarget::File {
                path: None,
                rotate_daily: true
            }
        );
    }
}
