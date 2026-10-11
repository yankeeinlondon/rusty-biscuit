//! The persistent symbol cache: one entry per file, keyed by the file and the
//! inputs that decide its records, recording which kinds and capabilities it
//! holds. `docs/cache-design.md` describes the entry and the lookup outcomes.

use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use tree_sitter::Language;

use super::stored_reexports::validate_reexports;
use super::stored_evidence::{
    LineStarts, ReadOccurrences, StoredEvidence, import_inventory, nullable, read_coverage, read_imports, read_occurrences, validate_evidence, validate_imports, write_coverage, write_imports, write_occurrences,
};
use super::{CacheError, PassFingerprint, PersistentCache, canonical_digest};
use crate::analysis::coverage::{add_record_passes, project, record_passes_suffice};
use crate::error::TreeHuggerError;
use crate::file::query::{Capability, SymbolQuery, SymbolQueryResult};
use crate::file::source_file::SourceFile;
use crate::queries::{QueryKind, resolve_query_text};
use crate::rule_registry::RuleRegistry;
use crate::shared::{
    Diagnostic, ExportRecord, FileSymbolIndex, ImportRecord, Occurrence, ProgrammingLanguage,
    ReexportReport, ReferenceCoverage, SchemaVersion, SymbolKindV2, SymbolRecord,
};

/// The queries whose text decides a file's records, imports and references.
const RECORD_QUERIES: [QueryKind; 3] = [QueryKind::Locals, QueryKind::Imports, QueryKind::References];
/// The queries whose text decides only its diagnostics.
const DIAGNOSTIC_QUERIES: [QueryKind; 2] = [QueryKind::Lint, QueryKind::Comments];

/// Everything besides a file's text that decides its cached analysis: the
/// language, the grammar's identity and the effective text of each query.
///
/// The query text is an input rather than read at hashing time so that a test
/// can change one query's text and observe the entry stop matching.
#[derive(Debug, Clone)]
pub struct FingerprintInputs {
    language: ProgrammingLanguage,
    grammar_id: &'static str,
    grammar_identity: String,
    query_texts: Vec<(QueryKind, String)>,
}

impl FingerprintInputs {
    /// The shipped inputs for `language` with the grammar it uses for `path`.
    ///
    /// ## Errors
    ///
    /// Returns an error when a query's inherited text cannot be resolved.
    pub fn shipped(language: ProgrammingLanguage, path: &Path) -> Result<Self, TreeHuggerError> {
        let (grammar, grammar_id) = language.grammar_for_path(path);
        let query_texts = RECORD_QUERIES
            .iter()
            .chain(&DIAGNOSTIC_QUERIES)
            .map(|&kind| Ok((kind, resolve_query_text(language, kind)?)))
            .collect::<Result<_, TreeHuggerError>>()?;
        Ok(Self {
            language,
            grammar_id,
            grammar_identity: grammar_identity(&grammar),
            query_texts,
        })
    }

    /// Replaces the text of the `kind` query.
    pub fn with_query_text(mut self, kind: QueryKind, text: impl Into<String>) -> Self {
        let text = text.into();
        match self.query_texts.iter_mut().find(|(existing, _)| *existing == kind) {
            Some(slot) => slot.1 = text,
            None => self.query_texts.push((kind, text)),
        }
        self
    }

    /// The inputs of the records, imports, exports and every pass but
    /// diagnostics: schema, implementation revision, grammar and the
    /// definition, import and reference queries.
    pub fn records_fingerprint(&self) -> PassFingerprint {
        PassFingerprint {
            schema_version: SchemaVersion::CURRENT,
            tree_hugger_version: implementation_version(),
            grammar_version: self.grammar_identity.clone(),
            grammar_id: self.grammar_id.to_string(),
            query_hash: self.query_digest(&RECORD_QUERIES),
            rule_metadata_hash: String::new(),
            enabled_rules_hash: String::new(),
            config_hash: String::new(),
            external_tool_version: None,
        }
    }

