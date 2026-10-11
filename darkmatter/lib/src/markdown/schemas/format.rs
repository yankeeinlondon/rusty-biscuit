//! Custom format and keyword validators for the schemas subsystem.
//!
//! Several darkmatter-specific schema fragments require validators beyond the
//! built-in JSON Schema vocabulary:
//!
//! - **`format: darkmatter-file`** (eager) — parses the value through
//!   [`biscuit_file::FileReference`] and confirms the resolved path exists.
//!   Resolution runs through the shared document-backed context
//!   ([`resolve_document_file_ref`]): implicit bare paths resolve
//!   prompt document directory first then the repository root, explicit
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
//! `Keyword` implementations. `match(...)` is suggestion metadata carried on
//! the SimplifiedSchema atom (`Constraint::Match` → completion); it is
//! validated for each declaring root-union arm, through the
//! `x-darkmatter-match` keyword in [`super::file_match`].
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
/// through the shared document-backed context (document-first then
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
/// references resolve from the document directory first, then the repository root, and
/// explicit `./`/`../` from the document directory only. When `None` (the bare
/// validator API with no document anchor) the validator resolves from
/// `context`'s `cwd`; it never reads the process's directory.
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
/// `jsonschema`'s `with_format` accepts `F: Fn(&str) -> bool + Send + Sync +
/// 'static`, so the anchors are captured by move into the closure — no
/// thread-local or process-global state is required.
pub fn register_darkmatter_formats(
    options: ValidationOptions,
    base_dir: Option<PathBuf>,
    fallback: Option<PathBuf>,
    context: biscuit_file::FileResolutionContext,
) -> ValidationOptions {
    register_darkmatter_formats_with(
        options,
        super::validate::FileValues::Resolved {
            base_dir,
            fallback,
            context: Box::new(context),
            callers: Default::default(),
        },
    )
}

