//! Layer-3 directive overlay: request-time scanning of Darkmatter DSL
//! directives.
//!
//! Every function here is a thin, side-effect-free wrapper over the Phase-8
//! library scanners ([`scan_darkmatter_directives`], [`scan_darkmatter_blocks`],
//! [`scan_disclosures`]) plus a small static catalog that supplies the hover
//! prose, completion labels, and option grammars the library scanners do not
//! carry. Nothing here reads the filesystem, spawns a process, or resolves a
//! target — the DSL provider does that against the graph.

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::directives_api::{
    BlockScanError, DarkmatterBlock, DirectiveKind, ParsedDirective, scan_darkmatter_blocks,
    scan_darkmatter_directives,
};
use darkmatter::markdown::render_tree::disclosure_scan::{DisclosureParse, scan_disclosures};

/// One entry in the directive-name completion and hover catalog.
pub struct DirectiveInfo {
    /// Canonical keyword including the leading `::`.
    pub keyword: &'static str,
    /// The library directive kind.
    pub kind: DirectiveKind,
    /// One-line semantics shown in completion detail and hover.
    pub summary: &'static str,
}

/// Every recognized directive keyword, with its one-line semantics. Longest
/// keywords first so name completion offers `::file-links` alongside `::file`.
pub const DIRECTIVE_CATALOG: &[DirectiveInfo] = &[
    DirectiveInfo {
        keyword: "::file",
        kind: DirectiveKind::File,
        summary: "Transclude another Markdown file's content in place.",
    },
    DirectiveInfo {
        keyword: "::code",
        kind: DirectiveKind::Code,
        summary: "Transclude a source file as a fenced code block.",
    },
    DirectiveInfo {
        keyword: "::url",
        kind: DirectiveKind::Url,
        summary: "Transclude remote Markdown from an allowed host.",
    },
    DirectiveInfo {
        keyword: "::file-links",
        kind: DirectiveKind::FileLinks,
        summary: "Insert a Markdown link list for a directory or glob.",
    },
    DirectiveInfo {
        keyword: "::toc-linking",
        kind: DirectiveKind::TocLinking,
        summary: "Cross-link a table of contents between documents.",
    },
    DirectiveInfo {
        keyword: "::shell",
        kind: DirectiveKind::Shell,
        summary: "Run a shell command and insert its output (policy-gated).",
    },
    DirectiveInfo {
        keyword: "::block",
        kind: DirectiveKind::Block,
        summary: "Open a conditional page block; `::end-block` closes it.",
    },
    DirectiveInfo {
        keyword: "::shell-block",
        kind: DirectiveKind::ShellBlock,
        summary: "Open a shell block; `::end-block` closes it (policy-gated).",
    },
    DirectiveInfo {
        keyword: "::end-block",
        kind: DirectiveKind::EndBlock,
        summary: "Close a `::block` or `::shell-block`.",
    },
    DirectiveInfo {
        keyword: "::disclosure",
        kind: DirectiveKind::Disclosure,
        summary: "Open a collapsible disclosure; `::details` divides summary/body.",
    },
    DirectiveInfo {
        keyword: "::details",
        kind: DirectiveKind::Details,
        summary: "Divide a disclosure's summary from its disclosed body.",
    },
    DirectiveInfo {
        keyword: "::end-disclosure",
        kind: DirectiveKind::EndDisclosure,
        summary: "Close a `::disclosure` block.",
    },
    DirectiveInfo {
        keyword: "::map",
        kind: DirectiveKind::Map,
        summary: "Repeat the body once per source entry, bound to the mapped value; `::end-map` closes it.",
    },
    DirectiveInfo {
        keyword: "::end-map",
        kind: DirectiveKind::EndMap,
        summary: "Close a `::map` block.",
    },
    DirectiveInfo {
        keyword: "::filter",
        kind: DirectiveKind::Filter,
        summary: "Repeat the body once per retained source entry, bound to the original item; `::end-filter` closes it.",
    },
    DirectiveInfo {
        keyword: "::end-filter",
        kind: DirectiveKind::EndFilter,
        summary: "Close a `::filter` block.",
    },
];

