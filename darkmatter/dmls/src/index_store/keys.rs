//! Candidate keys: strings derived from one outgoing link that name the file
//! it *probably* targets.
//!
//! Keys over-approximate and never decide: [`target_keys`] gives a target's
//! path key and file-name key, and every stored file holding either one is a
//! candidate referrer, which real resolution in the temporary graph then
//! confirms or drops. A key must never **miss** a true referrer, so whenever a
//! form is unclear this module emits a broader key.
//!
//! | Form | Keys |
//! | --- | --- |
//! | explicitly relative (`./a.md`, `../a/spec.md`) | the joined key: the source directory lexically joined with the target |
//! | bare (`a/b.md`, `spec.md`) | the joined key **and** the file-name key, because a bare reference also plans repository-root and other context-dependent candidates |
//! | root-prefixed (`/`, `~`, `^`, `@`, `$`, a scheme such as `file://`), a directory (`sub/`), a wiki link | the file-name key |
//! | same-document anchor, external URL | none |
//!
//! Every key is NFC-normalized and lowercased; a path key uses `/` separators
//! and folds `.` and `..` lexically. A target's keys drop one Markdown
//! extension from the final segment; a link emits both the raw and the
//! extension-dropped final segment, so `[[note.md]]` still meets a
//! `note.md.md` file. A percent-encoded or `<...>` target emits keys for both
//! its decoded and its authored spelling. Path keys always contain a `/` and
//! file-name keys never do, so the two kinds share one key column.

use std::collections::BTreeSet;
use std::path::Path;

use crate::graph::{DocumentIndex, LinkTarget};
use crate::wiki::{decode_percent_once, elide_markdown_extension, nfc};

/// The candidate keys of every link, wiki link, and transclusion in `index`.
pub fn candidate_keys(index: &DocumentIndex) -> BTreeSet<String> {
    let source_dir = index.path.parent().map_or_else(String::new, path_text);
    let mut keys = BTreeSet::new();
    for link in &index.links {
        match &link.target {
            LinkTarget::RelativePath { path, .. } => path_keys(&source_dir, path, &mut keys),
            LinkTarget::External => {
                // `file://` is classified external, yet it can name a local
                // Markdown file, so it gets the broad file-name key.
                if let Some(rest) = strip_prefix_ignore_ascii_case(&link.raw_target, "file:") {
                    let path = rest.split_once('#').map_or(rest, |(path, _)| path);
                    for variant in spellings(path) {
                        name_keys(&variant, &mut keys);
                    }
                }
            }
            LinkTarget::SameDocumentAnchor { .. } => {}
        }
    }
    for wiki in &index.wiki_links {
        // An empty target (`[[#heading]]`) names the source document itself.
        for variant in spellings(&wiki.target) {
            name_keys(&variant, &mut keys);
        }
    }
    for transclusion in &index.transclusions {
        path_keys(&source_dir, &transclusion.path, &mut keys);
    }
    keys
}

/// The keys a stored referrer of `target` holds at least one of: its path key
/// and its file-name key, in that order.
pub fn target_keys(target: &Path) -> [String; 2] {
    let normalized = normalize(&path_text(target));
    let path_key = fold(&with_last_segment(&normalized, elide_markdown_extension));
    let name = last_segment(&normalized);
    [path_key, fold(elide_markdown_extension(name))]
}

/// Keys for a link or transclusion path authored in a file under
/// `source_dir`.
fn path_keys(source_dir: &str, path: &str, keys: &mut BTreeSet<String>) {
    for variant in spellings(path) {
        match form(&variant) {
            Form::Explicit => joined_keys(source_dir, &variant, keys),
            Form::Bare => {
                joined_keys(source_dir, &variant, keys);
                name_keys(&variant, keys);
            }
            Form::RootPrefixed => name_keys(&variant, keys),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Explicit,
    Bare,
    RootPrefixed,
}

fn form(path: &str) -> Form {
    let unified = path.replace('\\', "/");
    if unified == "." || unified == ".." || unified.starts_with("./") || unified.starts_with("../") {
        return Form::Explicit;
    }
    let root_prefixed = unified.starts_with(['/', '~', '^', '@', '$'])
        || unified.ends_with('/')
        || has_scheme(&unified);
    if root_prefixed { Form::RootPrefixed } else { Form::Bare }
}

/// Whether `text` starts with `scheme:` (which also covers a `C:` drive).
fn has_scheme(text: &str) -> bool {
    let Some((scheme, _)) = text.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// The authored spelling plus its `<...>`-stripped and percent-decoded forms,
/// de-duplicated, empty ones dropped.
fn spellings(raw: &str) -> Vec<String> {
    let stripped = raw
        .strip_prefix('<')
        .and_then(|inner| inner.strip_suffix('>'))
        .unwrap_or(raw);
    let mut out: Vec<String> = Vec::with_capacity(3);
    for candidate in [raw.to_string(), stripped.to_string(), decode_percent_once(stripped).0] {
        if !candidate.is_empty() && !out.contains(&candidate) {
            out.push(candidate);
        }
    }
    out
}

fn joined_keys(source_dir: &str, path: &str, keys: &mut BTreeSet<String>) {
    let joined = normalize(&format!("{source_dir}/{path}"));
    if last_segment(&joined).is_empty() {
        return;
    }
    keys.insert(fold(&joined));
    keys.insert(fold(&with_last_segment(&joined, elide_markdown_extension)));
}

fn name_keys(path: &str, keys: &mut BTreeSet<String>) {
    let unified = path.replace('\\', "/");
    let Some(name) = unified.split('/').rev().find(|segment| !segment.is_empty()) else {
        return;
    };
    keys.insert(fold(name));
    let elided = elide_markdown_extension(name);
    keys.insert(fold(elided));
}

/// `path` as `/`-separated text with any Windows verbatim prefix removed.
fn path_text(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    match text.strip_prefix("//?/") {
        Some(rest) => rest.strip_prefix("UNC/").map_or_else(|| rest.to_string(), |unc| format!("//{unc}")),
        None => text,
    }
}

/// Lexically folds `.`, `..`, and repeated separators in `/`-separated
/// `text`. `..` never climbs above the root or a drive (`C:`).
fn normalize(text: &str) -> String {
    let absolute = text.starts_with('/');
    let mut segments: Vec<&str> = Vec::new();
    for segment in text.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                let at_floor = segments.is_empty()
                    || (segments.len() == 1 && segments[0].ends_with(':'))
                    || segments.last() == Some(&"..");
                if !at_floor {
                    segments.pop();
                } else if !absolute && segments.is_empty() {
                    segments.push("..");
                }
            }
            other => segments.push(other),
        }
    }
    let joined = segments.join("/");
    if absolute { format!("/{joined}") } else { joined }
}

