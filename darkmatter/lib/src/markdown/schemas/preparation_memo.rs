//! One preparation of each effective schema per observation of local files.
//!
//! A dry run checks one invocation in several passes (input check, reference
//! check, preview, recovery), and each pass prepares the effective schema of
//! the same prompt again: the `$schema` file resolved, the matched triggers
//! layered, the baseline merged, the validators compiled. While a
//! [`SharedReads`] is the only source of every schema read on the thread
//! ([`buffers::shared_reads_alone`]), each read returns the bytes the first
//! reader saw, so a second preparation from the same inputs would produce
//! the same schema. [`PreparedSchemaMemo`], carried by that observation
//! ([`SharedReads::memo`]), keeps each one.
//!
//! What is kept is the preparation only: every caller still validates its
//! own values against it, so each occurrence's verdict is its own. A
//! preparation that resolved a template (a `$schema` or matched trigger
//! payload holding `{{ … }}` slots) depends on the slot session's values
//! and approvals, so it is never kept. A real run holds no shared
//! observation and prepares every time, as before.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use simplified_schema::resolve::buffers::{self, SharedReads};
use simplified_schema::triggers::TriggerSource;
use simplified_schema::SchemaDigest;

use super::{DarkmatterSchemas, EffectiveSchema, Markdown, base_dir_for, document_path, frontmatter_as_json};
use crate::config::registration::RegisteredHostProfile;

/// The prepared effective schemas of one observation of local files, by
/// everything their preparation reads.
#[derive(Default)]
pub(super) struct PreparedSchemaMemo {
    entries: Mutex<Vec<Entry>>,
}

struct Entry {
    key: PreparationKey,
    /// Each schema source the preparation read, with the xxHash of its
    /// bytes: a hit is served only while the observation still serves
    /// those bytes.
    read: Vec<(PathBuf, u64)>,
    schema: Option<EffectiveSchema>,
}

/// Everything a concrete preparation of [`DarkmatterSchemas::effective_for`]
/// depends on besides the bytes of the files it reads.
#[derive(PartialEq)]
pub(super) struct PreparationKey {
    /// The document layer as written (`$schema`, or the override), and
    /// whether it is an override.
    schema: Option<(Value, bool)>,
    document: Option<PathBuf>,
    base_dir: PathBuf,
    roots: Vec<PathBuf>,
    context: biscuit_file::FileResolutionContext,
    fallback: Option<PathBuf>,
    caller_origins: simplified_schema::validate::CallerOrigins,
    baseline: Option<BaselineKey>,
    /// The registered host that identifies the document and its matched
    /// triggers.
    registered: Option<(ProfileKey, Vec<usize>)>,
    /// The schema-root and embedded triggers that match the document's
    /// values, in registry order.
    triggers: Vec<(TriggerSource, Option<SchemaDigest>)>,
}

/// A baseline by its digest, else by its allocation (held, so the address
/// is never reused while the key lives).
struct BaselineKey(Arc<Value>, Option<SchemaDigest>);

impl PartialEq for BaselineKey {
    fn eq(&self, other: &Self) -> bool {
        match (self.1, other.1) {
            (Some(left), Some(right)) => left == right,
            _ => Arc::ptr_eq(&self.0, &other.0),
        }
    }
}

/// A registered host profile by its allocation, held.
struct ProfileKey(Arc<RegisteredHostProfile>);

impl PartialEq for ProfileKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

