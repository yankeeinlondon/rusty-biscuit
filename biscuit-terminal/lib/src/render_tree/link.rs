//! Terminal-time resolution of authored file destinations on links.
//!
//! A link built by the Prose grammar keeps its authored destination
//! (`plan.md`, `./notes.md`) in the render tree, so Markdown and HTML output is
//! portable. Only the terminal target needs a usable `file://` URL for OSC 8,
//! so such a link is marked with [`file_reference_link`] and the terminal
//! renderer resolves it through [`terminal_link_url`].
//!
//! Links without the mark (darkmatter documents, components that build their
//! own absolute URLs) are emitted exactly as authored: their producers own
//! the base directory, which the terminal renderer cannot know.

use std::borrow::Cow;

use biscuit_file::{FileReference, FileReferenceKind};
use renderable::output::link_metadata::LinkMetadata;
use renderable::tree::{HintNamespace, RenderNode};

const LINK_HINTS: HintNamespace = HintNamespace("biscuit_terminal.link");

/// Hint key marking a link whose destination is an authored file reference,
/// resolved against the process working directory when rendered to a
/// terminal.
const FILE_REFERENCE: &str = "file_reference";

/// Builds a link whose `destination` is kept as authored on every target and
/// resolved to a `file://` URL only by the terminal renderer.
pub(crate) fn file_reference_link(destination: String, children: Vec<RenderNode>) -> RenderNode {
    let mut link = RenderNode::link(destination, None, children);
    link.attrs
        .set_hint(LINK_HINTS, FILE_REFERENCE, serde_json::Value::Bool(true));
    link
}

/// The OSC 8 destination for a link node's `url`.
///
/// A link marked by [`file_reference_link`] whose destination names a local
/// file becomes a `file://` URL; every other destination is returned as is.
pub(super) fn terminal_link_url<'a>(link: &RenderNode, url: &'a str) -> Cow<'a, str> {
    // Most links carry no extension data at all; skipping the bag lookup for
    // them keeps the styled terminal path free of hint round-trips.
    let marked = !link.attrs.data.is_empty()
        && link.attrs.get_hint(LINK_HINTS, FILE_REFERENCE) == Some(&serde_json::Value::Bool(true));
    match marked.then(|| file_url(url)).flatten() {
        Some(resolved) => Cow::Owned(resolved),
        None => Cow::Borrowed(url),
    }
}

/// The OSC 8 URI and parameter field for a link node whose authored
/// destination is `url`.
///
/// A link without [`link_metadata`](renderable::tree::NodeAttrs::link_metadata)
/// keeps today's destination ([`terminal_link_url`], fragment included) and
/// an empty parameter field. A link with metadata gets the bare URI (query
/// and fragment removed) and its [`osc8_parameters`].
pub(super) fn osc8_target(link: &RenderNode, url: &str) -> (String, String) {
    let Some(metadata) = link.attrs.link_metadata() else {
        return (terminal_link_url(link, url).into_owned(), String::new());
    };
    let (bare, params) = osc8_parameters(url, metadata);
    (terminal_link_url(link, bare).into_owned(), params)
}

/// The destination of a link carrying `metadata` without its query and
/// fragment, and the OSC 8 parameter field that replaces them.
///
/// The field holds colon-separated `key=value` pairs: `line`, `column`, and
/// `heading` first, in that order, then every other key sorted. `heading` is
/// the destination's fragment unless the metadata names one. A pair whose key
/// or value contains `:`, `;`, or a control character (or whose key is empty
/// or contains `=`) is omitted, because the field cannot carry it.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::render_tree::osc8_parameters;
/// use renderable::output::link_metadata::{LinkMetadata, QueryParams};
///
/// let metadata = LinkMetadata { query: QueryParams::from_iter([("line", 14_u64)]) };
/// let (uri, params) = osc8_parameters("file:///a.md#setup", &metadata);
/// assert_eq!((uri, params.as_str()), ("file:///a.md", "line=14:heading=setup"));
/// ```
#[must_use]
pub fn osc8_parameters<'a>(url: &'a str, metadata: &LinkMetadata) -> (&'a str, String) {
    let (bare, fragment) = split_destination(url);

    let query = &metadata.query;
    let mut pairs: Vec<(String, String)> = Vec::with_capacity(query.len() + 1);
    for key in ["line", "column"] {
        if let Some(value) = query.get(key) {
            pairs.push((key.to_string(), value.to_transport()));
        }
    }
    match (query.get("heading"), fragment.filter(|f| !f.is_empty())) {
        (Some(value), _) => pairs.push(("heading".to_string(), value.to_transport())),
        (None, Some(fragment)) => pairs.push(("heading".to_string(), fragment.to_string())),
        (None, None) => {}
    }
    let mut rest: Vec<(String, String)> = query
        .iter()
        .filter(|(key, _)| !matches!(*key, "line" | "column" | "heading"))
        .map(|(key, value)| (key.to_string(), value.to_transport()))
        .collect();
    rest.sort();
    pairs.extend(rest);

    let params = pairs
        .into_iter()
        .filter(|(key, value)| {
            !key.is_empty() && !key.contains('=') && osc8_param_safe(key) && osc8_param_safe(value)
        })
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(":");
    (bare, params)
}

/// Whether `text` can sit in an OSC 8 parameter field.
fn osc8_param_safe(text: &str) -> bool {
    !text.chars().any(|ch| ch == ':' || ch == ';' || ch.is_control())
}

/// Splits a destination into its bare part (no query, no fragment) and its
/// fragment.
fn split_destination(url: &str) -> (&str, Option<&str>) {
    let (before_fragment, fragment) = match url.split_once('#') {
        Some((before, fragment)) => (before, Some(fragment)),
        None => (url, None),
    };
    let bare = before_fragment
        .split_once('?')
        .map_or(before_fragment, |(bare, _)| bare);
    (bare, fragment)
}