/// The catalog entry for a directive kind.
pub fn info_for(kind: DirectiveKind) -> Option<&'static DirectiveInfo> {
    DIRECTIVE_CATALOG.iter().find(|info| info.kind == kind)
}

/// Whether a directive family transcludes a workspace-local file target
/// (`::file` / `::code`).
pub fn is_transclusion(kind: DirectiveKind) -> bool {
    matches!(kind, DirectiveKind::File | DirectiveKind::Code)
}

/// Whether a directive's positional target names local files that `md
/// compose` resolves: the transclusions plus `::toc-linking`'s fallback
/// chain (read through `ParsedDirective::target_chain`).
/// `::file-links` names a directory or glob, not one file.
pub fn has_file_target(kind: DirectiveKind) -> bool {
    is_transclusion(kind) || kind == DirectiveKind::TocLinking
}

/// The directive whose full line span contains `offset`, if any.
pub fn directive_at(source: &str, offset: usize) -> Option<ParsedDirective> {
    scan_darkmatter_directives(source)
        .into_iter()
        .find(|directive| directive.span.start <= offset && offset <= directive.span.end)
}

/// All directives in `source`, in document order.
pub fn directives(source: &str) -> Vec<ParsedDirective> {
    scan_darkmatter_directives(source)
}

/// Paired `::block` / `::shell-block` regions, or the pairing error.
pub fn blocks(source: &str) -> Result<Vec<DarkmatterBlock>, BlockScanError> {
    scan_darkmatter_blocks(source)
}

/// Disclosure triples, or the structural malformation error.
pub fn disclosures(source: &str) -> Result<Vec<DisclosureParse>, MarkdownError> {
    scan_disclosures(source)
}

/// The partial directive keyword (sans `::`) typed on a line that begins with
/// `::`, when the cursor is still inside that first token.
pub fn name_partial(line_prefix: &str) -> Option<&str> {
    let trimmed = line_prefix.trim_start();
    let rest = trimmed.strip_prefix("::")?;
    (!rest.contains(char::is_whitespace)).then_some(rest)
}

/// The named options of `::map` / `::filter` openers, in the order the
/// specification lists them.
pub const COLLECTION_OPTIONS: &[&str] = &["when", "filter", "sort", "sort-order", "while", "until"];

/// Whether `kind` opens or closes a `::map` / `::filter` region.
pub fn is_collection(kind: DirectiveKind) -> bool {
    matches!(
        kind,
        DirectiveKind::Map | DirectiveKind::EndMap | DirectiveKind::Filter | DirectiveKind::EndFilter
    )
}

/// Recognized option keys for a directive family — the completion set and the
/// allow-list for malformed-option diagnostics. Families not listed accept any
/// option (empty slice → no validation).
pub fn option_keys(kind: DirectiveKind) -> &'static [&'static str] {
    match kind {
        DirectiveKind::File | DirectiveKind::Code | DirectiveKind::Url => {
            &["when", "quotation", "disclosure", "replace", "exclude", "set"]
        }
        DirectiveKind::Block | DirectiveKind::ShellBlock => &["when"],
        DirectiveKind::Map | DirectiveKind::Filter => COLLECTION_OPTIONS,
        _ => &[],
    }
}

/// Enum-like completion values for a boolean-valued option key.
pub fn option_values(key: &str) -> Option<&'static [&'static str]> {
    match key {
        "quotation" | "disclosure" | "replace" => Some(&["true", "false"]),
        "sort-order" => Some(&["asc", "desc"]),
        _ => None,
    }
}

/// Whether `key` is accepted by `kind` (dotted `set.<name>` and `--flag` forms
/// pass permissively; empty allow-lists accept everything).
pub fn option_recognized(kind: DirectiveKind, key: &str) -> bool {
    // The collection grammar validates its own header (unknown, duplicate,
    // and bare options) and reports through `dm.collection.malformed`.
    if matches!(kind, DirectiveKind::Map | DirectiveKind::Filter) {
        return true;
    }
    let allowed = option_keys(kind);
    if allowed.is_empty() {
        return true;
    }
    if key.starts_with("--") || key.starts_with("set.") {
        return true;
    }
    allowed.contains(&key)
}