thread_local! {
    /// How many templates this thread has resolved: a preparation that
    /// moved it is not kept.
    static TEMPLATES_RESOLVED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Records that a template layer was resolved through a slot session.
pub(super) fn note_template_resolved() {
    TEMPLATES_RESOLVED.set(TEMPLATES_RESOLVED.get() + 1);
}

/// The process's [`observe_preparations`] observer.
static PREPARATION_OBSERVER: std::sync::OnceLock<fn(Option<&std::path::Path>)> = std::sync::OnceLock::new();

/// Installs `observer`, called with the document's file (`None` for a
/// document with none) each time an effective schema is prepared rather
/// than served from a memo, for the rest of the process: a host's test seam
/// that counts a binary's preparations from outside it. The first
/// installation wins; `false` when one was already installed.
pub fn observe_preparations(observer: fn(Option<&std::path::Path>)) -> bool {
    PREPARATION_OBSERVER.set(observer).is_ok()
}

pub(super) fn observe(document: Option<&std::path::Path>) {
    if let Some(observer) = PREPARATION_OBSERVER.get() {
        observer(document);
    }
}

/// The memo of the observation in force on this thread, when every schema
/// read here comes from it.
pub(super) fn in_force() -> Option<(SharedReads, Arc<PreparedSchemaMemo>)> {
    let shared = buffers::shared_reads_alone()?;
    let memo = shared.memo::<PreparedSchemaMemo>();
    Some((shared, memo))
}

impl PreparedSchemaMemo {
    /// The schema prepared for `key`, while `shared` still serves the bytes
    /// its preparation read.
    pub(super) fn get(&self, shared: &SharedReads, key: &PreparationKey) -> Option<Option<EffectiveSchema>> {
        let entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let entry = entries.iter().find(|entry| entry.key == *key)?;
        let unchanged = entry.read.iter().all(|(path, digest)| {
            shared.read(path).is_ok_and(|read| biscuit_hash::xx_hash_bytes(&read.bytes) == *digest)
        });
        unchanged.then(|| entry.schema.clone())
    }

    fn insert(&self, key: PreparationKey, read: Vec<(PathBuf, u64)>, schema: Option<EffectiveSchema>) {
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !entries.iter().any(|entry| entry.key == key) {
            entries.push(Entry { key, read, schema });
        }
    }
}

/// Runs `prepare` (a preparation for `key`) and keeps its schema in `memo`
/// when it resolved no template.
pub(super) fn prepare_and_keep<E>(
    memo: &PreparedSchemaMemo,
    key: PreparationKey,
    prepare: impl FnOnce() -> Result<super::SiteSchema, E>,
) -> Result<super::SiteSchema, E> {
    let templates = TEMPLATES_RESOLVED.get();
    let (prepared, read) = buffers::recording(prepare);
    if let Ok(super::SiteSchema::Effective(schema)) = &prepared
        && TEMPLATES_RESOLVED.get() == templates
    {
        let read = read.into_iter().map(|(path, text)| (path, biscuit_hash::xx_hash_bytes(text.as_bytes()))).collect();
        memo.insert(key, read, schema.clone());
    }
    prepared
}

impl DarkmatterSchemas {
    /// What a preparation of `source` (with `schema_override` in place of its
    /// `$schema`) depends on: the inputs it resolves the document layer
    /// from, and the triggers that match the document's values. Matching is
    /// passive, so computing the key reads no file.
    pub(super) fn preparation_key(&self, source: &Markdown, schema_override: Option<&Value>) -> PreparationKey {
        let schema = schema_override
            .map(|value| (value.clone(), true))
            .or_else(|| source.frontmatter().as_map().get("$schema").map(|value| (value.clone(), false)));
        let registered = self
            .registered_identification(source)
            .map(|(profile, matched)| (ProfileKey(profile), matched));
        let triggers = match self.triggers.as_ref().filter(|registry| !registry.is_empty()) {
            Some(registry) => {
                let pre_resolved = registry.payloads().len() == registry.triggers.len();
                simplified_schema::triggers::assemble::evaluate_registry(registry, &frontmatter_as_json(source), document_path(source))
                    .into_iter()
                    .enumerate()
                    .filter(|(_, eval)| eval.matched && eval.trigger.envelope.payload.is_some())
                    .map(|(index, eval)| {
                        (eval.trigger.source.clone(), pre_resolved.then(|| registry.payloads()[index].digest).flatten())
                    })
                    .collect()
            }
            None => Vec::new(),
        };
        PreparationKey {
            schema,
            document: document_path(source).map(PathBuf::from),
            base_dir: base_dir_for(source, &self.file_resolution_context),
            roots: self.bare_name_roots().to_vec(),
            context: self.file_resolution_context.clone(),
            fallback: self.cache.file_ref_fallback_dir().map(PathBuf::from),
            caller_origins: self.cache.caller_origins().clone(),
            baseline: self.baseline.as_ref().map(|baseline| BaselineKey(Arc::clone(&baseline.json_schema), baseline.digest)),
            registered,
            triggers,
        }
    }
}
