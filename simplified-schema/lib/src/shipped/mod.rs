//! Checked-in prepared forms of compiled-in schema sources.
//!
//! A crate that compiles schema text into its binary (a *shipped source*)
//! can also check in an *artifact*: one JSON file holding everything a run
//! would otherwise derive from that text before it reads a user document.
//! At run time the owner loads the artifact instead of preparing the text,
//! but only after [`accept`] proves the artifact describes exactly the text
//! compiled into this binary. Anything else (a missing, malformed, stale, or
//! incompatible artifact) is refused as a whole, and the owner prepares the
//! text as it always has ([`load_or_prepare`]).
//!
//! Three crates own one artifact each, in dependency order: Simplified
//! Schema (its envelope definitions), Darkmatter, and Claudine. This module
//! is the mechanism they share; it knows nothing about its hosts, which pass
//! their source tables, lower dependencies, payload records, and preparation
//! callbacks explicitly. See `simplified-schema/docs/topics/shipped-artifacts.md`.
//!
//! ```mermaid
//! flowchart LR
//!     B[artifact bytes] --> H{header matches<br/>sources, revision,<br/>dependencies, configuration?}
//!     H -->|no| F[refusal: ordinary preparation]
//!     H -->|yes| P{payload records<br/>decode and validate?}
//!     P -->|no| F
//!     P -->|yes| A[AcceptedArtifact]
//! ```
//!
//! ## Header
//!
//! [`ArtifactHeader`] records the wire version ([`WIRE_VERSION`]), the
//! owning crate and its preparation revision, an xxHash digest of every
//! source text the preparation read, the fingerprint of each lower artifact
//! it was prepared over, and any configuration that changes preparation.
//! An owner increments its preparation revision whenever its preparation
//! changes behavior, even when no source text changed.
//!
//! ## Fresh generation
//!
//! [`generate`] encodes a payload the owner prepared with
//! [`PreparationMode::Uncached`]: from current text, consulting no artifact
//! and no process memo along its whole dependency chain. A freshness test
//! compares that output with the checked-in bytes ([`check_fresh`]).

pub(crate) mod records;
pub mod registry;
mod wire;
pub(crate) mod wire_support;

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::embedded::EmbeddedDocument;

pub use records::{NamedTypeTableRecord, PreparedNamedTypes, PreparedSourcesDescriptor};

/// The artifact wire-format version this library reads and writes.
pub const WIRE_VERSION: u32 = 1;

/// An xxHash-64 digest (`biscuit_hash::xx_hash_bytes`), written on the wire
/// as exactly sixteen lowercase hexadecimal digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Digest(pub u64);

impl Digest {
    /// The digest of `bytes`.
    pub fn of(bytes: &[u8]) -> Self {
        Self(biscuit_hash::xx_hash_bytes(bytes))
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:016x}", self.0)
    }
}

impl Serialize for Digest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text.len() != 16 || !text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
            return Err(serde::de::Error::custom(format!(
                "a digest is sixteen lowercase hexadecimal digits, not `{text}`"
            )));
        }
        u64::from_str_radix(&text, 16).map(Self).map_err(serde::de::Error::custom)
    }
}

/// One source text a preparation read: its portable virtual identity (a
/// repository-relative path with `/` separators) and the digest of its exact
/// compiled bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFingerprint {
    pub identity: String,
    pub digest: Digest,
}

impl SourceFingerprint {
    /// The fingerprint of `document`'s text under its path.
    pub fn of(document: &EmbeddedDocument<'_>) -> Self {
        Self { identity: document.path.to_string(), digest: Digest::of(document.text.as_bytes()) }
    }
}

/// One lower artifact a preparation was built over: its owner, that owner's
/// preparation revision, and the digest of the lower artifact's bytes as
/// fresh generation produced them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyFingerprint {
    pub owner: String,
    pub preparation_revision: u32,
    pub digest: Digest,
}

impl DependencyFingerprint {
    /// The fingerprint of freshly generated lower `bytes`, for generation.
    pub fn of_bytes(owner: &str, preparation_revision: u32, bytes: &[u8]) -> Self {
        Self { owner: owner.to_string(), preparation_revision, digest: Digest::of(bytes) }
    }
}