/// Registers the formats with `format: darkmatter-file` judged as
/// `file_values` says: resolved in a request's context, or by syntax alone.
pub(crate) fn register_darkmatter_formats_with(
    options: ValidationOptions,
    file_values: super::validate::FileValues,
) -> ValidationOptions {
    options
        .with_format(DARKMATTER_FILE_FORMAT, move |value: &str| match &file_values {
            super::validate::FileValues::Syntax => match FileReference::new(value) {
                Ok(_) => context_free_path(value).is_none_or(|path| path.exists()),
                Err(_) => false,
            },
            super::validate::FileValues::Resolved { base_dir, fallback, context, .. } => {
                resolve_file_reference_in_context(value, base_dir.as_deref(), fallback.as_deref(), context)
                    .is_ok()
            }
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
            // A stored literal token is data: its text is the final value.
            if let Some(Ok(decoded)) = crate::markdown::literal_token::decode_leaf(value) {
                return crate::markdown::compose::expression::parse_condition(&decoded).is_ok();
            }
            if is_pending_expression_value(value) {
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

/// True when an `expression`-typed value still holds an unresolved `$(...)`
/// shell expression or `{{ ... }}` template. Such a value is pending, not a
/// final expression, so validation defers rather than eager-failing the parse.
/// A whole-value literal token is stored data and never pending.
///
/// Lexical only: nothing is evaluated, executed, or read. Neither marker is
/// expression syntax, so deferral never masks a malformed final expression.
/// Editor analysis must call this before parsing so it defers exactly where
/// schema validation does.
pub fn is_pending_expression_value(value: &str) -> bool {
    crate::markdown::literal_token::holds_pending_syntax(value)
}

/// The path an absolute `value` names, which needs no context to judge;
/// `None` for a value whose meaning depends on a context (a relative, `&`,
/// `^`, `@`, `~`, vault, or `{{VAR}}` reference) or that is malformed.
///
/// Structural validators judge such a path exactly as a context-bound one
/// does and every other value by its syntax alone.
pub(crate) fn context_free_path(value: &str) -> Option<PathBuf> {
    let reference = FileReference::new(value).ok()?;
    (reference.class().kind == biscuit_file::FileReferenceKind::Absolute)
        .then(|| PathBuf::from(reference.raw()))
}

/// Outcome of a [`resolve_file_reference_in_context`] call.
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
            Self::NoMatch { raw, resolved_from } => {
                let message = match resolved_from {
                    Some(dir) => format!(
                        "no existing file matched reference `{raw}` while resolving from `{}`",
                        dir.display()
                    ),
                    None => format!("no existing file matched reference `{raw}`"),
                };
                f.write_str(&crate::markdown::errors::with_glob_hint(
                    message,
                    biscuit_file::ResolutionFailure::NoMatch,
                    raw,
                ))
            }
        }
    }
}

/// Parses `value` as a `FileReference`, resolves it in `file_resolution_context`
/// from `base_dir` (the context's `cwd` when `None`), and confirms the
/// resolved path exists.
///
/// The reference resolves through the shared document-backed context
/// ([`resolve_document_file_ref`]): explicit `./`/`../` from the base only,
/// implicit bare paths from the base first then the repository root, and the
/// special kinds by their existing `FileReference` semantics. There is **no**
/// launch-area fallback for these document-authored references (D2) and **no**
/// ambient-CWD read — the `_fallback` (launch-area) anchor is retained on the
/// signature only for structural compatibility with the validator-cache
/// anchors and is not a resolution input.
///
/// Returns the resolved path on success, or a [`FileReferenceFailure`]
/// distinguishing the three failure modes so callers can render a
/// situation-appropriate diagnostic.
pub(crate) fn resolve_file_reference_in_context(
    value: &str,
    base_dir: Option<&Path>,
    _fallback: Option<&Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<PathBuf, FileReferenceFailure> {
    let reference = FileReference::new(value).map_err(|err| FileReferenceFailure::InvalidSyntax {
        raw: value.to_string(),
        err,
    })?;
    // Document-backed: document-first then repository-relative, no launch-area
    // fallback (D2) and no ambient CWD.
    let base_dir = base_dir.unwrap_or_else(|| file_resolution_context.cwd());
    let path = resolve_document_file_ref(&reference, base_dir, file_resolution_context).map_err(|err| {
        FileReferenceFailure::Resolution {
            raw: value.to_string(),
            err,
        }
    })?;
    let resolved_from = || Some(base_dir.to_path_buf());
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
) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
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

impl<'i> Keyword<'i> for DarkmatterUrlSchemeKeyword {
    fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        match instance {
            Value::String(s) if self.check(s) => Ok(()),
            Value::String(s) => Err(ValidationError::custom(format!(
                "`{s}` does not use an allowed URL scheme ({})",
                self.schemes.join(", ")
            ))),
            _ => Ok(()),
        }
    }

    fn is_valid(&self, instance: &'i Value) -> bool {
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
) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
    require_enabled_semantic_keyword(schema, DARKMATTER_TYPE_DEFINITION_KEYWORD)?;
    Ok(Box::new(DarkmatterTypeDefinitionKeyword))
}

/// Factory for the grammar-backed `x-darkmatter-schema` keyword.
pub fn schema_keyword_factory<'a>(
    _parent: &'a Map<String, Value>,
    schema: &'a Value,
    _schema_path: Location,
) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
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

impl<'i> Keyword<'i> for DarkmatterTypeDefinitionKeyword {
    fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        parse_type_definition_instance(instance).map_err(ValidationError::custom)
    }

    fn is_valid(&self, instance: &'i Value) -> bool {
        parse_type_definition_instance(instance).is_ok()
    }
}

struct DarkmatterSchemaKeyword;

