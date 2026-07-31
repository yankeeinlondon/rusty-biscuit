//! Custom format and keyword validators for the schemas subsystem.
//!
//! Several darkmatter-specific schema fragments require validators beyond the
//! built-in JSON Schema vocabulary:
//!
//! - **`format: darkmatter-file`** (eager) — parses the value through
//!   [`biscuit_file::FileReference`] and confirms the resolved path exists.
//!   Resolution runs through the shared document-backed context
//!   ([`resolve_document_file_ref`]): implicit bare paths resolve
//!   repository-root first then the prompt document directory, explicit
//!   `./`/`../` from the document directory only, with no launch-area fallback
//!   (D2) and no ambient-CWD read on the anchored path. This mirrors the
//!   expression path (`file_exists`/`frontmatter`) so schema validation and
//!   expression functions agree on the same `file` value. Emitted for
//!   SimplifiedSchema `file(eager)`.
//! - **`format: darkmatter-file-reference`** (lazy) — validates **syntax
//!   only** via construction-only [`biscuit_file::FileReference::new`]: a
//!   syntactically valid but not-yet-existing path passes; no resolve, no
//!   filesystem/git/env/vault lookup, no existence check. Emitted for
//!   SimplifiedSchema bare `file`.
//! - **`x-darkmatter-url-scheme`** — runs alongside `format: uri` and
//!   restricts the URL scheme to a configured list (case-insensitive).
//! - **`x-darkmatter-type-definition` / `x-darkmatter-schema`** — validate
//!   native string, mapping, or sequence carriers through the shared passive
//!   SimplifiedSchema parsers. They perform no resolution or I/O.
//! - **`format: darkmatter-yaml` / `format: darkmatter-json`** (Feature D) —
//!   the `yaml` / `json` content-format string types. The value must parse as
//!   YAML (JSON accepted, being a YAML subset) or strict JSON respectively;
//!   a native mapping/sequence/scalar is serialized to its target-format
//!   string by the schema coercion pass before it reaches the validator.
//!
//! `darkmatter-file` / `darkmatter-file-reference` are `Format`s (they see
//! only the string) and the `x-darkmatter-*` semantic validators are custom
//! `Keyword` implementations. `match(...)` is **not** a validation keyword: it is suggestion
//! metadata carried on the SimplifiedSchema atom (`Constraint::Match` →
//! completion), never lowered into the compiled JSON Schema.
//!
//! ## Examples
//!
//! ```ignore
//! use jsonschema::{Draft, options};
//! use darkmatter::markdown::schemas::format::{
//!     register_darkmatter_formats, url_scheme_keyword_factory,
//! };
//!
//! let validator = register_darkmatter_formats(
//!     options().with_draft(Draft::Draft202012),
//!     None,
//!     None,
//! )
//! .with_keyword("x-darkmatter-url-scheme", url_scheme_keyword_factory)
//! .build(&schema)?;
//! ```

use std::fmt;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileReferenceError};
use jsonschema::{Keyword, ValidationError, ValidationOptions, paths::Location};
use serde_json::{Map, Value};
use url::Url;

use crate::markdown::compose::expression::resolve_ctx::resolve_document_file_ref;
use crate::markdown::schemas::simplified::{
    parse_property_definition, parse_schema_declaration,
};

/// Format name registered for eager `file(eager)` SimplifiedSchema atoms and
/// for raw JSON Schema authors who want existence-checking.
///
/// The eager validator parses the value as a [`FileReference`], resolves it
/// through the shared document-backed context (repository-first then
/// source-relative for implicit paths), and fails when the file does not exist
/// on disk.
pub const DARKMATTER_FILE_FORMAT: &str = "darkmatter-file";

/// Format name registered for lazy, syntax-only `file` references.
///
/// Emitted by SimplifiedSchema bare `file` (no `eager`). Validation is
/// construction-only via [`FileReference::new`]: a syntactically valid
/// reference passes regardless of whether the target exists, resolves, or its
/// environment/vault/git context is available. Existence checking is the eager
/// [`DARKMATTER_FILE_FORMAT`]'s job.
pub const DARKMATTER_FILE_REFERENCE_FORMAT: &str = "darkmatter-file-reference";

/// Keyword name registered for `url(scheme(...))` constraints.
pub const DARKMATTER_URL_SCHEME_KEYWORD: &str = "x-darkmatter-url-scheme";

/// Keyword emitted for the `type-definition` semantic meta-type.
pub const DARKMATTER_TYPE_DEFINITION_KEYWORD: &str = "x-darkmatter-type-definition";

/// Keyword emitted for the `schema` semantic meta-type.
pub const DARKMATTER_SCHEMA_KEYWORD: &str = "x-darkmatter-schema";

/// Format name registered for the `yaml` content-format string type (Feature
/// D). The value must parse as valid YAML; because JSON is a YAML subset a JSON
/// string is accepted too.
pub const DARKMATTER_YAML_FORMAT: &str = "darkmatter-yaml";

/// Format name registered for the `json` content-format string type (Feature
/// D). The value must parse as strict JSON; YAML-only syntax is rejected.
pub const DARKMATTER_JSON_FORMAT: &str = "darkmatter-json";

