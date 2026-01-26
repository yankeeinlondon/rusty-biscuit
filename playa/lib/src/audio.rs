use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use url::Url;

use crate::detection::{
    detect_audio_format_from_bytes, detect_audio_format_from_path, detect_audio_format_from_url,
};
use crate::error::{DetectionError, InvalidAudio, PlaybackError};
use crate::playback::playa_explicit_with_options;
use crate::types::{AudioFormat, PlaybackOptions};

/// Audio source data.
#[derive(Debug, Clone)]
pub enum AudioData {
    /// Audio stored on disk.
    FilePath(PathBuf),
    /// Audio available via URL.
    Url(Url),
    /// Raw audio bytes held in memory.
    Bytes(Arc<Vec<u8>>),
}

/// The kind of audio source used by an [`Audio`] instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioSourceKind {
    /// Audio stored on disk.
    FilePath,
    /// Audio available via URL.
    Url,
    /// Audio stored in memory.
    Bytes,
}

/// Stateful audio wrapper with pause tracking.
#[derive(Debug, Clone)]
pub struct Audio {
    data: AudioData,
    format: AudioFormat,
    state: Arc<Mutex<AudioState>>,
}

#[derive(Debug)]
struct AudioState {
    paused_at_ms: Option<u32>,
    last_started_at: Option<Instant>,
    is_playing: bool,
    is_paused: bool,
}

impl Audio {
    /// Create an Audio instance from a file path.
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, InvalidAudio> {
        let path = path.into();
        let format = detect_audio_format_from_path(&path)?;
        Ok(Self::new(AudioData::FilePath(path), format))
    }

    /// Create an Audio instance from a URL.
    pub async fn from_url(url: impl AsRef<str>) -> Result<Self, InvalidAudio> {
        let format = detect_audio_format_from_url(url.as_ref()).await?;
        let url = Url::parse(url.as_ref()).map_err(DetectionError::Url)?;
        Ok(Self::new(AudioData::Url(url), format))
    }

    /// Create an Audio instance from in-memory bytes.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, InvalidAudio> {
        let bytes = bytes.into();
        let format = detect_audio_format_from_bytes(&bytes)?;
        Ok(Self::new(AudioData::Bytes(Arc::new(bytes)), format))
    }

    /// Return the detected audio format.
    pub fn format(&self) -> AudioFormat {
        self.format
    }

    /// Return the last paused position in milliseconds, if available.
    pub fn paused_at(&self) -> Result<Option<u32>, PlaybackError> {
        let state = self.state.lock().map_err(|_| PlaybackError::StateLock)?;
        Ok(state.paused_at_ms)
    }

    /// Play the audio using the best available player.
    pub fn play(&self) -> Result<(), PlaybackError> {
        self.play_with_options(PlaybackOptions::default())
    }

    /// Play the audio with custom volume/speed options.
    ///
    /// The player will be selected based on format compatibility AND the
    /// required capabilities (speed/volume control). If options require
    /// capabilities that no installed player supports, returns
    /// [`PlaybackError::NoPlayerWithCapabilities`].
    pub fn play_with_options(&self, options: PlaybackOptions) -> Result<(), PlaybackError> {
        playa_explicit_with_options(self.format, self.data.clone(), options)?;
        self.mark_playing()?;
        Ok(())
    }

    /// Mark the audio as paused and track the pause position.
    pub fn pause(&self) -> Result<(), PlaybackError> {
        let mut state = self.state.lock().map_err(|_| PlaybackError::StateLock)?;
        if !state.is_playing {
            return Err(PlaybackError::NotPlaying);
        }

        let paused_at_ms = state
            .last_started_at
            .map(|start| {
                let elapsed = start.elapsed().as_millis();
                if elapsed > u32::MAX as u128 {
                    u32::MAX
                } else {
                    elapsed as u32
                }
            })
            .unwrap_or(0);

        state.paused_at_ms = Some(paused_at_ms);
        state.is_playing = false;
        state.is_paused = true;
        state.last_started_at = None;
        Ok(())
    }

    /// Mark the audio as resumed.
    pub fn resume(&self) -> Result<(), PlaybackError> {
        let mut state = self.state.lock().map_err(|_| PlaybackError::StateLock)?;
        if !state.is_paused {
            return Err(PlaybackError::NotPaused);
        }

        state.is_paused = false;
        state.is_playing = true;
        state.last_started_at = Some(Instant::now());
        Ok(())
    }

    /// Return the source kind for this audio.
    pub fn source_kind(&self) -> AudioSourceKind {
        match &self.data {
            AudioData::FilePath(_) => AudioSourceKind::FilePath,
            AudioData::Url(_) => AudioSourceKind::Url,
            AudioData::Bytes(_) => AudioSourceKind::Bytes,
        }
    }

    /// Consume the `Audio` and return the underlying `AudioData`.
    pub fn into_data(self) -> AudioData {
        self.data
    }

    /// Record playback start time and clear pause state.
    pub(crate) fn mark_playing(&self) -> Result<(), PlaybackError> {
        let mut state = self.state.lock().map_err(|_| PlaybackError::StateLock)?;
        state.paused_at_ms = None;
        state.is_playing = true;
        state.is_paused = false;
        state.last_started_at = Some(Instant::now());
        Ok(())
    }

    fn new(data: AudioData, format: AudioFormat) -> Self {
        Self {
            data,
            format,
            state: Arc::new(Mutex::new(AudioState {
                paused_at_ms: None,
                last_started_at: None,
                is_playing: false,
                is_paused: false,
            })),
        }
    }
}
