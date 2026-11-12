//! Prepared trigger bundles: a host's [`EmbeddedSchemaBundle`] precompiled
//! into a shipped artifact.
//!
//! A host that ships an embedded bundle (Claudine) can check in what trigger
//! discovery would derive from it: each document's classification and each
//! payload's lowered schema. [`prepare_trigger_bundle`] produces that record
//! with every check ordinary discovery runs; the host's artifact holds it;
//! [`PreparedTriggerBundle::from_record`] rebuilds it inside artifact
//! acceptance (`simplified_schema::shipped::accept`); and a bundle built with
//! [`EmbeddedSchemaBundle::with_prepared`] offers it to discovery through a
//! [`PreparedBundleDescriptor`].
//!
//! Discovery serves a document from the prepared bundle only when the
//! supplied bundle has the same name and exactly the same files, byte for
//! byte, as the bundle it was prepared from. A served entry skips payload
//! resolution and the merge-compatibility and compile checks, which
//! generation already ran. Filename selection (schema-root files claiming a
//! name, invalid findings, bundle order) always happens first, and the
//! envelope's match conditions are parsed again from the compiled text, so
//! path patterns stay bound to the registry's own context. See "Prepared
//! bundles" in `darkmatter/docs/topics/schemas/definition.md`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use simplified_schema::errors::SchemaError;
use simplified_schema::shipped::{Acceptance, PreparationMode, RecordError, SourceFingerprint};
use simplified_schema::simplified::DefinitionTable;
use simplified_schema::{SchemaAdvisory, SchemaAdvisoryKind, SchemaShape, ValidatorCache};

use super::assemble::{ResolvedPayload, resolve_embedded_payload};
use super::discovery::EmbeddedTriggerLoad;
use super::envelope::parse_trigger_envelope_from_str;
use crate::markdown::schemas::embedded::EmbeddedSchemaBundle;

/// What trigger discovery derives from one bundle, in the bundle's sorted
/// file order (UTF-8 file-name bytes, the order its triggers layer in).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerBundleRecord {
    /// The bundle's name.
    pub bundle: String,
    /// One record per bundle document, every document included.
    pub documents: Vec<TriggerDocumentRecord>,
}

/// One bundle document and what discovery makes of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerDocumentRecord {
    /// The document's file name inside the bundle.
    pub name: String,
    pub outcome: TriggerOutcomeRecord,
}

/// How discovery classifies a bundle document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TriggerOutcomeRecord {
    /// The document does not claim `kind: schema-trigger`; it is only an
    /// import target.
    NotTrigger,
    /// A trigger without `$schema`: it matches but layers nothing.
    MatcherOnly,
    /// A trigger whose payload resolved, passed the merge-compatibility
    /// check, and compiled.
    Payload(Box<PayloadRecord>),
}

/// A resolved embedded trigger payload. An embedded payload has no
/// dependency files, no searched locations, and no slots, so only these
/// parts are kept.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadRecord {
    /// The lowered JSON Schema, an object.
    pub json_schema: Value,
    /// The payload's SimplifiedSchema object shape; `null` for a payload with
    /// no SimplifiedSchema projection.
    #[serde(deserialize_with = "simplified_schema::shipped::present")]
    pub shape: Option<SchemaShape>,
    /// The cyclic named types `shape` reaches, in allocation order.
    pub definitions: DefinitionTable,
    pub advisories: Vec<AdvisoryRecord>,
}

/// A [`SchemaAdvisory`] on the wire, naming its file by virtual path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AdvisoryRecord {
    MissingSimplifiedEnvelope { path: String },
}

impl AdvisoryRecord {
    fn of(advisory: &SchemaAdvisory) -> Result<Self, String> {
        let path = advisory
            .path()
            .to_str()
            .ok_or_else(|| format!("advisory path `{}` is not UTF-8", advisory.path().display()))?
            .to_string();
        Ok(match advisory.kind() {
            SchemaAdvisoryKind::MissingSimplifiedEnvelope => Self::MissingSimplifiedEnvelope { path },
        })
    }

    fn advisory(&self) -> SchemaAdvisory {
        match self {
            Self::MissingSimplifiedEnvelope { path } => SchemaAdvisory::missing_simplified_envelope(PathBuf::from(path)),
        }
    }
}

