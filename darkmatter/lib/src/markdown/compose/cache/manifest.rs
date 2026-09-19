//! Manifest types for the persistent remote transport cache.
//!
//! Each cached remote body has a JSON manifest recording its provenance,
//! HTTP validators, and freshness. No semantic-result manifest (composed
//! document, operation result, document snapshot) exists: those artifacts are
//! never persisted until a `ContentPolicy` defines their freshness.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use url::Url;

/// Current manifest schema version. Bump when a manifest's shape changes.
///
/// A manifest recording any other version is a cache miss. This is distinct
/// from [`STORE_LAYOUT_VERSION`], so a manifest-shape change keeps old entries
/// at the same paths, where they can still be found and purged.
///
/// Version history: `1` persisted the cleartext request URL; `2` persists
/// only [`redact_url`]'s diagnostic form.
pub const CACHE_VERSION: u16 = 2;

/// On-disk directory layout version, embedded in the store root as `v{N}`.
///
/// Bump only when the `manifests/` / `blobs/` path scheme changes.
pub const STORE_LAYOUT_VERSION: u16 = 1;

/// Manifest for a cached remote URL artifact.
///
/// Stores the fetched HTTP(S) response body (as a blob) alongside the
/// freshness metadata needed for TTL evaluation and conditional revalidation
/// (`If-None-Match` / `If-Modified-Since`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteUrlManifest {
    /// Cache format version for forward compatibility.
    pub cache_version: u16,
    /// Diagnostic form of the request URL from [`redact_url`]. Never the raw
    /// URL: manifests are persisted, and a URL can carry credentials.
    pub redacted_url: String,
    /// xxHash of the full, unredacted URL string (also the manifest entry
    /// key), so URLs differing only in userinfo or query never collide.
    pub source_id_hash: u64,
    /// HTTP status code from the response that produced the cached body.
    pub status: u16,
    /// `ETag` header from the cached response, if present.
    pub etag: Option<String>,
    /// `Last-Modified` header from the cached response, if present.
    pub last_modified: Option<String>,
    /// `Cache-Control` header from the cached response, if present.
    pub cache_control: Option<String>,
    /// When the cached body was last fetched or revalidated.
    pub fetched_at: SystemTime,
    /// When the cached body is considered stale (`None` = no TTL known).
    pub expires_at: Option<SystemTime>,
    /// xxHash of the response body bytes.
    pub content_hash: u64,
    /// Blob hash used to locate the stored body (equals `content_hash`).
    pub body_blob_hash: u64,
    /// Size of the cached body in bytes.
    pub size_bytes: u64,
}

/// The fields of a remote manifest that every [`CACHE_VERSION`] shares.
///
/// Read before the full manifest so a legacy entry that no longer
/// deserializes as [`RemoteUrlManifest`] can still be recognized as
/// `no-store` and purged.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteUrlManifestHeader {
    /// Manifest schema version the entry was written with.
    pub cache_version: u16,
    /// `Cache-Control` header recorded with the entry, if any.
    pub cache_control: Option<String>,
    /// Blob hash locating the stored body.
    pub body_blob_hash: u64,
}

/// Returns the form of `url` that may be persisted: `scheme://host[:port]/path`,
/// with userinfo and fragment removed and any query replaced by `?<redacted>`.
///
/// Redaction applies to persisted artifacts only. Errors and warnings shown to
/// the user may name the URL they supplied; a path segment can still carry a
/// token, which this form does not hide.
pub fn redact_url(url: &Url) -> String {
    let mut base = url.clone();
    // Both fail only for URLs that cannot carry userinfo, which have none.
    let _ = base.set_username("");
    let _ = base.set_password(None);
    base.set_query(None);
    base.set_fragment(None);
    let mut redacted = base.to_string();
    if url.query().is_some() {
        redacted.push_str("?<redacted>");
    }
    redacted
}

impl RemoteUrlManifest {
    /// Returns `true` if the cached body is still within its TTL at `now`.
    ///
    /// A manifest with no `expires_at` is never considered fresh, forcing
    /// revalidation under `Strict`/`Fallback` modes.
    pub fn is_fresh(&self, now: SystemTime) -> bool {
        self.expires_at.map(|exp| now < exp).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_url_strips_userinfo_query_and_fragment() {
        let cases = [
            (
                "http://user:secret@127.0.0.1:8080/doc.md?token=abc",
                "http://127.0.0.1:8080/doc.md?<redacted>",
            ),
            ("https://user@example.com/a/b.md#frag", "https://example.com/a/b.md"),
            ("https://:pw@example.com/doc.md", "https://example.com/doc.md"),
            ("https://example.com/doc.md?", "https://example.com/doc.md?<redacted>"),
            ("https://example.com:443/doc.md", "https://example.com/doc.md"),
            ("https://example.com", "https://example.com/"),
            ("http://[::1]:9000/%7Etoken/doc.md?x=1#y", "http://[::1]:9000/%7Etoken/doc.md?<redacted>"),
        ];
        for (raw, expected) in cases {
            let url = Url::parse(raw).unwrap();
            assert_eq!(redact_url(&url), expected, "{raw}");
        }
    }

    #[test]
    fn remote_manifest_roundtrips_without_a_raw_url() {
        let url = Url::parse("https://user:secret@example.com/doc.md?token=abc").unwrap();
        let manifest = RemoteUrlManifest {
            cache_version: CACHE_VERSION,
            redacted_url: redact_url(&url),
            source_id_hash: 1,
            status: 200,
            etag: Some("\"e1\"".to_string()),
            last_modified: None,
            cache_control: Some("max-age=60".to_string()),
            fetched_at: SystemTime::UNIX_EPOCH,
            expires_at: None,
            content_hash: 2,
            body_blob_hash: 2,
            size_bytes: 3,
        };
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(!json.contains("\"url\""), "{json}");
        assert!(!json.contains("secret") && !json.contains("token"), "{json}");

        let again: RemoteUrlManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_string(&again).unwrap(), json);
        let header: RemoteUrlManifestHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(header.cache_version, CACHE_VERSION);
        assert_eq!(header.body_blob_hash, 2);
    }

    /// The header is what lets a v1 entry be recognized as `no-store`: the
    /// full v2 manifest cannot deserialize one at all.
    #[test]
    fn v1_remote_manifest_parses_only_as_a_header() {
        let v1 = r#"{
            "cache_version": 1,
            "url": "https://user:secret@example.com/doc.md?token=abc",
            "source_id_hash": 1,
            "status": 200,
            "etag": null,
            "last_modified": null,
            "cache_control": "no-store",
            "fetched_at": {"secs_since_epoch": 0, "nanos_since_epoch": 0},
            "expires_at": null,
            "content_hash": 2,
            "body_blob_hash": 2,
            "size_bytes": 3
        }"#;
        assert!(serde_json::from_str::<RemoteUrlManifest>(v1).is_err());
        let header: RemoteUrlManifestHeader = serde_json::from_str(v1).unwrap();
        assert_eq!(header.cache_version, 1);
        assert_eq!(header.cache_control.as_deref(), Some("no-store"));
        assert_eq!(header.body_blob_hash, 2);
    }
}
