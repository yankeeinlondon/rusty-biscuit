//! The voice and rate `say` uses when it is invoked without `-v` or `-r`.
//!
//! Resolution goes through [`SystemSpeechDefaults`] so tests can pin the
//! result on any OS; only the production lookup touches macOS.

use std::future::Future;

use serde::Deserialize;
use tracing::debug;

use crate::traits::TtsVoiceInventory;

/// Source of the system's effective speech settings.
///
/// `None` means the setting could not be determined reliably, which makes it
/// unresolved: preparation does nothing and playback neither reads nor
/// publishes a reusable cache entry for it. The two settings are separate
/// lookups so a caller that already knows one never pays for the other.
pub(crate) trait SystemSpeechDefaults: Send + Sync {
    /// A voice name `say -v` accepts.
    fn voice(&self) -> impl Future<Output = Option<String>> + Send;
    /// Words per minute, as `say -r` takes it.
    fn rate_wpm(&self) -> impl Future<Output = Option<u32>> + Send;
}

/// The production lookup, through AppKit's documented `NSSpeechSynthesizer`.
///
/// Verified on macOS 27.2 by byte comparison: `say -o` with no flags wrote
/// the same WAV as `-v <reported voice> -r <reported rate>`, while a different
/// voice or rate changed the bytes.
///
/// The rate is reported as unresolved even though the lookup prints one. On
/// the verifying host it equaled the library's built-in 175 WPM, so it is not
/// yet shown to follow the user's Spoken Content rate; trusting it could play
/// a prepared file at a stale speed after the user moves the slider.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OsSpeechDefaults;

/// JXA that prints the default voice's name and a fresh synthesizer's rate.
const LOOKUP_SCRIPT: &str = r#"ObjC.import("AppKit");
var v = $.NSSpeechSynthesizer.defaultVoice;
var s = $.NSSpeechSynthesizer.alloc.initWithVoice(v);
JSON.stringify({voice: ObjC.unwrap($.NSSpeechSynthesizer.attributesForVoice(v).objectForKey("VoiceName")), rate: s.rate})"#;

impl SystemSpeechDefaults for OsSpeechDefaults {
    async fn voice(&self) -> Option<String> {
        let output = tokio::process::Command::new("osascript")
            .args(["-l", "JavaScript", "-e", LOOKUP_SCRIPT])
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true)
            .output()
            .await;
        let output = match output {
            Ok(output) if output.status.success() => output,
            other => {
                debug!(?other, "system voice lookup failed; the default voice is unresolved");
                return None;
            }
        };
        let voice = parse_lookup(&String::from_utf8_lossy(&output.stdout))?;
        // A name `say` does not list would not reproduce the default voice.
        let listed = super::SayProvider.list_voices().await.ok()?;
        listed.iter().any(|listed| listed.name == voice).then_some(voice)
    }

    async fn rate_wpm(&self) -> Option<u32> {
        None
    }
}

/// The lookup's one line of output, exactly.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LookupOutput {
    voice: String,
    rate: f64,
}

/// The voice from the lookup's output, when the whole output has the expected
/// shape: one JSON object holding a non-empty `voice` string and a positive,
/// whole-number `rate`. Anything else is unrecognized and so unresolved.
fn parse_lookup(stdout: &str) -> Option<String> {
    let stdout = stdout.trim();
    // A derived struct would also accept the fields as a JSON array.
    if !stdout.starts_with('{') {
        debug!("unrecognized system voice lookup output");
        return None;
    }
    let parsed: LookupOutput = match serde_json::from_str(stdout) {
        Ok(parsed) => parsed,
        Err(error) => {
            debug!(%error, "unrecognized system voice lookup output");
            return None;
        }
    };
    let whole_positive_rate = parsed.rate >= 1.0 && parsed.rate.fract() == 0.0;
    (!parsed.voice.trim().is_empty() && whole_positive_rate).then_some(parsed.voice)
}

