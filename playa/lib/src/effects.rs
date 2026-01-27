//! Sound effects module with categorized audio assets.
//!
//! This module provides embedded sound effects organized into categories.
//! Each category must be explicitly enabled via feature flags to minimize
//! binary size.
//!
//! ## Categories
//!
//! | Feature | Contents | Size |
//! |---------|----------|------|
//! | `sfx-ui` | Doorbells, alerts, hits | ~3MB |
//! | `sfx-cartoon` | Cartoon accents, cries | ~8MB |
//! | `sfx-reactions` | Laughs, cheers, trombone | ~4MB |
//! | `sfx-scifi` | Phase jumps, phasers | ~3MB |
//! | `sfx-atmosphere` | Music stings, transitions | ~7MB |
//! | `sfx-motion` | Whooshes, air sounds | ~5MB |
//!
//! ## Usage
//!
//! Enable features in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! playa = { version = "0.1", features = ["sfx-ui", "sfx-reactions"] }
//! ```

/// A categorized sound effect with embedded audio data.
///
/// Enable feature flags to include effect categories in your binary.
/// See the module documentation for the full list of categories.
///
/// ## Examples
///
/// ```ignore
/// use playa::effects::SoundEffect;
///
/// // With sfx-ui feature enabled:
/// let effect = SoundEffect::Doorbell;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SoundEffect {
    // === UI Sounds (sfx-ui) ===
    /// Classic doorbell sound.
    #[cfg(feature = "sfx-ui")]
    Doorbell,
    /// Alternative doorbell variant.
    #[cfg(feature = "sfx-ui")]
    Doorbell2,
    /// Space station alarm sound.
    #[cfg(feature = "sfx-ui")]
    SpaceAlarm,
    /// Short digital hit sound, variant 1.
    #[cfg(feature = "sfx-ui")]
    DitHit1,
    /// Short digital hit sound, variant 2.
    #[cfg(feature = "sfx-ui")]
    DitHit2,
    /// Electronic hit effect 01.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx01,
    /// Electronic hit effect 03.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx03,
    /// Electronic hit effect 06.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx06,
    /// Electronic hit effect 07.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx07,
    /// Electronic hit effect 14.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx14,
    /// Electronic hit effect 16.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx16,

    // === Cartoon Sounds (sfx-cartoon) ===
    /// Cartoon accent sound 01.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent01,
    /// Cartoon accent sound 02.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent02,
    /// Cartoon accent sound 04.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent04,
    /// Cartoon accent sound 07.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent07,
    /// Cartoon accent sound 09.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent09,
    /// Cartoon accent sound 10.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent10,
    /// Cartoon accent sound 12.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent12,
    /// Cartoon accent sound 17.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent17,
    /// Cartoon accent sound 25.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent25,
    /// Cartoon accent sound 31.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent31,
    /// Cartoon accent sound 33.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent33,
    /// Cartoon accent sound 35.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent35,
    /// Exaggerated cartoon crying sound.
    #[cfg(feature = "sfx-cartoon")]
    CartoonCry,

    // === Reaction Sounds (sfx-reactions) ===
    /// Crowd laughter.
    #[cfg(feature = "sfx-reactions")]
    CrowdLaugh,
    /// Crowd laughter with applause.
    #[cfg(feature = "sfx-reactions")]
    CrowdLaughApplause,
    /// Classic sad trombone (wah-wah).
    #[cfg(feature = "sfx-reactions")]
    SadTrombone,
    /// Small group cheering.
    #[cfg(feature = "sfx-reactions")]
    SmallGroupCheer,
    /// Female astonished gasp.
    #[cfg(feature = "sfx-reactions")]
    FemaleAstonishedGasp,
    /// Sneeze sound.
    #[cfg(feature = "sfx-reactions")]
    Sneeze,

    // === Sci-Fi Sounds (sfx-scifi) ===
    /// High-pitched descending tone.
    #[cfg(feature = "sfx-scifi")]
    HighDown,
    /// High-pitched ascending tone.
    #[cfg(feature = "sfx-scifi")]
    HighUp,
    /// Two-tone sci-fi sound.
    #[cfg(feature = "sfx-scifi")]
    TwoTone,
    /// Phase jump sound, variant 1.
    #[cfg(feature = "sfx-scifi")]
    PhaseJump1,
    /// Phase jump sound, variant 2.
    #[cfg(feature = "sfx-scifi")]
    PhaseJump2,
    /// Phase jump sound, variant 3.
    #[cfg(feature = "sfx-scifi")]
    PhaseJump3,
    /// Phase jump sound, variant 4.
    #[cfg(feature = "sfx-scifi")]
    PhaseJump4,
    /// Phase jump sound, variant 5.
    #[cfg(feature = "sfx-scifi")]
    PhaseJump5,
    /// Phaser descending sound, variant 1.
    #[cfg(feature = "sfx-scifi")]
    PhaserDown1,
    /// Phaser descending sound, variant 2.
    #[cfg(feature = "sfx-scifi")]
    PhaserDown2,
    /// Phaser descending sound, variant 3.
    #[cfg(feature = "sfx-scifi")]
    PhaserDown3,

    // === Atmosphere Sounds (sfx-atmosphere) ===
    /// Creepy dark logo sting.
    #[cfg(feature = "sfx-atmosphere")]
    CreepyDarkLogo,
    /// Elemental magic spell impact.
    #[cfg(feature = "sfx-atmosphere")]
    ElementalMagicSpellImpact,
    /// Epic orchestra transition.
    #[cfg(feature = "sfx-atmosphere")]
    EpicOrchestraTransition,
    /// Mysterious bass pulse.
    #[cfg(feature = "sfx-atmosphere")]
    MysteriousBass,
    /// Retro game sound.
    #[cfg(feature = "sfx-atmosphere")]
    RetroGame,

    // === Motion Sounds (sfx-motion) ===
    /// Air reverse burst sound.
    #[cfg(feature = "sfx-motion")]
    AirReverseBurst,
    /// Air woosh sound.
    #[cfg(feature = "sfx-motion")]
    AirWoosh,
    /// Air zoom vacuum sound.
    #[cfg(feature = "sfx-motion")]
    AirZoomVacuum,
    /// Arrow whoosh sound.
    #[cfg(feature = "sfx-motion")]
    ArrowWhoosh,
    /// Bicycle horn honk.
    #[cfg(feature = "sfx-motion")]
    BicycleHorn,
    /// Bottle cork pop.
    #[cfg(feature = "sfx-motion")]
    BottleCork,
    /// Bullet sound.
    #[cfg(feature = "sfx-motion")]
    Bullet,
}

