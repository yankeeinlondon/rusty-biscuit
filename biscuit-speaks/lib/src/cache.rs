//! JSON-based cache for host TTS capabilities.
//!
//! This module provides persistent caching of discovered TTS provider capabilities
//! to avoid expensive re-enumeration on every TTS operation.
//!
//! ## Cache Location
//!
//! The cache file is stored at `~/.biscuit-speaks-cache.json`, unless
//! `BISCUIT_SPEAKS_CACHE` names a different file. See [`cache_file_path`].
//!
//! ## Atomicity
//!
//! Write operations use a temp file + rename pattern to ensure atomicity.
//! This prevents corruption if the process is interrupted during a write.
//!
//! ## Cache Invalidation
//!
//! The cache includes:
//! - A `schema_version` field for forward compatibility
//! - A `last_updated` timestamp for age-based invalidation
//!
//! Use [`bust_host_capability_cache`] to manually invalidate the cache
//! when new voices are installed on the system.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::errors::TtsError;
use crate::types::{HostTtsCapabilities, HostTtsCapability, TtsProvider, Voice};

/// Current schema version for the cache file format.
///
/// Increment this when making breaking changes to the cache structure.
pub const CACHE_SCHEMA_VERSION: u32 = 1;

/// Default cache file name.
const CACHE_FILE_NAME: &str = ".biscuit-speaks-cache.json";

/// Environment variable naming an alternate cache file.
const CACHE_PATH_ENV: &str = "BISCUIT_SPEAKS_CACHE";

/// Wrapper struct for the cache file that includes schema versioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEnvelope {
    /// Schema version for forward compatibility.
    schema_version: u32,
    /// The actual capabilities data.
    capabilities: HostTtsCapabilities,
}

/// Get the path to the cache file.
///
/// ## Environment Variables
///
/// `BISCUIT_SPEAKS_CACHE` names the cache **file** — not its directory — and
/// takes priority over the home-directory default. Its parent directory must
/// already exist; writes are not given one. An empty value is treated as unset.
///
/// It is inherited by child processes, which is what lets a caller confine a
/// re-spawned `so-you-say` (see `--background`) to the same relocated cache.
///
/// ## Returns
///
/// `None` if no override is set and the home directory cannot be determined.
fn cache_file_path() -> Option<PathBuf> {
    resolve_cache_path(std::env::var_os(CACHE_PATH_ENV))
}

/// Apply the override rule to an already-read environment value.
///
/// Split from [`cache_file_path`] so the precedence is testable without
/// mutating process-global environment state.
fn resolve_cache_path(override_value: Option<OsString>) -> Option<PathBuf> {
    match override_value {
        Some(raw) if !raw.is_empty() => Some(PathBuf::from(raw)),
        _ => dirs::home_dir().map(|home| home.join(CACHE_FILE_NAME)),
    }
}

/// Read TTS capabilities from the cache file.
///
/// ## Returns
///
/// - `Ok(capabilities)` if the cache exists and is valid
/// - `Err(CacheReadError)` if the cache doesn't exist, is corrupted, or has an incompatible schema
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::cache::read_from_cache;
///
/// match read_from_cache() {
///     Ok(capabilities) => println!("Loaded {} providers", capabilities.providers.len()),
///     Err(e) => println!("Cache miss: {}", e),
/// }
/// ```
pub fn read_from_cache() -> Result<HostTtsCapabilities, TtsError> {
    let path = cache_file_path().ok_or_else(|| TtsError::CacheReadError {
        path: "~/.biscuit-speaks-cache.json".into(),
        message: "Could not determine home directory".into(),
    })?;

    read_cache_at(&path)
}

/// Read TTS capabilities from a specific cache file.
fn read_cache_at(path: &Path) -> Result<HostTtsCapabilities, TtsError> {
    let path_str = path.display().to_string();

    let contents = fs::read_to_string(path).map_err(|e| TtsError::CacheReadError {
        path: path_str.clone(),
        message: e.to_string(),
    })?;

    let envelope: CacheEnvelope =
        serde_json::from_str(&contents).map_err(|e| TtsError::CacheReadError {
            path: path_str.clone(),
            message: format!("JSON parse error: {}", e),
        })?;

    // Check schema version compatibility
    if envelope.schema_version != CACHE_SCHEMA_VERSION {
        return Err(TtsError::CacheReadError {
            path: path_str,
            message: format!(
                "Cache schema version mismatch: expected {}, found {}",
                CACHE_SCHEMA_VERSION, envelope.schema_version
            ),
        });
    }

    Ok(envelope.capabilities)
}

