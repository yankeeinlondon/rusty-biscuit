//! Custom format and keyword validators for the schemas subsystem.
//!
//! Two darkmatter-specific schema fragments require validators beyond the
//! built-in JSON Schema vocabulary:
//!
//! - **`format: darkmatter-file`** — parses the value through
//!   [`biscuit_file::FileReference`] and confirms the resolved path exists.
//!   Path resolution uses the live process working directory at validation
//!   time, mirroring the spec contract for `file` properties.
//! - **`x-darkmatter-match`** — runs alongside `darkmatter-file` and filters
//!   the resolved path through one or more glob patterns (positive +
//!   negative). Globsets are compiled once when the validator is built.
//! - **`x-darkmatter-url-scheme`** — runs alongside `format: uri` and
//!   restricts the URL scheme to a configured list (case-insensitive).
//!
//! `darkmatter-file` is a `Format` (it sees only the string) and the two
//! `x-darkmatter-*` keywords are custom `Keyword` implementations (they need
//! the surrounding schema fragment for their constraint list). See the ADR
//! in `schemas.md` for why both shapes are needed.
//!
//! ## Examples
//!
//! ```ignore
//! use jsonschema::{Draft, options};
//! use darkmatter::markdown::schemas::format::{
//!     match_keyword_factory, register_darkmatter_formats, url_scheme_keyword_factory,
//! };
//!
//! let validator = register_darkmatter_formats(
//!     options().with_draft(Draft::Draft202012),
//! )
//! .with_keyword("x-darkmatter-match", match_keyword_factory)
//! .with_keyword("x-darkmatter-url-scheme", url_scheme_keyword_factory)
//! .build(&schema)?;
//! ```

use std::fmt;
use std::path::PathBuf;

use biscuit_file::{FileReference, FileReferenceError};
use globset::{Glob, GlobSet, GlobSetBuilder};
use jsonschema::{Keyword, ValidationError, ValidationOptions, paths::Location};
use serde_json::{Map, Value};
use url::Url;

/// Format name registered for `file` SimplifiedSchema atoms.
pub const DARKMATTER_FILE_FORMAT: &str = "darkmatter-file";

/// Keyword name registered for `file(match(...))` constraints.
pub const DARKMATTER_MATCH_KEYWORD: &str = "x-darkmatter-match";

/// Keyword name registered for `url(scheme(...))` constraints.
pub const DARKMATTER_URL_SCHEME_KEYWORD: &str = "x-darkmatter-url-scheme";

/// Registers the `darkmatter-file` format on a `ValidationOptions` builder.
///
/// Splitting this out keeps validator construction in
/// [`crate::markdown::schemas::validate`] tidy and lets tests register only
/// the format without the keywords.
pub fn register_darkmatter_formats(options: ValidationOptions) -> ValidationOptions {
    options.with_format(DARKMATTER_FILE_FORMAT, validate_file_reference)
}

/// Validates a string by parsing it as a `FileReference` and confirming the
/// resolved path exists on disk at validation time.
///
/// Resolution uses the ambient process working directory. Failures (parse,
/// resolution, missing file) all return `false` so the JSON Schema layer
/// surfaces a uniform `format: darkmatter-file` error message.
fn validate_file_reference(value: &str) -> bool {
    resolve_file_reference(value).is_ok()
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
        /// CWD at the time of the call, used to render the
        /// `while resolving from <cwd>` clause. `None` when the process
        /// CWD cannot be determined.
        cwd: Option<PathBuf>,
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
            Self::NoMatch { raw, cwd } => match cwd {
                Some(cwd) => write!(
                    f,
                    "no existing file matched reference `{raw}` while resolving from `{}`",
                    cwd.display()
                ),
                None => write!(f, "no existing file matched reference `{raw}`"),
            },
        }
    }
}

/// Parses `value` as a `FileReference`, resolves it against the ambient
/// process CWD, and confirms the resolved path exists.
///
/// Returns the resolved path on success, or a [`FileReferenceFailure`]
/// distinguishing the three failure modes so callers can render a
/// situation-appropriate diagnostic.
pub(crate) fn resolve_file_reference(value: &str) -> Result<PathBuf, FileReferenceFailure> {
    let reference = FileReference::new(value).map_err(|err| FileReferenceFailure::InvalidSyntax {
        raw: value.to_string(),
        err,
    })?;
    let path = reference.resolve().map_err(|err| FileReferenceFailure::Resolution {
        raw: value.to_string(),
        err,
    })?;
    let path = path.ok_or_else(|| FileReferenceFailure::NoMatch {
        raw: value.to_string(),
        cwd: std::env::current_dir().ok(),
    })?;
    if !path.exists() {
        return Err(FileReferenceFailure::NoMatch {
            raw: value.to_string(),
            cwd: std::env::current_dir().ok(),
        });
    }
    Ok(path)
}

