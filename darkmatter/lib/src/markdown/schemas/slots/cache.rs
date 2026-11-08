//! The process-wide cache of resolved schema templates.
//!
//! A resolution is keyed by what it was computed from, never by who asked:
//!
//! - the template: its identity (the `$schema` value or trigger payload and
//!   where it was written), the exact text of every schema file it read, and
//!   its slots;
//! - the request context it resolved in, and the repository the automatic
//!   grant was judged against;
//! - [`SLOT_RESOLVER_VERSION`] and the embedded function catalog, so a build
//!   that evaluates differently never reuses an older result.
//!
//! An entry keeps what the resolution depended on, and a lookup re-checks it
//! before serving: each file it read (by `stat` first, rehashing only a file
//! whose size or modification time changed), each existence answer, each
//! context input, and that the requesting session's grants cover every read
//! the resolution made. A file changed within the modification time's
//! resolution without changing its size is not noticed; that is the cost of
//! not rehashing every input on every lookup.
//!
//! A resolution that read live state (a volatile function), a URL, or a
//! listing is served only to the session that produced it, because none of
//! those can be re-checked cheaply. Failures are cached like successes, with
//! their dependencies, so creating a missing file or repairing a bad one is
//! a miss; a failure that depends on grants (a read needing approval or
//! denied) or a cancellation is never cached. Nothing is written to disk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::SystemTime;

use biscuit_file::FileResolutionContext;
use simplified_schema::resolve::ResolvedSchema;
use simplified_schema::slots::{PendingSchema, SlotResolutionError};

use super::evaluator::ContextDigest;
use crate::markdown::compose::expression::data::{Observation, ObservationKind};
use crate::markdown::compose::expression::read_gate::ReadTarget;

/// Bumped when slot evaluation or substitution changes what a template
/// resolves to, so an entry computed by an older build is never served.
pub(crate) const SLOT_RESOLVER_VERSION: u32 = 1;

/// The key of one resolution: a BLAKE3 digest of the canonical JSON of its
/// inputs (see the module documentation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ResolutionKey([u8; 32]);

/// The key of resolving `pending`, identified by `template`, in `context`,
/// with automatic grants judged against `repository`.
pub(crate) fn resolution_key(
    pending: &PendingSchema,
    template: &str,
    context: &FileResolutionContext,
    repository: Option<&Path>,
) -> ResolutionKey {
    let hex = |bytes: [u8; 32]| bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    let sources: Vec<serde_json::Value> = pending
        .read_sources()
        .iter()
        .map(|(path, text)| {
            serde_json::json!([
                biscuit_file::to_portable_string(path),
                hex(biscuit_hash::blake3_hash_bytes(text.as_bytes()))
            ])
        })
        .collect();
    let slots: Vec<serde_json::Value> = pending
        .slots()
        .iter()
        .map(|slot| {
            serde_json::json!([
                slot.origin.as_deref().map(biscuit_file::to_portable_string),
                slot.span.start,
                slot.expression
            ])
        })
        .collect();
    let canonical = biscuit_hash::canonical_json(&serde_json::json!({
        "version": SLOT_RESOLVER_VERSION,
        "catalog": hex(biscuit_hash::blake3_hash_bytes(
            crate::markdown::compose::expression::catalog::catalog_source().as_bytes()
        )),
        "template": template,
        "sources": sources,
        "slots": slots,
        "context": crate::markdown::compose::file_resolution_context_identity(context),
        "repository": repository.map(biscuit_file::to_portable_string),
        "home": context.home_dir().is_some(),
    }));
    ResolutionKey(biscuit_hash::blake3_hash_bytes(canonical.as_bytes()))
}

/// What a resolution produced. Both arms are shared so a hit hands out the
/// same result.
pub(crate) type Outcome = Result<Arc<ResolvedSchema>, Arc<SlotResolutionError>>;

/// One input a resolution observed, with the `stat` of a file seen when the
/// entry was stored (`None`: always rehash).
#[derive(Debug, Clone)]
pub(crate) struct InputCheck {
    pub(crate) observation: Observation,
    stat: Option<FileStat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStat {
    len: u64,
    modified: SystemTime,
}

fn stat_of(path: &Path) -> Option<FileStat> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(FileStat { len: metadata.len(), modified: metadata.modified().ok()? })
}

/// The local path an observation's read-set entry spells.
fn local_path(observation: &Observation) -> PathBuf {
    PathBuf::from(&observation.entry)
}

impl InputCheck {
    /// `observation`, with the file's `stat` taken now when its bytes still
    /// hash to what was read (a file changed since the read keeps no `stat`,
    /// so the next lookup rehashes it).
    pub(crate) fn capture(observation: Observation) -> Self {
        let stat = match (observation.kind, observation.hash) {
            (ObservationKind::File, Some(hash)) => {
                let path = local_path(&observation);
                stat_of(&path).filter(|_| current_hash(&path) == Some(hash))
            }
            _ => None,
        };
        Self { observation, stat }
    }

