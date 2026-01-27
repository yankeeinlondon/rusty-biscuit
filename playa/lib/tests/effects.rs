//! Integration tests for the sound effects module.
//!
//! These tests verify that all embedded sound effects can be loaded
//! and their formats detected correctly.
//!
//! Run with: `cargo test -p playa --features sound-effects`

#![cfg(feature = "sound-effects")]

use std::collections::HashSet;

use playa::{detect_audio_format_from_bytes, AudioFileFormat, SoundEffect};

// ============================================================================
// Test Helpers
// ============================================================================

/// Verify an effect loads and returns the expected format.
fn verify_effect_format(effect: SoundEffect, expected_format: AudioFileFormat) {
    let bytes = effect.bytes();
    assert!(
        !bytes.is_empty(),
        "Effect {:?} should have non-empty bytes",
        effect
    );

    let format = detect_audio_format_from_bytes(bytes)
        .unwrap_or_else(|e| panic!("Format detection should succeed for {:?}: {}", effect, e));

    assert_eq!(
        format.file_format, expected_format,
        "Effect {:?} expected format {:?}, got {:?}",
        effect, expected_format, format.file_format
    );
}

// ============================================================================
// UI Effects (sfx-ui) - 11 effects, all WAV
// ============================================================================

#[test]
fn ui_effects_are_wav_format() {
    use SoundEffect::*;
    let effects = [
        Doorbell,
        Doorbell2,
        SpaceAlarm,
        DitHit1,
        DitHit2,
        ElectronicHitFx01,
        ElectronicHitFx03,
        ElectronicHitFx06,
        ElectronicHitFx07,
        ElectronicHitFx14,
        ElectronicHitFx16,
    ];

    for effect in effects {
        verify_effect_format(effect, AudioFileFormat::Wav);
    }
}

#[test]
fn ui_effects_count() {
    use SoundEffect::*;
    let effects: [SoundEffect; 11] = [
        Doorbell,
        Doorbell2,
        SpaceAlarm,
        DitHit1,
        DitHit2,
        ElectronicHitFx01,
        ElectronicHitFx03,
        ElectronicHitFx06,
        ElectronicHitFx07,
        ElectronicHitFx14,
        ElectronicHitFx16,
    ];

    let set: HashSet<_> = effects.iter().collect();
    assert_eq!(set.len(), 11, "All UI effects should be distinct");
}

// ============================================================================
// Cartoon Effects (sfx-cartoon) - 13 effects, all WAV
// ============================================================================

#[test]
fn cartoon_effects_are_wav_format() {
    use SoundEffect::*;
    let effects = [
        CartoonAccent01,
        CartoonAccent02,
        CartoonAccent04,
        CartoonAccent07,
        CartoonAccent09,
        CartoonAccent10,
        CartoonAccent12,
        CartoonAccent17,
        CartoonAccent25,
        CartoonAccent31,
        CartoonAccent33,
        CartoonAccent35,
        CartoonCry,
    ];

    for effect in effects {
        verify_effect_format(effect, AudioFileFormat::Wav);
    }
}

#[test]
fn cartoon_effects_count() {
    use SoundEffect::*;
    let effects: [SoundEffect; 13] = [
        CartoonAccent01,
        CartoonAccent02,
        CartoonAccent04,
        CartoonAccent07,
        CartoonAccent09,
        CartoonAccent10,
        CartoonAccent12,
        CartoonAccent17,
        CartoonAccent25,
        CartoonAccent31,
        CartoonAccent33,
        CartoonAccent35,
        CartoonCry,
    ];

    let set: HashSet<_> = effects.iter().collect();
    assert_eq!(set.len(), 13, "All cartoon effects should be distinct");
}

// ============================================================================
// Reaction Effects (sfx-reactions) - 6 effects, all WAV
// ============================================================================

#[test]
fn reaction_effects_are_wav_format() {
    use SoundEffect::*;
    let effects = [
        CrowdLaugh,
        CrowdLaughApplause,
        SadTrombone,
        SmallGroupCheer,
        FemaleAstonishedGasp,
        Sneeze,
    ];

    for effect in effects {
        verify_effect_format(effect, AudioFileFormat::Wav);
    }
}

#[test]
fn reaction_effects_count() {
    use SoundEffect::*;
    let effects: [SoundEffect; 6] = [
        CrowdLaugh,
        CrowdLaughApplause,
        SadTrombone,
        SmallGroupCheer,
        FemaleAstonishedGasp,
        Sneeze,
    ];

    let set: HashSet<_> = effects.iter().collect();
    assert_eq!(set.len(), 6, "All reaction effects should be distinct");
}

// ============================================================================
// Sci-Fi Effects (sfx-scifi) - 11 effects, all OGG
// ============================================================================