    /// The additional inputs of the diagnostics pass: the lint and comment
    /// queries, the built-in rules and whether experimental semantic rules
    /// run.
    pub fn diagnostics_fingerprint(&self, experimental_semantics: bool) -> PassFingerprint {
        PassFingerprint {
            query_hash: self.query_digest(&DIAGNOSTIC_QUERIES),
            rule_metadata_hash: rule_metadata_digest().to_string(),
            config_hash: format!("experimental_semantics={experimental_semantics}"),
            ..self.records_fingerprint()
        }
    }

    fn query_digest(&self, kinds: &[QueryKind]) -> String {
        let mut fields = vec![self.language.query_name().to_string()];
        for kind in kinds {
            let text = self
                .query_texts
                .iter()
                .find(|(existing, _)| existing == kind)
                .map_or("", |(_, text)| text.as_str());
            fields.push(kind.to_string());
            fields.push(text.to_string());
        }
        canonical_digest(&fields.iter().map(String::as_str).collect::<Vec<_>>())
    }
}

/// The crate version and extractor revision. A change to Rust extraction code
/// that changes records must bump `EXTRACTOR_REVISION`: query text alone
/// cannot detect it.
fn implementation_version() -> String {
    format!(
        "{}+rev.{}",
        env!("CARGO_PKG_VERSION"),
        crate::file::EXTRACTOR_REVISION
    )
}

/// What the compiled grammar reports about itself: its name and version
/// (when the grammar declares them), ABI and table sizes.
fn grammar_identity(grammar: &Language) -> String {
    let version = grammar.metadata().map_or_else(
        || "unversioned".to_string(),
        |metadata| {
            format!(
                "{}.{}.{}",
                metadata.major_version, metadata.minor_version, metadata.patch_version
            )
        },
    );
    format!(
        "name={};version={version};abi={};kinds={};fields={};states={}",
        grammar.name().unwrap_or("unnamed"),
        grammar.abi_version(),
        grammar.node_kind_count(),
        grammar.field_count(),
        grammar.parse_state_count()
    )
}

/// A digest of every built-in rule's metadata that reaches a diagnostic.
fn rule_metadata_digest() -> &'static str {
    static DIGEST: OnceLock<String> = OnceLock::new();
    DIGEST.get_or_init(|| {
        let registry = RuleRegistry::new();
        let mut ids: Vec<&String> = registry.rule_ids().collect();
        ids.sort();
        let fields: Vec<String> = ids
            .into_iter()
            .filter_map(|id| registry.get(id))
            .map(|rule| {
                format!(
                    "{}|{}|{:?}|{:?}|{:?}|{:?}|{}|{}",
                    rule.id,
                    rule.version,
                    rule.category,
                    rule.default_severity,
                    rule.confidence,
                    rule.languages,
                    rule.enabled_by_default,
                    rule.requires_experimental_semantics
                )
            })
            .collect();
        canonical_digest(&fields.iter().map(String::as_str).collect::<Vec<_>>())
    })
}

/// How a [`SymbolCache::query`] used the file's entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheLookup {
    /// The entry held every requested kind and capability: nothing was parsed
    /// or analyzed.
    Hit,
    /// The entry held every requested kind but not every capability; the
    /// missing ones were computed and the entry rewritten to hold both.
    /// `parsed` is whether that needed the file to be parsed (only the docs
    /// pass, semantics over stored occurrences and, for an entry of every
    /// kind outside Rust and Swift, containment work from the stored entry
    /// alone).
    WidenedCapabilities { parsed: bool },
    /// The entry lacked a requested kind; the file was analyzed once for the
    /// union of stored and requested kinds and capabilities, and the entry
    /// rewritten.
    WidenedKinds,
    /// No usable entry; the query was computed and its result stored.
    Miss(CacheMiss),
}

/// Why a file's entry could not be used. Never an empty result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheMiss {
    /// There is no entry (or the cache is disabled).
    Absent,
    /// The entry exists but could not be read.
    Unreadable(String),
    /// The entry is not a well-formed entry: invalid JSON, trailing content,
    /// a duplicate key, or a field absent, null or of the wrong shape. The
    /// text is the decoder's, naming the field.
    Malformed(String),
    /// A well-formed entry made for other inputs (the entry's or any record's
    /// schema version, fingerprint, file, language or source hash).
    Mismatched(&'static str),
    /// A well-formed entry whose contents do not match its recorded coverage.
    Inconsistent(&'static str),
}

impl fmt::Display for CacheMiss {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => formatter.write_str("no entry"),
            Self::Unreadable(error) => write!(formatter, "unreadable entry: {error}"),
            Self::Malformed(error) => write!(formatter, "malformed entry: {error}"),
            Self::Mismatched(field) => write!(formatter, "entry for a different {field}"),
            Self::Inconsistent(what) => write!(formatter, "inconsistent entry: {what}"),
        }
    }
}

