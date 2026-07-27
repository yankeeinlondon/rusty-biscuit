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
//! | `sfx-ui` | Doorbells, alerts, clicks, selections, window actions | 43 | ~4MB |
//! | `sfx-cartoon` | Cartoon accents, cries | 13 | ~8MB |
//! | `sfx-reactions` | Applause, laughs, cheers, trombone | 9 | ~4MB |
//! | `sfx-scifi` | Phase jumps, phasers | 11 | ~3MB |
//! | `sfx-atmosphere` | Music stings, transitions | 5 | ~7MB |
//! | `sfx-motion` | Whooshes, air sounds | 7 | ~5MB |
//! | **`sound-effects`** | **All categories** | **88** | **~31MB** |
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

mod effect_durations {
    include!(concat!(env!("OUT_DIR"), "/effect_durations.rs"));
}

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
    /// Electronic hit effect 1.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx1,
    /// Electronic hit effect 2.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx2,
    /// Electronic hit effect 3.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx3,
    /// Electronic hit effect 4.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx4,
    /// Electronic hit effect 5.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx5,
    /// Electronic hit effect 6.
    #[cfg(feature = "sfx-ui")]
    ElectronicHitFx6,
    /// Bong/bell notification sound.
    #[cfg(feature = "sfx-ui")]
    Bong,
    /// Click sound.
    #[cfg(feature = "sfx-ui")]
    Click,
    /// Confirmation sound.
    #[cfg(feature = "sfx-ui")]
    Confirmation,
    /// Drop sound, variant 1.
    #[cfg(feature = "sfx-ui")]
    Drop1,
    /// Drop sound, variant 2.
    #[cfg(feature = "sfx-ui")]
    Drop2,
    /// Drop sound, variant 3.
    #[cfg(feature = "sfx-ui")]
    Drop3,
    /// Drop sound, variant 4.
    #[cfg(feature = "sfx-ui")]
    Drop4,
    /// Error sound, variant 1.
    #[cfg(feature = "sfx-ui")]
    Error1,
    /// Error sound, variant 2.
    #[cfg(feature = "sfx-ui")]
    Error2,
    /// Glass/chime sound, variant 1.
    #[cfg(feature = "sfx-ui")]
    Glass1,
    /// Glass/chime sound, variant 2.
    #[cfg(feature = "sfx-ui")]
    Glass2,
    /// Glass/chime sound, variant 3.
    #[cfg(feature = "sfx-ui")]
    Glass3,
    /// Glass/chime sound, variant 4.
    #[cfg(feature = "sfx-ui")]
    Glass4,
    /// Window maximize sound, variant 1.
    #[cfg(feature = "sfx-ui")]
    Maximize1,
    /// Window maximize sound, variant 2.
    #[cfg(feature = "sfx-ui")]
    Maximize2,
    /// Window maximize sound, variant 3.
    #[cfg(feature = "sfx-ui")]
    Maximize3,
    /// Window maximize sound, variant 4.
    #[cfg(feature = "sfx-ui")]
    Maximize4,
    /// Window maximize sound, variant 5.
    #[cfg(feature = "sfx-ui")]
    Maximize5,
    /// Window minimize sound, variant 1.
    #[cfg(feature = "sfx-ui")]
    Minimize1,
    /// Window minimize sound, variant 2.
    #[cfg(feature = "sfx-ui")]
    Minimize2,
    /// Window minimize sound, variant 3.
    #[cfg(feature = "sfx-ui")]
    Minimize3,
    /// Mouse click sound.
    #[cfg(feature = "sfx-ui")]
    MouseClick,
    /// Pluck sound, variant 1.
    #[cfg(feature = "sfx-ui")]
    Pluck1,
    /// Pluck sound, variant 2.
    #[cfg(feature = "sfx-ui")]
    Pluck2,
    /// Question/prompt sound, variant 1.
    #[cfg(feature = "sfx-ui")]
    Question1,
    /// Question/prompt sound, variant 2.
    #[cfg(feature = "sfx-ui")]
    Question2,
    /// Selection sound, variant 1.
    #[cfg(feature = "sfx-ui")]
    Select1,
    /// Selection sound, variant 2.
    #[cfg(feature = "sfx-ui")]
    Select2,
    /// Selection sound, variant 3.
    #[cfg(feature = "sfx-ui")]
    Select3,
    /// Selection sound, variant 4.
    #[cfg(feature = "sfx-ui")]
    Select4,
    /// Switch/toggle sound, variant 1.
    #[cfg(feature = "sfx-ui")]
    Switch1,
    /// Switch/toggle sound, variant 2.
    #[cfg(feature = "sfx-ui")]
    Switch2,

    // === Cartoon Sounds (sfx-cartoon) ===
    /// Cartoon accent sound 1.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent1,
    /// Cartoon accent sound 2.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent2,
    /// Cartoon accent sound 3.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent3,
    /// Cartoon accent sound 4.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent4,
    /// Cartoon accent sound 5.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent5,
    /// Cartoon accent sound 6.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent6,
    /// Cartoon accent sound 7.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent7,
    /// Cartoon accent sound 8.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent8,
    /// Cartoon accent sound 9.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent9,
    /// Cartoon accent sound 10.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent10,
    /// Cartoon accent sound 11.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent11,
    /// Cartoon accent sound 12.
    #[cfg(feature = "sfx-cartoon")]
    CartoonAccent12,
    /// Exaggerated cartoon crying sound.
    #[cfg(feature = "sfx-cartoon")]
    CartoonCry,

    // === Reaction Sounds (sfx-reactions) ===
    /// Crowd applause.
    #[cfg(feature = "sfx-reactions")]
    CrowdApplause,
    /// Crowd applause (recital setting).
    #[cfg(feature = "sfx-reactions")]
    CrowdApplauseRecital,
    /// Crowd applause (stadium setting).
    #[cfg(feature = "sfx-reactions")]
    CrowdApplauseStadium,
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
const ELECTRONIC_HIT_FX_1_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx1.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_2_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx2.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_3_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx3.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_4_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx4.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_5_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx5.wav");
#[cfg(feature = "sfx-ui")]
const ELECTRONIC_HIT_FX_6_BYTES: &[u8] = include_bytes!("../../effects/electronic-hit-fx6.wav");
#[cfg(feature = "sfx-ui")]
const BONG_BYTES: &[u8] = include_bytes!("../../effects/bong.ogg");
#[cfg(feature = "sfx-ui")]
const CLICK_BYTES: &[u8] = include_bytes!("../../effects/click.ogg");
#[cfg(feature = "sfx-ui")]
const CONFIRMATION_BYTES: &[u8] = include_bytes!("../../effects/confirmation.ogg");
#[cfg(feature = "sfx-ui")]
const DROP_1_BYTES: &[u8] = include_bytes!("../../effects/drop1.ogg");
#[cfg(feature = "sfx-ui")]
const DROP_2_BYTES: &[u8] = include_bytes!("../../effects/drop2.ogg");
#[cfg(feature = "sfx-ui")]
const DROP_3_BYTES: &[u8] = include_bytes!("../../effects/drop3.ogg");
#[cfg(feature = "sfx-ui")]
const DROP_4_BYTES: &[u8] = include_bytes!("../../effects/drop4.ogg");
#[cfg(feature = "sfx-ui")]
const ERROR_1_BYTES: &[u8] = include_bytes!("../../effects/error1.ogg");
#[cfg(feature = "sfx-ui")]
const ERROR_2_BYTES: &[u8] = include_bytes!("../../effects/error2.ogg");
#[cfg(feature = "sfx-ui")]
const GLASS_1_BYTES: &[u8] = include_bytes!("../../effects/glass1.ogg");
#[cfg(feature = "sfx-ui")]
const GLASS_2_BYTES: &[u8] = include_bytes!("../../effects/glass2.ogg");
#[cfg(feature = "sfx-ui")]
const GLASS_3_BYTES: &[u8] = include_bytes!("../../effects/glass3.ogg");
#[cfg(feature = "sfx-ui")]
const GLASS_4_BYTES: &[u8] = include_bytes!("../../effects/glass4.ogg");
#[cfg(feature = "sfx-ui")]
const MAXIMIZE_1_BYTES: &[u8] = include_bytes!("../../effects/maximize1.ogg");
#[cfg(feature = "sfx-ui")]
const MAXIMIZE_2_BYTES: &[u8] = include_bytes!("../../effects/maximize2.ogg");
#[cfg(feature = "sfx-ui")]
const MAXIMIZE_3_BYTES: &[u8] = include_bytes!("../../effects/maximize3.ogg");
#[cfg(feature = "sfx-ui")]
const MAXIMIZE_4_BYTES: &[u8] = include_bytes!("../../effects/maximize4.ogg");
#[cfg(feature = "sfx-ui")]
const MAXIMIZE_5_BYTES: &[u8] = include_bytes!("../../effects/maximize5.ogg");
#[cfg(feature = "sfx-ui")]
const MINIMIZE_1_BYTES: &[u8] = include_bytes!("../../effects/minimize1.ogg");
#[cfg(feature = "sfx-ui")]
const MINIMIZE_2_BYTES: &[u8] = include_bytes!("../../effects/minimize2.ogg");
#[cfg(feature = "sfx-ui")]
const MINIMIZE_3_BYTES: &[u8] = include_bytes!("../../effects/minimize3.ogg");
#[cfg(feature = "sfx-ui")]
const MOUSE_CLICK_BYTES: &[u8] = include_bytes!("../../effects/mouseclick.ogg");
#[cfg(feature = "sfx-ui")]
const PLUCK_1_BYTES: &[u8] = include_bytes!("../../effects/pluck1.ogg");
#[cfg(feature = "sfx-ui")]
const PLUCK_2_BYTES: &[u8] = include_bytes!("../../effects/pluck2.ogg");
#[cfg(feature = "sfx-ui")]
const QUESTION_1_BYTES: &[u8] = include_bytes!("../../effects/question1.ogg");
#[cfg(feature = "sfx-ui")]
const QUESTION_2_BYTES: &[u8] = include_bytes!("../../effects/question2.ogg");
#[cfg(feature = "sfx-ui")]
const SELECT_1_BYTES: &[u8] = include_bytes!("../../effects/select1.ogg");
#[cfg(feature = "sfx-ui")]
const SELECT_2_BYTES: &[u8] = include_bytes!("../../effects/select2.ogg");
#[cfg(feature = "sfx-ui")]
const SELECT_3_BYTES: &[u8] = include_bytes!("../../effects/select3.ogg");
#[cfg(feature = "sfx-ui")]
const SELECT_4_BYTES: &[u8] = include_bytes!("../../effects/select4.ogg");
#[cfg(feature = "sfx-ui")]
const SWITCH_1_BYTES: &[u8] = include_bytes!("../../effects/switch.ogg");
#[cfg(feature = "sfx-ui")]
const SWITCH_2_BYTES: &[u8] = include_bytes!("../../effects/switch2.ogg");