#[test]
fn scifi_effects_are_ogg_format() {
    use SoundEffect::*;
    let effects = [
        HighDown,
        HighUp,
        TwoTone,
        PhaseJump1,
        PhaseJump2,
        PhaseJump3,
        PhaseJump4,
        PhaseJump5,
        PhaserDown1,
        PhaserDown2,
        PhaserDown3,
    ];

    for effect in effects {
        verify_effect_format(effect, AudioFileFormat::Ogg);
    }
}

#[test]
fn scifi_effects_count() {
    use SoundEffect::*;
    let effects: [SoundEffect; 11] = [
        HighDown,
        HighUp,
        TwoTone,
        PhaseJump1,
        PhaseJump2,
        PhaseJump3,
        PhaseJump4,
        PhaseJump5,
        PhaserDown1,
        PhaserDown2,
        PhaserDown3,
    ];

    let set: HashSet<_> = effects.iter().collect();
    assert_eq!(set.len(), 11, "All sci-fi effects should be distinct");
}

// ============================================================================
// Atmosphere Effects (sfx-atmosphere) - 5 effects, mixed formats
// ============================================================================

#[test]
fn atmosphere_effects_wav_format() {
    use SoundEffect::*;
    // WAV files in atmosphere category
    let wav_effects = [EpicOrchestraTransition, MysteriousBass, RetroGame];

    for effect in wav_effects {
        verify_effect_format(effect, AudioFileFormat::Wav);
    }
}

#[test]
fn atmosphere_effects_mp3_format() {
    use SoundEffect::*;
    // MP3 files in atmosphere category
    let mp3_effects = [CreepyDarkLogo, ElementalMagicSpellImpact];

    for effect in mp3_effects {
        verify_effect_format(effect, AudioFileFormat::Mp3);
    }
}

#[test]
fn atmosphere_effects_count() {
    use SoundEffect::*;
    let effects: [SoundEffect; 5] = [
        CreepyDarkLogo,
        ElementalMagicSpellImpact,
        EpicOrchestraTransition,
        MysteriousBass,
        RetroGame,
    ];

    let set: HashSet<_> = effects.iter().collect();
    assert_eq!(set.len(), 5, "All atmosphere effects should be distinct");
}

// ============================================================================
// Motion Effects (sfx-motion) - 7 effects, all WAV
// ============================================================================

#[test]
fn motion_effects_are_wav_format() {
    use SoundEffect::*;
    let effects = [
        AirReverseBurst,
        AirWoosh,
        AirZoomVacuum,
        ArrowWhoosh,
        BicycleHorn,
        BottleCork,
        Bullet,
    ];

    for effect in effects {
        verify_effect_format(effect, AudioFileFormat::Wav);
    }
}

#[test]
fn motion_effects_count() {
    use SoundEffect::*;
    let effects: [SoundEffect; 7] = [
        AirReverseBurst,
        AirWoosh,
        AirZoomVacuum,
        ArrowWhoosh,
        BicycleHorn,
        BottleCork,
        Bullet,
    ];

    let set: HashSet<_> = effects.iter().collect();
    assert_eq!(set.len(), 7, "All motion effects should be distinct");
}

// ============================================================================
// Aggregate Tests
// ============================================================================

#[test]
fn all_effects_have_valid_bytes() {
    use SoundEffect::*;

    let all_effects: Vec<SoundEffect> = vec![
        // UI (11)
        Doorbell,
        Doorbell2,
        SpaceAlarm,
        DitHit1,
        DitHit2,
        ElectronicHitFx01,
        ElectronicHitFx03,
        ElectronicHitFx06,
        ElectronicHitFx07,
        ElectronicHitFx14,
        ElectronicHitFx16,
        // Cartoon (13)
        CartoonAccent01,
        CartoonAccent02,
        CartoonAccent04,
        CartoonAccent07,
        CartoonAccent09,
        CartoonAccent10,
        CartoonAccent12,
        CartoonAccent17,
        CartoonAccent25,
        CartoonAccent31,
        CartoonAccent33,
        CartoonAccent35,
        CartoonCry,
        // Reactions (6)
        CrowdLaugh,
        CrowdLaughApplause,
        SadTrombone,
        SmallGroupCheer,
        FemaleAstonishedGasp,
        Sneeze,
        // Sci-Fi (11)
        HighDown,
        HighUp,
        TwoTone,
        PhaseJump1,
        PhaseJump2,
        PhaseJump3,
        PhaseJump4,
        PhaseJump5,
        PhaserDown1,
        PhaserDown2,
        PhaserDown3,
        // Atmosphere (5)
        CreepyDarkLogo,
        ElementalMagicSpellImpact,
        EpicOrchestraTransition,
        MysteriousBass,
        RetroGame,
        // Motion (7)
        AirReverseBurst,
        AirWoosh,
        AirZoomVacuum,
        ArrowWhoosh,
        BicycleHorn,
        BottleCork,
        Bullet,
    ];

    assert_eq!(
        all_effects.len(),
        53,
        "Total effect count should be 53 (11+13+6+11+5+7)"
    );

    for effect in &all_effects {
        let bytes = effect.bytes();
        assert!(
            bytes.len() >= 12,
            "Effect {:?} should have at least 12 bytes for format detection",
            effect
        );
    }
}

