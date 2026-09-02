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

/// Caller-owned identity retained after a schema-selected file value is projected.
#[derive(Clone, Debug)]
pub(crate) struct CallerFileProvenance {
    pub property: String,
    pub reference: String,
    pub origin: biscuit_file::FileResolutionContext,
    pub candidate: PathBuf,
    pub candidate_provenance: biscuit_file::RootProvenance,
}

type ProviderQueryResult = Result<Value, ExpressionError>;
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
    /// Repository (worktree) root for the resolution pass, discovered once from
    /// the resolution base directory. Implicit references anchor repository-root
    /// first, then the document directory (D2). Threaded through
    /// [`document_resolution_context`] so per-reference resolution reuses this
    /// root rather than rediscovering it. `None` when the base is not inside a
    /// worktree, in which case resolution falls back to a per-call discovery
    /// from `base_dir`.
    ///
    /// [`document_resolution_context`]: crate::markdown::compose::util::document_resolution_context
    pub repository_root: Option<PathBuf>,
    /// Package-area root captured for package (`!`) references.
    pub package_area: Option<PathBuf>,
    /// The captured launch-area directory, retained for diagnostics only.
    ///
    /// Per D2, the launch directory is a base for **top-level** references only
    /// (owned by Claudine); it is **not** a fallback for references authored
    /// inside a nested document. Darkmatter's nested-document resolution is
    /// repository-first then source-relative and never consults this directory.
    /// It is carried here solely so the `fallback_dir` facet of a
    /// [`FileReferenceDiagnostic`](super::error::FileReferenceDiagnostic)
    /// can surface the configured launch area.
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
    /// Host-captured request snapshot. When present, all local references
    /// derive from it and never rediscover ambient process state.
    pub(crate) file_resolution_context: Option<biscuit_file::FileResolutionContext>,
    /// Projected property/array occurrences mapped back to their immutable caller records.
    pub(crate) caller_file_provenance: HashMap<String, CallerFileProvenance>,
    /// Caller record selected from the source expression for the active filesystem call.
    pub(crate) active_caller_file_provenance: Option<CallerFileProvenance>,
}

