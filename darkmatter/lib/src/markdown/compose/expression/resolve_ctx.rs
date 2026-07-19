//! Resolution context for filesystem- and repository-aware expression functions.
//!
//! Read-only: these helpers resolve and read paths; they never mutate.

use biscuit_file::{FetchPolicy, FileReference, FileReferenceError, PathPosition};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use super::ExpressionError;
use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;

type ProviderQueryResult = Result<Value, String>;
type ProviderQuerySlot = Arc<OnceLock<ProviderQueryResult>>;

/// The document-relative resolution environment passed to filesystem
/// expression functions (`absolute`, `relative`, `frontmatter`, …).
///
/// When a remote-fetch runtime is attached, document-reading functions
/// (`frontmatter`, `markdown_title`, `markdown_body_empty`, `file_exists`)
/// also accept HTTP(S) URL arguments and read their content from the run's
/// remote-fetch cache rather than the filesystem.
#[derive(Clone, Debug, Default)]
pub struct ResolutionContext {
    /// Directory the current document lives in; relative/`@` refs resolve here.
    pub base_dir: PathBuf,
    /// Magic (`@`) search paths, mirroring the compose link-resolution config.
    pub magic_paths: Vec<(PathBuf, PathPosition)>,
    /// Explicit fallback anchor for caller-supplied file references that are
    /// not authored inside the document (e.g. a CLI-supplied path relative to
    /// the launch area). Resolution tries `base_dir` (document dir) first;
    /// only when that misses does it consult this directory. `None` disables
    /// the fallback, preserving the legacy document-only behavior for small
    /// unit tests. Production constructors thread the captured launch area
    /// here so resolution is independent of the mutated ambient process CWD.
    pub file_ref_fallback_dir: Option<PathBuf>,
    /// Run-local remote-fetch runtime for URL-typed arguments. `None` disables
    /// remote reads in expression functions.
    pub(crate) remote_fetch: Option<RemoteFetchRuntime>,
    /// Run-local memoization and single-flight slots for normalized provider calls.
    pub(crate) provider_queries: Arc<Mutex<HashMap<String, ProviderQuerySlot>>>,
    /// Captured context values (e.g. `ctx.agent`) available to read-side
    /// functions. Populated by production surfaces; tests can inject values
    /// directly via [`Self::with_ctx_value`].
    pub(crate) ctx_values: Map<String, Value>,
    /// Injectable home directory for skill-root discovery. When `None`,
    /// skill lookups fall back to `dirs::home_dir()`.
    pub(crate) home_dir: Option<PathBuf>,
}