/// The header every artifact starts with. Lists are sorted by identity and
/// hold no duplicates; an empty list is an explicit value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactHeader {
    pub wire_version: u32,
    pub owner: String,
    pub preparation_revision: u32,
    pub sources: Vec<SourceFingerprint>,
    pub dependencies: Vec<DependencyFingerprint>,
    pub configuration: Vec<(String, String)>,
}

/// The state of one lower artifact an owner's artifact depends on, as the
/// owner expects it now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LowerArtifact {
    /// The lower artifact is current: accepted at run time
    /// ([`AcceptedArtifact::as_dependency`]) or freshly generated
    /// ([`DependencyFingerprint::of_bytes`]).
    Current(DependencyFingerprint),
    /// The lower artifact was refused, so nothing prepared over it can be
    /// trusted: every artifact that depends on it is refused too.
    Refused { owner: String },
}

/// What an owner expects its artifact to describe: the text compiled into
/// this binary, the current lower artifacts, and the current configuration.
#[derive(Clone, Copy, Debug)]
pub struct ArtifactDescriptor<'a> {
    /// The owning crate (`simplified-schema`).
    pub owner: &'a str,
    /// The owner's current preparation revision (at least 1).
    pub preparation_revision: u32,
    /// Every source text the owner's preparation reads, complete.
    pub sources: &'a [EmbeddedDocument<'a>],
    /// Every lower artifact the preparation is built over.
    pub dependencies: &'a [LowerArtifact],
    /// Configuration that changes preparation, as `(key, value)` pairs.
    pub configuration: &'a [(&'a str, &'a str)],
}

impl ArtifactDescriptor<'_> {
    /// The header an artifact generated now must carry.
    ///
    /// ## Errors
    ///
    /// A message when the descriptor is not generable: a zero revision, an
    /// empty owner, a duplicate source, dependency, or configuration key, or
    /// a refused dependency.
    pub fn header(&self) -> Result<ArtifactHeader, String> {
        if self.owner.is_empty() {
            return Err("an artifact owner is a non-empty name".into());
        }
        if self.preparation_revision == 0 {
            return Err(format!("`{}`'s preparation revision must be at least 1", self.owner));
        }
        let mut sources: Vec<SourceFingerprint> = self.sources.iter().map(SourceFingerprint::of).collect();
        sources.sort_by(|left, right| left.identity.cmp(&right.identity));
        if let Some(pair) = sources.windows(2).find(|pair| pair[0].identity == pair[1].identity) {
            return Err(format!("source `{}` is listed twice", pair[0].identity));
        }
        let mut dependencies = Vec::with_capacity(self.dependencies.len());
        for dependency in self.dependencies {
            match dependency {
                LowerArtifact::Current(fingerprint) => dependencies.push(fingerprint.clone()),
                LowerArtifact::Refused { owner } => {
                    return Err(format!("lower artifact `{owner}` is not current"));
                }
            }
        }
        dependencies.sort_by(|left, right| left.owner.cmp(&right.owner));
        if let Some(pair) = dependencies.windows(2).find(|pair| pair[0].owner == pair[1].owner) {
            return Err(format!("dependency `{}` is listed twice", pair[0].owner));
        }
        let mut configuration: Vec<(String, String)> =
            self.configuration.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
        configuration.sort();
        if let Some(pair) = configuration.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            return Err(format!("configuration key `{}` is listed twice", pair[0].0));
        }
        Ok(ArtifactHeader {
            wire_version: WIRE_VERSION,
            owner: self.owner.to_string(),
            preparation_revision: self.preparation_revision,
            sources,
            dependencies,
            configuration,
        })
    }
}

/// Whether a caller prepares from compiled text consulting process state, or
/// freshly for generation.
#[derive(Clone, Copy, Debug, Default)]
pub enum PreparationMode<'a> {
    /// Ordinary preparation: accepted artifacts, registries, and process
    /// memos are consulted.
    #[default]
    Runtime,
    /// Generation: no artifact, registry, or memo is consulted along the
    /// whole dependency chain; the freshly prepared lower data is passed in.
    Uncached(&'a FreshLower),
}

/// What an uncached preparation is built over: the library's own envelope
/// definitions, prepared from current text for this one generation, and the
/// library artifact that preparation generates.
pub struct FreshLower {
    pub(crate) envelopes: crate::envelope::EnvelopeDefinitions,
    artifact: Vec<u8>,
}