/// Format name registered for the `expression` content-format string type
/// (feature `2026-07-12-literal-expression`). The value must parse under the
/// Darkmatter expression grammar and is **never evaluated** — validation is
/// pure `parse_condition(value).is_ok()`. Condition mode accepts a parse
/// superset of the value dialect (`&&` added, `||` re-lowered), so a string
/// valid in either dialect passes.
pub const DARKMATTER_EXPRESSION_FORMAT: &str = "darkmatter-expression";

/// Format name registered for the `datetime` SimplifiedSchema type.
///
/// Emitted in place of JSON Schema's built-in `date-time` format, whose RFC
/// 3339 grammar **requires** a zone offset and so rejects the offset-less local
/// datetimes ISO 8601 permits (e.g. `2026-07-10T15:05:34`). This validator
/// accepts both offset-bearing and offset-less ISO 8601 datetimes while still
/// range-checking the value via `chrono` (so `2026-13-99T00:00:00` is rejected),
/// keeping validation consistent with the offset-optional detection grammar in
/// [`super::detect`].
pub const DARKMATTER_DATETIME_FORMAT: &str = "darkmatter-datetime";

/// Format name registered for the `time` SimplifiedSchema type.
///
/// Sibling of [`DARKMATTER_DATETIME_FORMAT`] for the `time` type, whose
/// documented contract is "time of day with **optional** timezone" — a promise
/// the built-in RFC 3339 `time` format (offset required) breaks.
pub const DARKMATTER_TIME_FORMAT: &str = "darkmatter-time";

/// Accepts an ISO 8601 datetime with an **optional** zone offset.
///
/// Offset-bearing values are validated as RFC 3339 (a strict ISO 8601 profile);
/// offset-less values are validated as a naive local datetime. Both paths parse
/// through `chrono`, so out-of-range components are rejected.
fn valid_iso8601_datetime(value: &str) -> bool {
    if chrono::DateTime::parse_from_rfc3339(value).is_ok() {
        return true;
    }
    const NAIVE_FORMATS: &[&str] = &[
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M",
    ];
    NAIVE_FORMATS
        .iter()
        .any(|fmt| chrono::NaiveDateTime::parse_from_str(value, fmt).is_ok())
}

/// Accepts an ISO 8601 time-of-day with an **optional** zone offset.
///
/// Offset-less values parse as a naive time; an offset-bearing value is
/// validated by attaching a sentinel date and reusing
/// [`valid_iso8601_datetime`].
fn valid_iso8601_time(value: &str) -> bool {
    const NAIVE_FORMATS: &[&str] = &["%H:%M:%S", "%H:%M:%S%.f", "%H:%M"];
    if NAIVE_FORMATS
        .iter()
        .any(|fmt| chrono::NaiveTime::parse_from_str(value, fmt).is_ok())
    {
        return true;
    }
    valid_iso8601_datetime(&format!("2000-01-01T{value}"))
}

/// Registers the `darkmatter-file` format on a `ValidationOptions` builder.
///
/// `base_dir`, when `Some`, is the prompt document directory: implicit bare
/// references resolve repository-root first then the document directory, and
/// explicit `./`/`../` from the document directory only. When `None` (the bare
/// validator API with no document anchor) the validator resolves against the
/// ambient process CWD.
///
/// `fallback` (the captured launch area) is retained for structural
/// compatibility with the validator-cache anchors but is **not** a resolution
/// input: per D2 there is no launch-area fallback for references authored inside
/// a document.
///
/// This matches the shared resolution order encoded by
/// [`resolve_document_file_ref`], so the `darkmatter-file` validator and the
/// expression path (`file_exists`/`frontmatter`) agree on the same `file`
/// value.
///
/// Splitting this out keeps validator construction in
/// [`crate::markdown::schemas::validate`] tidy and lets tests register only
/// the format without the keywords.
///
/// jsonschema 0.42's `with_format` accepts `F: Fn(&str) -> bool +
/// Send + Sync + 'static`, so the anchors are captured by move into the
/// closure — no thread-local or process-global state is required.
pub fn register_darkmatter_formats(
    options: ValidationOptions,
    base_dir: Option<PathBuf>,
    fallback: Option<PathBuf>,
) -> ValidationOptions {
    register_darkmatter_formats_in_context(options, base_dir, fallback, None)
}

