//! Wiki file-target matching and ranking (R-8 "Path Matching" and
//! "Multi-Root Matching").
//!
//! A parsed target is classified as root-relative (`/notes/x`), path-suffix
//! (`folder/x`), or bare basename (`x`) and matched against the canonical
//! logical paths of the indexed documents. Ranking is: same-directory →
//! unique → ambiguous, evaluated inside the source document's wiki root first.
//! There is no lexicographic fallback for real resolution — only for the
//! stable display order of an ambiguous candidate list.

use super::logical_path::{decode_percent_once, elide_markdown_extension, nfc};

/// A document as the matcher sees it: its canonical logical path and which wiki
/// root it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiDoc {
    /// Canonical logical segments (NFC, final Markdown extension elided).
    pub canonical: Vec<String>,
    /// Index of the owning wiki root, or [`WikiDoc::NO_ROOT`] when the document
    /// lies outside every configured root.
    pub workspace_id: usize,
}

impl WikiDoc {
    /// Sentinel for a document that is under no wiki root.
    pub const NO_ROOT: usize = usize::MAX;

    /// The parent-directory segments (everything but the final segment).
    fn parent(&self) -> &[String] {
        &self.canonical[..self.canonical.len().saturating_sub(1)]
    }
}

/// How a target's path is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// Leading `/`: exact match within the source document's wiki root.
    RootRelative,
    /// Contains `/`: a path-suffix match across roots.
    Suffix,
    /// No `/`: a bare basename match.
    Basename,
}

/// A parsed, canonicalized file target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTarget {
    /// Interpretation of the path.
    pub kind: TargetKind,
    /// Decoded + NFC segments; the final segment is *not* extension-elided so
    /// `[[note.md]]` can still target a `note.md.md` file.
    pub segments: Vec<String>,
    /// Whether a malformed percent escape was left literal.
    pub invalid_percent: bool,
}

/// The outcome of parsing a wiki file target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseOutcome {
    /// The target was empty (`[[]]` / `[[|alias]]`).
    Empty,
    /// The target contained `.`/`..`/empty segments — not a v1 wiki concept.
    Rejected,
    /// A resolvable file target.
    Target(ParsedTarget),
}

/// A file-resolution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Match {
    /// Exactly one document.
    One(usize),
    /// Multiple candidates (ambiguous), in stable display order.
    Many(Vec<usize>),
    /// No candidate.
    None,
}

/// Parses and canonicalizes a wiki file target.
pub fn parse_file_target(raw: &str) -> ParseOutcome {
    if raw.is_empty() {
        return ParseOutcome::Empty;
    }
    let (decoded, invalid_percent) = decode_percent_once(raw);
    let normalized = nfc(&decoded);
    let (kind, body) = match normalized.strip_prefix('/') {
        Some(rest) => (TargetKind::RootRelative, rest),
        None if normalized.contains('/') => (TargetKind::Suffix, normalized.as_str()),
        None => (TargetKind::Basename, normalized.as_str()),
    };

    let segments: Vec<String> = body.split('/').map(str::to_string).collect();
    if segments
        .iter()
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return ParseOutcome::Rejected;
    }

    ParseOutcome::Target(ParsedTarget {
        kind,
        segments,
        invalid_percent,
    })
}

/// Resolves a parsed file target against `docs` from the `source` document.
pub fn resolve_file(target: &ParsedTarget, docs: &[WikiDoc], source: usize) -> Match {
    match target.kind {
        TargetKind::Basename => resolve_basename(&target.segments[0], docs, source),
        TargetKind::Suffix => resolve_suffix(&target.segments, docs, source),
        TargetKind::RootRelative => resolve_root_relative(&target.segments, docs, source),
    }
}

/// Whether a document's `canonical` path ends with the target `segments`, with
/// the final segment matched by either its raw or extension-elided form.
pub fn ends_with_target(canonical: &[String], segments: &[String]) -> bool {
    if canonical.len() < segments.len() || segments.is_empty() {
        return false;
    }
    let offset = canonical.len() - segments.len();
    for (position, target) in segments.iter().enumerate() {
        let actual = &canonical[offset + position];
        let is_last = position == segments.len() - 1;
        if is_last {
            if !last_matches(actual, target) {
                return false;
            }
        } else if actual != target {
            return false;
        }
    }
    true
}