/// The result of a cached query and how the cache was used.
#[derive(Debug)]
pub struct CachedQueryResult {
    /// Exactly what an uncached [`SourceFile::query`] returns.
    pub result: SymbolQueryResult,
    pub lookup: CacheLookup,
    /// Why writing the entry back failed, if it did. The result is complete
    /// either way.
    pub write_error: Option<CacheError>,
}

/// One persistent entry per file, shared by every query of that file.
///
/// The key is the file's path (separators normalized), its source hash, any
/// forced language and the [records fingerprint](FingerprintInputs::records_fingerprint);
/// the kinds and capabilities a query asks for are not part of it. The entry
/// records the kinds and capabilities it holds, so a narrower query is served
/// from a wider entry and a wider query widens the entry in place.
///
/// ## Examples
///
/// ```no_run
/// use tree_hugger::cache::{PersistentCache, SymbolCache};
/// use tree_hugger::{SourceFile, SymbolQuery};
///
/// let cache = SymbolCache::new(PersistentCache::default_cache("my-project", true));
/// let cached = cache.query(SourceFile::read("src/lib.rs")?, &SymbolQuery::new())?;
/// println!("{} records ({:?})", cached.result.index.symbols.len(), cached.lookup);
/// # Ok::<(), tree_hugger::TreeHuggerError>(())
/// ```
#[derive(Debug)]
pub struct SymbolCache {
    store: PersistentCache,
    query_text_overrides: Vec<(ProgrammingLanguage, QueryKind, String)>,
    fingerprints: Mutex<HashMap<(ProgrammingLanguage, &'static str), Arc<Fingerprints>>>,
}

#[derive(Debug)]
struct Fingerprints {
    records: String,
    /// Indexed by whether experimental semantic rules run.
    diagnostics: [String; 2],
}

impl SymbolCache {
    pub fn new(store: PersistentCache) -> Self {
        Self {
            store,
            query_text_overrides: Vec::new(),
            fingerprints: Mutex::new(HashMap::new()),
        }
    }

    /// Fingerprints `language`'s `kind` query as `text` instead of the
    /// shipped text. Analysis still runs the shipped queries.
    pub fn with_query_text(
        mut self,
        language: ProgrammingLanguage,
        kind: QueryKind,
        text: impl Into<String>,
    ) -> Self {
        self.query_text_overrides.push((language, kind, text.into()));
        self
    }

    /// The key of `source`'s entry.
    ///
    /// ## Errors
    ///
    /// As [`FingerprintInputs::shipped`].
    pub fn key(&self, source: &SourceFile) -> Result<String, TreeHuggerError> {
        Ok(entry_key(source, &self.fingerprints(source)?.records))
    }

    /// Runs `query` over `source`, from its entry when the entry covers the
    /// query, and stores what was computed.
    ///
    /// The file is parsed only when the entry lacks a requested kind, or a
    /// requested capability that needs the syntax tree; the bytes parsed are
    /// the bytes `source` hashed for the key.
    ///
    /// ## Errors
    ///
    /// Returns the errors of [`SourceFile::query`]. Cache failures are not
    /// errors: an unusable entry is a [`CacheLookup::Miss`] and a failed
    /// write is [`CachedQueryResult::write_error`].
    pub fn query(
        &self,
        source: SourceFile,
        query: &SymbolQuery,
    ) -> Result<CachedQueryResult, TreeHuggerError> {
        self.query_retaining(source, query).map(|(result, _)| result)
    }

