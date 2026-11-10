//! Error types for the schemas subsystem.
//!
//! The terminal presentation of [`SchemaError`] (its `BlockError`
//! implementation) is compiled only with the opt-in `terminal` feature: the
//! trait and the type live in other crates than the host that renders them,
//! so the implementation has to live here.
//!
//! Phase 1 introduced [`SchemaError::Grammar`] for parser/lexer failures.
//! Phase 2 added [`SchemaError::Convert`] for failures that surface while
//! lowering a `SchemaRoot` to JSON Schema. Phase 3 adds the runtime
//! variants used by the resolution and validation pipeline:
//!
//! - [`SchemaError::Unresolved`] — a `$schema` file reference could not be
//!   resolved (invalid syntax or a resolver error).
//! - [`SchemaError::NoMatch`] — a `$schema` file reference matched no file.
//! - [`SchemaError::NotASchemaDocument`] — a referenced file is not a schema
//!   definition a document's `$schema` may target ([`NotSchemaReason`]).
//! - [`SchemaError::UnsupportedJsonDialect`] — a referenced JSON file declares
//!   a JSON Schema dialect other than Draft 2020-12.
//! - [`SchemaError::InvalidJsonSchema`] — a referenced Draft 2020-12 JSON file
//!   fails the dialect's meta-schema.
//! - [`SchemaError::RemoteUnsupported`] — a `$schema` value is an `http(s)://`
//!   URL; remote schemas are not supported in v1.
//! - [`SchemaError::Baseline`] — the baseline schema could not be loaded or
//!   violates the "simple object schema" restriction.
//! - [`SchemaError::BuildValidator`] — `jsonschema` rejected the lowered JSON
//!   Schema at validator-construction time.
//! - [`SchemaError::Io`] — filesystem I/O failed while reading a referenced
//!   file.
//! - [`SchemaError::UnresolvedSlots`] — the schema is a template holding
//!   `{{ … }}` slots and cannot be lowered until they are resolved.
//! - [`SchemaError::SlotResult`] — a resolved slot's value cannot fill its
//!   position.
//! - [`SchemaError::SlotResolution`] — a host resolved a template's slots
//!   and the resolution failed ([`SlotResolutionFailure`]).
//! - [`SchemaError::FrontmatterShape`] — `$schema` is present in frontmatter
//!   but is not a YAML mapping/sequence/string.

use std::{
    fmt,
    ops::Range,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::span::SourceSpan;

/// A secondary location of a [`SchemaError::Grammar`] failure: the earlier
/// declaration a duplicate or a disagreeing overload conflicts with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarRelated {
    /// What the location is, as an editor would label it.
    pub message: String,
    /// Byte range in the same type-expression text as the primary `span`,
    /// unless `arm` names another option.
    pub span: Range<usize>,
    /// Set only when the failing definition is a YAML sequence union, whose
    /// options are separate scalars: the zero-based option whose text `span`
    /// is in. The primary failure is then located at a sibling option.
    pub arm: Option<usize>,
}

/// One reference of a rejected [`SchemaError::NonDescendingCycle`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleReference {
    /// The authored reference: the source and path of the named type or
    /// property that writes it.
    pub site: SchemaReferenceSite,
    /// The named type it refers to.
    pub target: String,
    /// The source whose named types declare `target`.
    pub target_source: SchemaSourceId,
    /// Byte range of the reference's name in `site.source`, when the
    /// resolver held that source's text (a schema file). A reference in the
    /// inline `$schema` is projected by the consumer that holds the document,
    /// as for [`SchemaError::UnknownNamedType`].
    pub span: Option<Range<usize>>,
}

use super::simplified::{
    SchemaConstraintConflictKind, SchemaConstraintParticipant, SchemaReferenceSite,
    SchemaSourceId, SchemaUnionRejection, Slot,
};


/// A conflict participant's diagnostic excerpt as a Markdown code span,
/// fenced so a backtick in the value cannot close it early.
fn participant_code_span(participant: &SchemaConstraintParticipant) -> String {
    renderable::markdown::code_span(&participant.constraint.diagnostic_excerpt())
}

/// Why a YAML document's `kind` selects no definition
/// ([`SchemaError::KindSelection`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KindSelectionCause {
    /// Two or more definitions claim the kind; `files` names each, sorted
    /// by path (a compiled-in definition by its repository-relative path).
    Duplicate { files: Vec<PathBuf> },
    /// The definition that claims the kind failed; `defect` is its failure,
    /// and `code` the host's diagnostic code for it when the host gives the
    /// failure one of its own (a kind definition holding schema slots).
    Reserved { file: PathBuf, defect: String, code: Option<&'static str> },
    /// No healthy definition claims the kind, and `file` is a definition
    /// that failed before revealing what it claims, so the kind may be its;
    /// `code` as for [`Self::Reserved`].
    Unavailable { file: PathBuf, defect: String, code: Option<&'static str> },
    /// A schema root could not be read, so its definitions are unknown.
    RootUnreadable { root: PathBuf, defect: String },
}

impl fmt::Display for KindSelectionCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate { files } => {
                let files: Vec<String> = files.iter().map(|file| format!("`{}`", file.display())).collect();
                write!(f, "it is claimed by more than one definition: {}", files.join(", "))
            }
            Self::Reserved { file, defect, .. } => {
                write!(f, "its definition `{}` failed: {defect}", file.display())
            }
            Self::Unavailable { file, defect, .. } => write!(
                f,
                "schema selection is unavailable: definition `{}` failed before revealing its kind: {defect}",
                file.display()
            ),
            Self::RootUnreadable { root, defect } => {
                write!(f, "schema root `{}` could not be read: {defect}", root.display())
            }
        }
    }
}

/// The Draft 2020-12 dialect URI, the only JSON Schema dialect a referenced
/// `.json` schema file may declare.
pub const DRAFT_2020_12_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Why a referenced file is not a schema definition
/// ([`SchemaError::NotASchemaDocument`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotSchemaReason {
    /// A tagged `kind: schema` document with `types` and no `$schema`: it
    /// serves named-type imports but declares no document properties.
    TypesOnly,
    /// A YAML mapping with no `$schema` property (or another `kind`).
    NoSchemaProperty,
    /// The document root is not a mapping (a JSON object for `.json`).
    NotAMapping,
    /// A JSON object without a string `$schema` dialect declaration.
    JsonMissingDialect,
    /// A `kind: yaml-schema` document: it defines a YAML document kind and is
    /// applied to documents by their `kind`, so it is neither a `$schema`
    /// target nor a `Name@file` import source.
    SelectedByKind,
}

impl NotSchemaReason {
    /// The migration hint for this reason in a file at `path`, when there is
    /// one. A non-object `.json` root gets the same dialect hint as a missing
    /// declaration.
    pub fn hint(self, path: &Path) -> Option<&'static str> {
        match self {
            NotSchemaReason::TypesOnly => Some("use a named-type import (`Name@file.yaml`) to reuse its types"),
            NotSchemaReason::JsonMissingDialect => Some(DECLARE_DIALECT_HINT),
            NotSchemaReason::NotAMapping if is_json_path(path) => Some(DECLARE_DIALECT_HINT),
            NotSchemaReason::SelectedByKind => {
                Some("move shared types into a `kind: schema` file and import them from there")
            }
            NotSchemaReason::NotAMapping | NotSchemaReason::NoSchemaProperty => None,
        }
    }
}

const DECLARE_DIALECT_HINT: &str = "declare `\"$schema\": \"https://json-schema.org/draft/2020-12/schema\"`";

fn is_json_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

impl fmt::Display for NotSchemaReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NotSchemaReason::TypesOnly => "it declares only `types`, with no `$schema` property",
            NotSchemaReason::NoSchemaProperty => "it has no `$schema` property",
            NotSchemaReason::NotAMapping => "its root is not a mapping",
            NotSchemaReason::JsonMissingDialect => "it does not declare its JSON Schema dialect in `$schema`",
            NotSchemaReason::SelectedByKind => {
                "it is a `yaml-schema`; it is selected by `kind`, not referenced"
            }
        })
    }
}

/// The `{{ … }}` slots that keep a schema from being lowered.
///
/// Each slot carries its authored span (in the type-expression string, or
/// the whole YAML or Markdown document once source-aware parsing projected
/// it) and its defining file when it was reached through an import.
///
/// `pending` is the handle that resolves the template: set by the public
/// resolution entry points (`resolve::resolve_yaml_schema_owned_by` and the
/// functions delegating to it, `resolve::prepare_standalone_yaml_schema`),
/// `None` where the schema cannot be resolved from the call that met it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct UnresolvedSlots {
    /// Every slot of the schema, in declaration order, each once.
    pub slots: Vec<Slot>,
    /// How to resolve the template, when the call that met it can.
    pub pending: Option<crate::slots::PendingSchema>,
    /// The expanded template the slots were found in, kept for
    /// [`crate::slots::PendingSchema::template`].
    pub(crate) template: Option<crate::slots::Template>,
}

impl UnresolvedSlots {
    /// The unresolved slots `slots`, with no resolution handle.
    pub fn new(slots: Vec<Slot>) -> Self {
        Self { slots, pending: None, template: None }
    }

    /// `slots`, found in the expanded `template`.
    pub(crate) fn with_template(slots: Vec<Slot>, template: crate::slots::Template) -> Self {
        Self { slots, pending: None, template: Some(template) }
    }
}

/// A secondary location of a [`SchemaError::SlotResult`]: the imported file
/// that holds a rejected definition, or the authored suffix a returned type
/// conflicts with. `file` and `span` are `None` when no source text proves
/// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotRelated {
    /// What this location contributes to the failure, in words.
    pub message: String,
    /// The file `span` is a byte range of; `None` for the slot's own text.
    pub file: Option<PathBuf>,
    /// Where the participant is.
    pub span: Option<SourceSpan>,
}

