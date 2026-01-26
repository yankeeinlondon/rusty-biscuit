mod audio;
mod detection;
mod error;
mod playa;
mod playback;
mod player;
mod types;

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
pub use crate::player::{
    all_players, match_available_players, match_players, AudioPlayer, Player, PLAYER_LOOKUP,
};
pub use crate::types::{AudioFileFormat, AudioFormat, Codec, PlaybackOptions, ResourceUsage};