    /// As [`query`](Self::query), and gives back `source`, for a caller that
    /// decides from the result whether to keep these exact bytes. The text is
    /// moved through any parse, never copied.
    ///
    /// ## Errors
    ///
    /// As [`query`](Self::query); the source is dropped with the error.
    pub fn query_retaining(
        &self,
        source: SourceFile,
        query: &SymbolQuery,
    ) -> Result<(CachedQueryResult, SourceFile), TreeHuggerError> {
        let fingerprints = self.fingerprints(&source)?;
        let key = entry_key(&source, &fingerprints.records);
        let diagnostics_fingerprint =
            fingerprints.diagnostics[usize::from(source.uses_experimental_semantics())].as_str();
        let entry_file = entry_file(source.path());

        let (lookup, coverage, index, source) = match self.read(&key, &source, &fingerprints.records) {
            Err(miss) => {
                let (analyzed, source) = source.analyze_retaining(query)?;
                (CacheLookup::Miss(miss), query.clone(), analyzed.index, source)
            }
            Ok(stored) => {
                let (mut coverage, stored_diagnostics_fingerprint, mut index) =
                    stored.into_parts(&source);
                // Diagnostics made under other rules or settings are missing,
                // not wrong: the rest of the entry still serves.
                if coverage.requires(Capability::Diagnostics)
                    && stored_diagnostics_fingerprint.as_deref() != Some(diagnostics_fingerprint)
                {
                    coverage = coverage.without(Capability::Diagnostics);
                    index.diagnostics.clear();
                }
                let missing = coverage.capabilities_missing_for(query);
                if !coverage.selects_all_of(query) {
                    let wider = coverage.union(query);
                    let (analyzed, source) = source.analyze_retaining(&wider)?;
                    (CacheLookup::WidenedKinds, wider, analyzed.index, source)
                } else if missing.is_empty() {
                    let hit = CachedQueryResult {
                        result: project(index, query).finish(query),
                        lookup: CacheLookup::Hit,
                        write_error: None,
                    };
                    return Ok((hit, source));
                } else if record_passes_suffice(
                    &missing,
                    coverage.capability_set(),
                    coverage.kind_set(),
                    index.language,
                ) {
                    add_record_passes(&mut index, coverage.capability_set(), &missing);
                    let lookup = CacheLookup::WidenedCapabilities { parsed: false };
                    (lookup, coverage.union(query), index, source)
                } else {
                    let wider = coverage.union(query);
                    let (analyzed, source) = source.analyze_retaining(&wider)?;
                    let lookup = CacheLookup::WidenedCapabilities { parsed: true };
                    (lookup, wider, analyzed.index, source)
                }
            }
        };

        let write_error = self
            .write(
                &key,
                &entry_file,
                &fingerprints.records,
                &coverage,
                diagnostics_fingerprint,
                &index,
            )
            .err();
        let result = CachedQueryResult {
            result: project(index, query).finish(query),
            lookup,
            write_error,
        };
        Ok((result, source))
    }

    fn fingerprints(&self, source: &SourceFile) -> Result<Arc<Fingerprints>, TreeHuggerError> {
        let language = source.language();
        let (_, grammar_id) = language.grammar_for_path(source.path());
        let memo_key = (language, grammar_id);
        if let Some(found) = self.lock_fingerprints().get(&memo_key) {
            return Ok(Arc::clone(found));
        }
        let mut inputs = FingerprintInputs::shipped(language, source.path())?;
        for (_, kind, text) in self
            .query_text_overrides
            .iter()
            .filter(|(overridden, ..)| *overridden == language)
        {
            inputs = inputs.with_query_text(*kind, text.clone());
        }
        let fingerprints = Arc::new(Fingerprints {
            records: inputs.records_fingerprint().digest(),
            diagnostics: [
                inputs.diagnostics_fingerprint(false).digest(),
                inputs.diagnostics_fingerprint(true).digest(),
            ],
        });
        self.lock_fingerprints()
            .insert(memo_key, Arc::clone(&fingerprints));
        Ok(fingerprints)
    }

    fn lock_fingerprints(
        &self,
    ) -> std::sync::MutexGuard<'_, HashMap<(ProgrammingLanguage, &'static str), Arc<Fingerprints>>>
    {
        // The map only memoizes pure values, so a panic elsewhere cannot have
        // left it inconsistent.
        self.fingerprints
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn read(
        &self,
        key: &str,
        source: &SourceFile,
        fingerprint: &str,
    ) -> Result<StoredEntry, CacheMiss> {
        let bytes = match self.store.get_bytes(key) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Err(CacheMiss::Absent),
            Err(error) => return Err(CacheMiss::Unreadable(error.to_string())),
        };
        let entry: StoredEntry = serde_json::from_slice(&bytes)
            .map_err(|error| CacheMiss::Malformed(error.to_string()))?;
        entry.validate(source, fingerprint)?;
        Ok(entry)
    }