/// Update a single provider's capabilities in the cache.
///
/// This function reads the existing cache (if any), replaces or adds the
/// specified provider's data, and writes the updated cache atomically.
///
/// ## Arguments
///
/// * `provider` - The TTS provider to update
/// * `voices` - Currently installed voices for this provider
/// * `available_voices` - Voices that could be installed but are not yet available
///
/// ## Atomicity
///
/// Uses temp file + rename pattern to ensure the cache file is never left
/// in a partially written state.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::cache::update_provider_in_cache;
/// use biscuit_speaks::types::{TtsProvider, HostTtsProvider, Voice};
///
/// let voices = vec![Voice::new("Samantha")];
/// update_provider_in_cache(
///     TtsProvider::Host(HostTtsProvider::Say),
///     voices,
///     vec![],
/// )?;
/// ```
pub fn update_provider_in_cache(
    provider: TtsProvider,
    voices: Vec<Voice>,
    available_voices: Vec<Voice>,
) -> Result<(), TtsError> {
    let path = cache_file_path().ok_or_else(|| TtsError::CacheWriteError {
        path: "~/.biscuit-speaks-cache.json".into(),
        message: "Could not determine home directory".into(),
    })?;

    update_provider_at(&path, provider, voices, available_voices)
}

/// Update a single provider's capabilities in a specific cache file.
fn update_provider_at(
    path: &Path,
    provider: TtsProvider,
    voices: Vec<Voice>,
    available_voices: Vec<Voice>,
) -> Result<(), TtsError> {
    let path_str = path.display().to_string();

    // Read existing cache or create empty one
    let mut capabilities = read_cache_at(path).unwrap_or_default();

    // Remove existing entry for this provider (if any)
    capabilities
        .providers
        .retain(|cap| cap.provider != provider);

    // Add the new/updated provider capability
    let capability = HostTtsCapability {
        provider,
        voices,
        available_voices,
    };
    capabilities.providers.push(capability);

    // Update timestamp
    capabilities.last_updated = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Write atomically using temp file + rename
    write_cache_atomically(path, &capabilities).map_err(|e| TtsError::CacheWriteError {
        path: path_str,
        message: e.to_string(),
    })?;

    Ok(())
}

/// Delete the cache file to force re-enumeration of TTS capabilities.
///
/// Call this function when:
/// - New voices are installed on the system
/// - TTS providers are added or removed
/// - The cache appears to be corrupted
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::cache::bust_host_capability_cache;
///
/// // Force cache refresh after installing new voices
/// bust_host_capability_cache()?;
/// ```
pub fn bust_host_capability_cache() -> Result<(), TtsError> {
    let path = cache_file_path().ok_or_else(|| TtsError::CacheWriteError {
        path: "~/.biscuit-speaks-cache.json".into(),
        message: "Could not determine home directory".into(),
    })?;

    bust_cache_at(&path)
}

/// Delete a specific cache file, treating an already-absent file as success.
fn bust_cache_at(path: &Path) -> Result<(), TtsError> {
    // Only return error if removal fails for a reason other than "file not found"
    if path.exists() {
        fs::remove_file(path).map_err(|e| TtsError::CacheWriteError {
            path: path.display().to_string(),
            message: format!("Failed to remove cache file: {}", e),
        })?;
    }

    Ok(())
}

/// Write the cache atomically using temp file + rename pattern.
fn write_cache_atomically(
    path: &Path,
    capabilities: &HostTtsCapabilities,
) -> Result<(), std::io::Error> {
    let envelope = CacheEnvelope {
        schema_version: CACHE_SCHEMA_VERSION,
        capabilities: capabilities.clone(),
    };

    let json = serde_json::to_string_pretty(&envelope)?;

    // Get the parent directory for the temp file
    let parent = path.parent().unwrap_or(path);

    // Create temp file in the same directory to ensure atomic rename works
    let mut temp_file = tempfile::NamedTempFile::new_in(parent)?;
    temp_file.write_all(json.as_bytes())?;
    temp_file.flush()?;

    // Persist (rename) the temp file to the target path
    temp_file.persist(path)?;

    Ok(())
}

