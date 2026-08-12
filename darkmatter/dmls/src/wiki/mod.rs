//! Wiki-link support (Layer 1 / R-8).
//!
//! DMLS resolves `[[wiki]]` links against the workspace with the same
//! cross-platform guarantee everywhere: a vault resolves identically on macOS,
//! Windows, and Linux. Matching is case-sensitive on every platform, percent
//! escapes are decoded once before NFC normalization, and Markdown extensions
//! are elided only on a target's final segment.
//!
//! The module is three pure layers so each rule is unit-testable in isolation:
//!
//! - [`scanner`] — lexical `[[…]]` extraction (forms, escapes, unsupported).
//! - [`logical_path`] — canonicalization (NFC, extension elision, percent
//!   decoding, portability keys).
//! - [`resolve`] — file-target classification, matching, and ranking.
//!
//! The graph builder ([`crate::graph`]) turns scanned links into wiki-link
//! nodes with resolved `references` edges; the [`crate::providers::wiki`]
//! provider answers navigation, hover, completion, and diagnostics from them.

pub mod logical_path;
pub mod resolve;
pub mod scanner;

pub use logical_path::{
    canonical_segments, decode_percent_once, elide_markdown_extension, has_markdown_extension, nfc,
    portability_key,
};
pub use resolve::{
    Match, ParseOutcome, ParsedTarget, TargetKind, WikiDoc, ends_with_target, parse_file_target,
    resolve_file, shortest_unique_suffix,
};
pub use scanner::{ScannedWikiLink, scan_wiki_links};