    fn write(
        &self,
        key: &str,
        file: &str,
        fingerprint: &str,
        coverage: &SymbolQuery,
        diagnostics_fingerprint: &str,
        index: &FileSymbolIndex,
    ) -> Result<(), CacheError> {
        // An entry must name at least one kind; a query selecting none has
        // nothing worth storing.
        if coverage.kind_set().is_some_and(BTreeSet::is_empty) {
            return Ok(());
        }
        let has_diagnostics = coverage.requires(Capability::Diagnostics);
        self.store.put(
            key,
            &WrittenEntry {
                schema_version: SchemaVersion::CURRENT,
                fingerprint,
                file,
                language: index.language,
                source_hash: &index.file_hash,
                covered_kinds: coverage.kind_set(),
                completed_capabilities: coverage.capability_set(),
                diagnostics_fingerprint: has_diagnostics.then_some(diagnostics_fingerprint),
                records: &index.symbols,
                imports: &index.imports,
                import_inventory: import_inventory(&index.imports),
                exports: &index.exports,
                diagnostics: &index.diagnostics,
                occurrences: &index.occurrences,
                reference_coverage: &index.reference_coverage,
                reexports: index.reexports.as_ref(),
            },
        )
    }
}

/// The file as the entry names it: the path exactly as analyzed (the file's
/// identity, see [`crate::file_identity`]), since the records repeat it.
fn entry_file(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn entry_key(source: &SourceFile, records_fingerprint: &str) -> String {
    let file = entry_file(source.path()).replace('\\', "/");
    let language = source
        .forced_language()
        .map_or("detected", |language| language.query_name());
    canonical_digest(&[
        "symbols",
        &file,
        source.hash(),
        language,
        records_fingerprint,
    ])
}

/// The entry as written; [`StoredEntry`] reads the same fields.
#[derive(Serialize)]
struct WrittenEntry<'a> {
    schema_version: SchemaVersion,
    fingerprint: &'a str,
    file: &'a str,
    language: ProgrammingLanguage,
    source_hash: &'a str,
    #[serde(serialize_with = "serialize_covered_kinds")]
    covered_kinds: Option<&'a BTreeSet<SymbolKindV2>>,
    completed_capabilities: &'a BTreeSet<Capability>,
    diagnostics_fingerprint: Option<&'a str>,
    records: &'a [SymbolRecord],
    #[serde(serialize_with = "write_imports")]
    imports: &'a [ImportRecord],
    import_inventory: String,
    exports: &'a [ExportRecord],
    diagnostics: &'a [Diagnostic],
    /// Every key written, `null` and empty lists included
    /// (`stored_evidence`).
    #[serde(serialize_with = "write_occurrences")]
    occurrences: &'a [Occurrence],
    #[serde(serialize_with = "write_coverage")]
    reference_coverage: &'a [ReferenceCoverage],
    /// `null` exactly when `reexports` is not a completed capability.
    reexports: Option<&'a ReexportReport>,
}

/// Every kind is the string `"all"`, never an empty list.
fn serialize_covered_kinds<S: Serializer>(
    kinds: &Option<&BTreeSet<SymbolKindV2>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match kinds {
        None => serializer.serialize_str(ALL_KINDS),
        Some(kinds) => serializer.collect_seq(kinds.iter()),
    }
}

const ALL_KINDS: &str = "all";

