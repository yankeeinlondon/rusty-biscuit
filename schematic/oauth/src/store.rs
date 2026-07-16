//! Token storage trait and built-in implementations.
//!
//! Provides [`TokenStore`] for persisting OAuth2 tokens, along with
//! [`MemoryTokenStore`] for ephemeral use and [`FileTokenStore`] for
//! persisting tokens across process restarts.

use crate::error::OAuthError;
use crate::types::StoredTokens;

/// Trait for persisting OAuth2 tokens.
///
/// Implementations must be `Send + Sync` to support use inside
/// async runtimes and shared state.
pub trait TokenStore: Send + Sync {
    /// Load previously stored tokens, returning `None` if no tokens exist.
    fn load(&self) -> Result<Option<StoredTokens>, OAuthError>;

    /// Persist the given tokens.
    fn save(&self, tokens: &StoredTokens) -> Result<(), OAuthError>;

    /// Remove all stored tokens.
    fn clear(&self) -> Result<(), OAuthError>;
}

/// In-memory token store for testing and short-lived processes.
///
/// Tokens are lost when the store is dropped.
pub struct MemoryTokenStore {
    tokens: std::sync::Mutex<Option<StoredTokens>>,
}

impl MemoryTokenStore {
    /// Creates a new empty in-memory token store.
    pub fn new() -> Self {
        Self {
            tokens: std::sync::Mutex::new(None),
        }
    }
}

impl Default for MemoryTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenStore for MemoryTokenStore {
    fn load(&self) -> Result<Option<StoredTokens>, OAuthError> {
        let guard = self
            .tokens
            .lock()
            .map_err(|e| OAuthError::TokenStore(e.to_string()))?;
        Ok(guard.clone())
    }

    fn save(&self, tokens: &StoredTokens) -> Result<(), OAuthError> {
        let mut guard = self
            .tokens
            .lock()
            .map_err(|e| OAuthError::TokenStore(e.to_string()))?;
        *guard = Some(tokens.clone());
        Ok(())
    }

    fn clear(&self) -> Result<(), OAuthError> {
        let mut guard = self
            .tokens
            .lock()
            .map_err(|e| OAuthError::TokenStore(e.to_string()))?;
        *guard = None;
        Ok(())
    }
}

/// File-based token store for persisting tokens across process restarts.
///
/// Tokens are stored as JSON at the specified path. Parent directories
/// are created automatically on save.
///
/// ## Notes
///
/// Because the file holds plaintext bearer credentials, saves are hardened:
/// the write is atomic (temp file + rename) so readers never observe a partial
/// file, and on Unix the token file is created `0o600` and its parent directory
/// `0o700` so other local users cannot read it. On Windows there are no POSIX
/// mode bits; confidentiality relies on the ACLs of the per-user profile
/// directory the path lives under.
pub struct FileTokenStore {
    path: std::path::PathBuf,
}

impl FileTokenStore {
    /// Creates a new file-based token store at the given path.
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Builds a unique sibling temp path for the atomic write.
    ///
    /// The suffix combines pid, a monotonically increasing counter, and a
    /// nanosecond timestamp so concurrent writers in this or another process
    /// never share a temp file.
    fn temp_path(&self) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();

        let mut file_name = self
            .path
            .file_name()
            .map(std::ffi::OsStr::to_os_string)
            .unwrap_or_default();
        file_name.push(format!(".tmp.{pid}.{nanos}.{counter}"));

        match self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            Some(dir) => dir.join(file_name),
            None => std::path::PathBuf::from(file_name),
        }
    }
}

/// Creates `dir` (and missing parents) with owner-only permissions on Unix.
#[cfg(unix)]
fn create_dir_secure(dir: &std::path::Path) -> Result<(), OAuthError> {
    use std::os::unix::fs::DirBuilderExt;

    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(|e| OAuthError::TokenStore(format!("Failed to create directory: {e}")))
}

#[cfg(not(unix))]
fn create_dir_secure(dir: &std::path::Path) -> Result<(), OAuthError> {
    std::fs::create_dir_all(dir)
        .map_err(|e| OAuthError::TokenStore(format!("Failed to create directory: {e}")))
}

/// Writes `bytes` to `path`, creating the file `0o600` on Unix.
#[cfg(unix)]
fn write_secure(path: &std::path::Path, bytes: &[u8]) -> Result<(), OAuthError> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| OAuthError::TokenStore(format!("Failed to create token file: {e}")))?;
    file.write_all(bytes)
        .map_err(|e| OAuthError::TokenStore(format!("Failed to write token file: {e}")))
}