impl ResolutionContext {
    /// Creates a context rooted at `base_dir` with no magic search paths and
    /// no remote-fetch support.
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            magic_paths: Vec::new(),
            repository_root: None,
            package_area: None,
            file_ref_fallback_dir: None,
            remote_fetch: None,
            provider_queries: Arc::new(Mutex::new(HashMap::new())),
            ctx_values: Map::new(),
            home_dir: None,
            file_resolution_context: None,
            caller_file_provenance: HashMap::new(),
            active_caller_file_provenance: None,
        }
    }

    /// Sets the repository (worktree) root for the resolution pass.
    #[must_use]
    pub fn with_repository_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.repository_root = Some(root.into());
        self
    }

    /// Sets the package-area root for package (`!`) references.
    #[must_use]
    pub fn with_package_area(mut self, root: impl Into<PathBuf>) -> Self {
        self.package_area = Some(root.into());
        self
    }

    /// Records the captured launch-area directory for diagnostics.
    ///
    /// Per D2 the launch directory is **not** a resolution fallback for
    /// references authored inside a nested document; it is retained only so the
    /// `fallback_dir` diagnostic facet can surface the configured launch area.
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
    ///
    /// The cache slot stores the typed [`ExpressionError`] itself rather than
    /// its rendered text, so a focused provider classification
    /// ([`ExpressionError::Provider`]) survives memoization and the replayed
    /// failure is byte-identical to the original — re-wrapping rendered text
    /// was what flattened the classification and risked a doubled `fn():`
    /// prefix.
    pub(crate) fn cached_provider_query(
        &self,
        function: &'static str,
        key: String,
        query: impl FnOnce() -> Result<Value, ExpressionError>,
    ) -> Result<Value, ExpressionError> {
        if let Some(remote_fetch) = &self.remote_fetch {
            return remote_fetch.cached_provider_query(key, query);
        }
        let slot = {
            let mut queries = self.provider_queries.lock().map_err(|_| ExpressionError::Other {
                function: function.to_string(),
                message: "provider query cache lock was poisoned".to_string(),
            })?;
            queries.entry(key).or_default().clone()
        };
        slot.get_or_init(query).clone()
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
        if self.file_resolution_context.is_some() {
            self.home_dir.clone()
        } else {
            self.home_dir.clone().or_else(dirs::home_dir)
        }
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

/// Canonical document-backed file-reference resolver shared by the expression
/// path and the schema validator.
///
/// Resolution runs through an explicit [`FileResolutionContext`] built by
/// [`document_resolution_context`], so it reads no ambient process state (CWD,
/// `$HOME`, environment, or git root) after the context is captured. For a
/// local filesystem reference the candidate order is (D2/D3):
///
/// - **explicit** `./`/`../` → the document `base_dir` only, no fallback;
/// - **implicit** bare paths → the repository root first, then `base_dir`;
/// - `~`/`~/…` → the user's home directory only;
/// - `@`/`!`/`vault:`/`%`/absolute/URL → their existing `FileReference`
///   semantics against the context's configured roots.
///
/// The launch-area fallback the previous two-step resolver consulted is
/// **removed for nested documents** (D2): only repository and authoring-document
/// candidates participate. The caller supplies the magic search roots and the
/// pass's cached `repository_root`; both surfaces — read-side expression
/// functions and the `darkmatter-file` schema format validator — share this
/// single order.
///
/// ## Returns
///
/// - `Ok(Some(path))` when the reference resolves to a regular file.
/// - `Ok(None)` when the reference is well-formed but no candidate matched.
/// - `Err` when the context is invalid or a required anchor cannot be
///   established (missing home, missing interpolation variable, unconfigured
///   vault, or a candidate probe I/O failure).
///
/// [`document_resolution_context`]: crate::markdown::compose::util::document_resolution_context
pub(crate) fn resolve_document_file_ref(
    file_ref: &FileReference,
    base_dir: &Path,
    repository_root: Option<&Path>,
    package_area: Option<&Path>,
    magic_paths: &[(PathBuf, PathPosition)],
    request_context: Option<&biscuit_file::FileResolutionContext>,
) -> Result<Option<PathBuf>, FileReferenceError> {
    let ctx = match request_context {
        Some(snapshot) if snapshot.base_dir() == base_dir => snapshot.clone(),
        Some(snapshot) => snapshot.for_base(base_dir),
        None => crate::markdown::compose::util::document_resolution_context(
            base_dir,
            None,
            magic_paths,
            repository_root,
            package_area,
        ),
    };
    file_ref.resolve_in_context(&ctx)
}

/// Resolves a document-backed reference to an absolute path **shape**: the
/// matched file when one exists, or — after a miss — the FIRST candidate from
/// the shared [`FileReference::candidate_plan`].
///
/// Path-component expression functions (`basename`, `dirname`, `join`, the
/// file-index family) operate on references whose target need not exist. The
/// missing-target shape comes from the same repository-first candidate order
/// execution probes (D1/D3) — never a private prefix branch plus
/// `base_dir.join`. An implicit bare miss therefore yields the repository-root
/// candidate, identical to how an existing implicit reference resolves; a shape
/// and an existing file can never disagree on anchoring.
///
/// ## Returns
///
/// - `Ok(path)` — the matched file, or the first candidate's path shape.
///
/// ## Errors
///
/// Propagates the typed [`FileReferenceError`] for any non-`NoMatch` failure
/// (invalid context, a missing home/vault anchor, or a candidate probe I/O
/// failure) and when the reference has no local candidate at all (a remote URL,
/// which callers reject up front).
pub(crate) fn resolve_document_file_ref_shape(
    file_ref: &FileReference,
    base_dir: &Path,
    repository_root: Option<&Path>,
    package_area: Option<&Path>,
    magic_paths: &[(PathBuf, PathPosition)],
    request_context: Option<&biscuit_file::FileResolutionContext>,
) -> Result<PathBuf, FileReferenceError> {
    let ctx = match request_context {
        Some(snapshot) => snapshot.for_base(base_dir),
        None => crate::markdown::compose::util::document_resolution_context(
            base_dir,
            None,
            magic_paths,
            repository_root,
            package_area,
        ),
    };
    if let Some(path) = file_ref.resolve_in_context(&ctx)? {
        return Ok(path);
    }
    // Clean miss: the path shape is the first candidate the shared plan would
    // have probed (repository-first for an implicit bare path), taken from
    // `FileReference` itself rather than re-deriving the grammar from the raw
    // string.
    file_ref
        .candidate_plan(&ctx)?
        .into_iter()
        .next()
        .map(|candidate| candidate.path().to_path_buf())
        .ok_or_else(|| {
            FileReferenceError::InvalidSyntax(format!(
                "reference `{}` has no local filesystem candidate",
                file_ref.raw()
            ))
        })
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
        assert!(ctx.repository_root.is_none());
        // The launch-area anchor is diagnostic-only and unset by default.
        assert!(ctx.file_ref_fallback_dir.is_none());
    }

    #[test]
    fn with_file_ref_fallback_dir_sets_the_field() {
        let ctx = ResolutionContext::new(PathBuf::from("/tmp/docdir"))
            .with_file_ref_fallback_dir("/tmp/launch");
        assert_eq!(ctx.file_ref_fallback_dir.as_deref(), Some(std::path::Path::new("/tmp/launch")));
    }

    /// Creates a temp directory that looks like a git repository root by
    /// planting a `.git` marker, so `find_git_root_from` anchors implicit
    /// references on it independent of the host's real repo boundaries.
    fn repo_fixture() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        dir
    }

    /// Implicit (bare) references resolve **repository-root first**: a same-named
    /// file present in BOTH the repository root and the nested document
    /// directory resolves to the repository-root copy (D2/D3 repository-first).
    #[test]
    fn implicit_reference_prefers_repository_root_over_base() {
        let repo = repo_fixture();
        let base_dir = repo.path().join("prompts");
        std::fs::create_dir_all(&base_dir).unwrap();
        std::fs::write(repo.path().join("shared.md"), "# Repo\n").unwrap();
        std::fs::write(base_dir.join("shared.md"), "# Source\n").unwrap();

        let file_ref = FileReference::new("shared.md").unwrap();
        let resolved = resolve_document_file_ref(&file_ref, &base_dir, None, None, &[], None)
            .unwrap()
            .expect("should resolve");

        assert_eq!(resolved, repo.path().join("shared.md"));
    }

    /// Explicit `./` references pin to the document directory only and never
    /// fall back to the repository root (D2).
    #[test]
    fn explicit_reference_resolves_from_base_only() {
        let repo = repo_fixture();
        let base_dir = repo.path().join("prompts");
        std::fs::create_dir_all(&base_dir).unwrap();
        // Same-named file at the repo root must NOT win for an explicit ref.
        std::fs::write(repo.path().join("shared.md"), "# Repo\n").unwrap();
        std::fs::write(base_dir.join("shared.md"), "# Source\n").unwrap();

        let file_ref = FileReference::new("./shared.md").unwrap();
        let resolved = resolve_document_file_ref(&file_ref, &base_dir, None, None, &[], None)
            .unwrap()
            .expect("should resolve from base");

        assert_eq!(resolved, base_dir.join("shared.md"));
    }

    #[test]
    fn package_reference_prefers_package_area_over_repository_root() {
        let repo = repo_fixture();
        let package_area = repo.path().join("darkmatter");
        let base_dir = package_area.join("docs");
        std::fs::create_dir_all(&base_dir).unwrap();
        std::fs::write(repo.path().join("shared.md"), "repository decoy").unwrap();
        std::fs::write(package_area.join("shared.md"), "package").unwrap();

        let file_ref = FileReference::new("!shared.md").unwrap();
        let resolved = resolve_document_file_ref(
            &file_ref,
            &base_dir,
            Some(repo.path()),
            Some(&package_area),
            &[],
            None,
        )
        .unwrap();

        assert_eq!(resolved, Some(package_area.join("shared.md")));
    }

    /// A missing file resolves to nothing — there is no launch-area fallback for
    /// a nested-document reference and no ambient-CWD consultation (D2).
    #[test]
    fn missing_reference_resolves_to_none() {
        let repo = repo_fixture();
        let base_dir = repo.path().join("prompts");
        std::fs::create_dir_all(&base_dir).unwrap();

        let file_ref = FileReference::new("absent.md").unwrap();
        let resolved =
            resolve_document_file_ref(&file_ref, &base_dir, None, None, &[], None).unwrap();

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

        let ctx = ResolutionContext {
            base_dir: PathBuf::from("/tmp"),
            magic_paths: Vec::new(),
            repository_root: None,
            package_area: None,
            file_ref_fallback_dir: None,
            remote_fetch: Some(rt),
            provider_queries: Default::default(),
            ctx_values: Map::new(),
            home_dir: None,
            file_resolution_context: None,
            caller_file_provenance: HashMap::new(),
            active_caller_file_provenance: None,
        };

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
        let ctx = ResolutionContext {
            base_dir: PathBuf::from("/tmp"),
            magic_paths: Vec::new(),
            repository_root: None,
            package_area: None,
            file_ref_fallback_dir: None,
            remote_fetch: Some(rt),
            provider_queries: Default::default(),
            ctx_values: Map::new(),
            home_dir: None,
            file_resolution_context: None,
            caller_file_provenance: HashMap::new(),
            active_caller_file_provenance: None,
        };

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