#[test]
fn all_effects_are_distinct() {
    use SoundEffect::*;

    let all_effects: Vec<SoundEffect> = vec![
        // UI (11)
        Doorbell,
        Doorbell2,
        SpaceAlarm,
        DitHit1,
        DitHit2,
        ElectronicHitFx01,
        ElectronicHitFx03,
        ElectronicHitFx06,
        ElectronicHitFx07,
        ElectronicHitFx14,
        ElectronicHitFx16,
        // Cartoon (13)
        CartoonAccent01,
        CartoonAccent02,
        CartoonAccent04,
        CartoonAccent07,
        CartoonAccent09,
        CartoonAccent10,
        CartoonAccent12,
        CartoonAccent17,
        CartoonAccent25,
        CartoonAccent31,
        CartoonAccent33,
        CartoonAccent35,
        CartoonCry,
        // Reactions (6)
        CrowdLaugh,
        CrowdLaughApplause,
        SadTrombone,
        SmallGroupCheer,
        FemaleAstonishedGasp,
        Sneeze,
        // Sci-Fi (11)
        HighDown,
        HighUp,
        TwoTone,
        PhaseJump1,
        PhaseJump2,
        PhaseJump3,
        PhaseJump4,
        PhaseJump5,
        PhaserDown1,
        PhaserDown2,
        PhaserDown3,
        // Atmosphere (5)
        CreepyDarkLogo,
        ElementalMagicSpellImpact,
        EpicOrchestraTransition,
        MysteriousBass,
        RetroGame,
        // Motion (7)
        AirReverseBurst,
        AirWoosh,
        AirZoomVacuum,
        ArrowWhoosh,
        BicycleHorn,
        BottleCork,
        Bullet,
    ];

    let set: HashSet<_> = all_effects.iter().collect();
    assert_eq!(
        set.len(),
        53,
        "All 53 effects should be distinct enum variants"
    );
}

#[test]
fn format_distribution_matches_expectations() {
    use SoundEffect::*;

    // WAV: 40 effects (UI=11, Cartoon=13, Reactions=6, Atmosphere=3, Motion=7)
    let wav_effects: Vec<SoundEffect> = vec![
        // UI (11)
        Doorbell,
        Doorbell2,
        SpaceAlarm,
        DitHit1,
        DitHit2,
        ElectronicHitFx01,
        ElectronicHitFx03,
        ElectronicHitFx06,
        ElectronicHitFx07,
        ElectronicHitFx14,
        ElectronicHitFx16,
        // Cartoon (13)
        CartoonAccent01,
        CartoonAccent02,
        CartoonAccent04,
        CartoonAccent07,
        CartoonAccent09,
        CartoonAccent10,
        CartoonAccent12,
        CartoonAccent17,
        CartoonAccent25,
        CartoonAccent31,
        CartoonAccent33,
        CartoonAccent35,
        CartoonCry,
        // Reactions (6)
        CrowdLaugh,
        CrowdLaughApplause,
        SadTrombone,
        SmallGroupCheer,
        FemaleAstonishedGasp,
        Sneeze,
        // Atmosphere WAV (3)
        EpicOrchestraTransition,
        MysteriousBass,
        RetroGame,
        // Motion (7)
        AirReverseBurst,
        AirWoosh,
        AirZoomVacuum,
        ArrowWhoosh,
        BicycleHorn,
        BottleCork,
        Bullet,
    ];

    // OGG: 11 effects (all Sci-Fi)
    let ogg_effects: Vec<SoundEffect> = vec![
        HighDown,
        HighUp,
        TwoTone,
        PhaseJump1,
        PhaseJump2,
        PhaseJump3,
        PhaseJump4,
        PhaseJump5,
        PhaserDown1,
        PhaserDown2,
        PhaserDown3,
    ];

    // MP3: 2 effects (Atmosphere)
    let mp3_effects: Vec<SoundEffect> = vec![CreepyDarkLogo, ElementalMagicSpellImpact];

    assert_eq!(wav_effects.len(), 40, "Expected 40 WAV effects");
    assert_eq!(ogg_effects.len(), 11, "Expected 11 OGG effects");
    assert_eq!(mp3_effects.len(), 2, "Expected 2 MP3 effects");

    // Verify format detection for each category
    for effect in &wav_effects {
        let format = detect_audio_format_from_bytes(effect.bytes()).unwrap();
        assert_eq!(
            format.file_format,
            AudioFileFormat::Wav,
            "Expected WAV for {:?}",
            effect
        );
    }

    for effect in &ogg_effects {
        let format = detect_audio_format_from_bytes(effect.bytes()).unwrap();
        assert_eq!(
            format.file_format,
            AudioFileFormat::Ogg,
            "Expected OGG for {:?}",
            effect
        );
    }

    for effect in &mp3_effects {
        let format = detect_audio_format_from_bytes(effect.bytes()).unwrap();
        assert_eq!(
            format.file_format,
            AudioFileFormat::Mp3,
            "Expected MP3 for {:?}",
            effect
        );
    }
}