/// Factory for the `x-darkmatter-match` keyword.
///
/// Reads the globs from the schema value (which must be a non-empty array of
/// strings) and compiles two `GlobSet`s — positive (no `!` prefix) and
/// negative (`!`-prefixed) — at validator-build time. Bad schema input
/// (non-array, non-string members, empty list, or an unparsable glob)
/// surfaces as `ValidationError::schema`.
///
/// Also enforces the `format: darkmatter-file` co-requirement: the glob
/// constraint only makes sense when the parent fragment is a
/// `darkmatter-file` reference. Authors who omit the `format` (or pair it
/// with any other `format`) get a build-time schema error rather than a
/// silently-incorrect validator.
pub fn match_keyword_factory<'a>(
    parent: &'a Map<String, Value>,
    schema: &'a Value,
    _schema_path: Location,
) -> Result<Box<dyn Keyword>, ValidationError<'a>> {
    match parent.get("format") {
        Some(Value::String(f)) if f == DARKMATTER_FILE_FORMAT => {}
        _ => {
            return Err(ValidationError::schema(format!(
                "x-darkmatter-match requires `{DARKMATTER_FILE_FORMAT}` to also be set as `format` on the same fragment"
            )));
        }
    }

    let arr = schema.as_array().ok_or_else(|| {
        ValidationError::schema("x-darkmatter-match must be an array of glob strings")
    })?;
    if arr.is_empty() {
        return Err(ValidationError::schema(
            "x-darkmatter-match must contain at least one glob",
        ));
    }

    let mut positive = GlobSetBuilder::new();
    let mut negative = GlobSetBuilder::new();
    let mut has_positive = false;

    for (idx, item) in arr.iter().enumerate() {
        let raw = item.as_str().ok_or_else(|| {
            ValidationError::schema(format!("x-darkmatter-match[{idx}] must be a string"))
        })?;
        let (target, has_pos) = if let Some(stripped) = raw.strip_prefix('!') {
            (stripped, false)
        } else {
            (raw, true)
        };
        let glob = Glob::new(target).map_err(|err| {
            ValidationError::schema(format!(
                "x-darkmatter-match[{idx}] `{raw}` is not a valid glob: {err}"
            ))
        })?;
        if has_pos {
            positive.add(glob);
            has_positive = true;
        } else {
            negative.add(glob);
        }
    }

    let positive = if has_positive {
        Some(positive.build().map_err(|err| {
            ValidationError::schema(format!("could not build positive glob set: {err}"))
        })?)
    } else {
        None
    };
    let negative = negative.build().map_err(|err| {
        ValidationError::schema(format!("could not build negative glob set: {err}"))
    })?;

    Ok(Box::new(DarkmatterMatchKeyword { positive, negative }))
}

struct DarkmatterMatchKeyword {
    /// `None` when the constraint contains only negative globs — any path is
    /// considered to "match" so long as no negative pattern rejects it.
    positive: Option<GlobSet>,
    negative: GlobSet,
}

impl DarkmatterMatchKeyword {
    fn check(&self, value: &str) -> bool {
        // Resolve via FileReference to get the path used by the glob test.
        let Ok(Some(path)) = FileReference::new(value).and_then(|r| r.resolve()) else {
            // Could not parse or resolve the reference — leave the failure
            // report to the `darkmatter-file` format validator and treat the
            // glob constraint as satisfied here.
            return true;
        };
        if !path.exists() {
            // File is missing — same rationale. Surfacing a glob failure on
            // top of the format validator's "no existing file matched"
            // diagnostic would be misleading and duplicated.
            return true;
        }

        // Match against several views of the path so patterns like `*.md`
        // (filename-only) and `src/**/*.rs` (relative path) both work even
        // when the resolver returns an absolute path.
        let candidates = match_candidates(&path, value);
        if candidates.iter().any(|c| self.negative.is_match(c)) {
            return false;
        }
        match &self.positive {
            Some(positive) => candidates.iter().any(|c| positive.is_match(c)),
            None => true,
        }
    }
}