impl ResolutionContext {
    /// Creates a context rooted at `base_dir` with no magic search paths and
    /// no remote-fetch support.
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            magic_paths: Vec::new(),
            file_ref_fallback_dir: None,
            remote_fetch: None,
            provider_queries: Arc::new(Mutex::new(HashMap::new())),
            ctx_values: Map::new(),
            home_dir: None,
        }
    }

    /// Sets the explicit fallback directory for caller-supplied file
    /// references (typically the captured launch area). Resolution still
    /// tries `base_dir` (the document directory) first.
    #[must_use]
    pub fn with_file_ref_fallback_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.file_ref_fallback_dir = Some(dir.into());
        self
    }

    /// Returns the launch directory when available, otherwise the document base directory.
    pub(crate) fn caller_dir(&self) -> &Path {
        self.file_ref_fallback_dir
            .as_deref()
            .unwrap_or(&self.base_dir)
    }

    /// Sets a captured context value (e.g. `agent`) for read-side functions.
    #[must_use]
    pub fn with_ctx_value(mut self, key: &str, value: Value) -> Self {
        self.ctx_values.insert(key.to_string(), value);
        self
    }

    /// Injects a home directory for hermetic skill-root tests.
    #[must_use]
    #[cfg(test)]
    pub fn with_home_dir(mut self, home_dir: PathBuf) -> Self {
        self.home_dir = Some(home_dir);
        self
    }

    /// Returns the captured value for a context key, if present.
    pub(crate) fn ctx_value(&self, key: &str) -> Option<&Value> {
        self.ctx_values.get(key)
    }

    /// Returns the run's shared exact-host policy, or deny-all when remote reads are disabled.
    pub(crate) fn remote_policy(&self) -> FetchPolicy {
        self.remote_fetch
            .as_ref()
            .map(RemoteFetchRuntime::policy)
            .unwrap_or_else(FetchPolicy::deny_all)
    }

    /// Runs one normalized provider query once per compose context.
    pub(crate) fn cached_provider_query(
        &self,
        function: &'static str,
        key: String,
        query: impl FnOnce() -> Result<Value, ExpressionError>,
    ) -> Result<Value, ExpressionError> {
        if let Some(remote_fetch) = &self.remote_fetch {
            return remote_fetch
                .cached_provider_query(key, || query().map_err(|error| error.to_string()))
                .map_err(|message| Self::cached_query_error(function, message));
        }
        let slot = {
            let mut queries = self.provider_queries.lock().map_err(|_| ExpressionError::Other {
                function: function.to_string(),
                message: "provider query cache lock was poisoned".to_string(),
            })?;
            queries.entry(key).or_default().clone()
        };
        slot.get_or_init(|| query().map_err(|error| error.to_string()))
            .clone()
            .map_err(|message| Self::cached_query_error(function, message))
    }

    /// Rebuilds a provider-query failure from its memoized text.
    ///
    /// The cache stores failures as `String` so a slot stays cloneable, which
    /// means the function prefix an inner `ExpressionError::Other` already
    /// rendered is baked into that text. Re-wrapping it unconditionally emits
    /// `pr(): pr(): …`, so an already-prefixed message is adopted as-is.
    fn cached_query_error(function: &str, message: String) -> ExpressionError {
        let prefix = format!("{function}(): ");
        ExpressionError::Other {
            function: function.to_string(),
            message: message.strip_prefix(&prefix).map(str::to_string).unwrap_or(message),
        }
    }

    /// Returns the executing agent name.
    ///
    /// Uses the captured `ctx.agent` value when available; otherwise reads
    /// `AGENT` from the environment with the same trim-and-default rules.
    pub(crate) fn agent(&self) -> String {
        if let Some(Value::String(s)) = self.ctx_value("agent") {
            return s.clone();
        }
        std::env::var("AGENT")
            .ok()
            .map(|s| s.trim_ascii().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Returns the home directory to use for skill-root discovery.
    pub(crate) fn home_dir(&self) -> Option<PathBuf> {
        self.home_dir.clone().or_else(dirs::home_dir)
    }

    /// Fetches the text body for an HTTP(S) URL argument.
    ///
    /// The pre-compose discovery scanner only registers URLs written as literal
    /// arguments. When a function receives a URL produced at evaluation time
    /// (e.g. from a frontmatter or state variable), no slot exists yet, so this
    /// registers and fetches it on demand before blocking on the result. Host
    /// policy is still enforced — registration of a disallowed host fills the
    /// slot with a denial error.
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(body))` when the URL was fetched.
    /// - `Ok(None)` when no remote-fetch runtime is attached.
    /// - `Err` when the URL is malformed, denied by policy, or the fetch failed.
    pub(crate) fn fetch_remote_text(&self, raw: &str) -> Result<Option<String>, String> {
        let Some(rf) = self.remote_fetch.as_ref() else {
            return Ok(None);
        };
        let url = url::Url::parse(raw).map_err(|e| format!("invalid URL {raw:?}: {e}"))?;
        if let Ok(Some(body)) = rf.get_content(&url) {
            return Ok(Some(body));
        }
        // Not pre-registered (or a prior probe found no slot): register the
        // evaluated URL now and block on its fetch.
        rf.register_nested(url.clone());
        match rf.get_content(&url) {
            Ok(Some(body)) => Ok(Some(body)),
            Ok(None) => Err(format!("remote URL {raw:?} could not be fetched")),
            Err(e) => Err(e),
        }
    }
}

/// Returns `true` when the argument is an HTTP(S) URL (handled remotely rather
/// than as a filesystem path).
///
/// Scheme detection is case-insensitive per RFC 3986, so values like
/// `HTTPS://example.com/doc.md` are recognized as remote URLs.
pub(crate) fn is_remote_url(raw: &str) -> bool {
    url::Url::parse(raw)
        .ok()
        .filter(|u| {
            let scheme = u.scheme();
            scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
        })
        .is_some()
}

/// Normalizes a filepath argument: strips a leading `file://` scheme and
/// collapses doubled `/` separators (per the spec's normalization note).
pub fn normalize_path_arg(raw: &str) -> String {
    let stripped = raw.strip_prefix("file://").unwrap_or(raw);
    // Collapse repeated slashes; a leading "./" or "../" is preserved because
    // only consecutive separators are merged, not the dots that precede them.
    let mut out = String::with_capacity(stripped.len());
    let mut prev_slash = false;
    for ch in stripped.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push(ch);
            }
            prev_slash = true;
        } else {
            out.push(ch);
            prev_slash = false;
        }
    }
    out
}

