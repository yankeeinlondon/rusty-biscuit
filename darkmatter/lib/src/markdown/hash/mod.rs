//! Markdown-aware hashing.
//!
//! Two layers live here:
//!
//! - The historical [`Markdown::hash`] / [`Markdown::hash_frontmatter`] /
//!   [`Markdown::hash_body`] methods, preserved verbatim for the directory
//!   aggregate mode and any external callers.
//! - The explicit, option-driven model ([`MdHashKind`], [`MdHashOptions`],
//!   [`ComputedHash`]) that the new `md hash` surface builds on. The library
//!   reads no environment; every input is passed in by the caller.
//!
//! Design: `darkmatter/features/2026-05-28-darkmatter-hashing/design.md`.

mod compare;
mod compute;
mod explain;
mod kind;
mod options;
mod save;
mod stored;
mod write;

pub use compare::{ComparisonDetail, HashComparison, IgnorePolicyAdvisory};
pub use compute::{ComputedHash, DetailedValue, FmHashPair, SectionTuple};
pub use explain::HashExplanation;
pub use kind::{KindRelation, MdHashKind, ParseMdHashKindError, select_kind};
pub use options::{DEFAULT_HASH_PROPERTY, LAST_UPDATED_KEY, MdHashOptions};
pub use save::SaveDecision;
pub use stored::{StoredHash, StoredHashValue};
pub use write::apply_hash_save_text;

use crate::markdown::Markdown;
use crate::markdown::FrontmatterMap;
use biscuit_file::serde_yaml_ng;
use biscuit_hash::{HashVariant, xx_hash, xx_hash_variant};

impl Markdown {
    /// Hash a markdown document's frontmatter and/or body.
    pub fn hash(&self, body_only: bool, frontmatter_only: bool, strict: bool) -> String {
        if body_only {
            format!("{:016x}", self.hash_body(strict))
        } else if frontmatter_only {
            format!("{:016x}", self.hash_frontmatter(strict))
        } else {
            format!(
                "{:016x}-{:016x}",
                self.hash_frontmatter(strict),
                self.hash_body(strict)
            )
        }
    }

    /// Hash a markdown document's frontmatter.
    pub fn hash_frontmatter(&self, strict: bool) -> u64 {
        hash_frontmatter_map(self.frontmatter().as_map(), strict)
    }

    /// Hash a markdown document's body/prose content.
    pub fn hash_body(&self, strict: bool) -> u64 {
        hash_content_with_policy(self.content(), strict)
    }
}

/// Renders a hash component as the canonical 16-digit lowercase hex string.
pub(super) fn hex(value: u64) -> String {
    format!("{value:016x}")
}

/// Hashes a frontmatter map's keys *and* values.
///
/// Empty or absent frontmatter hashes exactly as empty frontmatter
/// (`xx_hash("")`), so a document without a frontmatter block and a document
/// with an empty block produce the same value.
///
/// ## Notes
///
/// Non-strict canonicalization sorts keys and renders each entry as
/// `key:<json-value>` joined with a literal `\n` two-char sequence. This is the
/// historical shape and must stay stable so the default `md hash` output does
/// not drift for documents that carry no ignored keys.
pub(super) fn hash_frontmatter_map(map: &FrontmatterMap, strict: bool) -> u64 {
    if map.is_empty() {
        return xx_hash("");
    }
    if strict {
        let yaml = serde_yaml_ng::to_string(map).unwrap_or_default();
        xx_hash(&yaml)
    } else {
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        let canonical: String = keys
            .iter()
            .map(|k| {
                let v = &map[*k];
                format!("{}:{}", k, serde_json::to_string(v).unwrap_or_default())
            })
            .collect::<Vec<_>>()
            .join("\\n");
        xx_hash(&canonical)
    }
}

/// Hashes a frontmatter map's keys only, ignoring values.
///
/// Mirrors [`hash_frontmatter_map`]'s key-ordering policy so the keys-only
/// structure hash stays consistent with the keys+values hash: non-strict sorts
/// the keys, while strict preserves the document's insertion order so that
/// `--strict` performs no key reordering.
pub(super) fn hash_frontmatter_keys(map: &FrontmatterMap, strict: bool) -> u64 {
    if map.is_empty() {
        return xx_hash("");
    }
    let mut keys: Vec<&String> = map.keys().collect();
    if !strict {
        keys.sort();
    }
    let canonical = keys
        .iter()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join("\\n");
    xx_hash(&canonical)
}

/// Hashes content under the body whitespace policy.
///
/// Strict hashes the bytes verbatim; non-strict ignores leading/trailing
/// whitespace and blank lines so whitespace-only edits do not change the
/// semantic hash. Used for the body, the detailed preamble, and detailed
/// section content so all three share one whitespace policy.
pub(super) fn hash_content_with_policy(content: &str, strict: bool) -> u64 {
    if strict {
        xx_hash(content)
    } else {
        xx_hash_variant(
            content,
            vec![
                HashVariant::LeadingWhitespace,
                HashVariant::TrailingWhitespace,
                HashVariant::BlankLine,
            ],
        )
    }
}
