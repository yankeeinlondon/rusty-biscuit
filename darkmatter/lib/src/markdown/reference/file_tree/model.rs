//! View model types and graph-to-model transformation for the FileTree component.
//!
//! Transforms a [`ReferenceGraph`] plus optional [`ReferenceValidationReport`] into
//! a presentation-ready [`FileTreeModel`] without polluting the graph types with
//! terminal-only concerns.

use std::collections::HashMap;

use crate::markdown::compose::ComposeSource;
use crate::markdown::normalize::HeadingLevel;
use crate::markdown::reference::types::{
    NodeId, ReferenceGraph, ReferenceGraphNode, ReferenceInsertionContext, ReferenceKind,
    ReferenceRecord, ReferenceSyntax, ReferenceTarget,
};
use crate::markdown::reference::validate::{
    ReferenceIssueCode, ReferenceSeverity, ReferenceValidationReport,
};

// ── View model types ────────────────────────────────────────────────

/// The complete view model for a FileTree.
#[derive(Debug, Clone)]
pub struct FileTreeModel {
    pub root: FileTreeNode,
}

/// A single node in the FileTree view.
#[derive(Debug, Clone)]
pub struct FileTreeNode {
    /// The compose source for this node.
    pub source: ComposeSource,
    /// Display label for the file (filename, not full path).
    pub file_label: String,
    /// Icon classification for the file.
    pub file_icon_kind: FileTreeIconKind,
    /// Inline summary counts for the file head line.
    pub inline_summary: FileTreeInlineSummary,
    /// Reference groups above the file line.
    pub reference_groups: Vec<FileTreeReferenceGroup>,
    /// Transclusion edges below the file line.
    pub transclusions: Vec<FileTreeTransclusionEdge>,
    /// Recursively expanded child nodes (only populated when follow=true).
    pub children: Vec<FileTreeNode>,
    /// Node-level validation status.
    pub validation: FileTreeNodeValidation,
}

/// Icon classification for the file head line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTreeIconKind {
    Markdown,
    GenericFile,
}

/// Counts of inline resources for the file head summary.
#[derive(Debug, Clone, Default)]
pub struct FileTreeInlineSummary {
    pub inline_css_count: usize,
    pub inline_script_count: usize,
    pub meta_tag_count: usize,
}

impl FileTreeInlineSummary {
    pub fn is_empty(&self) -> bool {
        self.inline_css_count == 0 && self.inline_script_count == 0 && self.meta_tag_count == 0
    }

    /// Returns a formatted summary string like "(2 inline scripts, 3 meta tags)".
    pub fn to_display_string(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        if self.inline_css_count > 0 {
            let s = if self.inline_css_count == 1 { "" } else { "s" };
            parts.push(format!("{} inline CSS block{s}", self.inline_css_count));
        }
        if self.inline_script_count > 0 {
            let s = if self.inline_script_count == 1 {
                ""
            } else {
                "s"
            };
            parts.push(format!("{} inline script{s}", self.inline_script_count));
        }
        if self.meta_tag_count > 0 {
            let s = if self.meta_tag_count == 1 { "" } else { "s" };
            parts.push(format!("{} meta tag{s}", self.meta_tag_count));
        }
        Some(format!("({})", parts.join(", ")))
    }
}

/// Semantic grouping of references above the file line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileTreeReferenceGroupKind {
    RemoteHyperlinks,
    LocalHyperlinks,
    Images,
    Media,
    Iframes,
    CssImports,
    ScriptImports,
    FontImports,
    OtherLocalDependencies,
}

impl FileTreeReferenceGroupKind {
    /// Returns the fixed display order (lower = higher in the tree).
    pub fn sort_order(&self) -> u8 {
        match self {
            Self::RemoteHyperlinks => 0,
            Self::LocalHyperlinks => 1,
            Self::Images => 2,
            Self::Media => 3,
            Self::Iframes => 4,
            Self::CssImports => 5,
            Self::ScriptImports => 6,
            Self::FontImports => 7,
            Self::OtherLocalDependencies => 8,
        }
    }
}

/// A group of references sharing the same semantic kind.
#[derive(Debug, Clone)]
pub struct FileTreeReferenceGroup {
    pub kind: FileTreeReferenceGroupKind,
    pub rows: Vec<FileTreeReferenceRow>,
}