pub(crate) fn register_darkmatter_formats_in_context(
    options: ValidationOptions,
    base_dir: Option<PathBuf>,
    fallback: Option<PathBuf>,
    file_resolution_context: Option<biscuit_file::FileResolutionContext>,
) -> ValidationOptions {
    options
        .with_format(DARKMATTER_FILE_FORMAT, move |value: &str| {
            validate_file_reference(
                value,
                base_dir.as_deref(),
                fallback.as_deref(),
                file_resolution_context.as_ref(),
            )
        })
        .with_format(DARKMATTER_FILE_REFERENCE_FORMAT, |value: &str| {
            // Lazy contract: syntax only. `FileReference::new` is
            // construction-only — no `resolve()`, no `resolve_from()`, no
            // filesystem/git/env/vault lookup, no `path.exists()` check — so a
            // syntactically valid but not-yet-existing path validates here.
            // Laziness defers *existence*, not *syntax*: a malformed reference
            // still fails.
            FileReference::new(value).is_ok()
        })
        // Content-format string types (Feature D). A `format` validator only
        // ever sees a string, so a native (mapping/sequence/scalar) value is
        // serialized to its target-format string by the schema coercion pass
        // (`super::coerce`) before it reaches here.
        .with_format(DARKMATTER_YAML_FORMAT, |value: &str| {
            // `yaml` accepts any valid YAML; JSON is a YAML subset, so a JSON
            // string parses too. biscuit-file's `Yaml` is the parsing seam.
            biscuit_file::Yaml::from_str(value).is_ok()
        })
        .with_format(DARKMATTER_JSON_FORMAT, |value: &str| {
            // `json` is strict: only well-formed JSON parses, so YAML-only
            // syntax (e.g. `title: Foo`) is rejected.
            serde_json::from_str::<Value>(value).is_ok()
        })
        // The `expression` content-format string type. Parse-only and
        // side-effect-free: `parse_condition` never evaluates functions, shell,
        // I/O, or context. Condition mode is the either-dialect superset (§Q2),
        // so a value valid in either expression dialect validates here.
        .with_format(DARKMATTER_EXPRESSION_FORMAT, |value: &str| {
            // A value still holding an unresolved `$(...)` shell expression or
            // `{{ ... }}` template is pending, not a final expression: defer
            // rather than eager-fail the parse. This mirrors the pending-value
            // deferral the compose/validation layers apply to every content
            // string, and neither marker is part of expression syntax, so the
            // guard never masks a genuinely malformed expression.
            if value.contains("$(") || value.contains("{{") {
                return true;
            }
            crate::markdown::compose::expression::parse_condition(value).is_ok()
        })
        // ISO 8601 date/time types with an optional zone offset (the built-in
        // `date-time` / `time` formats require one; see the const docs).
        .with_format(DARKMATTER_DATETIME_FORMAT, |value: &str| {
            valid_iso8601_datetime(value)
        })
        .with_format(DARKMATTER_TIME_FORMAT, |value: &str| valid_iso8601_time(value))
}

/// Validates a string by parsing it as a `FileReference` and confirming the
/// resolved path exists on disk at validation time.
///
/// Resolution runs through the shared document-backed context (see
/// [`resolve_file_reference`]). Failures (parse, resolution, missing file)
/// all return `false` so the JSON Schema layer surfaces a uniform
/// `format: darkmatter-file` error message.
fn validate_file_reference(
    value: &str,
    base_dir: Option<&Path>,
    fallback: Option<&Path>,
    file_resolution_context: Option<&biscuit_file::FileResolutionContext>,
) -> bool {
    resolve_file_reference_in_context(value, base_dir, fallback, file_resolution_context).is_ok()
}

/// Outcome of a [`resolve_file_reference`] call.
///
/// Distinguishes the three ways a value can fail to resolve so error
/// reporting can target the right remediation hint (fix the syntax, fix
/// the context that prevented resolution, or point at the missing file).
#[derive(Debug)]
pub(crate) enum FileReferenceFailure {
    /// The string is not a parseable file reference (e.g. an unterminated
    /// `vault:` prefix).
    InvalidSyntax {
        /// The original input, retained for inclusion in the user-facing
        /// message.
        raw: String,
        /// The underlying parser error.
        err: FileReferenceError,
    },
    /// The reference parsed but could not be resolved against the
    /// filesystem or environment.
    Resolution {
        /// The original input, retained for inclusion in the user-facing
        /// message.
        raw: String,
        /// The underlying resolver error.
        err: FileReferenceError,
    },
    /// The reference parsed and resolved, but no file exists at the
    /// resolved path.
    NoMatch {
        /// The original input, retained for inclusion in the user-facing
        /// message.
        raw: String,
        /// The directory resolution anchored on, used to render the
        /// `while resolving from <dir>` clause. This is the document base
        /// directory for an anchored resolution; for the bare-API path with no
        /// document anchor it is the ambient process CWD (which is where that
        /// path genuinely resolves). `None` when no anchor is known.
        resolved_from: Option<PathBuf>,
    },
}

impl fmt::Display for FileReferenceFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSyntax { raw, err } => {
                write!(f, "`{raw}` is not a valid file reference: {err}")
            }
            Self::Resolution { raw, err } => {
                write!(f, "could not resolve file reference `{raw}`: {err}")
            }
            Self::NoMatch { raw, resolved_from } => match resolved_from {
                Some(dir) => write!(
                    f,
                    "no existing file matched reference `{raw}` while resolving from `{}`",
                    dir.display()
                ),
                None => write!(f, "no existing file matched reference `{raw}`"),
            },
        }
    }
}

