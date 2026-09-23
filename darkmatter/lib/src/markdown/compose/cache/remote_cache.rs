//! Persistent transport cache and freshness logic for remote URL artifacts.
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
//!
//! ## `Cache-Control` precedence (RFC 9111)
//!
//! Response directives outrank every mode and the TTL override: a `no-store`
//! response is never written (and an entry already recording it is purged
//! instead of served), and a `no-cache` entry is revalidated before every
//! reuse — including under `Optimistic` — with no stale serve under
//! `Fallback`. `biscuit_file`'s fetch joins every `Cache-Control` field line,
//! so a directive on any line counts.

use std::time::{Duration, SystemTime};

use biscuit_file::file_reference::fetch::{Conditional, FetchPolicy, PolicyClient, fetch};
use biscuit_hash::xx_hash;
use url::Url;

use super::manifest::{CACHE_VERSION, RemoteUrlManifest, RemoteUrlManifestHeader, redact_url};
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
    /// Classification of how the body was obtained.
    pub event: RemoteOutcomeEvent,
    /// Whether a conditional GET was issued (revalidation attempt).
    pub revalidated: bool,
}

/// Upper bound for a `max-age` delta, per RFC 9111 §1.2.2 (2^31 seconds).
const MAX_DELTA_SECONDS: u64 = 1 << 31;

/// The `Cache-Control` response directives this private transport cache obeys.
///
/// `max_age == Some(ZERO)` is a genuine zero freshness lifetime (store, then
/// revalidate before reuse) and is deliberately distinct from `no_store`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ResponseDirectives {
    /// `no-store`: the response must never be written or reused.
    pub no_store: bool,
    /// `no-cache` (including the `no-cache="field"` form): reuse requires a
    /// successful revalidation, under every freshness mode.
    pub no_cache: bool,
    /// First valid `max-age`, clamped to [`MAX_DELTA_SECONDS`].
    pub max_age: Option<Duration>,
}

impl ResponseDirectives {
    /// Parses every directive of a `Cache-Control` value.
    ///
    /// Names are case-insensitive, quoted values may contain commas, and a
    /// `max-age` value may be quoted. An unparsable `max-age` is ignored,
    /// which leaves the entry without a freshness lifetime (stale).
    pub fn parse(cache_control: Option<&str>) -> Self {
        let mut directives = Self::default();
        let Some(cache_control) = cache_control else {
            return directives;
        };
        for directive in split_directives(cache_control) {
            let (name, value) = match directive.split_once('=') {
                Some((name, value)) => (name.trim(), Some(value.trim())),
                None => (directive.trim(), None),
            };
            if name.eq_ignore_ascii_case("no-store") {
                directives.no_store = true;
            } else if name.eq_ignore_ascii_case("no-cache") {
                directives.no_cache = true;
            } else if name.eq_ignore_ascii_case("max-age") && directives.max_age.is_none() {
                directives.max_age = value.and_then(parse_delta_seconds);
            }
        }
        directives
    }

    /// Returns the directives as storable, or `None` for a `no-store`
    /// response — the only way to obtain a [`StorableDirectives`].
    pub fn storable(self) -> Option<StorableDirectives> {
        (!self.no_store).then_some(StorableDirectives(self))
    }
}

/// Directives proven not to carry `no-store`.
///
/// The write path accepts only this type, so no TTL override or later edit
/// can reach storage with a `no-store` response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StorableDirectives(ResponseDirectives);