/// A single reference row above the file line.
#[derive(Debug, Clone)]
pub struct FileTreeReferenceRow {
    pub kind: ReferenceKind,
    pub display_target: String,
    pub raw_reference_id: String,
    pub validation: Option<FileTreeReferenceValidation>,
}

/// Validation status for a single reference.
#[derive(Debug, Clone)]
pub struct FileTreeReferenceValidation {
    pub is_valid: bool,
    pub suffix: Option<String>,
    pub severity: ReferenceSeverity,
}

/// Transclusion kind for display purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTreeTransclusionKind {
    File,
    Code,
    Url,
    TocLinking,
    FileLinks,
    Prologue,
    Epilogue,
}

/// A transclusion edge below the file line.
#[derive(Debug, Clone)]
pub struct FileTreeTransclusionEdge {
    pub kind: FileTreeTransclusionKind,
    pub display_target: String,
    pub caption: String,
    pub directive_line: usize,
    pub followable: bool,
    pub child_node_id: Option<NodeId>,
    pub validation: Option<FileTreeReferenceValidation>,
}

/// Node-level validation summary.
#[derive(Debug, Clone, Default)]
pub struct FileTreeNodeValidation {
    pub issues_count: usize,
    pub has_errors: bool,
}

// ── Transformation ──────────────────────────────────────────────────

/// Build a [`FileTreeModel`] from a [`ReferenceGraph`] and optional validation report.
pub fn build_file_tree_model(
    graph: &ReferenceGraph,
    report: Option<&ReferenceValidationReport>,
    follow: bool,
) -> FileTreeModel {
    let issue_map = report.map(build_issue_map).unwrap_or_default();

    // Build an id → &ReferenceGraphNode map once for O(1) lookups
    // during recursive model construction (avoids O(n²) linear scans).
    let mut node_map: HashMap<&str, &ReferenceGraphNode> =
        HashMap::with_capacity(graph.nodes().len() + 1);
    node_map.insert(graph.root().node_id.as_ref(), graph.root());
    for node in graph.nodes() {
        node_map.insert(node.node_id.as_ref(), node);
    }

    let root = build_node_model(graph.root(), &node_map, &issue_map, follow);
    FileTreeModel { root }
}

/// Build a HashMap of reference_id → validation info for fast lookup.
fn build_issue_map(
    report: &ReferenceValidationReport,
) -> HashMap<String, FileTreeReferenceValidation> {
    let mut map = HashMap::new();
    for issue in &report.issues {
        let suffix = match issue.code {
            ReferenceIssueCode::MissingLocalTarget => Some("[missing]".to_string()),
            ReferenceIssueCode::InvalidUrl => Some("[invalid url]".to_string()),
            ReferenceIssueCode::UnsupportedScheme => Some("[unsupported]".to_string()),
            ReferenceIssueCode::RemoteUnreachable => Some("[unreachable]".to_string()),
            ReferenceIssueCode::MissingFragmentTarget => Some("[missing fragment]".to_string()),
            _ => Some("[issue]".to_string()),
        };
        map.entry(issue.reference_id.clone())
            .or_insert(FileTreeReferenceValidation {
                is_valid: false,
                suffix,
                severity: issue.severity,
            });
    }
    map
}