impl FreshLower {
    /// Prepares the library's envelope definitions from their compiled text,
    /// runs their generation checks, and generates the library's artifact,
    /// bypassing the checked-in artifact and the process registry.
    ///
    /// ## Errors
    ///
    /// A message when a definition does not prepare or fails a generation
    /// check.
    pub fn prepare() -> Result<Self, String> {
        crate::note_work!(ArtifactGeneration);
        let envelopes = crate::envelope::EnvelopeDefinitions::prepare_uncached().map_err(|error| error.to_string())?;
        let payload = registry::prepare_payload(&envelopes)?;
        let artifact = generate(&registry::descriptor(), &payload)?;
        Ok(Self { envelopes, artifact })
    }

    /// The library artifact this preparation generated.
    pub fn artifact(&self) -> &[u8] {
        &self.artifact
    }

    /// The generated library artifact as a dependency of a host's artifact.
    pub fn as_dependency(&self) -> LowerArtifact {
        LowerArtifact::Current(DependencyFingerprint::of_bytes(
            registry::OWNER,
            registry::PREPARATION_REVISION,
            &self.artifact,
        ))
    }
}

impl fmt::Debug for FreshLower {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("FreshLower").field("artifact_bytes", &self.artifact.len()).finish()
    }
}

/// Why an artifact was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FallbackReason {
    /// No artifact bytes were supplied.
    MissingArtifact,
    /// The bytes are not one well-formed artifact document: not UTF-8, not
    /// JSON, a repeated key, trailing content, a header or record field of
    /// the wrong shape, or an invalid header value.
    MalformedArtifact,
    /// The artifact's wire version is not [`WIRE_VERSION`].
    UnsupportedWireVersion,
    /// The owner's preparation revision changed since generation.
    PreparationRevisionMismatch,
    /// A source text differs, or the source inventory changed.
    SourceMismatch,
    /// A lower artifact differs, changed revision, or was refused.
    DependencyMismatch,
    /// Preparation configuration differs.
    ConfigurationMismatch,
    /// The artifact belongs to another owner, or a payload record breaks an
    /// invariant (a dangling reference, a duplicate identity, a file-backed
    /// namespace).
    UnusableRecord,
}

impl FallbackReason {
    /// Every reason, in declaration order.
    pub const ALL: &[FallbackReason] = &[
        Self::MissingArtifact,
        Self::MalformedArtifact,
        Self::UnsupportedWireVersion,
        Self::PreparationRevisionMismatch,
        Self::SourceMismatch,
        Self::DependencyMismatch,
        Self::ConfigurationMismatch,
        Self::UnusableRecord,
    ];

    #[cfg(feature = "work-counters")]
    fn work(self) -> crate::work_counters::Work {
        use crate::work_counters::Work;
        match self {
            Self::MissingArtifact => Work::ArtifactFallbackMissing,
            Self::MalformedArtifact => Work::ArtifactFallbackMalformed,
            Self::UnsupportedWireVersion => Work::ArtifactFallbackWireVersion,
            Self::PreparationRevisionMismatch => Work::ArtifactFallbackRevision,
            Self::SourceMismatch => Work::ArtifactFallbackSource,
            Self::DependencyMismatch => Work::ArtifactFallbackDependency,
            Self::ConfigurationMismatch => Work::ArtifactFallbackConfiguration,
            Self::UnusableRecord => Work::ArtifactFallbackRecord,
        }
    }
}

/// An artifact's refusal: why, and where in the artifact when a position is
/// known. Never published to users; generation commands render it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub reason: FallbackReason,
    /// The JSON pointer of the offending member (`/header/sources/0/digest`).
    pub pointer: Option<String>,
    /// 1-based line of the offending position in the artifact.
    pub line: Option<usize>,
    /// 1-based column of the offending position in the artifact.
    pub column: Option<usize>,
    pub detail: String,
}

impl Refusal {
    fn at(reason: FallbackReason, pointer: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { reason, pointer: Some(pointer.into()), line: None, column: None, detail: detail.into() }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self.reason)?;
        if let Some(pointer) = &self.pointer {
            write!(formatter, " at `{pointer}`")?;
        }
        if let (Some(line), Some(column)) = (self.line, self.column) {
            write!(formatter, " (line {line}, column {column})")?;
        }
        write!(formatter, ": {}", self.detail)
    }
}