/// Parses `value` as a `FileReference`, resolves it, and confirms the resolved
/// path exists.
///
/// When a document `base_dir` is supplied the reference resolves through the
/// shared document-backed context ([`resolve_document_file_ref`]): explicit
/// `./`/`../` from the base only, implicit bare paths repository-root first then
/// the base, and the special kinds by their existing `FileReference` semantics.
/// There is **no** launch-area fallback for these document-authored references
/// (D2) and **no** ambient-CWD read — the `_fallback` (launch-area) anchor is
/// retained on the signature only for structural compatibility with the
/// validator-cache anchors and is not a resolution input.
///
/// When `base_dir` is `None` — the bare validator API (`DarkmatterSchemas::new`
/// with no document anchor); never the document-backed compose path, which
/// always supplies a base — resolution falls back to the ambient process CWD via
/// [`FileReference::resolve`]. A no-match on that branch legitimately reports the
/// ambient CWD as the directory it resolved against.
///
/// Returns the resolved path on success, or a [`FileReferenceFailure`]
/// distinguishing the three failure modes so callers can render a
/// situation-appropriate diagnostic.
pub(crate) fn resolve_file_reference(
    value: &str,
    base_dir: Option<&Path>,
    fallback: Option<&Path>,
) -> Result<PathBuf, FileReferenceFailure> {
    resolve_file_reference_in_context(value, base_dir, fallback, None)
}

fn resolve_file_reference_in_context(
    value: &str,
    base_dir: Option<&Path>,
    _fallback: Option<&Path>,
    file_resolution_context: Option<&biscuit_file::FileResolutionContext>,
) -> Result<PathBuf, FileReferenceFailure> {
    let reference = FileReference::new(value).map_err(|err| FileReferenceFailure::InvalidSyntax {
        raw: value.to_string(),
        err,
    })?;
    let resolved = match base_dir {
        // Document-backed: repository-first then source-relative, no launch-area
        // fallback (D2) and no ambient CWD.
        Some(base_dir) => {
            let (repository_root, package_area) = match file_resolution_context {
                Some(snapshot) => (
                    snapshot.repository_root().map(Path::to_path_buf),
                    snapshot.package_area().map(Path::to_path_buf),
                ),
                None => {
                    let repository_root = crate::markdown::compose::find_git_root_from(base_dir);
                    let package_area = crate::markdown::compose::find_package_area_from(
                        base_dir,
                        repository_root.as_deref(),
                    );
                    (repository_root, package_area)
                }
            };
            resolve_document_file_ref(
                &reference,
                base_dir,
                repository_root.as_deref(),
                package_area.as_deref(),
                &[],
                file_resolution_context,
            )
        }
        // Bare validator API with no document anchor: resolve against the
        // ambient process CWD. Unreachable from a real compose run.
        None => reference.resolve(),
    };
    let path = resolved.map_err(|err| FileReferenceFailure::Resolution {
        raw: value.to_string(),
        err,
    })?;
    // The anchored path reports the document base directory; the bare-API path
    // resolved against the ambient CWD and reports that. No CWD re-read leaks
    // into an anchored diagnostic.
    let resolved_from = || {
        base_dir
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
    };
    let path = path.ok_or_else(|| FileReferenceFailure::NoMatch {
        raw: value.to_string(),
        resolved_from: resolved_from(),
    })?;
    if !path.exists() {
        return Err(FileReferenceFailure::NoMatch {
            raw: value.to_string(),
            resolved_from: resolved_from(),
        });
    }
    Ok(path)
}

/// Factory for the `x-darkmatter-url-scheme` keyword.
///
/// Reads the allowed schemes from the schema value (a non-empty array of
/// strings). All schemes are lowercased on both sides before comparison.
pub fn url_scheme_keyword_factory<'a>(
    _parent: &'a Map<String, Value>,
    schema: &'a Value,
    _schema_path: Location,
) -> Result<Box<dyn Keyword>, ValidationError<'a>> {
    let arr = schema.as_array().ok_or_else(|| {
        ValidationError::schema("x-darkmatter-url-scheme must be an array of strings")
    })?;
    if arr.is_empty() {
        return Err(ValidationError::schema(
            "x-darkmatter-url-scheme must contain at least one scheme",
        ));
    }

    let mut schemes = Vec::with_capacity(arr.len());
    for (idx, item) in arr.iter().enumerate() {
        let raw = item.as_str().ok_or_else(|| {
            ValidationError::schema(format!("x-darkmatter-url-scheme[{idx}] must be a string"))
        })?;
        schemes.push(raw.to_ascii_lowercase());
    }

    Ok(Box::new(DarkmatterUrlSchemeKeyword { schemes }))
}

struct DarkmatterUrlSchemeKeyword {
    schemes: Vec<String>,
}

impl DarkmatterUrlSchemeKeyword {
    fn check(&self, value: &str) -> bool {
        let Ok(url) = Url::parse(value) else {
            return false;
        };
        let scheme = url.scheme().to_ascii_lowercase();
        self.schemes.iter().any(|s| s == &scheme)
    }
}