// ============================================================================
// Embedded Audio Bytes
// ============================================================================
//
// Each constant is conditionally compiled based on its category feature flag.
// This ensures only enabled sound effect categories are embedded in the binary.

// === UI Sounds (sfx-ui) ===
#[cfg(feature = "sfx-ui")]
const DOORBELL_BYTES: &[u8] = include_bytes!("../../effects/doorbell.wav");
#[cfg(feature = "sfx-ui")]
const DOORBELL_2_BYTES: &[u8] = include_bytes!("../../effects/doorbell-2.wav");
#[cfg(feature = "sfx-ui")]
const SPACE_ALARM_BYTES: &[u8] = include_bytes!("../../effects/space-alarm.wav");
#[cfg(feature = "sfx-ui")]
const DIT_HIT_1_BYTES: &[u8] = include_bytes!("../../effects/dit-hit-1.wav");
#[cfg(feature = "sfx-ui")]
const DIT_HIT_2_BYTES: &[u8] = include_bytes!("../../effects/dit-hit-2.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_01_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx-01_1.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_03_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx-03_1.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_06_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx-06_1.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_07_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx-07_1.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_14_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx-14_1.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_16_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx-16_1.wav");

// === Cartoon Sounds (sfx-cartoon) ===
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_01_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-01.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_02_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-02.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_04_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-04.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_07_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-07.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_09_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-09.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_10_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-10.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_12_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-12.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_17_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-17.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_25_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-25.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_31_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-31.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_33_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-33.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_35_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent-35.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_CRY_BYTES: &[u8] = include_bytes!("../../effects/cartoon-cry.wav");