/// What is wrong with a value a `{{ … }}` slot produced
/// ([`SchemaError::SlotResult`]).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SlotResultKind {
    /// The list's slots produced no element and the list has no literal one.
    Empty,
    /// The value is not the JSON shape the position takes.
    WrongShape {
        /// The shape the position takes.
        expected: &'static str,
        /// The JSON kind the slot produced (`null`, `boolean`, `number`,
        /// `string`, `array`, `mapping`), naming the element for a list.
        found: String,
    },
    /// The value has the right shape but the position cannot take it: a
    /// number outside the numeric model, a negative or fractional count, a
    /// bound that crosses its counterpart, a default the type rejects or that
    /// disagrees with another.
    Inadmissible(String),
    /// A slot-supplied pattern the linear regex engine must not take: a
    /// lookaround, a backreference, more than
    /// [`MAX_SLOT_PATTERN_BYTES`](crate::slots::MAX_SLOT_PATTERN_BYTES), or a
    /// pattern that does not compile within the default size limit.
    PatternRejected(String),
    /// A template type's value is not a type definition: a JSON shape other
    /// than a string or an array, or text the ordinary definition parser
    /// rejects (the parser's error is kept unchanged). Shared, so the kind
    /// stays cheap to clone.
    NotATypeDefinition(std::sync::Arc<SchemaError>),
    /// A template type's returned definition holds an active `{{ … }}` slot
    /// of its own (a template constraint included): returned data never
    /// causes another expression to run.
    NestedSlot,
    /// A definition reachable from a template type's returned definition,
    /// through its imports, holds an active `{{ … }}` slot.
    ReachableSlot,
    /// A template type's returned definition names a local type (`Name` or
    /// `Name@this`); returned data has no schema of its own to name types
    /// from.
    LocalReference(String),
    /// A template type's returned definition conflicts with the suffix
    /// written after the slot (a default or constraint the merge rejects).
    SuffixConflict(String),
    /// A template type's returned definition is a valid definition that
    /// cannot stand at the slot's position once expanded (an invalid
    /// `non-null`, a union or tuple invariant).
    InvalidExpansion(String),
}

impl PartialEq for SlotResultKind {
    /// Compares by meaning: a wrapped definition error compares by its
    /// message, since [`SchemaError`] has no structural equality.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) | (Self::NestedSlot, Self::NestedSlot) | (Self::ReachableSlot, Self::ReachableSlot) => true,
            (Self::WrongShape { expected: a, found: b }, Self::WrongShape { expected: c, found: d }) => a == c && b == d,
            (Self::Inadmissible(a), Self::Inadmissible(b))
            | (Self::PatternRejected(a), Self::PatternRejected(b))
            | (Self::LocalReference(a), Self::LocalReference(b))
            | (Self::SuffixConflict(a), Self::SuffixConflict(b))
            | (Self::InvalidExpansion(a), Self::InvalidExpansion(b)) => a == b,
            (Self::NotATypeDefinition(a), Self::NotATypeDefinition(b)) => a.to_string() == b.to_string(),
            _ => false,
        }
    }
}

impl fmt::Display for SlotResultKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the list is empty: its slots produced no element and it has no literal one"),
            Self::WrongShape { expected, found } => write!(f, "expected {expected}, found {found}"),
            Self::Inadmissible(reason) | Self::PatternRejected(reason) => f.write_str(reason),
            Self::NotATypeDefinition(error) => write!(f, "it is not a type definition: {error}"),
            Self::NestedSlot => f.write_str(
                "the returned type definition holds a `{{ … }}` slot; returned data cannot run another expression",
            ),
            Self::ReachableSlot => f.write_str(
                "a type the returned definition imports holds a `{{ … }}` slot; returned data cannot run another expression",
            ),
            Self::LocalReference(name) => write!(
                f,
                "the returned type definition names the local type `{name}`; returned data can only import named \
                 types from a file (`{name}@./file.yaml`)"
            ),
            Self::SuffixConflict(reason) => write!(f, "the returned type conflicts with the written suffix: {reason}"),
            Self::InvalidExpansion(reason) => f.write_str(reason),
        }
    }
}

/// A failed resolution of a template ([`SchemaError::SlotResolution`]),
/// with the schema texts the resolution read, and the text its inline
/// value was written in, so each slot's span can be located in its defining
/// file ([`Self::slot_text`]) without reading it again.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SlotResolutionFailure {
    /// Why no schema was produced. Shared, so a host can cache the failure
    /// and report it again.
    pub error: std::sync::Arc<crate::slots::SlotResolutionError>,
    /// Every schema file the template's first pass read, with the text
    /// read ([`crate::slots::PendingSchema::read_sources`]).
    pub sources: std::sync::Arc<[(PathBuf, std::sync::Arc<str>)]>,
    /// The template's authored inline value
    /// ([`crate::slots::PendingSchema::authored_source`]).
    authored: Option<crate::slots::pending::AuthoredText>,
}

impl SlotResolutionFailure {
    /// The failure `error` of a template that read `sources`, with no
    /// authored inline value.
    pub fn new(
        error: std::sync::Arc<crate::slots::SlotResolutionError>,
        sources: std::sync::Arc<[(PathBuf, std::sync::Arc<str>)]>,
    ) -> Self {
        Self { error, sources, authored: None }
    }

    /// The failure `error` of resolving `pending`: its read texts and its
    /// authored inline value.
    pub fn of_pending(error: std::sync::Arc<crate::slots::SlotResolutionError>, pending: &crate::slots::PendingSchema) -> Self {
        Self { error, sources: pending.read_sources().to_vec().into(), authored: pending.authored_text().cloned() }
    }

    /// The text `slot`'s spans are byte ranges of, with the file it is
    /// (`None`: in-memory text): a schema file the resolution read, which
    /// the slot's origin names, or the inline value the template was
    /// written in ([`crate::slots::PendingSchema::slot_text`]). `None` when
    /// no text proves where the slot is.
    pub fn slot_text(&self, slot: &crate::simplified::Slot) -> Option<(Option<&Path>, &std::sync::Arc<str>)> {
        crate::slots::pending::slot_text(&self.sources, self.authored.as_ref(), slot)
    }
}

impl fmt::Display for SlotResolutionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the schema's slots could not be resolved: {}", self.error)
    }
}

impl fmt::Display for UnresolvedSlots {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.slots.len();
        write!(
            f,
            "the schema holds {count} unresolved `{{{{ … }}}}` slot{}; resolve it before lowering, validating, or layering it",
            if count == 1 { "" } else { "s" }
        )?;
        if let Some(first) = self.slots.first() {
            write!(f, " (first: `{{{{ {} }}}}`)", first.expression)?;
        }
        Ok(())
    }
}

fn not_schema_message(path: &Path, reason: NotSchemaReason) -> String {
    with_hint_line(
        format!("`{}` is not a valid schema definition: {reason}", path.display()),
        reason.hint(path),
    )
}