// ============================================================================
// Playa Integration
// ============================================================================

#[test]
fn effects_create_valid_playa_instances() {
    use playa::Playa;
    use SoundEffect::*;

    // Test one effect from each category
    let sample_effects = [
        Doorbell,        // UI - WAV
        CartoonAccent01, // Cartoon - WAV
        SadTrombone,     // Reactions - WAV
        PhaseJump1,      // Sci-Fi - OGG
        CreepyDarkLogo,  // Atmosphere - MP3
        AirWoosh,        // Motion - WAV
    ];

    for effect in sample_effects {
        let bytes = effect.bytes();
        let playa = Playa::from_bytes(bytes.to_vec());
        assert!(
            playa.is_ok(),
            "Playa::from_bytes should succeed for {:?}",
            effect
        );

        // Verify format is accessible
        let playa = playa.unwrap();
        let format = playa.format();
        assert!(
            format.file_format == AudioFileFormat::Wav
                || format.file_format == AudioFileFormat::Ogg
                || format.file_format == AudioFileFormat::Mp3,
            "Playa format should be WAV, OGG, or MP3 for {:?}, got {:?}",
            effect,
            format.file_format
        );
    }
}

#[test]
fn playa_format_matches_detection() {
    use playa::Playa;
    use SoundEffect::*;

    // Test all effects to ensure Playa reports same format as direct detection
    let all_effects: Vec<SoundEffect> = vec![
        // UI (11)
        Doorbell,
        Doorbell2,
        SpaceAlarm,
        DitHit1,
        DitHit2,
        ElectronicHitFx01,
        ElectronicHitFx03,
        ElectronicHitFx06,
        ElectronicHitFx07,
        ElectronicHitFx14,
        ElectronicHitFx16,
        // Cartoon (13)
        CartoonAccent01,
        CartoonAccent02,
        CartoonAccent04,
        CartoonAccent07,
        CartoonAccent09,
        CartoonAccent10,
        CartoonAccent12,
        CartoonAccent17,
        CartoonAccent25,
        CartoonAccent31,
        CartoonAccent33,
        CartoonAccent35,
        CartoonCry,
        // Reactions (6)
        CrowdLaugh,
        CrowdLaughApplause,
        SadTrombone,
        SmallGroupCheer,
        FemaleAstonishedGasp,
        Sneeze,
        // Sci-Fi (11)
        HighDown,
        HighUp,
        TwoTone,
        PhaseJump1,
        PhaseJump2,
        PhaseJump3,
        PhaseJump4,
        PhaseJump5,
        PhaserDown1,
        PhaserDown2,
        PhaserDown3,
        // Atmosphere (5)
        CreepyDarkLogo,
        ElementalMagicSpellImpact,
        EpicOrchestraTransition,
        MysteriousBass,
        RetroGame,
        // Motion (7)
        AirReverseBurst,
        AirWoosh,
        AirZoomVacuum,
        ArrowWhoosh,
        BicycleHorn,
        BottleCork,
        Bullet,
    ];

    for effect in all_effects {
        let bytes = effect.bytes();
        let detected = detect_audio_format_from_bytes(bytes)
            .unwrap_or_else(|e| panic!("Detection should succeed for {:?}: {}", effect, e));

        let playa = Playa::from_bytes(bytes.to_vec())
            .unwrap_or_else(|e| panic!("Playa creation should succeed for {:?}: {}", effect, e));

        assert_eq!(
            playa.format().file_format,
            detected.file_format,
            "Playa format should match detected format for {:?}",
            effect
        );
    }
}

#[test]
fn sound_effect_names_roundtrip() {
    for effect in SoundEffect::all() {
        let name = effect.name();
        assert_eq!(SoundEffect::from_name(name), Some(effect));
    }
}

#[test]
fn sound_effect_all_matches_total_count() {
    assert_eq!(SoundEffect::all().len(), 53, "Expected 53 total effects");
}
