//! Test-only seam (`cache-test-hooks`): writes a transport-cache entry the
//! way a successful fetch by `md compose --cache-root <root>` stores one, so
//! a consumer suite can prove an editor observation answers from a fresh
//! entry, and expires with it, without any network access.

use std::path::Path;
use std::time::{Duration, SystemTime};

use biscuit_hash::xx_hash;

use super::cache::FileStore;
use super::cache::manifest::{CACHE_VERSION, RemoteUrlManifest, redact_url};
use super::cache::types::ArtifactClass;

/// Stores `body` as the cached response for `url` in the transport store
/// under `cache_root`, fetched a second before `fetched_at` and expiring at
/// `expires_at`.
///
/// ## Errors
///
/// The store's write failure.
pub fn seed_remote_entry(
    cache_root: &Path,
    url: &url::Url,
    body: &str,
    fetched_at: SystemTime,
    expires_at: SystemTime,
) -> std::io::Result<()> {
    let store = FileStore::at(FileStore::resolve_cache_root(cache_root, None));
    let content_hash = xx_hash(body);
    let manifest = RemoteUrlManifest {
        cache_version: CACHE_VERSION,
        redacted_url: redact_url(url),
        source_id_hash: xx_hash(url.as_str()),
        status: 200,
        etag: None,
        last_modified: None,
        cache_control: Some("max-age=60".to_string()),
        fetched_at: fetched_at - Duration::from_secs(1),
        expires_at: Some(expires_at),
        content_hash,
        body_blob_hash: content_hash,
        size_bytes: body.len() as u64,
    };
    store
        .write_artifact(ArtifactClass::RemoteUrl, xx_hash(url.as_str()), &manifest, body.as_bytes(), content_hash, "remote")
        .map(|_| ())
        .map_err(|error| std::io::Error::other(error.to_string()))
}