// === Cartoon Sounds (sfx-cartoon) ===
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_1_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent1.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_2_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent2.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_3_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent3.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_4_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent4.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_5_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent5.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_6_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent6.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_7_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent7.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_8_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent8.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_9_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent9.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_10_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent10.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_11_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent11.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_ACCENT_12_BYTES: &[u8] = include_bytes!("../../effects/cartoon-accent12.wav");
#[cfg(feature = "sfx-cartoon")]
const CARTOON_CRY_BYTES: &[u8] = include_bytes!("../../effects/cartoon-cry.wav");

// === Reaction Sounds (sfx-reactions) ===
#[cfg(feature = "sfx-reactions")]
const CROWD_APPLAUSE_BYTES: &[u8] = include_bytes!("../../effects/crowd-applause.wav");
#[cfg(feature = "sfx-reactions")]
const CROWD_APPLAUSE_RECITAL_BYTES: &[u8] =
    include_bytes!("../../effects/crowd-applause-recital.wav");
#[cfg(feature = "sfx-reactions")]
const CROWD_APPLAUSE_STADIUM_BYTES: &[u8] =
    include_bytes!("../../effects/crowd-applause-stadium.wav");
#[cfg(feature = "sfx-reactions")]
const CROWD_LAUGH_BYTES: &[u8] = include_bytes!("../../effects/crowd-laugh.wav");
#[cfg(feature = "sfx-reactions")]
const CROWD_LAUGH_APPLAUSE_BYTES: &[u8] = include_bytes!("../../effects/crowd-laugh-applause.wav");
#[cfg(feature = "sfx-reactions")]
const SAD_TROMBONE_BYTES: &[u8] = include_bytes!("../../effects/sad-trombone.wav");
#[cfg(feature = "sfx-reactions")]
const SMALL_GROUP_CHEER_BYTES: &[u8] = include_bytes!("../../effects/small-group-cheer.wav");
#[cfg(feature = "sfx-reactions")]
const FEMALE_ASTONISHED_GASP_BYTES: &[u8] =
    include_bytes!("../../effects/female-astonished-gasp.wav");
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
const ELEMENTAL_MAGIC_SPELL_IMPACT_BYTES: &[u8] =
    include_bytes!("../../effects/elemental-magic-spell-impact.mp3");
