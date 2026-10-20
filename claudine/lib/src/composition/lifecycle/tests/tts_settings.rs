//! Lifecycle `say` playback configuration for each `tts` shape: `prewarm`
//! and a missing `provider` never change what lifecycle speech plays with.

use crate::composition::lifecycle::audio::tts_config_from_settings;
use crate::config::claudine_config::ClaudineConfig;
use crate::dispatch::loader::bridge_tts_settings;
use biscuit_speaks::TtsConfig;

fn lifecycle_config_for(tts: Option<serde_json::Value>) -> (TtsConfig, bool) {
    let mut document = serde_json::json!({ "preferred_agent": "claude" });
    if let Some(tts) = tts {
        document["tts"] = tts;
    }
    let config: ClaudineConfig = serde_json::from_value(document).unwrap();
    let settings = bridge_tts_settings(&config);
    (
        tts_config_from_settings(settings.tts.as_ref()),
        settings.tts_prewarm,
    )
}

#[test]
fn every_boolean_shape_plays_auto_detect_with_prewarm_on() {
    let auto_detect = TtsConfig::new();
    for tts in [None, Some(serde_json::json!(true)), Some(serde_json::json!(false))] {
        assert_eq!(
            lifecycle_config_for(tts.clone()),
            (auto_detect.clone(), true),
            "{tts:?}"
        );
    }
}

#[test]
fn provider_less_object_plays_as_true_and_applies_its_prewarm() {
    let (as_true, _) = lifecycle_config_for(Some(serde_json::json!(true)));
    assert_eq!(
        lifecycle_config_for(Some(serde_json::json!({ "prewarm": false }))),
        (as_true.clone(), false)
    );
    assert_eq!(
        lifecycle_config_for(Some(serde_json::json!({}))),
        (as_true, true)
    );
}

#[test]
fn prewarm_false_object_plays_as_the_same_object_without_it() {
    let (without, prewarm_without) = lifecycle_config_for(Some(
        serde_json::json!({ "provider": "kokoro", "voice": "af_heart" }),
    ));
    let (with_off, prewarm_with_off) = lifecycle_config_for(Some(
        serde_json::json!({ "provider": "kokoro", "voice": "af_heart", "prewarm": false }),
    ));
    assert_eq!(with_off, without);
    assert_eq!(with_off.requested_voice.as_deref(), Some("af_heart"));
    assert!(prewarm_without);
    assert!(!prewarm_with_off);
}

/// Lifecycle runs fall back to `GlobalSettings::default()` when
/// `config.json` cannot be loaded; pre-warm follows the auto-detected
/// playback configuration used there.
#[test]
fn default_settings_keep_prewarm_on() {
    let settings = crate::events::GlobalSettings::default();
    assert!(settings.tts_prewarm);
    assert!(settings.tts.is_none());
}
