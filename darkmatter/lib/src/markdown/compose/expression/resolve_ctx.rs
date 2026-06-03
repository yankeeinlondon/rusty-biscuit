//! Resolution context for filesystem-aware expression functions.
//!
//! Read-only: these helpers resolve and read paths; they never mutate.

use biscuit_file::PathPosition;
use std::path::PathBuf;

use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;

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
    /// Run-local remote-fetch runtime for URL-typed arguments. `None` disables
    /// remote reads in expression functions.
    pub(crate) remote_fetch: Option<RemoteFetchRuntime>,
}

impl ResolutionContext {
    /// Creates a context rooted at `base_dir` with no magic search paths and
    /// no remote-fetch support.
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            magic_paths: Vec::new(),
            remote_fetch: None,
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
pub(crate) fn is_remote_url(raw: &str) -> bool {
    raw.starts_with("http://") || raw.starts_with("https://")
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
    fn resolution_context_default_is_cwd_no_magic() {
        let ctx = ResolutionContext::new(PathBuf::from("/tmp/docdir"));
        assert_eq!(ctx.base_dir, PathBuf::from("/tmp/docdir"));
        assert!(ctx.magic_paths.is_empty());
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
            remote_fetch: Some(rt),
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
            remote_fetch: Some(rt),
        };

        let result = tokio::task::spawn_blocking(move || {
            ctx.fetch_remote_text("https://blocked.example/doc.md")
        })
        .await
        .unwrap();
        assert!(result.is_err());
    }
}
