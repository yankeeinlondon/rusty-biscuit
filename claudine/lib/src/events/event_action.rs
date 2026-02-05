use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

/// An action to execute when an `AgenticEvent` fires.
///
/// Actions are executed in declaration order. Multiple actions can be
/// attached to a single event binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventAction {
    /// Speak the message aloud using biscuit-speaks TTS.
    ///
    /// The message string supports `{placeholder}` interpolation
    /// from the event's metadata fields.
    Speak {
        /// Template message to speak (supports `{placeholder}` interpolation).
        message: String,
    },

    /// Log the event to a remote server or local file.
    Log {
        /// Where to send the log entry.
        target: LogTarget,
    },

    /// Report the event into the agent's output stream.
    ///
    /// When `handler` is `None`, a default plain-text summary is
    /// emitted. When provided, the `ReportHandler` controls
    /// formatting and filtering.
    Report {
        /// Report output handler. `None` uses default plain-text summary.
        handler: Option<ReportHandler>,
    },

    /// Run a command on the host system.
    ///
    /// The command must exist on the system PATH. Arguments are optional.
    /// By default, execution is fire-and-forget (non-blocking). Set
    /// `blocking` to `true` to wait for the command to complete.
    Run {
        /// Command name or path to executable.
        command: String,
        /// Optional arguments to pass to the command.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        /// When true, wait for the command to complete before proceeding.
        /// Defaults to false (fire-and-forget).
        #[serde(default)]
        blocking: bool,
    },

    /// Play an embedded sound effect from the playa library.
    ///
    /// Uses playa's 53 built-in effects across 6 categories.
    /// Playback is non-blocking.
    SoundEffect {
        /// Effect name (e.g., "success", "error", "sad-trombone").
        /// Must match one of playa's built-in effect names.
        name: String,

        /// Playback volume as a float from 0.0 (silent) to 1.0 (full).
        /// Defaults to 1.0 when omitted.
        #[serde(default = "default_volume")]
        volume: f32,

        /// Playback speed multiplier. 1.0 = normal, 1.5 = faster,
        /// 0.75 = slower. Defaults to 1.0 when omitted.
        #[serde(default = "default_speed")]
        speed: f32,
    },
}

/// Where to send log output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LogTarget {
    /// POST the event JSON to a remote endpoint.
    Server {
        /// Server URL.
        url: Url,
    },

    /// Append the event as a JSONL line to a local file.
    LocalFile {
        /// File path (supports `~` expansion).
        path: PathBuf,
    },
}

/// Controls how a `Report` action formats its output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReportHandler {
    /// Output format for the report line.
    pub format: ReportFormat,

    /// Optional template string for custom formatting.
    /// Supports `{placeholder}` interpolation from event metadata.
    pub template: Option<String>,

    /// When true, include the full event metadata in the report.
    /// When false (default), only include the summary.
    #[serde(default)]
    pub include_metadata: bool,
}

/// Format options for report output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    /// Human-readable plain text.
    Text,
    /// Structured JSON object.
    Json,
    /// Compact single-line format: `[EVENT] message`.
    Compact,
}

fn default_volume() -> f32 {
    1.0
}

fn default_speed() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_effect_with_defaults() {
        let action = EventAction::SoundEffect {
            name: "success".to_string(),
            volume: default_volume(),
            speed: default_speed(),
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "sound_effect");
        assert_eq!(json["name"], "success");
        assert_eq!(json["volume"], 1.0);
        assert_eq!(json["speed"], 1.0);
    }

    #[test]
    fn sound_effect_deserializes_with_defaults() {
        let json = serde_json::json!({
            "type": "sound_effect",
            "name": "power-up"
        });
        let action: EventAction = serde_json::from_value(json).unwrap();
        match action {
            EventAction::SoundEffect {
                name,
                volume,
                speed,
            } => {
                assert_eq!(name, "power-up");
                assert_eq!(volume, 1.0);
                assert_eq!(speed, 1.0);
            }
            _ => panic!("Expected SoundEffect"),
        }
    }

    #[test]
    fn speak_round_trip() {
        let action = EventAction::Speak {
            message: "Hello {tool_name}".to_string(),
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "speak");
        assert_eq!(json["message"], "Hello {tool_name}");
        let back: EventAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn log_local_file_round_trip() {
        let action = EventAction::Log {
            target: LogTarget::LocalFile {
                path: PathBuf::from("~/.claudine/events.jsonl"),
            },
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "log");
        assert_eq!(json["target"]["type"], "local_file");
        let back: EventAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn log_server_round_trip() {
        let action = EventAction::Log {
            target: LogTarget::Server {
                url: Url::parse("https://example.com/events").unwrap(),
            },
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["target"]["type"], "server");
        assert_eq!(json["target"]["url"], "https://example.com/events");
        let back: EventAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn report_with_handler() {
        let action = EventAction::Report {
            handler: Some(ReportHandler {
                format: ReportFormat::Compact,
                template: Some("[TOOL] {tool_name}: executing".to_string()),
                include_metadata: false,
            }),
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "report");
        assert_eq!(json["handler"]["format"], "compact");
        let back: EventAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn report_without_handler() {
        let json = serde_json::json!({
            "type": "report",
            "handler": null
        });
        let action: EventAction = serde_json::from_value(json).unwrap();
        match action {
            EventAction::Report { handler } => assert!(handler.is_none()),
            _ => panic!("Expected Report"),
        }
    }

    #[test]
    fn report_format_variants() {
        for (variant, expected) in [
            (ReportFormat::Text, "text"),
            (ReportFormat::Json, "json"),
            (ReportFormat::Compact, "compact"),
        ] {
            let json = serde_json::to_value(&variant).unwrap();
            assert_eq!(json.as_str().unwrap(), expected);
        }
    }

    #[test]
    fn run_round_trip() {
        let action = EventAction::Run {
            command: "notify-send".to_string(),
            args: Some(vec!["--urgency=low".to_string(), "done".to_string()]),
            blocking: false,
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "run");
        assert_eq!(json["command"], "notify-send");
        assert_eq!(json["args"], serde_json::json!(["--urgency=low", "done"]));
        let back: EventAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn run_deserializes_with_defaults() {
        let json = serde_json::json!({
            "type": "run",
            "command": "echo"
        });
        let action: EventAction = serde_json::from_value(json).unwrap();
        match action {
            EventAction::Run {
                command,
                args,
                blocking,
            } => {
                assert_eq!(command, "echo");
                assert!(args.is_none());
                assert!(!blocking);
            }
            _ => panic!("Expected Run"),
        }
    }

    #[test]
    fn run_blocking_round_trip() {
        let action = EventAction::Run {
            command: "sync".to_string(),
            args: None,
            blocking: true,
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "run");
        assert_eq!(json["blocking"], true);
        assert!(json.get("args").is_none());
        let back: EventAction = serde_json::from_value(json).unwrap();
        assert_eq!(back, action);
    }
}
