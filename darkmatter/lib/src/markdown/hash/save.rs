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

use super::compare::{HashComparison, compare_options, normalize_extras};
use super::compute::ComputedHash;
use super::explain::HashExplanation;
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
        Ok(self.plan_hash_save_artifact(stored, options)?.0)
    }

    /// [`Self::plan_hash_save`] plus the explanation `md hash --save` prints,
    /// both derived from the one like-for-like artifact the planning already
    /// computes. The explanation is `None` exactly when `stored` is `None`:
    /// a first baseline has nothing to compare against.
    ///
    /// Composing [`Self::plan_hash_save`] with [`Self::explain_hash_diff`]
    /// instead hashes the document a second time under the same stored-policy
    /// identity, since neither can see the other's artifact.
    ///
    /// ## Errors
    ///
    /// Propagates [`MarkdownError::MalformedStoredHash`] from
    /// [`Self::compare_hash`] when a stored flat value is malformed.
    ///
    /// [`MarkdownError::MalformedStoredHash`]: crate::markdown::MarkdownError::MalformedStoredHash
    pub fn plan_hash_save_explained(
        &self,
        stored: Option<&StoredHash>,
        options: &MdHashOptions,
    ) -> MarkdownResult<(SaveDecision, Option<HashExplanation>)> {
        let (decision, artifact) = self.plan_hash_save_artifact(stored, options)?;
        let explanation = match (stored, &artifact) {
            (Some(stored), Some(computed)) => {
                Some(self.diff_with_computed(stored, options, computed)?.1)
            }
            _ => None,
        };
        Ok((decision, explanation))
    }

    /// The planning core, also yielding the like-for-like comparison artifact it
    /// computed so an explanation can reuse it. The artifact is `None` when
    /// there was no stored hash to compare against.
    fn plan_hash_save_artifact(
        &self,
        stored: Option<&StoredHash>,
        options: &MdHashOptions,
    ) -> MarkdownResult<(SaveDecision, Option<ComputedHash>)> {
        let selected = select_kind(stored.map(|s| s.kind), options.forced_kind);

        // `--save` establishes a new baseline, so the written value is always
        // computed under the *current* options (and thus the current ignore-set).
        let fresh_baseline = || StoredHash {
            kind: selected,
            value: self.compute_hash(selected, options).to_stored_value(),
            ignored: normalize_extras(&options.extra_ignored),
        };

        let Some(stored) = stored else {
            // First baseline: record the hash, but a first write is not a
            // detected content change, so leave `last_updated` alone.
            return Ok((
                SaveDecision {
                    kind: selected,
                    new_stored: Some(fresh_baseline()),
                    bump_last_updated: false,
                    comparison: None,
                },
                None,
            ));
        };

        // The like-for-like comparison artifact, computed once here and reused
        // by the baseline below when — and only when — it is the *same*
        // artifact. `--save` legitimately deals in two different artifacts: the
        // comparison recomputes under the STORED ignore-set at the STORED kind,
        // while the new baseline is written under the CURRENT options at the
        // SELECTED kind. They coincide only when both identities agree, which is
        // the common "re-save an unchanged policy/kind" case; when they diverge
        // (a kind change or an ignore-policy change) the baseline must still be
        // computed separately, so identity is tested rather than assumed.
        let compare_artifact = self.compute_hash(stored.kind, &compare_options(stored, options));
        let comparison = self.compare_with_computed(stored, options, &compare_artifact)?;

        let baseline_is_compare_artifact = selected == stored.kind
            && normalize_extras(&options.extra_ignored) == normalize_extras(&stored.ignored);
        let baseline = || StoredHash {
            kind: selected,
            value: if baseline_is_compare_artifact {
                compare_artifact.clone().to_stored_value()
            } else {
                self.compute_hash(selected, options).to_stored_value()
            },
            ignored: normalize_extras(&options.extra_ignored),
        };

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

        Ok((
            SaveDecision {
                kind: selected,
                new_stored,
                bump_last_updated,
                comparison: Some(comparison),
            },
            Some(compare_artifact),
        ))
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

/// Finding 35.5 regression coverage.
///
/// `plan_hash_save` now computes the like-for-like comparison artifact once and
/// reuses it for the written baseline **only** when both carry the same
/// `(kind, effective MdHashOptions)` identity. The dangerous failure mode is
/// conflating the two artifacts `--save` legitimately deals in: the comparison
/// recomputes under the *stored* ignore-set at the *stored* kind, while the new
/// baseline is written under the *current* options at the *selected* kind.
#[cfg(test)]
mod finding_35_5 {
    use super::super::compute::probe;
    use super::*;
    use crate::markdown::hash::StoredHashValue;

    fn md(content: &str) -> Markdown {
        content.into()
    }

    fn doc() -> Markdown {
        md("---\ntitle: T\ndraft: true\nnotes: keep\n---\n# H\n\nBody.\n\n## Two\n\nMore.")
    }

    fn opts_ignoring(extra: &[&str]) -> MdHashOptions {
        MdHashOptions {
            extra_ignored: extra.iter().map(|s| s.to_string()).collect(),
            ..MdHashOptions::default()
        }
    }

    fn flat(value: &StoredHashValue) -> String {
        match value {
            StoredHashValue::Flat(s) => s.clone(),
            StoredHashValue::Detailed(_) => panic!("expected a flat stored value"),
        }
    }

    /// The identity guard: when the ignore policy changes, the written baseline
    /// must be computed under the CURRENT policy, never reuse the stored-policy
    /// comparison artifact. Reusing it here would silently persist a hash
    /// computed under the OLD ignore-set while recording the NEW `ignored` list —
    /// a corrupt baseline that reads as unchanged forever after.
    #[test]
    fn ignore_policy_change_writes_the_current_policy_baseline() {
        let doc = doc();

        // Stored under the old policy: nothing extra ignored.
        let old_opts = opts_ignoring(&[]);
        let stored = StoredHash {
            kind: MdHashKind::Simple,
            value: doc.compute_hash(MdHashKind::Simple, &old_opts).to_stored_value(),
            ignored: Vec::new(),
        };

        // Now `draft` is ignored: a different artifact.
        let new_opts = opts_ignoring(&["draft"]);
        let decision = doc.plan_hash_save(Some(&stored), &new_opts).unwrap();

        let written = decision
            .new_stored
            .as_ref()
            .expect("an ignore-policy change must rewrite the stored hash");

        let expected_current = doc
            .compute_hash(MdHashKind::Simple, &new_opts)
            .to_stored_value();
        assert_eq!(
            flat(&written.value),
            flat(&expected_current),
            "baseline must be the CURRENT-policy artifact"
        );
        assert_ne!(
            flat(&written.value),
            flat(&stored.value),
            "the current-policy artifact must differ from the stored-policy one here, \
             otherwise this test cannot detect conflation"
        );
        assert_eq!(written.ignored, vec!["draft".to_string()]);
        assert!(
            !decision.bump_last_updated,
            "an ignore-policy-only change must not bump last_updated"
        );
    }

    /// A kind change is the other divergent identity: the baseline is written at
    /// the SELECTED kind, not the stored one.
    #[test]
    fn kind_change_writes_the_selected_kind_baseline() {
        let doc = doc();
        let base = MdHashOptions::default();
        let stored = StoredHash {
            kind: MdHashKind::Simple,
            value: doc.compute_hash(MdHashKind::Simple, &base).to_stored_value(),
            ignored: Vec::new(),
        };

        let upgrade = MdHashOptions {
            forced_kind: Some(MdHashKind::Structured),
            ..MdHashOptions::default()
        };
        let decision = doc.plan_hash_save(Some(&stored), &upgrade).unwrap();

        let written = decision.new_stored.as_ref().expect("kind change rewrites");
        assert_eq!(written.kind, MdHashKind::Structured);
        assert_eq!(
            flat(&written.value),
            flat(&doc.compute_hash(MdHashKind::Structured, &upgrade).to_stored_value()),
            "baseline must be computed at the SELECTED kind"
        );
        assert_eq!(
            flat(&written.value).split('-').count(),
            4,
            "a structured value carries four components"
        );
    }

    /// The shared-identity case the reuse exists for: same kind, same policy,
    /// changed content. The baseline is the same artifact as the comparison, so
    /// reusing it must produce exactly what a fresh computation would.
    #[test]
    fn same_identity_reuse_matches_a_fresh_computation() {
        let doc = doc();
        let opts = MdHashOptions::default();

        // Stored from a DIFFERENT document, so content is changed and a baseline
        // is actually written.
        let previous = md("---\ntitle: T\ndraft: true\nnotes: keep\n---\n# H\n\nOld body.");
        let stored = StoredHash {
            kind: MdHashKind::Simple,
            value: previous
                .compute_hash(MdHashKind::Simple, &opts)
                .to_stored_value(),
            ignored: Vec::new(),
        };

        let decision = doc.plan_hash_save(Some(&stored), &opts).unwrap();
        let written = decision.new_stored.as_ref().expect("content change rewrites");

        assert_eq!(
            flat(&written.value),
            flat(&doc.compute_hash(MdHashKind::Simple, &opts).to_stored_value()),
            "the reused artifact must equal a freshly computed baseline"
        );
        assert!(decision.bump_last_updated);
    }

    /// Round trip: what `--save` writes must read back as unchanged, and a second
    /// `--save` must then leave the file untouched. This is the property the
    /// artifact reuse could break without any single-call assertion noticing.
    #[test]
    fn saved_baseline_reads_back_as_unchanged() {
        for extra in [vec![], vec!["draft"]] {
            for kind in [
                MdHashKind::Simple,
                MdHashKind::Structured,
                MdHashKind::Detailed,
                MdHashKind::Fm,
                MdHashKind::Body,
            ] {
                let doc = doc();
                let opts = MdHashOptions {
                    forced_kind: Some(kind),
                    extra_ignored: extra.iter().map(|s| s.to_string()).collect(),
                    ..MdHashOptions::default()
                };

                // First save: no stored hash -> first baseline.
                let first = doc.plan_hash_save(None, &opts).unwrap();
                let written = first.new_stored.expect("first baseline is written");

                // Read it back: the document must compare as unchanged...
                let comparison = doc.compare_hash(&written, &opts).unwrap();
                assert!(
                    !comparison.frontmatter_changed && !comparison.body_changed,
                    "{kind:?} with ignored={extra:?}: freshly saved baseline reads as changed"
                );
                assert!(
                    comparison.ignore_policy.is_none(),
                    "{kind:?} with ignored={extra:?}: saved baseline records a divergent policy"
                );

                // ...and a second save must leave the file untouched.
                let second = doc.plan_hash_save(Some(&written), &opts).unwrap();
                assert!(
                    second.new_stored.is_none(),
                    "{kind:?} with ignored={extra:?}: re-saving an unchanged document rewrote it"
                );
                assert!(!second.bump_last_updated);
            }
        }
    }

    /// The structural bound for `--save`: the two artifacts it legitimately
    /// needs are the stored-policy comparison and the current-policy baseline.
    /// When those identities coincide there is only one, and planning plus
    /// explanation together must not exceed that count.
    ///
    /// Routing the CLI through `plan_hash_save` and then `explain_hash_diff`
    /// hashed the stored-policy artifact a second time;
    /// [`Markdown::plan_hash_save_explained`] is the seam that shares it.
    #[test]
    fn save_computes_one_artifact_per_distinct_identity() {
        let doc = doc();

        // Same kind, same policy, changed content: one shared identity.
        let opts = MdHashOptions::default();
        let previous = md("---\ntitle: T\ndraft: true\nnotes: keep\n---\n# H\n\nOld body.");
        let stored = StoredHash {
            kind: MdHashKind::Simple,
            value: previous
                .compute_hash(MdHashKind::Simple, &opts)
                .to_stored_value(),
            ignored: Vec::new(),
        };
        let (result, calls) = probe::count_calls(|| doc.plan_hash_save_explained(Some(&stored), &opts));
        result.expect("plan succeeds");
        assert_eq!(
            calls, 1,
            "same kind and policy is ONE artifact for planning, baseline, and explanation",
        );

        // Divergent identity: the stored-policy comparison and the
        // current-policy baseline are genuinely different artifacts, so two is
        // the floor here — collapsing them would corrupt the baseline.
        let new_policy = opts_ignoring(&["draft"]);
        let (result, calls) =
            probe::count_calls(|| doc.plan_hash_save_explained(Some(&stored), &new_policy));
        result.expect("plan succeeds");
        assert_eq!(
            calls, 2,
            "an ignore-policy change keeps its two distinct artifacts, and adds no third",
        );

        // No stored hash: only the first baseline, and nothing to explain.
        let (result, calls) = probe::count_calls(|| doc.plan_hash_save_explained(None, &opts));
        let (_, explanation) = result.expect("plan succeeds");
        assert!(explanation.is_none(), "a first baseline has nothing to compare against");
        assert_eq!(calls, 1, "a first baseline is a single artifact");
    }

    /// `plan_hash_save_explained` must agree with the two operations the CLI
    /// previously composed, so `--save` output stays byte-identical.
    #[test]
    fn save_explanation_matches_the_separate_operations() {
        let doc = doc();
        for opts in [MdHashOptions::default(), opts_ignoring(&["draft"])] {
            for kind in [MdHashKind::Simple, MdHashKind::Structured, MdHashKind::Detailed] {
                let previous = md("---\ntitle: T\ndraft: true\nnotes: keep\n---\n# H\n\nOld body.");
                let stored = StoredHash {
                    kind,
                    value: previous.compute_hash(kind, &opts).to_stored_value(),
                    ignored: normalize_extras(&opts.extra_ignored),
                };

                let (decision, explanation) =
                    doc.plan_hash_save_explained(Some(&stored), &opts).unwrap();

                assert_eq!(decision, doc.plan_hash_save(Some(&stored), &opts).unwrap());
                assert_eq!(
                    explanation.expect("a stored hash yields an explanation").render(),
                    doc.explain_hash_diff(&stored, &opts).unwrap().render(),
                    "{kind:?}: --save explanation drifted",
                );
            }
        }
    }
}
