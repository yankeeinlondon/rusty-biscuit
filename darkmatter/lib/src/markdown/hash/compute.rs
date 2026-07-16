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
use crate::markdown::MarkdownTocNode;

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
        #[cfg(test)]
        probe::record();

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
                fm_keys: hex(hash_frontmatter_keys(&filtered, strict)),
                body: hex(hash_content_with_policy(self.content(), strict)),
                body_structure: hex(hash_body_structure(self)),
            },
            MdHashKind::Detailed => {
                ComputedHash::Detailed(compute_detailed(self, &filtered, strict))
            }
        }
    }
}

/// Counts [`Markdown::compute_hash`] calls so tests can police the spec's
/// at-most-one-artifact-per-`(kind, effective options)` bound structurally.
///
/// The bound is a structural property, not a performance one: a timing
/// measurement cannot distinguish "computed once" from "computed twice, cheaply".
/// The counter is thread-local so tests sharing a process stay independent.
#[cfg(test)]
pub(super) mod probe {
    use std::cell::Cell;

    thread_local! {
        static COMPUTE_CALLS: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn record() {
        COMPUTE_CALLS.with(|calls| calls.set(calls.get() + 1));
    }

    /// Runs `body`, returning its value alongside the number of `compute_hash`
    /// calls it made.
    pub(in crate::markdown::hash) fn count_calls<T>(body: impl FnOnce() -> T) -> (T, usize) {
        let start = COMPUTE_CALLS.with(Cell::get);
        let value = body();
        (value, COMPUTE_CALLS.with(Cell::get) - start)
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

/// The literal heading text for the heading whose source line starts at
/// `start_byte`: the heading source with only ATX/setext markers and surrounding
/// whitespace removed. Inline Markdown syntax is preserved, so `# Install *Now*`
/// yields `Install *Now*` rather than the parsed `Install Now`.
///
/// For ATX headings, the leading `#` run and an optional whitespace-preceded
/// closing `#` run are stripped. Setext heading text lines carry no markers and
/// are only trimmed.
fn literal_heading(content: &str, start_byte: usize) -> String {
    let line = content[start_byte..].split('\n').next().unwrap_or("");
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        // Setext heading: the underline lives on the following line, so the
        // captured text line carries no markers.
        return trimmed.to_string();
    }
    let body = trimmed.trim_start_matches('#').trim_start().trim_end();
    if body.ends_with('#') {
        // A trailing `#` run is a closing sequence only when whitespace
        // precedes it (CommonMark); otherwise the `#`s are heading content.
        let stripped = body.trim_end_matches('#');
        if stripped.is_empty() || stripped.ends_with(char::is_whitespace) {
            return stripped.trim_end().to_string();
        }
    }
    body.to_string()
}

/// Hashes the document's heading structure: each heading in document order
/// rendered as `<#...> <literal-heading>`, joined with newlines. Captures
/// heading text, level, and order without section content. Empty when there are
/// no headings.
///
/// The heading text is the literal source ([`literal_heading`]), so an inline
/// markup change such as `# A *B*` vs `# A B` is a structural difference. This is
/// a verbatim structural fingerprint — it applies no whitespace normalization, so
/// `strict` does not affect it. Whitespace-only differences in the heading
/// *source* surface through the `body` value component (which is verbatim under
/// strict), not through this skeleton.
fn hash_body_structure(md: &Markdown) -> u64 {
    let toc = md.toc();
    let headings = toc.all_headings();
    if headings.is_empty() {
        return xx_hash("");
    }
    let content = md.content();
    let joined = headings
        .iter()
        .map(|node| {
            format!(
                "{} {}",
                "#".repeat(node.level.hash_count()),
                literal_heading(content, node.source_span.0)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    xx_hash(&joined)
}

/// Builds the nested `detailed` value: the frontmatter pair, a nullable
/// preamble hash, and one section tuple per heading in document order.
///
/// A section's content is all content after the heading line up to the next
/// heading at the same or a parent (lower-or-equal) level — its full subtree,
/// including any nested child sections. It is hashed under the body whitespace
/// policy so a whitespace-only edit does not change the non-strict content hash.
/// Because a parent's slice contains its children, editing a child section
/// changes both the child tuple and every ancestor tuple.
fn compute_detailed(md: &Markdown, filtered: &FrontmatterMap, strict: bool) -> DetailedValue {
    let frontmatter = FmHashPair {
        fm: hex(hash_frontmatter_map(filtered, strict)),
        keys: hex(hash_frontmatter_keys(filtered, strict)),
    };

    let toc = md.toc();

    let preamble = if toc.preamble.trim().is_empty() {
        None
    } else {
        Some(hex(hash_content_with_policy(&toc.preamble, strict)))
    };

    let content = md.content();
    let headings = toc.all_headings();
    let sections = headings
        .iter()
        .enumerate()
        .map(|(index, node)| SectionTuple {
            level: node.level.as_u8(),
            heading: literal_heading(content, node.source_span.0),
            content_hash: hex(hash_content_with_policy(
                section_content(content, &headings, index),
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

/// The content slice owned by the heading at `index`: everything after the
/// heading line up to the next heading at the same or a parent (lower-or-equal)
/// level, or end of document. This is the spec's detailed-section boundary and
/// includes any nested child sections.
///
/// Returns `""` when the heading has no trailing content (e.g. a trailing
/// heading with no body). Byte offsets come from `MarkdownTocNode::source_span`,
/// which indexes into the same `content` string the TOC was built from.
fn section_content<'a>(content: &'a str, headings: &[&MarkdownTocNode], index: usize) -> &'a str {
    let node = headings[index];
    let level = node.level.as_u8();
    let start = node.source_span.0;
    let end = headings[index + 1..]
        .iter()
        .find(|next| next.level.as_u8() <= level)
        .map_or(content.len(), |next| next.source_span.0);

    let section = &content[start..end];
    match section.find('\n') {
        Some(newline) => &section[newline + 1..],
        None => "",
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
    fn structured_body_structure_distinguishes_inline_heading_markup() {
        // The structural fingerprint hashes the literal heading source, so an
        // inline-markup-only change is a structural difference even though the
        // parsed title (`Install Now`) is identical.
        let styled = md("# Install *Now*\n\nBody.");
        let plain = md("# Install Now\n\nBody.");
        let opts = MdHashOptions::default();

        let body_structure = |computed: ComputedHash| match computed {
            ComputedHash::Structured { body_structure, .. } => body_structure,
            other => panic!("expected Structured, got {other:?}"),
        };

        assert_ne!(
            body_structure(styled.compute_hash(MdHashKind::Structured, &opts)),
            body_structure(plain.compute_hash(MdHashKind::Structured, &opts)),
            "inline markup in the heading source is a structural difference",
        );
    }

    #[test]
    fn detailed_section_heading_is_literal_source_text() {
        // The persisted heading keeps inline Markdown and strips only the ATX
        // markers (leading `#` run and the whitespace-preceded closing `##`).
        let doc = md("# Install *Now* ##\n\nBody.");
        let ComputedHash::Detailed(value) =
            doc.compute_hash(MdHashKind::Detailed, &MdHashOptions::default())
        else {
            panic!("expected Detailed");
        };
        assert_eq!(value.sections[0].heading, "Install *Now*");
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
    fn detailed_child_edit_changes_child_and_parent_tuples() {
        // A section's content is its full subtree, so editing a nested child
        // changes the child tuple AND every ancestor tuple.
        let original = md("# Intro\n\nA.\n\n## Setup\n\nB.");
        let edited = md("# Intro\n\nA.\n\n## Setup\n\nB changed.");
        let opts = MdHashOptions::default();

        let ComputedHash::Detailed(orig) = original.compute_hash(MdHashKind::Detailed, &opts)
        else {
            panic!("expected Detailed");
        };
        let ComputedHash::Detailed(new) = edited.compute_hash(MdHashKind::Detailed, &opts) else {
            panic!("expected Detailed");
        };

        assert_eq!(orig.sections[0].heading, "Intro");
        assert_eq!(orig.sections[1].heading, "Setup");

        assert_ne!(
            orig.sections[1].content_hash, new.sections[1].content_hash,
            "child Setup tuple must change",
        );
        assert_ne!(
            orig.sections[0].content_hash, new.sections[0].content_hash,
            "parent Intro tuple must change because it subsumes the child subtree",
        );
    }

    #[test]
    fn detailed_sibling_edit_leaves_other_sibling_tuple_stable() {
        // The boundary for a section is the next same-or-parent heading, so an
        // edit under one H1 sibling must not perturb the other sibling's tuple.
        let original = md("# One\n\nX.\n\n# Two\n\nY.");
        let edited = md("# One\n\nX.\n\n# Two\n\nY changed.");
        let opts = MdHashOptions::default();

        let ComputedHash::Detailed(orig) = original.compute_hash(MdHashKind::Detailed, &opts)
        else {
            panic!("expected Detailed");
        };
        let ComputedHash::Detailed(new) = edited.compute_hash(MdHashKind::Detailed, &opts) else {
            panic!("expected Detailed");
        };

        assert_eq!(orig.sections[0].heading, "One");
        assert_eq!(orig.sections[1].heading, "Two");
        assert_eq!(
            orig.sections[0].content_hash, new.sections[0].content_hash,
            "sibling One must be unchanged",
        );
        assert_ne!(
            orig.sections[1].content_hash, new.sections[1].content_hash,
            "sibling Two must change",
        );
    }

    #[test]
    fn structured_strict_preserves_frontmatter_key_order() {
        // Same keys and values, different insertion order. Non-strict sorts keys
        // (so `fm_keys` matches); strict preserves order (so `fm_keys` differs).
        let a = md("---\nbeta: 1\nalpha: 2\n---\n# H\n\nBody.");
        let b = md("---\nalpha: 2\nbeta: 1\n---\n# H\n\nBody.");

        let strict = MdHashOptions {
            strict: true,
            ..MdHashOptions::default()
        };
        let non_strict = MdHashOptions::default();

        let fm_keys = |computed: ComputedHash| match computed {
            ComputedHash::Structured { fm_keys, .. } => fm_keys,
            other => panic!("expected Structured, got {other:?}"),
        };

        assert_ne!(
            fm_keys(a.compute_hash(MdHashKind::Structured, &strict)),
            fm_keys(b.compute_hash(MdHashKind::Structured, &strict)),
            "strict must not reorder frontmatter keys",
        );
        assert_eq!(
            fm_keys(a.compute_hash(MdHashKind::Structured, &non_strict)),
            fm_keys(b.compute_hash(MdHashKind::Structured, &non_strict)),
            "non-strict sorts keys, so insertion order is irrelevant",
        );
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
