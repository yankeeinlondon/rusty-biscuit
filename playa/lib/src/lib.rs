mod audio;
mod detection;
mod error;
mod playa;
mod playback;
mod player;
mod types;

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
    playa_async, playa_explicit_async, playa_explicit_with_options_async, playa_with_player_async,
    playa_with_player_and_options_async,
};
pub use crate::player::{
    all_players, match_available_players, match_players, AudioPlayer, Player, PLAYER_LOOKUP,
};
pub use crate::types::{AudioFileFormat, AudioFormat, Codec, PlaybackOptions, ResourceUsage};