/// Whether a raw token is a `::` directive-shaped word (`::` then an ASCII
/// letter). Used by the unknown-directive scan to tell a directive attempt from
/// ordinary `::` text.
pub fn looks_like_directive(trimmed: &str) -> bool {
    trimmed
        .strip_prefix("::")
        .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_alphabetic()))
}

/// The keyword token of a directive-shaped line (`::foo bar` → `::foo`).
pub fn keyword_token(trimmed: &str) -> &str {
    let end = trimmed
        .char_indices()
        .find(|(_, c)| c.is_whitespace())
        .map(|(index, _)| index)
        .unwrap_or(trimmed.len());
    &trimmed[..end]
}

/// Whether a keyword token names a recognized directive.
pub fn is_known_keyword(keyword: &str) -> bool {
    DIRECTIVE_CATALOG.iter().any(|info| info.keyword == keyword)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directive_at_finds_containing_line() {
        let source = "text\n::file ./a.md\nmore\n";
        let offset = source.find("./a.md").unwrap();
        let directive = directive_at(source, offset).unwrap();
        assert_eq!(directive.kind, DirectiveKind::File);
    }

    #[test]
    fn test_name_partial() {
        assert_eq!(name_partial("::fi"), Some("fi"));
        assert_eq!(name_partial("  ::end-"), Some("end-"));
        assert_eq!(name_partial("::file ./a"), None);
        assert_eq!(name_partial("text"), None);
    }

    #[test]
    fn test_option_recognition() {
        assert!(option_recognized(DirectiveKind::File, "when"));
        assert!(option_recognized(DirectiveKind::File, "set.name"));
        assert!(!option_recognized(DirectiveKind::File, "bogus"));
        // Families with no allow-list accept anything.
        assert!(option_recognized(DirectiveKind::Shell, "anything"));
        // The collection grammar reports its own unknown options.
        assert!(option_recognized(DirectiveKind::Map, "bogus"));
    }

    #[test]
    fn test_keyword_token_and_known() {
        assert_eq!(keyword_token("::file ./a.md"), "::file");
        assert!(is_known_keyword("::file"));
        assert!(!is_known_keyword("::frobnicate"));
        for keyword in ["::map", "::end-map", "::filter", "::end-filter"] {
            assert!(is_known_keyword(keyword), "{keyword}");
        }
        assert!(looks_like_directive("::frobnicate x"));
        assert!(!looks_like_directive(":: not"));
    }

    /// Every directive kind has exactly one catalog row spelled as the
    /// library spells it; the exhaustive `match` fails to compile when the
    /// library adds a kind this inventory does not list.
    #[test]
    fn catalog_covers_every_library_directive_kind() {
        const fn listed(kind: DirectiveKind) -> bool {
            match kind {
                DirectiveKind::File
                | DirectiveKind::Code
                | DirectiveKind::Url
                | DirectiveKind::FileLinks
                | DirectiveKind::TocLinking
                | DirectiveKind::Shell
                | DirectiveKind::Block
                | DirectiveKind::ShellBlock
                | DirectiveKind::EndBlock
                | DirectiveKind::Disclosure
                | DirectiveKind::Details
                | DirectiveKind::EndDisclosure
                | DirectiveKind::Map
                | DirectiveKind::EndMap
                | DirectiveKind::Filter
                | DirectiveKind::EndFilter => true,
            }
        }
        assert!(listed(DirectiveKind::Map));
        assert_eq!(DIRECTIVE_CATALOG.len(), 16);
        for info in DIRECTIVE_CATALOG {
            assert_eq!(info.keyword, info.kind.keyword());
            assert_eq!(DIRECTIVE_CATALOG.iter().filter(|other| other.kind == info.kind).count(), 1);
        }
    }
}
