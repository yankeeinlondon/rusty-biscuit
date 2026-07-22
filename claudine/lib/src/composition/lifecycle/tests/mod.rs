//! Tests for lifecycle configuration parsing and validation.

use super::*;
use serde_json::json;
use std::sync::Mutex;

fn dummy_path() -> &'static Path {
    Path::new("test.md")
}

#[derive(Debug, Clone, PartialEq)]
enum EmittedAction {
    Stderr {
        signal: LifecycleSignal,
        text: String,
    },
    Message {
        text: String,
    },
    Notification {
        title: String,
    },
    Speech {
        text: String,
    },
    Effect {
        name: String,
    },
}

struct RecordingEmitter {
    actions: Mutex<Vec<EmittedAction>>,
}

impl RecordingEmitter {
    fn new() -> Self {
        Self {
            actions: Mutex::new(Vec::new()),
        }
    }

    fn actions(&self) -> Vec<EmittedAction> {
        self.actions.lock().unwrap().clone()
    }

    fn signals(&self) -> Vec<LifecycleSignal> {
        self.actions
            .lock()
            .unwrap()
            .iter()
            .filter_map(|a| match a {
                EmittedAction::Stderr { signal, .. } => Some(*signal),
                _ => None,
            })
            .collect()
    }
}

impl LifecycleEmitter for RecordingEmitter {
    fn emit_stderr(&self, signal: LifecycleSignal, text: &str, _term: &Terminal) {
        self.actions.lock().unwrap().push(EmittedAction::Stderr {
            signal,
            text: text.to_string(),
        });
    }

    fn emit_message(
        &self,
        text: &str,
        _source_path: &Path,
        _repo_root: Option<&Path>,
        _messaging: &RuntimeMessagingSettings,
    ) {
        self.actions.lock().unwrap().push(EmittedAction::Message {
            text: text.to_string(),
        });
    }

    fn emit_speech(&self, text: &str, _tts_config: TtsConfig) {
        self.actions.lock().unwrap().push(EmittedAction::Speech {
            text: text.to_string(),
        });
    }

    fn emit_effect(&self, name: &str) {
        self.actions.lock().unwrap().push(EmittedAction::Effect {
            name: name.to_string(),
        });
    }

    fn emit_notification(&self, title: &str) {
        self.actions
            .lock()
            .unwrap()
            .push(EmittedAction::Notification {
                title: title.to_string(),
            });
    }
}

fn test_config() -> LifecycleConfig {
    parse_lifecycle_config(
        &json!({
            "start":   { "stderr": "starting" },
            "success": { "stderr": "done" },
            "blocked": { "stderr": "blocked" },
            "failure": { "stderr": "failed" },
        }),
        dummy_path(),
    )
    .unwrap()
}

fn test_ctx() -> (GlobalSettings, RuntimeMessagingSettings, Terminal) {
    (
        GlobalSettings::default(),
        RuntimeMessagingSettings {
            user: None,
            repo: None,
        },
        Terminal::default(),
    )
}

fn make_guard<'a>(
    config: &'a LifecycleConfig,
    ctx: &'a LifecycleRuntimeContext<'a>,
    emitter: &'a RecordingEmitter,
) -> LifecycleRunGuard<'a> {
    LifecycleRunGuard::new(config, ctx, emitter)
}

fn fm_from_json(value: serde_json::Value) -> darkmatter::markdown::Frontmatter {
    let mut fm = darkmatter::markdown::Frontmatter::new();
    if let serde_json::Value::Object(map) = value {
        for (key, val) in map {
            fm.insert(&key, val).unwrap();
        }
    }
    fm
}


mod action_shape_control;
mod audio_emission;
mod diagnostics;
mod guard_runtime;
mod parse_config;
mod validation;