/// Errors produced by the schemas subsystem.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// Multiple independent property definitions failed conversion.
    ///
    /// Children retain their typed property and source information. Root
    /// structural failures remain standalone variants because traversal
    /// cannot continue meaningfully after them.
    #[error("multiple SimplifiedSchema properties are invalid ({})", errors.len())]
    Aggregate { errors: Vec<SchemaError> },

    /// A typed schema-definition failure originating in a referenced file.
    ///
    /// The wrapper preserves both the source path and the original error tree
    /// so editor clients can retain per-property diagnostics while pointing
    /// related information at the file that authored those properties.
    #[error("referenced schema `{}` is invalid: {source}", path.display())]
    ReferencedSchema {
        path: PathBuf,
        #[source]
        source: Box<SchemaError>,
    },

    /// A SimplifiedSchema failed to parse.
    ///
    /// `property` identifies the offending property (or a synthetic name like
    /// `"<root>"` / `"<arm[N]>"` when the failure is structural).
    /// `message` is a short description; `span` is the byte range within the
    /// originating type-and-constraint string. `related` holds the earlier
    /// declarations a duplicate or disagreeing overload conflicts with;
    /// empty for a failure with one location.
    #[error("invalid SimplifiedSchema for property `{property}`: {message}")]
    Grammar {
        property: String,
        message: String,
        span: Range<usize>,
        related: Vec<GrammarRelated>,
    },

    /// A parsed SimplifiedSchema failed to convert to JSON Schema.
    ///
    /// Raised for constraint/type mismatches (e.g. `integer` on a `string`
    /// atom), conflicting hoisted defaults across union arms, and root-union
    /// arms that are still `FileRef` at conversion time (resolution is a
    /// Phase 3 concern).
    #[error("cannot convert SimplifiedSchema for `{property}` to JSON Schema: {message}")]
    Convert { property: String, message: String },

    /// The schema is a template: it holds `{{ … }}` slots, so it cannot be
    /// lowered to JSON Schema, prepared for validation, or layered until an
    /// explicit resolution step substitutes them. Every lowering entry point
    /// returns this before lowering anything; it is never a placeholder
    /// schema.
    #[error("{0}")]
    UnresolvedSlots(Box<UnresolvedSlots>),

    /// A `{{ … }}` slot's value cannot fill its position. `slot.span` (in
    /// `slot.origin`'s file when set) is where it is reported; resolution
    /// stops at the first such value and publishes nothing.
    #[error("the value of `{{{{ {} }}}}` cannot be used here: {kind}", slot.expression)]
    SlotResult {
        /// The slot that produced the value.
        slot: Box<Slot>,
        /// What is wrong with it.
        kind: SlotResultKind,
        /// The path of the property whose slot it is, when known.
        property: Option<String>,
        /// Other participants: an imported file holding a rejected
        /// definition, or a conflicting authored suffix.
        related: Vec<SlotRelated>,
    },

    /// A read that a template type's returned definition introduced (an
    /// import, an example file, or a path probe) was refused by the host's
    /// read gate before anything was opened. Distinct from a failed or
    /// cyclic import, so a host keeps its grant policy for it.
    #[error("a read needed by `{{{{ {} }}}}` was refused: {refusal}", slot.expression)]
    SlotReadRefused {
        /// The template type whose returned definition needs the read.
        slot: Box<Slot>,
        /// The host's refusal, with its code.
        refusal: crate::slots::SlotAdmissionError,
    },

    /// A host resolved the template's slots and the resolution produced no
    /// schema: a refused or failed slot, a value that cannot fill its
    /// position, or a cancellation. The host's typed causes are kept.
    #[error("{0}")]
    SlotResolution(Box<SlotResolutionFailure>),

    /// A `$schema` file reference could not be resolved: the syntax is
    /// invalid, or biscuit-file rejected it. A reference that resolves but
    /// matches no file is [`SchemaError::NoMatch`].
    #[error("could not resolve $schema reference `{reference}`")]
    Unresolved {
        reference: String,
        #[source]
        source: biscuit_file::FileReferenceError,
    },

    /// A `$schema` file reference matched no file
    /// ([`ResolutionFailure::NoMatch`](biscuit_file::ResolutionFailure::NoMatch)).
    /// `bare_name` is set for a bare schema name that no schema root holds.
    /// The message carries the literal-glob hint
    /// ([`ResolutionFailure::glob_hint`](biscuit_file::ResolutionFailure::glob_hint)).
    #[error("{}", crate::errors::with_glob_hint(format!("could not resolve $schema reference `{reference}`"), biscuit_file::ResolutionFailure::NoMatch, reference))]
    NoMatch { reference: String, bare_name: bool },

    /// A file is not a schema definition a document's `$schema` may target:
    /// a YAML document needs a top-level `$schema`, a JSON file must be an
    /// object declaring the Draft 2020-12 dialect. The message carries the
    /// reason's migration hint ([`NotSchemaReason::hint`]) on a `hint:` line.
    #[error("{}", not_schema_message(path, *reason))]
    NotASchemaDocument { path: PathBuf, reason: NotSchemaReason },

    /// A referenced JSON schema file declares a `$schema` dialect other than
    /// exactly [`DRAFT_2020_12_DIALECT`] (an empty string or a trailing `#`
    /// included).
    #[error(
        "`{path}` declares the unsupported JSON Schema dialect `{declared}`; only \
         `https://json-schema.org/draft/2020-12/schema` is supported"
    )]
    UnsupportedJsonDialect { path: PathBuf, declared: String },

    /// A referenced Draft 2020-12 JSON schema file fails the dialect's
    /// meta-schema. Distinct from a format rejection: the file is a JSON
    /// Schema in the supported dialect, but not a valid one.
    #[error("`{path}` is not a valid Draft 2020-12 JSON Schema: {message}")]
    InvalidJsonSchema { path: PathBuf, message: String },

    /// A recognized standalone SimplifiedSchema envelope is malformed, or a
    /// schema document's text cannot be read as a schema.
    ///
    /// `path` names the document: the file read, or `content` for text the
    /// `SimplifiedSchema` factories were handed directly. When a typed failure
    /// caused the rejection (a malformed type string in the payload, the
    /// `types` table, or a Markdown frontmatter `$schema`), `source` keeps it
    /// unchanged and `message` repeats its text. `span` is the rejected text's
    /// byte range in the exact document supplied (for Markdown, the whole file
    /// including its frontmatter fence; line endings, CRLF included, counted
    /// as written): the rejected token when it can be projected, otherwise the
    /// whole rejected value. Either is `None` when unavailable.
    #[error("invalid standalone schema document `{path}`: {message}")]
    SchemaDocument {
        path: PathBuf,
        message: String,
        #[source]
        source: Option<Box<SchemaError>>,
        span: Option<Range<usize>>,
    },

    /// A `$schema` value is a remote URL; v1 only supports local references.
    #[error("remote $schema references are not supported in this version: `{reference}`")]
    RemoteUnsupported { reference: String },

    /// The baseline schema could not be loaded, parsed, or violates the
    /// "simple object schema" restriction documented in the spec.
    ///
    /// When the failure originated in another `SchemaError` (for example a
    /// parse or conversion error while loading the baseline from disk), the
    /// underlying error is preserved on `source` so programmatic consumers
    /// (e.g. `--format json` rendering) can inspect the chain. Structural
    /// violations of the "simple object schema" restriction have no
    /// upstream cause and leave `source` as `None`.
    #[error("baseline schema is invalid: {message}")]
    Baseline {
        message: String,
        #[source]
        source: Option<Box<SchemaError>>,
    },

    /// `jsonschema` rejected a converted schema at validator-construction
    /// time. The detailed message is preserved on the variant because
    /// `jsonschema::ValidationError` is not `'static` and is awkward to
    /// retain as a `#[source]`.
    #[error("could not build JSON Schema validator: {message}")]
    BuildValidator { message: String },

    /// I/O failed while reading a referenced or baseline schema file, or
    /// while inspecting or enumerating a schema root folder.
    #[error("io error reading `{path}`")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// `$schema` is present but its YAML/JSON shape is unsupported (not a
    /// mapping, sequence, or string).
    #[error("$schema frontmatter value has an unsupported shape: {message}")]
    FrontmatterShape { message: String },

    /// A chain of acyclic named-type references is deeper than the resolver
    /// expands (32 levels). Recursion is not this error: a cycle that
    /// descends into a child value is supported, and one that does not is
    /// [`SchemaError::NonDescendingCycle`]. `chain` describes the reference
    /// path where expansion stopped.
    #[error("named-type import cycle detected: {chain}")]
    ImportCycle { chain: String },

    /// Named types form a cycle that never descends into a child value: an
    /// alias cycle (`a: b`, `b: a`), a self-union (`a: [a, string]`), or a
    /// cycle through a `function(...)` signature. Such a type revisits the
    /// same value without progress, so no document can satisfy it.
    ///
    /// Each entry of `cycles` is one rejected cycle and lists every
    /// reference in it; a recursive group that also holds descending cycles
    /// reports only the non-descending ones.
    #[error(
        "named types form a cycle that never descends into a child value: {}",
        cycles
            .iter()
            .map(|cycle| cycle
                .iter()
                .map(|reference| format!("{} -> `{}`", reference.site.path, reference.target))
                .collect::<Vec<_>>()
                .join(", "))
            .collect::<Vec<_>>()
            .join("; ")
    )]
    NonDescendingCycle { cycles: Vec<Vec<CycleReference>> },

    /// A local `$ref` names no target in the schema it was written in: a
    /// `#/$defs/…` pointer whose definition is absent, or a reference chain
    /// that never reaches a schema. `location` is the JSON Pointer of the
    /// schema holding the reference, in the schema that carried it
    /// (`schema`, for example `the baseline` or a trigger file).
    #[error("{schema} has a `$ref` `{reference}` at `{location}` that names no definition")]
    UnresolvedReference {
        schema: String,
        reference: String,
        location: String,
    },

    /// A schema joined to another at a merge boundary (a baseline, a trigger
    /// payload, a root-union arm) uses a reference form whose meaning would
    /// change once its definitions move to the merged root: a `$id`, an
    /// anchor, a dynamic reference, an external target, or a local reference
    /// outside `#/$defs/…`. `schema` names the joined schema and `location`
    /// is the JSON Pointer of the offending keyword's schema.
    #[error("{schema} cannot be merged: {reason} at `{location}`")]
    UnsupportedMergeReference {
        schema: String,
        location: String,
        reason: String,
    },

    /// A named-type reference (bare `Name` or explicit `Name@file`) names a
    /// type its namespace does not define.
    ///
    /// `namespace` is the schema whose named types were searched: the
    /// defining file for a reference written inside a named-type file, else
    /// the inline `$schema`. `site` addresses the authored reference so an
    /// editor can range the diagnostic on the name.
    #[error("{}: unknown named type `{name}` in {namespace}", site.path)]
    UnknownNamedType {
        name: String,
        namespace: SchemaSourceId,
        site: SchemaReferenceSite,
    },

    /// Constraints merged onto one reused named type contradict each other.
    ///
    /// `site` is the consuming reference; `constraints` lists every
    /// constraint taking part, in merge order (inherited first), each with
    /// the value that took part and its own source and span, so the conflict
    /// can point at all of them, including declarations reached through
    /// multi-hop references. Messages show each value as its
    /// [`Constraint::diagnostic_excerpt`](super::Constraint::diagnostic_excerpt)
    /// in a code span; the structured `constraint` keeps the full value.
    #[error(
        "{}: contradictory {kind} on named type `{name}`: {}",
        site.path,
        constraints
            .iter()
            .map(|participant| format!(
                "{} in {}",
                participant_code_span(participant),
                participant.origin.source
            ))
            .collect::<Vec<_>>()
            .join(" and ")
    )]
    ConstraintConflict {
        name: String,
        kind: SchemaConstraintConflictKind,
        site: SchemaReferenceSite,
        constraints: Vec<SchemaConstraintParticipant>,
    },

    /// A use-site postfix asks for an operation a union-typed named type
    /// cannot take: a value constraint without `[]`, a constraint that is not
    /// valid on an array, `[]` over an arm that is already an array, or `[]`
    /// over an arm whose default is not an array.
    ///
    /// `site` is the consuming reference (its path names the property);
    /// `definition` is the union's declaration in the namespace that defines
    /// it. The two boxed fields keep [`SchemaError`] small.
    #[error("{}: {rejection} (named type `{name}`)", site.path)]
    UnionReference {
        name: String,
        rejection: Box<SchemaUnionRejection>,
        site: SchemaReferenceSite,
        definition: Box<SchemaReferenceSite>,
    },

    /// A `$schema` file-reference chain revisits a file it is already
    /// resolving. A schema file whose whole payload is a reference
    /// (`$schema: ./other.yaml`) — and each root-union file arm — re-enters
    /// reference resolution, so a self- or mutually-referencing pair would
    /// otherwise recurse until the process stack overflows. `chain` lists the
    /// canonical paths in visit order.
    #[error("$schema file-reference cycle detected: {chain}")]
    ReferenceCycle { chain: String },

    /// A `$schema` file-reference chain exceeded the delegation depth cap
    /// while every file in it was distinct. Deliberately distinct from
    /// [`SchemaError::ReferenceCycle`]: an acyclic chain has no loop to break,
    /// so reporting it as a cycle would send the author looking for a
    /// self-reference that does not exist. `limit` is the cap that was
    /// reached; `chain` lists the canonical paths in visit order, ending at
    /// the hop that would have exceeded it.
    #[error("$schema file-reference chain exceeded the {limit}-file depth limit: {chain}")]
    ReferenceDepthExceeded { limit: usize, chain: String },

    /// An `example(...)` constraint (Feature A) referenced a file that is
    /// missing, malformed, or does not validate against the example-artifact
    /// envelope (or its inherited `parameters` shape). Example files are
    /// documentation the schema promises are trustworthy, so a broken example
    /// is a fail-loud schema-load error, never a warning. `reference` is the
    /// authored `example(...)` reference; `message` is the underlying reason.
    #[error("invalid example artifact `{reference}`: {message}")]
    InvalidExample { reference: String, message: String },

    /// A schema-trigger envelope or match expression is malformed (a field of
    /// the wrong shape, an unknown key, or an otherwise unparseable `match:`
    /// payload). `message` starts with the field it is about (for example
    /// `` `match[1].group[0]`: ``) and describes the failure; the trigger
    /// grammar is documented in the "Repository Trigger Schemas" section of
    /// `darkmatter/docs/topics/schemas/definition.md`. A `$path` pattern that
    /// is not an allowed glob reference is reported here too. `span` is the
    /// byte range, in the trigger file's text, of the node the message is
    /// about; `None` until a caller holding that text locates it.
    #[error("invalid schema-trigger: {message}")]
    TriggerMatch { message: String, span: Option<Range<usize>> },

    /// A YAML document's `kind` names a document kind whose definition
    /// cannot be selected: two definitions claim it, its definition failed,
    /// or the registry of definitions could not be completed. `kind` is the
    /// document's authored `kind`.
    #[error("cannot select a schema for `kind: {kind}`: {cause}")]
    KindSelection { kind: String, cause: KindSelectionCause },

    /// A schema-trigger file uses a spelling that was renamed without an
    /// alias: `kind: trigger-schema` (now `schema-trigger`) or the `any:`
    /// combinator (now `group:`). `field` is where the spelling appears,
    /// `found` the rejected spelling, and `replacement` the one to use.
    #[error("`{field}`: `{found}` was renamed; use `{replacement}`")]
    TriggerRenamed {
        field: String,
        found: String,
        replacement: String,
    },

    /// A file claims `kind: schema-trigger` but is not valid YAML, so it never
    /// reached the match grammar. `message` is the YAML parser's reason without
    /// its location; `line` and `column` are one-based (the column counts
    /// characters, not bytes) and present when the parser reported them.
    #[error("schema-trigger envelope is not valid YAML{}: {message}", yaml_location(*.line, *.column))]
    TriggerEnvelopeYaml {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },

    /// A schema-trigger match condition used a constraint outside the
    /// match-safe subset (constraints that resolve files, import types, load
    /// examples, provide defaults, mark generated values, or eager-existence-
    /// check `file`). `property` names the offending property; `constraint`
    /// names the forbidden constraint keyword.
    #[error(
        "schema-trigger match condition for `{property}` uses a forbidden constraint: {constraint}"
    )]
    TriggerForbiddenConstraint { property: String, constraint: String },

    /// A schema-trigger match mapping mixes structural keys (combinators
    /// `all`/`group`/`none`/`min-match` or the `$path` predicate) with
    /// property-condition keys. Mixing is a load error — never a guess about
    /// the author's intent. `structural` and `properties` list the offending
    /// keys.
    #[error(
        "schema-trigger match mapping mixes structural keys ({structural:?}) with property keys \
         ({properties:?}); use `all:` to combine them"
    )]
    TriggerMixedMapping {
        structural: Vec<String>,
        properties: Vec<String>,
    },

    /// A schema-trigger's whole match expression is vacuous: some way of
    /// satisfying it includes no presence-requiring condition (a `required`
    /// gate or `$path` predicate inside `all`/`group`/`min-match`, not under
    /// `none`), so the trigger can match a document that has none of its
    /// keys — almost always an authoring mistake.
    #[error("schema-trigger match is vacuous (no presence-requiring condition)")]
    TriggerVacuousMatch,

    /// Two trigger filenames in the same `schemas/` root collide after
    /// case-folding (e.g. `Foo.yaml` and `foo.yaml`), which would activate
    /// differently on case-sensitive vs case-insensitive filesystems. `root`
    /// is the offending schema root directory; `files` lists the colliding
    /// filenames.
    #[error(
        "trigger schema files in `{}` collide after case-folding: {files:?}",
        root.display()
    )]
    TriggerCaseFoldCollision {
        root: PathBuf,
        files: Vec<String>,
    },

    /// A schema-trigger envelope file claimed `kind: schema-trigger` but could
    /// not be parsed. The underlying YAML, grammar, or envelope error is on
    /// `source`; `path` is the offending file.
    #[error("schema-trigger file `{}` {}", path.display(), load_failure(source))]
    TriggerLoad {
        path: PathBuf,
        #[source]
        source: Box<SchemaError>,
    },

    /// A trigger in a host's embedded schema bundle (such as Darkmatter's
    /// `EmbeddedSchemaBundle`)
    /// claimed `kind: schema-trigger` but could not be loaded: a malformed
    /// envelope, or a payload that failed to resolve inside its bundle.
    /// `bundle` and `name` identify the embedded document; it has no host
    /// path, so this is never a [`SchemaError::TriggerLoad`].
    #[error("embedded schema-trigger `{bundle}/{name}` {}", load_failure(source))]
    EmbeddedTriggerLoad {
        bundle: String,
        name: String,
        #[source]
        source: Box<SchemaError>,
    },

    /// A host's embedded schema bundle (such as Darkmatter's
    /// `EmbeddedSchemaBundle`) has an unusable name or file list.
    #[error("embedded schema bundle `{bundle}` is invalid: {message}")]
    EmbeddedBundle { bundle: String, message: String },

    /// A bare-name schema reference (no path component) was not found in any
    /// schema root, but a file of that name exists in the document's own
    /// directory. The author likely intended a relative reference; the error
    /// suggests `./‹name›` so the migration is pointed rather than opaque.
    /// `reference` is the authored bare name; `suggestion` is the `./`-prefixed
    /// form that would resolve to the sibling file.
    #[error(
        "bare-name schema reference `{reference}` was not found in any schema root; \
         a document-sibling file exists — use `{suggestion}` to reference it"
    )]
    BareNameSiblingExists { reference: String, suggestion: String },

    /// A schema-trigger payload did not resolve to a merge-compatible object
    /// schema. Trigger payloads must be SimplifiedSchema single objects or
    /// raw JSON Schemas satisfying the simple-object baseline contract; root
    /// unions and non-object schemas are rejected because the property-keyed
    /// merge has no sound "later wins" meaning for them. `path` is the
    /// trigger envelope file; `payload` is the authored `$schema:` reference
    /// (or `<inline>` for an inline payload); `payload_path` is the file
    /// that authored the rejected schema when the payload is a file
    /// reference (the last file of an alias chain); `reason` explains the
    /// rejection.
    #[error(
        "schema-trigger payload in `{}` is not merge-compatible: {reason}",
        path.display()
    )]
    TriggerPayloadNotMergeable {
        path: PathBuf,
        payload: String,
        payload_path: Option<PathBuf>,
        reason: String,
    },

    /// A schema-trigger payload passed the merge-compatibility gate but its
    /// JSON Schema could not be compiled into a validator. `path` is the
    /// trigger envelope; `payload` is the authored `$schema:` reference (or
    /// `<inline>`); `payload_path` is the file that authored the schema when
    /// the payload is a file reference (the last file of an alias chain);
    /// `message` is the compiler's reason.
    #[error(
        "schema-trigger payload in `{}` failed to compile: {message}",
        path.display()
    )]
    TriggerPayloadCompile {
        path: PathBuf,
        payload: String,
        payload_path: Option<PathBuf>,
        message: String,
    },

    /// A schema-trigger payload resolved to a trigger envelope file (directly
    /// or through an import cycle), rather than to an ordinary schema file.
    /// `trigger` is the originating envelope; `payload_path` is the file the
    /// payload resolved to; `chain` describes the resolution path for the
    /// indirect case.
    #[error(
        "schema-trigger payload in `{}` resolved to a schema-trigger envelope `{}`",
        trigger.display(),
        payload_path.display()
    )]
    TriggerPayloadCycle {
        trigger: PathBuf,
        payload_path: PathBuf,
        chain: String,
    },

    /// A document `$schema` directly referenced a `kind: schema-trigger` file.
    /// Trigger schemas activate by placement and match, never by reference;
    /// the error names the payload (`suggestion`) as the file to reference
    /// instead.
    #[error(
        "document `$schema` referenced a schema-trigger file `{reference}`; trigger schemas \
         activate by placement and match — reference the payload `{suggestion}` instead"
    )]
    TriggerSchemaReferenced {
        reference: String,
        suggestion: String,
    },
}

