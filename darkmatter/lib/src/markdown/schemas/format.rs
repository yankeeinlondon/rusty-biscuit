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

use biscuit_file::FileReference;
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
    let Ok(reference) = FileReference::new(value) else {
        return false;
    };
    matches!(reference.resolve(), Ok(Some(path)) if path.exists())
}

/// Factory for the `x-darkmatter-match` keyword.
///
/// Reads the globs from the schema value (which must be a non-empty array of
/// strings) and compiles two `GlobSet`s — positive (no `!` prefix) and
/// negative (`!`-prefixed) — at validator-build time. Bad schema input
/// (non-array, non-string members, empty list, or an unparsable glob)
/// surfaces as `ValidationError::schema`.
pub fn match_keyword_factory<'a>(
    _parent: &'a Map<String, Value>,
    schema: &'a Value,
    _schema_path: Location,
) -> Result<Box<dyn Keyword>, ValidationError<'a>> {
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
            ValidationError::schema(format!(
                "x-darkmatter-match[{idx}] must be a string"
            ))
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
            ValidationError::schema(format!(
                "could not build positive glob set: {err}"
            ))
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
        let path = match FileReference::new(value).and_then(|r| r.resolve()) {
            Ok(Some(p)) => p,
            _ => return false,
        };

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
            ValidationError::schema(format!(
                "x-darkmatter-url-scheme[{idx}] must be a string"
            ))
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
    use std::sync::Mutex;

    // Tests in this module mutate the process CWD; serialise them so they
    // do not race with one another.
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("create temp dir")
    }

    #[test]
    fn file_format_accepts_existing_file() {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = temp_dir();
        let path = dir.path().join("README.md");
        std::fs::write(&path, b"x").unwrap();
        let prior = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        assert!(validate_file_reference("./README.md"));
        std::env::set_current_dir(prior).unwrap();
    }

    #[test]
    fn file_format_rejects_missing_file() {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = temp_dir();
        let prior = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        assert!(!validate_file_reference("./does-not-exist.md"));
        std::env::set_current_dir(prior).unwrap();
    }

    fn make_match_keyword(globs: Value) -> Box<dyn Keyword> {
        let parent = Map::new();
        match_keyword_factory(&parent, &globs, Location::default())
            .expect("factory should accept")
    }

    #[test]
    fn match_keyword_positive_only_accepts_match() {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = temp_dir();
        let path = dir.path().join("notes.md");
        std::fs::write(&path, b"x").unwrap();
        let prior = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let kw = make_match_keyword(json!(["*.md"]));
        assert!(kw.is_valid(&Value::String("./notes.md".into())));
        std::env::set_current_dir(prior).unwrap();
    }

    #[test]
    fn match_keyword_respects_negative_globs() {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = temp_dir();
        let bad = dir.path().join("_draft.md");
        std::fs::write(&bad, b"x").unwrap();
        let prior = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let kw = make_match_keyword(json!(["*.md", "!_*.md"]));
        assert!(!kw.is_valid(&Value::String("./_draft.md".into())));
        std::env::set_current_dir(prior).unwrap();
    }

    #[test]
    fn match_keyword_negative_only_accepts_unless_excluded() {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = temp_dir();
        let good = dir.path().join("ok.md");
        std::fs::write(&good, b"x").unwrap();
        let prior = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let kw = make_match_keyword(json!(["!_*.md"]));
        assert!(kw.is_valid(&Value::String("./ok.md".into())));
        std::env::set_current_dir(prior).unwrap();
    }

    #[test]
    fn match_keyword_rejects_empty_arr() {
        let parent = Map::new();
        let empty = json!([]);
        let err = match_keyword_factory(&parent, &empty, Location::default())
            .err()
            .expect("expected factory error");
        assert!(err.to_string().contains("at least one glob"));
    }

    #[test]
    fn url_scheme_keyword_accepts_match() {
        let parent = Map::new();
        let kw = url_scheme_keyword_factory(&parent, &json!(["https", "http"]), Location::default())
            .unwrap();
        assert!(kw.is_valid(&Value::String("https://example.com".into())));
        assert!(kw.is_valid(&Value::String("HTTP://example.com".into())));
        assert!(!kw.is_valid(&Value::String("ftp://example.com".into())));
    }

    #[test]
    fn url_scheme_keyword_rejects_non_url() {
        let parent = Map::new();
        let kw = url_scheme_keyword_factory(&parent, &json!(["https"]), Location::default())
            .unwrap();
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