/// Canonical caller-supplied file-reference resolver encoding the single
/// resolution order shared by the expression path and the schema validator.
///
/// Resolution order for local filesystem references:
///
/// 1. absolute paths are returned as-is by `FileReference`;
/// 2. document-relative via `file_ref.resolve_from(base_dir)`;
/// 3. launch-area fallback via `file_ref.resolve_from(fallback)` when present;
/// 4. **no ambient-CWD fallback** — callers that need it must pass an explicit
///    `fallback`.
///
/// The caller owns constructing `file_ref` (including any `@` magic-path
/// injection), so the two production surfaces — read-side expression functions
/// and the `darkmatter-file` schema format validator — share the order while
/// retaining their own preprocessing. This replaces the implicit ambient-CWD
/// `FileReference::resolve()` fallback so resolution no longer depends on the
/// mutated process working directory.
///
/// ## Returns
///
/// - `Ok(Some(path))` when the reference resolves to a path.
/// - `Ok(None)` when the reference is well-formed but resolves to nothing.
/// - `Err` when the reference requires state that cannot be determined.
pub(crate) fn resolve_file_ref_with_fallback(
    file_ref: &FileReference,
    base_dir: &Path,
    fallback: Option<&Path>,
) -> Result<Option<PathBuf>, FileReferenceError> {
    if let Some(path) = file_ref.resolve_from(base_dir)? {
        return Ok(Some(path));
    }
    if let Some(fallback) = fallback {
        return file_ref.resolve_from(fallback);
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn normalizes_file_scheme_and_double_slashes() {
        assert_eq!(normalize_path_arg("file://foo/bar"), "foo/bar");
        assert_eq!(normalize_path_arg("foo//bar"), "foo/bar");
        assert_eq!(normalize_path_arg("./a//b"), "./a/b");
    }

    #[test]
    fn is_remote_url_is_case_insensitive() {
        assert!(is_remote_url("http://example.com"));
        assert!(is_remote_url("https://example.com"));
        assert!(is_remote_url("HTTP://example.com"));
        assert!(is_remote_url("HTTPS://example.com"));
        assert!(is_remote_url("Http://example.com"));
        assert!(is_remote_url("hTtPs://example.com/doc.md"));
        assert!(is_remote_url("HTTPS://example.com/doc.md"));

        assert!(!is_remote_url("ftp://example.com"));
        assert!(!is_remote_url("C:/foo/bar"));
        assert!(!is_remote_url("foo/bar.md"));
        assert!(!is_remote_url("file://foo/bar"));
    }

    #[test]
    fn resolution_context_default_is_cwd_no_magic() {
        let ctx = ResolutionContext::new(PathBuf::from("/tmp/docdir"));
        assert_eq!(ctx.base_dir, PathBuf::from("/tmp/docdir"));
        assert!(ctx.magic_paths.is_empty());
        // `new(base_dir)` leaves the fallback unset so existing unit tests
        // keep the legacy document-only resolution behavior.
        assert!(ctx.file_ref_fallback_dir.is_none());
    }

    #[test]
    fn with_file_ref_fallback_dir_sets_the_field() {
        let ctx = ResolutionContext::new(PathBuf::from("/tmp/docdir"))
            .with_file_ref_fallback_dir("/tmp/launch");
        assert_eq!(ctx.file_ref_fallback_dir.as_deref(), Some(std::path::Path::new("/tmp/launch")));
    }

    #[test]
    fn caller_dir_prefers_launch_fallback_and_otherwise_uses_base_dir() {
        let document_only = ResolutionContext::new(PathBuf::from("/tmp/docdir"));
        assert_eq!(document_only.caller_dir(), Path::new("/tmp/docdir"));

        let launched_elsewhere = document_only.with_file_ref_fallback_dir("/tmp/launch");
        assert_eq!(launched_elsewhere.caller_dir(), Path::new("/tmp/launch"));
    }

    /// A same-named file present in BOTH the document dir and the launch-area
    /// fallback resolves to the document-dir copy — document-first contract
    /// (verification goal #9).
    #[test]
    fn document_relative_hit_wins_over_fallback_conflict() {
        let doc_dir = tempfile::TempDir::new().unwrap();
        let launch_dir = tempfile::TempDir::new().unwrap();
        std::fs::write(doc_dir.path().join("spec.md"), "# Document\n").unwrap();
        std::fs::write(launch_dir.path().join("spec.md"), "# Launch\n").unwrap();

        let file_ref = FileReference::new("spec.md").unwrap();
        let resolved = resolve_file_ref_with_fallback(
            &file_ref,
            doc_dir.path(),
            Some(launch_dir.path()),
        )
        .unwrap()
        .expect("should resolve");

        assert_eq!(resolved, doc_dir.path().join("spec.md"));
    }

    /// A path missing under `base_dir` but present under the launch-area
    /// fallback resolves via the fallback (verification goal #8 precursor).
    #[test]
    fn missing_under_base_dir_resolves_via_fallback() {
        let doc_dir = tempfile::TempDir::new().unwrap();
        let launch_dir = tempfile::TempDir::new().unwrap();
        std::fs::write(launch_dir.path().join("caller.md"), "# Caller\n").unwrap();

        let file_ref = FileReference::new("caller.md").unwrap();
        let resolved = resolve_file_ref_with_fallback(
            &file_ref,
            doc_dir.path(),
            Some(launch_dir.path()),
        )
        .unwrap()
        .expect("should resolve via fallback");

        assert_eq!(resolved, launch_dir.path().join("caller.md"));
    }

    /// With no fallback, a path missing under `base_dir` resolves to nothing
    /// (no ambient-CWD consultation) — preserves today's no-fallback behavior.
    #[test]
    fn missing_under_base_dir_without_fallback_resolves_to_none() {
        let doc_dir = tempfile::TempDir::new().unwrap();

        let file_ref = FileReference::new("absent.md").unwrap();
        let resolved = resolve_file_ref_with_fallback(&file_ref, doc_dir.path(), None).unwrap();

        assert!(resolved.is_none());
    }

    /// An evaluated (non-literal) URL the discovery scanner never saw must still
    /// fetch: `fetch_remote_text` registers it on demand at point of use. This
    /// covers `{{ markdown_title(remote_doc) }}` where `remote_doc` is a
    /// frontmatter/state string rather than a literal URL argument.
    #[tokio::test]
    async fn fetch_remote_text_registers_unseen_url_on_demand() {
        use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;
        use biscuit_file::file_reference::fetch::FetchPolicy;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/dynamic.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string("# Dynamic\n\nbody"))
            .mount(&server)
            .await;

        let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
        let rt = RemoteFetchRuntime::with_policy(policy);
        let url = format!("{}/dynamic.md", server.uri());

        let mut ctx = ResolutionContext::new(PathBuf::from("/tmp"));
        ctx.remote_fetch = Some(rt);

        // The URL was never registered by discovery; the read-side function
        // must register-and-fetch it now and return the body.
        let body = tokio::task::spawn_blocking(move || ctx.fetch_remote_text(&url))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(body.as_deref(), Some("# Dynamic\n\nbody"));
    }

    /// On-demand registration still honors the host allowlist: a disallowed
    /// evaluated URL fails with a policy denial rather than fetching.
    #[tokio::test]
    async fn fetch_remote_text_on_demand_enforces_policy() {
        use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;
        use biscuit_file::file_reference::fetch::FetchPolicy;

        // Deny-all policy: even a well-formed URL must be denied at registration.
        let rt = RemoteFetchRuntime::with_policy(FetchPolicy::deny_all());
        let mut ctx = ResolutionContext::new(PathBuf::from("/tmp"));
        ctx.remote_fetch = Some(rt);

        let result = tokio::task::spawn_blocking(move || {
            ctx.fetch_remote_text("https://blocked.example/doc.md")
        })
        .await
        .unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn provider_queries_are_run_local_single_flight_and_memoized() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let context = ResolutionContext::new(PathBuf::from("/tmp"));
        let executions = Arc::new(AtomicUsize::new(0));
        let mut workers = Vec::new();
        for _ in 0..2 {
            let context = context.clone();
            let executions = Arc::clone(&executions);
            workers.push(std::thread::spawn(move || {
                context
                    .cached_provider_query("pr", "pr:123".to_string(), || {
                        executions.fetch_add(1, Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        Ok(Value::String("result".to_string()))
                    })
                    .unwrap()
            }));
        }

        for worker in workers {
            assert_eq!(worker.join().unwrap(), Value::String("result".to_string()));
        }
        assert_eq!(executions.load(Ordering::SeqCst), 1);

        let second_context = ResolutionContext::new(PathBuf::from("/tmp"));
        second_context
            .cached_provider_query("pr", "pr:123".to_string(), || {
                executions.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            })
            .unwrap();
        assert_eq!(executions.load(Ordering::SeqCst), 2);
    }
}