/// Splits a `Cache-Control` value on commas outside quoted strings.
fn split_directives(value: &str) -> impl Iterator<Item = &str> {
    let mut parts = Vec::new();
    let (mut start, mut in_quotes, mut escaped) = (0, false, false);
    for (index, ch) in value.char_indices() {
        match ch {
            _ if escaped => escaped = false,
            '\\' if in_quotes => escaped = true,
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                parts.push(&value[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&value[start..]);
    parts.into_iter().filter(|part| !part.trim().is_empty())
}

/// Parses an RFC 9111 delta-seconds value, optionally quoted, saturating at
/// [`MAX_DELTA_SECONDS`].
fn parse_delta_seconds(value: &str) -> Option<Duration> {
    let digits = value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let seconds = digits.parse::<u64>().unwrap_or(u64::MAX).min(MAX_DELTA_SECONDS);
    Some(Duration::from_secs(seconds))
}

/// Computes an `expires_at` instant for a storable response.
///
/// Precedence: `no-cache` → no freshness lifetime (always revalidate) →
/// explicit TTL override → `max-age` → `None`.
pub(crate) fn compute_expires_at(
    now: SystemTime,
    ttl_override: Option<Duration>,
    directives: StorableDirectives,
) -> Option<SystemTime> {
    let StorableDirectives(directives) = directives;
    if directives.no_cache {
        return None;
    }
    ttl_override
        .or(directives.max_age)
        .and_then(|lifetime| now.checked_add(lifetime))
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

/// Decides the fetch action from the cached entry's directives, its
/// freshness, and the mode.
///
/// `cached` is `None` when no usable entry exists. A `no-cache` entry always
/// revalidates, whatever the mode or TTL override.
fn decide_action(
    cached: Option<&ResponseDirectives>,
    within_ttl: bool,
    cfg: &RemoteCacheConfig,
) -> CacheAction {
    let Some(directives) = cached else {
        return CacheAction::FetchFresh;
    };
    if cfg.refresh || directives.no_cache {
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

/// A usable cached entry: manifest, body, and the directives it was stored
/// under.
struct CachedEntry {
    manifest: RemoteUrlManifest,
    body: String,
    directives: ResponseDirectives,
}

/// Reads a cached manifest and its body blob, if both are present.
///
/// A manifest recording `no-store` (written before this cache honored it) is
/// never served: it is purged and reported as absent, whatever its
/// [`CACHE_VERSION`]. Any other manifest from another version is a miss and is
/// left on disk.
fn read_cached(
    store: &FileStore,
    key: u64,
    url: &Url,
    warnings: &mut Vec<String>,
) -> Option<CachedEntry> {
    let raw: serde_json::Value = store
        .read_manifest(ArtifactClass::RemoteUrl, key)
        .ok()
        .flatten()?;
    let header: RemoteUrlManifestHeader = serde_json::from_value(raw.clone()).ok()?;
    let directives = ResponseDirectives::parse(header.cache_control.as_deref());
    if directives.no_store {
        purge_cached(store, key, header.body_blob_hash, url, warnings);
        return None;
    }
    if header.cache_version != CACHE_VERSION {
        return None;
    }
    let manifest: RemoteUrlManifest = serde_json::from_value(raw).ok()?;
    let bytes = store
        .read_blob(manifest.body_blob_hash, REMOTE_BLOB_EXT)
        .ok()
        .flatten()?;
    let body = String::from_utf8(bytes).ok()?;
    Some(CachedEntry {
        manifest,
        body,
        directives,
    })
}

/// Best-effort removal of a cached entry; failures become cache warnings.
fn purge_cached(
    store: &FileStore,
    key: u64,
    blob_hash: u64,
    url: &Url,
    warnings: &mut Vec<String>,
) {
    if let Err(err) = store.remove_manifest(ArtifactClass::RemoteUrl, key) {
        warnings.push(format!(
            "failed to remove no-store remote cache manifest {key:016x} ({}): {err}",
            redact_url(url)
        ));
    }
    // Blobs are content-addressed, so another manifest could share this one.
    // Removing it anyway is deliberate: that entry merely re-fetches, which is
    // strictly safer than leaving no-store bytes on disk.
    if let Err(err) = store.remove_blob(blob_hash, REMOTE_BLOB_EXT) {
        warnings.push(format!(
            "failed to remove no-store remote cache body {blob_hash:016x} ({}): {err}",
            redact_url(url)
        ));
    }
}

/// Response metadata recorded in a remote manifest.
struct ResponseMeta {
    status: u16,
    etag: Option<String>,
    last_modified: Option<String>,
    cache_control: Option<String>,
}

/// Stores a response, or purges `previous` when it is `no-store`.
///
/// `previous` is the blob hash of an entry this response supersedes. A
/// `no-store` answer to a revalidation invalidates that entry, so it is
/// removed rather than left to be served stale later.
#[allow(clippy::too_many_arguments)]
fn store_response(
    store: &FileStore,
    key: u64,
    url: &Url,
    body: &str,
    content_hash: u64,
    meta: ResponseMeta,
    previous: Option<u64>,
    now: SystemTime,
    ttl_override: Option<Duration>,
    warnings: &mut Vec<String>,
) {
    match ResponseDirectives::parse(meta.cache_control.as_deref()).storable() {
        Some(directives) => write_cached(
            store,
            key,
            url,
            body,
            content_hash,
            meta,
            directives,
            now,
            ttl_override,
            warnings,
        ),
        None => {
            if let Some(blob_hash) = previous {
                purge_cached(store, key, blob_hash, url, warnings);
            }
        }
    }
}

/// Writes a remote artifact (manifest + body blob) to the store.
#[allow(clippy::too_many_arguments)]
fn write_cached(
    store: &FileStore,
    key: u64,
    url: &Url,
    body: &str,
    content_hash: u64,
    meta: ResponseMeta,
    directives: StorableDirectives,
    now: SystemTime,
    ttl_override: Option<Duration>,
    warnings: &mut Vec<String>,
) {
    let redacted_url = redact_url(url);
    let manifest = RemoteUrlManifest {
        cache_version: CACHE_VERSION,
        redacted_url,
        source_id_hash: key,
        status: meta.status,
        etag: meta.etag,
        last_modified: meta.last_modified,
        cache_control: meta.cache_control,
        fetched_at: now,
        expires_at: compute_expires_at(now, ttl_override, directives),
        content_hash,
        body_blob_hash: content_hash,
        size_bytes: body.len() as u64,
    };
    // Non-fatal: the store creates its directories here, so an unusable cache
    // root (unwritable, or a regular file) first fails at this write and the
    // fetch stays network-only, with a cache warning.
    if let Err(err) = store.write_artifact(
        ArtifactClass::RemoteUrl,
        key,
        &manifest,
        body.as_bytes(),
        content_hash,
        REMOTE_BLOB_EXT,
    ) {
        warnings.push(format!(
            "failed to write remote cache entry {key:016x} ({}): {err}",
            manifest.redacted_url
        ));
    }
}

/// Fetches `url`, consulting and updating the persistent remote cache.
///
/// When `store` is `None`, performs a plain network fetch with no caching.
/// Non-fatal cache I/O failures (a failed write, or a failed `no-store`
/// purge) are appended to `warnings`, on success and error alike.
///
/// ## Errors
///
/// Returns an error string when the network fetch fails and no usable cached
/// body is available under the active freshness mode, or when the response
/// body is not valid UTF-8. A `no-cache` entry whose revalidation fails is an
/// error even under [`RemoteFreshnessMode::Fallback`].
pub(crate) async fn fetch_with_cache(
    store: Option<&FileStore>,
    client: &PolicyClient,
    url: &Url,
    policy: &FetchPolicy,
    cfg: &RemoteCacheConfig,
    warnings: &mut Vec<String>,
) -> Result<RemoteFetchOutcome, String> {
    // No persistent store: plain network fetch, no caching.
    let Some(store) = store else {
        let resp = fetch(client, url, policy, &Conditional::default())
            .await
            .map_err(|e| e.to_string())?;
        let body = String::from_utf8(resp.body.to_vec())
            .map_err(|e| format!("invalid UTF-8 in response body: {e}"))?;
        return Ok(RemoteFetchOutcome {
            body,
            event: RemoteOutcomeEvent::Fetched,
            revalidated: false,
        });
    };

    let key = xx_hash(url.as_str());
    let now = SystemTime::now();
    let cached = read_cached(store, key, url, warnings);
    let within_ttl = cached
        .as_ref()
        .map(|entry| entry.manifest.is_fresh(now))
        .unwrap_or(false);

    match decide_action(cached.as_ref().map(|entry| &entry.directives), within_ttl, cfg) {
        CacheAction::ServeCache => {
            let entry = cached.expect("ServeCache implies a cache entry");
            Ok(RemoteFetchOutcome {
                body: entry.body,
                event: RemoteOutcomeEvent::CacheHit,
                revalidated: false,
            })
        }
        CacheAction::Revalidate => {
            let CachedEntry {
                manifest,
                body: cached_body,
                directives,
            } = cached.expect("Revalidate implies a cache entry");
            let conditional = Conditional {
                if_none_match: manifest.etag.clone(),
                if_modified_since: manifest.last_modified.clone(),
            };
            match fetch(client, url, policy, &conditional).await {
                Ok(resp) if resp.is_not_modified() => {
                    // Preserve the cached body; refresh freshness metadata
                    // with the 304's headers, which may now say no-store.
                    store_response(
                        store,
                        key,
                        url,
                        &cached_body,
                        manifest.content_hash,
                        ResponseMeta {
                            status: manifest.status,
                            etag: resp.etag.or(manifest.etag),
                            last_modified: resp.last_modified.or(manifest.last_modified),
                            cache_control: resp.cache_control.or(manifest.cache_control),
                        },
                        Some(manifest.body_blob_hash),
                        now,
                        cfg.ttl_override,
                        warnings,
                    );
                    Ok(RemoteFetchOutcome {
                        body: cached_body,
                        event: RemoteOutcomeEvent::NotModified,
                        revalidated: true,
                    })
                }
                Ok(resp) => {
                    // Fresh 200: replace the cached body and manifest.
                    let body = String::from_utf8(resp.body.to_vec())
                        .map_err(|e| format!("invalid UTF-8 in response body: {e}"))?;
                    let content_hash = xx_hash(&body);
                    store_response(
                        store,
                        key,
                        url,
                        &body,
                        content_hash,
                        ResponseMeta {
                            status: resp.status,
                            etag: resp.etag,
                            last_modified: resp.last_modified,
                            cache_control: resp.cache_control,
                        },
                        Some(manifest.body_blob_hash),
                        now,
                        cfg.ttl_override,
                        warnings,
                    );
                    Ok(RemoteFetchOutcome {
                        body,
                        event: RemoteOutcomeEvent::Fetched,
                        revalidated: true,
                    })
                }
                // RFC 9111 §5.2.2.4 forbids reusing a no-cache response
                // without successful validation, so Fallback cannot serve it.
                Err(_) if cfg.mode == RemoteFreshnessMode::Fallback && !directives.no_cache => {
                    Ok(RemoteFetchOutcome {
                        body: cached_body,
                        event: RemoteOutcomeEvent::StaleServed,
                        revalidated: true,
                    })
                }
                Err(err) => Err(err.to_string()),
            }
        }
        CacheAction::FetchFresh => {
            let resp = fetch(client, url, policy, &Conditional::default())
                .await
                .map_err(|e| e.to_string())?;
            let body = String::from_utf8(resp.body.to_vec())
                .map_err(|e| format!("invalid UTF-8 in response body: {e}"))?;
            let content_hash = xx_hash(&body);
            store_response(
                store,
                key,
                url,
                &body,
                content_hash,
                ResponseMeta {
                    status: resp.status,
                    etag: resp.etag,
                    last_modified: resp.last_modified,
                    cache_control: resp.cache_control,
                },
                None,
                now,
                cfg.ttl_override,
                warnings,
            );
            Ok(RemoteFetchOutcome {
                body,
                event: RemoteOutcomeEvent::Fetched,
                revalidated: false,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(value: &str) -> ResponseDirectives {
        ResponseDirectives::parse(Some(value))
    }

    fn secs(n: u64) -> Option<Duration> {
        Some(Duration::from_secs(n))
    }

    #[test]
    fn directives_extract_max_age() {
        assert_eq!(parse("max-age=300").max_age, secs(300));
        assert_eq!(parse("public, max-age=60").max_age, secs(60));
        assert_eq!(parse("MAX-AGE=7").max_age, secs(7));
        assert_eq!(parse("max-age=\"60\"").max_age, secs(60));
        assert_eq!(parse("max-age = 5").max_age, secs(5));
    }

    /// A zero freshness lifetime is storable and distinct from `no-store`.
    #[test]
    fn zero_max_age_is_not_no_store() {
        let directives = parse("max-age=0, must-revalidate");
        assert_eq!(directives.max_age, Some(Duration::ZERO));
        assert!(!directives.no_store && !directives.no_cache);
        assert!(directives.storable().is_some());
    }

    #[test]
    fn no_store_and_no_cache_are_flags_not_zero_ttl() {
        let no_store = parse("no-store");
        assert_eq!((no_store.no_store, no_store.no_cache, no_store.max_age), (true, false, None));
        assert!(no_store.storable().is_none());

        let no_cache = parse("no-cache");
        assert_eq!((no_cache.no_store, no_cache.no_cache, no_cache.max_age), (false, true, None));
        assert!(no_cache.storable().is_some());
    }

    /// The original defect: the first recognized directive won, so a
    /// `max-age` ahead of `no-store` hid it.
    #[test]
    fn no_store_after_max_age_is_not_lost() {
        let directives = parse("max-age=3600, no-store");
        assert!(directives.no_store);
        assert_eq!(directives.max_age, secs(3600));
        assert!(directives.storable().is_none());
        assert!(parse("public, max-age=3600, No-Store").no_store);
        assert!(parse("max-age=60,no-cache").no_cache);
    }

    #[test]
    fn field_scoped_no_cache_counts_as_no_cache() {
        let directives = parse("no-cache=\"Set-Cookie, X-Token\", max-age=60");
        assert!(directives.no_cache);
        assert_eq!(directives.max_age, secs(60));
    }

    /// Commas and directive names inside a quoted value are not directives.
    #[test]
    fn quoted_values_do_not_leak_directives() {
        let directives = parse("private=\"no-store, x\", max-age=10");
        assert!(!directives.no_store);
        assert_eq!(directives.max_age, secs(10));
    }

    #[test]
    fn absent_or_invalid_max_age_is_none() {
        assert_eq!(parse("public").max_age, None);
        assert_eq!(parse("").max_age, None);
        assert_eq!(parse("max-age=").max_age, None);
        assert_eq!(parse("max-age=-1").max_age, None);
        assert_eq!(parse("max-age=1.5").max_age, None);
        assert_eq!(ResponseDirectives::parse(None), ResponseDirectives::default());
    }

    #[test]
    fn first_max_age_wins_and_huge_values_saturate() {
        assert_eq!(parse("max-age=10, max-age=99").max_age, secs(10));
        assert_eq!(
            parse("max-age=99999999999999999999999").max_age,
            secs(MAX_DELTA_SECONDS)
        );
    }

    fn storable(value: Option<&str>) -> StorableDirectives {
        ResponseDirectives::parse(value)
            .storable()
            .expect("test input is storable")
    }

    #[test]
    fn expires_at_prefers_override_over_max_age() {
        let now = SystemTime::UNIX_EPOCH;
        let exp = compute_expires_at(now, secs(10), storable(Some("max-age=999")));
        assert_eq!(exp, Some(now + Duration::from_secs(10)));
    }

    #[test]
    fn expires_at_falls_back_to_max_age() {
        let now = SystemTime::UNIX_EPOCH;
        let exp = compute_expires_at(now, None, storable(Some("max-age=42")));
        assert_eq!(exp, Some(now + Duration::from_secs(42)));
    }

    #[test]
    fn expires_at_none_without_ttl_or_header() {
        assert_eq!(compute_expires_at(SystemTime::UNIX_EPOCH, None, storable(None)), None);
    }

    /// A TTL override cannot give a `no-cache` response a freshness lifetime.
    #[test]
    fn ttl_override_does_not_make_no_cache_fresh() {
        let now = SystemTime::UNIX_EPOCH;
        let exp = compute_expires_at(now, secs(3600), storable(Some("no-cache, max-age=60")));
        assert_eq!(exp, None);
    }

    fn cfg(mode: RemoteFreshnessMode, refresh: bool) -> RemoteCacheConfig {
        RemoteCacheConfig {
            ttl_override: None,
            refresh,
            mode,
        }
    }

    const MODES: [RemoteFreshnessMode; 3] = [
        RemoteFreshnessMode::Optimistic,
        RemoteFreshnessMode::Strict,
        RemoteFreshnessMode::Fallback,
    ];

    const PLAIN: ResponseDirectives = ResponseDirectives {
        no_store: false,
        no_cache: false,
        max_age: None,
    };

    #[test]
    fn decide_without_entry_fetches_fresh() {
        for mode in MODES {
            assert_eq!(decide_action(None, false, &cfg(mode, false)), CacheAction::FetchFresh);
        }
    }

    #[test]
    fn decide_refresh_always_revalidates() {
        for mode in MODES {
            assert_eq!(
                decide_action(Some(&PLAIN), true, &cfg(mode, true)),
                CacheAction::Revalidate
            );
        }
    }

    /// `no-cache` revalidates under every mode, even Optimistic, even when a
    /// TTL override says the entry is fresh.
    #[test]
    fn decide_no_cache_always_revalidates() {
        let no_cache = parse("no-cache");
        for mode in MODES {
            let mut config = cfg(mode, false);
            config.ttl_override = secs(3600);
            assert_eq!(
                decide_action(Some(&no_cache), true, &config),
                CacheAction::Revalidate,
                "{mode:?}"
            );
        }
    }

    #[test]
    fn decide_optimistic_serves_cache_even_when_stale() {
        assert_eq!(
            decide_action(Some(&PLAIN), false, &cfg(RemoteFreshnessMode::Optimistic, false)),
            CacheAction::ServeCache
        );
    }

    #[test]
    fn decide_strict_serves_within_ttl_revalidates_past() {
        let strict = cfg(RemoteFreshnessMode::Strict, false);
        assert_eq!(decide_action(Some(&PLAIN), true, &strict), CacheAction::ServeCache);
        assert_eq!(decide_action(Some(&PLAIN), false, &strict), CacheAction::Revalidate);
    }

    #[test]
    fn decide_fallback_matches_strict_for_action() {
        let fallback = cfg(RemoteFreshnessMode::Fallback, false);
        assert_eq!(decide_action(Some(&PLAIN), true, &fallback), CacheAction::ServeCache);
        assert_eq!(decide_action(Some(&PLAIN), false, &fallback), CacheAction::Revalidate);
    }

    /// When a purge cannot remove the entry, the failure is a warning, not an
    /// error. A directory standing where the manifest file should be makes
    /// `remove_file` fail portably.
    #[test]
    fn purge_failure_is_reported_as_warnings() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::at(dir.path().to_path_buf());
        let manifest_path = store.manifest_path(ArtifactClass::RemoteUrl, 0xabcd);
        let blob_path = store.blob_path(0x1234, REMOTE_BLOB_EXT);
        std::fs::create_dir_all(manifest_path.join("occupied")).unwrap();
        std::fs::create_dir_all(blob_path.join("occupied")).unwrap();

        let url = Url::parse("https://user:secret@example.com/doc.md?token=abc").unwrap();
        let mut warnings = Vec::new();
        purge_cached(&store, 0xabcd, 0x1234, &url, &mut warnings);

        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].contains("manifest 000000000000abcd"), "{warnings:?}");
        assert!(warnings[1].contains("body 0000000000001234"), "{warnings:?}");
        for warning in &warnings {
            assert!(
                warning.contains("https://example.com/doc.md?<redacted>"),
                "{warning}"
            );
            assert!(!warning.contains("secret") && !warning.contains("token"), "{warning}");
        }
    }

    #[test]
    fn purge_of_missing_entry_is_silent() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::at(dir.path().join("never-created"));
        let url = Url::parse("https://example.com/doc.md").unwrap();
        let mut warnings = Vec::new();
        purge_cached(&store, 1, 2, &url, &mut warnings);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(!dir.path().join("never-created").exists());
    }
}
