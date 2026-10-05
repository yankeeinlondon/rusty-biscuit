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

    #[test]
    fn only_marked_links_are_resolved() {
        let marked = file_reference_link("Cargo.toml".to_string(), vec![RenderNode::text("x")]);
        assert!(terminal_link_url(&marked, "Cargo.toml").starts_with("file://"));
        let plain = RenderNode::link("Cargo.toml", None, vec![RenderNode::text("x")]);
        assert_eq!(terminal_link_url(&plain, "Cargo.toml"), "Cargo.toml");
    }
}
