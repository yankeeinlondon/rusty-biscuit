//! Like-for-like comparison of a document against its stored hash.
//!
//! Change detection **always** recomputes the document under the document's
//! *stored* ignore-set, never the current environment's ignore-set, so results
//! are reproducible across machines and CI. When the two ignore-sets differ, a
//! separate [`IgnorePolicyAdvisory`] is surfaced — it is never counted as a
//! content change.

use super::compute::{ComputedHash, SectionTuple};
use super::kind::MdHashKind;
use super::options::MdHashOptions;
use super::stored::{StoredHash, StoredHashValue, malformed};
use crate::markdown::{Markdown, MarkdownResult};

/// Outcome of comparing a document against its stored hash, at the resolution
/// the stored kind affords.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashComparison {
    /// The kind the comparison was performed at (the stored kind).
    pub kind: MdHashKind,
    /// Whether the frontmatter concern changed. Always `false` for the `body`
    /// kind, which carries no frontmatter concern.
    pub frontmatter_changed: bool,
    /// Whether the body concern changed. Always `false` for the `fm` kind,
    /// which carries no body concern.
    pub body_changed: bool,
    /// Kind-specific finer resolution, where available.
    pub detail: ComparisonDetail,
    /// An advisory noting that the stored ignore-set differs from the current
    /// one. Never counted as a content change.
    pub ignore_policy: Option<IgnorePolicyAdvisory>,
}

/// Finer comparison resolution beyond the coarse frontmatter/body booleans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonDetail {
    /// `fm` / `body` / `simple` — only the coarse booleans are meaningful.
    Coarse,
    /// `structured` — distinguishes key changes from value-only changes and
    /// structural body changes from semantic content changes.
    Structured {
        /// Whether the frontmatter keys (ignoring values) changed.
        fm_keys_changed: bool,
        /// Whether the heading structure changed.
        body_structure_changed: bool,
    },
    /// `detailed` — records the same key/structure resolution as `structured`
    /// plus a coarse preamble/section split. The per-section alignment that the
    /// detailed explanation renders is computed separately by the explanation
    /// engine.
    Detailed {
        /// Whether the frontmatter keys (ignoring values) changed.
        fm_keys_changed: bool,
        /// Whether the heading structure (levels, headings, order) changed.
        body_structure_changed: bool,
        /// Whether the preamble changed.
        preamble_changed: bool,
        /// Whether any section's heading, level, order, or content changed.
        sections_changed: bool,
    },
}

/// Advisory that the document's stored ignore-set differs from the current one.
///
/// Both lists are sorted and hold only the *extra* ignored properties (never the
/// always-managed `hash` / `last_updated` keys).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnorePolicyAdvisory {
    /// The extra properties the current options would ignore.
    pub now_ignoring: Vec<String>,
    /// The extra properties the stored hash recorded.
    pub previously: Vec<String>,
}

impl Markdown {
    /// Compares this document against a stored hash, recomputing under the
    /// stored ignore-set (like-for-like).
    ///
    /// The comparison is performed at the stored hash's kind: a `simple` stored
    /// hash yields only the coarse frontmatter/body booleans, while
    /// `structured` and `detailed` carry finer [`ComparisonDetail`]. Any
    /// difference between the stored ignore-set and the current one is reported
    /// as an [`IgnorePolicyAdvisory`] and never counted as a content change.
    ///
    /// ## Errors
    ///
    /// Returns [`MarkdownError::MalformedStoredHash`] when a stored flat value
    /// does not have the component count its kind requires.
    ///
    /// [`MarkdownError::MalformedStoredHash`]: crate::markdown::MarkdownError::MalformedStoredHash
    pub fn compare_hash(
        &self,
        stored: &StoredHash,
        options: &MdHashOptions,
    ) -> MarkdownResult<HashComparison> {
        let computed = self.compute_hash(stored.kind, &compare_options(stored, options));
        self.compare_with_computed(stored, options, &computed)
    }

    /// [`Self::compare_hash`] against an already-computed artifact.
    ///
    /// `computed` **must** be `compute_hash(stored.kind, compare_options(stored,
    /// options))` — the like-for-like artifact under the *stored* ignore-set.
    /// Callers that already hold that exact artifact use this to avoid
    /// recomputing it; everyone else should call [`Self::compare_hash`].
    pub(super) fn compare_with_computed(
        &self,
        stored: &StoredHash,
        options: &MdHashOptions,
        computed: &ComputedHash,
    ) -> MarkdownResult<HashComparison> {
        let (frontmatter_changed, body_changed, detail) =
            diff_components(&stored.value, computed, &options.property)?;

        let ignore_policy = ignore_policy_advisory(&stored.ignored, &options.extra_ignored);

        Ok(HashComparison {
            kind: stored.kind,
            frontmatter_changed,
            body_changed,
            detail,
            ignore_policy,
        })
    }
}

