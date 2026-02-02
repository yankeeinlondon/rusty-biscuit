//! Data types for volume snapshots and session identification.

/// Identifies an audio session or endpoint for ducking operations.
///
/// Each platform uses different mechanisms to identify audio sources:
/// - macOS: Endpoint-wide (default output device)
/// - Windows: Per-session with process ID
/// - Linux: Per-sink-input with name and index
#[derive(Debug, Clone, PartialEq)]
pub enum SessionId {
    /// macOS: The default output endpoint (virtual master volume).
    MacEndpoint,

    /// Windows: A WASAPI audio session identified by process ID and session key.
    WasapiSession {
        /// The process ID owning this session.
        pid: u32,
        /// Session identifier key.
        key: String,
    },

    /// Linux: A PulseAudio/PipeWire sink input.
    PulseSinkInput {
        /// The sink input index.
        index: u32,
        /// The application name (from `application.name` property).
        name: String,
    },

    /// Linux: ALSA master mixer control (fallback).
    AlsaMaster,

    /// macOS: Media key pause/resume fallback.
    ///
    /// Used when CoreAudio volume control is unavailable (e.g., professional
    /// DACs, USB audio interfaces). Pauses other audio via media key simulation.
    MediaKeys,
}

/// Volume state for a single audio session or channel.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionVolume {
    /// Identifier for this session.
    pub id: SessionId,
    /// Per-channel volume levels (0.0 to 1.0).
    /// Most sessions have 1-2 channels (mono/stereo).
    pub channels: Vec<f32>,
    /// Whether the session is muted.
    pub mute: bool,
}

impl SessionVolume {
    /// Creates a new session volume entry.
    pub fn new(id: SessionId, channels: Vec<f32>, mute: bool) -> Self {
        Self { id, channels, mute }
    }

    /// Returns the average volume across all channels.
    pub fn average_volume(&self) -> f32 {
        if self.channels.is_empty() {
            return 0.0;
        }
        self.channels.iter().sum::<f32>() / self.channels.len() as f32
    }

    /// Creates a scaled copy of this volume at the given scalar.
    pub fn scaled(&self, scalar: f32) -> Self {
        Self {
            id: self.id.clone(),
            channels: self.channels.iter().map(|v| v * scalar).collect(),
            mute: self.mute,
        }
    }
}

/// A snapshot of volume states to restore after ducking.
#[derive(Debug, Clone, Default)]
pub struct VolumeSnapshot {
    /// All captured session volumes.
    pub entries: Vec<SessionVolume>,
}

impl VolumeSnapshot {
    /// Creates an empty snapshot.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Creates a snapshot with the given entries.
    pub fn with_entries(entries: Vec<SessionVolume>) -> Self {
        Self { entries }
    }

    /// Returns true if the snapshot is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the number of sessions in the snapshot.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Adds a session volume to the snapshot.
    pub fn push(&mut self, volume: SessionVolume) {
        self.entries.push(volume);
    }
}