#[cfg(feature = "sfx-atmosphere")]
const EPIC_ORCHESTRA_TRANSITION_BYTES: &[u8] =
    include_bytes!("../../effects/epic-orchestra-transition.wav");
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
            Self::ElectronicHitFx1 => "electronic-hit-fx1",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx2 => "electronic-hit-fx2",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx3 => "electronic-hit-fx3",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx4 => "electronic-hit-fx4",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx5 => "electronic-hit-fx5",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx6 => "electronic-hit-fx6",
            #[cfg(feature = "sfx-ui")]
            Self::Bong => "bong",
            #[cfg(feature = "sfx-ui")]
            Self::Click => "click",
            #[cfg(feature = "sfx-ui")]
            Self::Confirmation => "confirmation",
            #[cfg(feature = "sfx-ui")]
            Self::Drop1 => "drop-1",
            #[cfg(feature = "sfx-ui")]
            Self::Drop2 => "drop-2",
            #[cfg(feature = "sfx-ui")]
            Self::Drop3 => "drop-3",
            #[cfg(feature = "sfx-ui")]
            Self::Drop4 => "drop-4",
            #[cfg(feature = "sfx-ui")]
            Self::Error1 => "error-1",
            #[cfg(feature = "sfx-ui")]
            Self::Error2 => "error-2",
            #[cfg(feature = "sfx-ui")]
            Self::Glass1 => "glass-1",
            #[cfg(feature = "sfx-ui")]
            Self::Glass2 => "glass-2",
            #[cfg(feature = "sfx-ui")]
            Self::Glass3 => "glass-3",
            #[cfg(feature = "sfx-ui")]
            Self::Glass4 => "glass-4",
            #[cfg(feature = "sfx-ui")]
            Self::Maximize1 => "maximize-1",
            #[cfg(feature = "sfx-ui")]
            Self::Maximize2 => "maximize-2",
            #[cfg(feature = "sfx-ui")]
            Self::Maximize3 => "maximize-3",
            #[cfg(feature = "sfx-ui")]
            Self::Maximize4 => "maximize-4",
            #[cfg(feature = "sfx-ui")]
            Self::Maximize5 => "maximize-5",
            #[cfg(feature = "sfx-ui")]
            Self::Minimize1 => "minimize-1",
            #[cfg(feature = "sfx-ui")]
            Self::Minimize2 => "minimize-2",
            #[cfg(feature = "sfx-ui")]
            Self::Minimize3 => "minimize-3",
            #[cfg(feature = "sfx-ui")]
            Self::MouseClick => "mouseclick",
            #[cfg(feature = "sfx-ui")]
            Self::Pluck1 => "pluck-1",
            #[cfg(feature = "sfx-ui")]
            Self::Pluck2 => "pluck-2",
            #[cfg(feature = "sfx-ui")]
            Self::Question1 => "question-1",
            #[cfg(feature = "sfx-ui")]
            Self::Question2 => "question-2",
            #[cfg(feature = "sfx-ui")]
            Self::Select1 => "select-1",
            #[cfg(feature = "sfx-ui")]
            Self::Select2 => "select-2",
            #[cfg(feature = "sfx-ui")]
            Self::Select3 => "select-3",
            #[cfg(feature = "sfx-ui")]
            Self::Select4 => "select-4",
            #[cfg(feature = "sfx-ui")]
            Self::Switch1 => "switch-1",
            #[cfg(feature = "sfx-ui")]
            Self::Switch2 => "switch-2",

            // === Cartoon Sounds (sfx-cartoon) ===
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent1 => "cartoon-accent1",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent2 => "cartoon-accent2",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent3 => "cartoon-accent3",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent4 => "cartoon-accent4",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent5 => "cartoon-accent5",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent6 => "cartoon-accent6",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent7 => "cartoon-accent7",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent8 => "cartoon-accent8",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent9 => "cartoon-accent9",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent10 => "cartoon-accent10",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent11 => "cartoon-accent11",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent12 => "cartoon-accent12",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonCry => "cartoon-cry",

            // === Reaction Sounds (sfx-reactions) ===
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdApplause => "crowd-applause",
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdApplauseRecital => "crowd-applause-recital",
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdApplauseStadium => "crowd-applause-stadium",
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

    /// Return the duration of this effect in milliseconds.
    ///
    /// ## Returns
    ///
    /// - `Some(duration_ms)` when the effect exists in the build-time manifest.
    /// - `None` if duration metadata is unavailable.
    pub fn duration_ms(&self) -> Option<u32> {
        effect_durations::duration_ms_for_name(self.name())
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
            Self::ElectronicHitFx1,
            Self::ElectronicHitFx2,
            Self::ElectronicHitFx3,
            Self::ElectronicHitFx4,
            Self::ElectronicHitFx5,
            Self::ElectronicHitFx6,
            Self::Bong,
            Self::Click,
            Self::Confirmation,
            Self::Drop1,
            Self::Drop2,
            Self::Drop3,
            Self::Drop4,
            Self::Error1,
            Self::Error2,
            Self::Glass1,
            Self::Glass2,
            Self::Glass3,
            Self::Glass4,
            Self::Maximize1,
            Self::Maximize2,
            Self::Maximize3,
            Self::Maximize4,
            Self::Maximize5,
            Self::Minimize1,
            Self::Minimize2,
            Self::Minimize3,
            Self::MouseClick,
            Self::Pluck1,
            Self::Pluck2,
            Self::Question1,
            Self::Question2,
            Self::Select1,
            Self::Select2,
            Self::Select3,
            Self::Select4,
            Self::Switch1,
            Self::Switch2,
        ]);

        #[cfg(feature = "sfx-cartoon")]
        effects.extend_from_slice(&[
            Self::CartoonAccent1,
            Self::CartoonAccent2,
            Self::CartoonAccent3,
            Self::CartoonAccent4,
            Self::CartoonAccent5,
            Self::CartoonAccent6,
            Self::CartoonAccent7,
            Self::CartoonAccent8,
            Self::CartoonAccent9,
            Self::CartoonAccent10,
            Self::CartoonAccent11,
            Self::CartoonAccent12,
            Self::CartoonCry,
        ]);

        #[cfg(feature = "sfx-reactions")]
        effects.extend_from_slice(&[
            Self::CrowdApplause,
            Self::CrowdApplauseRecital,
            Self::CrowdApplauseStadium,
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
    /// Also handles variations like `electronic-hit-fx-1` vs `electronic-hit-fx1`
    /// by falling back to fuzzy matching when exact matching fails.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use playa::SoundEffect;
    ///
    /// let effect = SoundEffect::from_name("sad-trombone");
    /// assert_eq!(effect, Some(SoundEffect::SadTrombone));
    ///
    /// // Fuzzy matching handles dash variations
    /// let effect = SoundEffect::from_name("electronic-hit-fx-1");
    /// assert_eq!(effect, Some(SoundEffect::ElectronicHitFx1));
    /// ```
    ///
    /// ## Returns
    ///
    /// - `Some(effect)` when the name matches a compiled effect.
    /// - `None` when no match is found.
    pub fn from_name(name: &str) -> Option<Self> {
        // First try exact matching with basic normalization
        let normalized = normalize_effect_name(name);
        let exact_match = match normalized.as_str() {
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
            "electronic-hit-fx1" => Some(Self::ElectronicHitFx1),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx2" => Some(Self::ElectronicHitFx2),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx3" => Some(Self::ElectronicHitFx3),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx4" => Some(Self::ElectronicHitFx4),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx5" => Some(Self::ElectronicHitFx5),
            #[cfg(feature = "sfx-ui")]
            "electronic-hit-fx6" => Some(Self::ElectronicHitFx6),
            #[cfg(feature = "sfx-ui")]
            "bong" => Some(Self::Bong),
            #[cfg(feature = "sfx-ui")]
            "click" => Some(Self::Click),
            #[cfg(feature = "sfx-ui")]
            "confirmation" => Some(Self::Confirmation),
            #[cfg(feature = "sfx-ui")]
            "drop-1" => Some(Self::Drop1),
            #[cfg(feature = "sfx-ui")]
            "drop-2" => Some(Self::Drop2),
            #[cfg(feature = "sfx-ui")]
            "drop-3" => Some(Self::Drop3),
            #[cfg(feature = "sfx-ui")]
            "drop-4" => Some(Self::Drop4),
            #[cfg(feature = "sfx-ui")]
            "error-1" => Some(Self::Error1),
            #[cfg(feature = "sfx-ui")]
            "error-2" => Some(Self::Error2),
            #[cfg(feature = "sfx-ui")]
            "glass-1" => Some(Self::Glass1),
            #[cfg(feature = "sfx-ui")]
            "glass-2" => Some(Self::Glass2),
            #[cfg(feature = "sfx-ui")]
            "glass-3" => Some(Self::Glass3),
            #[cfg(feature = "sfx-ui")]
            "glass-4" => Some(Self::Glass4),
            #[cfg(feature = "sfx-ui")]
            "maximize-1" => Some(Self::Maximize1),
            #[cfg(feature = "sfx-ui")]
            "maximize-2" => Some(Self::Maximize2),
            #[cfg(feature = "sfx-ui")]
            "maximize-3" => Some(Self::Maximize3),
            #[cfg(feature = "sfx-ui")]
            "maximize-4" => Some(Self::Maximize4),
            #[cfg(feature = "sfx-ui")]
            "maximize-5" => Some(Self::Maximize5),
            #[cfg(feature = "sfx-ui")]
            "minimize-1" => Some(Self::Minimize1),
            #[cfg(feature = "sfx-ui")]
            "minimize-2" => Some(Self::Minimize2),
            #[cfg(feature = "sfx-ui")]
            "minimize-3" => Some(Self::Minimize3),
            #[cfg(feature = "sfx-ui")]
            "mouseclick" => Some(Self::MouseClick),
            #[cfg(feature = "sfx-ui")]
            "pluck-1" => Some(Self::Pluck1),
            #[cfg(feature = "sfx-ui")]
            "pluck-2" => Some(Self::Pluck2),
            #[cfg(feature = "sfx-ui")]
            "question-1" => Some(Self::Question1),
            #[cfg(feature = "sfx-ui")]
            "question-2" => Some(Self::Question2),
            #[cfg(feature = "sfx-ui")]
            "select-1" => Some(Self::Select1),
            #[cfg(feature = "sfx-ui")]
            "select-2" => Some(Self::Select2),
            #[cfg(feature = "sfx-ui")]
            "select-3" => Some(Self::Select3),
            #[cfg(feature = "sfx-ui")]
            "select-4" => Some(Self::Select4),
            #[cfg(feature = "sfx-ui")]
            "switch-1" => Some(Self::Switch1),
            #[cfg(feature = "sfx-ui")]
            "switch-2" => Some(Self::Switch2),

            // === Cartoon Sounds (sfx-cartoon) ===
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent1" => Some(Self::CartoonAccent1),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent2" => Some(Self::CartoonAccent2),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent3" => Some(Self::CartoonAccent3),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent4" => Some(Self::CartoonAccent4),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent5" => Some(Self::CartoonAccent5),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent6" => Some(Self::CartoonAccent6),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent7" => Some(Self::CartoonAccent7),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent8" => Some(Self::CartoonAccent8),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent9" => Some(Self::CartoonAccent9),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent10" => Some(Self::CartoonAccent10),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent11" => Some(Self::CartoonAccent11),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-accent12" => Some(Self::CartoonAccent12),
            #[cfg(feature = "sfx-cartoon")]
            "cartoon-cry" => Some(Self::CartoonCry),

            // === Reaction Sounds (sfx-reactions) ===
            #[cfg(feature = "sfx-reactions")]
            "crowd-applause" => Some(Self::CrowdApplause),
            #[cfg(feature = "sfx-reactions")]
            "crowd-applause-recital" => Some(Self::CrowdApplauseRecital),
            #[cfg(feature = "sfx-reactions")]
            "crowd-applause-stadium" => Some(Self::CrowdApplauseStadium),
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
        };

        if exact_match.is_some() {
            return exact_match;
        }

        // Fall back to fuzzy matching (strips all non-alphanumeric chars)
        // This handles variations like "electronic-hit-fx-1" vs "electronic-hit-fx1"
        let fuzzy = normalize_for_matching(name);
        Self::all()
            .into_iter()
            .find(|effect| normalize_for_matching(effect.name()) == fuzzy)
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
            Self::ElectronicHitFx1 => ELECTRONIC_HIT_FX_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx2 => ELECTRONIC_HIT_FX_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx3 => ELECTRONIC_HIT_FX_3_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx4 => ELECTRONIC_HIT_FX_4_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx5 => ELECTRONIC_HIT_FX_5_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx6 => ELECTRONIC_HIT_FX_6_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Bong => BONG_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Click => CLICK_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Confirmation => CONFIRMATION_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Drop1 => DROP_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Drop2 => DROP_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Drop3 => DROP_3_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Drop4 => DROP_4_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Error1 => ERROR_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Error2 => ERROR_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Glass1 => GLASS_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Glass2 => GLASS_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Glass3 => GLASS_3_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Glass4 => GLASS_4_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Maximize1 => MAXIMIZE_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Maximize2 => MAXIMIZE_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Maximize3 => MAXIMIZE_3_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Maximize4 => MAXIMIZE_4_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Maximize5 => MAXIMIZE_5_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Minimize1 => MINIMIZE_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Minimize2 => MINIMIZE_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Minimize3 => MINIMIZE_3_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::MouseClick => MOUSE_CLICK_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Pluck1 => PLUCK_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Pluck2 => PLUCK_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Question1 => QUESTION_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Question2 => QUESTION_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Select1 => SELECT_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Select2 => SELECT_2_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Select3 => SELECT_3_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Select4 => SELECT_4_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Switch1 => SWITCH_1_BYTES,
            #[cfg(feature = "sfx-ui")]
            Self::Switch2 => SWITCH_2_BYTES,

            // === Cartoon Sounds (sfx-cartoon) ===
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent1 => CARTOON_ACCENT_1_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent2 => CARTOON_ACCENT_2_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent3 => CARTOON_ACCENT_3_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent4 => CARTOON_ACCENT_4_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent5 => CARTOON_ACCENT_5_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent6 => CARTOON_ACCENT_6_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent7 => CARTOON_ACCENT_7_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent8 => CARTOON_ACCENT_8_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent9 => CARTOON_ACCENT_9_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent10 => CARTOON_ACCENT_10_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent11 => CARTOON_ACCENT_11_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent12 => CARTOON_ACCENT_12_BYTES,
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonCry => CARTOON_CRY_BYTES,

            // === Reaction Sounds (sfx-reactions) ===
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdApplause => CROWD_APPLAUSE_BYTES,
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdApplauseRecital => CROWD_APPLAUSE_RECITAL_BYTES,
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdApplauseStadium => CROWD_APPLAUSE_STADIUM_BYTES,
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
    /// use `play_async` (requires the `async` feature). Async callers that
    /// need synchronous playback should use `tokio::task::spawn_blocking`.
    ///
    /// When the `sfx-native` feature is enabled, plays directly through the OS
    /// audio subsystem via rodio. On macOS with `sfx-native-audio`, routes to
    /// the system sound device (configurable in System Settings → Sound).
    /// Falls back to host player delegation if native playback fails.
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
    /// // With sfx-reactions feature enabled:
    /// SoundEffect::SadTrombone.play()?;
    /// # Ok::<(), playa::PlaybackError>(())
    /// ```
    pub fn play(self) -> Result<(), crate::PlaybackError> {
        self.play_with_options(&crate::PlaybackOptions::default())
    }

    /// Play this sound effect with specific playback options.
    ///
    /// Supports volume (0.0 = silent, 1.0 = normal, >1.0 = amplified) and
    /// speed (1.0 = normal, <1.0 = slower, >1.0 = faster) control.
    ///
    /// When the `sfx-native` feature is enabled, options are applied directly
    /// via rodio's Player. Otherwise, control depends on the host player's
    /// capabilities. If a native device-open operation times out, playa
    /// disables further native playback attempts for the rest of the process
    /// and this method falls back directly to host playback on later calls.
    ///
    /// This method blocks until playback completes or times out. On Linux with
    /// `sfx-native-audio`, PulseAudio wait loops are bounded by deadlines.
    /// Async callers should use `tokio::task::spawn_blocking` to avoid
    /// blocking the async runtime.
    pub fn play_with_options(
        self,
        options: &crate::PlaybackOptions,
    ) -> Result<(), crate::PlaybackError> {
        // Try native playback first when available.
        #[cfg(feature = "sfx-native")]
        {
            match crate::sfx_player::play_sfx(self.bytes(), options) {
                Ok(()) => return Ok(()),
                Err(error) if !error.should_fallback_to_host() => {
                    return Err(crate::PlaybackError::AudioSubsystem {
                        detail: error.to_string(),
                    });
                }
                Err(_) => {}
            }
        }

        let playa = crate::Playa::from_bytes(self.as_bytes().to_vec())
            .map_err(|crate::InvalidAudio::Detection(e)| crate::PlaybackError::Detection(e))?
            .with_options(options.clone());
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

    /// Returns the category name for this sound effect.
    ///
    /// Categories correspond to the feature flags used to enable groups
    /// of effects (e.g., "UI", "Cartoon", "Reactions").
    pub fn category(&self) -> &'static str {
        match self {
            #[cfg(feature = "sfx-ui")]
            Self::Doorbell
            | Self::Doorbell2
            | Self::SpaceAlarm
            | Self::DitHit1
            | Self::DitHit2
            | Self::ElectronicHitFx1
            | Self::ElectronicHitFx2
            | Self::ElectronicHitFx3
            | Self::ElectronicHitFx4
            | Self::ElectronicHitFx5
            | Self::ElectronicHitFx6
            | Self::Bong
            | Self::Click
            | Self::Confirmation
            | Self::Drop1
            | Self::Drop2
            | Self::Drop3
            | Self::Drop4
            | Self::Error1
            | Self::Error2
            | Self::Glass1
            | Self::Glass2
            | Self::Glass3
            | Self::Glass4
            | Self::Maximize1
            | Self::Maximize2
            | Self::Maximize3
            | Self::Maximize4
            | Self::Maximize5
            | Self::Minimize1
            | Self::Minimize2
            | Self::Minimize3
            | Self::MouseClick
            | Self::Pluck1
            | Self::Pluck2
            | Self::Question1
            | Self::Question2
            | Self::Select1
            | Self::Select2
            | Self::Select3
            | Self::Select4
            | Self::Switch1
            | Self::Switch2 => "UI",

            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent1
            | Self::CartoonAccent2
            | Self::CartoonAccent3
            | Self::CartoonAccent4
            | Self::CartoonAccent5
            | Self::CartoonAccent6
            | Self::CartoonAccent7
            | Self::CartoonAccent8
            | Self::CartoonAccent9
            | Self::CartoonAccent10
            | Self::CartoonAccent11
            | Self::CartoonAccent12
            | Self::CartoonCry => "Cartoon",

            #[cfg(feature = "sfx-reactions")]
            Self::CrowdApplause
            | Self::CrowdApplauseRecital
            | Self::CrowdApplauseStadium
            | Self::CrowdLaugh
            | Self::CrowdLaughApplause
            | Self::SadTrombone
            | Self::SmallGroupCheer
            | Self::FemaleAstonishedGasp
            | Self::Sneeze => "Reactions",

            #[cfg(feature = "sfx-scifi")]
            Self::HighDown
            | Self::HighUp
            | Self::TwoTone
            | Self::PhaseJump1
            | Self::PhaseJump2
            | Self::PhaseJump3
            | Self::PhaseJump4
            | Self::PhaseJump5
            | Self::PhaserDown1
            | Self::PhaserDown2
            | Self::PhaserDown3 => "Sci-Fi",

            #[cfg(feature = "sfx-atmosphere")]
            Self::CreepyDarkLogo
            | Self::ElementalMagicSpellImpact
            | Self::EpicOrchestraTransition
            | Self::MysteriousBass
            | Self::RetroGame => "Atmosphere",

            #[cfg(feature = "sfx-motion")]
            Self::AirReverseBurst
            | Self::AirWoosh
            | Self::AirZoomVacuum
            | Self::ArrowWhoosh
            | Self::BicycleHorn
            | Self::BottleCork
            | Self::Bullet => "Motion",
        }
    }

    /// Returns a short description for this sound effect.
    pub fn description(&self) -> &'static str {
        match self {
            // UI
            #[cfg(feature = "sfx-ui")]
            Self::Doorbell => "classic doorbell",
            #[cfg(feature = "sfx-ui")]
            Self::Doorbell2 => "alternative doorbell",
            #[cfg(feature = "sfx-ui")]
            Self::SpaceAlarm => "space station alarm",
            #[cfg(feature = "sfx-ui")]
            Self::DitHit1 => "short digital hit",
            #[cfg(feature = "sfx-ui")]
            Self::DitHit2 => "short digital hit variant",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx1 => "electronic hit",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx2 => "electronic hit",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx3 => "electronic hit",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx4 => "electronic hit",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx5 => "electronic hit",
            #[cfg(feature = "sfx-ui")]
            Self::ElectronicHitFx6 => "electronic hit",
            #[cfg(feature = "sfx-ui")]
            Self::Bong => "bong notification",
            #[cfg(feature = "sfx-ui")]
            Self::Click => "click sound",
            #[cfg(feature = "sfx-ui")]
            Self::Confirmation => "confirmation sound",
            #[cfg(feature = "sfx-ui")]
            Self::Drop1 => "drop sound",
            #[cfg(feature = "sfx-ui")]
            Self::Drop2 => "drop sound",
            #[cfg(feature = "sfx-ui")]
            Self::Drop3 => "drop sound",
            #[cfg(feature = "sfx-ui")]
            Self::Drop4 => "drop sound",
            #[cfg(feature = "sfx-ui")]
            Self::Error1 => "error sound",
            #[cfg(feature = "sfx-ui")]
            Self::Error2 => "error sound",
            #[cfg(feature = "sfx-ui")]
            Self::Glass1 => "glass chime",
            #[cfg(feature = "sfx-ui")]
            Self::Glass2 => "glass chime",
            #[cfg(feature = "sfx-ui")]
            Self::Glass3 => "glass chime",
            #[cfg(feature = "sfx-ui")]
            Self::Glass4 => "glass chime",
            #[cfg(feature = "sfx-ui")]
            Self::Maximize1 => "window maximize",
            #[cfg(feature = "sfx-ui")]
            Self::Maximize2 => "window maximize",
            #[cfg(feature = "sfx-ui")]
            Self::Maximize3 => "window maximize",
            #[cfg(feature = "sfx-ui")]
            Self::Maximize4 => "window maximize",
            #[cfg(feature = "sfx-ui")]
            Self::Maximize5 => "window maximize",
            #[cfg(feature = "sfx-ui")]
            Self::Minimize1 => "window minimize",
            #[cfg(feature = "sfx-ui")]
            Self::Minimize2 => "window minimize",
            #[cfg(feature = "sfx-ui")]
            Self::Minimize3 => "window minimize",
            #[cfg(feature = "sfx-ui")]
            Self::MouseClick => "mouse click",
            #[cfg(feature = "sfx-ui")]
            Self::Pluck1 => "pluck sound",
            #[cfg(feature = "sfx-ui")]
            Self::Pluck2 => "pluck sound",
            #[cfg(feature = "sfx-ui")]
            Self::Question1 => "question prompt",
            #[cfg(feature = "sfx-ui")]
            Self::Question2 => "question prompt",
            #[cfg(feature = "sfx-ui")]
            Self::Select1 => "selection sound",
            #[cfg(feature = "sfx-ui")]
            Self::Select2 => "selection sound",
            #[cfg(feature = "sfx-ui")]
            Self::Select3 => "selection sound",
            #[cfg(feature = "sfx-ui")]
            Self::Select4 => "selection sound",
            #[cfg(feature = "sfx-ui")]
            Self::Switch1 => "switch toggle",
            #[cfg(feature = "sfx-ui")]
            Self::Switch2 => "switch toggle",

            // Cartoon
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent1 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent2 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent3 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent4 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent5 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent6 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent7 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent8 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent9 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent10 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent11 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonAccent12 => "cartoon accent",
            #[cfg(feature = "sfx-cartoon")]
            Self::CartoonCry => "exaggerated crying",

            // Reactions
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdApplause => "crowd applause",
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdApplauseRecital => "crowd applause (recital)",
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdApplauseStadium => "crowd applause (stadium)",
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdLaugh => "crowd laughter",
            #[cfg(feature = "sfx-reactions")]
            Self::CrowdLaughApplause => "crowd laughter with applause",
            #[cfg(feature = "sfx-reactions")]
            Self::SadTrombone => "classic wah-wah",
            #[cfg(feature = "sfx-reactions")]
            Self::SmallGroupCheer => "small group cheering",
            #[cfg(feature = "sfx-reactions")]
            Self::FemaleAstonishedGasp => "astonished gasp",
            #[cfg(feature = "sfx-reactions")]
            Self::Sneeze => "sneeze",

            // Sci-Fi
            #[cfg(feature = "sfx-scifi")]
            Self::HighDown => "descending tone",
            #[cfg(feature = "sfx-scifi")]
            Self::HighUp => "ascending tone",
            #[cfg(feature = "sfx-scifi")]
            Self::TwoTone => "two-tone alert",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump1 => "phase jump",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump2 => "phase jump",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump3 => "phase jump",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump4 => "phase jump",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaseJump5 => "phase jump",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaserDown1 => "phaser descending",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaserDown2 => "phaser descending",
            #[cfg(feature = "sfx-scifi")]
            Self::PhaserDown3 => "phaser descending",

            // Atmosphere
            #[cfg(feature = "sfx-atmosphere")]
            Self::CreepyDarkLogo => "creepy logo sting",
            #[cfg(feature = "sfx-atmosphere")]
            Self::ElementalMagicSpellImpact => "magic spell impact",
            #[cfg(feature = "sfx-atmosphere")]
            Self::EpicOrchestraTransition => "orchestra transition",
            #[cfg(feature = "sfx-atmosphere")]
            Self::MysteriousBass => "mysterious bass pulse",
            #[cfg(feature = "sfx-atmosphere")]
            Self::RetroGame => "retro game sound",

            // Motion
            #[cfg(feature = "sfx-motion")]
            Self::AirReverseBurst => "air reverse burst",
            #[cfg(feature = "sfx-motion")]
            Self::AirWoosh => "air woosh",
            #[cfg(feature = "sfx-motion")]
            Self::AirZoomVacuum => "air zoom vacuum",
            #[cfg(feature = "sfx-motion")]
            Self::ArrowWhoosh => "arrow whoosh",
            #[cfg(feature = "sfx-motion")]
            Self::BicycleHorn => "bicycle horn honk",
            #[cfg(feature = "sfx-motion")]
            Self::BottleCork => "bottle cork pop",
            #[cfg(feature = "sfx-motion")]
            Self::Bullet => "bullet sound",
        }
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

    /// Return the kebab-case names of all compiled-in sound effects, sorted alphabetically.
    ///
    /// The list reflects only the feature flags enabled at compile time.
    /// With all features (`sound-effects`) this returns 88 names.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// for name in playa::SoundEffect::all_names() {
    ///     println!("{name}");
    /// }
    /// ```
    pub fn all_names() -> &'static [&'static str] {
        effect_durations::EFFECT_NAMES
    }
}

/// Normalize an effect name by lowercasing and converting separators.
///
/// This keeps the structure (dashes) for exact matching in `from_name`.
fn normalize_effect_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

/// Normalize an effect name by stripping all non-alphanumeric characters.
///
/// This is used for fuzzy matching to handle variations like
/// `electronic-hit-fx-1` vs `electronic-hit-fx1`.
fn normalize_for_matching(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}