fn match_candidates(resolved: &std::path::Path, raw: &str) -> Vec<String> {
    let mut out = Vec::with_capacity(4);
    out.push(resolved.to_string_lossy().into_owned());
    if let Some(name) = resolved.file_name().and_then(|n| n.to_str()) {
        out.push(name.to_string());
    }
    // The raw input is useful for filename-only patterns like `*.md` and
    // relative paths like `src/**/*.rs` that callers explicitly wrote.
    out.push(raw.to_string());
    if let Some(stripped) = raw.strip_prefix("./") {
        out.push(stripped.to_string());
    }
    out
}

impl Keyword for DarkmatterMatchKeyword {
    fn validate<'i>(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        match instance {
            Value::String(s) if self.check(s) => Ok(()),
            Value::String(s) => Err(ValidationError::custom(format!(
                "`{s}` does not match the configured file globs"
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
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_format_accepts_existing_file() {
        let dir = temp_dir();
        let path = dir.path().join("README.md");
        std::fs::write(&path, b"x").unwrap();
        let _cwd = CwdGuard::enter(dir.path());
        assert!(validate_file_reference("./README.md"));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_format_rejects_missing_file() {
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        assert!(!validate_file_reference("./does-not-exist.md"));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn resolve_file_reference_returns_path_for_existing_file() {
        let dir = temp_dir();
        let path = dir.path().join("README.md");
        std::fs::write(&path, b"x").unwrap();
        let _cwd = CwdGuard::enter(dir.path());
        let resolved = resolve_file_reference("./README.md").expect("should resolve");
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
        let err = resolve_file_reference("./does-not-exist.md")
            .expect_err("should fail with NoMatch");
        let rendered = err.to_string();
        assert!(
            rendered.contains("no existing file matched reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains("`./does-not-exist.md`"), "rendered: {rendered}");
        assert!(rendered.contains("while resolving from"), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::NoMatch { cwd: Some(_), .. }));
    }

    #[test]
    fn resolve_file_reference_reports_invalid_syntax() {
        // Empty input is rejected at parse time.
        let err = resolve_file_reference("").expect_err("should fail with InvalidSyntax");
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
        let err = resolve_file_reference(&raw).expect_err("should fail with Resolution");
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
        let err = resolve_file_reference("vault:notes/today.md")
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
        let err = resolve_file_reference(raw).expect_err("should fail with NoMatch");
        let rendered = err.to_string();
        assert!(
            rendered.contains("no existing file matched reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains(&format!("`{raw}`")), "rendered: {rendered}");
        // The contract does not fabricate a candidate absolute path; it only
        // reports the raw reference and the resolution directory.
        assert!(!rendered.contains("/darkmatter-test-missing-absolute-xyz.md "), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::NoMatch { cwd: Some(_), .. }));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn resolve_file_reference_no_match_for_missing_magic_path() {
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        let raw = "@darkmatter-test-missing-magic-xyz.md";
        let err = resolve_file_reference(raw).expect_err("should fail with NoMatch");
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
        let err = resolve_file_reference(raw).expect_err("should fail with NoMatch");
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
        let err = resolve_file_reference(raw).expect_err("should fail with NoMatch");
        let rendered = err.to_string();
        assert!(
            rendered.contains("no existing file matched reference"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains(&format!("`{raw}`")), "rendered: {rendered}");
        assert!(!rendered.contains("darkmatter-test-missing-recursive-xyz.md "), "rendered: {rendered}");
        assert!(matches!(err, FileReferenceFailure::NoMatch { cwd: Some(_), .. }));
    }

    #[test]
    fn file_reference_failure_no_match_omits_cwd_when_unknown() {
        let cwd: Option<PathBuf> = None;
        let failure = FileReferenceFailure::NoMatch {
            raw: "./x".to_string(),
            cwd,
        };
        let rendered = failure.to_string();
        assert_eq!(
            rendered,
            "no existing file matched reference `./x`",
        );
    }

    fn make_match_keyword(globs: Value) -> Box<dyn Keyword> {
        let mut parent = Map::new();
        parent.insert(
            "format".into(),
            Value::String(DARKMATTER_FILE_FORMAT.into()),
        );
        match_keyword_factory(&parent, &globs, Location::default()).expect("factory should accept")
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn match_keyword_positive_only_accepts_match() {
        let dir = temp_dir();
        let path = dir.path().join("notes.md");
        std::fs::write(&path, b"x").unwrap();
        let _cwd = CwdGuard::enter(dir.path());
        let kw = make_match_keyword(json!(["*.md"]));
        assert!(kw.is_valid(&Value::String("./notes.md".into())));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn match_keyword_respects_negative_globs() {
        let dir = temp_dir();
        let bad = dir.path().join("_draft.md");
        std::fs::write(&bad, b"x").unwrap();
        let _cwd = CwdGuard::enter(dir.path());
        let kw = make_match_keyword(json!(["*.md", "!_*.md"]));
        assert!(!kw.is_valid(&Value::String("./_draft.md".into())));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn match_keyword_negative_only_accepts_unless_excluded() {
        let dir = temp_dir();
        let good = dir.path().join("ok.md");
        std::fs::write(&good, b"x").unwrap();
        let _cwd = CwdGuard::enter(dir.path());
        let kw = make_match_keyword(json!(["!_*.md"]));
        assert!(kw.is_valid(&Value::String("./ok.md".into())));
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn match_keyword_returns_valid_for_missing_file() {
        // The glob constraint is intentionally narrow (`*.md`) but the target
        // file does not exist. The format validator owns the "no existing
        // file matched" diagnostic; the glob constraint must not also fire
        // here, otherwise a user sees two errors for the same underlying
        // issue.
        let dir = temp_dir();
        let _cwd = CwdGuard::enter(dir.path());
        let kw = make_match_keyword(json!(["*.md"]));
        assert!(
            kw.is_valid(&Value::String("./does-not-exist.txt".into())),
            "missing files should be passed through to the format validator",
        );
    }

    #[test]
    fn match_keyword_returns_valid_when_value_cannot_resolve() {
        // Empty input is rejected at `FileReference::new` time. Same
        // delegation as the missing-file case: the glob constraint must not
        // produce its own (misleading) error on top of the format
        // validator's.
        let kw = make_match_keyword(json!(["*.md"]));
        assert!(
            kw.is_valid(&Value::String("".into())),
            "unparseable file references should not be double-reported",
        );
    }

    #[test]
    fn match_keyword_rejects_parent_without_format() {
        // Schema-build guard: `x-darkmatter-match` is only meaningful next
        // to `format: darkmatter-file`. Without the format, the format
        // validator never runs, so the glob check has nothing to filter
        // against and the build must fail loudly.
        let parent = Map::new();
        let globs = json!(["*.md"]);
        let err = match_keyword_factory(&parent, &globs, Location::default())
            .err()
            .expect("expected factory error");
        assert!(
            err.to_string().contains(DARKMATTER_FILE_FORMAT),
            "expected error to name the required format, got: {}",
            err,
        );
    }

    #[test]
    fn match_keyword_rejects_parent_with_wrong_format() {
        // A different `format` (e.g. `uri`) signals a different contract
        // and is not interchangeable with `darkmatter-file`. The schema
        // build must reject this combination.
        let mut parent = Map::new();
        parent.insert("format".into(), Value::String("uri".into()));
        let globs = json!(["*.md"]);
        let err = match_keyword_factory(&parent, &globs, Location::default())
            .err()
            .expect("expected factory error");
        assert!(
            err.to_string().contains(DARKMATTER_FILE_FORMAT),
            "expected error to name the required format, got: {}",
            err,
        );
    }

    #[test]
    fn match_keyword_rejects_parent_with_non_string_format() {
        // `format: ["darkmatter-file"]` (an array) or `format: 1` (a number)
        // are not the contract this keyword expects. Build must fail rather
        // than silently accept.
        let mut parent = Map::new();
        parent.insert("format".into(), Value::Array(vec![]));
        assert!(
            match_keyword_factory(&parent, &json!(["*.md"]), Location::default()).is_err(),
            "non-string format must be rejected",
        );
    }

    #[test]
    fn match_keyword_rejects_empty_arr() {
        let mut parent = Map::new();
        parent.insert(
            "format".into(),
            Value::String(DARKMATTER_FILE_FORMAT.into()),
        );
        let empty = json!([]);
        let err = match_keyword_factory(&parent, &empty, Location::default())
            .err()
            .expect("expected factory error");
        assert!(err.to_string().contains("at least one glob"));
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
