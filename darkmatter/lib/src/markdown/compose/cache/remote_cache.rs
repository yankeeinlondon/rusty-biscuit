//! Persistent cache and freshness logic for remote URL artifacts.
//!
//! Wraps the shared `biscuit_file` fetch primitive with TTL evaluation,
//! conditional revalidation (`If-None-Match` / `If-Modified-Since`), and
//! stale-on-failure behavior driven by [`RemoteFreshnessMode`].
//!
//! ## Freshness modes
//!
//! - [`RemoteFreshnessMode::Optimistic`] — serve any cached body without
//!   revalidation, ignoring TTL.
//! - [`RemoteFreshnessMode::Strict`] — serve within TTL without network;
//!   revalidate with a conditional GET past TTL.
//! - [`RemoteFreshnessMode::Fallback`] — like `Strict`, but serve the stale
//!   cached body when revalidation fails on the network.
//!
//! `refresh` forces a conditional GET even when the cached body is fresh.

use std::time::{Duration, SystemTime};

use biscuit_file::file_reference::fetch::{Conditional, FetchPolicy, fetch};
use biscuit_hash::xx_hash;
use url::Url;

use super::manifest::{CACHE_VERSION, RemoteUrlManifest};
use super::store::FileStore;
use super::types::ArtifactClass;
use crate::markdown::compose::remote::RemoteFreshnessMode;

/// File extension used for remote response-body blobs.
const REMOTE_BLOB_EXT: &str = "remote";

/// Freshness inputs that govern a single remote fetch.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RemoteCacheConfig {
    /// Explicit TTL override (`None` = derive from `Cache-Control`).
    pub ttl_override: Option<Duration>,
    /// Force a conditional GET even when the cached body is fresh.
    pub refresh: bool,
    /// Staleness tolerance mode.
    pub mode: RemoteFreshnessMode,
}

/// What happened during a remote fetch, used to classify statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteOutcomeEvent {
    /// Body came from the network with a fresh `200` response.
    Fetched,
    /// Body came from the persistent cache without any network access.
    CacheHit,
    /// A conditional GET returned `304`; the cached body was preserved.
    NotModified,
    /// The network failed and a stale cached body was served (`Fallback`).
    StaleServed,
}

/// Result of [`fetch_with_cache`].
#[derive(Debug, Clone)]
pub(crate) struct RemoteFetchOutcome {
    /// The response body as UTF-8 text.
    pub body: String,
    /// xxHash of the body bytes (closure-hash dependency value).
    pub content_hash: u64,
    /// Classification of how the body was obtained.
    pub event: RemoteOutcomeEvent,
    /// Whether a conditional GET was issued (revalidation attempt).
    pub revalidated: bool,
}

/// Parses a `max-age` (in seconds) out of a `Cache-Control` header value.
///
/// Returns `Some(Duration::ZERO)` for `no-cache`/`no-store` so such responses
/// are always treated as immediately stale.
pub(crate) fn parse_max_age(cache_control: &str) -> Option<Duration> {
    for directive in cache_control.split(',') {
        let directive = directive.trim();
        let lower = directive.to_ascii_lowercase();
        if lower == "no-cache" || lower == "no-store" {
            return Some(Duration::ZERO);
        }
        if let Some(value) = lower.strip_prefix("max-age=")
            && let Ok(secs) = value.trim().parse::<u64>()
        {
            return Some(Duration::from_secs(secs));
        }
    }
    None
}

/// Computes an `expires_at` instant from the TTL override or `Cache-Control`.
///
/// Precedence: explicit override → `Cache-Control: max-age` → `None`.
pub(crate) fn compute_expires_at(
    now: SystemTime,
    ttl_override: Option<Duration>,
    cache_control: Option<&str>,
) -> Option<SystemTime> {
    if let Some(ttl) = ttl_override {
        return Some(now + ttl);
    }
    cache_control
        .and_then(parse_max_age)
        .map(|max_age| now + max_age)
}

/// What to do given the cache state and freshness configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheAction {
    /// Serve the cached body without any network access.
    ServeCache,
    /// Issue a conditional GET to revalidate the cached body.
    Revalidate,
    /// No usable cache entry — issue an unconditional GET.
    FetchFresh,
}

/// Decides the fetch action from cache presence, freshness, and mode.
fn decide_action(
    cached_present: bool,
    within_ttl: bool,
    cfg: &RemoteCacheConfig,
) -> CacheAction {
    if !cached_present {
        return CacheAction::FetchFresh;
    }
    if cfg.refresh {
        return CacheAction::Revalidate;
    }
    match cfg.mode {
        RemoteFreshnessMode::Optimistic => CacheAction::ServeCache,
        RemoteFreshnessMode::Strict | RemoteFreshnessMode::Fallback => {
            if within_ttl {
                CacheAction::ServeCache
            } else {
                CacheAction::Revalidate
            }
        }
    }
}

