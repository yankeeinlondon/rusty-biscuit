# Playa Kickoff

This plan will build the first release of the `Playa` Library and CLI.

## Features

- read [README.md](../README.md) for context on how this library / CLI are intended to work
- add a `justfile` for "build", "install", and "lint"
- **match_players**(AudioFormat) -> Option<Vec<AudioPlayer>>
    - use the metadata defined in this repo to determine which of the player _can_ play the audio format passed in
- **match_available_players**(AudioFormat) -> Option<Vec<AudioPlayer>>
    - first use the `match_player(AudioFormat)` function to get a list of capable players for the audio format passed in
    - then filter down to only those the host has installed
- **detect_audio_format**(audio) -> Option<AudioFormat>
    - given a file, URL, or stream ... detects the audio format
    - use the `audio` skill and choose an appropriate crate to help with the detection logic
- **playa_explicit**(AudioFormat, audio)
    - implement the `playa_explicit(AudioFormat, audio)` function
- **playa(audio)**
    - implement the `playa(audio)` function -- which unlike the
- `Audio` struct


### Provided Structure

To start us off we've added the following code which should be considered a good first draft but not finalized:

- `AudioPlayer` enum in `lib.rs`
- `Player` struct in `lib.rs`



### Audio Detection

Audio format detection uses content-based MIME detection via the `infer` crate (lightweight, ~80 formats, MIT license, ~150ns match time).

#### Design Principles

- **Content over extension**: Never trust file extensions alone; always detect by magic bytes
- **Separate container vs codec**: Use distinct `AudioFileFormat` (container) and `Codec` types
- **Streaming-friendly**: Only need first ~256 bytes for detection (header inspection)
- **Multi-source support**: Handle local files, HTTP URLs, and in-memory byte slices

#### Type Definitions

```rust
/// Audio codecs (compression algorithms)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Codec {
    Pcm,       // Raw uncompressed
    Flac,      // Lossless compression
    Alac,      // Apple Lossless
    Mp3,       // MPEG-1 Audio Layer III
    Aac,       // Advanced Audio Coding
    Vorbis,    // Ogg Vorbis (lossy)
    Opus,      // Modern low-bitrate codec
}

/// Audio file containers (file format wrappers)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioFileFormat {
    Wav,       // PCM container
    Aiff,      // Apple PCM container
    Flac,      // FLAC container (codec = container)
    Mp3,       // MP3 container (codec = container)
    Ogg,       // Ogg container (Vorbis, Opus, FLAC)
    M4a,       // MP4 audio (AAC, ALAC)
    Webm,      // WebM audio (Vorbis, Opus)
}

/// Combined detection result
pub struct AudioFormat {
    pub file_format: AudioFileFormat,
    pub codec: Option<Codec>,  // None if codec detection requires deeper parsing
}
```

#### Detection Function Signatures

```rust
/// Detect audio format from a file path
pub fn detect_audio_format_from_path(path: &Path) -> Result<AudioFormat, DetectionError>;

/// Detect audio format from raw bytes (header only - first 256+ bytes)
pub fn detect_audio_format_from_bytes(data: &[u8]) -> Result<AudioFormat, DetectionError>;

/// Detect audio format from a URL (fetches first 256 bytes via HTTP Range request)
pub async fn detect_audio_format_from_url(url: &str) -> Result<AudioFormat, DetectionError>;
```

#### Implementation Notes

- Use `infer::get()` for initial MIME detection
- Map MIME types to `AudioFileFormat` enum variants:
  - `audio/mpeg` -> `Mp3`
  - `audio/flac` -> `Flac`
  - `audio/ogg` -> `Ogg`
  - `audio/wav` or `audio/x-wav` -> `Wav`
  - `audio/aiff` -> `Aiff`
  - `audio/mp4` or `audio/x-m4a` -> `M4a`
  - `audio/webm` -> `Webm`