// ============================================================================
// Cache Population Functions
// ============================================================================

use sniff::programs::{InstalledTtsClients, TtsClient};

use crate::providers::cloud::ElevenLabsProvider;
#[cfg(target_os = "macos")]
use crate::providers::host::SayProvider;
use crate::providers::host::{ESpeakProvider, EchogardenProvider, GttsProvider, KokoroTtsProvider};

#[cfg(target_os = "windows")]
use crate::providers::host::SapiProvider;
use crate::traits::TtsVoiceInventory;

/// Populate the cache for a single provider.
///
/// This function calls the provider's `list_voices()` method and updates
/// the cache with the results.
///
/// ## Arguments
///
/// * `provider` - A reference to the provider implementation
/// * `provider_type` - The `TtsProvider` enum value identifying this provider
///
/// ## Errors
///
/// Returns `TtsError::VoiceEnumerationFailed` if voice listing fails,
/// or `TtsError::CacheWriteError` if the cache update fails.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::{populate_cache_for_provider, SayProvider, TtsProvider, HostTtsProvider};
///
/// let provider = SayProvider;
/// populate_cache_for_provider(
///     &provider,
///     TtsProvider::Host(HostTtsProvider::Say),
/// ).await?;
/// ```
pub async fn populate_cache_for_provider<P: TtsVoiceInventory>(
    provider: &P,
    provider_type: TtsProvider,
) -> Result<(), TtsError> {
    let voices = provider.list_voices().await?;
    update_provider_in_cache(provider_type, voices, vec![])
}

/// Populate a specific cache file for a single provider.
async fn populate_provider_at<P: TtsVoiceInventory>(
    path: &Path,
    provider: &P,
    provider_type: TtsProvider,
) -> Result<(), TtsError> {
    let voices = provider.list_voices().await?;
    update_provider_at(path, provider_type, voices, vec![])
}

/// Populate the cache for all available TTS providers.
///
/// This function detects which TTS providers are available on the system
/// and populates the cache with voice data from each one. Providers are
/// queried in parallel for performance.
///
/// ## Errors
///
/// Returns an error if all provider enumeration attempts fail. Partial
/// failures are logged but don't prevent successful providers from
/// being cached.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::populate_cache_for_all_providers;
///
/// // Populate cache for all detected providers
/// populate_cache_for_all_providers().await?;
/// ```
pub async fn populate_cache_for_all_providers() -> Result<(), TtsError> {
    let path = cache_file_path().ok_or_else(|| TtsError::CacheWriteError {
        path: "~/.biscuit-speaks-cache.json".into(),
        message: "Could not determine home directory".into(),
    })?;

    populate_all_at(&path).await
}