/// Reads a cached manifest and its body blob, if both are present.
fn read_cached(store: &FileStore, key: u64) -> Option<(RemoteUrlManifest, String)> {
    let manifest: RemoteUrlManifest = store
        .read_manifest(ArtifactClass::RemoteUrl, key)
        .ok()
        .flatten()?;
    let bytes = store
        .read_blob(manifest.body_blob_hash, REMOTE_BLOB_EXT)
        .ok()
        .flatten()?;
    let body = String::from_utf8(bytes).ok()?;
    Some((manifest, body))
}

/// Writes a remote artifact (manifest + body blob) to the store.
#[allow(clippy::too_many_arguments)]
fn write_cached(
    store: &FileStore,
    key: u64,
    url: &Url,
    body: &str,
    content_hash: u64,
    status: u16,
    etag: Option<String>,
    last_modified: Option<String>,
    cache_control: Option<String>,
    now: SystemTime,
    ttl_override: Option<Duration>,
) {
    let expires_at = compute_expires_at(now, ttl_override, cache_control.as_deref());
    let manifest = RemoteUrlManifest {
        cache_version: CACHE_VERSION,
        url: url.to_string(),
        source_id_hash: key,
        status,
        etag,
        last_modified,
        cache_control,
        fetched_at: now,
        expires_at,
        content_hash,
        body_blob_hash: content_hash,
        size_bytes: body.len() as u64,
    };
    let _ = store.write_artifact(
        ArtifactClass::RemoteUrl,
        key,
        &manifest,
        body.as_bytes(),
        content_hash,
        REMOTE_BLOB_EXT,
    );
}

