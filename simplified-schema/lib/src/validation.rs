//! The shape of one validation finding: [`ValidationProblem`], its
//! classifications, its JSON-Pointer view, and the structured cause of a
//! file-reference failure.

use std::path::PathBuf;

use serde_json::Value;

/// Coarse classification of a validation problem.
///
/// Renderers use this to pick the right category label (e.g. `missing`,
/// `type`, `invalid`) without having to infer from `property` presence or
/// substring-matching `message`. Mapped from `jsonschema::ValidationErrorKind`
/// at problem-construction time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationProblemKind {
    /// A required property is missing from the instance.
    Missing,
    /// The instance value did not match the expected schema type.
    Type,
    /// Any other constraint failure (format, range, length, pattern, etc.).
    Invalid,
}

/// A single validation problem.
#[derive(Debug, Clone)]
pub struct ValidationProblem {
    /// JSON pointer to the failing instance value (e.g. `/tags/2`).
    ///
    /// For `Required` failures this points at the *parent* object (empty for
    /// the document root), since the missing property has no instance value.
    /// Use [`Self::property`] to recover the missing property's name.
    pub path: String,
    /// Human-readable message; format mirrors `jsonschema`'s default.
    pub message: String,
    /// Coarse failure category, derived from the underlying validator error.
    pub kind: ValidationProblemKind,
    /// Name of the missing property, set only for `Required` failures.
    pub property: Option<String>,
    /// Source line of the problem in the frontmatter, when available.
    pub line: Option<u32>,
    /// Source column of the problem in the frontmatter, when available.
    pub column: Option<u32>,
    /// Index of the root-union arm under which this problem was raised, when
    /// the schema is a root union.
    pub arm_index: Option<usize>,
    /// Declared description of the failing property, resolved from the
    /// compiled JSON Schema (the SimplifiedSchema `-> {description}` text, or a
    /// `description` keyword authored in a referenced JSON Schema file).
    /// `None` when the property declares no description.
    pub description: Option<String>,
    /// Fine-grained failure classification (R-5 Priority 1). Unlike the coarse
    /// [`ValidationProblemKind`], this separates constraint violations, unknown
    /// keys, and file-reference failures so span-aware consumers (DMLS) can
    /// pick a ranging rule per category.
    pub code: ValidationProblemCode,
    /// The [`path`](Self::path) parsed into decoded JSON-Pointer segments, so
    /// consumers walk a frontmatter AST without re-parsing `~0`/`~1` escapes.
    pub instance_path: JsonPointer,
    /// Location of the failing keyword within the compiled JSON Schema, parsed
    /// into JSON-Pointer segments. `None` when the validator reported no schema
    /// path.
    pub schema_path: Option<JsonPointer>,
    /// The specific undeclared key for an `additionalProperties` /
    /// unknown-key failure. `None` for every other category. The failure's
    /// [`path`](Self::path) points at the *parent* object in this case, so this
    /// is the only way to recover the offending key.
    pub offending_property: Option<String>,
    /// Structured cause of a `format: darkmatter-file` /
    /// `darkmatter-file-reference` failure (R-5 Priority 4). `Some` only when
    /// [`code`](Self::code) is [`ValidationProblemCode::InvalidFileReference`];
    /// [`message`](Self::message) still carries the same rendered text.
    pub file_reference: Option<FileReferenceDiagnostic>,
    /// Caller-owned resolution context retained when projection, rather than
    /// ordinary document validation, produced the file-reference failure.
    pub caller_file: Option<CallerFileReferenceProvenance>,
}

impl ValidationProblem {
    /// Whether this problem comes from a required check this build cannot
    /// run, rather than from the value: a file value's resolution or a
    /// `match()` glob, without the `file` feature. Such a check fails rather
    /// than passing silently, and this tells the two failures apart. Always
    /// `false` with the `file` feature, where every check runs.
    pub fn is_unavailable_check(&self) -> bool {
        !cfg!(feature = "file") && self.is_file_check()
    }