// === Reaction Sounds (sfx-reactions) ===
#[cfg(feature = "sfx-reactions")]
const CROWD_LAUGH_BYTES: &[u8] = include_bytes!("../../effects/crowd-laugh.wav");
#[cfg(feature = "sfx-reactions")]
const CROWD_LAUGH_APPLAUSE_BYTES: &[u8] = include_bytes!("../../effects/crowd-laugh-applause.wav");
#[cfg(feature = "sfx-reactions")]
const SAD_TROMBONE_BYTES: &[u8] = include_bytes!("../../effects/sad-trombone.wav");
#[cfg(feature = "sfx-reactions")]
const SMALL_GROUP_CHEER_BYTES: &[u8] = include_bytes!("../../effects/small-group-cheer.wav");
#[cfg(feature = "sfx-reactions")]
const FEMALE_ASTONISHED_GASP_BYTES: &[u8] = include_bytes!("../../effects/female-astonished-gasp.wav");
#[cfg(feature = "sfx-reactions")]
const SNEEZE_BYTES: &[u8] = include_bytes!("../../effects/sneeze.wav");

// === Sci-Fi Sounds (sfx-scifi) ===
#[cfg(feature = "sfx-scifi")]
const HIGH_DOWN_BYTES: &[u8] = include_bytes!("../../effects/highDown.ogg");
#[cfg(feature = "sfx-scifi")]
const HIGH_UP_BYTES: &[u8] = include_bytes!("../../effects/highUp.ogg");
#[cfg(feature = "sfx-scifi")]
const TWO_TONE_BYTES: &[u8] = include_bytes!("../../effects/two-tone.ogg");
#[cfg(feature = "sfx-scifi")]
const PHASE_JUMP_1_BYTES: &[u8] = include_bytes!("../../effects/phaseJump1.ogg");
#[cfg(feature = "sfx-scifi")]
const PHASE_JUMP_2_BYTES: &[u8] = include_bytes!("../../effects/phaseJump2.ogg");
#[cfg(feature = "sfx-scifi")]
const PHASE_JUMP_3_BYTES: &[u8] = include_bytes!("../../effects/phaseJump3.ogg");
#[cfg(feature = "sfx-scifi")]
const PHASE_JUMP_4_BYTES: &[u8] = include_bytes!("../../effects/phaseJump4.ogg");
#[cfg(feature = "sfx-scifi")]
const PHASE_JUMP_5_BYTES: &[u8] = include_bytes!("../../effects/phaseJump5.ogg");
#[cfg(feature = "sfx-scifi")]
const PHASER_DOWN_1_BYTES: &[u8] = include_bytes!("../../effects/phaserDown1.ogg");
#[cfg(feature = "sfx-scifi")]
const PHASER_DOWN_2_BYTES: &[u8] = include_bytes!("../../effects/phaserDown2.ogg");
#[cfg(feature = "sfx-scifi")]
const PHASER_DOWN_3_BYTES: &[u8] = include_bytes!("../../effects/phaserDown3.ogg");

// === Atmosphere Sounds (sfx-atmosphere) ===
#[cfg(feature = "sfx-atmosphere")]
const CREEPY_DARK_LOGO_BYTES: &[u8] = include_bytes!("../../effects/creepy-dark-logo.mp3");
#[cfg(feature = "sfx-atmosphere")]
const ELEMENTAL_MAGIC_SPELL_IMPACT_BYTES: &[u8] = include_bytes!("../../effects/elemental-magic-spell-impact.mp3");
#[cfg(feature = "sfx-atmosphere")]
const EPIC_ORCHESTRA_TRANSITION_BYTES: &[u8] = include_bytes!("../../effects/epic-orchestra-transition.wav");
#[cfg(feature = "sfx-atmosphere")]
const MYSTERIOUS_BASS_BYTES: &[u8] = include_bytes!("../../effects/mysterious-bass-pulse.wav");
#[cfg(feature = "sfx-atmosphere")]
const RETRO_GAME_BYTES: &[u8] = include_bytes!("../../effects/retro-game.wav");

