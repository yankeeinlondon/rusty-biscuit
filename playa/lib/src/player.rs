use std::collections::HashMap;
use std::sync::LazyLock;

use sniff_lib::programs::{HeadlessAudio, InstalledHeadlessAudio, ProgramMetadata};

use crate::types::{AudioFileFormat, AudioFormat, Codec, ResourceUsage};

/// Identifier for a supported audio player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioPlayer {
    /// mpv media player.
    Mpv,
    /// FFplay (FFmpeg) player.
    FfPlay,
    /// VLC (cvlc) player.
    Vlc,
    /// MPlayer.
    MPlayer,
    /// GStreamer gst-play.
    GstreamerGstPlay,
    /// SoX play.
    Sox,
    /// mpg123 MP3 player.
    Mpg123,
    /// ogg123 player.
    Ogg123,
    /// ALSA aplay.
    AlsaAplay,
    /// PulseAudio paplay.
    PulseaudioPaplay,
    /// PipeWire pw-play.
    Pipewire,
}

impl AudioPlayer {
    /// Map this player to the Sniff headless audio enum.
    pub const fn as_headless_audio(self) -> HeadlessAudio {
        match self {
            AudioPlayer::Mpv => HeadlessAudio::Mpv,
            AudioPlayer::FfPlay => HeadlessAudio::Ffplay,
            AudioPlayer::Vlc => HeadlessAudio::Vlc,
            AudioPlayer::MPlayer => HeadlessAudio::MPlayer,
            AudioPlayer::GstreamerGstPlay => HeadlessAudio::GstreamerGstPlay,
            AudioPlayer::Sox => HeadlessAudio::Sox,
            AudioPlayer::Mpg123 => HeadlessAudio::Mpg123,
            AudioPlayer::Ogg123 => HeadlessAudio::Ogg123,
            AudioPlayer::AlsaAplay => HeadlessAudio::AlsaAplay,
            AudioPlayer::PulseaudioPaplay => HeadlessAudio::PulseaudioPaplay,
            AudioPlayer::Pipewire => HeadlessAudio::Pipewire,
        }
    }
}

/// Extended metadata for audio players beyond what Sniff provides.
#[derive(Debug, Clone, Copy)]
pub struct Player {
    /// The player identifier.
    pub id: AudioPlayer,
    /// Reference to Sniff's program enum for detection.
    pub sniff_program: HeadlessAudio,
    /// Codecs this player can decode.
    pub supported_codecs: &'static [Codec],
    /// File containers this player can read.
    pub supported_formats: &'static [AudioFileFormat],
    /// Can accept audio from stdin or URLs.
    pub takes_stream_input: bool,
    /// Can output audio over network (e.g., Icecast).
    pub supplies_stream_output: bool,
    /// Whether the player is open source.
    pub is_open_source: bool,
    /// CPU/memory usage classification.
    pub resource_usage: ResourceUsage,
    /// Supports playback speed/tempo control via CLI.
    pub supports_speed_control: bool,
    /// Supports volume control via CLI.
    pub supports_volume_control: bool,
}

impl Player {
    /// Return the binary name from Sniff metadata.
    pub fn binary_name(&self) -> &'static str {
        self.sniff_program.binary_name()
    }

    /// Return the display name from Sniff metadata.
    pub fn display_name(&self) -> &'static str {
        self.sniff_program.display_name()
    }

    /// Return the website URL from Sniff metadata.
    pub fn website(&self) -> &'static str {
        self.sniff_program.website()
    }

    /// Return the description from Sniff metadata.
    pub fn description(&self) -> &'static str {
        self.sniff_program.description()
    }

    /// Check if the player can decode the given codec.
    pub fn supports_codec(&self, codec: Codec) -> bool {
        self.supported_codecs.contains(&codec)
    }

    /// Check if the player supports the given file format.
    pub fn supports_format(&self, format: AudioFileFormat) -> bool {
        self.supported_formats.contains(&format)
    }

    /// Check if the player supports playback speed control.
    pub fn supports_speed(&self) -> bool {
        self.supports_speed_control
    }

    /// Check if the player supports volume control.
    pub fn supports_volume(&self) -> bool {
        self.supports_volume_control
    }

    /// Calculate capability score for ranking.
    ///
    /// Higher scores indicate more capable players. Scoring:
    /// - +4 for speed control (most valuable for TTS use cases)
    /// - +3 for volume control (commonly needed)
    /// - +2 for stream input (enables piping without disk I/O)
    pub fn capability_score(&self) -> u8 {
        let mut score: u8 = 0;
        if self.supports_speed_control {
            score += 4;
        }
        if self.supports_volume_control {
            score += 3;
        }
        if self.takes_stream_input {
            score += 2;
        }
        score
    }
}