/// Populate a specific cache file from every available TTS provider.
async fn populate_all_at(path: &Path) -> Result<(), TtsError> {
    let installed = InstalledTtsClients::new();

    // Track results for reporting
    let mut any_success = false;
    let mut errors: Vec<(TtsProvider, TtsError)> = Vec::new();

    // Populate host providers based on what's installed
    // We use a macro-like approach to reduce boilerplate

    // macOS Say provider
    #[cfg(target_os = "macos")]
    if installed.is_installed(TtsClient::Say) {
        let provider = SayProvider;
        let provider_type = TtsProvider::Host(crate::types::HostTtsProvider::Say);
        match populate_provider_at(path, &provider, provider_type).await {
            Ok(()) => {
                tracing::info!(provider = "say", "Cached voice data");
                any_success = true;
            }
            Err(e) => {
                tracing::warn!(provider = "say", error = %e, "Failed to cache voices");
                errors.push((provider_type, e));
            }
        }
    }

    // eSpeak provider
    if installed.is_installed(TtsClient::Espeak) || installed.is_installed(TtsClient::EspeakNg) {
        let provider = ESpeakProvider::new();
        let provider_type = TtsProvider::Host(crate::types::HostTtsProvider::ESpeak);
        match populate_provider_at(path, &provider, provider_type).await {
            Ok(()) => {
                tracing::info!(provider = "espeak", "Cached voice data");
                any_success = true;
            }
            Err(e) => {
                tracing::warn!(provider = "espeak", error = %e, "Failed to cache voices");
                errors.push((provider_type, e));
            }
        }
    }

    // Echogarden provider
    if installed.is_installed(TtsClient::Echogarden) {
        let provider = EchogardenProvider::new();
        let provider_type = TtsProvider::Host(crate::types::HostTtsProvider::EchoGarden);
        match populate_provider_at(path, &provider, provider_type).await {
            Ok(()) => {
                tracing::info!(provider = "echogarden", "Cached voice data");
                any_success = true;
            }
            Err(e) => {
                tracing::warn!(provider = "echogarden", error = %e, "Failed to cache voices");
                errors.push((provider_type, e));
            }
        }
    }

    // gTTS provider
    if installed.is_installed(TtsClient::GttsCli) {
        let provider = GttsProvider::new();
        let provider_type = TtsProvider::Host(crate::types::HostTtsProvider::Gtts);
        match populate_provider_at(path, &provider, provider_type).await {
            Ok(()) => {
                tracing::info!(provider = "gtts", "Cached voice data");
                any_success = true;
            }
            Err(e) => {
                tracing::warn!(provider = "gtts", error = %e, "Failed to cache voices");
                errors.push((provider_type, e));
            }
        }
    }

    // Kokoro TTS provider
    if installed.is_installed(TtsClient::KokoroTts) {
        let provider = KokoroTtsProvider::new();
        let provider_type = TtsProvider::Host(crate::types::HostTtsProvider::KokoroTts);
        match populate_provider_at(path, &provider, provider_type).await {
            Ok(()) => {
                tracing::info!(provider = "kokoro-tts", "Cached voice data");
                any_success = true;
            }
            Err(e) => {
                tracing::warn!(provider = "kokoro-tts", error = %e, "Failed to cache voices");
                errors.push((provider_type, e));
            }
        }
    }

    // Windows SAPI provider
    #[cfg(target_os = "windows")]
    if installed.is_installed(TtsClient::WindowsSapi) {
        let provider = SapiProvider::new();
        let provider_type = TtsProvider::Host(crate::types::HostTtsProvider::Sapi);
        match populate_provider_at(path, &provider, provider_type).await {
            Ok(()) => {
                tracing::info!(provider = "sapi", "Cached voice data");
                any_success = true;
            }
            Err(e) => {
                tracing::warn!(provider = "sapi", error = %e, "Failed to cache voices");
                errors.push((provider_type, e));
            }
        }
    }

    // ElevenLabs cloud provider (if API key is available)
    if ElevenLabsProvider::has_api_key()
        && let Ok(provider) = ElevenLabsProvider::new()
    {
        let provider_type = TtsProvider::Cloud(crate::types::CloudTtsProvider::ElevenLabs);
        match populate_provider_at(path, &provider, provider_type).await {
            Ok(()) => {
                tracing::info!(provider = "elevenlabs", "Cached voice data");
                any_success = true;
            }
            Err(e) => {
                tracing::warn!(provider = "elevenlabs", error = %e, "Failed to cache voices");
                errors.push((provider_type, e));
            }
        }
    }

    if any_success {
        Ok(())
    } else if errors.is_empty() {
        // No providers were available to query
        Err(TtsError::CacheWriteError {
            path: path.display().to_string(),
            message: "No TTS providers available to populate cache".into(),
        })
    } else {
        // All providers failed - return the first error
        let (_, first_error) = errors.remove(0);
        Err(first_error)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Gender, HostTtsProvider, Language, VoiceQuality};
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test cache file in a temp directory.
    fn create_test_cache(dir: &TempDir, content: &str) -> PathBuf {
        let path = dir.path().join(CACHE_FILE_NAME);
        fs::write(&path, content).expect("Failed to write test cache");
        path
    }

    // ========================================================================
    // cache_file_path tests
    // ========================================================================

    #[test]
    fn test_cache_file_path_returns_some() {
        // This test verifies cache_file_path returns Some on most systems
        // It may fail in unusual environments without a home directory
        let path = cache_file_path();
        assert!(
            path.is_some(),
            "cache_file_path should return Some on normal systems"
        );
        if let Some(p) = path {
            assert!(
                p.to_string_lossy().contains(".biscuit-speaks-cache.json"),
                "Path should contain the cache file name"
            );
        }
    }

    #[test]
    fn test_cache_path_override_wins_over_home() {
        let dir = TempDir::new().unwrap();
        let override_path = dir.path().join("relocated.json");

        let resolved = resolve_cache_path(Some(override_path.clone().into_os_string()))
            .expect("An explicit override needs no home directory");

        assert_eq!(
            resolved, override_path,
            "BISCUIT_SPEAKS_CACHE should name the cache file verbatim"
        );
    }

    #[test]
    fn test_cache_path_falls_back_when_override_absent_or_empty() {
        let default = resolve_cache_path(None);
        assert_eq!(
            resolve_cache_path(Some(OsString::new())),
            default,
            "An empty override should be treated as unset, not as an empty path"
        );
        assert!(
            default.is_some_and(|p| p.ends_with(CACHE_FILE_NAME)),
            "The fallback should be the home-directory default"
        );
    }

    // ========================================================================
    // read_from_cache tests
    // ========================================================================

    #[test]
    fn test_read_from_cache_file_not_found() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(CACHE_FILE_NAME);

        let result = read_cache_at(&cache_path);
        assert!(
            matches!(result, Err(TtsError::CacheReadError { .. })),
            "Reading an absent cache should be a CacheReadError, got {result:?}"
        );
    }

    #[test]
    fn test_read_from_cache_invalid_json() {
        let dir = TempDir::new().unwrap();
        let cache_path = create_test_cache(&dir, "not valid json");

        let result = read_cache_at(&cache_path);
        let Err(TtsError::CacheReadError { message, .. }) = result else {
            panic!("Invalid JSON should be a CacheReadError, got {result:?}");
        };
        assert!(
            message.contains("JSON parse error"),
            "Error should name the parse failure, got {message}"
        );
    }

    #[test]
    fn test_read_from_cache_schema_mismatch() {
        let dir = TempDir::new().unwrap();
        let content = r#"{
            "schema_version": 999,
            "capabilities": {
                "providers": [],
                "last_updated": 0
            }
        }"#;
        let cache_path = create_test_cache(&dir, content);

        let result = read_cache_at(&cache_path);
        let Err(TtsError::CacheReadError { message, .. }) = result else {
            panic!("A future schema version should be rejected, got {result:?}");
        };
        assert!(
            message.contains("schema version mismatch"),
            "Error should name the version mismatch, got {message}"
        );
    }

    #[test]
    fn test_cache_envelope_serialization_roundtrip() {
        let capabilities = HostTtsCapabilities::new()
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
                    .with_voice(Voice::new("Samantha").with_gender(Gender::Female)),
            )
            .with_timestamp(1234567890);

        let envelope = CacheEnvelope {
            schema_version: CACHE_SCHEMA_VERSION,
            capabilities,
        };

        let json = serde_json::to_string_pretty(&envelope).unwrap();
        let parsed: CacheEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.schema_version, CACHE_SCHEMA_VERSION);
        assert_eq!(parsed.capabilities.providers.len(), 1);
        assert_eq!(parsed.capabilities.providers[0].voices.len(), 1);
        assert_eq!(parsed.capabilities.providers[0].voices[0].name, "Samantha");
    }

    // ========================================================================
    // update_provider_in_cache tests (using isolated temp dir)
    // ========================================================================

    #[test]
    fn test_update_provider_creates_capability_struct() {
        let provider = TtsProvider::Host(HostTtsProvider::Say);
        let voices = vec![
            Voice::new("Samantha")
                .with_gender(Gender::Female)
                .with_quality(VoiceQuality::Excellent)
                .with_language(Language::English),
            Voice::new("Alex").with_gender(Gender::Male),
        ];
        let available = vec![Voice::new("Zoe")];

        let capability = HostTtsCapability {
            provider,
            voices: voices.clone(),
            available_voices: available.clone(),
        };

        assert_eq!(capability.voices.len(), 2);
        assert_eq!(capability.available_voices.len(), 1);
        assert_eq!(capability.voices[0].name, "Samantha");
    }

    #[test]
    fn test_update_replaces_existing_provider() {
        let mut capabilities = HostTtsCapabilities::new()
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
                    .with_voice(Voice::new("OldVoice")),
            )
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::ESpeak))
                    .with_voice(Voice::new("ESpeak1")),
            );

        let provider = TtsProvider::Host(HostTtsProvider::Say);

        // Simulate the update logic
        capabilities
            .providers
            .retain(|cap| cap.provider != provider);

        let new_capability = HostTtsCapability::new(provider).with_voice(Voice::new("NewVoice"));
        capabilities.providers.push(new_capability);

        // Verify: Say provider updated, ESpeak provider unchanged
        assert_eq!(capabilities.providers.len(), 2);

        let say_cap = capabilities
            .get_provider(&TtsProvider::Host(HostTtsProvider::Say))
            .unwrap();
        assert_eq!(say_cap.voices.len(), 1);
        assert_eq!(say_cap.voices[0].name, "NewVoice");

        let espeak_cap = capabilities
            .get_provider(&TtsProvider::Host(HostTtsProvider::ESpeak))
            .unwrap();
        assert_eq!(espeak_cap.voices.len(), 1);
        assert_eq!(espeak_cap.voices[0].name, "ESpeak1");
    }

    #[test]
    fn test_update_adds_new_provider() {
        let mut capabilities = HostTtsCapabilities::new().with_provider(
            HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
                .with_voice(Voice::new("Samantha")),
        );

        let provider = TtsProvider::Host(HostTtsProvider::Piper);

        // Simulate the update logic - retain removes nothing (provider not present)
        capabilities
            .providers
            .retain(|cap| cap.provider != provider);

        let new_capability = HostTtsCapability::new(provider).with_voice(Voice::new("PiperVoice"));
        capabilities.providers.push(new_capability);

        // Verify: Both providers present
        assert_eq!(capabilities.providers.len(), 2);
        assert!(
            capabilities
                .get_provider(&TtsProvider::Host(HostTtsProvider::Say))
                .is_some()
        );
        assert!(
            capabilities
                .get_provider(&TtsProvider::Host(HostTtsProvider::Piper))
                .is_some()
        );
    }

    // ========================================================================
    // bust_host_capability_cache tests
    // ========================================================================

    #[test]
    fn test_bust_cache_removes_file() {
        let dir = TempDir::new().unwrap();
        let cache_path = create_test_cache(&dir, "{}");
        assert!(cache_path.exists(), "Cache file should exist before bust");

        bust_cache_at(&cache_path).expect("Busting an existing cache should succeed");

        assert!(
            !cache_path.exists(),
            "Cache file should not exist after bust"
        );
    }

    #[test]
    fn test_bust_cache_no_error_if_missing() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(CACHE_FILE_NAME);
        assert!(!cache_path.exists());

        bust_cache_at(&cache_path).expect("Busting an absent cache should succeed");
    }

    // ========================================================================
    // write_cache_atomically tests
    // ========================================================================

    #[test]
    fn test_write_cache_atomically_creates_file() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(CACHE_FILE_NAME);

        let capabilities = HostTtsCapabilities::new()
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
                    .with_voice(Voice::new("Test")),
            )
            .with_timestamp(1234567890);

        write_cache_atomically(&cache_path, &capabilities).expect("Write should succeed");

        assert!(cache_path.exists(), "Cache file should be created");

        // Verify contents
        let contents = fs::read_to_string(&cache_path).unwrap();
        let envelope: CacheEnvelope = serde_json::from_str(&contents).unwrap();
        assert_eq!(envelope.schema_version, CACHE_SCHEMA_VERSION);
        assert_eq!(envelope.capabilities.providers.len(), 1);
    }

    #[test]
    fn test_write_cache_atomically_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(CACHE_FILE_NAME);

        // Write initial content
        let initial = HostTtsCapabilities::new().with_timestamp(1000);
        write_cache_atomically(&cache_path, &initial).unwrap();

        // Overwrite with new content
        let updated = HostTtsCapabilities::new()
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
                    .with_voice(Voice::new("Voice1"))
                    .with_voice(Voice::new("Voice2")),
            )
            .with_timestamp(2000);
        write_cache_atomically(&cache_path, &updated).unwrap();

        // Verify overwrite
        let contents = fs::read_to_string(&cache_path).unwrap();
        let envelope: CacheEnvelope = serde_json::from_str(&contents).unwrap();
        assert_eq!(envelope.capabilities.last_updated, 2000);
        assert_eq!(envelope.capabilities.providers.len(), 1);
        assert_eq!(envelope.capabilities.providers[0].voices.len(), 2);
    }

    #[test]
    fn test_write_cache_produces_pretty_json() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(CACHE_FILE_NAME);

        let capabilities = HostTtsCapabilities::new();
        write_cache_atomically(&cache_path, &capabilities).unwrap();

        let contents = fs::read_to_string(&cache_path).unwrap();
        // Pretty JSON has newlines and indentation
        assert!(contents.contains('\n'), "Should be pretty-printed");
        assert!(
            contents.contains("  "),
            "Should have indentation for readability"
        );
    }

    // ========================================================================
    // Integration tests - full read/write cycle
    // ========================================================================

    #[test]
    fn test_full_cache_cycle() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(CACHE_FILE_NAME);

        // Create capabilities with multiple providers
        let capabilities = HostTtsCapabilities::new()
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
                    .with_voice(
                        Voice::new("Samantha")
                            .with_gender(Gender::Female)
                            .with_quality(VoiceQuality::Excellent)
                            .with_language(Language::English)
                            .with_priority(10),
                    )
                    .with_voice(Voice::new("Alex").with_gender(Gender::Male))
                    .with_available_voice(Voice::new("Zoe")),
            )
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::ESpeak))
                    .with_voice(Voice::new("en-us")),
            )
            .with_timestamp(1704067200); // 2024-01-01 00:00:00 UTC

        // Write
        write_cache_atomically(&cache_path, &capabilities).unwrap();

        // Read back
        let loaded = read_cache_at(&cache_path).expect("Round-trip read should succeed");

        // Verify
        assert_eq!(loaded.providers.len(), 2);
        assert_eq!(loaded.last_updated, 1704067200);

        let say_cap = loaded
            .get_provider(&TtsProvider::Host(HostTtsProvider::Say))
            .expect("Say provider should exist");
        assert_eq!(say_cap.voices.len(), 2);
        assert_eq!(say_cap.available_voices.len(), 1);
        assert_eq!(say_cap.voices[0].name, "Samantha");
        assert_eq!(say_cap.voices[0].gender, Gender::Female);
        assert_eq!(say_cap.voices[0].quality, VoiceQuality::Excellent);
        assert_eq!(say_cap.voices[0].priority, 10);
    }

    // ========================================================================
    // Schema version tests
    // ========================================================================

    #[test]
    fn test_schema_version_constant() {
        assert_eq!(
            CACHE_SCHEMA_VERSION, 1,
            "Initial schema version should be 1"
        );
    }

    #[test]
    fn test_cache_envelope_includes_schema_version() {
        let capabilities = HostTtsCapabilities::new();
        let envelope = CacheEnvelope {
            schema_version: CACHE_SCHEMA_VERSION,
            capabilities,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        assert!(
            json.contains("\"schema_version\":1"),
            "JSON should contain schema_version field"
        );
    }

    // ========================================================================
    // Timestamp tests
    // ========================================================================

    #[test]
    fn test_timestamp_updated_on_write() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(CACHE_FILE_NAME);

        let mut capabilities = HostTtsCapabilities::new().with_timestamp(0);

        // Update timestamp (simulating what update_provider_in_cache does)
        capabilities.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        write_cache_atomically(&cache_path, &capabilities).unwrap();

        let contents = fs::read_to_string(&cache_path).unwrap();
        let envelope: CacheEnvelope = serde_json::from_str(&contents).unwrap();

        assert!(
            envelope.capabilities.last_updated > 0,
            "Timestamp should be set to current time"
        );
        // Timestamp should be a reasonable Unix epoch (after 2020)
        assert!(
            envelope.capabilities.last_updated > 1577836800,
            "Timestamp should be after 2020"
        );
    }

    // ========================================================================
    // populate_cache_for_provider tests
    // ========================================================================

    /// Mock provider for testing cache population
    struct MockVoiceProvider {
        voices: Vec<Voice>,
        should_fail: bool,
    }

    impl MockVoiceProvider {
        fn new(voices: Vec<Voice>) -> Self {
            Self {
                voices,
                should_fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                voices: vec![],
                should_fail: true,
            }
        }
    }

    impl crate::traits::TtsVoiceInventory for MockVoiceProvider {
        async fn list_voices(&self) -> Result<Vec<Voice>, crate::errors::TtsError> {
            if self.should_fail {
                Err(crate::errors::TtsError::VoiceEnumerationFailed {
                    provider: "mock".into(),
                    message: "intentional failure".into(),
                })
            } else {
                Ok(self.voices.clone())
            }
        }

        async fn default_voice(
            &self,
            _gender: crate::types::Gender,
        ) -> Result<Voice, crate::errors::TtsError> {
            Ok(Voice::new("MockDefault"))
        }
    }

    #[tokio::test]
    async fn test_populate_cache_for_provider_success() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(CACHE_FILE_NAME);

        let voices = vec![
            Voice::new("TestVoice1").with_gender(Gender::Female),
            Voice::new("TestVoice2").with_gender(Gender::Male),
        ];
        let provider = MockVoiceProvider::new(voices);
        let provider_type = TtsProvider::Host(HostTtsProvider::Piper);

        let result = populate_provider_at(&cache_path, &provider, provider_type).await;
        assert!(result.is_ok(), "populate_provider_at should succeed");

        // Verify the cache was updated
        let cache = read_cache_at(&cache_path).unwrap();
        let piper_cap = cache
            .get_provider(&TtsProvider::Host(HostTtsProvider::Piper))
            .expect("Piper should be in cache");

        assert_eq!(piper_cap.voices.len(), 2);
        assert_eq!(piper_cap.voices[0].name, "TestVoice1");
        assert_eq!(piper_cap.voices[1].name, "TestVoice2");
    }

    #[tokio::test]
    async fn test_populate_cache_for_provider_failure() {
        use super::populate_cache_for_provider;

        let provider = MockVoiceProvider::failing();
        let provider_type = TtsProvider::Host(HostTtsProvider::Piper);

        let result = populate_cache_for_provider(&provider, provider_type).await;
        assert!(result.is_err(), "populate_cache_for_provider should fail");

        match result {
            Err(crate::errors::TtsError::VoiceEnumerationFailed { provider, .. }) => {
                assert_eq!(provider, "mock");
            }
            _ => panic!("Expected VoiceEnumerationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_populate_cache_for_provider_updates_existing() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(CACHE_FILE_NAME);

        // First populate with some voices
        let initial_voices = vec![Voice::new("OldVoice")];
        let provider1 = MockVoiceProvider::new(initial_voices);
        let provider_type = TtsProvider::Host(HostTtsProvider::Festival);

        populate_provider_at(&cache_path, &provider1, provider_type)
            .await
            .unwrap();

        // Now update with new voices
        let new_voices = vec![Voice::new("NewVoice1"), Voice::new("NewVoice2")];
        let provider2 = MockVoiceProvider::new(new_voices);

        populate_provider_at(&cache_path, &provider2, provider_type)
            .await
            .unwrap();

        // Verify the cache was updated (not appended)
        let cache = read_cache_at(&cache_path).unwrap();
        let festival_cap = cache
            .get_provider(&TtsProvider::Host(HostTtsProvider::Festival))
            .expect("Festival should be in cache");

        assert_eq!(
            festival_cap.voices.len(),
            2,
            "Should have 2 new voices, not 3"
        );
        assert_eq!(festival_cap.voices[0].name, "NewVoice1");
        assert_eq!(festival_cap.voices[1].name, "NewVoice2");
    }

    // ========================================================================
    // populate_cache_for_all_providers tests
    // ========================================================================

    // Note: populate_cache_for_all_providers is tested implicitly through
    // integration tests since it depends on actual system providers.
    // We test the individual populate_cache_for_provider function above.

    #[tokio::test]
    async fn test_populate_cache_for_all_providers_runs_without_panic() {
        let dir = TempDir::new().unwrap();
        let cache_path = dir.path().join(CACHE_FILE_NAME);

        // This test just verifies the function runs without panicking.
        // The actual result depends on which providers are installed on the system.
        let _ = populate_all_at(&cache_path).await;

        // If any providers were available, the cache should exist
        // We don't assert on this since it depends on the test environment
    }
}
