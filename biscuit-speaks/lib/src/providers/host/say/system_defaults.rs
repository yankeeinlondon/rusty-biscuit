//! The voice and rate `say` uses when it is invoked without `-v` or `-r`.
//!
//! Resolution goes through [`SystemSpeechDefaults`] so tests can pin the
//! result on any OS; only the production lookup touches macOS.

#![allow(dead_code, reason = "scaffolding: no caller resolves Say system defaults yet")]

use std::future::Future;

/// Effective system speech settings.
///
/// Each field is `None` when it could not be determined reliably, which makes
/// that setting unresolved: preparation does nothing and playback neither reads
/// nor publishes a reusable cache entry for it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SystemSpeech {
    /// A voice name `say -v` accepts.
    pub voice: Option<String>,
    /// Words per minute, as `say -r` takes it.
    pub rate_wpm: Option<u32>,
}

/// Source of the system's effective speech settings.
///
/// One lookup returns both settings so they describe the same moment.
pub(crate) trait SystemSpeechDefaults: Send + Sync {
    fn lookup(&self) -> impl Future<Output = SystemSpeech> + Send;
}

/// The production lookup.
///
/// Reports both settings as unresolved until a verified OS lookup replaces it,
/// so system defaults keep today's behavior of never being cached.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OsSpeechDefaults;

impl SystemSpeechDefaults for OsSpeechDefaults {
    async fn lookup(&self) -> SystemSpeech {
        SystemSpeech::default()
    }
}

/// A lookup that returns whatever the test last set, and counts calls.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct FakeSpeechDefaults {
    settings: std::sync::Mutex<SystemSpeech>,
    lookups: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl FakeSpeechDefaults {
    pub(crate) fn new(voice: Option<&str>, rate_wpm: Option<u32>) -> Self {
        let fake = Self::default();
        fake.set(voice, rate_wpm);
        fake
    }

    pub(crate) fn set(&self, voice: Option<&str>, rate_wpm: Option<u32>) {
        *self.settings.lock().unwrap() = SystemSpeech {
            voice: voice.map(str::to_owned),
            rate_wpm,
        };
    }

    pub(crate) fn lookups(&self) -> usize {
        self.lookups.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
impl SystemSpeechDefaults for FakeSpeechDefaults {
    async fn lookup(&self) -> SystemSpeech {
        self.lookups.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.settings.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn production_lookup_leaves_both_settings_unresolved() {
        assert_eq!(OsSpeechDefaults.lookup().await, SystemSpeech::default());
    }

    #[tokio::test]
    async fn fake_lookup_reports_the_latest_pinned_settings() {
        let fake = FakeSpeechDefaults::new(Some("Ava (Premium)"), Some(175));
        assert_eq!(
            fake.lookup().await,
            SystemSpeech { voice: Some("Ava (Premium)".into()), rate_wpm: Some(175) }
        );
        fake.set(Some("Samantha"), None);
        assert_eq!(
            fake.lookup().await,
            SystemSpeech { voice: Some("Samantha".into()), rate_wpm: None }
        );
        assert_eq!(fake.lookups(), 2);
    }
}