/// Whether a canonical final segment matches a raw target segment: exactly, or
/// after eliding the target's Markdown extension.
fn last_matches(canonical_last: &str, target_last: &str) -> bool {
    canonical_last == target_last || canonical_last == elide_markdown_extension(target_last)
}

fn resolve_basename(target: &str, docs: &[WikiDoc], source: usize) -> Match {
    let candidates: Vec<usize> = docs
        .iter()
        .enumerate()
        .filter(|(_, doc)| doc.canonical.last().is_some_and(|last| last_matches(last, target)))
        .map(|(index, _)| index)
        .collect();
    rank(candidates, docs, source)
}

fn resolve_suffix(segments: &[String], docs: &[WikiDoc], source: usize) -> Match {
    let candidates: Vec<usize> = docs
        .iter()
        .enumerate()
        .filter(|(_, doc)| ends_with_target(&doc.canonical, segments))
        .map(|(index, _)| index)
        .collect();
    rank(candidates, docs, source)
}

fn resolve_root_relative(segments: &[String], docs: &[WikiDoc], source: usize) -> Match {
    let source_root = docs[source].workspace_id;
    let mut candidates: Vec<usize> = docs
        .iter()
        .enumerate()
        .filter(|(_, doc)| {
            (source_root == WikiDoc::NO_ROOT || doc.workspace_id == source_root)
                && doc.canonical.len() == segments.len()
                && ends_with_target(&doc.canonical, segments)
        })
        .map(|(index, _)| index)
        .collect();
    match candidates.len() {
        0 => Match::None,
        1 => Match::One(candidates[0]),
        _ => {
            sort_for_display(&mut candidates, docs);
            Match::Many(candidates)
        }
    }
}

/// Applies same-directory → unique → ambiguous ranking to a candidate set.
fn rank(mut candidates: Vec<usize>, docs: &[WikiDoc], source: usize) -> Match {
    match candidates.len() {
        0 => Match::None,
        1 => Match::One(candidates[0]),
        _ => {
            let local: Vec<usize> = candidates
                .iter()
                .copied()
                .filter(|&candidate| same_directory(&docs[candidate], &docs[source]))
                .collect();
            if local.len() == 1 {
                Match::One(local[0])
            } else {
                sort_for_display(&mut candidates, docs);
                Match::Many(candidates)
            }
        }
    }
}

/// Whether `candidate` sits in the same directory (and wiki root) as `source`.
fn same_directory(candidate: &WikiDoc, source: &WikiDoc) -> bool {
    candidate.workspace_id == source.workspace_id && candidate.parent() == source.parent()
}

/// Stable lexicographic order (by root, then path) for ambiguous candidate
/// display only — never for real resolution.
fn sort_for_display(candidates: &mut [usize], docs: &[WikiDoc]) {
    candidates.sort_by(|&a, &b| {
        docs[a]
            .workspace_id
            .cmp(&docs[b].workspace_id)
            .then_with(|| docs[a].canonical.cmp(&docs[b].canonical))
    });
}