impl std::error::Error for Refusal {}

/// A payload record that breaks an invariant, with the JSON pointer of the
/// record relative to the payload (`/definitions/1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordError {
    pub pointer: String,
    pub message: String,
}

impl RecordError {
    pub fn new(pointer: impl Into<String>, message: impl Into<String>) -> Self {
        Self { pointer: pointer.into(), message: message.into() }
    }
}

/// The scope of one artifact's acceptance, handed to its payload
/// validation: the header already matched. Only [`accept`] constructs one,
/// so a record type that takes it (such as
/// [`PreparedNamedTypes::from_records`]) can only be built from an artifact
/// being accepted.
pub struct Acceptance<'a> {
    header: &'a ArtifactHeader,
}

impl Acceptance<'_> {
    /// The accepted header.
    pub fn header(&self) -> &ArtifactHeader {
        self.header
    }
}

/// An artifact whose header matched what its owner expects and whose payload
/// decoded and validated. Only [`accept`] constructs one, so holding one is
/// the proof that its entries may skip the checks generation already ran.
#[derive(Debug)]
pub struct AcceptedArtifact<T> {
    header: ArtifactHeader,
    digest: Digest,
    payload: T,
}

impl<T> AcceptedArtifact<T> {
    pub fn header(&self) -> &ArtifactHeader {
        &self.header
    }

    pub fn payload(&self) -> &T {
        &self.payload
    }

    /// This artifact as the current state of a dependency of a higher
    /// artifact. Its bytes are the bytes fresh generation produces, since the
    /// header proved them current.
    pub fn as_dependency(&self) -> LowerArtifact {
        LowerArtifact::Current(DependencyFingerprint {
            owner: self.header.owner.clone(),
            preparation_revision: self.header.preparation_revision,
            digest: self.digest,
        })
    }

    /// The same artifact with its payload transformed, keeping the proof.
    pub fn map<U>(self, transform: impl FnOnce(T) -> U) -> AcceptedArtifact<U> {
        AcceptedArtifact { header: self.header, digest: self.digest, payload: transform(self.payload) }
    }
}

/// The result of [`load_or_prepare`]: the accepted artifact, or the owner's
/// ordinary preparation and the reason the artifact was refused.
#[derive(Debug)]
pub enum Loaded<T, P> {
    Accepted(AcceptedArtifact<T>),
    Prepared { prepared: P, refusal: Refusal },
}

/// Encodes `payload` as the artifact `descriptor` describes.
///
/// ## Errors
///
/// A message when the descriptor is not generable ([`ArtifactDescriptor::header`])
/// or a payload value is not representable.
pub fn generate<T: Serialize>(descriptor: &ArtifactDescriptor<'_>, payload: &T) -> Result<Vec<u8>, String> {
    wire::encode(&descriptor.header()?, payload)
}

