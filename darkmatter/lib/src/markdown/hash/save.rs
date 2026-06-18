//! The `--save` decision: which kind to write, what stored value to persist,
//! and whether `last_updated` should be bumped.
//!
//! This is the *decision* layer only. The write-back path (mutating the
//! frontmatter model and re-serializing the document) consumes a
//! [`SaveDecision`]; it does not live here.
//!
//! ## Rules
//!
//! Change is always detected like-for-like under the document's *stored*
//! ignore-set (see [`Markdown::compare_hash`]). The kind written is the
//! *selected* kind (forced, else matched, else `simple`), and the value is
//! computed under the *current* options, since `--save` establishes a new
//! baseline.
//!
//! - **Same kind** — write only if content changed (bumping `last_updated`), or
//!   if only the ignore policy changed (rewrite value + `ignored`, no bump).
//! - **Higher resolution** (upgrade) — evaluate change at the old, lower
//!   resolution; upgrade either way, bumping `last_updated` only when the lower
//!   view changed.
//! - **Lower resolution** (downgrade) — evaluate change at the new, lower
//!   resolution; downgrade either way, bumping only when the lower view changed.
//! - **Incomparable** (`fm` ↔ `body`) — rewrite at the forced kind without
//!   treating the switch as a content change.
//! - **No stored hash** — write the first baseline without bumping.

use super::compare::{HashComparison, normalize_extras};
use super::kind::{KindRelation, MdHashKind, select_kind};
use super::options::MdHashOptions;
use super::stored::StoredHash;
use crate::markdown::{Markdown, MarkdownResult};

/// What `--save` decided for a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveDecision {
    /// The kind selected for this save.
    pub kind: MdHashKind,
    /// The stored hash to persist, or `None` when the file needs no change.
    pub new_stored: Option<StoredHash>,
    /// Whether `last_updated` should be set to today's date. Only meaningful
    /// when `new_stored` is `Some`.
    pub bump_last_updated: bool,
    /// The like-for-like comparison that drove the decision, or `None` when
    /// there was no stored hash (a first baseline).
    pub comparison: Option<HashComparison>,
}

