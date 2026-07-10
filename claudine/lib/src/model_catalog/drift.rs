//! Drift comparison between a provider's live model surface and the
//! generated expected-offerings baseline.
//!
//! Both functions are pure comparisons over data the caller already
//! holds — no subprocess, no network, no cache access — so the wrapper
//! can run them on the spawn path unconditionally. A `Some` result is a
//! [`SignalEvent::ModelCatalogDrift`] ready for
//! `SignalHub::emit_bespoke` with `SignalSource::Wrapper`
//! (design/model-catalog-boundary.md §"Runtime model_catalog migration"
//! step 2: the dynamic listing is drift-channel input, never a
//! validation source).

use claudine_catalog_types::{DriftObservation, SignalEvent};

use super::{ModelCatalogService, expected_baseline, expected_ids};
use crate::provider::Provider;

/// Diff a dynamic model listing against the expected-offerings baseline.
///
/// Ruling (2026-07-06): the comparison is scoped to namespaces the
/// expected set claims. Each expected id claims its prefix up to and
/// including the first `/` (`opencode/claude-x` claims `opencode/`);
/// ids without a `/` claim the bare namespace. Listed ids OUTSIDE every
/// claimed namespace are user-connected sources (e.g. a personal
/// `anthropic/...` connection surfaced by `opencode models`), not
/// catalog drift — they are ignored entirely.
///
/// Within a claimed namespace, a listed id counts as `unexpected` only
/// when [`ModelCatalogService::is_valid`] rejects it — which excludes
/// baseline ids, their aliases, user overrides, and offering-source
/// namespaces. `missing` is every expected id absent from the FULL
/// listing (case-insensitive; not namespace-scoped, since an expected
/// id vanishing from the listing is drift wherever the rest of the
/// listing points).
///
/// ## Returns
///
/// `None` when both sides are empty; otherwise a
/// [`SignalEvent::ModelCatalogDrift`] with
/// `observed_via: DriftObservation::Listing`.
pub fn listing_drift(
    provider: Provider,
    listed: &[String],
    service: &ModelCatalogService,
) -> Option<SignalEvent> {
    let expected = expected_ids(provider);
    let claimed: std::collections::HashSet<String> =
        expected.iter().map(|id| namespace_of(id)).collect();

    let unexpected: Vec<String> = listed
        .iter()
        .filter(|id| claimed.contains(&namespace_of(id)))
        .filter(|id| !service.is_valid(provider, id))
        .cloned()
        .collect();
    let missing: Vec<String> = expected
        .into_iter()
        .filter(|expected_id| {
            !listed
                .iter()
                .any(|listed_id| listed_id.eq_ignore_ascii_case(expected_id))
        })
        .collect();

    if unexpected.is_empty() && missing.is_empty() {
        return None;
    }
    Some(SignalEvent::ModelCatalogDrift {
        unexpected,
        missing,
        observed_via: DriftObservation::Listing,
    })
}

/// Check one stream-observed resolved model id against the baseline.
///
/// Ruling (2026-07-06): a relaxed match ladder avoids false positives
/// from providers that report fuller ids than the catalog carries (e.g.
/// a dated `claude-opus-4-8-20250801` against expected
/// `claude-opus-4-8`). The resolved id is matched — no drift — when any
/// of:
///
/// - [`ModelCatalogService::is_valid`] accepts it (baseline ids,
///   aliases, user overrides, offering-source namespaces);
/// - any expected id or alias is a case-insensitive prefix of it;
/// - it is a case-insensitive prefix of any expected id.
///
/// ## Returns
///
/// `None` when matched; otherwise a [`SignalEvent::ModelCatalogDrift`]
/// carrying just the resolved id, with
/// `observed_via: DriftObservation::ResolvedModel`.
pub fn resolved_model_drift(
    provider: Provider,
    resolved: &str,
    service: &ModelCatalogService,
) -> Option<SignalEvent> {
    if service.is_valid(provider, resolved) {
        return None;
    }
    let baseline_prefix_of_resolved = expected_baseline(provider)
        .iter()
        .any(|entry| starts_with_ignore_case(resolved, entry));
    let resolved_prefix_of_expected = expected_ids(provider)
        .iter()
        .any(|id| starts_with_ignore_case(id, resolved));
    if baseline_prefix_of_resolved || resolved_prefix_of_expected {
        return None;
    }
    Some(SignalEvent::ModelCatalogDrift {
        unexpected: vec![resolved.to_string()],
        missing: Vec::new(),
        observed_via: DriftObservation::ResolvedModel,
    })
}