impl Keyword for DarkmatterUrlSchemeKeyword {
    fn validate<'i>(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        match instance {
            Value::String(s) if self.check(s) => Ok(()),
            Value::String(s) => Err(ValidationError::custom(format!(
                "`{s}` does not use an allowed URL scheme ({})",
                self.schemes.join(", ")
            ))),
            _ => Ok(()),
        }
    }

    fn is_valid(&self, instance: &Value) -> bool {
        match instance {
            Value::String(s) => self.check(s),
            _ => true,
        }
    }
}

/// Factory for the grammar-backed `x-darkmatter-type-definition` keyword.
pub fn type_definition_keyword_factory<'a>(
    _parent: &'a Map<String, Value>,
    schema: &'a Value,
    _schema_path: Location,
) -> Result<Box<dyn Keyword>, ValidationError<'a>> {
    require_enabled_semantic_keyword(schema, DARKMATTER_TYPE_DEFINITION_KEYWORD)?;
    Ok(Box::new(DarkmatterTypeDefinitionKeyword))
}

/// Factory for the grammar-backed `x-darkmatter-schema` keyword.
pub fn schema_keyword_factory<'a>(
    _parent: &'a Map<String, Value>,
    schema: &'a Value,
    _schema_path: Location,
) -> Result<Box<dyn Keyword>, ValidationError<'a>> {
    require_enabled_semantic_keyword(schema, DARKMATTER_SCHEMA_KEYWORD)?;
    Ok(Box::new(DarkmatterSchemaKeyword))
}

fn require_enabled_semantic_keyword<'a>(
    schema: &'a Value,
    keyword: &str,
) -> Result<(), ValidationError<'a>> {
    if schema.as_bool() == Some(true) {
        Ok(())
    } else {
        Err(ValidationError::schema(format!("{keyword} must be true")))
    }
}