/// The ` at line L, column C` suffix of a YAML failure, empty without a
/// complete location.
fn yaml_location(line: Option<usize>, column: Option<usize>) -> String {
    match (line, column) {
        (Some(line), Some(column)) => format!(" at line {line}, column {column}"),
        (Some(line), None) => format!(" at line {line}"),
        _ => String::new(),
    }
}

/// The predicate of a trigger load failure: a YAML failure says so directly
/// rather than calling the file "malformed" with a nested envelope message.
fn load_failure(source: &SchemaError) -> String {
    match source {
        SchemaError::TriggerEnvelopeYaml { message, line, column } => {
            format!("is not valid YAML{}: {message}", yaml_location(*line, *column))
        }
        other => format!("is malformed: {other}"),
    }
}

/// Hint for a claimed trigger envelope that is not valid YAML.
const TRIGGER_YAML_HINT: &str = "Fix the YAML syntax at the reported position. A plain value \
     that contains <cyan>: </cyan> or <cyan> #</cyan>, or starts with an indicator such as \
     <cyan>@</cyan> or <cyan>*</cyan>, must be quoted, for example \
     <cyan>title: \"Note: quoted\"</cyan>.";

impl SchemaError {
    /// The hint a wrapping error (a compose-time schema preparation failure)
    /// shows in place of its generic one: today only a claimed trigger
    /// envelope that is not valid YAML, directly or inside a trigger load.
    pub fn preparation_hint(&self) -> Option<&'static str> {
        match self {
            SchemaError::TriggerEnvelopeYaml { .. } => Some(TRIGGER_YAML_HINT),
            SchemaError::TriggerLoad { source, .. } | SchemaError::EmbeddedTriggerLoad { source, .. } => {
                source.preparation_hint()
            }
            _ => None,
        }
    }
}