/// A model id's namespace: the prefix up to and including the first `/`
/// (lowercased for case-insensitive membership), or the empty string for
/// bare ids.
fn namespace_of(id: &str) -> String {
    match id.find('/') {
        Some(pos) => id[..=pos].to_ascii_lowercase(),
        None => String::new(),
    }
}

/// Case-insensitive `haystack.starts_with(prefix)`; `get` keeps a
/// mid-codepoint boundary from panicking on non-ASCII ids.
fn starts_with_ignore_case(haystack: &str, prefix: &str) -> bool {
    haystack
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::config::claudine_config::ProviderModelOverride;

    fn assert_listing_event(
        event: SignalEvent,
        expected_unexpected: &[&str],
        expected_missing: &[&str],
    ) {
        match event {
            SignalEvent::ModelCatalogDrift {
                unexpected,
                missing,
                observed_via,
            } => {
                assert_eq!(unexpected, expected_unexpected);
                assert_eq!(missing, expected_missing);
                assert_eq!(observed_via, DriftObservation::Listing);
            }
            other => panic!("expected ModelCatalogDrift, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // listing_drift
    // -----------------------------------------------------------------

    #[test]
    fn full_listing_matches_baseline_returns_none() {
        let service = ModelCatalogService::new();
        let listed = expected_ids(Provider::OpenCode);
        assert_eq!(listing_drift(Provider::OpenCode, &listed, &service), None);
    }

    /// Ruling: ids outside every claimed namespace are user-connected
    /// sources, not catalog drift. All opencode expected ids ride
    /// `opencode/`, so an `anthropic/...` listing entry is ignored.
    #[test]
    fn opencode_listing_ignores_unclaimed_namespaces() {
        let service = ModelCatalogService::new();
        let mut listed = expected_ids(Provider::OpenCode);
        listed.push("anthropic/claude-x".into());
        listed.push("ollama/llama3.3".into());
        assert_eq!(listing_drift(Provider::OpenCode, &listed, &service), None);
    }

    #[test]
    fn opencode_listing_flags_unknown_id_in_claimed_namespace() {
        let service = ModelCatalogService::new();
        let mut listed = expected_ids(Provider::OpenCode);
        listed.push("opencode/not-in-catalog".into());
        let event = listing_drift(Provider::OpenCode, &listed, &service)
            .expect("unknown claimed-namespace id is drift");
        assert_listing_event(event, &["opencode/not-in-catalog"], &[]);
    }

    #[test]
    fn opencode_listing_detects_missing_expected_id() {
        let service = ModelCatalogService::new();
        let mut listed = expected_ids(Provider::OpenCode);
        let dropped = listed.pop().expect("opencode has expected offerings");
        let event = listing_drift(Provider::OpenCode, &listed, &service)
            .expect("dropped expected id is drift");
        assert_listing_event(event, &[], &[dropped.as_str()]);
    }

    /// Missing comparison is case-insensitive: a re-cased listing entry
    /// still covers its expected id.
    #[test]
    fn opencode_listing_missing_check_is_case_insensitive() {
        let service = ModelCatalogService::new();
        let listed: Vec<String> = expected_ids(Provider::OpenCode)
            .into_iter()
            .map(|id| id.to_ascii_uppercase())
            .collect();
        assert_eq!(listing_drift(Provider::OpenCode, &listed, &service), None);
    }

    /// `is_valid` gating means user overrides suppress the unexpected
    /// classification for ids the user explicitly added.
    #[test]
    fn opencode_listing_accepts_user_override_id() {
        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::OpenCode,
            ProviderModelOverride::AddList(vec!["opencode/custom-model".into()]),
        );
        let service = ModelCatalogService::with_overrides(overrides);
        let mut listed = expected_ids(Provider::OpenCode);
        listed.push("opencode/custom-model".into());
        assert_eq!(listing_drift(Provider::OpenCode, &listed, &service), None);
    }

    /// Claude's expected ids are bare (no `/`), so the bare namespace is
    /// claimed — and an alias like `opus` in the listing passes through
    /// `is_valid` rather than being flagged.
    #[test]
    fn bare_namespace_listing_excludes_aliases_from_unexpected() {
        let service = ModelCatalogService::new();
        let mut listed = expected_ids(Provider::Claude);
        listed.push("opus".into());
        assert_eq!(listing_drift(Provider::Claude, &listed, &service), None);

        listed.push("totally-unknown-model".into());
        let event = listing_drift(Provider::Claude, &listed, &service)
            .expect("unknown bare-namespace id is drift");
        assert_listing_event(event, &["totally-unknown-model"], &[]);
    }

    // -----------------------------------------------------------------
    // resolved_model_drift
    // -----------------------------------------------------------------

    fn assert_resolved_event(event: SignalEvent, expected_id: &str) {
        match event {
            SignalEvent::ModelCatalogDrift {
                unexpected,
                missing,
                observed_via,
            } => {
                assert_eq!(unexpected, vec![expected_id.to_string()]);
                assert!(missing.is_empty());
                assert_eq!(observed_via, DriftObservation::ResolvedModel);
            }
            other => panic!("expected ModelCatalogDrift, got {other:?}"),
        }
    }

    #[test]
    fn resolved_exact_id_and_alias_match() {
        let service = ModelCatalogService::new();
        assert_eq!(
            resolved_model_drift(Provider::Claude, "claude-opus-4-8", &service),
            None
        );
        assert_eq!(resolved_model_drift(Provider::Claude, "opus", &service), None);
    }

    /// Ruling: a dated fuller id than the catalog carries is matched via
    /// the expected-id-is-prefix arm.
    #[test]
    fn resolved_dated_id_matches_via_expected_prefix() {
        let service = ModelCatalogService::new();
        assert_eq!(
            resolved_model_drift(Provider::Claude, "claude-opus-4-8-20250801", &service),
            None
        );
        assert_eq!(
            resolved_model_drift(Provider::Claude, "CLAUDE-OPUS-4-8-20250801", &service),
            None
        );
    }

    /// Ruling: a resolved id shorter than the catalog id is matched via
    /// the resolved-is-prefix arm.
    #[test]
    fn resolved_truncated_id_matches_via_resolved_prefix() {
        let service = ModelCatalogService::new();
        assert_eq!(
            resolved_model_drift(Provider::Claude, "claude-opus", &service),
            None
        );
    }

    #[test]
    fn resolved_unknown_id_emits_drift() {
        let service = ModelCatalogService::new();
        let event = resolved_model_drift(Provider::Claude, "gpt-5.5", &service)
            .expect("foreign id is drift");
        assert_resolved_event(event, "gpt-5.5");
    }

    #[test]
    fn resolved_override_id_is_accepted() {
        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::Claude,
            ProviderModelOverride::AddList(vec!["my-custom-model".into()]),
        );
        let service = ModelCatalogService::with_overrides(overrides);
        assert_eq!(
            resolved_model_drift(Provider::Claude, "my-custom-model", &service),
            None
        );
    }

    /// Offering-source namespaces pass through `is_valid`, so a local
    /// runner id never reads as drift.
    #[test]
    fn resolved_offering_source_id_is_accepted() {
        let service = ModelCatalogService::new();
        assert_eq!(
            resolved_model_drift(Provider::OpenCode, "ollama/llama3.3", &service),
            None
        );
        let event = resolved_model_drift(Provider::OpenCode, "opencode/not-in-catalog", &service)
            .expect("unknown claimed-namespace id is drift");
        assert_resolved_event(event, "opencode/not-in-catalog");
    }
}