    /// Whether this problem is a file value's resolution failure or a
    /// `match()` glob rejection: the checks that need a file-resolution
    /// context. Where no context exists (no `file` feature, or a schema built
    /// from in-memory content) these are unavailable checks; malformed file
    /// syntax is judged everywhere and is not one of them.
    pub(crate) fn is_file_check(&self) -> bool {
        // Without a context every parsed file value's resolution fails, and a
        // `match()` glob accepts nothing.
        matches!(self.file_reference, Some(FileReferenceDiagnostic::ResolutionFailed { .. }))
            || self.schema_path.as_ref().is_some_and(|path| {
                path.segments().last().map(String::as_str) == Some(crate::file_match::DARKMATTER_MATCH_KEYWORD)
            })
    }
}

/// Resolution evidence attached to a caller-owned file validation problem.
#[derive(Debug, Clone)]
pub struct CallerFileReferenceProvenance {
    /// The immutable context captured where the caller authored the value.
    pub origin: biscuit_file::FileResolutionContext,
    /// The selected or first attempted candidate, when the reference parsed.
    pub candidate: Option<biscuit_file::ResolutionCandidate>,
}

/// Fine-grained classification of a [`ValidationProblem`] (R-5 Priority 1).
///
/// Finer than [`ValidationProblemKind`]: constraint violations, unknown keys,
/// and file-reference failures each get their own variant so a diagnostic
/// surface can choose the correct ranging rule (value node, key node, whole
/// entry) without substring-matching the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationProblemCode {
    /// A required property is absent.
    MissingRequired,
    /// The instance value's type did not match the declared type.
    TypeMismatch,
    /// A non-type constraint failed (range, length, pattern, enum, format
    /// other than a file reference, …).
    ConstraintViolation,
    /// A key not declared by the schema was present under
    /// `additionalProperties: false`.
    UnknownKey,
    /// A `format: darkmatter-file` / `darkmatter-file-reference` value failed
    /// to parse, resolve, or match an existing file.
    InvalidFileReference,
    /// A whole-file YAML document of a host's own kind satisfies its
    /// definition but fails a check the definition cannot state (an
    /// invalid trigger `$path` glob, an undeclared category).
    KindCheck,
}

/// Code of a frontmatter value whose type does not match its declaration.
pub const SCHEMA_TYPE_MISMATCH_CODE: &str = "dm.schema.type_mismatch";
/// Code of a failed non-type constraint (range, length, pattern, enum, …).
pub const SCHEMA_CONSTRAINT_CODE: &str = "dm.schema.constraint";
/// Code of an absent required key.
pub const SCHEMA_MISSING_REQUIRED_CODE: &str = "dm.schema.missing_required";
/// Code of a key the schema does not declare.
pub const SCHEMA_UNKNOWN_KEY_CODE: &str = "dm.schema.unknown_key";
/// Code of a `file(...)`-typed value that failed to parse, resolve, or match.
pub const SCHEMA_INVALID_FILE_REFERENCE_CODE: &str = "dm.schema.invalid_file_reference";
/// Code of a document-kind check beyond its definition.
pub const SCHEMA_KIND_CHECK_CODE: &str = "dm.schema.kind_check";
/// Code of a schema that could not be resolved, merged, or compiled.
pub const SCHEMA_PREPARE_CODE: &str = "dm.schema.prepare";

impl ValidationProblemCode {
    /// The shared diagnostic code DMLS and `md compose --output json`
    /// report this problem under.
    pub fn diagnostic_code(self) -> &'static str {
        match self {
            Self::MissingRequired => SCHEMA_MISSING_REQUIRED_CODE,
            Self::TypeMismatch => SCHEMA_TYPE_MISMATCH_CODE,
            Self::ConstraintViolation => SCHEMA_CONSTRAINT_CODE,
            Self::UnknownKey => SCHEMA_UNKNOWN_KEY_CODE,
            Self::InvalidFileReference => SCHEMA_INVALID_FILE_REFERENCE_CODE,
            Self::KindCheck => SCHEMA_KIND_CHECK_CODE,
        }
    }
}