#[cfg(not(unix))]
fn write_secure(path: &std::path::Path, bytes: &[u8]) -> Result<(), OAuthError> {
    std::fs::write(path, bytes)
        .map_err(|e| OAuthError::TokenStore(format!("Failed to write token file: {e}")))
}

impl TokenStore for FileTokenStore {
    fn load(&self) -> Result<Option<StoredTokens>, OAuthError> {
        match std::fs::read_to_string(&self.path) {
            Ok(contents) => {
                let tokens: StoredTokens = serde_json::from_str(&contents).map_err(|e| {
                    OAuthError::TokenStore(format!("Failed to parse token file: {e}"))
                })?;
                Ok(Some(tokens))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(OAuthError::TokenStore(format!(
                "Failed to read token file: {e}"
            ))),
        }
    }

    fn save(&self, tokens: &StoredTokens) -> Result<(), OAuthError> {
        let json = serde_json::to_string_pretty(tokens)
            .map_err(|e| OAuthError::TokenStore(format!("Failed to serialize tokens: {e}")))?;

        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            create_dir_secure(parent)?;
        }

        // Write to a uniquely-named temp file in the same directory, then rename
        // over the target. Rename is atomic on the same filesystem, so a reader
        // sees either the old or the new file, and the unique suffix keeps
        // concurrent writers from clobbering a shared temp file.
        let temp_path = self.temp_path();
        write_secure(&temp_path, json.as_bytes())?;

        std::fs::rename(&temp_path, &self.path).map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            OAuthError::TokenStore(format!("Failed to persist token file: {e}"))
        })?;

        Ok(())
    }

    fn clear(&self) -> Result<(), OAuthError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(OAuthError::TokenStore(format!(
                "Failed to remove token file: {e}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_roundtrip() {
        let store = MemoryTokenStore::new();

        // Initially empty
        assert!(store.load().unwrap().is_none());

        // Save and reload
        let tokens = StoredTokens {
            access_token: "access_abc".into(),
            refresh_token: Some("refresh_xyz".into()),
            expires_at: None,
            scopes: vec!["read".into()],
        };
        store.save(&tokens).unwrap();

        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.access_token, "access_abc");
        assert_eq!(loaded.refresh_token.as_deref(), Some("refresh_xyz"));
        assert_eq!(loaded.scopes, vec!["read"]);

        // Clear
        store.clear().unwrap();
        assert!(store.load().unwrap().is_none());
    }

    #[test]
    fn file_store_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tokens.json");
        let store = FileTokenStore::new(&path);

        // Initially empty (file does not exist)
        assert!(store.load().unwrap().is_none());

        // Save and reload
        let tokens = StoredTokens {
            access_token: "file_access".into(),
            refresh_token: Some("file_refresh".into()),
            expires_at: Some(chrono::Utc::now()),
            scopes: vec!["write".into()],
        };
        store.save(&tokens).unwrap();

        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.access_token, "file_access");
        assert_eq!(loaded.refresh_token.as_deref(), Some("file_refresh"));

        // Clear
        store.clear().unwrap();
        assert!(store.load().unwrap().is_none());
    }

    #[test]
    fn file_store_handles_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let store = FileTokenStore::new(&path);

        assert!(store.load().unwrap().is_none());
    }

    #[test]
    fn file_store_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("tokens.json");
        let store = FileTokenStore::new(&path);

        let tokens = StoredTokens {
            access_token: "nested_token".into(),
            refresh_token: None,
            expires_at: None,
            scopes: vec![],
        };
        store.save(&tokens).unwrap();

        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.access_token, "nested_token");
    }

    #[cfg(unix)]
    #[test]
    fn file_store_uses_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let token_dir = dir.path().join("secrets");
        let path = token_dir.join("tokens.json");
        let store = FileTokenStore::new(&path);

        store
            .save(&StoredTokens {
                access_token: "perm_token".into(),
                refresh_token: None,
                expires_at: None,
                scopes: vec![],
            })
            .unwrap();

        let file_mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(file_mode, 0o600, "token file should be owner read/write only");

        let dir_mode = std::fs::metadata(&token_dir)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700, "token directory should be owner-only");
    }

    #[test]
    fn file_store_clear_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tokens.json");
        let store = FileTokenStore::new(&path);

        // Clearing a nonexistent file should succeed
        store.clear().unwrap();
        store.clear().unwrap();
    }
}
