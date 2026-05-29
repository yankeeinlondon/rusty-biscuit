//! Computed hash shapes and the `Markdown::compute_hash` entry point.

use std::collections::BTreeSet;

use biscuit_hash::xx_hash;
use serde::de::Deserializer;
use serde::ser::{SerializeTuple, Serializer};
use serde::{Deserialize, Serialize};

use super::kind::MdHashKind;
use super::options::MdHashOptions;
use super::{hash_content_with_policy, hash_frontmatter_keys, hash_frontmatter_map, hex};
use crate::markdown::FrontmatterMap;
use crate::markdown::Markdown;

/// The frontmatter component of a `structured` or `detailed` hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FmHashPair {
    /// Hash of frontmatter keys *and* values.
    pub fm: String,
    /// Hash of frontmatter keys only.
    pub keys: String,
}

/// One section of a `detailed` hash, persisted as a `[level, "heading", hash]`
/// tuple. Custom serde keeps the on-disk YAML/JSON shape an array while the
/// in-memory form stays readable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionTuple {
    /// Heading level (1-6).
    pub level: u8,
    /// Literal heading text (without the leading `#` markers).
    pub heading: String,
    /// Hash of the section's own content (excludes child sections).
    pub content_hash: String,
}

impl Serialize for SectionTuple {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut tuple = serializer.serialize_tuple(3)?;
        tuple.serialize_element(&self.level)?;
        tuple.serialize_element(&self.heading)?;
        tuple.serialize_element(&self.content_hash)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for SectionTuple {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (level, heading, content_hash) = <(u8, String, String)>::deserialize(deserializer)?;
        Ok(Self {
            level,
            heading,
            content_hash,
        })
    }
}

/// The persisted nested shape of a `detailed` hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailedValue {
    /// Frontmatter keys+values and keys-only hashes.
    pub frontmatter: FmHashPair,
    /// Hash of the preamble (content before the first heading), or `None` when
    /// there is no preamble content.
    pub preamble: Option<String>,
    /// Sections in document order.
    pub sections: Vec<SectionTuple>,
}

/// A freshly computed, kind-tagged hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputedHash {
    /// Frontmatter-only hash.
    Fm(String),
    /// Body-only hash.
    Body(String),
    /// `{fm}-{body}`.
    Simple {
        /// Frontmatter hash.
        fm: String,
        /// Body hash.
        body: String,
    },
    /// `{fm}-{fm_keys}-{body}-{body_structure}`.
    Structured {
        /// Frontmatter keys+values hash.
        fm: String,
        /// Frontmatter keys-only hash.
        fm_keys: String,
        /// Body hash.
        body: String,
        /// Body heading-structure hash.
        body_structure: String,
    },
    /// Nested detailed shape.
    Detailed(DetailedValue),
}

impl ComputedHash {
    /// The kind this value was computed for.
    pub fn kind(&self) -> MdHashKind {
        match self {
            ComputedHash::Fm(_) => MdHashKind::Fm,
            ComputedHash::Body(_) => MdHashKind::Body,
            ComputedHash::Simple { .. } => MdHashKind::Simple,
            ComputedHash::Structured { .. } => MdHashKind::Structured,
            ComputedHash::Detailed(_) => MdHashKind::Detailed,
        }
    }

    /// The flat string form for non-detailed kinds, or `None` for `detailed`.
    ///
    /// This is the value the default `md hash` prints and the string stored in
    /// a flat (non-`detailed`) `hash` property.
    pub fn flat_string(&self) -> Option<String> {
        match self {
            ComputedHash::Fm(hash) | ComputedHash::Body(hash) => Some(hash.clone()),
            ComputedHash::Simple { fm, body } => Some(format!("{fm}-{body}")),
            ComputedHash::Structured {
                fm,
                fm_keys,
                body,
                body_structure,
            } => Some(format!("{fm}-{fm_keys}-{body}-{body_structure}")),
            ComputedHash::Detailed(_) => None,
        }
    }
}