/// A parsed RFC 6901 JSON Pointer.
///
/// [`ValidationProblem::path`] carries the raw pointer string (`/tags/2`); this
/// is the decoded segment view (`["tags", "2"]`) span-aware consumers walk
/// without re-parsing the `~0`/`~1` escapes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JsonPointer {
    segments: Vec<String>,
}

impl JsonPointer {
    /// Parses a JSON-Pointer string into its decoded segments. An empty string
    /// (the document root) yields an empty pointer.
    pub fn parse(pointer: &str) -> Self {
        if pointer.is_empty() {
            return Self::default();
        }
        let body = pointer.strip_prefix('/').unwrap_or(pointer);
        let segments = body.split('/').map(decode_pointer_segment).collect();
        Self { segments }
    }

    /// The decoded segments, outermost first.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// `true` for the document-root pointer (no segments).
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// The first (top-level) segment, if any.
    pub fn first(&self) -> Option<&str> {
        self.segments.first().map(String::as_str)
    }

    /// Re-encodes the pointer to its canonical RFC 6901 string form.
    pub fn as_pointer_string(&self) -> String {
        if self.segments.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        for segment in &self.segments {
            out.push('/');
            out.push_str(&segment.replace('~', "~0").replace('/', "~1"));
        }
        out
    }
}

impl FileReferenceDiagnostic {
    /// The failure class, as biscuit-file classifies file-reference errors:
    /// malformed syntax is [`InvalidReference`], a missing file is
    /// [`NoMatch`].
    ///
    /// [`InvalidReference`]: biscuit_file::ResolutionFailure::InvalidReference
    /// [`NoMatch`]: biscuit_file::ResolutionFailure::NoMatch
    pub fn resolution_failure(&self) -> biscuit_file::ResolutionFailure {
        match self {
            Self::InvalidSyntax { .. } => biscuit_file::ResolutionFailure::InvalidReference,
            Self::ResolutionFailed { failure, .. } => *failure,
            Self::NoMatch { .. } => biscuit_file::ResolutionFailure::NoMatch,
        }
    }
}

fn decode_pointer_segment(segment: &str) -> String {
    segment.replace("~1", "/").replace("~0", "~")
}

/// Structured cause of a file-reference validation failure (R-5 Priority 4).
///
/// Replaces the previous opaque substituted message string with a typed cause
/// so consumers can offer the right remediation (fix the syntax, fix the
/// resolution context, or point at the missing file) and range the diagnostic
/// against the resolved-from directory when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileReferenceDiagnostic {
    /// The value is not a parseable file reference.
    InvalidSyntax {
        /// The offending raw value.
        raw: String,
    },
    /// The reference parsed but could not be resolved against the filesystem
    /// or environment.
    ResolutionFailed {
        /// The offending raw value.
        raw: String,
        /// Why it could not be resolved.
        failure: biscuit_file::ResolutionFailure,
    },
    /// The reference parsed and resolved, but no file exists at the resolved
    /// path.
    NoMatch {
        /// The offending raw value.
        raw: String,
        /// The directory resolution was anchored at, when known.
        resolved_from: Option<PathBuf>,
    },
}

/// The authored value a problem's JSON Pointer names, or its nearest
/// existing ancestor when the pointer names a missing member, with that
/// node's own pointer. `None` for the root pointer or an absent top-level
/// key.
pub fn authored_node_at<'a>(frontmatter: &'a Value, pointer: &str) -> Option<(String, &'a Value)> {
    let mut segments = pointer
        .strip_prefix('/')?
        .split('/')
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"));
    let key = segments.next().filter(|key| !key.is_empty())?;
    let mut node = frontmatter.get(&key)?;
    let encode = |segment: &str| segment.replace('~', "~0").replace('/', "~1");
    let mut node_pointer = format!("/{}", encode(&key));
    for segment in segments {
        let child = match node {
            Value::Object(map) => map.get(&segment),
            Value::Array(items) => segment.parse::<usize>().ok().and_then(|index| items.get(index)),
            _ => None,
        };
        match child {
            Some(child) => {
                node = child;
                node_pointer.push('/');
                node_pointer.push_str(&encode(&segment));
            }
            None => break,
        }
    }
    Some((node_pointer, node))
}