/// The like-for-like options a comparison recomputes under: the document's
/// *stored* ignore-set, never the current environment's, so results are
/// reproducible across machines and CI.
///
/// This is the identity that decides whether two hash artifacts are the same
/// artifact — `--save` computes a *second*, different one under the current
/// options for its new baseline, and the two must not be conflated.
pub(super) fn compare_options(stored: &StoredHash, options: &MdHashOptions) -> MdHashOptions {
    MdHashOptions {
        property: options.property.clone(),
        extra_ignored: stored.ignored.clone(),
        forced_kind: None,
        strict: options.strict,
    }
}

/// Compares a stored value against the freshly computed hash at the same kind.
fn diff_components(
    stored: &StoredHashValue,
    computed: &ComputedHash,
    property: &str,
) -> MarkdownResult<(bool, bool, ComparisonDetail)> {
    match (stored, computed) {
        (StoredHashValue::Flat(s), ComputedHash::Fm(c)) => {
            Ok((s != c, false, ComparisonDetail::Coarse))
        }
        (StoredHashValue::Flat(s), ComputedHash::Body(c)) => {
            Ok((false, s != c, ComparisonDetail::Coarse))
        }
        (StoredHashValue::Flat(s), ComputedHash::Simple { fm, body }) => {
            let parts = split_flat(s, 2, property)?;
            Ok((parts[0] != fm, parts[1] != body, ComparisonDetail::Coarse))
        }
        (
            StoredHashValue::Flat(s),
            ComputedHash::Structured {
                fm,
                fm_keys,
                body,
                body_structure,
            },
        ) => {
            let parts = split_flat(s, 4, property)?;
            Ok((
                parts[0] != fm,
                parts[2] != body,
                ComparisonDetail::Structured {
                    fm_keys_changed: parts[1] != fm_keys,
                    body_structure_changed: parts[3] != body_structure,
                },
            ))
        }
        (StoredHashValue::Detailed(s), ComputedHash::Detailed(c)) => {
            let preamble_changed = s.preamble != c.preamble;
            let sections_changed = s.sections != c.sections;
            Ok((
                s.frontmatter.fm != c.frontmatter.fm,
                preamble_changed || sections_changed,
                ComparisonDetail::Detailed {
                    fm_keys_changed: s.frontmatter.keys != c.frontmatter.keys,
                    body_structure_changed: !same_structure(&s.sections, &c.sections),
                    preamble_changed,
                    sections_changed,
                },
            ))
        }
        // `StoredHash::parse` and `compute_hash` both derive their shape from the
        // same kind, so a flat-vs-detailed mismatch can only arise from a
        // hand-built `StoredHash`.
        _ => Err(malformed(
            property,
            "stored value shape does not match its kind",
        )),
    }
}

/// Splits a flat hash string into exactly `expected` `-`-joined components.
fn split_flat<'a>(
    flat: &'a str,
    expected: usize,
    property: &str,
) -> MarkdownResult<Vec<&'a str>> {
    let parts: Vec<&str> = flat.split('-').collect();
    if parts.len() != expected {
        return Err(malformed(
            property,
            format!(
                "expected {expected} '-'-joined hash components, found {}",
                parts.len()
            ),
        ));
    }
    Ok(parts)
}

/// Whether two section lists share the same heading structure: identical
/// `(level, heading)` sequence, ignoring content hashes.
fn same_structure(a: &[SectionTuple], b: &[SectionTuple]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.level == y.level && x.heading == y.heading)
}

/// Builds an advisory when the stored extra-ignore list differs from the
/// current one. Both inputs are normalized (sorted, de-duplicated) first.
fn ignore_policy_advisory(stored: &[String], current: &[String]) -> Option<IgnorePolicyAdvisory> {
    let previously = normalize_extras(stored);
    let now_ignoring = normalize_extras(current);
    if previously == now_ignoring {
        None
    } else {
        Some(IgnorePolicyAdvisory {
            now_ignoring,
            previously,
        })
    }
}

