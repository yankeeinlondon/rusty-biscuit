//! Shared helpers used across the compose pipeline that are not stages
//! themselves: path abbreviation, git-root discovery, reference-target
//! location, and pre-effective-state frontmatter preparation.
//!
//! These are re-exported from `compose/mod.rs` so in-crate callers continue to
//! reach them as `compose::<name>`.

use super::Markdown;
use super::context;
use super::context::options::ComposeOptions;
use biscuit_file::{FileResolutionContext, PathPosition};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::trace;

/// Shorten an absolute path for display in diagnostics.
///
/// Tries to make the path relative to the git repo root discovered by
/// walking up from the file's parent directory. Falls back to `~/…` when
/// the path is under the user's home directory, or the absolute path
/// otherwise.
pub(crate) fn abbreviate_path(path: &Path) -> String {
    // Try git repo root first (walk up looking for .git)
    if let Some(root) = find_git_root_from(path)
        && let Ok(rel) = path.strip_prefix(&root)
    {
        return rel.display().to_string();
    }

    // Fall back to ~/… for paths under HOME
    if let Some(home) = dirs::home_dir()
        && let Ok(rel) = path.strip_prefix(&home)
    {
        return format!("~/{}", rel.display());
    }

    path.display().to_string()
}

/// Walk up from `start` toward the filesystem root looking for a `.git` entry,
/// returning the directory that contains it.
///
/// The search begins at `start` itself: a file `start` simply has no `.git`
/// child, so its parent is examined on the next iteration. This deliberately
/// avoids branching on an `is_dir()` probe of `start` — under heavy parallel
/// filesystem load that stat can fail transiently, and skipping `start` would
/// then let a `.git` in a shared ancestor (e.g. `$TMPDIR`) hijack the resolved
/// root. Checking `start.join(".git")` first keeps a pinned repo root
/// (including a tempdir with its own `.git` marker) authoritative.
pub fn find_git_root_from(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Build an explicit, request-scoped [`FileResolutionContext`] for a
/// document-backed reference.
///
/// `base_dir` is the authoring document's directory (the base for the
/// references it contains); `source_path`, when known, is the document file
/// itself. Repository, package, and package-area scopes come only from the
/// request snapshot and are recomputed for the authoring document.
///
/// [`ComposeOptions::expression_resolution_context`]: super::context::options::ComposeOptions::expression_resolution_context
pub(crate) fn document_resolution_context(
    base_dir: &Path,
    source_path: Option<&Path>,
    magic_paths: &[(PathBuf, PathPosition)],
    request_context: Option<&FileResolutionContext>,
) -> FileResolutionContext {
    let mut ctx = match (request_context, source_path) {
        (Some(snapshot), Some(source)) => snapshot.for_source(source),
        (Some(snapshot), None) => snapshot.for_base(base_dir),
        (None, _) => FileResolutionContext::new(base_dir),
    };
    for (path, position) in magic_paths {
        ctx = ctx.add_magic_path(path.clone(), *position);
    }
    ctx
}

/// Helper to find target range within content.
pub(crate) fn find_target_range(
    content: &str,
    record: &crate::markdown::reference::ReferenceRecord,
    raw_target: &str,
) -> Option<(usize, usize)> {
    let span = &record.origin.span;
    if span.end > content.len() {
        trace!(
            "find_target_range: span.end {} > content.len {}",
            span.end,
            content.len()
        );
        return None;
    }
    let outer_text = &content[span.clone()];

    // Try attribute-aware search for HTML syntax first
    if let Some(attr_name) = get_attribute_name_for_syntax(&record.origin.syntax) {
        // Try searching for attr="target" or attr = "target" (with optional whitespace)
        let patterns = [
            format!(r#"{}="{}""#, attr_name, raw_target),
            format!("{}='{}'", attr_name, raw_target),
        ];
        for pattern in &patterns {
            if let Some(idx) = outer_text.find(pattern) {
                let mut actual_idx = idx + attr_name.len() + 1; // skip past attr_name=
                // Skip optional whitespace after =
                while actual_idx < outer_text.len() && outer_text[actual_idx..].starts_with(' ') {
                    actual_idx += 1;
                }
                let start = span.start + actual_idx + 1; // skip past quote
                let end = start + raw_target.len();
                trace!(
                    "find_target_range: attribute-aware match for '{}' in {:?} at {}",
                    raw_target, record.origin.syntax, start
                );
                return Some((start, end));
            }
        }

        // Try HTML-encoded form (for entities like &amp;)
        let encoded = html_escape::encode_quoted_attribute(raw_target);
        if encoded != raw_target {
            let patterns = [
                format!(r#"{}="{}""#, attr_name, encoded),
                format!("{}='{}'", attr_name, encoded),
            ];
            for pattern in &patterns {
                if let Some(idx) = outer_text.find(pattern.as_str()) {
                    let actual_idx = idx + attr_name.len() + 1;
                    // Skip optional whitespace after =
                    let mut quote_start = actual_idx;
                    while quote_start < outer_text.len()
                        && outer_text[quote_start..].starts_with(' ')
                    {
                        quote_start += 1;
                    }
                    let start = span.start + quote_start + 1; // skip past quote
                    let end = start + encoded.len();
                    trace!(
                        "find_target_range: HTML-encoded match for '{}' in {:?} at {}",
                        raw_target, record.origin.syntax, start
                    );
                    return Some((start, end));
                }
            }
        }
    }

    // Fallback: search for raw target string with context check
    let mut start_idx = 0;
    while let Some(idx) = outer_text[start_idx..].find(raw_target) {
        let actual_idx = start_idx + idx;

        if actual_idx > 0 {
            let prev_char = outer_text[..actual_idx].chars().next_back();
            trace!(
                "find_target_range: found '{}' at {}, prev_char: {:?}",
                raw_target, actual_idx, prev_char
            );
            if matches!(prev_char, Some('(' | '=' | '"' | '\'' | '<')) {
                let start = span.start + actual_idx;
                let end = start + raw_target.len();
                return Some((start, end));
            }
        } else {
            trace!("find_target_range: found '{}' at 0", raw_target);
            // Edge case: if raw_target is the exact span, match it.
            let start = span.start + actual_idx;
            let end = start + raw_target.len();
            return Some((start, end));
        }

        start_idx = actual_idx + 1;
    }
    trace!(
        "find_target_range: failed to find '{}' in '{}'",
        raw_target, outer_text
    );
    None
}

/// Maps a ReferenceSyntax to its target attribute name for HTML-aware matching.
fn get_attribute_name_for_syntax(
    syntax: &crate::markdown::reference::ReferenceSyntax,
) -> Option<&'static str> {
    use crate::markdown::reference::ReferenceSyntax;
    match syntax {
        ReferenceSyntax::HtmlAnchor | ReferenceSyntax::HtmlLinkTag => Some("href"),
        ReferenceSyntax::HtmlImage
        | ReferenceSyntax::HtmlVideoTag
        | ReferenceSyntax::HtmlAudioTag
        | ReferenceSyntax::HtmlSourceTag
        | ReferenceSyntax::HtmlIframeTag
        | ReferenceSyntax::HtmlScriptTag => Some("src"),
        _ => None,
    }
}

/// Applies pre-effective-state frontmatter preparation shared by runtime
/// compose and shell-command discovery.
///
/// This mutates frontmatter with external-state defaults and `--set`
/// overrides using the same rules the real compose pipeline uses. When
/// requested, it also captures the post-merge/pre-interpolation string
/// snapshot used for frontmatter shell executable provenance checks.
pub(crate) fn prepare_frontmatter_for_compose(
    markdown: &mut Markdown,
    options: &ComposeOptions,
    capture_pre_interpolation_snapshot: bool,
) -> Option<HashMap<String, String>> {
    // Apply external state as defaults using deep-merge: nested keys
    // from external state fill in missing values at every level, not
    // just top-level keys. Frontmatter values take precedence.
    if let Some(external) = options.external_state.as_ref() {
        let fm = markdown.frontmatter_mut().as_map_mut();
        let current = Value::Object(fm.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let merged = context::effective_state::deep_merge(external, &current);
        if let Value::Object(map) = merged {
            *fm = map.into_iter().collect();
        }
    }

    // Apply set overrides: unconditionally overwrite frontmatter keys.
    if let Some(overrides) = options.set_overrides.as_ref().and_then(Value::as_object) {
        let fm = markdown.frontmatter_mut().as_map_mut();
        for (key, value) in overrides {
            fm.insert(key.clone(), value.clone());
        }
    }

    capture_pre_interpolation_snapshot.then(|| {
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .filter_map(|(key, value)| value.as_str().map(|s| (key.clone(), s.to_string())))
            .collect()
    })
}

#[cfg(test)]
mod resolution_context_tests {
    //! Shared-fixture parity for the single seam every document-backed
    //! Darkmatter surface routes through.
    //!
    //! Transclusion, expression `file(...)`, schema `file(...)`, and local link
    //! resolution all build their [`FileResolutionContext`] from
    //! [`document_resolution_context`] and resolve through
    //! [`FileReference::resolve_in_context`]. Cross-surface parity is therefore a
    //! property of this one helper: given the same base/source/repository inputs,
    //! every surface produces the identical context and the identical resolution.
    //! Proving the seam document-first on a real collision fixture proves the
    //! shared contract for all of them at Level 1, where the resolution semantics
    //! live; only the terminal *rendering* of a failure needs Level 2.

    use super::{FileResolutionContext, document_resolution_context};
    use biscuit_file::FileReference;
    use std::fs;

    #[test]
    fn seam_resolves_implicit_document_first_and_explicit_source_only() {
        let repo = tempfile::TempDir::new().unwrap();
        let root = repo.path();
        // `find_git_root_from` only needs a `.git` marker, matching the other
        // in-crate resolution tests.
        fs::create_dir_all(root.join(".git")).unwrap();

        // A name-collision: the same file exists at the repository root and under
        // the authoring document's directory, so precedence is observable.
        fs::write(root.join("notes.md"), b"repo").unwrap();
        let base = root.join("prompts");
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("notes.md"), b"source decoy").unwrap();

        let request = FileResolutionContext::new(&base).with_repository_root(root);
        let ctx: FileResolutionContext = document_resolution_context(
            &base,
            Some(&base.join("router.md")),
            &[],
            Some(&request),
        );

        // Implicit bare reference: the source candidate wins over the repository
        // twin. Canonicalize both sides so an explicit `.` path component or a
        // `/var`->`/private/var` tempdir symlink cannot mask the anchor identity.
        let implicit = FileReference::new("notes.md")
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap()
            .map(|p| p.canonicalize().unwrap());
        assert_eq!(
            implicit,
            Some(base.join("notes.md").canonicalize().unwrap()),
            "every surface sharing this seam resolves implicit references \
             document-first",
        );

        // Explicit `./` reference: pinned to the source directory, no fallback.
        let explicit = FileReference::new("./notes.md")
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap()
            .map(|p| p.canonicalize().unwrap());
        assert_eq!(
            explicit,
            Some(base.join("notes.md").canonicalize().unwrap()),
            "the explicit form stays source-relative across every surface",
        );
    }

    #[test]
    fn seam_resolves_package_reference_from_captured_area_before_repository() {
        let repo = tempfile::TempDir::new().unwrap();
        let root = repo.path();
        let package_area = root.join("darkmatter");
        let base = package_area.join("docs");
        fs::create_dir_all(&base).unwrap();
        fs::write(root.join("shared.md"), b"repository decoy").unwrap();
        fs::write(package_area.join("shared.md"), b"package").unwrap();

        let request = FileResolutionContext::new(&base)
            .with_repository_root(root)
            .with_package_area(&package_area);
        let ctx = document_resolution_context(
            &base,
            Some(&base.join("guide.md")),
            &[],
            Some(&request),
        );
        let resolved = FileReference::new("^shared.md")
            .unwrap()
            .resolve_in_context(&ctx)
            .unwrap();

        assert_eq!(resolved, Some(package_area.join("shared.md")));
    }
}