    /// Whether the input can be re-checked outside the command that
    /// observed it: a URL body, a listing, and a glob cannot cheaply.
    pub(crate) fn rechecks(&self) -> bool {
        matches!(
            self.observation.kind,
            ObservationKind::File | ObservationKind::Exists | ObservationKind::MissingProbe
        )
    }

    /// Whether the input is still what was observed. `trust_stat` lets an
    /// unchanged `stat` stand for unchanged bytes, unless an editor buffer
    /// is open for the file. An input that cannot be
    /// re-checked ([`Self::rechecks`]) never holds.
    pub(crate) fn holds(&self, trust_stat: bool) -> bool {
        let path = local_path(&self.observation);
        match self.observation.kind {
            ObservationKind::File => {
                // An open editor buffer replaces the file, and its edits
                // leave the file's `stat` unchanged.
                if trust_stat
                    && let Some(stat) = self.stat
                    && simplified_schema::resolve::buffers::text(&path).is_none()
                    && stat_of(&path) == Some(stat)
                {
                    return true;
                }
                current_hash(&path) == self.observation.hash
            }
            ObservationKind::Exists => path.exists(),
            ObservationKind::MissingProbe => !path.exists(),
            ObservationKind::Url | ObservationKind::Listing | ObservationKind::Glob => false,
        }
    }
}

/// BLAKE3 of the bytes a slot read of `path` sees now (an open editor
/// buffer's text when there is one); `None` when it cannot be read.
fn current_hash(path: &Path) -> Option<[u8; 32]> {
    match simplified_schema::resolve::buffers::text(path) {
        Some(text) => Some(biscuit_hash::blake3_hash_bytes(text.as_bytes())),
        None => std::fs::read(path).ok().map(|bytes| biscuit_hash::blake3_hash_bytes(&bytes)),
    }
}

/// One context input a slot read, in the context of the file defining the
/// slot (`None`: the request context itself).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextCheck {
    pub(crate) defining: Option<PathBuf>,
    pub(crate) digest: ContextDigest,
}

/// One read a resolution's gate allowed, for the coverage check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GrantedRead {
    /// Where the reading slot was written.
    pub(crate) defining: PathBuf,
    pub(crate) target: ReadTarget,
}

/// A stored resolution and what it depended on.
#[derive(Debug)]
pub(crate) struct CacheEntry {
    pub(crate) outcome: Outcome,
    pub(crate) inputs: Vec<InputCheck>,
    pub(crate) context: Vec<ContextCheck>,
    pub(crate) reads: Vec<GrantedRead>,
    /// The session the entry may be served to, for an entry that depends on
    /// something only that command can vouch for; `None` for any session.
    pub(crate) only_for: Option<u64>,
}

/// The digest each recorded context input has now, in `request`.
pub(crate) fn context_holds(check: &ContextCheck, request: &FileResolutionContext) -> bool {
    let context = super::evaluator::slot_context(request, check.defining.as_deref());
    super::evaluator::context_digest_now(&check.digest.input, &context) == check.digest.digest
}

static CACHE: LazyLock<Mutex<HashMap<ResolutionKey, Arc<CacheEntry>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// The stored entry for `key`, unchecked.
pub(crate) fn lookup(key: &ResolutionKey) -> Option<Arc<CacheEntry>> {
    CACHE.lock().unwrap_or_else(PoisonError::into_inner).get(key).cloned()
}

/// Stores `entry` under `key`, replacing an older one.
pub(crate) fn store(key: ResolutionKey, entry: Arc<CacheEntry>) {
    CACHE.lock().unwrap_or_else(PoisonError::into_inner).insert(key, entry);
}

/// Empties the cache (see
/// [`crate::markdown::schemas::reset_process_caches_for_bench`]).
pub(crate) fn clear() {
    CACHE.lock().unwrap_or_else(PoisonError::into_inner).clear();
}

/// Whether a failed resolution may be cached: not when it depends on the
/// grants in force or was canceled.
pub(crate) fn caches_failure(error: &SlotResolutionError) -> bool {
    match error {
        SlotResolutionError::Failed(failed) => failed.iter().all(|(_, failure)| {
            !matches!(
                failure.code,
                Some(super::codes::SLOT_APPROVAL_REQUIRED_CODE)
                    | Some(super::codes::SLOT_DENIED_CODE)
                    | Some(super::codes::SLOT_GRANT_STORE_INVALID_CODE)
                    | Some(super::codes::SLOT_CYCLE_CODE)
            )
        }),
        SlotResolutionError::Canceled => false,
        _ => true,
    }
}