/// A lookup that returns whatever the test last set, and counts calls.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct FakeSpeechDefaults {
    settings: std::sync::Mutex<(Option<String>, Option<u32>)>,
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
        *self.settings.lock().unwrap() = (voice.map(str::to_owned), rate_wpm);
    }

    /// Lookups of either setting.
    pub(crate) fn lookups(&self) -> usize {
        self.lookups.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
impl SystemSpeechDefaults for FakeSpeechDefaults {
    async fn voice(&self) -> Option<String> {
        self.lookups.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.settings.lock().unwrap().0.clone()
    }

    async fn rate_wpm(&self) -> Option<u32> {
        self.lookups.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.settings.lock().unwrap().1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn production_rate_is_unresolved_without_running_anything() {
        assert_eq!(OsSpeechDefaults.rate_wpm().await, None);
    }

    #[tokio::test]
    async fn fake_lookup_reports_the_latest_pinned_settings() {
        let fake = FakeSpeechDefaults::new(Some("Ava (Premium)"), Some(175));
        assert_eq!(fake.voice().await.as_deref(), Some("Ava (Premium)"));
        assert_eq!(fake.rate_wpm().await, Some(175));
        fake.set(Some("Samantha"), None);
        assert_eq!(fake.voice().await.as_deref(), Some("Samantha"));
        assert_eq!(fake.rate_wpm().await, None);
        assert_eq!(fake.lookups(), 4);
    }

    /// Input robustness for the lookup output: the control row is what the
    /// lookup printed on the verifying host; every edited row is unresolved.
    #[test]
    fn only_the_exact_lookup_shape_resolves_a_voice() {
        let control = r#"{"voice":"Ava (Premium)","rate":175}"#;
        assert_eq!(parse_lookup(control).as_deref(), Some("Ava (Premium)"));
        assert_eq!(parse_lookup(&format!("{control}\n")).as_deref(), Some("Ava (Premium)"));

        let rows = [
            ("voice absent", r#"{"rate":175}"#),
            ("voice null", r#"{"voice":null,"rate":175}"#),
            ("voice wrong type", r#"{"voice":7,"rate":175}"#),
            ("voice empty", r#"{"voice":"","rate":175}"#),
            ("voice blank", r#"{"voice":"  ","rate":175}"#),
            ("voice duplicate", r#"{"voice":"Ava (Premium)","voice":"Alex","rate":175}"#),
            ("rate absent", r#"{"voice":"Ava (Premium)"}"#),
            ("rate null", r#"{"voice":"Ava (Premium)","rate":null}"#),
            ("rate wrong type", r#"{"voice":"Ava (Premium)","rate":"175"}"#),
            ("rate zero", r#"{"voice":"Ava (Premium)","rate":0}"#),
            ("rate negative", r#"{"voice":"Ava (Premium)","rate":-175}"#),
            ("rate fractional", r#"{"voice":"Ava (Premium)","rate":175.5}"#),
            ("rate duplicate", r#"{"voice":"Ava (Premium)","rate":175,"rate":200}"#),
            ("unknown field", r#"{"voice":"Ava (Premium)","rate":175,"pitch":1}"#),
            ("empty object", "{}"),
            ("not an object", r#"["Ava (Premium)",175]"#),
            ("trailing content", r#"{"voice":"Ava (Premium)","rate":175} extra"#),
            ("two objects", "{\"voice\":\"Ava (Premium)\",\"rate\":175}\n{\"voice\":\"Alex\",\"rate\":175}"),
            ("empty output", ""),
            ("an osascript error", "execution error: Error: ReferenceError (-2700)"),
        ];
        for (row, output) in rows {
            assert_eq!(parse_lookup(output), None, "{row}: {output}");
        }
    }

    /// The real lookup on a real Mac resolves a voice `say` lists.
    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn real_system_voice_lookup_names_a_listed_voice() {
        let voice = OsSpeechDefaults.voice().await.expect("a Mac has a default voice");
        let listed = super::super::SayProvider.list_voices().await.unwrap();
        assert!(listed.iter().any(|listed| listed.name == voice), "{voice}");
    }
}
