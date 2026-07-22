//! Rewrites catalog-authored documentation links into targets an editor can
//! actually resolve.
//!
//! The expression-function catalog authors its query-vocabulary reference
//! sibling-relative (`darkmatter-expressions.md#…`). That spelling is correct
//! where it is generated — inside the topic doc's own function table — but a
//! hover or completion popup is not a document: the editor resolves a relative
//! Markdown destination against the *active* file, which may sit anywhere in
//! the workspace. The same bytes therefore name a different (usually
//! nonexistent) file on every surface that shows them.
//!
//! This module is the response-boundary rewrite. [`resolve`] turns the relative
//! target into an absolute `file://` URI anchored on the topic doc's real
//! location, or — when the doc does not ship alongside the active document —
//! removes the link markup entirely rather than emitting a dead one. Hover
//! carries the vocabulary itself (see
//! [`format_function_block`](super::expressions::format_function_block)), so a
//! dropped link costs the reader nothing.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// The authored destination prefix, up to and including the fragment separator.
/// Only this exact target is rewritten; a general relative-link rewriter would
/// have to guess at every other authored destination's intended base.
const RELATIVE_TARGET: &str = "darkmatter-expressions.md#";

/// The topic doc's location relative to a repository root.
const TOPIC_DOC: [&str; 4] = ["darkmatter", "docs", "topics", "darkmatter-expressions.md"];