/// Recursively build a view model node from a graph node.
fn build_node_model(
    node: &ReferenceGraphNode,
    node_map: &HashMap<&str, &ReferenceGraphNode>,
    issue_map: &HashMap<String, FileTreeReferenceValidation>,
    follow: bool,
) -> FileTreeNode {
    let file_label = extract_file_label(&node.source);
    let file_icon_kind = if file_label.ends_with(".md") || file_label.ends_with(".dm") {
        FileTreeIconKind::Markdown
    } else {
        FileTreeIconKind::GenericFile
    };

    // Classify references into groups and inline summary
    let mut inline_summary = FileTreeInlineSummary::default();
    let mut group_map: HashMap<FileTreeReferenceGroupKind, Vec<FileTreeReferenceRow>> =
        HashMap::new();
    let mut transclusion_records: Vec<&ReferenceRecord> = Vec::new();

    for record in &node.local_references.records {
        // TOC-synthesized references only appear in follow (composed) mode.
        if !follow && record.attributes.get("toc_synthesized").is_some() {
            continue;
        }

        match classify_reference_group(record) {
            Some(group_kind) => {
                let display_target = record.target.raw().unwrap_or("").to_string();
                let validation = issue_map.get(&record.id).cloned();
                group_map
                    .entry(group_kind)
                    .or_default()
                    .push(FileTreeReferenceRow {
                        kind: record.kind,
                        display_target,
                        raw_reference_id: record.id.clone(),
                        validation,
                    });
            }
            None => {
                // Inline types or transclusions
                match record.kind {
                    ReferenceKind::InlineCss => inline_summary.inline_css_count += 1,
                    ReferenceKind::InlineScript => inline_summary.inline_script_count += 1,
                    ReferenceKind::MetaTag => inline_summary.meta_tag_count += 1,
                    ReferenceKind::Transclusion => transclusion_records.push(record),
                    _ => {}
                }
            }
        }
    }

    // Sort groups by fixed order, deduplicate rows by display_target
    let mut reference_groups: Vec<FileTreeReferenceGroup> = group_map
        .into_iter()
        .map(|(kind, rows)| {
            let mut seen = std::collections::HashSet::new();
            let deduped = rows
                .into_iter()
                .filter(|r| seen.insert(r.display_target.clone()))
                .collect();
            FileTreeReferenceGroup {
                kind,
                rows: deduped,
            }
        })
        .collect();
    reference_groups.sort_by_key(|g| g.kind.sort_order());

    // Build transclusion edges
    let mut transclusions = Vec::new();
    let mut children = Vec::new();

    for record in &transclusion_records {
        let trans_kind = syntax_to_transclusion_kind(record.origin.syntax);
        let is_literal = record.attributes.get("fm_literal").is_some();
        let display_target = if is_literal {
            String::new()
        } else {
            record.target.raw().unwrap_or("").to_string()
        };
        let is_local_path = matches!(&record.target, ReferenceTarget::LocalPath { .. });
        let followable =
            !is_literal && record.origin.syntax.is_followable_transclusion() && is_local_path;

        // Find the matching child insertion for caption context.
        // Match by reference_id when available (stable across frontmatter
        // entries that share the same line number), falling back to
        // directive_line for backwards compatibility.
        let insertion = node
            .child_insertions
            .iter()
            .find(|ins| match &ins.reference_id {
                Some(ref_id) => ref_id == &record.id,
                None => ins.directive_line == record.origin.line,
            });

        let caption = if is_literal {
            "includes static text".to_string()
        } else {
            insertion
                .map(|ins| transclusion_caption(&ins.context))
                .unwrap_or_else(|| default_caption_for_syntax(record.origin.syntax))
        };

        let child_node_id = insertion.map(|ins| ins.child_node_id.clone());

        let validation = issue_map.get(&record.id).cloned();

        transclusions.push(FileTreeTransclusionEdge {
            kind: trans_kind,
            display_target,
            caption,
            directive_line: record.origin.line,
            followable,
            child_node_id: child_node_id.clone(),
            validation,
        });

        // Follow into child if enabled
        if follow
            && followable
            && let Some(ref cid) = child_node_id
            && let Some(child_graph_node) = node_map.get(cid.as_ref())
        {
            children.push(build_node_model(
                child_graph_node,
                node_map,
                issue_map,
                follow,
            ));
        }
    }

    // Node-level validation
    let mut node_issues = 0;
    let mut has_errors = false;
    for record in &node.local_references.records {
        if let Some(v) = issue_map.get(&record.id) {
            node_issues += 1;
            if v.severity == ReferenceSeverity::Error {
                has_errors = true;
            }
        }
    }

    FileTreeNode {
        source: node.source.clone(),
        file_label,
        file_icon_kind,
        inline_summary,
        reference_groups,
        transclusions,
        children,
        validation: FileTreeNodeValidation {
            issues_count: node_issues,
            has_errors,
        },
    }
}

/// Extract a display label from a compose source.
fn extract_file_label(source: &ComposeSource) -> String {
    match source {
        ComposeSource::File(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| p.to_string_lossy().to_string()),
        ComposeSource::Url(u) => u.to_string(),
        ComposeSource::Unknown => "<unknown>".to_string(),
    }
}