/// An entry as read. Every field is required and none has a default: a
/// field absent, null where a value is required, of the wrong type, or
/// repeated makes the entry malformed.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEntry {
    schema_version: SchemaVersion,
    fingerprint: String,
    file: String,
    language: ProgrammingLanguage,
    source_hash: String,
    covered_kinds: CoveredKinds,
    completed_capabilities: CompletedCapabilities,
    /// Required but nullable: `null` when the entry holds no diagnostics.
    #[serde(deserialize_with = "nullable")]
    diagnostics_fingerprint: Option<String>,
    records: Vec<SymbolRecord>,
    #[serde(deserialize_with = "read_imports")]
    imports: Vec<ImportRecord>,
    import_inventory: String,
    exports: Vec<ExportRecord>,
    diagnostics: Vec<Diagnostic>,
    /// Required (an empty list is a file with none), in the strict form
    /// `stored_evidence` reads; non-empty only under the occurrences
    /// capability.
    #[serde(deserialize_with = "read_occurrences")]
    occurrences: ReadOccurrences,
    #[serde(deserialize_with = "read_coverage")]
    reference_coverage: Vec<ReferenceCoverage>,
    /// Required but nullable: `null` when the entry did not compute
    /// re-exports.
    #[serde(deserialize_with = "nullable")]
    reexports: Option<ReexportReport>,
}

impl StoredEntry {
    fn validate(&self, source: &SourceFile, fingerprint: &str) -> Result<(), CacheMiss> {
        if self.schema_version != SchemaVersion::CURRENT {
            return Err(CacheMiss::Mismatched("schema version"));
        }
        // Records are served as they are stored, inside a current-version
        // index, so each must be exactly the current version too; the cache
        // never translates between versions, minor ones included.
        if self
            .records
            .iter()
            .any(|record| record.schema_version != SchemaVersion::CURRENT)
        {
            return Err(CacheMiss::Mismatched("record schema version"));
        }
        if self.fingerprint != fingerprint {
            return Err(CacheMiss::Mismatched("fingerprint"));
        }
        if self.file != entry_file(source.path()) {
            return Err(CacheMiss::Mismatched("file"));
        }
        if self.language != source.language() {
            return Err(CacheMiss::Mismatched("language"));
        }
        if self.source_hash != source.hash() {
            return Err(CacheMiss::Mismatched("source hash"));
        }
        let has_diagnostics = self.completed_capabilities.0.contains(&Capability::Diagnostics);
        if has_diagnostics != self.diagnostics_fingerprint.is_some() {
            return Err(CacheMiss::Inconsistent(
                "diagnostics fingerprint without the diagnostics capability, or the reverse",
            ));
        }
        if !has_diagnostics && !self.diagnostics.is_empty() {
            return Err(CacheMiss::Inconsistent(
                "diagnostics stored without the diagnostics capability",
            ));
        }
        // Without its capability a record holds no metrics facet and no set
        // flag; a stored one would be served as computed when it was not.
        let capabilities = &self.completed_capabilities.0;
        if !capabilities.contains(&Capability::Metrics)
            && self.records.iter().any(|record| record.metrics.is_some())
        {
            return Err(CacheMiss::Inconsistent("metrics stored without the metrics capability"));
        }
        if !capabilities.contains(&Capability::Flags)
            && self
                .records
                .iter()
                .any(|record| record.visibility.is_test || record.visibility.is_deprecated)
        {
            return Err(CacheMiss::Inconsistent("a flag set without the flags capability"));
        }
        if capabilities.contains(&Capability::Metrics)
            && self.records.iter().any(|record| record.metrics.is_none())
        {
            return Err(CacheMiss::Inconsistent("a record without metrics under the metrics capability"));
        }
        let has_reexports = capabilities.contains(&Capability::Reexports);
        if has_reexports != self.reexports.is_some() {
            return Err(CacheMiss::Inconsistent(
                "a re-export report without the reexports capability, or the reverse",
            ));
        }
        if !has_reexports && self.imports.iter().any(|import| import.reexported) {
            return Err(CacheMiss::Inconsistent("a reexported marker without the reexports capability"));
        }
        let lines = LineStarts::new(source.source());
        validate_imports(&lines, self.language, &self.imports, &self.import_inventory).map_err(CacheMiss::Inconsistent)?;
        if let Some(report) = &self.reexports {
            validate_reexports(&lines, self.language, &self.imports, report).map_err(CacheMiss::Inconsistent)?;
        }
        validate_evidence(&StoredEvidence {
            lines: &lines,
            language: self.language,
            capabilities,
            covered_kinds: self.covered_kinds.0.as_ref(),
            occurrences: &self.occurrences.occurrences,
            path_starts: &self.occurrences.path_starts,
            import_starts: &self.occurrences.import_starts,
            coverage: &self.reference_coverage,
            records: &self.records,
            imports: &self.imports,
        })
        .map_err(CacheMiss::Inconsistent)?;
        if let Some(kinds) = &self.covered_kinds.0
            && self.records.iter().any(|record| !kinds.contains(&record.kind))
        {
            return Err(CacheMiss::Inconsistent("a record of a kind the entry does not cover"));
        }
        Ok(())
    }