/// Rewrites every authored [`RELATIVE_TARGET`] destination in `markdown` so the
/// emitted Markdown is safe to hand an editor.
///
/// `anchor` is the active document's path; the topic doc is located by walking
/// its ancestors. A resolvable doc yields an absolute `file://` URI with the
/// authored fragment preserved; an unresolvable one yields the link's label as
/// plain text.
///
/// ## Returns
///
/// The input borrowed unchanged when it carries no authored target — the
/// overwhelmingly common case, so no filesystem probe is paid for markdown that
/// could not contain the link.
pub fn resolve<'a>(markdown: &'a str, anchor: &Path) -> Cow<'a, str> {
    if !markdown.contains(RELATIVE_TARGET) {
        return Cow::Borrowed(markdown);
    }
    let topic_uri = topic_doc(anchor).and_then(|path| file_uri(&path));

    let mut out = String::with_capacity(markdown.len() + 64);
    let mut rest = markdown;
    while let Some(target) = rest.find(RELATIVE_TARGET) {
        // Only a Markdown destination is a link; the same characters in prose
        // (or in the drift tests that assert on the authored spelling) are not.
        let Some(label_end) = rest[..target].strip_suffix("](").map(str::len) else {
            let consumed = target + RELATIVE_TARGET.len();
            out.push_str(&rest[..consumed]);
            rest = &rest[consumed..];
            continue;
        };
        let Some(close) = rest[target..].find(')').map(|offset| target + offset) else {
            break;
        };
        let fragment = &rest[target + RELATIVE_TARGET.len()..close];

        match &topic_uri {
            // `rest[..target]` already ends with the destination's `](`.
            Some(uri) => {
                out.push_str(&rest[..target]);
                out.push_str(uri);
                out.push('#');
                out.push_str(fragment);
                out.push(')');
            }
            None => {
                let head = &rest[..label_end];
                match head.rfind('[') {
                    Some(open) => {
                        out.push_str(&head[..open]);
                        out.push_str(&head[open + 1..]);
                    }
                    // No opening bracket: not a link after all, so the
                    // destination text stays exactly as authored.
                    None => out.push_str(&rest[..close + 1]),
                }
            }
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// The topic doc reachable from `anchor`, found by walking the active
/// document's ancestors for a `darkmatter/docs/topics/` checkout.
///
/// ## Notes
///
/// Anchoring on the document rather than on the `initialize` workspace roots is
/// deliberate: a root is optional (a client may open a single file with no
/// folder at all), and a multi-root session would still have to pick which root
/// owns the document. Ancestors always exist and always belong to the file the
/// user is actually looking at.
fn topic_doc(anchor: &Path) -> Option<PathBuf> {
    for base in anchor.parent()?.ancestors() {
        let mut candidate = base.to_path_buf();
        candidate.extend(TOPIC_DOC);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// The `file://` URI for an absolute path.
///
/// Goes through `url` — the same conversion
/// [`file_path_to_uri`](crate::workspace::file_path_to_uri) uses — so Windows
/// drive letters serialize as `file:///C:/…`, separators normalize to `/`, and
/// non-URI bytes percent-encode. Hand-rolling this is how `file://` targets
/// break on one platform only.
fn file_uri(path: &Path) -> Option<String> {
    url::Url::from_file_path(path).ok().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a workspace holding the topic doc plus a document that is *not*
    /// under `darkmatter/docs/topics/`, and returns (workspace, document path).
    fn workspace_with_topic_doc() -> (tempfile::TempDir, PathBuf) {
        let workspace = tempfile::tempdir().unwrap();
        let topics = workspace.path().join("darkmatter/docs/topics");
        std::fs::create_dir_all(&topics).unwrap();
        std::fs::write(
            topics.join("darkmatter-expressions.md"),
            "# Expressions\n\n## Provider Query Vocabulary\n",
        )
        .unwrap();
        let document = workspace.path().join("notes/deep/page.md");
        std::fs::create_dir_all(document.parent().unwrap()).unwrap();
        std::fs::write(&document, "body\n").unwrap();
        (workspace, document)
    }

    const AUTHORED: &str =
        "See the [provider query vocabulary](darkmatter-expressions.md#provider-query-vocabulary) for keys.";

    #[test]
    fn markdown_without_the_authored_target_is_borrowed_unchanged() {
        let anchor = Path::new("/nowhere/page.md");
        let plain = "**`length(list) -> number`**\n\nThe number of items.";
        assert!(matches!(resolve(plain, anchor), Cow::Borrowed(_)));
    }

    #[test]
    fn a_resolvable_topic_doc_becomes_an_absolute_file_uri_with_the_fragment_kept() {
        let (workspace, document) = workspace_with_topic_doc();
        let resolved = resolve(AUTHORED, &document);

        // The emitted target is absolute, and it resolves back to a real file
        // whose anchor exists — the property the relative spelling lacked.
        let target = emitted_target(&resolved);
        assert!(target.starts_with("file:///"), "{target}");
        let url = url::Url::parse(&target).expect("emitted target parses as a URL");
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.fragment(), Some("provider-query-vocabulary"));
        let path = url.to_file_path().expect("file URI converts back to a path");
        assert!(path.is_file(), "{path:?}");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("## Provider Query Vocabulary"));
        // Label and surrounding prose survive untouched.
        assert!(resolved.starts_with("See the [provider query vocabulary]("));
        assert!(resolved.ends_with(") for keys."));
        drop(workspace);
    }

    #[test]
    fn a_document_inside_the_topic_directory_also_resolves() {
        // The ancestor walk passes through `topics/`, `docs/`, `darkmatter/`
        // before reaching the root that owns the doc, so a sibling document
        // must not short-circuit on a partial prefix.
        let (workspace, _) = workspace_with_topic_doc();
        let sibling = workspace.path().join("darkmatter/docs/topics/other.md");
        std::fs::write(&sibling, "body\n").unwrap();
        let resolved = resolve(AUTHORED, &sibling);
        assert!(emitted_target(&resolved).starts_with("file:///"));
    }

    #[test]
    fn an_unresolvable_topic_doc_drops_the_link_and_keeps_the_label() {
        let workspace = tempfile::tempdir().unwrap();
        let document = workspace.path().join("page.md");
        std::fs::write(&document, "body\n").unwrap();

        let resolved = resolve(AUTHORED, &document);
        assert_eq!(resolved, "See the provider query vocabulary for keys.");
        // No dead relative destination survives on any surface.
        assert!(!resolved.contains("darkmatter-expressions.md"));
        assert!(!resolved.contains(']'));
    }

    #[test]
    fn every_occurrence_is_rewritten_not_just_the_first() {
        let (workspace, document) = workspace_with_topic_doc();
        let twice = format!("{AUTHORED}\n\n{AUTHORED}");
        let resolved = resolve(&twice, &document);
        assert_eq!(resolved.matches("file:///").count(), 2);
        assert!(!resolved.contains("](darkmatter-expressions.md#"));
        drop(workspace);
    }

    #[test]
    fn the_target_outside_a_destination_is_left_alone() {
        // Prose (and this feature's own drift tests) may name the authored
        // spelling without it being a link; rewriting that would corrupt text.
        let (workspace, document) = workspace_with_topic_doc();
        let prose = "The catalog authors darkmatter-expressions.md#provider-query-vocabulary.";
        assert_eq!(resolve(prose, &document), prose);
        drop(workspace);
    }

    #[test]
    fn an_unterminated_destination_is_left_alone() {
        let (workspace, document) = workspace_with_topic_doc();
        let truncated = "[vocabulary](darkmatter-expressions.md#provider-query-vocabulary";
        assert_eq!(resolve(truncated, &document), truncated);
        drop(workspace);
    }

    #[test]
    fn file_uri_percent_encodes_and_normalizes_separators() {
        // A space is the cheap host-independent proof that the destination is
        // URI-encoded rather than pasted; `url` applies the same rule to every
        // other non-URI byte.
        let workspace = tempfile::tempdir().unwrap();
        let spaced = workspace.path().join("my notes").join("doc.md");
        std::fs::create_dir_all(spaced.parent().unwrap()).unwrap();
        let uri = file_uri(&spaced).expect("absolute path converts");
        assert!(uri.contains("my%20notes"), "{uri}");
        assert!(!uri.contains('\\'), "{uri}");
        assert!(!uri.contains(' '), "{uri}");
    }

    #[test]
    fn file_uri_rejects_a_relative_path() {
        assert!(file_uri(Path::new("relative/doc.md")).is_none());
    }

    /// Windows is the platform where a hand-rolled `file://` encoder breaks:
    /// the drive letter needs the third slash (`file:///C:/…`) and every `\`
    /// must become `/`. This runs only on the Windows CI leg — it cannot be
    /// exercised on macOS or Linux, where `C:\…` is not an absolute path.
    #[cfg(windows)]
    #[test]
    fn windows_paths_serialize_with_a_drive_letter_and_forward_slashes() {
        let uri = file_uri(Path::new(r"C:\repo\darkmatter\docs\topics\darkmatter-expressions.md"))
            .expect("an absolute Windows path converts");
        assert_eq!(
            uri,
            "file:///C:/repo/darkmatter/docs/topics/darkmatter-expressions.md"
        );
        assert!(!uri.contains('\\'));
    }

    /// The destination of the first Markdown link in `markdown`.
    fn emitted_target(markdown: &str) -> String {
        let open = markdown.find("](").expect("a link destination") + 2;
        let close = markdown[open..].find(')').expect("a closed destination");
        markdown[open..open + close].to_string()
    }
}
