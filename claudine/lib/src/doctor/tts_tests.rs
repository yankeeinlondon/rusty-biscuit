//! The `tts` row from a real user configuration file, read through the
//! doctor's passive observation, against an injected candidate slice in
//! runtime order.

use biscuit_speaks::{CloudTtsProvider, HostTtsProvider, TtsProvider};

use super::super::report::row;
use super::super::{ContextUnavailable, DoctorRow, Environment, Severity, config, run_with};
use super::assess;

const KOKORO: TtsProvider = TtsProvider::Host(HostTtsProvider::KokoroTts);
const PIPER: TtsProvider = TtsProvider::Host(HostTtsProvider::Piper);
const ESPEAK: TtsProvider = TtsProvider::Host(HostTtsProvider::ESpeak);
const MIMIC: TtsProvider = TtsProvider::Host(HostTtsProvider::Mimic3);
const ELEVENLABS: TtsProvider = TtsProvider::Cloud(CloudTtsProvider::ElevenLabs);

/// An unrecognized provider name that must never reach the report.
const SENTINEL: &str = "zz-sentinel-engine";

/// The `tts` row for a user `config.yaml` holding `text`.
fn tts_row(text: &str, available: &[TtsProvider]) -> DoctorRow {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claudine")).unwrap();
    std::fs::write(dir.path().join(".claudine/config.yaml"), text).unwrap();
    let environment = Environment::for_test(Some(dir.path().to_path_buf()), Err(ContextUnavailable));
    let observations = config::observe(&environment);
    row("tts", assess(&observations.user_config, available), None)
}

#[track_caller]
fn assert_tts(row: &DoctorRow, severity: Severity, needles: &[&str]) {
    assert_eq!(row.severity, severity, "{row:#?}");
    let text = format!("{}\n{}", row.summary, row.detail);
    for needle in needles {
        assert!(text.contains(needle), "`{needle}` not in {row:#?}");
    }
    assert!(text.contains("provider detection checks local prerequisites only"), "{row:#?}");
    assert!(!row.fixable);
}

#[test]
fn automatic_selection_reports_the_highest_rated_and_the_runtime_first_candidate() {
    let row = tts_row("tts: true\n", &[ESPEAK, KOKORO, PIPER]);
    assert_tts(&row, Severity::Ok, &["hook speech uses Kokoro (excellent quality)", "automatic selection tries eSpeak NG (low quality) first"]);
    assert_eq!(row.summary, "hook speech uses Kokoro (excellent quality)");
}

#[test]
fn equal_ratings_select_the_candidate_runtime_tries_first() {
    let row = tts_row("tts: true\n", &[KOKORO, ELEVENLABS]);
    assert_eq!(row.summary, "hook speech uses Kokoro (excellent quality)");
    assert!(!row.detail.contains("tries"), "{row:#?}");

    let row = tts_row("tts: true\n", &[ELEVENLABS, KOKORO]);
    assert_eq!(row.summary, "hook speech uses ElevenLabs (excellent quality)");
    assert!(!row.detail.contains("tries"), "{row:#?}");
}

#[test]
fn every_enabled_automatic_shape_without_a_candidate_is_an_error() {
    for text in ["tts: true\n", "tts: {}\n", "tts:\n  provider:\n", "tts:\n  provider: null\n  prewarm: false\n"] {
        let row = tts_row(text, &[]);
        assert_tts(&row, Severity::Error, &["hook speech is on but no speech provider is available", "Remedy: install a local provider"]);
    }
}

#[test]
fn quality_below_good_warns_and_good_or_better_passes_when_enabled() {
    assert_tts(&tts_row("tts: true\n", &[ESPEAK]), Severity::Warn, &["the best available speech provider, eSpeak NG, has low quality"]);
    assert_tts(&tts_row("tts: true\n", &[MIMIC, ESPEAK]), Severity::Warn, &["Mimic 3, has moderate quality"]);
    assert_tts(&tts_row("tts: true\n", &[ESPEAK, PIPER]), Severity::Ok, &["hook speech uses Piper (good quality)"]);
}

