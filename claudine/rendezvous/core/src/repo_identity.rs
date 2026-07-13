//! Canonical repository identity.
//!
//! "Same repo" must mean the same thing everywhere — the scheduler's
//! has-the-repo test against the `repos/{node_id}` register and the
//! logging pipeline's repo attribution both key on this form. This is
//! the single canonicalization the host-capability spec's S4 calls for;
//! do not grow a second one elsewhere.
//!
//! The canonical form is `host[:port]/path`, derived from a git remote
//! URL: scheme, credentials, and a trailing `.git` are stripped, and
//! the host is lowercased (DNS is case-insensitive). Path case is
//! preserved — some hosts treat repository paths case-sensitively, so
//! collapsing case could merge genuinely different repos.

/// Canonicalize a git remote URL. Returns `None` for URLs that carry
/// no host identity (local paths, `file://`) — a remote-less checkout
/// has no mesh-wide identity and cannot appear in a repos register.
///
/// ## Examples
///
/// ```
/// use rendezvous_core::canonical_repo_id;
///
/// assert_eq!(
///     canonical_repo_id("git@github.com:acme/widget.git").as_deref(),
///     Some("github.com/acme/widget"),
/// );
/// assert_eq!(
///     canonical_repo_id("https://GitHub.com/acme/widget.git").as_deref(),
///     Some("github.com/acme/widget"),
/// );
/// assert_eq!(canonical_repo_id("/Users/ken/code/widget"), None);
/// ```
#[must_use]
pub fn canonical_repo_id(remote_url: &str) -> Option<String> {
    let url = remote_url.trim();
    if url.is_empty() {
        return None;
    }

    // Scheme-form URLs: ssh://, https://, http://, git://.
    if let Some((scheme, rest)) = url.split_once("://") {
        if !matches!(scheme, "ssh" | "https" | "http" | "git") {
            return None; // file://, etc. — no host identity
        }
        let rest = rest.rsplit_once('@').map_or(rest, |(_, r)| r);
        let (host_port, path) = rest.split_once('/')?;
        return assemble(host_port, path);
    }

    // scp-like SSH form: [user@]host:path (no scheme). Distinguish from
    // Windows drive paths (`C:\...`) by requiring a dot or user@ in the
    // host part and a non-numeric path start (a numeric segment after
    // `:` would be a port, which the scp form does not carry).
    if let Some((host_part, path)) = url.split_once(':') {
        if path.starts_with("//") || path.starts_with('\\') {
            return None;
        }
        let host = host_part.rsplit_once('@').map_or(host_part, |(_, h)| h);
        if host.contains('.') && !host.contains('/') && !path.is_empty() {
            return assemble(host, path);
        }
    }

    None
}

fn assemble(host_port: &str, path: &str) -> Option<String> {
    let path = path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .trim_end_matches('/');
    if host_port.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("{}/{path}", host_port.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::canonical_repo_id;

    #[test]
    fn canonical_forms() {
        let cases = [
            ("git@github.com:acme/widget.git", Some("github.com/acme/widget")),
            ("git@github.com:acme/widget", Some("github.com/acme/widget")),
            ("https://github.com/acme/widget.git", Some("github.com/acme/widget")),
            ("https://github.com/acme/widget/", Some("github.com/acme/widget")),
            ("http://github.com/acme/widget", Some("github.com/acme/widget")),
            ("ssh://git@github.com/acme/widget.git", Some("github.com/acme/widget")),
            ("git://github.com/acme/widget.git", Some("github.com/acme/widget")),
            // Host case folds; path case is preserved.
            ("https://GitHub.COM/Acme/Widget", Some("github.com/Acme/Widget")),
            // Self-hosted with port and nested groups.
            (
                "ssh://git@gitea.local:2222/team/sub/widget.git",
                Some("gitea.local:2222/team/sub/widget"),
            ),
            ("https://gitlab.com/group/sub/widget.git", Some("gitlab.com/group/sub/widget")),
            // Credentials stripped.
            ("https://ken@github.com/acme/widget.git", Some("github.com/acme/widget")),
            // No host identity.
            ("/Users/ken/code/widget", None),
            ("file:///Users/ken/code/widget", None),
            ("C:\\code\\widget", None),
            ("", None),
        ];
        for (input, expected) in cases {
            assert_eq!(
                canonical_repo_id(input).as_deref(),
                expected,
                "input: {input}"
            );
        }
    }
}