/// Prepares `bundle` as trigger discovery would, running every check it runs:
/// the bundle's shape, each envelope, each payload's resolution inside the
/// bundle (through `mode`), its merge-compatibility, and its compilation.
///
/// [`PreparationMode::Uncached`] consults no process memo, accepted artifact,
/// or prepared descriptor (the bundle's own included), and compiles with a
/// private validator cache.
///
/// ## Errors
///
/// The bundle's shape error, or the first document's failure wrapped in a
/// [`SchemaError::EmbeddedBundle`] naming the document. A template payload
/// (one holding `{{ … }}` slots) is among them: an embedded schema never
/// holds slots, at run time or here.
pub fn prepare_trigger_bundle(
    bundle: EmbeddedSchemaBundle,
    mode: PreparationMode<'_>,
) -> Result<TriggerBundleRecord, SchemaError> {
    bundle.check()?;
    let private_cache;
    let cache: &ValidatorCache = match mode {
        PreparationMode::Runtime => &simplified_schema::validate::SHARED_VALIDATOR_CACHE,
        PreparationMode::Uncached(_) => {
            private_cache = ValidatorCache::new();
            &private_cache
        }
    };
    let failed = |name: &str, error: SchemaError| SchemaError::EmbeddedBundle {
        bundle: bundle.name().to_string(),
        message: format!("document `{name}` does not prepare: {error}"),
    };
    let mut documents = Vec::with_capacity(bundle.files().len());
    for (name, text) in bundle.sorted_files() {
        let outcome = match parse_trigger_envelope_from_str(text).map_err(|error| failed(name, error))? {
            None => TriggerOutcomeRecord::NotTrigger,
            Some(envelope) => match envelope.payload.as_ref() {
                None => TriggerOutcomeRecord::MatcherOnly,
                Some(payload) => {
                    let (resolved, _) = resolve_embedded_payload(Some(payload), bundle, name, mode, cache)
                        .map_err(|error| failed(name, error))?;
                    TriggerOutcomeRecord::Payload(Box::new(PayloadRecord {
                        json_schema: (*resolved.json_schema).clone(),
                        shape: resolved.shape.as_deref().cloned(),
                        definitions: (*resolved.definitions).clone(),
                        advisories: resolved
                            .advisories
                            .iter()
                            .map(AdvisoryRecord::of)
                            .collect::<Result<_, _>>()
                            .map_err(|message| SchemaError::EmbeddedBundle {
                                bundle: bundle.name().to_string(),
                                message: format!("document `{name}` does not prepare: {message}"),
                            })?,
                    }))
                }
            },
        };
        documents.push(TriggerDocumentRecord { name: name.to_string(), outcome });
    }
    Ok(TriggerBundleRecord { bundle: bundle.name().to_string(), documents })
}

/// One prepared document, ready for discovery.
#[derive(Debug)]
enum PreparedEntry {
    NotTrigger,
    MatcherOnly,
    /// Shared, so every registry layering it sees one allocation and the
    /// `Arc`-keyed merge and validator memos stay warm.
    Payload(Box<ResolvedPayload>),
}

/// An accepted [`TriggerBundleRecord`]: the prepared documents of one bundle
/// and the fingerprints of the texts they were prepared from.
///
/// Only constructed inside artifact acceptance ([`Self::from_record`] takes an
/// [`Acceptance`]), so an entry served from here passed generation's checks.
pub struct PreparedTriggerBundle {
    bundle: String,
    /// The fingerprints of every bundle document, by virtual path
    /// `{bundle}/{file}`, sorted by identity.
    sources: Vec<SourceFingerprint>,
    entries: HashMap<String, PreparedEntry>,
    /// Whether a supplied file table (by address and length) matched
    /// `sources`, so each table is hashed once.
    matched: Mutex<Vec<(usize, usize, bool)>>,
}

