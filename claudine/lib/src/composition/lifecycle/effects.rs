//! Claudine's [`LifecycleEffects`] service: Darkmatter's text-only
//! communication calls resolved against Claudine's settings and delivered
//! through a [`LifecycleEmitter`].

use std::path::Path;
use std::sync::OnceLock;

use biscuit_speaks::TtsConfig;
use biscuit_terminal::terminal::Terminal;
use darkmatter::lifecycle::{LifecycleEffects, LifecycleSignal};

use super::{LifecycleEmitter, tts_config_from_settings};
use crate::events::GlobalSettings;
use crate::messaging::RuntimeMessagingSettings;

/// Adapts a [`LifecycleEmitter`] to Darkmatter's [`LifecycleEffects`].
///
/// The emitter keeps receiving the configuration it always has — the TTS
/// configuration resolved from `settings`, the messaging route, the source
/// provenance, and the terminal — so the adapter is the one place that
/// configuration is resolved for lifecycle communication. The TTS
/// configuration is resolved on the first speech and reused for every later
/// one through the same adapter.
pub struct ClaudineLifecycleEffects<'a> {
    /// Where the communication goes.
    pub emitter: &'a dyn LifecycleEmitter,
    /// Global settings (TTS configuration).
    pub settings: &'a GlobalSettings,
    /// Messaging routes.
    pub messaging: &'a RuntimeMessagingSettings,
    /// Terminal the text channels render for.
    pub term: &'a Terminal,
    /// Composition source path (messenger provenance).
    pub source_path: &'a Path,
    /// Repository root, if any.
    pub repo_root: Option<&'a Path>,
    tts_config: OnceLock<TtsConfig>,
}

impl<'a> ClaudineLifecycleEffects<'a> {
    /// Deliver through `emitter` with the given configuration.
    pub fn new(
        emitter: &'a dyn LifecycleEmitter,
        settings: &'a GlobalSettings,
        messaging: &'a RuntimeMessagingSettings,
        term: &'a Terminal,
        source_path: &'a Path,
        repo_root: Option<&'a Path>,
    ) -> Self {
        Self {
            emitter,
            settings,
            messaging,
            term,
            source_path,
            repo_root,
            tts_config: OnceLock::new(),
        }
    }
}

impl LifecycleEffects for ClaudineLifecycleEffects<'_> {
    fn stdout(&self, text: &str) {
        self.emitter.emit_stdout(text, self.term);
    }

    fn stderr(&self, signal: LifecycleSignal, text: &str) {
        self.emitter.emit_stderr(signal, text, self.term);
    }

    fn info(&self, text: &str) {
        self.emitter.emit_info(text, self.term);
    }

    fn warn(&self, text: &str) {
        self.emitter.emit_warn(text, self.term);
    }

    fn success(&self, text: &str) {
        self.emitter.emit_success(text, self.term);
    }

    fn message(&self, text: &str) {
        self.emitter
            .emit_message(text, self.source_path, self.repo_root, self.messaging);
    }

    fn speech(&self, text: &str) {
        let config = self
            .tts_config
            .get_or_init(|| tts_config_from_settings(self.settings.tts.as_ref()))
            .clone();
        self.emitter.emit_speech(text, config);
    }

    fn effect(&self, name: &str) {
        self.emitter.emit_effect(name);
    }

    fn notification(&self, title: &str) {
        self.emitter.emit_notification(title);
    }
}