impl Markdown {
    /// Decides what a `--save` should write for this document.
    ///
    /// Pass the document's parsed stored hash, or `None` if it has none. The
    /// returned [`SaveDecision`] reports the selected kind, the stored hash to
    /// persist (or `None` to leave the file untouched), and whether
    /// `last_updated` should be bumped. The decision is deterministic and reads
    /// no clock; the caller supplies today's date when applying the write.
    ///
    /// ## Errors
    ///
    /// Propagates [`MarkdownError::MalformedStoredHash`] from
    /// [`Markdown::compare_hash`] when a stored flat value is malformed.
    ///
    /// [`MarkdownError::MalformedStoredHash`]: crate::markdown::MarkdownError::MalformedStoredHash
    pub fn plan_hash_save(
        &self,
        stored: Option<&StoredHash>,
        options: &MdHashOptions,
    ) -> MarkdownResult<SaveDecision> {
        let selected = select_kind(stored.map(|s| s.kind), options.forced_kind);

        // `--save` establishes a new baseline, so the written value is always
        // computed under the *current* options (and thus the current ignore-set).
        let baseline = || StoredHash {
            kind: selected,
            value: self.compute_hash(selected, options).to_stored_value(),
            ignored: normalize_extras(&options.extra_ignored),
        };

        let Some(stored) = stored else {
            // First baseline: record the hash, but a first write is not a
            // detected content change, so leave `last_updated` alone.
            return Ok(SaveDecision {
                kind: selected,
                new_stored: Some(baseline()),
                bump_last_updated: false,
                comparison: None,
            });
        };

        let comparison = self.compare_hash(stored, options)?;
        let relation = MdHashKind::relate(stored.kind, selected);

        let (new_stored, bump_last_updated) = match relation {
            // The switch crosses incomparable concerns; we cannot evaluate
            // change across it, so rewrite at the forced kind without a bump.
            KindRelation::Incomparable => (Some(baseline()), false),
            _ => {
                // The lower of the two resolutions is the view we judge change
                // at: the stored kind for same/upgrade, the forced kind for a
                // downgrade.
                let view_kind = match relation {
                    KindRelation::Lower => selected,
                    _ => stored.kind,
                };
                let content_changed = match view_kind {
                    MdHashKind::Fm => comparison.frontmatter_changed,
                    MdHashKind::Body => comparison.body_changed,
                    _ => comparison.frontmatter_changed || comparison.body_changed,
                };

                if content_changed {
                    (Some(baseline()), true)
                } else if relation != KindRelation::Same {
                    // A resolution change with no content change: rewrite the
                    // kind, leave `last_updated` untouched.
                    (Some(baseline()), false)
                } else if comparison.ignore_policy.is_some() {
                    // Ignore-policy-only change: recompute the value under the
                    // new set and rewrite `ignored`, but do not bump.
                    (Some(baseline()), false)
                } else {
                    // Nothing changed at all: leave the file untouched.
                    (None, false)
                }
            }
        };

        Ok(SaveDecision {
            kind: selected,
            new_stored,
            bump_last_updated,
            comparison: Some(comparison),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::hash::StoredHashValue;

    fn md(content: &str) -> Markdown {
        content.into()
    }

    /// A stored hash computed from `doc` at `kind` under `opts`.
    fn stored_at(doc: &Markdown, kind: MdHashKind, opts: &MdHashOptions) -> StoredHash {
        StoredHash {
            kind,
            value: doc.compute_hash(kind, opts).to_stored_value(),
            ignored: normalize_extras(&opts.extra_ignored),
        }
    }

    fn forced(kind: MdHashKind) -> MdHashOptions {
        MdHashOptions {
            forced_kind: Some(kind),
            ..MdHashOptions::default()
        }
    }

    #[test]
    fn same_kind_no_change_leaves_file_untouched() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&doc, MdHashKind::Simple, &opts);

        let decision = doc.plan_hash_save(Some(&stored), &opts).unwrap();
        assert!(decision.new_stored.is_none());
        assert!(!decision.bump_last_updated);
    }

    #[test]
    fn same_kind_content_change_writes_and_bumps() {
        let original = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = stored_at(&original, MdHashKind::Simple, &opts);

        let edited = md("---\ntitle: T\n---\n# H\n\nNew body.");
        let decision = edited.plan_hash_save(Some(&stored), &opts).unwrap();
        let new_stored = decision.new_stored.expect("content changed → write");
        assert_eq!(new_stored.kind, MdHashKind::Simple);
        assert!(decision.bump_last_updated);
    }

    #[test]
    fn no_stored_hash_writes_first_baseline_without_bump() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let decision = doc
            .plan_hash_save(None, &MdHashOptions::default())
            .unwrap();
        let new_stored = decision.new_stored.expect("baseline written");
        assert_eq!(new_stored.kind, MdHashKind::Simple);
        assert!(!decision.bump_last_updated);
        assert!(decision.comparison.is_none());
    }

    #[test]
    fn upgrade_without_lower_change_rewrites_kind_no_bump() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let stored = stored_at(&doc, MdHashKind::Simple, &MdHashOptions::default());

