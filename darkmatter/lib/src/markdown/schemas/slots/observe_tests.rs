//! Remote reads of an editor observation: answered only from a fresh
//! transport-cache entry the request may use, never fetched or written.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use biscuit_hash::xx_hash;
use serde_json::json;

use super::*;
use crate::markdown::compose::DataRuntime;
use crate::markdown::compose::cache::FileStore;
use crate::markdown::compose::cache::manifest::{CACHE_VERSION, RemoteUrlManifest, redact_url};
use crate::markdown::compose::cache::types::ArtifactClass;
use crate::markdown::schemas::slots::grants::{Grant, GrantDecision, GrantScope, GrantTarget};

/// Nothing listens here, and the observation never connects anyway.
const URL: &str = "http://127.0.0.1:9/data.yaml";

struct Remote {
    _dir: tempfile::TempDir,
    root: PathBuf,
    context: FileResolutionContext,
}

impl Remote {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let home = root.join(".home");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(".darkmatter.yaml"), "permissions:\n    file-read: []\n").unwrap();
        let context = FileResolutionContext::from_snapshot(&root, Some(home), std::collections::HashMap::new());
        Self { _dir: dir, root, context }
    }

    /// Stores `body` for [`URL`] in the transport store under the root,
    /// expiring at `expires_at`.
    fn seed(&self, body: &str, expires_at: SystemTime) {
        let store = FileStore::at(FileStore::resolve_cache_root(&self.root, None));
        let url = url::Url::parse(URL).unwrap();
        let content_hash = xx_hash(body);
        let manifest = RemoteUrlManifest {
            cache_version: CACHE_VERSION,
            redacted_url: redact_url(&url),
            source_id_hash: xx_hash(url.as_str()),
            status: 200,
            etag: None,
            last_modified: None,
            cache_control: Some("max-age=60".to_string()),
            fetched_at: SystemTime::now() - Duration::from_secs(1),
            expires_at: Some(expires_at),
            content_hash,
            body_blob_hash: content_hash,
            size_bytes: body.len() as u64,
        };
        store
            .write_artifact(ArtifactClass::RemoteUrl, xx_hash(url.as_str()), &manifest, body.as_bytes(), content_hash, "remote")
            .unwrap();
    }

    fn observe(&self, grants: Vec<Grant>, expression: &str) -> PropertyObservations {
        let session = Arc::new(SlotSession::for_context(DataRuntime::new(), &self.context).with_run_grants(grants));
        session.observe_properties(&ObserveRequest {
            document: self.root.join("doc.md"),
            context: self.context.clone(),
            seed: serde_json::Map::from_iter([("p".to_string(), json!("{{ … }}"))]),
            sites: vec![PropertySite { key: "p".to_string(), expression: expression.to_string(), eligible: true }],
            cache_root: Some(self.root.clone()),
            now: SystemTime::now(),
        })
    }

    /// Every file under the transport store, with its bytes.
    fn snapshot(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.map(Result::unwrap) {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, files);
                } else {
                    files.push((path.clone(), std::fs::read(&path).unwrap()));
                }
            }
        }
        let mut files = Vec::new();
        walk(&self.root.join(".darkmatter"), &mut files);
        files.sort();
        files
    }
}

fn allow_url() -> Grant {
    Grant { decision: GrantDecision::Allow, target: GrantTarget::Url(url::Url::parse(URL).unwrap()), scope: GrantScope::Once }
}

fn outcome(observations: &PropertyObservations) -> &PropertyOutcome {
    observations.get("p").expect("the eligible site was evaluated")
}

