//! Audio file caching for TTS providers.
//!
//! This module provides a caching layer for TTS audio files, avoiding
//! redundant generation when the same text is spoken with the same settings.
//!
//! ## Cache Key Components
//!
//! The cache key is generated from:
//! - Provider name (e.g., "kokoro", "gtts", "elevenlabs")
//! - Voice ID (provider-specific identifier)
//! - Text content
//! - Audio format/extension
//! - Speed (only for providers that bake speed into the audio)
//!
//! ## Provider Speed Handling
//!
//! | Provider | Speed in Hash | Reason |
//! |----------|---------------|--------|
//! | Kokoro | No | Playa handles playback speed |
//! | Gtts | No | Playa handles playback speed |
//! | EchoGarden | Yes | Speed baked via --speed flag |
//! | ElevenLabs | Yes | Speed baked via API |
//!
//! ## Examples
//!
//! ```ignore
//! use biscuit_speaks::audio_cache::CacheKey;
//!
//! // For providers where playa handles speed
//! let key = CacheKey::new("kokoro", "af_heart", "Hello world", "wav");
//! let path = key.cache_path();
//!
//! // For providers that bake speed into audio
//! let key = CacheKey::new("elevenlabs", "voice-id", "Hello", "mp3")
//!     .with_speed(1.25);
//! ```

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use biscuit_hash::xx_hash;

/// Errors that can occur during cache operations.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// Failed to write to the cache.
    #[error("Cache write failed: {0}")]
    WriteError(#[from] std::io::Error),

    /// The cache key is invalid.
    #[error("Invalid cache key: {reason}")]
    InvalidKey {
        /// Reason the key is invalid.
        reason: String,
    },
}

/// A cache key for TTS audio files.
///
/// The cache key deterministically identifies a specific audio generation
/// request. The same inputs will always produce the same hash, enabling
/// efficient cache lookups.
#[derive(Debug, Clone)]
pub struct CacheKey {
    /// Provider name (e.g., "kokoro", "gtts", "elevenlabs").
    provider: String,
    /// Voice identifier (provider-specific).
    voice_id: String,
    /// Text content to be spoken.
    text: String,
    /// Audio format extension (e.g., "wav", "mp3").
    format: String,
    /// Speed multiplier (only included for providers that bake speed).
    speed: Option<f32>,
}

impl CacheKey {
    /// Create a new cache key.
    ///
    /// ## Arguments
    ///
    /// * `provider` - Provider name (e.g., "kokoro", "gtts")
    /// * `voice_id` - Voice identifier
    /// * `text` - Text content to be spoken
    /// * `format` - Audio format extension (e.g., "wav", "mp3")
    pub fn new(
        provider: impl Into<String>,
        voice_id: impl Into<String>,
        text: impl Into<String>,
        format: impl Into<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            voice_id: voice_id.into(),
            text: text.into(),
            format: format.into(),
            speed: None,
        }
    }

    /// Include speed in the cache key.
    ///
    /// Only use this for providers that bake speed into the audio
    /// (EchoGarden, ElevenLabs). For providers where playa handles
    /// playback speed (Kokoro, Gtts), do NOT include speed.
    #[must_use]
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = Some(speed);
        self
    }

    /// Generate the xxHash for this cache key.
    ///
    /// The hash is deterministic: same inputs always produce the same output.
    pub fn generate_hash(&self) -> String {
        // Build the cache key string
        let key = if let Some(speed) = self.speed {
            format!(
                "{}:{}:{}:{}:{:.3}",
                self.provider, self.voice_id, self.text, self.format, speed
            )
        } else {
            format!(
                "{}:{}:{}:{}",
                self.provider, self.voice_id, self.text, self.format
            )
        };

        // Hash and format as 16-character hex
        format!("{:016x}", xx_hash(&key))
    }

    /// Generate the full cache file path.
    ///
    /// Returns a path in the system temp directory with the format:
    /// `{temp_dir}/biscuit-speaks-{hash}.{ext}`
    pub fn cache_path(&self) -> PathBuf {
        let hash = self.generate_hash();
        let filename = format!("biscuit-speaks-{}.{}", hash, self.format);
        std::env::temp_dir().join(filename)
    }

    /// Check if a cached file exists for this key.
    pub fn cache_exists(&self) -> bool {
        self.cache_path().exists()
    }
}

