mod audio;
mod channels;
mod detection;
mod error;
#[cfg(feature = "sfx-native")]
mod native_audio;
mod playa;
mod playback;
mod player;
mod types;

#[cfg(all(
    target_os = "windows",
    any(feature = "sfx-native-audio", feature = "audio-ducking-windows")
))]
mod windows_com;

#[cfg(feature = "audio-ducking")]
pub mod ducking;

#[cfg(feature = "sfx-native")]
pub mod sfx_player;

#[cfg(feature = "native-playback")]
pub mod native_player;

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
pub use crate::channels::{OutputChannel, SampleRateRange};

#[cfg(feature = "sfx-native")]
pub use crate::channels::get_output_channels;

/// Returns the name of the default sound-effects output device, when it can
/// differ from the default audio output device.
///
/// On macOS (with `sfx-native-audio`), this performs a cheap CoreAudio
/// property lookup — no cpal enumeration. On other platforms, returns
/// `None`, which callers should treat as "same as the default audio
/// output device".
///
/// Use this when you need to highlight the default sound-effects device
/// without paying the cost of full output-channel enumeration.
#[cfg(feature = "sfx-native")]
pub fn get_default_sfx_device_name() -> Option<String> {
    #[cfg(all(target_os = "macos", feature = "sfx-native-audio"))]
    {
        crate::sfx_player::macos::get_system_sound_device_name()
    }
    #[cfg(not(all(target_os = "macos", feature = "sfx-native-audio")))]
    {
        None
    }
}

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