impl Markdown {
    /// Computes a hash of the requested [`MdHashKind`] under the given options.
    ///
    /// The active hash property and `last_updated` (plus any extra ignored
    /// properties) are filtered from the frontmatter before hashing, so a hash
    /// is stable across save round-trips and never hashes itself. The source
    /// document is never mutated.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::hash::{ComputedHash, MdHashKind, MdHashOptions};
    ///
    /// let md: Markdown = "# Title\n\nBody".into();
    /// let computed = md.compute_hash(MdHashKind::Simple, &MdHashOptions::default());
    /// assert!(matches!(computed, ComputedHash::Simple { .. }));
    /// ```
    pub fn compute_hash(&self, kind: MdHashKind, options: &MdHashOptions) -> ComputedHash {
        let ignore = options.ignore_set();
        let filtered = filtered_frontmatter(self, &ignore);
        let strict = options.strict;

        match kind {
            MdHashKind::Fm => ComputedHash::Fm(hex(hash_frontmatter_map(&filtered, strict))),
            MdHashKind::Body => {
                ComputedHash::Body(hex(hash_content_with_policy(self.content(), strict)))
            }
            MdHashKind::Simple => ComputedHash::Simple {
                fm: hex(hash_frontmatter_map(&filtered, strict)),
                body: hex(hash_content_with_policy(self.content(), strict)),
            },
            MdHashKind::Structured => ComputedHash::Structured {
                fm: hex(hash_frontmatter_map(&filtered, strict)),
                fm_keys: hex(hash_frontmatter_keys(&filtered)),
                body: hex(hash_content_with_policy(self.content(), strict)),
                body_structure: hex(hash_body_structure(self)),
            },
            MdHashKind::Detailed => {
                ComputedHash::Detailed(compute_detailed(self, &filtered, strict))
            }
        }
    }
}