const ALL_PLAYERS: [AudioPlayer; 11] = [
    AudioPlayer::Mpv,
    AudioPlayer::FfPlay,
    AudioPlayer::Vlc,
    AudioPlayer::MPlayer,
    AudioPlayer::GstreamerGstPlay,
    AudioPlayer::Sox,
    AudioPlayer::Mpg123,
    AudioPlayer::Ogg123,
    AudioPlayer::AlsaAplay,
    AudioPlayer::PulseaudioPaplay,
    AudioPlayer::Pipewire,
];

/// Return all known audio players in canonical order.
pub fn all_players() -> &'static [AudioPlayer] {
    &ALL_PLAYERS
}

static FFMPEG_CODECS: &[Codec] = &[
    Codec::Pcm,
    Codec::Flac,
    Codec::Alac,
    Codec::Mp3,
    Codec::Aac,
    Codec::Vorbis,
    Codec::Opus,
];

static FFMPEG_FORMATS: &[AudioFileFormat] = &[
    AudioFileFormat::Wav,
    AudioFileFormat::Aiff,
    AudioFileFormat::Flac,
    AudioFileFormat::Mp3,
    AudioFileFormat::Ogg,
    AudioFileFormat::M4a,
    AudioFileFormat::Webm,
];

static SOX_CODECS: &[Codec] = &[Codec::Pcm, Codec::Flac, Codec::Mp3, Codec::Vorbis];

static SOX_FORMATS: &[AudioFileFormat] = &[
    AudioFileFormat::Wav,
    AudioFileFormat::Flac,
    AudioFileFormat::Ogg,
    AudioFileFormat::Mp3,
];

static OGG123_CODECS: &[Codec] = &[Codec::Vorbis, Codec::Opus, Codec::Flac];

static OGG123_FORMATS: &[AudioFileFormat] = &[AudioFileFormat::Ogg];

static MPG123_CODECS: &[Codec] = &[Codec::Mp3];

static MPG123_FORMATS: &[AudioFileFormat] = &[AudioFileFormat::Mp3];

static APLAY_CODECS: &[Codec] = &[Codec::Pcm];

static APLAY_FORMATS: &[AudioFileFormat] = &[AudioFileFormat::Wav];

static PULSE_CODECS: &[Codec] = &[Codec::Pcm];

static PULSE_FORMATS: &[AudioFileFormat] = &[AudioFileFormat::Wav];

static PIPEWIRE_CODECS: &[Codec] = &[Codec::Pcm, Codec::Flac];

static PIPEWIRE_FORMATS: &[AudioFileFormat] = &[AudioFileFormat::Wav, AudioFileFormat::Flac];

