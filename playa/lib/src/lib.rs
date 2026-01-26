mod audio;
mod detection;
mod error;
mod playback;
mod player;
mod types;

pub use crate::audio::{Audio, AudioData, AudioSourceKind};
pub use crate::detection::{
    detect_audio_format_from_bytes, detect_audio_format_from_path, detect_audio_format_from_url,
};
pub use crate::error::{DetectionError, InvalidAudio, PlaybackError};
pub use crate::playback::{playa, playa_explicit, playa_with_player};
pub use crate::player::{
    all_players, match_available_players, match_players, AudioPlayer, Player, PLAYER_LOOKUP,
};
pub use crate::types::{AudioFileFormat, AudioFormat, Codec, ResourceUsage};
