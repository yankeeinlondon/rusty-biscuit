/// Audio codecs (compression algorithms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Codec {
    /// Raw uncompressed PCM audio.
    Pcm,
    /// FLAC lossless compression.
    Flac,
    /// Apple Lossless (ALAC).
    Alac,
    /// MPEG-1 Audio Layer III.
    Mp3,
    /// Advanced Audio Coding.
    Aac,
    /// Ogg Vorbis.
    Vorbis,
    /// Opus codec.
    Opus,
}

/// Audio file containers (file format wrappers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioFileFormat {
    /// WAV container (typically PCM).
    Wav,
    /// AIFF container (typically PCM).
    Aiff,
    /// FLAC container (codec = container).
    Flac,
    /// MP3 container (codec = container).
    Mp3,
    /// Ogg container (Vorbis, Opus, FLAC).
    Ogg,
    /// MP4 audio container (AAC, ALAC).
    M4a,
    /// WebM audio container (Vorbis, Opus).
    Webm,
}

/// Combined audio format detection result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioFormat {
    /// The container format.
    pub file_format: AudioFileFormat,
    /// The codec if it can be inferred from the container.
    pub codec: Option<Codec>,
}

impl AudioFormat {
    /// Create a new audio format pairing.
    pub const fn new(file_format: AudioFileFormat, codec: Option<Codec>) -> Self {
        Self { file_format, codec }
    }
}

/// CPU and memory usage classification for players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceUsage {
    /// Minimal resource usage.
    Low,
    /// Moderate resource usage.
    Medium,
    /// High resource usage.
    High,
}