/// Static lookup table for all supported players.
pub static PLAYER_LOOKUP: LazyLock<HashMap<AudioPlayer, Player>> = LazyLock::new(|| {
    let mut map = HashMap::with_capacity(11);

    // Tier 1: Full controllability (speed + volume + stream)
    map.insert(
        AudioPlayer::Mpv,
        Player {
            id: AudioPlayer::Mpv,
            sniff_program: HeadlessAudio::Mpv,
            supported_codecs: FFMPEG_CODECS,
            supported_formats: FFMPEG_FORMATS,
            takes_stream_input: true,
            supplies_stream_output: false,
            is_open_source: true,
            resource_usage: ResourceUsage::Medium,
            supports_speed_control: true,  // --speed=N
            supports_volume_control: true, // --volume=N
        },
    );

    map.insert(
        AudioPlayer::FfPlay,
        Player {
            id: AudioPlayer::FfPlay,
            sniff_program: HeadlessAudio::Ffplay,
            supported_codecs: FFMPEG_CODECS,
            supported_formats: FFMPEG_FORMATS,
            takes_stream_input: true,
            supplies_stream_output: false,
            is_open_source: true,
            resource_usage: ResourceUsage::Medium,
            supports_speed_control: true,  // -af atempo=N
            supports_volume_control: true, // -volume N
        },
    );

    // Tier 2: Volume + stream (no speed control)
    map.insert(
        AudioPlayer::Vlc,
        Player {
            id: AudioPlayer::Vlc,
            sniff_program: HeadlessAudio::Vlc,
            supported_codecs: FFMPEG_CODECS,
            supported_formats: FFMPEG_FORMATS,
            takes_stream_input: true,
            supplies_stream_output: true,
            is_open_source: true,
            resource_usage: ResourceUsage::Medium,
            supports_speed_control: false,
            supports_volume_control: true, // --gain=N
        },
    );

    map.insert(
        AudioPlayer::MPlayer,
        Player {
            id: AudioPlayer::MPlayer,
            sniff_program: HeadlessAudio::MPlayer,
            supported_codecs: FFMPEG_CODECS,
            supported_formats: FFMPEG_FORMATS,
            takes_stream_input: true,
            supplies_stream_output: false,
            is_open_source: true,
            resource_usage: ResourceUsage::Medium,
            supports_speed_control: false,
            supports_volume_control: true, // -softvol -volume N
        },
    );

    map.insert(
        AudioPlayer::GstreamerGstPlay,
        Player {
            id: AudioPlayer::GstreamerGstPlay,
            sniff_program: HeadlessAudio::GstreamerGstPlay,
            supported_codecs: FFMPEG_CODECS,
            supported_formats: FFMPEG_FORMATS,
            takes_stream_input: true,
            supplies_stream_output: true,
            is_open_source: true,
            resource_usage: ResourceUsage::Medium,
            supports_speed_control: false,
            supports_volume_control: true, // --volume=N
        },
    );

    map.insert(
        AudioPlayer::Sox,
        Player {
            id: AudioPlayer::Sox,
            sniff_program: HeadlessAudio::Sox,
            supported_codecs: SOX_CODECS,
            supported_formats: SOX_FORMATS,
            takes_stream_input: true,
            supplies_stream_output: false,
            is_open_source: true,
            resource_usage: ResourceUsage::Low,
            supports_speed_control: true, // speed effect
            supports_volume_control: true, // -v N
        },
    );

    // Tier 3: Stream only (no volume/speed control)
    map.insert(
        AudioPlayer::Mpg123,
        Player {
            id: AudioPlayer::Mpg123,
            sniff_program: HeadlessAudio::Mpg123,
            supported_codecs: MPG123_CODECS,
            supported_formats: MPG123_FORMATS,
            takes_stream_input: true,
            supplies_stream_output: false,
            is_open_source: true,
            resource_usage: ResourceUsage::Low,
            supports_speed_control: false,
            supports_volume_control: false,
        },
    );

    map.insert(
        AudioPlayer::Ogg123,
        Player {
            id: AudioPlayer::Ogg123,
            sniff_program: HeadlessAudio::Ogg123,
            supported_codecs: OGG123_CODECS,
            supported_formats: OGG123_FORMATS,
            takes_stream_input: false, // ogg123 does NOT support stdin
            supplies_stream_output: false,
            is_open_source: true,
            resource_usage: ResourceUsage::Low,
            supports_speed_control: false,
            supports_volume_control: false,
        },
    );

    // Tier 4: No controllability
    map.insert(
        AudioPlayer::AlsaAplay,
        Player {
            id: AudioPlayer::AlsaAplay,
            sniff_program: HeadlessAudio::AlsaAplay,
            supported_codecs: APLAY_CODECS,
            supported_formats: APLAY_FORMATS,
            takes_stream_input: false,
            supplies_stream_output: false,
            is_open_source: true,
            resource_usage: ResourceUsage::Low,
            supports_speed_control: false,
            supports_volume_control: false,
        },
    );

    // Tier 3: Volume only (Linux audio subsystems)
    map.insert(
        AudioPlayer::PulseaudioPaplay,
        Player {
            id: AudioPlayer::PulseaudioPaplay,
            sniff_program: HeadlessAudio::PulseaudioPaplay,
            supported_codecs: PULSE_CODECS,
            supported_formats: PULSE_FORMATS,
            takes_stream_input: false,
            supplies_stream_output: false,
            is_open_source: true,
            resource_usage: ResourceUsage::Low,
            supports_speed_control: false,
            supports_volume_control: true, // --volume=N
        },
    );

    map.insert(
        AudioPlayer::Pipewire,
        Player {
            id: AudioPlayer::Pipewire,
            sniff_program: HeadlessAudio::Pipewire,
            supported_codecs: PIPEWIRE_CODECS,
            supported_formats: PIPEWIRE_FORMATS,
            takes_stream_input: false,
            supplies_stream_output: false,
            is_open_source: true,
            resource_usage: ResourceUsage::Low,
            supports_speed_control: false,
            supports_volume_control: true, // --volume=N
        },
    );

    map
});