- For containers that support multiple codecs (Ogg, M4a, Webm), optionally use `symphonia` for deeper codec detection
- URL detection uses HTTP `Range: bytes=0-255` header to minimize bandwidth

#### Dependencies

```toml
[dependencies]
infer = "0.19"           # Content-based MIME detection
# Optional for deeper codec detection:
symphonia = { version = "0.5", features = ["all"], optional = true }
```

#### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("unknown audio format")]
    UnknownFormat,
    #[error("not an audio file: detected {0}")]
    NotAudio(String),
    #[error("insufficient data for detection (need at least 256 bytes)")]
    InsufficientData,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Http(String),
}
```

### Player Metadata

The `Playa` library extends the `Sniff` library's `HeadlessAudio` enum with additional audio-specific metadata. While `Sniff` provides detection, installation, version, and website information, `Playa` adds codec/format support and playback capabilities.

#### Design Principles

- **Delegate to Sniff**: Use `sniff_lib::programs::HeadlessAudio` for detection/installation; don't duplicate
- **Explicit format lists**: Even for players with FFmpeg backends, enumerate supported formats explicitly
- **Runtime queryable**: All metadata accessible via the `Player` struct and `PLAYER_LOOKUP` static map

#### Player Struct Definition

```rust
use sniff_lib::programs::{HeadlessAudio, ProgramMetadata};

/// CPU/memory usage classification for players
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceUsage {
    /// Minimal resource usage (aplay, mpg123, ogg123)
    Low,
    /// Moderate resource usage (mpv, ffplay, VLC)
    Medium,
    /// High resource usage
    High,
}

/// Extended metadata for audio players beyond what Sniff provides
pub struct Player {
    /// The player identifier (mirrors HeadlessAudio)
    pub id: AudioPlayer,

    /// Reference to Sniff's program enum for detection/installation
    pub sniff_program: HeadlessAudio,

    /// Codecs this player can decode
    pub supported_codecs: &'static [Codec],

    /// File containers this player can read
    pub supported_formats: &'static [AudioFileFormat],

    /// Can accept audio from stdin or URLs (not just file paths)
    pub takes_stream_input: bool,

    /// Can output audio over network (e.g., to Icecast)
    pub supplies_stream_output: bool,

    /// Whether the player is open source
    pub is_open_source: bool,

    /// CPU/memory usage classification
    pub resource_usage: ResourceUsage,
}
```

#### Player Capabilities Matrix

| Player | Codecs | Formats | Stream In | Stream Out | OSS | Resources |
|--------|--------|---------|-----------|------------|-----|-----------|
| **mpv** | All | All | Yes (HTTP, RTMP, HLS) | No | Yes | Medium |
| **ffplay** | All | All | Yes (HTTP, pipes) | No | Yes | Medium |
| **VLC (cvlc)** | All | All | Yes (HTTP, RTSP, RTP) | Yes (Icecast) | Yes | Medium |
| **MPlayer** | All | All | Yes | No | Yes | Medium |
| **GStreamer** | All | All | Yes | Yes | Yes | Medium |
| **SoX** | PCM, MP3, FLAC, Vorbis | WAV, FLAC, Ogg, MP3 | Yes (pipes) | No | Yes | Low |
| **mpg123** | MP3 only | MP3 | Yes (HTTP) | No | Yes | Low |
| **ogg123** | Vorbis, Opus, FLAC | Ogg | Yes (HTTP) | No | Yes | Low |
| **aplay** | PCM only | WAV, AU | No | No | Yes | Low |
| **paplay** | PCM only | WAV | No | No | Yes | Low |
| **pw-play** | PCM, FLAC | WAV, FLAC | No | No | Yes | Low |

#### Static Lookup Implementation

```rust
use std::collections::HashMap;
use std::sync::LazyLock;

/// All codecs supported by FFmpeg-based players
static FFMPEG_CODECS: &[Codec] = &[
    Codec::Pcm, Codec::Flac, Codec::Alac, Codec::Mp3,
    Codec::Aac, Codec::Vorbis, Codec::Opus,
];