    /// The entry's coverage as a query, its diagnostics fingerprint and its
    /// contents as an index of `source`.
    fn into_parts(self, source: &SourceFile) -> (SymbolQuery, Option<String>, FileSymbolIndex) {
        let mut coverage = SymbolQuery::new().capabilities(self.completed_capabilities.0);
        if let Some(kinds) = self.covered_kinds.0 {
            coverage = coverage.kinds(kinds);
        }
        let index = FileSymbolIndex {
            schema_version: SchemaVersion::CURRENT,
            file: source.path().to_path_buf(),
            language: source.language(),
            file_hash: source.hash().to_string(),
            completed_passes: Vec::new(),
            symbols: self.records,
            imports: self.imports,
            exports: self.exports,
            diagnostics: self.diagnostics,
            occurrences: self.occurrences.occurrences,
            reference_coverage: self.reference_coverage,
            reexports: self.reexports,
        };
        (coverage, self.diagnostics_fingerprint, index)
    }
}

/// `None` = every kind (written `"all"`); otherwise a non-empty, strictly
/// ascending list.
struct CoveredKinds(Option<BTreeSet<SymbolKindV2>>);

impl<'de> Deserialize<'de> for CoveredKinds {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KindsVisitor;

        impl<'de> Visitor<'de> for KindsVisitor {
            type Value = CoveredKinds;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("\"all\" or a non-empty, ascending list of symbol kinds")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<CoveredKinds, E> {
                if value == ALL_KINDS {
                    Ok(CoveredKinds(None))
                } else {
                    Err(E::invalid_value(de::Unexpected::Str(value), &self))
                }
            }

            fn visit_seq<A: SeqAccess<'de>>(self, sequence: A) -> Result<CoveredKinds, A::Error> {
                Ok(CoveredKinds(Some(strictly_ascending_set(sequence, &self)?)))
            }
        }

        deserializer.deserialize_any(KindsVisitor)
    }
}

/// A non-empty, strictly ascending capability list, closed under
/// prerequisites (so it always holds `declarations`).
struct CompletedCapabilities(BTreeSet<Capability>);

impl<'de> Deserialize<'de> for CompletedCapabilities {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CapabilitiesVisitor;

        impl<'de> Visitor<'de> for CapabilitiesVisitor {
            type Value = CompletedCapabilities;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(
                    "a non-empty, ascending list of capabilities closed under prerequisites",
                )
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                sequence: A,
            ) -> Result<CompletedCapabilities, A::Error> {
                let capabilities = strictly_ascending_set(sequence, &self)?;
                let closed = SymbolQuery::new().capabilities(capabilities.iter().copied());
                if *closed.capability_set() != capabilities {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Other("capabilities missing a prerequisite"),
                        &self,
                    ));
                }
                Ok(CompletedCapabilities(capabilities))
            }
        }

        deserializer.deserialize_seq(CapabilitiesVisitor)
    }
}

/// Reads a sequence that must be non-empty and strictly ascending, so a
/// duplicate or a reordered list is rejected rather than normalized.
fn strictly_ascending_set<'de, T, A>(
    mut sequence: A,
    expected: &dyn de::Expected,
) -> Result<BTreeSet<T>, A::Error>
where
    T: Deserialize<'de> + Ord,
    A: SeqAccess<'de>,
{
    let mut items = BTreeSet::new();
    let mut count = 0usize;
    while let Some(item) = sequence.next_element::<T>()? {
        if items.last().is_some_and(|last| *last >= item) {
            return Err(de::Error::invalid_value(
                de::Unexpected::Other("an unordered or repeated element"),
                expected,
            ));
        }
        items.insert(item);
        count += 1;
    }
    if count == 0 {
        return Err(de::Error::invalid_length(0, expected));
    }
    Ok(items)
}