impl<'i> Keyword<'i> for DarkmatterSchemaKeyword {
    fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        parse_schema_instance(instance).map_err(ValidationError::custom)
    }

    fn is_valid(&self, instance: &'i Value) -> bool {
        parse_schema_instance(instance).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    /// The old context-free path anchored at the process CWD; these tests
    /// anchor the same lookups at the context's `cwd` instead.
    fn context_at(dir: &Path) -> biscuit_file::FileResolutionContext {
        biscuit_file::FileResolutionContext::new(dir)
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("create temp dir")
    }

    /// An external document (a `cwd` in another repository) has no
    /// repository of its own unless a catalog supplies one: a bare reference
    /// neither borrows the request repository nor rediscovers the child's.
    /// The request repository stays reachable through the launch `@` scope.
    #[test]
    fn eager_file_validation_of_an_external_document_keeps_only_the_launch_scope() {
        let request_repo = temp_dir();
        let nested_repo = temp_dir();
        std::fs::create_dir_all(request_repo.path().join(".git")).unwrap();
        std::fs::create_dir_all(nested_repo.path().join(".git")).unwrap();
        std::fs::create_dir_all(nested_repo.path().join("docs")).unwrap();
        let request_target = request_repo.path().join("spec.md");
        std::fs::write(&request_target, "request").unwrap();
        std::fs::write(nested_repo.path().join("spec.md"), "child decoy").unwrap();
        let context = biscuit_file::FileResolutionContext::new(request_repo.path())
            .with_repository_root(request_repo.path())
            .for_trusted_external_cwd(nested_repo.path().join("docs"));
        assert_eq!(context.repository_root(), None);

        let bare = resolve_file_reference_in_context(
            "spec.md",
            Some(&nested_repo.path().join("docs")),
            None,
            &context,
        );
        assert!(matches!(bare, Err(FileReferenceFailure::NoMatch { .. })), "{bare:?}");

        let magic = resolve_file_reference_in_context(
            "@spec.md",
            Some(&nested_repo.path().join("docs")),
            None,
            &context,
        )
        .unwrap();
        assert_eq!(magic, request_target);
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
    fn file_format_accepts_existing_file() {
        let dir = temp_dir();
        let path = dir.path().join("README.md");
        std::fs::write(&path, b"x").unwrap();
        assert!(resolve_file_reference_in_context("./README.md", None, None, &context_at(dir.path())).is_ok());
    }

    #[test]
    fn file_format_rejects_missing_file() {
        let dir = temp_dir();
        assert!(
            resolve_file_reference_in_context(
                "./does-not-exist.md",
                None,
                None,
                &context_at(dir.path()),
            )
            .is_err()
        );
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

        let base = member.join("docs");
        let context = crate::markdown::compose::build_resolution_context(
            &crate::markdown::compose::RequestSnapshot::new(&base),
        )
        .unwrap();
        assert_eq!(
            std::fs::canonicalize(
                resolve_file_reference_in_context(
                    "^shared.md",
                    Some(&base),
                    None,
                    &context,
                )
                .unwrap(),
            )
            .unwrap(),
            std::fs::canonicalize(package_target).unwrap(),
        );
    }

    #[test]
    fn lazy_reference_format_accepts_missing_file() {
        // Eager sibling of `file_format_rejects_missing_file`: the lazy,
        // syntax-only validator accepts a syntactically valid, not-yet-existing
        // path (no resolve, no existence check).
        assert!(FileReference::new("./does-not-exist.md").is_ok());
    }

    #[test]
    fn raw_json_schema_format_compat_eager_vs_lazy() {
        // Raw JSON Schema compatibility contract: `format: darkmatter-file`
        // stays eager (rejects a missing file), while
        // `format: darkmatter-file-reference` is lazy syntax-only (accepts the
        // same missing path, rejects only malformed syntax). Raw JSON Schema
        // authors keep the established eager `darkmatter-file` semantics.
        let dir = temp_dir();
        std::fs::write(dir.path().join("exists.md"), b"x").unwrap();

        let build = |format: &str| {
            register_darkmatter_formats(
                jsonschema::options().with_draft(jsonschema::Draft::Draft202012),
                None,
                None,
                context_at(dir.path()),
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
    fn lazy_reference_format_accepts_missing_path_that_eager_rejects() {
        // Phase 2 checkpoint: the same syntactically valid, not-yet-existing
        // path passes the lazy `darkmatter-file-reference` validator and fails
        // the eager `darkmatter-file` validator. Built through
        // `register_darkmatter_formats` so both closures are exercised exactly
        // as the production validator wires them.
        let dir = temp_dir();
        let missing = serde_json::json!("./not-created-yet.md");

        let build = |format: &str| {
            register_darkmatter_formats(
                jsonschema::options().with_draft(jsonschema::Draft::Draft202012),
                None,
                None,
                context_at(dir.path()),
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
    fn resolve_file_reference_returns_path_for_existing_file() {
        let dir = temp_dir();
        let path = dir.path().join("README.md");
        std::fs::write(&path, b"x").unwrap();
        let resolved = resolve_file_reference_in_context("./README.md", None, None, &context_at(dir.path())).expect("should resolve");
        // On macOS the temp dir is exposed under both /var/folders/... and
        // /private/var/folders/... depending on how the path is rooted, so
        // compare existence and the trailing component rather than full
        // string equality.
        assert!(resolved.exists());
        assert_eq!(resolved.file_name(), path.file_name());
    }

    #[test]
    fn resolve_file_reference_reports_missing_file() {
        let dir = temp_dir();
        let err = resolve_file_reference_in_context("./does-not-exist.md", None, None, &context_at(dir.path()))
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
        let err = resolve_file_reference_in_context("", None, None, &context_at(&std::env::temp_dir())).expect_err("should fail with InvalidSyntax");
        let rendered = err.to_string();
        assert!(
            rendered.contains("is not a valid file reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains("``"), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::InvalidSyntax { .. }));
    }

    #[test]
    fn resolve_file_reference_reports_resolution_error_for_unset_env_var() {
        let dir = temp_dir();
        let var_name = "DARKMATTER_TEST_UNSET_ENV_REF_98765";
        // The context's environment, not the process's, is consulted; an
        // empty one leaves the variable unset without mutating the process.
        let context = context_at(dir.path()).with_env(std::collections::HashMap::new());
        let raw = format!("{{{{{var_name}}}}}/notes.md");
        let err = resolve_file_reference_in_context(&raw, None, None, &context)
            .expect_err("should fail with Resolution");
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
    fn resolve_file_reference_reports_resolution_error_for_unconfigured_vault() {
        let dir = temp_dir();
        let err = resolve_file_reference_in_context("vault:notes/today.md", None, None, &context_at(dir.path()))
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
    fn resolve_file_reference_no_match_for_missing_absolute_path() {
        let dir = temp_dir();
        // An absolute path on this host: `/tmp/...` is foreign on Windows.
        let raw = biscuit_file::to_portable_string(&dir.path().join("darkmatter-test-missing-absolute-xyz.md"));
        let raw = raw.as_str();
        let err = resolve_file_reference_in_context(raw, None, None, &context_at(dir.path())).expect_err("should fail with NoMatch");
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
    fn resolve_file_reference_no_match_for_missing_magic_path() {
        let dir = temp_dir();
        let raw = "@darkmatter-test-missing-magic-xyz.md";
        let err = resolve_file_reference_in_context(raw, None, None, &context_at(dir.path())).expect_err("should fail with NoMatch");
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
    fn resolve_file_reference_rejects_removed_package_sigil() {
        let dir = temp_dir();
        let raw = "!darkmatter-test-missing-package-xyz.md";
        let err = resolve_file_reference_in_context(raw, None, None, &context_at(dir.path()))
            .expect_err("removed package sigil should fail parsing");
        let rendered = err.to_string();
        assert!(
            rendered.contains("removed") && rendered.contains('^'),
            "rendered: {rendered}"
        );
        assert!(rendered.contains(&format!("`{raw}`")), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::InvalidSyntax { .. }));
    }

    #[test]
    fn resolve_file_reference_no_match_for_missing_recursive_path() {
        let dir = temp_dir();
        let raw = "%darkmatter-test-missing-recursive-xyz.md";
        let err = resolve_file_reference_in_context(raw, None, None, &context_at(dir.path())).expect_err("should fail with NoMatch");
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
        let validator = crate::markdown::schemas::validate::build_structural_validator(&json)
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

    #[test]
    fn pending_expression_values_are_classified_lexically() {
        use super::super::format::is_pending_expression_value;
        for pending in ["{{ x }}", "a == {{ x }}", "$(date)", "x == $(cmd", "{{ not ( valid }}"] {
            assert!(is_pending_expression_value(pending), "{pending:?}");
        }
        for final_value in ["a == b", "a ((", "", "$ (x)", "{ {x} }", "{x: 1}"] {
            assert!(!is_pending_expression_value(final_value), "{final_value:?}");
        }
        // A stored literal token is data, whatever it holds; a malformed one
        // stays pending so composition reports it.
        let token = crate::markdown::literal_token::encode("{{ x }} && $(cmd)");
        assert!(!is_pending_expression_value(&token));
        assert!(is_pending_expression_value("{{!data:v9:YQ}}"));
    }

    #[test]
    fn expression_validation_parses_the_text_a_literal_token_holds() {
        use crate::markdown::literal_token::encode;
        assert!(accepts("when: expression", &json!({ "when": encode("a == b") })));
        assert!(!accepts("when: expression", &json!({ "when": encode("a ((") })));
        assert!(!accepts("when: expression", &json!({ "when": encode("{{ x }}") })));
    }

    #[test]
    fn expression_validation_defers_exactly_on_pending_values() {
        use super::super::format::is_pending_expression_value;
        // Malformed-but-pending values are accepted; a malformed final value
        // is not, so the deferral cannot hide a real defect.
        for (value, accepted) in [
            ("{{ x ( }}", true),
            ("$(broken ((", true),
            ("a && {{ flag }}", true),
            ("ctx.area ? 'a' : 'b'", true),
            ("a ((", false),
        ] {
            assert_eq!(
                accepts("when: expression", &json!({ "when": value })),
                accepted,
                "{value:?}"
            );
            assert_eq!(
                is_pending_expression_value(value),
                accepted && value != "ctx.area ? 'a' : 'b'",
                "{value:?}"
            );
        }
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