/// Return players capable of handling the given format, ordered by capability.
pub fn match_players(format: AudioFormat) -> Vec<AudioPlayer> {
    let mut candidates: Vec<AudioPlayer> = ALL_PLAYERS
        .iter()
        .copied()
        .filter(|player| player_supports_format(*player, format))
        .collect();

    candidates.sort_by(|left, right| compare_players(*left, *right));
    candidates
}

/// Return installed players capable of handling the given format, ordered by capability.
pub fn match_available_players(format: AudioFormat) -> Vec<AudioPlayer> {
    let installed = InstalledHeadlessAudio::new();
    match_players(format)
        .into_iter()
        .filter(|player| installed.is_installed(player.as_headless_audio()))
        .collect()
}

fn player_supports_format(player: AudioPlayer, format: AudioFormat) -> bool {
    let Some(metadata) = PLAYER_LOOKUP.get(&player) else {
        return false;
    };
    if !metadata.supports_format(format.file_format) {
        return false;
    }
    match format.codec {
        Some(codec) => metadata.supports_codec(codec),
        None => true,
    }
}

fn compare_players(left: AudioPlayer, right: AudioPlayer) -> std::cmp::Ordering {
    let left_score = player_score(left);
    let right_score = player_score(right);
    right_score
        .cmp(&left_score)
        .then_with(|| player_index(left).cmp(&player_index(right)))
}

/// Score a player based on capabilities.
///
/// Higher scores = more capable players (prioritize controllability).
/// Returns (capability_score, format_count, codec_count) for comparison.
fn player_score(player: AudioPlayer) -> (u8, usize, usize) {
    let Some(metadata) = PLAYER_LOOKUP.get(&player) else {
        return (0, 0, 0);
    };

    let capability_score = metadata.capability_score();
    let format_count = metadata.supported_formats.len();
    let codec_count = metadata.supported_codecs.len();

    (capability_score, format_count, codec_count)
}

