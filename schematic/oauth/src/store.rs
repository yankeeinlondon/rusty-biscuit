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
pub struct FileTokenStore {
    path: std::path::PathBuf,
}

impl FileTokenStore {
    /// Creates a new file-based token store at the given path.
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }
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
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| OAuthError::TokenStore(format!("Failed to create directory: {e}")))?;
        }
        std::fs::write(&self.path, json)
            .map_err(|e| OAuthError::TokenStore(format!("Failed to write token file: {e}")))?;
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