/// The shortest suffix of `target`'s canonical path that resolves uniquely to
/// it from `source`, escalating one leading segment at a time (R-8 "Completion
/// Insertion Policy", `shortest`). Falls back to the full canonical path when
/// no suffix is unique (cross-root duplication).
pub fn shortest_unique_suffix(docs: &[WikiDoc], target: usize, source: usize) -> Vec<String> {
    let canonical = &docs[target].canonical;
    for depth in 1..=canonical.len() {
        let suffix = canonical[canonical.len() - depth..].to_vec();
        let kind = if depth == 1 {
            TargetKind::Basename
        } else {
            TargetKind::Suffix
        };
        let parsed = ParsedTarget {
            kind,
            segments: suffix.clone(),
            invalid_percent: false,
        };
        if resolve_file(&parsed, docs, source) == Match::One(target) {
            return suffix;
        }
    }
    canonical.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(path: &str, workspace: usize) -> WikiDoc {
        WikiDoc {
            canonical: path.split('/').map(str::to_string).collect(),
            workspace_id: workspace,
        }
    }

    fn parsed(raw: &str) -> ParsedTarget {
        match parse_file_target(raw) {
            ParseOutcome::Target(target) => target,
            other => panic!("expected target, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_classifies_kinds() {
        assert_eq!(parsed("note").kind, TargetKind::Basename);
        assert_eq!(parsed("folder/note").kind, TargetKind::Suffix);
        assert_eq!(parsed("/notes/note").kind, TargetKind::RootRelative);
    }

    #[test]
    fn test_parse_percent_decode_before_matching() {
        let target = parsed("My%20Note");
        assert_eq!(target.segments, vec!["My Note"]);
        assert!(!target.invalid_percent);
    }

    #[test]
    fn test_parse_rejects_dot_segments() {
        assert_eq!(parse_file_target("./note"), ParseOutcome::Rejected);
        assert_eq!(parse_file_target("../note"), ParseOutcome::Rejected);
        assert_eq!(parse_file_target("a//b"), ParseOutcome::Rejected);
        assert_eq!(parse_file_target(""), ParseOutcome::Empty);
    }

    #[test]
    fn test_same_directory_wins_over_unique_basename() {
        // Source in notes/; Target.md in notes/ and notes/other/.
        let docs = vec![
            doc("notes/Source", 0),
            doc("notes/Target", 0),
            doc("notes/other/Target", 0),
        ];
        assert_eq!(resolve_file(&parsed("Target"), &docs, 0), Match::One(1));
    }

    #[test]
    fn test_ambiguous_basename_across_roots() {
        let docs = vec![
            doc("notes/Source", 0),
            doc("notes/ambiguous/Same", 0),
            doc("notes/also/Same", 0),
            doc("notes/ambiguous/Same", 1),
        ];
        // Ambiguous, sorted for stable display by (root, path): "also" < "ambiguous".
        assert_eq!(
            resolve_file(&parsed("Same"), &docs, 0),
            Match::Many(vec![2, 1, 3])
        );
    }

    #[test]
    fn test_case_sensitive() {
        let docs = vec![doc("notes/Source", 0), doc("notes/Case", 0), doc("notes/case", 0)];
        assert_eq!(resolve_file(&parsed("case"), &docs, 0), Match::One(2));
        assert_eq!(resolve_file(&parsed("Case"), &docs, 0), Match::One(1));
    }

    #[test]
    fn test_no_space_dash_equivalence() {
        let docs = vec![doc("notes/Source", 0), doc("notes/my-note", 0)];
        assert_eq!(resolve_file(&parsed("my note"), &docs, 0), Match::None);
        assert_eq!(resolve_file(&parsed("my-note"), &docs, 0), Match::One(1));
    }

    #[test]
    fn test_root_relative_within_root() {
        let docs = vec![
            doc("notes/Source", 0),
            doc("notes/folder/Target", 0),
            doc("notes/folder/Target", 1),
        ];
        assert_eq!(
            resolve_file(&parsed("/notes/folder/Target"), &docs, 0),
            Match::One(1)
        );
    }

    #[test]
    fn test_doubled_extension_target() {
        // `[[note.md]]` targets a physical note.md.md whose canonical is note.md.
        let docs = vec![doc("literal/note.md", 0), doc("notes/Source", 0)];
        assert_eq!(
            resolve_file(&parsed("literal/note.md"), &docs, 1),
            Match::One(0)
        );
    }

    #[test]
    fn test_shortest_unique_suffix_escalates() {
        let docs = vec![
            doc("notes/Source", 0),
            doc("a/Same", 0),
            doc("b/Same", 0),
        ];
        // Basename `Same` is ambiguous, so completion escalates to `a/Same`.
        assert_eq!(shortest_unique_suffix(&docs, 1, 0), vec!["a", "Same"]);
    }
}