/// Classify a reference record into a group kind, or None for inline/transclusion types.
fn classify_reference_group(record: &ReferenceRecord) -> Option<FileTreeReferenceGroupKind> {
    match record.kind {
        ReferenceKind::Hyperlink => match &record.target {
            ReferenceTarget::RemoteUrl { .. } => Some(FileTreeReferenceGroupKind::RemoteHyperlinks),
            ReferenceTarget::LocalPath { .. } | ReferenceTarget::Fragment { .. } => {
                Some(FileTreeReferenceGroupKind::LocalHyperlinks)
            }
            _ => None,
        },
        ReferenceKind::Image => Some(FileTreeReferenceGroupKind::Images),
        ReferenceKind::HtmlVideo | ReferenceKind::HtmlAudio | ReferenceKind::HtmlSource => {
            Some(FileTreeReferenceGroupKind::Media)
        }
        ReferenceKind::HtmlIframe => Some(FileTreeReferenceGroupKind::Iframes),
        ReferenceKind::CssImport => Some(FileTreeReferenceGroupKind::CssImports),
        ReferenceKind::ScriptImport => Some(FileTreeReferenceGroupKind::ScriptImports),
        ReferenceKind::FontImport => Some(FileTreeReferenceGroupKind::FontImports),
        // Inline types and transclusions are not rendered as dependency rows
        ReferenceKind::InlineCss
        | ReferenceKind::InlineScript
        | ReferenceKind::MetaTag
        | ReferenceKind::Transclusion => None,
    }
}

/// Map a `ReferenceSyntax` to a display-oriented `FileTreeTransclusionKind`.
fn syntax_to_transclusion_kind(syntax: ReferenceSyntax) -> FileTreeTransclusionKind {
    match syntax {
        ReferenceSyntax::DirectiveFile => FileTreeTransclusionKind::File,
        ReferenceSyntax::DirectiveCode => FileTreeTransclusionKind::Code,
        ReferenceSyntax::DirectiveUrl => FileTreeTransclusionKind::Url,
        ReferenceSyntax::DirectiveTocLinking => FileTreeTransclusionKind::TocLinking,
        ReferenceSyntax::DirectiveFileLinks => FileTreeTransclusionKind::FileLinks,
        ReferenceSyntax::FrontmatterPrologue => FileTreeTransclusionKind::Prologue,
        ReferenceSyntax::FrontmatterEpilogue => FileTreeTransclusionKind::Epilogue,
        _ => FileTreeTransclusionKind::File,
    }
}

/// Generate a transclusion caption from insertion context.
pub fn transclusion_caption(context: &ReferenceInsertionContext) -> String {
    let section = context
        .section_heading_text
        .as_deref()
        .map(|h| {
            let hashes = "#".repeat(
                context
                    .section_heading_level
                    .unwrap_or(HeadingLevel::H2)
                    .hash_count(),
            );
            format!(" into the '{hashes} {h}' section")
        })
        .unwrap_or_default();

    match context.directive_kind {
        Some(ReferenceSyntax::DirectiveFile)
        | Some(ReferenceSyntax::FrontmatterPrologue)
        | Some(ReferenceSyntax::FrontmatterEpilogue) => {
            format!("inserted{section}")
        }
        Some(ReferenceSyntax::DirectiveTocLinking) => {
            format!("TOC elements linked{section}")
        }
        Some(ReferenceSyntax::DirectiveFileLinks) => {
            format!("file links rendered{section}")
        }
        Some(ReferenceSyntax::DirectiveCode) => {
            format!("inserted code{section}")
        }
        Some(ReferenceSyntax::DirectiveUrl) => {
            format!("transcluded from URL{section}")
        }
        _ => "inserted".to_string(),
    }
}