#[cfg(feature = "terminal")]
impl biscuit_terminal::errors::BlockError for SchemaError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            SchemaError::Aggregate { errors } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "multiple schema properties are invalid",
                ))
                .body(
                    errors
                        .iter()
                        .map(|error| Prose::new(Prose::escape_text_outside_code_spans(&error.to_string())))
                        .collect::<Vec<_>>(),
                )
                .hint("Fix each listed property definition, then validate the schema again."),
            SchemaError::ReferencedSchema { path, source } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "referenced schema is invalid",
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "The schema at <cyan>{}</cyan> contains an invalid definition.",
                            Prose::escape_text(&path.display().to_string())
                        )),
                        Prose::new(format!(
                            "<dim>Cause:</dim> {}",
                            Prose::escape_text_outside_code_spans(&source.to_string())
                        )),
                    ])
                    .hint("Fix the referenced schema definition, then validate again.")
            }
            SchemaError::Grammar {
                property,
                message,
                span,
                ..
            } => {
                let body = vec![
                    Prose::new(format!(
                        "SimplifiedSchema parsing failed for <cyan>{}</cyan>:",
                        Prose::escape_text(property)
                    )),
                    Prose::new(format!(
                        "<dim>Span:</dim> bytes {}..{}",
                        span.start, span.end
                    )),
                    Prose::new(format!(
                        "<dim>Message:</dim> {}",
                        Prose::escape_text_outside_code_spans(message)
                    )),
                ];
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "grammar parse failed"))
                    .body(body)
                    .hint(
                        "Check the type-and-constraint string. Shapes: <cyan>{type}</cyan>, \
                         <cyan>{type}(constraint;constraint)</cyan>, \
                         <cyan>{type} -> description</cyan>",
                    )
            }

            SchemaError::UnresolvedSlots(unresolved) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "schema template is unresolved"))
                .body(
                    unresolved
                        .slots
                        .iter()
                        .map(|slot| {
                            Prose::new(format!(
                                "<dim>Slot:</dim> <cyan>{}</cyan> <dim>(bytes {}..{})</dim>",
                                Prose::escape_text(&format!("{{{{ {} }}}}", slot.expression)),
                                slot.span.start,
                                slot.span.end
                            ))
                        })
                        .collect::<Vec<_>>(),
                )
                .hint("Resolve the schema's slots explicitly before validating documents against it."),

            SchemaError::SlotResolution(failure) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "schema slots could not be resolved"))
                .body(vec![Prose::new(Prose::escape_text(&failure.error.to_string()))])
                .hint("Fix the slot's expression or the data it reads, or grant the read it needs."),

            SchemaError::SlotResult { slot, kind, property, related } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "slot value cannot be used"))
                .body(
                    std::iter::once(Prose::new(format!(
                        "<dim>Slot:</dim> <cyan>{}</cyan> <dim>(bytes {}..{})</dim>",
                        Prose::escape_text(&format!("{{{{ {} }}}}", slot.expression)),
                        slot.span.start,
                        slot.span.end
                    )))
                    .chain(property.iter().map(|property| {
                        Prose::new(format!("<dim>Property:</dim> <cyan>{}</cyan>", Prose::escape_text(property)))
                    }))
                    .chain(std::iter::once(Prose::new(format!(
                        "<dim>Problem:</dim> {}",
                        Prose::escape_text(&kind.to_string())
                    ))))
                    .chain(related.iter().map(|related| {
                        let place = related.file.as_ref().map(|file| format!(" <dim>({})</dim>", Prose::escape_text(&file.display().to_string())));
                        Prose::new(format!(
                            "<dim>Related:</dim> {}{}",
                            Prose::escape_text(&related.message),
                            place.unwrap_or_default()
                        ))
                    }))
                    .collect::<Vec<_>>(),
                )
                .hint("Change the slot's expression or the data it reads so it yields a value the position takes."),

            SchemaError::SlotReadRefused { slot, refusal } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "slot read refused"))
                .body(vec![
                    Prose::new(format!(
                        "<dim>Slot:</dim> <cyan>{}</cyan> <dim>(bytes {}..{})</dim>",
                        Prose::escape_text(&format!("{{{{ {} }}}}", slot.expression)),
                        slot.span.start,
                        slot.span.end
                    )),
                    Prose::new(format!("<dim>Refused:</dim> {}", Prose::escape_text(&refusal.message))),
                ])
                .hint("Grant the read the returned type definition needs, or change the data it reads."),

            SchemaError::Convert { property, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "JSON Schema conversion failed",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Could not convert SimplifiedSchema for <cyan>{}</cyan>:",
                        Prose::escape_text(property)
                    )),
                    Prose::new(format!(
                        "<dim>Reason:</dim> {}",
                        Prose::escape_text_outside_code_spans(message)
                    )),
                ])
                .hint(
                    "Verify constraints are compatible with the declared type (e.g. <cyan>min</cyan> \
                     on <cyan>number</cyan>, <cyan>pattern</cyan> on <cyan>string</cyan>).",
                ),

            SchemaError::Unresolved { reference, source } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "$schema unresolved"))
                .body(vec![
                    Prose::new(format!(
                        "Could not resolve <cyan>$schema: {}</cyan>",
                        Prose::escape_text(reference)
                    )),
                    Prose::new(format!(
                        "<dim>Cause:</dim> {}",
                        Prose::escape_text_outside_code_spans(&source.to_string())
                    )),
                ])
                .hint(
                    "Confirm the file exists, uses a supported reference prefix (e.g. <cyan>./</cyan>, \
                     <cyan>~/</cyan>, <cyan>@/</cyan>), and is readable.",
                ),

            SchemaError::NoMatch { reference, bare_name } => {
                let cause = if *bare_name {
                    format!("no schema root holds `{reference}`")
                } else {
                    format!("no file matched `{reference}`")
                };
                let failure = biscuit_file::ResolutionFailure::NoMatch;
                let mut body = vec![
                    Prose::new(format!(
                        "Could not resolve <cyan>$schema: {}</cyan>",
                        Prose::escape_text(reference)
                    )),
                    Prose::new(format!("<dim>Cause:</dim> {}", Prose::escape_text_outside_code_spans(&cause))),
                ];
                if let Some(hint) = failure.glob_hint(reference) {
                    body.push(Prose::new(format!("<dim>hint:</dim> {hint}")));
                }
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "$schema unresolved"))
                    .body(body)
                    .hint(
                        "Confirm the file exists, uses a supported reference prefix (e.g. <cyan>./</cyan>, \
                         <cyan>~/</cyan>, <cyan>@/</cyan>), and is readable.",
                    )
            }

            SchemaError::NotASchemaDocument { path, reason } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "not a schema definition"))
                .body(vec![
                    Prose::new(format!(
                        "<cyan>{}</cyan> is not a valid schema definition.",
                        Prose::escape_text(&path.display().to_string())
                    )),
                    Prose::new(format!(
                        "<dim>Reason:</dim> {}",
                        Prose::escape_text_outside_code_spans(&reason.to_string())
                    )),
                ])
                .hint(match reason.hint(path) {
                    Some(hint) => Prose::escape_text_outside_code_spans(hint),
                    None => "Give a YAML schema a top-level <cyan>$schema:</cyan>, or write a JSON \
                             object declaring <cyan>https://json-schema.org/draft/2020-12/schema</cyan>."
                        .to_string(),
                }),

            SchemaError::UnsupportedJsonDialect { path, declared } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "unsupported JSON Schema dialect"))
                .body(Prose::new(format!(
                    "<cyan>{}</cyan> declares the dialect <cyan>{}</cyan>.",
                    Prose::escape_text(&path.display().to_string()),
                    Prose::escape_text(declared)
                )))
                .hint("Only <cyan>https://json-schema.org/draft/2020-12/schema</cyan> is supported."),

            SchemaError::InvalidJsonSchema { path, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "invalid JSON Schema"))
                .body(vec![
                    Prose::new(format!(
                        "<cyan>{}</cyan> is not a valid Draft 2020-12 JSON Schema.",
                        Prose::escape_text(&path.display().to_string())
                    )),
                    Prose::new(format!("<dim>Reason:</dim> {}", Prose::escape_text(message))),
                ])
                .hint("Fix the schema so it satisfies the Draft 2020-12 meta-schema."),

            SchemaError::SchemaDocument { path, message, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "standalone schema document invalid",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Could not parse <cyan>{}</cyan> as a standalone SimplifiedSchema.",
                        Prose::escape_text(&path.display().to_string())
                    )),
                    Prose::new(format!(
                        "<dim>Reason:</dim> {}",
                        Prose::escape_text_outside_code_spans(message)
                    )),
                ])
                .hint(
                    "A schema document is a <cyan>$schema:</cyan> export with optional <cyan>types:</cyan>, \
                     <cyan>description:</cyan>, and <cyan>categories:</cyan>; a <cyan>kind: schema</cyan> \
                     or <cyan>kind: yaml-schema</cyan> document follows its definition in \
                     <cyan>simplified-schema/schemas/</cyan>.",
                ),

            SchemaError::RemoteUnsupported { reference } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "remote $schema not supported",
                ))
                .body(Prose::new(format!(
                    "Remote schemas are not supported in v1: <cyan>{}</cyan>",
                    Prose::escape_text(reference)
                )))
                .hint("Download the schema locally and reference it with a relative or absolute path."),

            SchemaError::Baseline { message, source } => {
                let mut body = vec![Prose::new(format!(
                    "<dim>Reason:</dim> {}",
                    Prose::escape_text_outside_code_spans(message)
                ))];
                if let Some(cause) = source {
                    body.push(Prose::new(format!(
                        "<dim>Cause:</dim> {}",
                        Prose::escape_text_outside_code_spans(&cause.to_string())
                    )));
                }
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "baseline schema invalid"))
                    .body(body)
                    .hint(
                        "A baseline schema must be a simple object schema (root <cyan>type: object</cyan> \
                         with only <cyan>properties</cyan> / <cyan>required</cyan>).",
                    )
            }

            SchemaError::BuildValidator { message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "validator construction failed",
                ))
                .body(Prose::new(format!(
                    "<dim>jsonschema:</dim> {}",
                    Prose::escape_text_outside_code_spans(message)
                )))
                .hint(
                    "The lowered JSON Schema was rejected at compile time. Re-run with \
                     <cyan>RUST_LOG=darkmatter=debug</cyan> for more detail.",
                ),

            SchemaError::Io { path, source } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "I/O failure"))
                .body(vec![
                    Prose::new(format!(
                        "<dim>Path:</dim> <cyan>{}</cyan>",
                        Prose::escape_text(&path.display().to_string())
                    )),
                    Prose::new(format!(
                        "<dim>Cause:</dim> {} <dim>({:?})</dim>",
                        Prose::escape_text_outside_code_spans(&source.to_string()),
                        source.kind()
                    )),
                ])
                .hint("Check the path exists and the process has permission to read it."),

            SchemaError::FrontmatterShape { message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "$schema frontmatter shape unsupported",
                ))
                .body(Prose::new(format!(
                    "<dim>Reason:</dim> {}",
                    Prose::escape_text_outside_code_spans(message)
                )))
                .hint(
                    "<cyan>$schema</cyan> must be a YAML mapping (inline schema), a string (file \
                     reference), or a sequence (root union).",
                ),

            SchemaError::ImportCycle { chain } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "named-type import cycle",
                ))
                .body(Prose::new(format!(
                    "<dim>Cycle:</dim> {}",
                    Prose::escape_text(chain)
                )))
                .hint(
                    "A chain of named-type references (<cyan>Name@file</cyan>) that never repeats \
                     a type is limited to 32 levels. Shorten the chain.",
                ),

            SchemaError::NonDescendingCycle { cycles } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "non-descending type cycle"))
                .body(cycles.iter().flatten().map(|reference| {
                    Prose::new(format!(
                        "<cyan>{}</cyan> in {} refers to <cyan>{}</cyan> from {}",
                        Prose::escape_text(&reference.site.path.to_string()),
                        Prose::escape_text_outside_code_spans(&reference.site.source.to_string()),
                        Prose::escape_text(&reference.target),
                        Prose::escape_text_outside_code_spans(&reference.target_source.to_string())
                    ))
                }).collect::<Vec<_>>())
                .hint(
                    "A recursive type must reach itself again only through a child value: an \
                     object property, a dictionary value, a list item, or a tuple position \
                     (<cyan>item: \"{ children: item[] }\"</cyan>). An alias, a union arm, or a \
                     function signature does not count.",
                ),

            SchemaError::UnresolvedReference { schema, reference, location } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "unresolved schema reference"))
                    .body(Prose::new(format!(
                        "{} refers to <cyan>{}</cyan> at <cyan>{}</cyan>, which names no definition.",
                        Prose::escape_text_outside_code_spans(schema),
                        Prose::escape_text(reference),
                        Prose::escape_text(location)
                    )))
                    .hint(
                        "A <cyan>#/$defs/…</cyan> reference resolves only in the schema that \
                         declares it; add the definition to that schema's root <cyan>$defs</cyan>.",
                    )
            }

            SchemaError::UnsupportedMergeReference { schema, location, reason } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "unsupported reference at a merge"))
                    .body(Prose::new(format!(
                        "{} at <cyan>{}</cyan>: {}",
                        Prose::escape_text_outside_code_spans(schema),
                        Prose::escape_text(location),
                        Prose::escape_text_outside_code_spans(reason)
                    )))
                    .hint(
                        "A schema merged into another (a baseline, a trigger payload, a root-union \
                         arm) may refer only to its own root <cyan>$defs</cyan> with \
                         <cyan>#/$defs/name</cyan>, and may not change scope with <cyan>$id</cyan>.",
                    )
            }

            SchemaError::UnknownNamedType { name, namespace, site } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "unknown named type"))
                    .body(Prose::new(format!(
                        "<cyan>{}</cyan> references <cyan>{}</cyan>, which {} does not define.",
                        Prose::escape_text(&site.path.to_string()),
                        Prose::escape_text(name),
                        Prose::escape_text_outside_code_spans(&namespace.to_string())
                    )))
                    .hint(
                        "A bare name resolves only in the schema that writes it; use \
                         <cyan>Name@./file.yaml</cyan> for a type from another file.",
                    )
            }

            SchemaError::ConstraintConflict { name, kind, site, constraints } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "contradictory constraints"))
                    .body(
                        std::iter::once(Prose::new(format!(
                            "<cyan>{}</cyan> reuses <cyan>{}</cyan> with contradictory {kind}:",
                            Prose::escape_text(&site.path.to_string()),
                            Prose::escape_text(name)
                        )))
                        .chain(constraints.iter().map(|participant| {
                            let origin = &participant.origin;
                            // A code span's contents are literal Prose, so
                            // the excerpt is fenced rather than escaped.
                            Prose::new(format!(
                                "{} at {} in {}",
                                participant_code_span(participant),
                                Prose::escape_text(&origin.path.to_string()),
                                Prose::escape_text_outside_code_spans(&origin.source.to_string())
                            ))
                        }))
                        .collect::<Vec<_>>(),
                    )
                    .hint("Change the use-site constraint so it agrees with the named type.")
            }

            SchemaError::UnionReference { name, rejection, site, definition } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "unsupported operation on a union type",
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "<cyan>{}</cyan> references the union <cyan>{}</cyan>: {}.",
                            Prose::escape_text(&site.path.to_string()),
                            Prose::escape_text(name),
                            Prose::escape_text_outside_code_spans(&rejection.to_string())
                        )),
                        Prose::new(format!(
                            "<dim>Defined at:</dim> {} in {}",
                            Prose::escape_text(&definition.path.to_string()),
                            Prose::escape_text_outside_code_spans(&definition.source.to_string())
                        )),
                    ])
                    .hint(
                        "A union reference accepts <cyan>required</cyan>, <cyan>generated</cyan>, \
                         <cyan>eager</cyan>, a description, and <cyan>[]</cyan> with array \
                         constraints. Put value constraints on the union's arms.",
                    )
            }

            SchemaError::ReferenceCycle { chain } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "$schema reference cycle"))
                .body(Prose::new(format!(
                    "<dim>Chain:</dim> {}",
                    Prose::escape_text(chain)
                )))
                .hint(
                    "A schema file that only declares <cyan>$schema: ./other.yaml</cyan> delegates \
                     to that file. Break the loop so the chain terminates at a file that declares \
                     an actual schema.",
                ),

            SchemaError::ReferenceDepthExceeded { limit, chain } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "$schema reference chain too deep",
                    ))
                    .body(vec![
                        Prose::new(format!("<dim>Limit:</dim> {limit} files")),
                        Prose::new(format!(
                            "<dim>Chain:</dim> {}",
                            Prose::escape_text(chain)
                        )),
                    ])
                    .hint(
                        "Every file in this chain is distinct, so there is no loop to break. \
                         Flatten the delegation: point <cyan>$schema</cyan> at the file that \
                         declares the actual schema instead of chaining through intermediate \
                         redirects.",
                    )
            }

            SchemaError::InvalidExample { reference, message } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "invalid example artifact"))
                    .body(vec![
                        Prose::new(format!(
                            "<cyan>example({})</cyan> does not validate:",
                            Prose::escape_text(reference)
                        )),
                        Prose::new(format!(
                            "<dim>Reason:</dim> {}",
                            Prose::escape_text_outside_code_spans(message)
                        )),
                    ])
                    .hint(
                        "Each <cyan>example(...)</cyan> file must exist and validate against the \
                         example-artifact envelope (<cyan>kind</cyan>, <cyan>invocation</cyan>, \
                         <cyan>returns</cyan>, <cyan>description</cyan>).",
                    )
            }

            SchemaError::TriggerMatch { message, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger invalid",
                ))
                .body(Prose::new(format!(
                    "<dim>Reason:</dim> {}",
                    Prose::escape_text_outside_code_spans(message)
                )))
                .hint(
                    "See the trigger grammar: combinators (<cyan>all</cyan>/<cyan>group</cyan>/\
                     <cyan>none</cyan>/<cyan>min-match</cyan>), property conditions, and the \
                     <cyan>$path</cyan> predicate.",
                ),

            SchemaError::KindSelection { kind, cause } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "YAML document kind cannot be selected"))
                .body(Prose::new(format!(
                    "<dim>kind:</dim> <cyan>{}</cyan>\n<dim>Reason:</dim> {}",
                    Prose::escape_text(kind),
                    Prose::escape_text_outside_code_spans(&cause.to_string())
                )))
                .hint(match cause {
                    KindSelectionCause::Duplicate { .. } => {
                        "Keep exactly one <cyan>kind: yaml-schema</cyan> definition claiming this kind in the schema roots."
                    }
                    KindSelectionCause::Reserved { .. } | KindSelectionCause::Unavailable { .. } => {
                        "Fix the named definition; run <cyan>md schema validate</cyan> on it to see its defect."
                    }
                    KindSelectionCause::RootUnreadable { .. } => "Make the schema root readable, or remove it.",
                }),

            SchemaError::TriggerRenamed { field, found, replacement } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "schema-trigger spelling renamed"))
                .body(Prose::new(format!(
                    "<cyan>{}</cyan> uses <cyan>{}</cyan>, which was renamed with no alias.",
                    Prose::escape_text(field),
                    Prose::escape_text(found)
                )))
                .hint(format!("Write <cyan>{}</cyan> instead.", Prose::escape_text(replacement))),

            SchemaError::TriggerEnvelopeYaml { message, line, column } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "schema-trigger envelope is not valid YAML",
                    ))
                    .body(Prose::new(format!(
                        "<dim>Reason:</dim> {}{}",
                        Prose::escape_text_outside_code_spans(message),
                        Prose::escape_text(&yaml_location(*line, *column))
                    )))
                    .hint(TRIGGER_YAML_HINT)
            }

            SchemaError::TriggerForbiddenConstraint { property, constraint } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "schema-trigger forbidden constraint",
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "Property <cyan>{}</cyan> uses a forbidden constraint in a match \
                             condition: <cyan>{}</cyan>",
                            Prose::escape_text(property),
                            Prose::escape_text(constraint)
                        )),
                    ])
                    .hint(
                        "Match conditions allow only structural types and pure constraints: \
                         <cyan>required</cyan>, <cyan>enum</cyan>, <cyan>pattern</cyan>, \
                         length/range, item-count, key-count.",
                    )
            }

            SchemaError::TriggerMixedMapping {
                structural,
                properties,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger mixed mapping",
                ))
                .body(Prose::new(format!(
                    "A match mapping cannot mix structural keys ({:?}) with property keys \
                     ({:?}).",
                    structural, properties
                )))
                .hint(
                    "Wrap the conditions in an <cyan>all:</cyan> combinator to combine a \
                     structural key with property conditions.",
                ),

            SchemaError::TriggerVacuousMatch => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "schema-trigger vacuous match"))
                .body(Prose::new(
                    "A way of satisfying the match has no presence-requiring condition (a <cyan>required</cyan> \
                     gate or <cyan>$path</cyan> predicate), so it matches every document.",
                ))
                .hint(
                    "Add a <cyan>required</cyan> constraint or a <cyan>$path</cyan> predicate so \
                     the trigger only fires on matching documents.",
                ),

            SchemaError::TriggerCaseFoldCollision { root, files } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "schema-trigger case-fold collision",
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "Two files in <cyan>{}</cyan> collide after case-folding:",
                            Prose::escape_text(&root.display().to_string())
                        )),
                        Prose::new(format!(
                            "<dim>Files:</dim> {}",
                            files
                                .iter()
                                .map(|f| format!("{f:?}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                    ])
                    .hint(
                        "Rename one of the colliding files so they differ by more than case, \
                         preventing divergent behavior on case-sensitive vs case-insensitive \
                         filesystems.",
                    )
            }

            SchemaError::TriggerLoad { path, source } => {
                let yaml = matches!(**source, SchemaError::TriggerEnvelopeYaml { .. });
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        if yaml { "schema-trigger file is not valid YAML" } else { "schema-trigger file malformed" },
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "Trigger file <cyan>{}</cyan> could not be parsed:",
                            Prose::escape_text(&path.display().to_string())
                        )),
                        Prose::new(format!(
                            "<dim>Reason:</dim> {}",
                            Prose::escape_text_outside_code_spans(&source.to_string())
                        )),
                    ])
                    .hint(if yaml {
                        TRIGGER_YAML_HINT
                    } else {
                        "Fix the match grammar or envelope shape. See the trigger grammar: \
                         combinators (<cyan>all</cyan>/<cyan>any</cyan>/<cyan>none</cyan>/\
                         <cyan>min-match</cyan>), property conditions, and the \
                         <cyan>$path</cyan> predicate."
                    })
            }

            SchemaError::EmbeddedTriggerLoad { bundle, name, source } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "embedded schema-trigger malformed"))
                    .body(vec![
                        Prose::new(format!(
                            "Embedded trigger <cyan>{}</cyan> could not be loaded:",
                            Prose::escape_text(&format!("{bundle}/{name}"))
                        )),
                        Prose::new(format!(
                            "<dim>Reason:</dim> {}",
                            Prose::escape_text_outside_code_spans(&source.to_string())
                        )),
                    ])
                    .hint(
                        "The trigger ships inside the host binary; a schema file of the same \
                         name in a schema root replaces it.",
                    )
            }

            SchemaError::EmbeddedBundle { bundle, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "embedded schema bundle invalid"))
                .body(vec![Prose::new(format!(
                    "Embedded schema bundle <cyan>{}</cyan>: {}",
                    Prose::escape_text(bundle),
                    Prose::escape_text_outside_code_spans(message)
                ))]),

            SchemaError::BareNameSiblingExists {
                reference,
                suggestion,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "bare-name reference not in schema roots",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Bare-name reference <cyan>{}</cyan> was not found in any schema root.",
                        Prose::escape_text(reference)
                    )),
                    Prose::new(format!(
                        "A document-sibling file of that name exists. Use \
                         <cyan>{}</cyan> to reference it.",
                        Prose::escape_text(suggestion)
                    )),
                ])
                .hint(
                    "Prefix bare names with <cyan>./</cyan> to reference a file relative to the \
                     document's directory, or place the schema in a discovered <cyan>schemas/</cyan> \
                     root to use bare-name resolution.",
                ),

            SchemaError::TriggerPayloadNotMergeable {
                path,
                payload,
                reason,
                ..
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger payload not merge-compatible",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Trigger file <cyan>{}</cyan> payload <cyan>{}</cyan> is not a \
                         merge-compatible object schema:",
                        Prose::escape_text(&path.display().to_string()),
                        Prose::escape_text(payload)
                    )),
                    Prose::new(format!(
                        "<dim>Reason:</dim> {}",
                        Prose::escape_text_outside_code_spans(reason)
                    )),
                ])
                .hint(
                    "A trigger payload must be a SimplifiedSchema single object or a raw JSON \
                     Schema satisfying the simple-object baseline contract. Root unions and \
                     non-object schemas are rejected as trigger payloads.",
                ),

            SchemaError::TriggerPayloadCycle {
                trigger,
                payload_path,
                chain,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger payload cycle",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Trigger file <cyan>{}</cyan> payload resolved to a schema-trigger \
                         envelope <cyan>{}</cyan>:",
                        Prose::escape_text(&trigger.display().to_string()),
                        Prose::escape_text(&payload_path.display().to_string())
                    )),
                    Prose::new(format!("<dim>Chain:</dim> {}", Prose::escape_text(chain))),
                ])
                .hint(
                    "A trigger payload must not resolve to a schema-trigger envelope, directly or \
                     through an import cycle. Reference an ordinary schema file instead.",
                ),

            SchemaError::TriggerPayloadCompile {
                path,
                payload,
                message,
                ..
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger payload failed to compile",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Trigger file <cyan>{}</cyan> payload <cyan>{}</cyan> could not be \
                         compiled into a validator:",
                        Prose::escape_text(&path.display().to_string()),
                        Prose::escape_text(payload)
                    )),
                    Prose::new(format!(
                        "<dim>Reason:</dim> {}",
                        Prose::escape_text_outside_code_spans(message)
                    )),
                ]),

            SchemaError::TriggerSchemaReferenced {
                reference,
                suggestion,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger file referenced as document schema",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Document <cyan>$schema</cyan> referenced a schema-trigger file \
                         <cyan>{}</cyan>. Trigger schemas activate by placement and match, never \
                         by reference.",
                        Prose::escape_text(reference)
                    )),
                    Prose::new(format!(
                        "Reference the payload <cyan>{}</cyan> instead.",
                        Prose::escape_text(suggestion)
                    )),
                ])
                .hint(
                    "Use the payload file (without the <cyan>.trigger</cyan> infix) as the \
                     document <cyan>$schema</cyan>. Trigger envelopes live in <cyan>schemas/</cyan> \
                     and activate automatically when their match conditions hold.",
                ),
        }
    }
}