/// Accepts `bytes` as the artifact `expected` describes: the document shape,
/// the header, then the payload records `validate` checks.
///
/// ## Errors
///
/// The [`Refusal`] of the first check that failed. Nothing of a refused
/// artifact is exposed.
pub fn accept<T, U>(
    expected: &ArtifactDescriptor<'_>,
    bytes: Option<&[u8]>,
    validate: impl FnOnce(&Acceptance<'_>, T) -> Result<U, RecordError>,
) -> Result<AcceptedArtifact<U>, Refusal>
where
    T: serde::de::DeserializeOwned,
{
    let accepted = accept_inner(expected, bytes, validate);
    #[cfg(feature = "work-counters")]
    match &accepted {
        Ok(_) => crate::work_counters::add(crate::work_counters::Work::ArtifactAccept, 1),
        Err(refusal) => crate::work_counters::add(refusal.reason.work(), 1),
    }
    accepted
}

fn accept_inner<T, U>(
    expected: &ArtifactDescriptor<'_>,
    bytes: Option<&[u8]>,
    validate: impl FnOnce(&Acceptance<'_>, T) -> Result<U, RecordError>,
) -> Result<AcceptedArtifact<U>, Refusal>
where
    T: serde::de::DeserializeOwned,
{
    let bytes = bytes.ok_or_else(|| Refusal {
        reason: FallbackReason::MissingArtifact,
        pointer: None,
        line: None,
        column: None,
        detail: format!("no `{}` artifact was supplied", expected.owner),
    })?;
    crate::note_work!(ArtifactDecode);
    let decoded = wire::decode_header(bytes)?;
    check_header(&decoded.header, expected)?;
    let payload: T = wire::decode_payload(&decoded)?;
    let payload = validate(&Acceptance { header: &decoded.header }, payload).map_err(|error| {
        Refusal::at(FallbackReason::UnusableRecord, format!("/payload{}", error.pointer), error.message)
    })?;
    Ok(AcceptedArtifact { header: decoded.header, digest: Digest::of(bytes), payload })
}

/// Compares a decoded header with what its owner expects, value by value.
fn check_header(header: &ArtifactHeader, expected: &ArtifactDescriptor<'_>) -> Result<(), Refusal> {
    use FallbackReason::*;
    if header.wire_version != WIRE_VERSION {
        return Err(Refusal::at(
            UnsupportedWireVersion,
            "/header/wire_version",
            format!("wire version {} is not {WIRE_VERSION}", header.wire_version),
        ));
    }
    if header.owner.is_empty() {
        return Err(Refusal::at(MalformedArtifact, "/header/owner", "an artifact owner is a non-empty name"));
    }
    if header.owner != expected.owner {
        return Err(Refusal::at(
            UnusableRecord,
            "/header/owner",
            format!("the artifact belongs to `{}`, not `{}`", header.owner, expected.owner),
        ));
    }
    if header.preparation_revision == 0 {
        return Err(Refusal::at(MalformedArtifact, "/header/preparation_revision", "a preparation revision is at least 1"));
    }
    if header.preparation_revision != expected.preparation_revision {
        return Err(Refusal::at(
            PreparationRevisionMismatch,
            "/header/preparation_revision",
            format!("prepared at revision {}, current is {}", header.preparation_revision, expected.preparation_revision),
        ));
    }
    check_sorted(header.sources.iter().map(|source| source.identity.as_str()), "/header/sources", "identity")?;
    check_sorted(header.dependencies.iter().map(|dependency| dependency.owner.as_str()), "/header/dependencies", "owner")?;
    check_sorted(header.configuration.iter().map(|(key, _)| key.as_str()), "/header/configuration", "")?;
    for (index, dependency) in header.dependencies.iter().enumerate() {
        if dependency.preparation_revision == 0 {
            return Err(Refusal::at(
                MalformedArtifact,
                format!("/header/dependencies/{index}/preparation_revision"),
                "a preparation revision is at least 1",
            ));
        }
    }

    let current = expected.header_unchecked();
    if header.sources != current.sources {
        let (pointer, detail) = first_source_difference(&header.sources, &current.sources);
        return Err(Refusal::at(SourceMismatch, pointer, detail));
    }
    if let Some(LowerArtifact::Refused { owner }) =
        expected.dependencies.iter().find(|dependency| matches!(dependency, LowerArtifact::Refused { .. }))
    {
        return Err(Refusal::at(DependencyMismatch, "/header/dependencies", format!("lower artifact `{owner}` was refused")));
    }
    if header.dependencies != current.dependencies {
        return Err(Refusal::at(
            DependencyMismatch,
            "/header/dependencies",
            "a lower artifact differs from the one this artifact was prepared over",
        ));
    }
    if header.configuration != current.configuration {
        return Err(Refusal::at(ConfigurationMismatch, "/header/configuration", "the preparation configuration differs"));
    }
    Ok(())
}

impl ArtifactDescriptor<'_> {
    /// The expected header's lists, sorted, without the generation checks: a
    /// refused dependency is reported by the caller.
    fn header_unchecked(&self) -> ArtifactHeader {
        let mut sources: Vec<SourceFingerprint> = self.sources.iter().map(SourceFingerprint::of).collect();
        sources.sort_by(|left, right| left.identity.cmp(&right.identity));
        let mut dependencies: Vec<DependencyFingerprint> = self
            .dependencies
            .iter()
            .filter_map(|dependency| match dependency {
                LowerArtifact::Current(fingerprint) => Some(fingerprint.clone()),
                LowerArtifact::Refused { .. } => None,
            })
            .collect();
        dependencies.sort_by(|left, right| left.owner.cmp(&right.owner));
        let mut configuration: Vec<(String, String)> =
            self.configuration.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
        configuration.sort();
        ArtifactHeader {
            wire_version: WIRE_VERSION,
            owner: self.owner.to_string(),
            preparation_revision: self.preparation_revision,
            sources,
            dependencies,
            configuration,
        }
    }
}

/// Refuses a list whose keys are empty, unsorted, or repeated.
fn check_sorted<'k>(keys: impl Iterator<Item = &'k str>, pointer: &str, field: &str) -> Result<(), Refusal> {
    let member = |index: usize| if field.is_empty() { format!("{pointer}/{index}/0") } else { format!("{pointer}/{index}/{field}") };
    let mut previous: Option<&str> = None;
    for (index, key) in keys.enumerate() {
        if key.is_empty() {
            return Err(Refusal::at(FallbackReason::MalformedArtifact, member(index), "an identity is a non-empty string"));
        }
        if let Some(previous) = previous {
            if previous == key {
                return Err(Refusal::at(FallbackReason::MalformedArtifact, member(index), format!("`{key}` is listed twice")));
            }
            if previous > key {
                return Err(Refusal::at(FallbackReason::MalformedArtifact, member(index), format!("`{key}` is out of order")));
            }
        }
        previous = Some(key);
    }
    Ok(())
}