        // Same document, forced to structured: an upgrade with no content change.
        let decision = doc
            .plan_hash_save(Some(&stored), &forced(MdHashKind::Structured))
            .unwrap();
        let new_stored = decision.new_stored.expect("kind upgraded → write");
        assert_eq!(new_stored.kind, MdHashKind::Structured);
        assert!(!decision.bump_last_updated);
    }

    #[test]
    fn upgrade_with_lower_change_bumps() {
        let original = md("---\ntitle: T\n---\n# H\n\nBody.");
        let stored = stored_at(&original, MdHashKind::Simple, &MdHashOptions::default());

        let edited = md("---\ntitle: T\n---\n# H\n\nChanged body.");
        let decision = edited
            .plan_hash_save(Some(&stored), &forced(MdHashKind::Structured))
            .unwrap();
        let new_stored = decision.new_stored.expect("changed → write");
        assert_eq!(new_stored.kind, MdHashKind::Structured);
        assert!(decision.bump_last_updated);
    }

    #[test]
    fn downgrade_without_lower_change_rewrites_kind_no_bump() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let stored = stored_at(&doc, MdHashKind::Structured, &MdHashOptions::default());

        let decision = doc
            .plan_hash_save(Some(&stored), &forced(MdHashKind::Simple))
            .unwrap();
        let new_stored = decision.new_stored.expect("kind downgraded → write");
        assert_eq!(new_stored.kind, MdHashKind::Simple);
        assert!(!decision.bump_last_updated);
    }

    #[test]
    fn downgrade_ignores_changes_outside_the_lower_view() {
        // Stored structured; the body changed but the forced lower kind is `fm`,
        // whose view (frontmatter) did not change — so no bump.
        let original = md("---\ntitle: T\n---\n# H\n\nBody.");
        let stored = stored_at(&original, MdHashKind::Structured, &MdHashOptions::default());

        let edited = md("---\ntitle: T\n---\n# H\n\nBody rewritten.");
        let decision = edited
            .plan_hash_save(Some(&stored), &forced(MdHashKind::Fm))
            .unwrap();
        let new_stored = decision.new_stored.expect("kind downgraded → write");
        assert_eq!(new_stored.kind, MdHashKind::Fm);
        assert!(
            !decision.bump_last_updated,
            "body change is outside the fm view"
        );
    }

    #[test]
    fn downgrade_with_lower_change_bumps() {
        let original = md("---\ntitle: T\n---\n# H\n\nBody.");
        let stored = stored_at(&original, MdHashKind::Structured, &MdHashOptions::default());

        let edited = md("---\ntitle: Changed\n---\n# H\n\nBody.");
        let decision = edited
            .plan_hash_save(Some(&stored), &forced(MdHashKind::Fm))
            .unwrap();
        assert!(decision.bump_last_updated, "fm view changed");
    }

    #[test]
    fn incomparable_kind_switch_rewrites_without_bump() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let stored = stored_at(&doc, MdHashKind::Fm, &MdHashOptions::default());

        let decision = doc
            .plan_hash_save(Some(&stored), &forced(MdHashKind::Body))
            .unwrap();
        let new_stored = decision.new_stored.expect("kind switched → write");
        assert_eq!(new_stored.kind, MdHashKind::Body);
        assert!(!decision.bump_last_updated);
    }

    #[test]
    fn ignore_policy_only_change_rewrites_without_bump() {
        // Baseline ignored nothing extra; the document carries an unchanged
        // `draft` key. The current policy adds `draft` to the ignore-set.
        let doc = md("---\ntitle: T\ndraft: true\n---\n# H\n\nBody.");
        let stored = stored_at(&doc, MdHashKind::Simple, &MdHashOptions::default());

        let new_policy = MdHashOptions {
            extra_ignored: vec!["draft".to_string()],
            ..MdHashOptions::default()
        };
        let decision = doc.plan_hash_save(Some(&stored), &new_policy).unwrap();
        let new_stored = decision.new_stored.expect("ignore policy changed → write");
        assert_eq!(new_stored.ignored, vec!["draft".to_string()]);
        assert!(!decision.bump_last_updated);
        // The recomputed value drops the ignored `draft` key from the fm hash,
        // so it must differ from the stored value that hashed `draft`.
        assert_ne!(new_stored.value, stored.value);
        // The advisory rides along on the comparison.
        assert!(
            decision
                .comparison
                .expect("compared")
                .ignore_policy
                .is_some()
        );
    }

    #[test]
    fn ignore_policy_only_recomputes_value_to_a_simple_string() {
        let doc = md("---\ntitle: T\ndraft: true\n---\n# H\n\nBody.");
        let stored = stored_at(&doc, MdHashKind::Simple, &MdHashOptions::default());

        let new_policy = MdHashOptions {
            extra_ignored: vec!["draft".to_string()],
            ..MdHashOptions::default()
        };
        let new_stored = doc
            .plan_hash_save(Some(&stored), &new_policy)
            .unwrap()
            .new_stored
            .unwrap();
        assert!(matches!(new_stored.value, StoredHashValue::Flat(_)));
    }
}