#[test]
fn a_recognized_provider_is_assessed_alone() {
    let row = tts_row("tts:\n  provider: espeak\n", &[KOKORO, ESPEAK]);
    assert_tts(&row, Severity::Warn, &["the configured speech provider, eSpeak NG, has low quality", "Remedy: set `tts.provider` to a higher-quality"]);
    assert!(!row.detail.contains("Kokoro"), "a pinned provider never names another candidate: {row:#?}");

    let row = tts_row("tts:\n  provider: ElevenLabs\n  voice: Rachel\n", &[PIPER, ELEVENLABS]);
    assert_tts(&row, Severity::Ok, &["hook speech uses ElevenLabs (excellent quality)"]);
}

#[test]
fn a_missing_pinned_provider_is_an_error_despite_an_available_fallback() {
    let row = tts_row("tts:\n  provider: piper\n", &[ESPEAK, KOKORO]);
    assert_tts(
        &row,
        Severity::Error,
        &["the configured speech provider Piper is not available", "does not fall back", "Remedy: install the configured provider"],
    );
}

#[test]
fn an_unrecognized_provider_warns_at_least_and_uses_automatic_candidates() {
    let text = format!("tts:\n  provider: {SENTINEL}\n");
    let row = tts_row(&text, &[KOKORO]);
    assert_tts(&row, Severity::Warn, &["the configured speech provider is not recognized", "selects a provider automatically", "hook speech uses Kokoro"]);
    assert_eq!(row.summary, "the configured speech provider is not recognized");

    let row = tts_row(&text, &[]);
    assert_tts(&row, Severity::Error, &["hook speech is on but no speech provider is available", "not recognized"]);

    for row in [tts_row(&text, &[KOKORO]), tts_row(&text, &[])] {
        assert!(!format!("{row:?}").contains(SENTINEL), "the raw name stays out of the report: {row:#?}");
    }
}

#[test]
fn disabled_hook_speech_is_informational_and_names_lifecycle_say() {
    for text in ["logging: true\n", "tts:\n", "tts: null\n", "tts: false\n"] {
        let none = tts_row(text, &[]);
        assert_tts(&none, Severity::Info, &["hook speech is off and no speech provider is available", "lifecycle `say` can still speak"]);

        let low = tts_row(text, &[ESPEAK]);
        assert_tts(&low, Severity::Info, &["eSpeak NG, has low quality", "lifecycle `say` can still speak"]);

        let good = tts_row(text, &[ESPEAK, PIPER]);
        assert_tts(&good, Severity::Ok, &["lifecycle `say` can still speak"]);
        assert_eq!(good.summary, "hook speech is off; Piper is available to lifecycle `say` (good quality)", "{text:?}");
    }
}

#[test]
fn an_unusable_user_configuration_leaves_the_settings_undetermined() {
    for text in ["tts: 3\n", "tts:\n  provider: \"\"\n", "tts:\n  provider: 3\n", "tts: [true]\n", "nonsense: true\n"] {
        let row = tts_row(text, &[KOKORO]);
        assert_eq!(row.severity, Severity::Info, "{text:?}: {row:#?}");
        assert_eq!(row.summary, "speech settings could not be determined", "{text:?}");
        assert!(!row.detail.contains("Kokoro"));
    }
    let dir = tempfile::tempdir().unwrap();
    let observations = config::observe(&Environment::for_test(Some(dir.path().to_path_buf()), Err(ContextUnavailable)));
    let missing = row("tts", assess(&observations.user_config, &[KOKORO]), None);
    assert_eq!(missing.summary, "speech settings could not be determined");
}

/// The registry callback reads this host's detection and a real pass; it
/// produces one row whatever is installed.
#[test]
fn the_registered_check_reports_one_row_from_host_detection() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".claudine")).unwrap();
    std::fs::write(dir.path().join(".claudine/config.yaml"), "tts: false\n").unwrap();
    let environment = Environment::for_test(Some(dir.path().to_path_buf()), Err(ContextUnavailable));
    let registry = super::super::REGISTRY.iter().filter(|check| check.id == "tts");
    let rows: Vec<DoctorRow> = registry.flat_map(|check| run_with(std::slice::from_ref(check), &environment, false)).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "tts");
    assert!(matches!(rows[0].severity, Severity::Ok | Severity::Info), "disabled speech never warns: {:#?}", rows[0]);
}