/// Fetches `url`, consulting and updating the persistent remote cache.
///
/// When `store` is `None`, performs a plain network fetch with no caching.
///
/// ## Errors
///
/// Returns an error string when the network fetch fails and no usable cached
/// body is available under the active freshness mode, or when the response
/// body is not valid UTF-8.
pub(crate) async fn fetch_with_cache(
    store: Option<&FileStore>,
    client: &reqwest::Client,
    url: &Url,
    policy: &FetchPolicy,
    cfg: &RemoteCacheConfig,
) -> Result<RemoteFetchOutcome, String> {
    // No persistent store: plain network fetch, no caching.
    let Some(store) = store else {
        let resp = fetch(client, url, policy, &Conditional::default())
            .await
            .map_err(|e| e.to_string())?;
        let body = String::from_utf8(resp.body.to_vec())
            .map_err(|e| format!("invalid UTF-8 in response body: {e}"))?;
        let content_hash = xx_hash(&body);
        return Ok(RemoteFetchOutcome {
            body,
            content_hash,
            event: RemoteOutcomeEvent::Fetched,
            revalidated: false,
        });
    };

    let key = xx_hash(url.as_str());
    let now = SystemTime::now();
    let cached = read_cached(store, key);
    let within_ttl = cached
        .as_ref()
        .map(|(m, _)| m.is_fresh(now))
        .unwrap_or(false);

    match decide_action(cached.is_some(), within_ttl, cfg) {
        CacheAction::ServeCache => {
            let (manifest, body) = cached.expect("ServeCache implies a cache entry");
            Ok(RemoteFetchOutcome {
                body,
                content_hash: manifest.content_hash,
                event: RemoteOutcomeEvent::CacheHit,
                revalidated: false,
            })
        }
        CacheAction::Revalidate => {
            let (manifest, cached_body) =
                cached.expect("Revalidate implies a cache entry");
            let conditional = Conditional {
                if_none_match: manifest.etag.clone(),
                if_modified_since: manifest.last_modified.clone(),
            };
            match fetch(client, url, policy, &conditional).await {
                Ok(resp) if resp.is_not_modified() => {
                    // Preserve the cached body; refresh freshness metadata.
                    write_cached(
                        store,
                        key,
                        url,
                        &cached_body,
                        manifest.content_hash,
                        manifest.status,
                        resp.etag.or(manifest.etag),
                        resp.last_modified.or(manifest.last_modified),
                        resp.cache_control.or(manifest.cache_control),
                        now,
                        cfg.ttl_override,
                    );
                    Ok(RemoteFetchOutcome {
                        body: cached_body,
                        content_hash: manifest.content_hash,
                        event: RemoteOutcomeEvent::NotModified,
                        revalidated: true,
                    })
                }
                Ok(resp) => {
                    // Fresh 200: replace the cached body and manifest.
                    let body = String::from_utf8(resp.body.to_vec())
                        .map_err(|e| format!("invalid UTF-8 in response body: {e}"))?;
                    let content_hash = xx_hash(&body);
                    write_cached(
                        store,
                        key,
                        url,
                        &body,
                        content_hash,
                        resp.status,
                        resp.etag,
                        resp.last_modified,
                        resp.cache_control,
                        now,
                        cfg.ttl_override,
                    );
                    Ok(RemoteFetchOutcome {
                        body,
                        content_hash,
                        event: RemoteOutcomeEvent::Fetched,
                        revalidated: true,
                    })
                }
                Err(err) => {
                    // Network/HTTP failure during revalidation.
                    if cfg.mode == RemoteFreshnessMode::Fallback {
                        Ok(RemoteFetchOutcome {
                            body: cached_body,
                            content_hash: manifest.content_hash,
                            event: RemoteOutcomeEvent::StaleServed,
                            revalidated: true,
                        })
                    } else {
                        Err(err.to_string())
                    }
                }
            }
        }
        CacheAction::FetchFresh => {
            let resp = fetch(client, url, policy, &Conditional::default())
                .await
                .map_err(|e| e.to_string())?;
            let body = String::from_utf8(resp.body.to_vec())
                .map_err(|e| format!("invalid UTF-8 in response body: {e}"))?;
            let content_hash = xx_hash(&body);
            write_cached(
                store,
                key,
                url,
                &body,
                content_hash,
                resp.status,
                resp.etag,
                resp.last_modified,
                resp.cache_control,
                now,
                cfg.ttl_override,
            );
            Ok(RemoteFetchOutcome {
                body,
                content_hash,
                event: RemoteOutcomeEvent::Fetched,
                revalidated: false,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_max_age_extracts_seconds() {
        assert_eq!(parse_max_age("max-age=300"), Some(Duration::from_secs(300)));
        assert_eq!(
            parse_max_age("public, max-age=60"),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            parse_max_age("max-age=0, must-revalidate"),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn parse_max_age_no_cache_is_zero() {
        assert_eq!(parse_max_age("no-cache"), Some(Duration::ZERO));
        assert_eq!(parse_max_age("no-store"), Some(Duration::ZERO));
    }

    #[test]
    fn parse_max_age_absent_returns_none() {
        assert_eq!(parse_max_age("public"), None);
        assert_eq!(parse_max_age(""), None);
    }

    #[test]
    fn expires_at_prefers_override() {
        let now = SystemTime::UNIX_EPOCH;
        let exp = compute_expires_at(now, Some(Duration::from_secs(10)), Some("max-age=999"));
        assert_eq!(exp, Some(now + Duration::from_secs(10)));
    }

    #[test]
    fn expires_at_falls_back_to_cache_control() {
        let now = SystemTime::UNIX_EPOCH;
        let exp = compute_expires_at(now, None, Some("max-age=42"));
        assert_eq!(exp, Some(now + Duration::from_secs(42)));
    }

    #[test]
    fn expires_at_none_without_ttl_or_header() {
        assert_eq!(compute_expires_at(SystemTime::UNIX_EPOCH, None, None), None);
    }

    fn cfg(mode: RemoteFreshnessMode, refresh: bool) -> RemoteCacheConfig {
        RemoteCacheConfig {
            ttl_override: None,
            refresh,
            mode,
        }
    }

    #[test]
    fn decide_no_cache_fetches_fresh() {
        for mode in [
            RemoteFreshnessMode::Optimistic,
            RemoteFreshnessMode::Strict,
            RemoteFreshnessMode::Fallback,
        ] {
            assert_eq!(
                decide_action(false, false, &cfg(mode, false)),
                CacheAction::FetchFresh
            );
        }
    }

    #[test]
    fn decide_refresh_always_revalidates() {
        for mode in [
            RemoteFreshnessMode::Optimistic,
            RemoteFreshnessMode::Strict,
            RemoteFreshnessMode::Fallback,
        ] {
            assert_eq!(
                decide_action(true, true, &cfg(mode, true)),
                CacheAction::Revalidate
            );
        }
    }

    #[test]
    fn decide_optimistic_serves_cache_even_when_stale() {
        assert_eq!(
            decide_action(true, false, &cfg(RemoteFreshnessMode::Optimistic, false)),
            CacheAction::ServeCache
        );
    }

    #[test]
    fn decide_strict_serves_within_ttl_revalidates_past() {
        assert_eq!(
            decide_action(true, true, &cfg(RemoteFreshnessMode::Strict, false)),
            CacheAction::ServeCache
        );
        assert_eq!(
            decide_action(true, false, &cfg(RemoteFreshnessMode::Strict, false)),
            CacheAction::Revalidate
        );
    }

    #[test]
    fn decide_fallback_matches_strict_for_action() {
        assert_eq!(
            decide_action(true, true, &cfg(RemoteFreshnessMode::Fallback, false)),
            CacheAction::ServeCache
        );
        assert_eq!(
            decide_action(true, false, &cfg(RemoteFreshnessMode::Fallback, false)),
            CacheAction::Revalidate
        );
    }
}