fn parse_type_definition_instance(instance: &Value) -> Result<(), String> {
    let yaml = serde_yaml_ng::to_value(instance).map_err(|error| error.to_string())?;
    parse_property_definition("<instance>", &yaml)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn parse_schema_instance(instance: &Value) -> Result<(), String> {
    let yaml = serde_yaml_ng::to_value(instance).map_err(|error| error.to_string())?;
    parse_schema_declaration(&yaml)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

struct DarkmatterTypeDefinitionKeyword;

impl Keyword for DarkmatterTypeDefinitionKeyword {
    fn validate<'i>(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        parse_type_definition_instance(instance).map_err(ValidationError::custom)
    }

    fn is_valid(&self, instance: &Value) -> bool {
        parse_type_definition_instance(instance).is_ok()
    }
}

struct DarkmatterSchemaKeyword;

impl Keyword for DarkmatterSchemaKeyword {
    fn validate<'i>(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        parse_schema_instance(instance).map_err(ValidationError::custom)
    }

    fn is_valid(&self, instance: &Value) -> bool {
        parse_schema_instance(instance).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    /// RAII guard that captures the process CWD on construction, switches to
    /// the requested directory, and restores the captured CWD on drop —
    /// including on panic.
    ///
    /// Tests that mutate CWD are annotated with
    /// `#[serial_test::serial("darkmatter-file-cwd")]` so concurrent tests
    /// across this module and `validate::tests` cannot race on process-global
    /// state.
    struct CwdGuard {
        prior: PathBuf,
    }

    impl CwdGuard {
        fn enter(dir: &Path) -> Self {
            let prior = std::env::current_dir().expect("read CWD");
            std::env::set_current_dir(dir).expect("set CWD");
            Self { prior }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            // Best-effort restore; a failure here cannot be reported via the
            // panic path without masking the original test failure.
            let _ = std::env::set_current_dir(&self.prior);
        }
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("create temp dir")
    }

    #[test]
    fn eager_file_validation_reuses_request_repository() {
        let request_repo = temp_dir();
        let nested_repo = temp_dir();
        std::fs::create_dir_all(request_repo.path().join(".git")).unwrap();
        std::fs::create_dir_all(nested_repo.path().join(".git/docs")).unwrap();
        let request_target = request_repo.path().join("spec.md");
        std::fs::write(&request_target, "request").unwrap();
        let context = biscuit_file::FileResolutionContext::new(request_repo.path())
            .with_repository_root(request_repo.path())
            .for_trusted_external_base(nested_repo.path().join("docs"));

        let resolved = resolve_file_reference_in_context(
            "spec.md",
            Some(&nested_repo.path().join("docs")),
            None,
            Some(&context),
        )
        .unwrap();

        assert_eq!(resolved, request_target);
    }

    #[test]
    fn iso8601_datetime_accepts_offset_optional_and_rejects_garbage() {
        // Offset-less local datetime — valid ISO 8601, rejected by RFC 3339.
        assert!(valid_iso8601_datetime("2026-07-10T15:05:34"));
        assert!(valid_iso8601_datetime("2026-07-10T15:05"));
        // Offset-bearing forms.
        assert!(valid_iso8601_datetime("2026-07-10T15:05:34Z"));
        assert!(valid_iso8601_datetime("2026-07-10T15:05:34+01:00"));
        assert!(valid_iso8601_datetime("2026-07-10T15:05:34.123Z"));
        // Range-checked: impossible components are rejected, not merely matched.
        assert!(!valid_iso8601_datetime("2026-13-99T25:61:61"));
        assert!(!valid_iso8601_datetime("not-a-datetime"));
        assert!(!valid_iso8601_datetime("2026-07-10"));
    }

    #[test]
    fn iso8601_time_accepts_offset_optional_and_rejects_garbage() {
        assert!(valid_iso8601_time("15:05:34"));
        assert!(valid_iso8601_time("15:05"));
        assert!(valid_iso8601_time("15:05:34Z"));
        assert!(valid_iso8601_time("15:05:34+01:00"));
        assert!(!valid_iso8601_time("25:61:61"));
        assert!(!valid_iso8601_time("noon"));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_format_accepts_existing_file() {
        let dir = temp_dir();
        let path = dir.path().join("README.md");
        std::fs::write(&path, b"x").unwrap();
        let _cwd = CwdGuard::enter(dir.path());
        assert!(validate_file_reference("./README.md", None, None, None));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_format_rejects_missing_file() {
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        assert!(!validate_file_reference(
            "./does-not-exist.md",
            None,
            None,
            None,
        ));
    }

    #[test]
    fn file_format_package_reference_prefers_package_area_over_repository() {
        let dir = temp_dir();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        gix::init(&root).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"darkmatter/lib\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        let package_area = root.join("darkmatter");
        let member = package_area.join("lib");
        std::fs::create_dir_all(member.join("src")).unwrap();
        std::fs::create_dir_all(member.join("docs")).unwrap();
        std::fs::write(
            member.join("Cargo.toml"),
            "[package]\nname = \"fixture-darkmatter\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(member.join("src/lib.rs"), "").unwrap();
        std::fs::write(root.join("shared.md"), "repository decoy").unwrap();
        let package_target = package_area.join("shared.md");
        std::fs::write(&package_target, "package").unwrap();

        assert_eq!(
            std::fs::canonicalize(
                resolve_file_reference("!shared.md", Some(&member.join("docs")), None).unwrap()
            )
            .unwrap(),
            std::fs::canonicalize(package_target).unwrap(),
        );
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn lazy_reference_format_accepts_missing_file() {
        // Eager sibling of `file_format_rejects_missing_file`: the lazy,
        // syntax-only validator accepts a syntactically valid, not-yet-existing
        // path (no resolve, no existence check).
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        assert!(FileReference::new("./does-not-exist.md").is_ok());
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn raw_json_schema_format_compat_eager_vs_lazy() {
        // Raw JSON Schema compatibility contract: `format: darkmatter-file`
        // stays eager (rejects a missing file), while
        // `format: darkmatter-file-reference` is lazy syntax-only (accepts the
        // same missing path, rejects only malformed syntax). Raw JSON Schema
        // authors keep the established eager `darkmatter-file` semantics.
        let dir = temp_dir();
        std::fs::write(dir.path().join("exists.md"), b"x").unwrap();
        let _cwd = CwdGuard::enter(dir.path());

        let build = |format: &str| {
            register_darkmatter_formats(
                jsonschema::options().with_draft(jsonschema::Draft::Draft202012),
                None,
                None,
            )
            .should_validate_formats(true)
            .build(&json!({ "type": "string", "format": format }))
            .expect("validator builds")
        };
        let eager = build(DARKMATTER_FILE_FORMAT);
        let lazy = build(DARKMATTER_FILE_REFERENCE_FORMAT);

        let missing = json!("./missing.md");
        let existing = json!("./exists.md");
        // The empty string is rejected at `FileReference` parse time — the
        // canonical malformed-syntax input.
        let malformed = json!("");

        assert!(!eager.is_valid(&missing), "eager rejects missing");
        assert!(eager.is_valid(&existing), "eager accepts existing");
        assert!(lazy.is_valid(&missing), "lazy accepts missing");
        assert!(lazy.is_valid(&existing), "lazy accepts existing");
        assert!(!lazy.is_valid(&malformed), "lazy rejects malformed syntax");
        assert!(!eager.is_valid(&malformed), "eager rejects malformed syntax");
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn lazy_reference_format_accepts_missing_path_that_eager_rejects() {
        // Phase 2 checkpoint: the same syntactically valid, not-yet-existing
        // path passes the lazy `darkmatter-file-reference` validator and fails
        // the eager `darkmatter-file` validator. Built through
        // `register_darkmatter_formats` so both closures are exercised exactly
        // as the production validator wires them.
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        let missing = serde_json::json!("./not-created-yet.md");

        let build = |format: &str| {
            register_darkmatter_formats(
                jsonschema::options().with_draft(jsonschema::Draft::Draft202012),
                None,
                None,
            )
            .should_validate_formats(true)
            .build(&json!({ "type": "string", "format": format }))
            .expect("validator builds")
        };

        let lazy = build(DARKMATTER_FILE_REFERENCE_FORMAT);
        let eager = build(DARKMATTER_FILE_FORMAT);

        assert!(
            lazy.is_valid(&missing),
            "lazy darkmatter-file-reference must accept a syntactically valid missing path",
        );
        assert!(
            !eager.is_valid(&missing),
            "eager darkmatter-file must reject a missing path",
        );

        // Laziness defers existence, not syntax: both reject malformed input
        // (an empty reference string).
        let malformed = serde_json::json!("");
        assert!(!lazy.is_valid(&malformed), "lazy must reject malformed syntax");
        assert!(!eager.is_valid(&malformed), "eager must reject malformed syntax");
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn resolve_file_reference_returns_path_for_existing_file() {
        let dir = temp_dir();
        let path = dir.path().join("README.md");
        std::fs::write(&path, b"x").unwrap();
        let _cwd = CwdGuard::enter(dir.path());
        let resolved = resolve_file_reference("./README.md", None, None).expect("should resolve");
        // On macOS the temp dir is exposed under both /var/folders/... and
        // /private/var/folders/... depending on how the path is rooted, so
        // compare existence and the trailing component rather than full
        // string equality.
        assert!(resolved.exists());
        assert_eq!(resolved.file_name(), path.file_name());
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn resolve_file_reference_reports_missing_file() {
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        let err = resolve_file_reference("./does-not-exist.md", None, None)
            .expect_err("should fail with NoMatch");
        let rendered = err.to_string();
        assert!(
            rendered.contains("no existing file matched reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains("`./does-not-exist.md`"), "rendered: {rendered}");
        assert!(rendered.contains("while resolving from"), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::NoMatch { resolved_from: Some(_), .. }));
    }

    #[test]
    fn resolve_file_reference_reports_invalid_syntax() {
        // Empty input is rejected at parse time.
        let err = resolve_file_reference("", None, None).expect_err("should fail with InvalidSyntax");
        let rendered = err.to_string();
        assert!(
            rendered.contains("is not a valid file reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains("``"), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::InvalidSyntax { .. }));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn resolve_file_reference_reports_resolution_error_for_unset_env_var() {
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        let var_name = "DARKMATTER_TEST_UNSET_ENV_REF_98765";
        // Safety: the name is unique enough that no other process should set
        // it, but unset it defensively before resolving.
        unsafe { std::env::remove_var(var_name); }
        let raw = format!("{{{{{var_name}}}}}/notes.md");
        let err = resolve_file_reference(&raw, None, None).expect_err("should fail with Resolution");
        let rendered = err.to_string();
        assert!(
            rendered.contains("could not resolve file reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains(&format!("`{raw}`")), "rendered: {rendered}");
        assert!(
            rendered.contains(&format!("environment variable `{var_name}` is not set")),
            "rendered: {rendered}"
        );
        assert!(matches!(err, FileReferenceFailure::Resolution { .. }));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn resolve_file_reference_reports_resolution_error_for_unconfigured_vault() {
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        let err = resolve_file_reference("vault:notes/today.md", None, None)
            .expect_err("should fail with Resolution");
        let rendered = err.to_string();
        assert!(
            rendered.contains("could not resolve file reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains("`vault:notes/today.md`"), "rendered: {rendered}");
        assert!(
            rendered.contains("vault reference used without any configured vault roots"),
            "rendered: {rendered}"
        );
        assert!(matches!(err, FileReferenceFailure::Resolution { .. }));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn resolve_file_reference_no_match_for_missing_absolute_path() {
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        let raw = "/tmp/darkmatter-test-missing-absolute-xyz.md";
        let err = resolve_file_reference(raw, None, None).expect_err("should fail with NoMatch");
        let rendered = err.to_string();
        assert!(
            rendered.contains("no existing file matched reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains(&format!("`{raw}`")), "rendered: {rendered}");
        // The contract does not fabricate a candidate absolute path; it only
        // reports the raw reference and the resolution directory.
        assert!(!rendered.contains("/darkmatter-test-missing-absolute-xyz.md "), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::NoMatch { resolved_from: Some(_), .. }));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn resolve_file_reference_no_match_for_missing_magic_path() {
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        let raw = "@darkmatter-test-missing-magic-xyz.md";
        let err = resolve_file_reference(raw, None, None).expect_err("should fail with NoMatch");
        let rendered = err.to_string();
        assert!(
            rendered.contains("no existing file matched reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains(&format!("`{raw}`")), "rendered: {rendered}");
        assert!(!rendered.contains("darkmatter-test-missing-magic-xyz.md "), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::NoMatch { .. }));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn resolve_file_reference_no_match_for_missing_package_path() {
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        let raw = "!darkmatter-test-missing-package-xyz.md";
        let err = resolve_file_reference(raw, None, None).expect_err("should fail with NoMatch");
        let rendered = err.to_string();
        assert!(
            rendered.contains("no existing file matched reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains(&format!("`{raw}`")), "rendered: {rendered}");
        assert!(!rendered.contains("darkmatter-test-missing-package-xyz.md "), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::NoMatch { .. }));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn resolve_file_reference_no_match_for_missing_recursive_path() {
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        let raw = "%darkmatter-test-missing-recursive-xyz.md";
        let err = resolve_file_reference(raw, None, None).expect_err("should fail with NoMatch");
        let rendered = err.to_string();
        assert!(
            rendered.contains("no existing file matched reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains(&format!("`{raw}`")), "rendered: {rendered}");
        assert!(!rendered.contains("darkmatter-test-missing-recursive-xyz.md "), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::NoMatch { resolved_from: Some(_), .. }));
    }

    #[test]
    fn file_reference_failure_no_match_omits_resolved_from_when_unknown() {
        let resolved_from: Option<PathBuf> = None;
        let failure = FileReferenceFailure::NoMatch {
            raw: "./x".to_string(),
            resolved_from,
        };
        let rendered = failure.to_string();
        assert_eq!(
            rendered,
            "no existing file matched reference `./x`",
        );
    }

    #[test]
    fn url_scheme_keyword_accepts_match() {
        let parent = Map::new();
        let kw =
            url_scheme_keyword_factory(&parent, &json!(["https", "http"]), Location::default())
                .unwrap();
        assert!(kw.is_valid(&Value::String("https://example.com".into())));
        assert!(kw.is_valid(&Value::String("HTTP://example.com".into())));
        assert!(!kw.is_valid(&Value::String("ftp://example.com".into())));
    }

    #[test]
    fn url_scheme_keyword_rejects_non_url() {
        let parent = Map::new();
        let kw =
            url_scheme_keyword_factory(&parent, &json!(["https"]), Location::default()).unwrap();
        assert!(!kw.is_valid(&Value::String("not a url".into())));
    }

    #[test]
    fn url_scheme_keyword_rejects_empty_arr() {
        let parent = Map::new();
        let empty = json!([]);
        let err = url_scheme_keyword_factory(&parent, &empty, Location::default())
            .err()
            .expect("expected factory error");
        assert!(err.to_string().contains("at least one scheme"));
    }
}

/// End-to-end coverage for the schema-plus content-format string types
/// (`darkmatter/features/2026-07-08-schema-plus/`, Feature D). Each exercises
/// the public parse → convert → coerce → validate path — the same order
/// [`crate::markdown::schemas::EffectiveSchema::validate`] runs, so a native
/// value is serialized by the coercion pass before the `format` validator sees
/// it.
#[cfg(test)]
mod schema_plus_content_formats {
    use serde_json::{Value, json};

    /// Runs the effective-schema validation path for a one-line SimplifiedSchema
    /// body against `instance`: convert, coerce a transient copy, then validate.
    /// Returns `true` when the coerced instance validates.
    fn accepts(schema_yaml: &str, instance: &Value) -> bool {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(schema_yaml).expect("yaml");
        let schema =
            crate::markdown::schemas::simplified::parse_yaml_schema(&v).expect("parse schema");
        let json = crate::markdown::schemas::simplified::to_json_schema(&schema).expect("convert");
        let coerced = crate::markdown::schemas::coerce::coerce_frontmatter(&json, instance);
        let validator = crate::markdown::schemas::validate::build_validator(&json, None, None)
            .expect("build validator");
        validator.is_valid(&coerced.value)
    }

    #[test]
    fn yaml_accepts_yaml_string() {
        assert!(accepts(
            "frontmatter: yaml",
            &json!({ "frontmatter": "title: Foo\ntags: [a, b]" })
        ));
    }

    #[test]
    fn yaml_accepts_json_string() {
        assert!(accepts(
            "frontmatter: yaml",
            &json!({ "frontmatter": "{\"title\": \"Foo\"}" })
        ));
    }

    #[test]
    fn yaml_accepts_native_mapping() {
        assert!(accepts(
            "frontmatter: yaml",
            &json!({ "frontmatter": { "title": "Foo", "tags": ["a", "b"] } })
        ));
    }

    #[test]
    fn yaml_rejects_malformed() {
        assert!(!accepts(
            "frontmatter: yaml",
            &json!({ "frontmatter": "key: [unterminated" })
        ));
    }

    #[test]
    fn json_rejects_yaml_only() {
        assert!(!accepts("config: json", &json!({ "config": "title: Foo" })));
    }

    #[test]
    fn json_accepts_json_string() {
        assert!(accepts(
            "config: json",
            &json!({ "config": "{\"title\": \"Foo\"}" })
        ));
    }

    /// The `{ frontmatter: yaml }` union arm from `example.yaml`'s `invocation`
    /// union (Phase 5 validation checkpoint): the string arm accepts a plain
    /// expression string, and the inline-object arm accepts both an authored
    /// YAML string and a native mapping (coerced to a YAML string).
    #[test]
    fn frontmatter_yaml_union_arm_accepts_string_and_native_mapping() {
        let schema = "invocation:\n    - string(required)\n    - { frontmatter: yaml }\n";
        // Plain string invocation → string arm.
        assert!(accepts(schema, &json!({ "invocation": "as_csv(list)" })));
        // Frontmatter block authored as a YAML string → inline-object arm.
        assert!(accepts(
            schema,
            &json!({ "invocation": { "frontmatter": "list: [1, 2, 3]" } })
        ));
        // Frontmatter block as a native mapping → coerced to a YAML string.
        assert!(accepts(
            schema,
            &json!({ "invocation": { "frontmatter": { "list": [1, 2, 3] } } })
        ));
    }
}
