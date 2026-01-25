use std::collections::HashMap;
use std::sync::LazyLock;

/// Common executable names you’d expect to find in PATH for headless/CLI audio playback tools.
pub static HEADLESS_AUDIO_COMMANDS: LazyLock<HashMap<&'static str, Vec<&'static str>>> = LazyLock::new(|| {
    HashMap::from([
        // mpv
        ("mpv", vec!["mpv"]),
        // FFmpeg
        ("ffplay", vec!["ffplay"]),
        // VLC
        ("vlc", vec!["vlc", "cvlc"]),
        // MPlayer
        ("mplayer", vec!["mplayer"]),
        // GStreamer playback tool (packaging commonly uses gst-play-1.0; some distros also provide gst-play)
        ("gstreamer_gst_play", vec!["gst-play-1.0", "gst-play"]),
        // SoX (invoking as `play` sets output to default sound device; `sox` is the base command)
        ("sox", vec!["play", "sox"]),
        // mpg123
        ("mpg123", vec!["mpg123"]),
        // vorbis-tools (ogg123)
        ("ogg123", vec!["ogg123"]),
        // ALSA utils
        ("alsa_aplay", vec!["aplay"]),
        // PulseAudio utils
        ("pulseaudio_paplay", vec!["paplay", "pacat"]),
        // PipeWire tools
        ("pipewire", vec!["pw-cat", "pw-play"]),
    ])
});