/// Fallback caption when no insertion context is available.
///
/// Used when the target document couldn't be loaded (e.g. missing file),
/// so no `ReferenceInsertion` was created in the graph builder.
fn default_caption_for_syntax(syntax: ReferenceSyntax) -> String {
    match syntax {
        ReferenceSyntax::DirectiveTocLinking => "TOC elements linked".to_string(),
        ReferenceSyntax::DirectiveFileLinks => "file links rendered".to_string(),
        ReferenceSyntax::DirectiveCode => "inserted code".to_string(),
        ReferenceSyntax::DirectiveUrl => "transcluded from URL".to_string(),
        ReferenceSyntax::DirectiveFile
        | ReferenceSyntax::FrontmatterPrologue
        | ReferenceSyntax::FrontmatterEpilogue => "inserted".to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_summary_empty() {
        let s = FileTreeInlineSummary::default();
        assert!(s.is_empty());
        assert!(s.to_display_string().is_none());
    }

    #[test]
    fn inline_summary_singular() {
        let s = FileTreeInlineSummary {
            inline_css_count: 1,
            inline_script_count: 0,
            meta_tag_count: 1,
        };
        let display = s.to_display_string().unwrap();
        assert_eq!(display, "(1 inline CSS block, 1 meta tag)");
    }

    #[test]
    fn inline_summary_plural() {
        let s = FileTreeInlineSummary {
            inline_css_count: 2,
            inline_script_count: 3,
            meta_tag_count: 0,
        };
        let display = s.to_display_string().unwrap();
        assert_eq!(display, "(2 inline CSS blocks, 3 inline scripts)");
    }

    #[test]
    fn caption_file_with_section() {
        let ctx = ReferenceInsertionContext {
            directive_kind: Some(ReferenceSyntax::DirectiveFile),
            section_heading_text: Some("Intro".to_string()),
            section_heading_level: Some(HeadingLevel::H2),
        };
        assert_eq!(
            transclusion_caption(&ctx),
            "inserted into the '## Intro' section"
        );
    }

    #[test]
    fn caption_toc_linking() {
        let ctx = ReferenceInsertionContext {
            directive_kind: Some(ReferenceSyntax::DirectiveTocLinking),
            section_heading_text: Some("Links".to_string()),
            section_heading_level: Some(HeadingLevel::H2),
        };
        assert_eq!(
            transclusion_caption(&ctx),
            "TOC elements linked into the '## Links' section"
        );
    }

    #[test]
    fn caption_no_section() {
        let ctx = ReferenceInsertionContext {
            directive_kind: Some(ReferenceSyntax::DirectiveFile),
            section_heading_text: None,
            section_heading_level: None,
        };
        assert_eq!(transclusion_caption(&ctx), "inserted");
    }

    #[test]
    fn caption_url() {
        let ctx = ReferenceInsertionContext {
            directive_kind: Some(ReferenceSyntax::DirectiveUrl),
            section_heading_text: Some("Summary".to_string()),
            section_heading_level: Some(HeadingLevel::H2),
        };
        assert_eq!(
            transclusion_caption(&ctx),
            "transcluded from URL into the '## Summary' section"
        );
    }

    #[test]
    fn group_sort_order() {
        use FileTreeReferenceGroupKind::*;
        let groups = vec![
            FontImports,
            RemoteHyperlinks,
            ScriptImports,
            Images,
            LocalHyperlinks,
            CssImports,
        ];
        let mut sorted = groups.clone();
        sorted.sort_by_key(|g| g.sort_order());
        assert_eq!(
            sorted,
            vec![
                RemoteHyperlinks,
                LocalHyperlinks,
                Images,
                CssImports,
                ScriptImports,
                FontImports
            ]
        );
    }

    #[test]
    fn file_label_from_path() {
        let source = ComposeSource::File(std::path::PathBuf::from("/docs/readme.md"));
        assert_eq!(extract_file_label(&source), "readme.md");
    }

    #[test]
    fn file_label_from_url() {
        let source = ComposeSource::Url(url::Url::parse("https://example.com/doc.md").unwrap());
        assert_eq!(extract_file_label(&source), "https://example.com/doc.md");
    }

    #[test]
    fn file_label_unknown() {
        assert_eq!(extract_file_label(&ComposeSource::Unknown), "<unknown>");
    }

    #[test]
    fn followability() {
        assert!(ReferenceSyntax::DirectiveFile.is_followable_transclusion());
        assert!(ReferenceSyntax::DirectiveTocLinking.is_followable_transclusion());
        assert!(ReferenceSyntax::FrontmatterPrologue.is_followable_transclusion());
        assert!(ReferenceSyntax::FrontmatterEpilogue.is_followable_transclusion());
        assert!(!ReferenceSyntax::DirectiveUrl.is_followable_transclusion());
        assert!(!ReferenceSyntax::DirectiveCode.is_followable_transclusion());
        assert!(!ReferenceSyntax::MarkdownLink.is_followable_transclusion());
    }

    #[test]
    fn caption_uses_h3_heading_level() {
        let ctx = ReferenceInsertionContext {
            directive_kind: Some(ReferenceSyntax::DirectiveFile),
            section_heading_text: Some("Details".to_string()),
            section_heading_level: Some(HeadingLevel::H3),
        };
        assert_eq!(
            transclusion_caption(&ctx),
            "inserted into the '### Details' section"
        );
    }

    #[test]
    fn caption_uses_h4_heading_level() {
        let ctx = ReferenceInsertionContext {
            directive_kind: Some(ReferenceSyntax::DirectiveFile),
            section_heading_text: Some("Sub".to_string()),
            section_heading_level: Some(HeadingLevel::H4),
        };
        assert_eq!(
            transclusion_caption(&ctx),
            "inserted into the '#### Sub' section"
        );
    }

    #[test]
    fn caption_heading_level_defaults_to_h2() {
        let ctx = ReferenceInsertionContext {
            directive_kind: Some(ReferenceSyntax::DirectiveFile),
            section_heading_text: Some("Test".to_string()),
            section_heading_level: None,
        };
        assert_eq!(
            transclusion_caption(&ctx),
            "inserted into the '## Test' section"
        );
    }

    #[test]
    fn reference_id_matching_over_directive_line() {
        use crate::markdown::Markdown;
        use crate::markdown::reference::provenance::{
            ReferenceDependencyManifest, ReferenceGraphMode,
        };
        use crate::markdown::reference::snapshot::PreparedHeadingSnapshot;
        use crate::markdown::reference::types::{
            ReferenceGraph, ReferenceGraphNode, ReferenceGraphOptions, ReferenceInsertion,
            ReferenceInsertionContext, ReferenceOrigin, ReferenceRecord, ReferenceSet,
            ReferenceTarget,
        };

        // Simulate two prologues: both have directive_line=0 but different reference_ids
        let root = ReferenceGraphNode {
            node_id: "root".into(),
            source: ComposeSource::Unknown,
            local_references: ReferenceSet {
                records: vec![
                    ReferenceRecord {
                        id: "ref_a".into(),
                        kind: ReferenceKind::Transclusion,
                        target: ReferenceTarget::LocalPath { raw: "a.md".into() },
                        origin: ReferenceOrigin {
                            source: ComposeSource::Unknown,
                            line: 0,
                            span: 0..0,
                            syntax: ReferenceSyntax::FrontmatterPrologue,
                        },
                        attributes: serde_json::Map::new(),
                    },
                    ReferenceRecord {
                        id: "ref_b".into(),
                        kind: ReferenceKind::Transclusion,
                        target: ReferenceTarget::LocalPath { raw: "b.md".into() },
                        origin: ReferenceOrigin {
                            source: ComposeSource::Unknown,
                            line: 0,
                            span: 0..0,
                            syntax: ReferenceSyntax::FrontmatterPrologue,
                        },
                        attributes: serde_json::Map::new(),
                    },
                ],
            },
            child_insertions: vec![
                ReferenceInsertion {
                    child_node_id: "child_a".into(),
                    directive_line: 0,
                    insertion_order: 0,
                    reference_id: Some("ref_a".into()),
                    context: ReferenceInsertionContext {
                        directive_kind: Some(ReferenceSyntax::FrontmatterPrologue),
                        section_heading_text: None,
                        section_heading_level: None,
                    },
                },
                ReferenceInsertion {
                    child_node_id: "child_b".into(),
                    directive_line: 0,
                    insertion_order: 1,
                    reference_id: Some("ref_b".into()),
                    context: ReferenceInsertionContext {
                        directive_kind: Some(ReferenceSyntax::FrontmatterPrologue),
                        section_heading_text: None,
                        section_heading_level: None,
                    },
                },
            ],
        };

        let child_a = ReferenceGraphNode {
            node_id: "child_a".into(),
            source: ComposeSource::File(std::path::PathBuf::from("a.md")),
            local_references: ReferenceSet {
                records: vec![ReferenceRecord {
                    id: "a_link".into(),
                    kind: ReferenceKind::Hyperlink,
                    target: ReferenceTarget::RemoteUrl {
                        raw: "https://a.example.com".into(),
                    },
                    origin: ReferenceOrigin {
                        source: ComposeSource::Unknown,
                        line: 1,
                        span: 0..10,
                        syntax: ReferenceSyntax::MarkdownLink,
                    },
                    attributes: serde_json::Map::new(),
                }],
            },
            child_insertions: vec![],
        };

        let child_b = ReferenceGraphNode {
            node_id: "child_b".into(),
            source: ComposeSource::File(std::path::PathBuf::from("b.md")),
            local_references: ReferenceSet {
                records: vec![ReferenceRecord {
                    id: "b_link".into(),
                    kind: ReferenceKind::Hyperlink,
                    target: ReferenceTarget::RemoteUrl {
                        raw: "https://b.example.com".into(),
                    },
                    origin: ReferenceOrigin {
                        source: ComposeSource::Unknown,
                        line: 1,
                        span: 0..10,
                        syntax: ReferenceSyntax::MarkdownLink,
                    },
                    attributes: serde_json::Map::new(),
                }],
            },
            child_insertions: vec![],
        };

        let graph = ReferenceGraph::from_build(
            &Markdown::new(""),
            &ReferenceGraphOptions::default(),
            ReferenceGraphMode::Full,
            root,
            vec![child_a, child_b],
            ReferenceDependencyManifest::default(),
            PreparedHeadingSnapshot::default(),
        );

        let model = build_file_tree_model(&graph, None, true);

        // Both children should appear (not a duplicated first child)
        assert_eq!(
            model.root.children.len(),
            2,
            "expected 2 children, not duplicates of first"
        );
        assert_eq!(model.root.children[0].file_label, "a.md");
        assert_eq!(model.root.children[1].file_label, "b.md");
    }

    #[test]
    fn epilogue_matches_by_reference_id() {
        use crate::markdown::Markdown;
        use crate::markdown::reference::provenance::{
            ReferenceDependencyManifest, ReferenceGraphMode,
        };
        use crate::markdown::reference::snapshot::PreparedHeadingSnapshot;
        use crate::markdown::reference::types::{
            ReferenceGraph, ReferenceGraphNode, ReferenceGraphOptions, ReferenceInsertion,
            ReferenceInsertionContext, ReferenceOrigin, ReferenceRecord, ReferenceSet,
            ReferenceTarget,
        };

        let root = ReferenceGraphNode {
            node_id: "root".into(),
            source: ComposeSource::Unknown,
            local_references: ReferenceSet {
                records: vec![ReferenceRecord {
                    id: "epi_ref".into(),
                    kind: ReferenceKind::Transclusion,
                    target: ReferenceTarget::LocalPath {
                        raw: "epilogue.md".into(),
                    },
                    origin: ReferenceOrigin {
                        source: ComposeSource::Unknown,
                        line: usize::MAX,
                        span: 0..0,
                        syntax: ReferenceSyntax::FrontmatterEpilogue,
                    },
                    attributes: serde_json::Map::new(),
                }],
            },
            child_insertions: vec![ReferenceInsertion {
                child_node_id: "child_epi".into(),
                directive_line: usize::MAX,
                insertion_order: 0,
                reference_id: Some("epi_ref".into()),
                context: ReferenceInsertionContext {
                    directive_kind: Some(ReferenceSyntax::FrontmatterEpilogue),
                    section_heading_text: None,
                    section_heading_level: None,
                },
            }],
        };

        let child = ReferenceGraphNode {
            node_id: "child_epi".into(),
            source: ComposeSource::File(std::path::PathBuf::from("epilogue.md")),
            local_references: ReferenceSet::default(),
            child_insertions: vec![],
        };

        let graph = ReferenceGraph::from_build(
            &Markdown::new(""),
            &ReferenceGraphOptions::default(),
            ReferenceGraphMode::Full,
            root,
            vec![child],
            ReferenceDependencyManifest::default(),
            PreparedHeadingSnapshot::default(),
        );

        let model = build_file_tree_model(&graph, None, true);

        // The epilogue should now be followed as a child
        assert_eq!(model.root.children.len(), 1);
        assert_eq!(model.root.children[0].file_label, "epilogue.md");
        // The transclusion edge should have a child_node_id
        assert!(model.root.transclusions[0].child_node_id.is_some());
    }
}