fn player_index(player: AudioPlayer) -> usize {
    ALL_PLAYERS
        .iter()
        .position(|candidate| *candidate == player)
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_players_mp3_orders_capable_players() {
        let format = AudioFormat::new(AudioFileFormat::Mp3, Some(Codec::Mp3));
        let players = match_players(format);

        assert!(players.contains(&AudioPlayer::Mpv));
        assert!(players.contains(&AudioPlayer::FfPlay));
        assert!(players.contains(&AudioPlayer::Vlc));
        assert!(players.contains(&AudioPlayer::MPlayer));
        assert!(players.contains(&AudioPlayer::GstreamerGstPlay));
        assert!(players.contains(&AudioPlayer::Sox));
        assert!(players.contains(&AudioPlayer::Mpg123));

        // Tier 1 players (speed+volume+stream) should be first
        let tier1_players = [AudioPlayer::Mpv, AudioPlayer::FfPlay, AudioPlayer::Sox];
        for tier1 in &tier1_players {
            assert!(
                players.iter().position(|p| p == tier1).unwrap()
                    < players.iter().position(|p| *p == AudioPlayer::Mpg123).unwrap(),
                "Tier 1 player {:?} should rank before mpg123",
                tier1
            );
        }
    }

    #[test]
    fn match_players_wav_includes_basic_players() {
        let format = AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm));
        let players = match_players(format);
        assert!(players.contains(&AudioPlayer::AlsaAplay));
        assert!(players.contains(&AudioPlayer::PulseaudioPaplay));
        assert!(players.contains(&AudioPlayer::Pipewire));
    }

    #[test]
    fn player_score_prioritizes_controllability() {
        // Tier 1 players should score highest (9 = speed+volume+stream)
        let mpv_score = player_score(AudioPlayer::Mpv);
        let ffplay_score = player_score(AudioPlayer::FfPlay);
        let sox_score = player_score(AudioPlayer::Sox);

        assert_eq!(mpv_score.0, 9, "mpv should have capability score 9");
        assert_eq!(ffplay_score.0, 9, "ffplay should have capability score 9");
        assert_eq!(sox_score.0, 9, "sox should have capability score 9");

        // Tier 2 players have volume + stream = 5
        let vlc_score = player_score(AudioPlayer::Vlc);
        let mplayer_score = player_score(AudioPlayer::MPlayer);
        let gstreamer_score = player_score(AudioPlayer::GstreamerGstPlay);

        assert_eq!(vlc_score.0, 5, "vlc should have capability score 5");
        assert_eq!(mplayer_score.0, 5, "mplayer should have capability score 5");
        assert_eq!(gstreamer_score.0, 5, "gstreamer should have capability score 5");

        // Tier 3 players with volume only = 3
        let paplay_score = player_score(AudioPlayer::PulseaudioPaplay);
        let pipewire_score = player_score(AudioPlayer::Pipewire);

        assert_eq!(paplay_score.0, 3, "paplay should have capability score 3");
        assert_eq!(pipewire_score.0, 3, "pipewire should have capability score 3");

        // Tier 3 players with stream only = 2
        let mpg123_score = player_score(AudioPlayer::Mpg123);

        assert_eq!(mpg123_score.0, 2, "mpg123 should have capability score 2");

        // ogg123 has no stream input (despite plan initially saying it does)
        let ogg123_score = player_score(AudioPlayer::Ogg123);
        assert_eq!(ogg123_score.0, 0, "ogg123 should have capability score 0");

        // Tier 4: aplay has nothing = 0
        let aplay_score = player_score(AudioPlayer::AlsaAplay);
        assert_eq!(aplay_score.0, 0, "aplay should have capability score 0");
    }

    #[test]
    fn match_players_prefers_controllable_players() {
        let format = AudioFormat::new(AudioFileFormat::Wav, Some(Codec::Pcm));
        let players = match_players(format);

        // Players with speed+volume should come before those without
        let mpv_pos = players.iter().position(|p| *p == AudioPlayer::Mpv);
        let aplay_pos = players.iter().position(|p| *p == AudioPlayer::AlsaAplay);

        assert!(
            mpv_pos.unwrap() < aplay_pos.unwrap(),
            "mpv should rank before aplay"
        );

        // Tier 1 (mpv, ffplay, sox) should all beat Tier 4 (aplay)
        let ffplay_pos = players.iter().position(|p| *p == AudioPlayer::FfPlay);
        let sox_pos = players.iter().position(|p| *p == AudioPlayer::Sox);

        assert!(
            ffplay_pos.unwrap() < aplay_pos.unwrap(),
            "ffplay should rank before aplay"
        );
        assert!(
            sox_pos.unwrap() < aplay_pos.unwrap(),
            "sox should rank before aplay"
        );
    }

    #[test]
    fn capability_score_method_matches_scoring() {
        for player in all_players() {
            let metadata = PLAYER_LOOKUP.get(player).unwrap();
            let method_score = metadata.capability_score();
            let fn_score = player_score(*player).0;
            assert_eq!(
                method_score, fn_score,
                "capability_score() should match player_score() for {:?}",
                player
            );
        }
    }
}