/// Resolves an authored destination to a `file://` URL with
/// [`FileReference`]'s grammar: `./x` against the working directory, a bare
/// `x` against the working directory then the repository root, and `~`, `@`,
/// `&`, `^`, and `vault:` by their own rules.
///
/// A relative or absolute path that matches no existing file still links to
/// where it would be, joined onto the working directory. `None` for a URL
/// with a scheme (`https:`, `mailto:`, `file:`), a fragment (`#x`), and a
/// sigil reference that matches nothing.
fn file_url(destination: &str) -> Option<String> {
    if destination.is_empty() || destination.starts_with('#') {
        return None;
    }
    let reference = FileReference::new(destination).ok()?;
    let kind = reference.class().kind;
    if kind == FileReferenceKind::Url || (kind != FileReferenceKind::Vault && has_url_scheme(destination)) {
        return None;
    }
    let path = match reference.resolve() {
        Ok(Some(path)) => path,
        _ if matches!(
            kind,
            FileReferenceKind::ExplicitRelative
                | FileReferenceKind::ImplicitRelative
                | FileReferenceKind::Absolute
        ) =>
        {
            std::path::absolute(destination).ok()?
        }
        _ => return None,
    };
    url::Url::from_file_path(path).ok().map(String::from)
}

/// Whether `destination` starts with a URL scheme. A single letter is a
/// Windows drive (`C:/x`), not a scheme.
fn has_url_scheme(destination: &str) -> bool {
    url::Url::parse(destination).is_ok_and(|parsed| parsed.scheme().len() > 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_of(url: &str) -> std::path::PathBuf {
        url::Url::parse(url).unwrap().to_file_path().unwrap()
    }

    #[test]
    fn urls_fragments_and_empty_destinations_are_not_files() {
        for destination in [
            "https://example.com",
            "http://example.com",
            "mailto:test@example.com",
            "file:///path/to/file",
            "#section",
            "",
        ] {
            assert_eq!(file_url(destination), None, "{destination}");
        }
    }

    #[test]
    fn existing_relative_files_resolve_against_the_working_directory() {
        let cwd = std::env::current_dir().unwrap();
        for destination in ["Cargo.toml", "./Cargo.toml", "src/lib.rs"] {
            let url = file_url(destination).unwrap();
            assert_eq!(
                biscuit_file::canonicalize_simplified(&path_of(&url)).unwrap(),
                biscuit_file::canonicalize_simplified(&cwd.join(destination)).unwrap(),
                "{destination}"
            );
        }
    }

    #[test]
    fn missing_paths_still_link_to_where_they_would_be() {
        let cwd = std::env::current_dir().unwrap();
        for destination in ["./missing-file.txt", "missing/file.txt"] {
            let url = file_url(destination).unwrap();
            assert_eq!(path_of(&url), cwd.join(destination), "{destination}");
        }
        let absolute = std::path::absolute("/usr/local/bin/test").unwrap();
        assert_eq!(path_of(&file_url("/usr/local/bin/test").unwrap()), absolute);
    }

    fn metadata_link(url: &str, pairs: &[(&str, renderable::output::link_metadata::QueryValue)]) -> RenderNode {
        use renderable::output::link_metadata::{LinkMetadata, QueryParams};
        let mut link = RenderNode::link(url, None, vec![RenderNode::text("x")]);
        link.attrs.set_link_metadata(LinkMetadata {
            query: pairs.iter().cloned().collect::<QueryParams>(),
        });
        link
    }

    #[test]
    fn a_link_without_metadata_keeps_its_destination_and_no_params() {
        let link = RenderNode::link("https://e.com/a?b=1#c", None, vec![RenderNode::text("x")]);
        assert_eq!(
            osc8_target(&link, "https://e.com/a?b=1#c"),
            ("https://e.com/a?b=1#c".to_string(), String::new())
        );
    }

    #[test]
    fn metadata_params_are_ordered_and_the_uri_is_bare() {
        let link = metadata_link(
            "https://e.com/doc.md?x=1#setup",
            &[
                ("zeta", "z".into()),
                ("column", 7_u64.into()),
                ("alpha", true.into()),
                ("line", 14_u64.into()),
            ],
        );
        let (uri, params) = osc8_target(&link, "https://e.com/doc.md?x=1#setup");
        assert_eq!(uri, "https://e.com/doc.md");
        assert_eq!(params, "line=14:column=7:heading=setup:alpha=true:zeta=z");
    }

    #[test]
    fn a_metadata_heading_wins_over_the_fragment_and_unsafe_pairs_are_omitted() {
        let link = metadata_link(
            "https://e.com/#frag",
            &[
                ("heading", "intro".into()),
                ("bad", "a:b".into()),
                ("semi", "a;b".into()),
                ("ctl", "a\u{7}b".into()),
                ("ok", "v".into()),
            ],
        );
        let (uri, params) = osc8_target(&link, "https://e.com/#frag");
        assert_eq!(uri, "https://e.com/");
        assert_eq!(params, "heading=intro:ok=v");
    }

    #[test]
    fn only_marked_links_are_resolved() {
        let marked = file_reference_link("Cargo.toml".to_string(), vec![RenderNode::text("x")]);
        assert!(terminal_link_url(&marked, "Cargo.toml").starts_with("file://"));
        let plain = RenderNode::link("Cargo.toml", None, vec![RenderNode::text("x")]);
        assert_eq!(terminal_link_url(&plain, "Cargo.toml"), "Cargo.toml");
    }
}