/// `message` followed by biscuit-file's literal-glob hint
/// ([`ResolutionFailure::glob_hint`](biscuit_file::ResolutionFailure::glob_hint))
/// on its own `hint:` line, when `failure` is a no-match of a `reference`
/// whose text looks like a glob; otherwise `message` unchanged.
///
/// Every message that reports a single-file failure passes through
/// this, so a literal-glob miss is explained in the same words on every
/// surface and no message tests for wildcards itself.
///
/// ## Examples
///
/// ```
/// use biscuit_file::ResolutionFailure;
/// use simplified_schema::errors::with_glob_hint;
///
/// let glob = with_glob_hint("File not found: docs/*.md", ResolutionFailure::NoMatch, "docs/*.md");
/// assert!(glob.contains("::file-links"));
/// let plain = with_glob_hint("File not found: a.md", ResolutionFailure::NoMatch, "a.md");
/// assert_eq!(plain, "File not found: a.md");
/// ```
pub fn with_glob_hint(
    message: impl Into<String>,
    failure: biscuit_file::ResolutionFailure,
    reference: &str,
) -> String {
    with_hint_line(message, failure.glob_hint(reference))
}

/// `message` followed by `hint` on its own `hint:` line, when there is one:
/// the line [`with_glob_hint`] writes, for a caller that computed the hint
/// when the failure was raised.
pub fn with_hint_line(message: impl Into<String>, hint: Option<&str>) -> String {
    let mut message = message.into();
    if let Some(hint) = hint {
        message.push_str("\nhint: ");
        message.push_str(hint);
    }
    message
}

