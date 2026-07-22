//! Trust boundary for provider-supplied web links.
//!
//! A provider response body is attacker-adjacent data: the repository owner
//! controls `html_url`/`web_url`, and Darkmatter splices whatever arrives into
//! a Markdown link destination. Every link a focused record publishes passes
//! through here first, so a consumer can rely on it being an absolute `http(s)`
//! URL, carrying no credentials, on the same site as the repository the query
//! resolved to.
//!
//! Route shape is deliberately *not* checked. That is
//! [`super::provider_url`]'s job for inbound author-supplied references; here
//! the question is only whether the destination is safe to publish, and a
//! provider is entitled to link its own site however it likes.

use url::Url;

/// Normalizes a provider-supplied link, or drops it when it is not a canonical
/// web link for `repository_host`.
///
/// ## Returns
///
/// The serialized URL — WHATWG-normalized, so tabs and newlines are stripped
/// and spaces and control characters are percent-encoded — or `None` when the
/// value fails any check.
///
/// ## Notes
///
/// Dropping rather than erroring is deliberate. The projection contract
/// includes the web link "when available", so a link-less record is an
/// already-specified output shape; one unusable URL in a hundred-item page
/// must not abort an otherwise valid authoring run over a decorative field.
///
/// The host must match exactly, ASCII case-insensitively and ignoring a leading
/// `www.`. No subdomain or suffix relation is accepted — `github.com.evil.test`
/// and `evil.test` are precisely the shapes this rejection exists for. A remote
/// with no host of its own has nothing to compare against and therefore
/// publishes no links at all.
///
/// Credentials are refused outright: a canonical provider link never carries
/// them, and `https://github.com@evil.test/` reads as one site while resolving
/// to another.
pub(crate) fn trusted_web_link(raw: Option<String>, repository_host: &str) -> Option<String> {
    let raw = raw?;
    let url = Url::parse(raw.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    let host = url.host_str()?;
    same_site(host, repository_host).then(|| url.to_string())
}

fn same_site(left: &str, right: &str) -> bool {
    !right.trim().is_empty() && canonical_site(left) == canonical_site(right)
}

fn canonical_site(host: &str) -> String {
    let lowered = host.trim().to_ascii_lowercase();
    lowered.strip_prefix("www.").unwrap_or(&lowered).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(raw: &str, host: &str) -> Option<String> {
        trusted_web_link(Some(raw.to_string()), host)
    }

    #[test]
    fn same_site_http_links_survive_normalized() {
        assert_eq!(
            link("https://git.example/acme/project/-/jobs/7", "git.example").as_deref(),
            Some("https://git.example/acme/project/-/jobs/7")
        );
        assert_eq!(
            link("http://git.example:8443/pr/7", "GIT.Example").as_deref(),
            Some("http://git.example:8443/pr/7"),
            "the host comparison is case-insensitive and port-independent"
        );
        assert_eq!(
            link("https://www.github.com/acme/p/pull/1", "github.com").as_deref(),
            Some("https://www.github.com/acme/p/pull/1"),
            "providers serve the same site with and without the www label"
        );
    }

    /// Whitespace and control characters cannot survive into a destination: the
    /// WHATWG parser strips tabs and newlines and percent-encodes the rest.
    #[test]
    fn whitespace_and_control_characters_are_normalized_away() {
        let normalized = link("https://git.example/pr/a b\tc\nd\u{1}e", "git.example").unwrap();
        assert_eq!(normalized, "https://git.example/pr/a%20bcd%01e");
        assert!(!normalized.chars().any(char::is_whitespace));
    }

    #[test]
    fn non_http_schemes_are_dropped() {
        for raw in [
            "javascript:alert(1)",
            "data:text/html;base64,PHNjcmlwdD4=",
            "file:///etc/passwd",
            "vbscript:msgbox(1)",
            "ftp://git.example/pr/7",
            "/acme/project/pull/7",
            "not a url at all",
        ] {
            assert_eq!(link(raw, "git.example"), None, "accepted {raw}");
        }
    }

    #[test]
    fn cross_site_and_look_alike_hosts_are_dropped() {
        for raw in [
            "https://evil.example/acme/project/pull/7",
            "https://git.example.evil.test/pr/7",
            "https://notgit.example/pr/7",
            "https://evil.test/git.example/pr/7",
            "https://sub.git.example/pr/7",
        ] {
            assert_eq!(link(raw, "git.example"), None, "accepted {raw}");
        }
    }

    /// The rendered host and the resolved host disagree, which is the whole
    /// point of the shape — so it never reaches a destination.
    #[test]
    fn credentialed_urls_are_dropped() {
        assert_eq!(link("https://git.example@evil.test/pr/7", "git.example"), None);
        assert_eq!(link("https://user:token@git.example/pr/7", "git.example"), None);
    }

    #[test]
    fn a_remote_without_a_host_publishes_no_links() {
        assert_eq!(link("https://git.example/pr/7", ""), None);
        assert_eq!(link("https://git.example/pr/7", "   "), None);
        assert_eq!(trusted_web_link(None, "git.example"), None);
    }
}
