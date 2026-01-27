//! Sound effects module with categorized audio assets.
//!
//! This module provides embedded sound effects organized into categories.
//! Each category must be explicitly enabled via feature flags to minimize
//! binary size.
//!
//! ## Binary Size Warning
//!
//! Sound effects are embedded directly in your binary. Enabling all categories
//! (`sound-effects` feature) adds approximately **30MB** to your binary size.
//! Only enable the categories you actually need.
//!
//! ## Categories
//!
//! | Feature | Contents | Effects | Size |
//! |---------|----------|---------|------|
//! | `sfx-ui` | Doorbells, alerts, hits | 11 | ~3MB |
//! | `sfx-cartoon` | Cartoon accents, cries | 13 | ~8MB |
//! | `sfx-reactions` | Laughs, cheers, trombone | 6 | ~4MB |
//! | `sfx-scifi` | Phase jumps, phasers | 11 | ~3MB |
//! | `sfx-atmosphere` | Music stings, transitions | 5 | ~7MB |
//! | `sfx-motion` | Whooshes, air sounds | 7 | ~5MB |
//! | **`sound-effects`** | **All categories** | **53** | **~30MB** |
//!
//! ## Usage
//!
//! Enable features in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! playa = { version = "0.1", features = ["sfx-ui", "sfx-reactions"] }
//! ```
//!
//! Then use the effects in your code:
//!
//! ```ignore
//! use playa::SoundEffect;
//!
//! // Play synchronously (blocks until complete)
//! SoundEffect::SadTrombone.play()?;
//!
//! // Play asynchronously (requires `async` feature)
//! SoundEffect::Doorbell.play_async().await?;
//! ```
//!
//! ## Supported Formats
//!
//! Effects are stored in their original formats:
//! - **WAV**: UI, cartoon, reactions, atmosphere (partial), motion
//! - **OGG**: Sci-fi effects
//! - **MP3**: Some atmosphere effects
//!
//! Format detection is automatic - you don't need to know the underlying format.

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
    /// Return the stable CLI name for this effect (kebab-case).
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use playa::SoundEffect;
    ///
    /// // With sfx-reactions feature enabled:
    /// let name = SoundEffect::SadTrombone.name();
    /// assert_eq!(name, "sad-trombone");
    /// ```
    ///
    /// ## Returns
    ///
    /// - The kebab-case name for this effect.
    pub fn name(&self) -> &'static str {
        match self {
            // === UI Sounds (sfx-ui) ===
            #[cfg(feature = "sfx-ui")]
            Self::Doorbell => "doorbell",
            #[cfg(feature = "sfx-ui")]
            Self::Doorbell2 => "doorbell-2",
            #[cfg(feature = "sfx-ui")]
            Self::SpaceAlarm => "space-alarm",
            #[cfg(feature = "sfx-ui")]
            Self::DitHit1 => "dit-hit-1",
            #[cfg(feature = "sfx-ui")]
            Self::DitHit2 => "dit-hit-2",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx01 => "electronic-hit-fx-01",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx03 => "electronic-hit-fx-03",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx06 => "electronic-hit-fx-06",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx07 => "electronic-hit-fx-07",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx14 => "electronic-hit-fx-14",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx16 => "electronic-hit-fx-16",

            // === Cartoon Sounds (sfx-cartoon) ===
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent01 => "cartoon-accent-01",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent02 => "cartoon-accent-02",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent04 => "cartoon-accent-04",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent07 => "cartoon-accent-07",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent09 => "cartoon-accent-09",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent10 => "cartoon-accent-10",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent12 => "cartoon-accent-12",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent17 => "cartoon-accent-17",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent25 => "cartoon-accent-25",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent31 => "cartoon-accent-31",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent33 => "cartoon-accent-33",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent35 => "cartoon-accent-35",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonCry => "cartoon-cry",

            // === Reaction Sounds (sfx-reactions) ===
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdLaugh => "crowd-laugh",
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdLaughApplause => "crowd-laugh-applause",
            #[cfg(feature = "sfx-reactions")]
            Self::SadTrombone => "sad-trombone",
            #[cfg(feature = "sfx-reactions")]
            Self::SmallGroupCheer => "small-group-cheer",
            #[cfg(feature = "sfx-reactions")]
            Self::FemaleAstonishedGasp => "female-astonished-gasp",
            #[cfg(feature = "sfx-reactions")]
            Self::Sneeze => "sneeze",

            // === Sci-Fi Sounds (sfx-scifi) ===
            #[cfg(feature = "sfx-scifi")]
            Self::HighDown => "high-down",
            #[cfg(feature = "sfx-scifi")]
            Self::HighUp => "high-up",
            #[cfg(feature = "sfx-scifi")]
            Self::TwoTone => "two-tone",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump1 => "phase-jump-1",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump2 => "phase-jump-2",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump3 => "phase-jump-3",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump4 => "phase-jump-4",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump5 => "phase-jump-5",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaserDown1 => "phaser-down-1",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaserDown2 => "phaser-down-2",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaserDown3 => "phaser-down-3",

            // === Atmosphere Sounds (sfx-atmosphere) ===
            #[cfg(feature = "sfx-atmosphere")]
            Self::CreepyDarkLogo => "creepy-dark-logo",
            #[cfg(feature = "sfx-atmosphere")]
            Self::ElementalMagicSpellImpact => "elemental-magic-spell-impact",
            #[cfg(feature = "sfx-atmosphere")]
            Self::EpicOrchestraTransition => "epic-orchestra-transition",
            #[cfg(feature = "sfx-atmosphere")]
            Self::MysteriousBass => "mysterious-bass",
            #[cfg(feature = "sfx-atmosphere")]
            Self::RetroGame => "retro-game",

            // === Motion Sounds (sfx-motion) ===
            #[cfg(feature = "sfx-motion")]
            Self::AirReverseBurst => "air-reverse-burst",
            #[cfg(feature = "sfx-motion")]
            Self::AirWoosh => "air-woosh",
            #[cfg(feature = "sfx-motion")]
            Self::AirZoomVacuum => "air-zoom-vacuum",
            #[cfg(feature = "sfx-motion")]
            Self::ArrowWhoosh => "arrow-whoosh",
            #[cfg(feature = "sfx-motion")]
            Self::BicycleHorn => "bicycle-horn",
            #[cfg(feature = "sfx-motion")]
            Self::BottleCork => "bottle-cork",
            #[cfg(feature = "sfx-motion")]
            Self::Bullet => "bullet",
        }
    }

    /// Return all available effects compiled into this build.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use playa::SoundEffect;
    ///
    /// let effects = SoundEffect::all();
    /// assert!(!effects.is_empty());
    /// ```
    ///
    /// ## Returns
    ///
    /// - A vector of all compiled sound effects in category order.
    pub fn all() -> Vec<SoundEffect> {
        let mut effects = Vec::new();

        #[cfg(feature = "sfx-ui")]
        effects.extend_from_slice(&[
            Self::Doorbell,
            Self::Doorbell2,
            Self::SpaceAlarm,
            Self::DitHit1,
            Self::DitHit2,
            Self::ElectronicHitFx01,
            Self::ElectronicHitFx03,
            Self::ElectronicHitFx06,
            Self::ElectronicHitFx07,
            Self::ElectronicHitFx14,
            Self::ElectronicHitFx16,
        ]);

        #[cfg(feature = "sfx-cartoon")]
        effects.extend_from_slice(&[
            Self::CartoonAccent01,
            Self::CartoonAccent02,
            Self::CartoonAccent04,
            Self::CartoonAccent07,
            Self::CartoonAccent09,
            Self::CartoonAccent10,
            Self::CartoonAccent12,
            Self::CartoonAccent17,
            Self::CartoonAccent25,
            Self::CartoonAccent31,
            Self::CartoonAccent33,
            Self::CartoonAccent35,
            Self::CartoonCry,
        ]);

        #[cfg(feature = "sfx-reactions")]
        effects.extend_from_slice(&[
            Self::CrowdLaugh,
            Self::CrowdLaughApplause,
            Self::SadTrombone,
            Self::SmallGroupCheer,
            Self::FemaleAstonishedGasp,
            Self::Sneeze,
        ]);

        #[cfg(feature = "sfx-scifi")]
        effects.extend_from_slice(&[
            Self::HighDown,
            Self::HighUp,
            Self::TwoTone,
            Self::PhaseJump1,
            Self::PhaseJump2,
            Self::PhaseJump3,
            Self::PhaseJump4,
            Self::PhaseJump5,
            Self::PhaserDown1,
            Self::PhaserDown2,
            Self::PhaserDown3,
        ]);

        #[cfg(feature = "sfx-atmosphere")]
        effects.extend_from_slice(&[
            Self::CreepyDarkLogo,
            Self::ElementalMagicSpellImpact,
            Self::EpicOrchestraTransition,
            Self::MysteriousBass,
            Self::RetroGame,
        ]);

        #[cfg(feature = "sfx-motion")]
        effects.extend_from_slice(&[
            Self::AirReverseBurst,
            Self::AirWoosh,
            Self::AirZoomVacuum,
            Self::ArrowWhoosh,
            Self::BicycleHorn,
            Self::BottleCork,
            Self::Bullet,
        ]);

        effects
    }

    /// Parse a CLI effect name into a `SoundEffect`.
    ///
    /// Accepts kebab-case names and normalizes case, underscores, and spaces.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use playa::SoundEffect;
    ///
    /// let effect = SoundEffect::from_name("sad-trombone");
    /// assert_eq!(effect, Some(SoundEffect::SadTrombone));
    /// ```
    ///
    /// ## Returns
    ///
    /// - `Some(effect)` when the name matches a compiled effect.
    /// - `None` when no match is found.
    pub fn from_name(name: &str) -> Option<Self> {
        let normalized = normalize_effect_name(name);
        match normalized.as_str() {
            // === UI Sounds (sfx-ui) ===
            #[cfg(feature = "sfx-ui")]
            "doorbell" => Some(Self::Doorbell),
            #[cfg(feature = "sfx-ui")]
            "doorbell-2" => Some(Self::Doorbell2),
            #[cfg(feature = "sfx-ui")]
            "space-alarm" => Some(Self::SpaceAlarm),
            #[cfg(feature = "sfx-ui")]
            "dit-hit-1" => Some(Self::DitHit1),
            #[cfg(feature = "sfx-ui")]
            "dit-hit-2" => Some(Self::DitHit2),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx-01" => Some(Self::ElectronicHitFx01),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx-03" => Some(Self::ElectronicHitFx03),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx-06" => Some(Self::ElectronicHitFx06),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx-07" => Some(Self::ElectronicHitFx07),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx-14" => Some(Self::ElectronicHitFx14),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx-16" => Some(Self::ElectronicHitFx16),

            // === Cartoon Sounds (sfx-cartoon) ===
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-01" => Some(Self::CartoonAccent01),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-02" => Some(Self::CartoonAccent02),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-04" => Some(Self::CartoonAccent04),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-07" => Some(Self::CartoonAccent07),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-09" => Some(Self::CartoonAccent09),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-10" => Some(Self::CartoonAccent10),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-12" => Some(Self::CartoonAccent12),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-17" => Some(Self::CartoonAccent17),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-25" => Some(Self::CartoonAccent25),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-31" => Some(Self::CartoonAccent31),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-33" => Some(Self::CartoonAccent33),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent-35" => Some(Self::CartoonAccent35),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-cry" => Some(Self::CartoonCry),

            // === Reaction Sounds (sfx-reactions) ===
            #[cfg(feature = "sfx-reactions")]
            "crowd-laugh" => Some(Self::CrowdLaugh),
            #[cfg(feature = "sfx-reactions")]
            "crowd-laugh-applause" => Some(Self::CrowdLaughApplause),
            #[cfg(feature = "sfx-reactions")]
            "sad-trombone" => Some(Self::SadTrombone),
            #[cfg(feature = "sfx-reactions")]
            "small-group-cheer" => Some(Self::SmallGroupCheer),
            #[cfg(feature = "sfx-reactions")]
            "female-astonished-gasp" => Some(Self::FemaleAstonishedGasp),
            #[cfg(feature = "sfx-reactions")]
            "sneeze" => Some(Self::Sneeze),

            // === Sci-Fi Sounds (sfx-scifi) ===
            #[cfg(feature = "sfx-scifi")]
            "high-down" => Some(Self::HighDown),
            #[cfg(feature = "sfx-scifi")]
            "high-up" => Some(Self::HighUp),
            #[cfg(feature = "sfx-scifi")]
            "two-tone" => Some(Self::TwoTone),
            #[cfg(feature = "sfx-scifi")]
            "phase-jump-1" => Some(Self::PhaseJump1),
            #[cfg(feature = "sfx-scifi")]
            "phase-jump-2" => Some(Self::PhaseJump2),
            #[cfg(feature = "sfx-scifi")]
            "phase-jump-3" => Some(Self::PhaseJump3),
            #[cfg(feature = "sfx-scifi")]
            "phase-jump-4" => Some(Self::PhaseJump4),
            #[cfg(feature = "sfx-scifi")]
            "phase-jump-5" => Some(Self::PhaseJump5),
            #[cfg(feature = "sfx-scifi")]
            "phaser-down-1" => Some(Self::PhaserDown1),
            #[cfg(feature = "sfx-scifi")]
            "phaser-down-2" => Some(Self::PhaserDown2),
            #[cfg(feature = "sfx-scifi")]
            "phaser-down-3" => Some(Self::PhaserDown3),

            // === Atmosphere Sounds (sfx-atmosphere) ===
            #[cfg(feature = "sfx-atmosphere")]
            "creepy-dark-logo" => Some(Self::CreepyDarkLogo),
            #[cfg(feature = "sfx-atmosphere")]
            "elemental-magic-spell-impact" => Some(Self::ElementalMagicSpellImpact),
            #[cfg(feature = "sfx-atmosphere")]
            "epic-orchestra-transition" => Some(Self::EpicOrchestraTransition),
            #[cfg(feature = "sfx-atmosphere")]
            "mysterious-bass" => Some(Self::MysteriousBass),
            #[cfg(feature = "sfx-atmosphere")]
            "retro-game" => Some(Self::RetroGame),

            // === Motion Sounds (sfx-motion) ===
            #[cfg(feature = "sfx-motion")]
            "air-reverse-burst" => Some(Self::AirReverseBurst),
            #[cfg(feature = "sfx-motion")]
            "air-woosh" => Some(Self::AirWoosh),
            #[cfg(feature = "sfx-motion")]
            "air-zoom-vacuum" => Some(Self::AirZoomVacuum),
            #[cfg(feature = "sfx-motion")]
            "arrow-whoosh" => Some(Self::ArrowWhoosh),
            #[cfg(feature = "sfx-motion")]
            "bicycle-horn" => Some(Self::BicycleHorn),
            #[cfg(feature = "sfx-motion")]
            "bottle-cork" => Some(Self::BottleCork),
            #[cfg(feature = "sfx-motion")]
            "bullet" => Some(Self::Bullet),
            _ => None,
        }
    }

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
    /// use `play_async` (requires the `async` feature).
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

fn normalize_effect_name(name: &str) -> String {
    name.trim()
        .to_ascii_lowercase()
        .replace('_', "-")
        .replace(' ', "-")
}