// The tests render through the `BlockError` presentation, so they need the
// `terminal` feature.
#[cfg(all(test, feature = "terminal"))]
mod tests {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::errors::BlockError;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    use super::*;

    fn render(err: &SchemaError) -> String {
        let term = biscuit_terminal::terminal::Terminal::default();
        strip_escape_codes(err.status_block(&term).render_optimistic(Some(80)))
    }

    #[test]
    fn grammar_display_includes_property_and_message() {
        let err = SchemaError::Grammar {
            property: "title".into(),
            message: "unexpected `(`".into(),
            span: 6..7,
            related: Vec::new(),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("title"));
        assert!(rendered.contains("unexpected `(`"));
    }

    #[test]
    fn remote_unsupported_display_includes_reference() {
        let err = SchemaError::RemoteUnsupported {
            reference: "https://example.com/schema.json".into(),
        };
        assert!(err.to_string().contains("https://example.com/schema.json"));
    }

    #[test]
    fn not_a_schema_document_display_includes_path_reason_and_hint() {
        let err = SchemaError::NotASchemaDocument {
            path: PathBuf::from("./schemas/post.yaml"),
            reason: NotSchemaReason::TypesOnly,
        };
        let rendered = err.to_string();
        assert!(rendered.contains("post.yaml"), "{rendered}");
        assert!(rendered.contains("not a valid schema definition"), "{rendered}");
        assert!(rendered.contains("\nhint: use a named-type import (`Name@file.yaml`)"), "{rendered}");
        let json = SchemaError::NotASchemaDocument {
            path: PathBuf::from("schema.JSON"),
            reason: NotSchemaReason::NotAMapping,
        };
        assert!(json.to_string().contains(DRAFT_2020_12_DIALECT), "{json}");
        let yaml = SchemaError::NotASchemaDocument {
            path: PathBuf::from("schema.yaml"),
            reason: NotSchemaReason::NotAMapping,
        };
        assert!(!yaml.to_string().contains("hint:"), "{yaml}");
    }

    #[test]
    fn json_dialect_errors_display_path_and_detail() {
        let dialect = SchemaError::UnsupportedJsonDialect {
            path: PathBuf::from("s.json"),
            declared: "http://json-schema.org/draft-07/schema#".into(),
        };
        assert!(dialect.to_string().contains("draft-07"), "{dialect}");
        let invalid = SchemaError::InvalidJsonSchema {
            path: PathBuf::from("s.json"),
            message: "\"type\" is not valid".into(),
        };
        assert!(invalid.to_string().contains("s.json"), "{invalid}");
    }

    #[test]
    fn grammar_block_renders_property_and_span() {
        let err = SchemaError::Grammar {
            property: "title".into(),
            message: "unexpected `(`".into(),
            span: 6..7,
            related: Vec::new(),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("grammar parse failed"),
            "missing summary: {out}",
        );
        assert!(out.contains("title"), "missing property name: {out}");
        assert!(out.contains("unexpected"), "missing detail message: {out}");
    }