impl std::fmt::Debug for PreparedTriggerBundle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedTriggerBundle")
            .field("bundle", &self.bundle)
            .field("sources", &self.sources)
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl PreparedTriggerBundle {
    /// The prepared bundle of `record`, whose documents must be exactly the
    /// accepted header's sources named `{bundle}/{file}` (a single file-name
    /// segment), listed once each in sorted file order.
    ///
    /// ## Errors
    ///
    /// A [`RecordError`] under `pointer` for an unusable bundle name, a
    /// document that is not a header source, a repeated or out-of-order
    /// document, a header bundle source the record omits, or a payload whose
    /// JSON Schema is not an object.
    pub fn from_record(acceptance: &Acceptance<'_>, pointer: &str, record: TriggerBundleRecord) -> Result<Self, RecordError> {
        if !simplified_schema::embedded::is_plain_segment(&record.bundle) {
            return Err(RecordError::new(
                format!("{pointer}/bundle"),
                format!("bundle name `{}` is not one path segment", record.bundle),
            ));
        }
        let header = acceptance.header();
        let prefix = format!("{}/", record.bundle);
        let mut sources = Vec::with_capacity(record.documents.len());
        let mut entries = HashMap::with_capacity(record.documents.len());
        let mut previous: Option<String> = None;
        for (index, document) in record.documents.into_iter().enumerate() {
            let at = format!("{pointer}/documents/{index}");
            if let Some(previous) = &previous
                && previous.as_bytes() >= document.name.as_bytes()
            {
                let problem = if *previous == document.name { "is listed twice" } else { "is out of order" };
                return Err(RecordError::new(format!("{at}/name"), format!("`{}` {problem}", document.name)));
            }
            let identity = format!("{prefix}{}", document.name);
            let source = header
                .sources
                .iter()
                .find(|source| source.identity == identity)
                .ok_or_else(|| RecordError::new(format!("{at}/name"), format!("`{identity}` is not a source of this artifact")))?;
            sources.push(source.clone());
            let entry = match document.outcome {
                TriggerOutcomeRecord::NotTrigger => PreparedEntry::NotTrigger,
                TriggerOutcomeRecord::MatcherOnly => PreparedEntry::MatcherOnly,
                TriggerOutcomeRecord::Payload(payload) => {
                    if !payload.json_schema.is_object() {
                        return Err(RecordError::new(
                            format!("{at}/outcome/Payload/json_schema"),
                            "a payload's JSON Schema is an object",
                        ));
                    }
                    PreparedEntry::Payload(Box::new(ResolvedPayload {
                        json_schema: Arc::new(payload.json_schema),
                        shape: payload.shape.map(Arc::new),
                        definitions: Arc::new(payload.definitions),
                        dependencies: Vec::new(),
                        searched: Vec::new(),
                        advisories: payload.advisories.iter().map(AdvisoryRecord::advisory).collect(),
                        pending: None,
                    }))
                }
            };
            entries.insert(document.name.clone(), entry);
            previous = Some(document.name);
        }
        if let Some(omitted) = header.sources.iter().find(|source| {
            source.identity.strip_prefix(&prefix).is_some_and(|name| {
                simplified_schema::embedded::is_plain_segment(name) && !entries.contains_key(name)
            })
        }) {
            return Err(RecordError::new(
                format!("{pointer}/documents"),
                format!("bundle document `{}` has no record", omitted.identity),
            ));
        }
        sources.sort_by(|left, right| left.identity.cmp(&right.identity));
        Ok(Self { bundle: record.bundle, sources, entries, matched: Mutex::new(Vec::new()) })
    }

    /// The bundle name the records were prepared for.
    #[must_use]
    pub fn bundle(&self) -> &str {
        &self.bundle
    }

    /// Whether `bundle` is the bundle these records were prepared from: the
    /// same name and the same complete set of files, each with the same text.
    #[must_use]
    pub fn matches(&self, bundle: &EmbeddedSchemaBundle) -> bool {
        if bundle.name() != self.bundle {
            return false;
        }
        let files = bundle.files();
        let key = (files.as_ptr() as usize, files.len());
        let mut matched = self.matched.lock().expect("prepared trigger bundle match memo poisoned");
        if let Some(&(_, _, result)) = matched.iter().find(|(address, length, _)| (*address, *length) == key) {
            return result;
        }
        let mut supplied: Vec<SourceFingerprint> = bundle
            .documents()
            .iter()
            .map(|(path, text)| SourceFingerprint::of(&simplified_schema::embedded::EmbeddedDocument { path, text }))
            .collect();
        supplied.sort_by(|left, right| left.identity.cmp(&right.identity));
        let result = supplied == self.sources;
        matched.push((key.0, key.1, result));
        result
    }

    /// Discovery's result for document `name` of `bundle`, when `bundle`
    /// [matches](Self::matches): `Some(None)` for a document that is not a
    /// trigger, `Some(Some(..))` with the envelope parsed again from the
    /// compiled text and the prepared payload. `None` sends the caller to
    /// ordinary preparation.
    pub(crate) fn load(&self, bundle: &EmbeddedSchemaBundle, name: &str) -> Option<EmbeddedTriggerLoad> {
        if !self.matches(bundle) {
            return None;
        }
        let entry = self.entries.get(name)?;
        if matches!(entry, PreparedEntry::NotTrigger) {
            return Some(None);
        }
        let (_, text) = bundle.files().iter().find(|(file, _)| *file == name)?;
        // The text matched its fingerprint, so this parse succeeded during
        // generation; anything else falls back rather than trusting the record.
        let envelope = parse_trigger_envelope_from_str(text).ok()??;
        let payload = match (entry, envelope.payload.is_some()) {
            (PreparedEntry::MatcherOnly, false) => ResolvedPayload::matcher_only(),
            (PreparedEntry::Payload(payload), true) => (**payload).clone(),
            _ => return None,
        };
        Some(Some((envelope, payload)))
    }
}

/// A host's prepared trigger bundle, found through `provider` only when
/// discovery reaches the bundle's documents.
#[derive(Clone, Copy, Debug)]
pub struct PreparedBundleDescriptor {
    /// The owning crate, for diagnostics.
    pub owner: &'static str,
    /// The accepted prepared bundle, or `None` when the host's artifact was
    /// refused.
    pub provider: fn() -> Option<&'static PreparedTriggerBundle>,
}