/// Atomically write data to a file.
///
/// Uses a write-to-temp-then-rename pattern to prevent partial writes
/// from corrupting the cache. This is safe for concurrent access.
///
/// ## Arguments
///
/// * `path` - Final destination path
/// * `data` - Data to write
///
/// ## Errors
///
/// Returns `CacheError::WriteError` if the write or rename fails.
pub fn write_atomic(path: &std::path::Path, data: &[u8]) -> Result<(), CacheError> {
    // Create a temp file in the same directory to ensure same filesystem
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));

    // Ensure parent directory exists
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    // Use a unique temp file name with PID and timestamp
    let temp_path = parent.join(format!(
        ".biscuit-speaks-tmp-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    // Write to temp file
    {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
    }

    // Atomically rename to final path
    fs::rename(&temp_path, path)?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_new() {
        let key = CacheKey::new("kokoro", "af_heart", "Hello world", "wav");
        assert_eq!(key.provider, "kokoro");
        assert_eq!(key.voice_id, "af_heart");
        assert_eq!(key.text, "Hello world");
        assert_eq!(key.format, "wav");
        assert!(key.speed.is_none());
    }

    #[test]
    fn test_cache_key_with_speed() {
        let key = CacheKey::new("elevenlabs", "voice-id", "Hello", "mp3").with_speed(1.25);
        assert_eq!(key.speed, Some(1.25));
    }

    #[test]
    fn test_hash_determinism() {
        // Same inputs must produce same hash
        let key1 = CacheKey::new("kokoro", "af_heart", "Hello world", "wav");
        let key2 = CacheKey::new("kokoro", "af_heart", "Hello world", "wav");

        let hash1 = key1.generate_hash();
        let hash2 = key2.generate_hash();

        assert_eq!(hash1, hash2);

        // Run multiple times to ensure determinism
        for _ in 0..10 {
            let key = CacheKey::new("kokoro", "af_heart", "Hello world", "wav");
            assert_eq!(key.generate_hash(), hash1);
        }
    }

    #[test]
    fn test_hash_uniqueness_different_text() {
        let key1 = CacheKey::new("kokoro", "af_heart", "Hello", "wav");
        let key2 = CacheKey::new("kokoro", "af_heart", "World", "wav");

        assert_ne!(key1.generate_hash(), key2.generate_hash());
    }

    #[test]
    fn test_hash_uniqueness_different_voice() {
        let key1 = CacheKey::new("kokoro", "af_heart", "Hello", "wav");
        let key2 = CacheKey::new("kokoro", "af_sky", "Hello", "wav");

        assert_ne!(key1.generate_hash(), key2.generate_hash());
    }

    #[test]
    fn test_hash_uniqueness_different_provider() {
        let key1 = CacheKey::new("kokoro", "af_heart", "Hello", "wav");
        let key2 = CacheKey::new("gtts", "af_heart", "Hello", "wav");

        assert_ne!(key1.generate_hash(), key2.generate_hash());
    }

    #[test]
    fn test_hash_uniqueness_different_format() {
        let key1 = CacheKey::new("kokoro", "af_heart", "Hello", "wav");
        let key2 = CacheKey::new("kokoro", "af_heart", "Hello", "mp3");

        assert_ne!(key1.generate_hash(), key2.generate_hash());
    }

    #[test]
    fn test_speed_affects_hash() {
        let key1 = CacheKey::new("elevenlabs", "voice-id", "Hello", "mp3");
        let key2 = CacheKey::new("elevenlabs", "voice-id", "Hello", "mp3").with_speed(1.0);
        let key3 = CacheKey::new("elevenlabs", "voice-id", "Hello", "mp3").with_speed(1.25);

        // No speed vs speed should differ
        assert_ne!(key1.generate_hash(), key2.generate_hash());
        // Different speeds should differ
        assert_ne!(key2.generate_hash(), key3.generate_hash());
    }

    #[test]
    fn test_cache_path_format() {
        let key = CacheKey::new("kokoro", "af_heart", "Hello", "wav");
        let path = key.cache_path();

        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with("biscuit-speaks-"));
        assert!(filename.ends_with(".wav"));
        // Hash should be 16 hex characters
        let hash_part = filename
            .strip_prefix("biscuit-speaks-")
            .unwrap()
            .strip_suffix(".wav")
            .unwrap();
        assert_eq!(hash_part.len(), 16);
        assert!(hash_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_cache_path_mp3_format() {
        let key = CacheKey::new("elevenlabs", "voice-id", "Hello", "mp3");
        let path = key.cache_path();

        assert!(path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .ends_with(".mp3"));
    }

    #[test]
    fn test_cache_exists_false_for_nonexistent() {
        let key = CacheKey::new("test", "nonexistent", "unique text 12345", "wav");
        assert!(!key.cache_exists());
    }

    #[test]
    fn test_atomic_write() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let temp_dir = std::env::temp_dir();
        let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let test_file = temp_dir.join(format!(
            "biscuit-speaks-test-atomic-{}-{}.tmp",
            std::process::id(),
            unique_id
        ));

        // Clean up if exists
        let _ = fs::remove_file(&test_file);

        let data = b"test content for atomic write";
        write_atomic(&test_file, data).expect("Write should succeed");

        // Verify content
        let read_data = fs::read(&test_file).expect("Should be able to read");
        assert_eq!(read_data, data);

        // Clean up
        let _ = fs::remove_file(&test_file);
    }

    #[test]
    fn test_atomic_write_overwrites() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1000);

        let temp_dir = std::env::temp_dir();
        let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let test_file = temp_dir.join(format!(
            "biscuit-speaks-test-overwrite-{}-{}.tmp",
            std::process::id(),
            unique_id
        ));

        // Clean up if exists
        let _ = fs::remove_file(&test_file);

        // Write initial content
        write_atomic(&test_file, b"initial").expect("First write should succeed");

        // Overwrite
        write_atomic(&test_file, b"updated").expect("Second write should succeed");

        // Verify new content
        let read_data = fs::read(&test_file).expect("Should be able to read");
        assert_eq!(read_data, b"updated");

        // Clean up
        let _ = fs::remove_file(&test_file);
    }
}