/// Sorts and de-duplicates a list of extra-ignore property names.
pub(super) fn normalize_extras(extras: &[String]) -> Vec<String> {
    let mut names: Vec<String> = extras.to_vec();
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::hash::StoredHashValue;

    fn md(content: &str) -> Markdown {
        content.into()
    }

    /// Builds a stored hash by computing the document at `kind` under `opts`.
    fn stored_at(doc: &Markdown, kind: MdHashKind, opts: &MdHashOptions) -> StoredHash {
        StoredHash {
            kind,
            value: doc.compute_hash(kind, opts).to_stored_value(),
            ignored: super::normalize_extras(&opts.extra_ignored),
        }
    }

    #[test]
    fn simple_unchanged_reports_no_change() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&doc, MdHashKind::Simple, &opts);

        let cmp = doc.compare_hash(&stored, &opts).unwrap();
        assert!(!cmp.frontmatter_changed);
        assert!(!cmp.body_changed);
        assert!(cmp.ignore_policy.is_none());
    }

    #[test]
    fn simple_detects_body_only_change() {
        let original = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Simple, &opts);

        let edited = md("---\ntitle: T\n---\n# H\n\nBody changed.");
        let cmp = edited.compare_hash(&stored, &opts).unwrap();
        assert!(!cmp.frontmatter_changed);
        assert!(cmp.body_changed);
    }

    #[test]
    fn structured_distinguishes_value_change_from_key_change() {
        let original = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Structured, &opts);

        // Only a value changed; the key set is identical.
        let value_only = md("---\ntitle: Different\n---\n# H\n\nBody.");
        let cmp = value_only.compare_hash(&stored, &opts).unwrap();
        assert!(cmp.frontmatter_changed);
        match cmp.detail {
            ComparisonDetail::Structured {
                fm_keys_changed,
                body_structure_changed,
            } => {
                assert!(!fm_keys_changed, "keys unchanged, only the value moved");
                assert!(!body_structure_changed);
            }
            other => panic!("expected structured detail, got {other:?}"),
        }
    }

    #[test]
    fn detailed_separates_structure_from_content() {
        let original = md("# Intro\n\nA.\n\n## Setup\n\nB.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Detailed, &opts);

        // Same headings, edited content under "Setup".
        let edited = md("# Intro\n\nA.\n\n## Setup\n\nB changed.");
        let cmp = edited.compare_hash(&stored, &opts).unwrap();
        assert!(cmp.body_changed);
        match cmp.detail {
            ComparisonDetail::Detailed {
                body_structure_changed,
                sections_changed,
                preamble_changed,
                ..
            } => {
                assert!(!body_structure_changed, "headings unchanged");
                assert!(sections_changed, "section content changed");
                assert!(!preamble_changed);
            }
            other => panic!("expected detailed detail, got {other:?}"),
        }
    }

    #[test]
    fn comparison_uses_stored_ignore_set_not_env() {
        // Baseline ignores `draft`; the property is present and would otherwise
        // change the frontmatter hash.
        let stored_opts = MdHashOptions {
            extra_ignored: vec!["draft".to_string()],
            ..MdHashOptions::default()
        };
        let doc = md("---\ntitle: T\ndraft: true\n---\n# H\n\nBody.");
        let stored = stored_at(&doc, MdHashKind::Simple, &stored_opts);

        // The current env ignores nothing extra, but `draft` is unchanged in the
        // document, so a like-for-like comparison under the stored set must still
        // report "unchanged".
        let env_opts = MdHashOptions::default();
        let cmp = doc.compare_hash(&stored, &env_opts).unwrap();
        assert!(!cmp.frontmatter_changed);
        assert!(!cmp.body_changed);
        // The ignore-policy difference is surfaced separately.
        let advisory = cmp.ignore_policy.expect("policy differs");
        assert_eq!(advisory.previously, vec!["draft".to_string()]);
        assert!(advisory.now_ignoring.is_empty());
    }

    #[test]
    fn malformed_flat_component_count_errors() {
        let doc = md("# H\n\nBody.");
        let stored = StoredHash {
            kind: MdHashKind::Structured,
            // Only two components where four are required.
            value: StoredHashValue::Flat("aaaa-bbbb".to_string()),
            ignored: Vec::new(),
        };
        let err = doc
            .compare_hash(&stored, &MdHashOptions::default())
            .unwrap_err();
        assert!(err.to_string().contains("expected 4"));
    }
}