fn last_segment(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn with_last_segment(path: &str, change: impl Fn(&str) -> &str) -> String {
    match path.rsplit_once('/') {
        Some((head, last)) => format!("{head}/{}", change(last)),
        None => change(path).to_string(),
    }
}

fn fold(text: &str) -> String {
    nfc(text).to_lowercase()
}

fn strip_prefix_ignore_ascii_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix).then(|| &text[prefix.len()..])
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::graph::index_document;

    fn keys_of(path: &str, source: &str) -> BTreeSet<String> {
        candidate_keys(&index_document(&PathBuf::from(path), source))
    }

    fn matches(keys: &BTreeSet<String>, target: &str) -> bool {
        target_keys(Path::new(target)).iter().any(|key| keys.contains(key))
    }

    #[test]
    fn explicitly_relative_link_yields_only_the_joined_key() {
        let keys = keys_of("/ws/docs/topics/a.md", "[x](../b.md#goals)\n");
        assert_eq!(keys, BTreeSet::from(["/ws/docs/b".to_string(), "/ws/docs/b.md".to_string()]));
        assert!(matches(&keys, "/ws/docs/b.md"));
        assert!(!matches(&keys, "/ws/other/b.md"), "no file-name key for a `../` link");
    }

    #[test]
    fn bare_root_relative_link_and_transclusion_meet_their_target_by_file_name() {
        // S1's missed shape: a bare path written relative to the repository
        // root, in a nested document.
        let keys = keys_of(
            "/ws/docs/topics/a.md",
            "[b](area/docs/b.md)\n\n::file area/docs/c.md\n",
        );
        assert!(keys.contains("/ws/docs/topics/area/docs/b"), "joined key kept: {keys:?}");
        assert!(matches(&keys, "/ws/area/docs/b.md"));
        assert!(matches(&keys, "/ws/area/docs/c.md"));
    }

    #[test]
    fn wiki_link_keys_cover_case_nfd_and_extension_variants() {
        let keys = keys_of("/ws/notes/w.md", "[[Folder/Spec]] [[cafe\u{301}]] [[note.md]] [[#local]]\n");
        assert!(matches(&keys, "/ws/x/spec.md"));
        assert!(matches(&keys, "/ws/y/caf\u{e9}.md"));
        assert!(matches(&keys, "/ws/note.md"));
        assert!(matches(&keys, "/ws/note.md.md"), "raw final segment kept: {keys:?}");
        assert!(!keys.iter().any(String::is_empty));
    }

    #[test]
    fn unusual_forms_are_decoded_before_keying() {
        let keys = keys_of(
            "/ws/docs/g.md",
            "[a](my%20file.md) [b](<other file.md>) [c](file:///ws/docs/abs.md) [d](~/notes/home.md) [e](sub/) [f](/docs/root.md)\n",
        );
        for target in [
            "/ws/docs/my file.md",
            "/ws/docs/my%20file.md",
            "/ws/docs/other file.md",
            "/elsewhere/abs.md",
            "/elsewhere/home.md",
            "/elsewhere/sub.md",
            "/elsewhere/root.md",
        ] {
            assert!(matches(&keys, target), "{target} missed by {keys:?}");
        }
    }

    #[test]
    fn external_links_and_anchors_yield_no_keys() {
        let keys = keys_of("/ws/a.md", "[a](https://example.com/b.md) [b](#top) [c](mailto:x@y.z)\n");
        assert!(keys.is_empty(), "{keys:?}");
    }

    #[test]
    fn windows_paths_key_like_their_forward_slash_form() {
        assert_eq!(target_keys(Path::new(r"C:\WS\Docs\B.md"))[0], "c:/ws/docs/b");
        assert_eq!(target_keys(Path::new(r"\\?\C:\ws\docs\b.md"))[0], "c:/ws/docs/b");
        assert_eq!(normalize("C:/ws/../../x.md"), "C:/x.md");
        assert_eq!(normalize("/ws/./a//../b.md"), "/ws/b.md");
    }
}