/// Clones the document's frontmatter map with ignored keys removed (exact-key
/// match). Returns an owned map so the source document is never mutated.
fn filtered_frontmatter(md: &Markdown, ignore: &BTreeSet<String>) -> FrontmatterMap {
    md.frontmatter()
        .as_map()
        .iter()
        .filter(|(key, _)| !ignore.contains(key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// Hashes the document's heading structure: each heading in document order
/// rendered as `<#...> <title>`, joined with newlines. Captures heading text,
/// level, and order without section content. Empty when there are no headings.
fn hash_body_structure(md: &Markdown) -> u64 {
    let toc = md.toc();
    let headings = toc.all_headings();
    if headings.is_empty() {
        return xx_hash("");
    }
    let joined = headings
        .iter()
        .map(|node| format!("{} {}", "#".repeat(node.level.hash_count()), node.title))
        .collect::<Vec<_>>()
        .join("\n");
    xx_hash(&joined)
}

/// Builds the nested `detailed` value: the frontmatter pair, a nullable
/// preamble hash, and one section tuple per heading in document order.
///
/// Section content is the heading's own prelude (content before its first
/// child heading), hashed under the body whitespace policy so a whitespace-only
/// edit does not change the non-strict content hash.
fn compute_detailed(md: &Markdown, filtered: &FrontmatterMap, strict: bool) -> DetailedValue {
    let frontmatter = FmHashPair {
        fm: hex(hash_frontmatter_map(filtered, strict)),
        keys: hex(hash_frontmatter_keys(filtered)),
    };

    let toc = md.toc();

    let preamble = if toc.preamble.trim().is_empty() {
        None
    } else {
        Some(hex(hash_content_with_policy(&toc.preamble, strict)))
    };

    let sections = toc
        .all_headings()
        .iter()
        .map(|node| SectionTuple {
            level: node.level.as_u8(),
            heading: node.title.clone(),
            content_hash: hex(hash_content_with_policy(
                node.prelude_content().unwrap_or(""),
                strict,
            )),
        })
        .collect();

    DetailedValue {
        frontmatter,
        preamble,
        sections,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FM_BODY: &str = "---\ntitle: Hello\nauthor: Alice\n---\n# Intro\n\nWelcome.\n\n## Setup\n\nSteps.";

    fn md(content: &str) -> Markdown {
        content.into()
    }

    fn is_hex16(s: &str) -> bool {
        s.len() == 16 && s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    }

    #[test]
    fn fm_and_body_are_single_hex16_strings() {
        let doc = md(FM_BODY);
        let opts = MdHashOptions::default();

        let ComputedHash::Fm(fm) = doc.compute_hash(MdHashKind::Fm, &opts) else {
            panic!("expected Fm");
        };
        let ComputedHash::Body(body) = doc.compute_hash(MdHashKind::Body, &opts) else {
            panic!("expected Body");
        };

        assert!(is_hex16(&fm), "fm hash not hex16: {fm}");
        assert!(is_hex16(&body), "body hash not hex16: {body}");
    }

    #[test]
    fn simple_matches_legacy_two_part_shape() {
        let doc = md(FM_BODY);
        let opts = MdHashOptions::default();

        let computed = doc.compute_hash(MdHashKind::Simple, &opts);
        let flat = computed.flat_string().unwrap();

        // Document carries no ignored keys, so the default hash must equal the
        // historical `hash(false, false, false)` output exactly.
        assert_eq!(flat, doc.hash(false, false, false));

        let (fm, body) = flat.split_once('-').unwrap();
        assert!(is_hex16(fm));
        assert!(is_hex16(body));
    }

    #[test]
    fn structured_has_four_hex16_parts() {
        let doc = md(FM_BODY);
        let flat = doc
            .compute_hash(MdHashKind::Structured, &MdHashOptions::default())
            .flat_string()
            .unwrap();

        let parts: Vec<&str> = flat.split('-').collect();
        assert_eq!(parts.len(), 4, "expected four parts, got {flat}");
        assert!(parts.iter().all(|p| is_hex16(p)));
    }

    #[test]
    fn detailed_captures_preamble_and_sections() {
        let doc = md("---\ntitle: T\n---\nLead-in prose.\n\n# Intro\n\nA.\n\n## Setup\n\nB.");
        let ComputedHash::Detailed(value) =
            doc.compute_hash(MdHashKind::Detailed, &MdHashOptions::default())
        else {
            panic!("expected Detailed");
        };

        assert!(is_hex16(&value.frontmatter.fm));
        assert!(is_hex16(&value.frontmatter.keys));
        assert!(value.preamble.as_deref().is_some_and(is_hex16));

        assert_eq!(value.sections.len(), 2);
        assert_eq!(value.sections[0].level, 1);
        assert_eq!(value.sections[0].heading, "Intro");
        assert_eq!(value.sections[1].level, 2);
        assert_eq!(value.sections[1].heading, "Setup");
        assert!(value.sections.iter().all(|s| is_hex16(&s.content_hash)));
    }

    #[test]
    fn detailed_preamble_is_none_without_lead_in() {
        let doc = md("# Intro\n\nBody only.");
        let ComputedHash::Detailed(value) =
            doc.compute_hash(MdHashKind::Detailed, &MdHashOptions::default())
        else {
            panic!("expected Detailed");
        };
        assert!(value.preamble.is_none());
    }

    #[test]
    fn detailed_section_tuple_serializes_as_array() {
        let tuple = SectionTuple {
            level: 2,
            heading: "Setup".to_string(),
            content_hash: "00000000000000ab".to_string(),
        };
        let json = serde_json::to_value(&tuple).unwrap();
        assert_eq!(json, serde_json::json!([2, "Setup", "00000000000000ab"]));

        let round_trip: SectionTuple = serde_json::from_value(json).unwrap();
        assert_eq!(round_trip, tuple);
    }

    #[test]
    fn whitespace_only_body_change_keeps_non_strict_hashes() {
        let original = md("# Intro\n\nWelcome.\n\n## Setup\n\nSteps.");
        let respaced = md("# Intro\n\n\n\nWelcome.   \n\n## Setup\n\n   Steps.");
        let opts = MdHashOptions::default();

        for kind in [MdHashKind::Body, MdHashKind::Simple, MdHashKind::Structured] {
            assert_eq!(
                original.compute_hash(kind, &opts),
                respaced.compute_hash(kind, &opts),
                "{kind} changed under whitespace-only edit",
            );
        }

        assert_eq!(
            original.compute_hash(MdHashKind::Detailed, &opts),
            respaced.compute_hash(MdHashKind::Detailed, &opts),
        );
    }

    #[test]
    fn ignored_keys_do_not_affect_frontmatter_hash() {
        let plain = md("---\ntitle: T\n---\n# H\n\nBody.");
        let with_managed = md("---\ntitle: T\nhash: stale-value\nlast_updated: 2020-01-01\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();

        assert_eq!(
            plain.compute_hash(MdHashKind::Fm, &opts),
            with_managed.compute_hash(MdHashKind::Fm, &opts),
        );
    }

    #[test]
    fn absent_frontmatter_hashes_as_empty_frontmatter() {
        let absent = md("# Heading\n\nBody.");
        let empty = md("---\n---\n# Heading\n\nBody.");
        let opts = MdHashOptions::default();

        let ComputedHash::Fm(absent_fm) = absent.compute_hash(MdHashKind::Fm, &opts) else {
            panic!("expected Fm");
        };
        let ComputedHash::Fm(empty_fm) = empty.compute_hash(MdHashKind::Fm, &opts) else {
            panic!("expected Fm");
        };

        assert_eq!(absent_fm, empty_fm);
        // Empty frontmatter hashes the empty string.
        assert_eq!(absent_fm, hex(xx_hash("")));
    }

    #[test]
    fn extra_ignored_property_excluded_from_hash() {
        let with_extra = md("---\ntitle: T\ndraft: true\n---\n# H\n\nBody.");
        let without_extra = md("---\ntitle: T\n---\n# H\n\nBody.");

        let opts = MdHashOptions {
            extra_ignored: vec!["draft".to_string()],
            ..MdHashOptions::default()
        };

        assert_eq!(
            with_extra.compute_hash(MdHashKind::Fm, &opts),
            without_extra.compute_hash(MdHashKind::Fm, &opts),
        );
    }
}