/// The pointer and description of the first difference between recorded and
/// current source inventories.
fn first_source_difference(recorded: &[SourceFingerprint], current: &[SourceFingerprint]) -> (String, String) {
    for (index, (left, right)) in recorded.iter().zip(current).enumerate() {
        if left.identity != right.identity {
            return (
                format!("/header/sources/{index}/identity"),
                format!("the artifact lists `{}` where `{}` is compiled in", left.identity, right.identity),
            );
        }
        if left.digest != right.digest {
            return (format!("/header/sources/{index}/digest"), format!("`{}` changed since generation", left.identity));
        }
    }
    (
        "/header/sources".into(),
        format!("the artifact lists {} sources, {} are compiled in", recorded.len(), current.len()),
    )
}

/// Accepts `bytes` or falls back to `prepare`, recording which happened.
pub fn load_or_prepare<T, U, P>(
    expected: &ArtifactDescriptor<'_>,
    bytes: Option<&[u8]>,
    validate: impl FnOnce(&Acceptance<'_>, T) -> Result<U, RecordError>,
    prepare: impl FnOnce() -> P,
) -> Loaded<U, P>
where
    T: serde::de::DeserializeOwned,
{
    match accept(expected, bytes, validate) {
        Ok(accepted) => Loaded::Accepted(accepted),
        Err(refusal) => {
            tracing::debug!(owner = expected.owner, %refusal, "shipped artifact refused; preparing compiled sources");
            Loaded::Prepared { prepared: prepare(), refusal }
        }
    }
}

/// A checked-in artifact that differs from fresh generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaleArtifact {
    /// The artifact's repository-relative path.
    pub artifact: String,
    /// The first line (1-based) that differs, if both have one.
    pub line: usize,
    /// The command that regenerates it.
    pub command: String,
}

impl fmt::Display for StaleArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "`{}` is stale (first difference at line {}); regenerate it with `{}`",
            self.artifact, self.line, self.command
        )
    }
}

impl std::error::Error for StaleArtifact {}

/// Compares checked-in artifact bytes with freshly generated ones.
///
/// ## Errors
///
/// [`StaleArtifact`] naming the first differing line and `command`.
pub fn check_fresh(checked_in: &[u8], fresh: &[u8], artifact: &str, command: &str) -> Result<(), StaleArtifact> {
    if checked_in == fresh {
        return Ok(());
    }
    let mut line = 1;
    for (left, right) in checked_in.split(|byte| *byte == b'\n').zip(fresh.split(|byte| *byte == b'\n')) {
        if left != right {
            break;
        }
        line += 1;
    }
    Err(StaleArtifact { artifact: artifact.to_string(), line, command: command.to_string() })
}