/// An allowed URL with a fresh entry is observed from the cache, its expiry
/// reported, and the store left as it was.
#[test]
fn an_allowed_url_is_read_from_a_fresh_cache_entry() {
    let remote = Remote::new();
    let expires_at = SystemTime::now() + Duration::from_secs(600);
    remote.seed("- 1\n- 2\n", expires_at);
    let before = remote.snapshot();
    let observations = remote.observe(vec![allow_url()], &format!("load_data('{URL}')"));
    assert!(matches!(outcome(&observations), PropertyOutcome::Observed { value, load: Some(_) } if *value == json!([1, 2])), "{observations:?}");
    assert_eq!(observations.remote_expiry, Some(expires_at));
    let observations = remote.observe(vec![allow_url()], &format!("file_exists('{URL}')"));
    assert!(matches!(outcome(&observations), PropertyOutcome::Observed { value, .. } if *value == json!(true)), "{observations:?}");
    assert_eq!(remote.snapshot(), before, "nothing was written");
}

/// A URL nothing allows is refused before the cache is consulted, though a
/// fresh entry is there; an expired entry and a missing one are pending.
#[test]
fn a_denied_expired_or_missing_entry_leaves_the_read_pending() {
    let remote = Remote::new();
    remote.seed("- 1\n", SystemTime::now() + Duration::from_secs(600));
    let denied = remote.observe(Vec::new(), &format!("load_data('{URL}')"));
    assert!(matches!(outcome(&denied), PropertyOutcome::Pending(PendingReason::ReadRefused)), "{denied:?}");
    assert_eq!(denied.remote_expiry, None);
    let denied = remote.observe(Vec::new(), &format!("try(file_exists('{URL}'), false)"));
    assert!(matches!(outcome(&denied), PropertyOutcome::Pending(PendingReason::ReadRefused)), "{denied:?}");

    let expired = Remote::new();
    expired.seed("- 1\n", SystemTime::now() - Duration::from_secs(5));
    let observations = expired.observe(vec![allow_url()], &format!("try(load_data('{URL}'), 'fallback')"));
    assert!(matches!(outcome(&observations), PropertyOutcome::Pending(PendingReason::RemoteUnavailable)), "{observations:?}");

    let absent = Remote::new();
    let observations = absent.observe(vec![allow_url()], &format!("file_exists('{URL}')"));
    assert!(matches!(outcome(&observations), PropertyOutcome::Pending(PendingReason::RemoteUnavailable)), "{observations:?}");
}

/// The editor does not run `try_frontmatter`, so a read it cannot observe
/// is never answered as `{ ok: false }`: the property stays pending.
#[test]
fn try_frontmatter_is_never_observed_as_a_failed_read() {
    for (grants, name) in [(vec![allow_url()], "allowed, no cache entry"), (Vec::new(), "refused")] {
        let observations = Remote::new().observe(grants, &format!("try_frontmatter('{URL}').ok"));
        assert!(matches!(outcome(&observations), PropertyOutcome::Pending(PendingReason::Unsupported(_))), "{name}: {observations:?}");
    }
}

/// A cached body past the remaining byte allowance is not decoded and stops
/// the pass.
#[test]
fn a_cached_body_past_the_allowance_stops_the_pass() {
    use crate::markdown::compose::expression::iteration::IterationLimits;
    let remote = Remote::new();
    remote.seed(&"- 1\n".repeat(64), SystemTime::now() + Duration::from_secs(600));
    let limits = IterationLimits::new(std::num::NonZeroU64::new(25_000).unwrap(), std::num::NonZeroU64::new(32).unwrap());
    let session = Arc::new(
        SlotSession::for_context(DataRuntime::new(), &remote.context).with_run_grants(vec![allow_url()]).with_slot_limits(limits),
    );
    let observations = session.observe_properties(&ObserveRequest {
        document: remote.root.join("doc.md"),
        context: remote.context.clone(),
        seed: serde_json::Map::new(),
        sites: vec![PropertySite { key: "p".to_string(), expression: format!("load_data('{URL}')"), eligible: true }],
        cache_root: Some(remote.root.clone()),
        now: SystemTime::now(),
    });
    assert!(matches!(outcome(&observations), PropertyOutcome::Pending(PendingReason::Exhausted)), "{observations:?}");
    assert!(observations.exhausted);
}