/// All formats supported by FFmpeg-based players
static FFMPEG_FORMATS: &[AudioFileFormat] = &[
    AudioFileFormat::Wav, AudioFileFormat::Aiff, AudioFileFormat::Flac,
    AudioFileFormat::Mp3, AudioFileFormat::Ogg, AudioFileFormat::M4a,
    AudioFileFormat::Webm,
];

pub static PLAYER_LOOKUP: LazyLock<HashMap<AudioPlayer, Player>> = LazyLock::new(|| {
    let mut m = HashMap::with_capacity(11);

    m.insert(AudioPlayer::Mpv, Player {
        id: AudioPlayer::Mpv,
        sniff_program: HeadlessAudio::Mpv,
        supported_codecs: FFMPEG_CODECS,
        supported_formats: FFMPEG_FORMATS,
        takes_stream_input: true,
        supplies_stream_output: false,
        is_open_source: true,
        resource_usage: ResourceUsage::Medium,
    });

    m.insert(AudioPlayer::Mpg123, Player {
        id: AudioPlayer::Mpg123,
        sniff_program: HeadlessAudio::Mpg123,
        supported_codecs: &[Codec::Mp3],
        supported_formats: &[AudioFileFormat::Mp3],
        takes_stream_input: true,
        supplies_stream_output: false,
        is_open_source: true,
        resource_usage: ResourceUsage::Low,
    });

    // ... remaining players follow same pattern

    m
});
```

#### Integration with Sniff

The `Player` struct provides a bridge to Sniff's metadata:

```rust
impl Player {
    /// Get the binary name from Sniff
    pub fn binary_name(&self) -> &'static str {
        self.sniff_program.binary_name()
    }

    /// Get the website URL from Sniff
    pub fn website(&self) -> &'static str {
        self.sniff_program.website()
    }

    /// Get the description from Sniff
    pub fn description(&self) -> &'static str {
        self.sniff_program.description()
    }

    /// Check if player can handle the given format
    pub fn supports_format(&self, format: AudioFileFormat) -> bool {
        self.supported_formats.contains(&format)
    }

    /// Check if player can decode the given codec
    pub fn supports_codec(&self, codec: Codec) -> bool {
        self.supported_codecs.contains(&codec)
    }
}
```


### `Audio` struct

While this library provides the utility functions we've mentioned (e.g., `match_players`, `match_available_players`, `detect_audio`, `playa`, `playa_explicit`) we will also build a struct called `Audio` which provides a convenient and straight forward way to interact with audio content.

```rust
pub struct Audio {
    cache: Option<Vec<u8>>,
    /// how many milliseconds into the recording was it paused?
    paused_at: Option<u32>,
    pub source: Option<String>,
    pub codec: Option<Codec>,
    pub file_format: Option<AudioFileFormat>,
    pub uri: Option<Uri>
}

impl TryFrom for Audio {
    async fn try_from<T: Into<Url>>(url: T) -> Result<Audio, InvalidAudio>;
    async fn try_from(binary: Vec<u8>) -> Result<Audio, InvalidAudio>;
    async fn try_from(binary: &[u8]) -> Result<Audio, InvalidAudio>;
}

impl for Audio {
    /// takes a local filepath or a File URI (e.g., `file://...`) to an audio file
    pub fn new<T:Into<String>>(source: T) -> Result<Audio,InvalidAudio> { };

    /// whether the content is ready to be played (e.g., codec and file format detected,
    /// content local or cached to local, at least one available player is ready to play
    /// the audio.
    pub fn ready() -> bool { };

    /// The audio file or stream has been analyzed and the codec and file format determined
    /// so that we can detect if there are any installed players which
    pub fn playable() -> bool { };

    /// play the audio content
    pub fn play() -> ();

    /// pause the audio content (if currently playing)
    pub fn pause() -> ();

    /// resume the audio content from a paused state,
    /// no effect if the audio had not been paused previously
    pub fn resume() -> ();

}

```