    #[test]
    fn convert_block_renders_property_and_reason() {
        let err = SchemaError::Convert {
            property: "age".into(),
            message: "min constraint not valid for string".into(),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("JSON Schema conversion failed"),
            "missing summary: {out}",
        );
        assert!(out.contains("age"), "missing property: {out}");
        assert!(out.contains("min constraint"), "missing reason: {out}");
    }

    fn site(property: &str) -> SchemaReferenceSite {
        SchemaReferenceSite {
            source: SchemaSourceId::InlineDocument,
            path: crate::SchemaSourcePath::root().property(property),
        }
    }

    #[test]
    fn unknown_named_type_names_the_type_the_reference_and_the_namespace() {
        let err = SchemaError::UnknownNamedType {
            name: "postcode".into(),
            namespace: SchemaSourceId::File(PathBuf::from("contacts.yaml")),
            site: site("address"),
        };
        assert_eq!(
            err.to_string(),
            "address: unknown named type `postcode` in `contacts.yaml`"
        );
        let out = render(&err);
        assert!(out.contains("unknown named type"), "missing summary: {out}");
        assert!(out.contains("postcode") && out.contains("contacts.yaml"), "{out}");
    }

    #[test]
    fn constraint_conflict_lists_every_origin_in_merge_order() {
        let origin = |source: SchemaSourceId, path: &str, keyword: &str| crate::SchemaConstraintOrigin {
            source,
            path: crate::SchemaSourcePath::root().property(path),
            keyword: keyword.into(),
            occurrence: 0,
            span: None,
        };
        let err = SchemaError::ConstraintConflict {
            name: "title_base".into(),
            kind: SchemaConstraintConflictKind::Bounds,
            site: site("title"),
            constraints: vec![
                SchemaConstraintParticipant {
                    origin: origin(SchemaSourceId::File(PathBuf::from("types.yaml")), "label", "min"),
                    constraint: crate::Constraint::MinLen(10),
                },
                SchemaConstraintParticipant {
                    origin: origin(SchemaSourceId::InlineDocument, "title", "max"),
                    constraint: crate::Constraint::MaxLen(5),
                },
            ],
        };
        assert_eq!(
            err.to_string(),
            "title: contradictory bounds on named type `title_base`: `min(10)` in `types.yaml` and \
             `max(5)` in the inline `$schema`"
        );
        let out = render(&err);
        assert!(out.contains("contradictory constraints"), "missing summary: {out}");
        assert!(out.contains("min(10) at label in types.yaml"), "{out}");
        assert!(out.contains("max(5) at title in the inline $schema"), "{out}");
    }

    /// A bounds conflict between `participant` (inherited, from a file) and
    /// a plain use-site `max(5)`.
    fn conflict_with(participant: crate::Constraint) -> SchemaError {
        let origin = |source: SchemaSourceId, path: &str| crate::SchemaConstraintOrigin {
            source,
            path: crate::SchemaSourcePath::root().property(path),
            keyword: "default".into(),
            occurrence: 0,
            span: None,
        };
        SchemaError::ConstraintConflict {
            name: "slug".into(),
            kind: SchemaConstraintConflictKind::Default,
            site: site("tags"),
            constraints: vec![
                SchemaConstraintParticipant {
                    origin: origin(SchemaSourceId::File(PathBuf::from("types.yaml")), "slug"),
                    constraint: participant,
                },
                SchemaConstraintParticipant {
                    origin: origin(SchemaSourceId::InlineDocument, "tags"),
                    constraint: crate::Constraint::Default(serde_json::json!(5)),
                },
            ],
        }
    }

    /// Asserts `Display` and the terminal block both show `excerpt` for the
    /// first participant, and that the structured value is `constraint`.
    fn assert_excerpt_everywhere(constraint: crate::Constraint, excerpt: &str) {
        let err = conflict_with(constraint.clone());
        let fenced = renderable::markdown::code_span(excerpt);
        let display = err.to_string();
        assert!(
            display.contains(&format!("{fenced} in `types.yaml` and `default(5)` in the inline")),
            "Display lacks {fenced}: {display}"
        );
        let term = biscuit_terminal::terminal::Terminal::default();
        let out = strip_escape_codes(err.status_block(&term).render_optimistic(Some(1000)));
        assert!(out.contains(&format!("{excerpt} at slug in types.yaml")), "terminal lacks {excerpt}: {out}");
        let SchemaError::ConstraintConflict { constraints, .. } = err else {
            unreachable!("built as a conflict");
        };
        assert_eq!(constraints[0].constraint, constraint, "the structured value is the full constraint");
    }

    #[test]
    fn conflict_excerpts_escape_control_characters_identically() {
        let constraint = crate::Constraint::Default(serde_json::json!("a\nb\tc\u{1b}[31md"));
        assert_excerpt_everywhere(constraint, r"default('a\nb\tc\u{1b}[31md')");
    }

    #[test]
    fn conflict_excerpts_keep_terminal_markup_literal() {
        let constraint = crate::Constraint::Default(serde_json::json!("<red>**x**</red>[y]"));
        let excerpt = "default(<red>**x**</red>[y])";
        assert_excerpt_everywhere(constraint.clone(), excerpt);
        let term = biscuit_terminal::terminal::Terminal::default();
        let raw = conflict_with(constraint).status_block(&term).render_optimistic(Some(1000));
        assert!(!raw.contains("\u{1b}[31m"), "markup must not become color: {raw:?}");
    }

    #[test]
    fn conflict_excerpts_shorten_long_unicode_values_to_160_scalars() {
        let value = "é".repeat(200);
        let constraint = crate::Constraint::Default(serde_json::json!(value));
        let excerpt = constraint.diagnostic_excerpt();
        assert_eq!(excerpt.chars().count(), 160);
        assert!(excerpt.starts_with("default(é") && excerpt.ends_with("é…"), "{excerpt}");
        assert_excerpt_everywhere(constraint, &excerpt);
    }

    #[test]
    fn conflict_excerpts_render_option_sets_without_panicking() {
        use crate::{AlphanumericOption, Constraint};
        assert_excerpt_everywhere(Constraint::Options(vec![]), "alphanumeric");
        assert_excerpt_everywhere(
            Constraint::Options(vec![AlphanumericOption::AllowPeriods, AlphanumericOption::AllowDashes]),
            "alphanumeric(allow-dashes,allow-periods)",
        );
    }

    #[test]
    fn conflict_excerpts_render_compound_defaults_as_compact_json() {
        let compound = serde_json::json!({"tags": ["a`b", "c"], "n": 1});
        let constraint = crate::Constraint::Default(compound);
        assert_excerpt_everywhere(constraint, r#"default({"tags":["a`b","c"],"n":1})"#);

        let long = serde_json::json!((0..100).collect::<Vec<_>>());
        let constraint = crate::Constraint::Default(long.clone());
        let excerpt = constraint.diagnostic_excerpt();
        assert_eq!(excerpt.chars().count(), 160);
        assert!(excerpt.starts_with("default([0,1,2,") && excerpt.ends_with('…'), "{excerpt}");
        assert_excerpt_everywhere(constraint, &excerpt);
    }

    #[test]
    fn union_reference_names_the_rejection_the_reference_and_the_definition() {
        let err = SchemaError::UnionReference {
            name: "policy".into(),
            rejection: Box::new(SchemaUnionRejection::Constraint {
                keyword: "max".into(),
                array_level: false,
            }),
            site: site("content_policy"),
            definition: Box::new(SchemaReferenceSite {
                source: SchemaSourceId::File(PathBuf::from("policy.yaml")),
                path: crate::SchemaSourcePath::root().property("policy"),
            }),
        };
        assert_eq!(
            err.to_string(),
            "content_policy: constraint `max` cannot apply to a union-typed type (named type \
             `policy`)"
        );
        let out = render(&err);
        assert!(out.contains("unsupported operation on a union type"), "missing summary: {out}");
        assert!(out.contains("content_policy") && out.contains("policy.yaml"), "{out}");
    }

    #[test]
    fn remote_unsupported_block_renders_url() {
        let err = SchemaError::RemoteUnsupported {
            reference: "https://example.com/schema.json".into(),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("remote $schema not supported"),
            "missing summary: {out}",
        );
        assert!(
            out.contains("https://example.com/schema.json"),
            "missing url: {out}",
        );
    }

    #[test]
    fn not_a_schema_document_block_renders_path() {
        let err = SchemaError::NotASchemaDocument {
            path: PathBuf::from("./schemas/post.yaml"),
            reason: NotSchemaReason::NoSchemaProperty,
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(out.contains("not a schema definition"), "missing summary: {out}");
        assert!(out.contains("post.yaml"), "missing path: {out}");
    }

    #[test]
    fn json_dialect_blocks_render_path() {
        let dialect = render(&SchemaError::UnsupportedJsonDialect {
            path: PathBuf::from("s.json"),
            declared: "http://json-schema.org/draft-07/schema#".into(),
        });
        assert!(dialect.contains("unsupported JSON Schema dialect"), "{dialect}");
        assert!(dialect.contains("s.json"), "{dialect}");
        let invalid = render(&SchemaError::InvalidJsonSchema {
            path: PathBuf::from("s.json"),
            message: "bad".into(),
        });
        assert!(invalid.contains("invalid JSON Schema"), "{invalid}");
    }

    #[test]
    fn baseline_block_renders_message() {
        let err = SchemaError::Baseline {
            message: "must be a simple object schema".into(),
            source: None,
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("baseline schema invalid"),
            "missing summary: {out}",
        );
        assert!(
            out.contains("simple object schema"),
            "missing message: {out}",
        );
    }

    #[test]
    fn build_validator_block_renders_message() {
        let err = SchemaError::BuildValidator {
            message: "unknown keyword `xfoo`".into(),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("validator construction failed"),
            "missing summary: {out}",
        );
        assert!(out.contains("xfoo"), "missing detail: {out}");
    }

    #[test]
    fn io_block_renders_path_and_kind() {
        let err = SchemaError::Io {
            path: PathBuf::from("/missing.yaml"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(out.contains("I/O failure"), "missing summary: {out}");
        assert!(out.contains("/missing.yaml"), "missing path: {out}");
        assert!(out.contains("NotFound"), "missing io kind: {out}");
    }

    #[test]
    fn frontmatter_shape_block_renders_message() {
        let err = SchemaError::FrontmatterShape {
            message: "expected mapping, got integer".into(),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("frontmatter shape unsupported"),
            "missing summary: {out}",
        );
        assert!(out.contains("expected mapping"), "missing detail: {out}",);
    }
}

