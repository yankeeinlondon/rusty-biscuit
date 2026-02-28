mod audio;
mod detection;
mod error;
mod playa;
mod playback;
mod player;
mod types;

#[cfg(feature = "audio-ducking")]
pub mod ducking;

#[cfg(feature = "sfx-native")]
pub mod sfx_player;

#[cfg(any(
    feature = "sound-effects",
    feature = "sfx-ui",
    feature = "sfx-cartoon",
    feature = "sfx-reactions",
    feature = "sfx-scifi",
    feature = "sfx-atmosphere",
    feature = "sfx-motion",
))]
mod effects;

#[cfg(any(
    feature = "sound-effects",
    feature = "sfx-ui",
    feature = "sfx-cartoon",
    feature = "sfx-reactions",
    feature = "sfx-scifi",
    feature = "sfx-atmosphere",
    feature = "sfx-motion",
))]
pub use crate::effects::SoundEffect;

pub use crate::audio::{Audio, AudioData, AudioSourceKind};
pub use crate::detection::{
    detect_audio_format_from_bytes, detect_audio_format_from_path, detect_audio_format_from_url,
};
pub use crate::error::{DetectionError, InvalidAudio, PlaybackError};
pub use crate::playa::Playa;
pub use crate::playback::{
    playa, playa_explicit, playa_explicit_with_options, playa_with_player,
    playa_with_player_and_options,
};

#[cfg(feature = "async")]
pub use crate::playback::{
    playa_async, playa_explicit_async, playa_explicit_with_options_async,
    playa_with_player_and_options_async, playa_with_player_async,
};
pub use crate::player::{
    AudioPlayer, PLAYER_LOOKUP, Player, all_players, match_available_players, match_players,
};
pub use crate::types::{AudioFileFormat, AudioFormat, Codec, PlaybackOptions, ResourceUsage};