// === Motion Sounds (sfx-motion) ===
#[cfg(feature = "sfx-motion")]
const AIR_REVERSE_BURST_BYTES: &[u8] = include_bytes!("../../effects/air-reverse-burst.wav");
#[cfg(feature = "sfx-motion")]
const AIR_WOOSH_BYTES: &[u8] = include_bytes!("../../effects/air-woosh.wav");
#[cfg(feature = "sfx-motion")]
const AIR_ZOOM_VACUUM_BYTES: &[u8] = include_bytes!("../../effects/air-zoom-vacuum.wav");
#[cfg(feature = "sfx-motion")]
const ARROW_WHOOSH_BYTES: &[u8] = include_bytes!("../../effects/arrow-whoosh.wav");
#[cfg(feature = "sfx-motion")]
const BICYCLE_HORN_BYTES: &[u8] = include_bytes!("../../effects/bicycle-horn.wav");
#[cfg(feature = "sfx-motion")]
const BOTTLE_CORK_BYTES: &[u8] = include_bytes!("../../effects/bottle-cork.wav");
#[cfg(feature = "sfx-motion")]
const BULLET_BYTES: &[u8] = include_bytes!("../../effects/bullet.wav");

impl SoundEffect {
    /// Returns the embedded audio bytes for this sound effect.
    ///
    /// This method is private and used internally by play methods.
    /// Each match arm is guarded by the same feature flag as its
    /// corresponding enum variant.
    fn as_bytes(&self) -> &'static [u8] {
        match self {
            // === UI Sounds (sfx-ui) ===
            #[cfg(feature = "sfx-ui")]
            Self::Doorbell => DOORBELL_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Doorbell2 => DOORBELL_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::SpaceAlarm => SPACE_ALARM_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::DitHit1 => DIT_HIT_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::DitHit2 => DIT_HIT_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx01 => ELECTRONIC_HIT_FX_01_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx03 => ELECTRONIC_HIT_FX_03_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx06 => ELECTRONIC_HIT_FX_06_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx07 => ELECTRONIC_HIT_FX_07_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx14 => ELECTRONIC_HIT_FX_14_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx16 => ELECTRONIC_HIT_FX_16_BYTES,

            // === Cartoon Sounds (sfx-cartoon) ===
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent01 => CARTOON_ACCENT_01_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent02 => CARTOON_ACCENT_02_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent04 => CARTOON_ACCENT_04_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent07 => CARTOON_ACCENT_07_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent09 => CARTOON_ACCENT_09_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent10 => CARTOON_ACCENT_10_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent12 => CARTOON_ACCENT_12_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent17 => CARTOON_ACCENT_17_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent25 => CARTOON_ACCENT_25_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent31 => CARTOON_ACCENT_31_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent33 => CARTOON_ACCENT_33_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent35 => CARTOON_ACCENT_35_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonCry => CARTOON_CRY_BYTES,

            // === Reaction Sounds (sfx-reactions) ===
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdLaugh => CROWD_LAUGH_BYTES,
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdLaughApplause => CROWD_LAUGH_APPLAUSE_BYTES,
            #[cfg(feature = "sfx-reactions")]
            Self::SadTrombone => SAD_TROMBONE_BYTES,
            #[cfg(feature = "sfx-reactions")]
            Self::SmallGroupCheer => SMALL_GROUP_CHEER_BYTES,
            #[cfg(feature = "sfx-reactions")]
            Self::FemaleAstonishedGasp => FEMALE_ASTONISHED_GASP_BYTES,
            #[cfg(feature = "sfx-reactions")]
            Self::Sneeze => SNEEZE_BYTES,

            // === Sci-Fi Sounds (sfx-scifi) ===
            #[cfg(feature = "sfx-scifi")]
            Self::HighDown => HIGH_DOWN_BYTES,
            #[cfg(feature = "sfx-scifi")]
            Self::HighUp => HIGH_UP_BYTES,
            #[cfg(feature = "sfx-scifi")]
            Self::TwoTone => TWO_TONE_BYTES,
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump1 => PHASE_JUMP_1_BYTES,
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump2 => PHASE_JUMP_2_BYTES,
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump3 => PHASE_JUMP_3_BYTES,
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump4 => PHASE_JUMP_4_BYTES,
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump5 => PHASE_JUMP_5_BYTES,
            #[cfg(feature = "sfx-scifi")]
            Self::PhaserDown1 => PHASER_DOWN_1_BYTES,
            #[cfg(feature = "sfx-scifi")]
            Self::PhaserDown2 => PHASER_DOWN_2_BYTES,
            #[cfg(feature = "sfx-scifi")]
            Self::PhaserDown3 => PHASER_DOWN_3_BYTES,

            // === Atmosphere Sounds (sfx-atmosphere) ===
            #[cfg(feature = "sfx-atmosphere")]
            Self::CreepyDarkLogo => CREEPY_DARK_LOGO_BYTES,
            #[cfg(feature = "sfx-atmosphere")]
            Self::ElementalMagicSpellImpact => ELEMENTAL_MAGIC_SPELL_IMPACT_BYTES,
            #[cfg(feature = "sfx-atmosphere")]
            Self::EpicOrchestraTransition => EPIC_ORCHESTRA_TRANSITION_BYTES,
            #[cfg(feature = "sfx-atmosphere")]
            Self::MysteriousBass => MYSTERIOUS_BASS_BYTES,
            #[cfg(feature = "sfx-atmosphere")]
            Self::RetroGame => RETRO_GAME_BYTES,

            // === Motion Sounds (sfx-motion) ===
            #[cfg(feature = "sfx-motion")]
            Self::AirReverseBurst => AIR_REVERSE_BURST_BYTES,
            #[cfg(feature = "sfx-motion")]
            Self::AirWoosh => AIR_WOOSH_BYTES,
            #[cfg(feature = "sfx-motion")]
            Self::AirZoomVacuum => AIR_ZOOM_VACUUM_BYTES,
            #[cfg(feature = "sfx-motion")]
            Self::ArrowWhoosh => ARROW_WHOOSH_BYTES,
            #[cfg(feature = "sfx-motion")]
            Self::BicycleHorn => BICYCLE_HORN_BYTES,
            #[cfg(feature = "sfx-motion")]
            Self::BottleCork => BOTTLE_CORK_BYTES,
            #[cfg(feature = "sfx-motion")]
            Self::Bullet => BULLET_BYTES,
        }
    }

    /// Play this sound effect synchronously.
    ///
    /// This method blocks until playback completes. For non-blocking playback,
    /// use [`play_async`](Self::play_async) (requires the `async` feature).
    ///
    /// ## Errors
    ///
    /// Returns `PlaybackError` if:
    /// - No compatible audio player is available
    /// - The player process fails to spawn or exits with an error
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use playa::SoundEffect;
    ///
    /// // With sfx-reactions feature enabled:
    /// SoundEffect::SadTrombone.play()?;
    /// # Ok::<(), playa::PlaybackError>(())
    /// ```
    pub fn play(self) -> Result<(), crate::PlaybackError> {
        let playa = crate::Playa::from_bytes(self.as_bytes().to_vec())
            .map_err(|crate::InvalidAudio::Detection(e)| crate::PlaybackError::Detection(e))?;
        playa.play()
    }

    /// Returns the embedded audio bytes for this sound effect.
    ///
    /// This method is useful for testing format detection or
    /// for accessing the raw bytes without triggering playback.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use playa::{SoundEffect, detect_audio_format_from_bytes};
    ///
    /// // With sfx-reactions feature enabled:
    /// let bytes = SoundEffect::SadTrombone.bytes();
    /// let format = detect_audio_format_from_bytes(bytes)?;
    /// ```
    pub fn bytes(&self) -> &'static [u8] {
        self.as_bytes()
    }

    /// Play this sound effect asynchronously.
    ///
    /// This method returns immediately and plays the sound in the background.
    /// Requires the `async` feature flag.
    ///
    /// ## Errors
    ///
    /// Returns `PlaybackError` if:
    /// - No compatible audio player is available
    /// - The player process fails to spawn or exits with an error
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use playa::SoundEffect;
    ///
    /// // With sfx-reactions and async features enabled:
    /// SoundEffect::SadTrombone.play_async().await?;
    /// # Ok::<(), playa::PlaybackError>(())
    /// ```
    #[cfg(feature = "async")]
    pub async fn play_async(self) -> Result<(), crate::PlaybackError> {
        use std::sync::Arc;
        let bytes = Arc::new(self.as_bytes().to_vec());
        crate::playback::playa_async(crate::AudioData::Bytes(bytes)).await
    }
}
