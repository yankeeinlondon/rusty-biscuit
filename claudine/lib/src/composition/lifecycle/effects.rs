//! Claudine's [`LifecycleEffects`] service: Darkmatter's text-only
//! communication calls resolved against Claudine's settings and delivered
//! through a [`LifecycleEmitter`].
//!
//! A `test-fixtures` build also carries an in-process hold on one `info`
//! text, so terminal tests can keep Claudine itself busy inside a lifecycle
//! event (see `test_hold`). A default build has no such hold.

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
        #[cfg(feature = "test-fixtures")]
        test_hold::hold_if_requested(text);
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

/// In-process lifecycle work that a keypress cannot end, for the terminal
/// Ctrl+C tests of the host's repeat-interrupt ladder.
///
/// A lifecycle `shell` action cannot stand in for it: the command runs in its
/// own process group and holds the terminal's foreground, so a typed Ctrl+C
/// reaches the command rather than Claudine. This hold keeps Claudine in the
/// event's `info` action, with the terminal still Claudine's.
///
/// Active only for the exact text [`HOLD_TEXT`] with [`HOLD_ENV`] set to a
/// path prefix: after the text is emitted it writes `<prefix>.reached`, then
/// waits for `<prefix>.release` for at most [`HOLD_BOUND`], or until the
/// directory holding the marker files is removed (a test that dropped its
/// workspace).
#[cfg(feature = "test-fixtures")]
mod test_hold {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    const HOLD_TEXT: &str = "terminal-lifecycle-test-hold";
    const HOLD_ENV: &str = "CLAUDINE_TEST_LIFECYCLE_HOLD";
    /// Ends a hold whose test never released it, so a failed test cannot
    /// leave Claudine parked indefinitely.
    const HOLD_BOUND: Duration = Duration::from_secs(60);
    const POLL: Duration = Duration::from_millis(10);

    pub(super) fn hold_if_requested(text: &str) {
        if text != HOLD_TEXT {
            return;
        }
        let Ok(prefix) = std::env::var(HOLD_ENV) else {
            return;
        };
        let reached = PathBuf::from(format!("{prefix}.reached"));
        let release = PathBuf::from(format!("{prefix}.release"));
        if std::fs::write(&reached, b"").is_err() {
            return;
        }
        let dir = match reached.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => PathBuf::from("."),
        };
        let deadline = Instant::now() + HOLD_BOUND;
        while !release.exists() && dir.is_dir() && Instant::now() < deadline {
            std::thread::sleep(POLL);
        }
    }
}
