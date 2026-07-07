//! `family_latest` resolution over the vendored family-index slice.
//!
//! WHY this exists: rolling aliases (`sonnet`, `flash`) re-point across
//! releases, so "which model did the alias mean" is only answerable
//! against a dated snapshot — the family index vendored from the
//! unchained-ai models-catalog artifact
//! (design/model-catalog-boundary.md "`family_latest` semantics"). The
//! concrete answer is stamped into session logs so reporting sees what
//! an alias meant *that session*: [`family_latest_stamp`] builds the
//! record here; the wrapper's summary writer attaches it to the
//! SessionEnd row.
//!
//! Staleness contract (ContentPolicy `on_stale: warn`): when the
//! artifact's `generated_at` is older than
//! [`FAMILY_LATEST_MAX_AGE_DAYS`], answers carry
//! [`Staleness::Stale`] — the warning travels WITH the answer and
//! resolution still succeeds.
//!
//! The resolved `latest` is an identity key naming a *release*, not an
//! offering (Checkpoint F): consumers pick a concrete offering from the
//! release's duplicate group by their own policy.

use std::sync::LazyLock;

use chrono::{DateTime, Utc};
pub use claudine_catalog_types::FamilyRow;
use claudine_catalog_types::{ResolvesVia, family_key};
use serde::{Deserialize, Serialize};

use super::families_generated::{ARTIFACT_GENERATED_AT, FAMILY_INDEX};
use crate::provider::provider_info;
use crate::provider_id::Provider;

/// ContentPolicy max age: answers computed against an artifact older
/// than this many days are [`Staleness::Stale`].
pub const FAMILY_LATEST_MAX_AGE_DAYS: u64 = 30;

/// Freshness of a `family_latest` answer relative to the vendored
/// artifact's `generated_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staleness {
    /// The artifact is within [`FAMILY_LATEST_MAX_AGE_DAYS`].
    Fresh,
    /// The artifact exceeded the max age; the answer may name an
    /// outdated release.
    Stale {
        /// Whole days since the artifact's `generated_at`.
        age_days: u64,
    },
}

/// A resolved `family_latest` answer.
#[derive(Debug, Clone, Copy)]
pub struct FamilyLatest {
    /// Identity key of the family's newest release.
    pub identity_key: &'static str,
    /// The full vendored family row (members, rolling aliases).
    pub family: &'static FamilyRow,
    pub staleness: Staleness,
}

/// The artifact stamp, parsed once — generated as RFC3339 by
/// claudine-gen, so a parse failure means a hand-edited generated file.
static GENERATED_AT: LazyLock<DateTime<Utc>> = LazyLock::new(|| {
    DateTime::parse_from_rfc3339(ARTIFACT_GENERATED_AT)
        .expect("generated ARTIFACT_GENERATED_AT is RFC3339")
        .with_timezone(&Utc)
});

fn staleness_at(now: DateTime<Utc>) -> Staleness {
    let age_days = (now - *GENERATED_AT).num_days();
    if age_days > FAMILY_LATEST_MAX_AGE_DAYS as i64 {
        Staleness::Stale {
            age_days: age_days as u64,
        }
    } else {
        Staleness::Fresh
    }
}

/// Resolves a `vendor/family` key to its newest release at `now`
/// (clock-injected core of [`family_latest`]).
///
/// Returns `None` for keys outside the vendored slice and for families
/// whose artifact entry carries no `latest`.
pub fn family_latest_at(family_key: &str, now: DateTime<Utc>) -> Option<FamilyLatest> {
    let row = FAMILY_INDEX
        .binary_search_by(|row| row.key.cmp(family_key))
        .ok()
        .map(|index| &FAMILY_INDEX[index])?;
    Some(FamilyLatest {
        identity_key: row.latest?,
        family: row,
        staleness: staleness_at(now),
    })
}

/// [`family_latest_at`] against the current clock.
pub fn family_latest(family_key: &str) -> Option<FamilyLatest> {
    family_latest_at(family_key, Utc::now())
}

/// Resolves a provider's rolling alias (a record marked
/// `resolves: family_latest`) through the family index at `now`.
///
/// Duplicate-alias records are guaranteed by generation to derive one
/// family key (cross-family duplicates lose their marks), so the first
/// marked carrier answers for all of them.
pub fn resolve_alias_at(
    provider: Provider,
    alias: &str,
    now: DateTime<Utc>,
) -> Option<FamilyLatest> {
    provider_info(provider)
        .expected_offerings
        .iter()
        .find_map(|offering| {
            (offering.resolves == Some(ResolvesVia::FamilyLatest)
                && offering.alias == Some(alias))
            .then_some(offering.catalog_id)
            .flatten()
        })
        .and_then(|catalog_id| family_latest_at(family_key(catalog_id), now))
}

/// [`resolve_alias_at`] against the current clock.
pub fn resolve_alias(provider: Provider, alias: &str) -> Option<FamilyLatest> {
    resolve_alias_at(provider, alias, Utc::now())
}

/// The session-log record of one `family_latest` alias resolution,
/// serialized into the SessionEnd summary row as `extra["family_latest"]`
/// so reporting can see which model an alias meant *that session*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyLatestStamp {
    /// The rolling alias the run requested (e.g. `opus`, `flash`).
    pub alias: String,
    /// The family's newest release identity key.
    pub identity_key: String,
    /// The `vendor/family` key the alias derived.
    pub family_key: String,
    /// The vendored artifact's `generated_at`, RFC3339 verbatim.
    pub artifact_generated_at: String,
    /// Whether the artifact exceeded [`FAMILY_LATEST_MAX_AGE_DAYS`] at
    /// stamp time.
    pub stale: bool,
    /// Whole days past `generated_at`; only carried when `stale`.
    pub age_days: Option<u64>,
}

/// Builds the stamp for a run whose requested model is a marked rolling
/// alias; `None` when `requested_model` is not such an alias.
///
/// Ruling (Checkpoint F): the stamp records the family-latest identity
/// key VERBATIM — including variant releases like
/// `google/gemini-pro@3.1+preview`. `latest` names a *release*, not an
/// offering, so no base-model election happens here; a consumer wanting
/// a base offering strips variants itself (deferred until such a
/// consumer exists).
pub fn family_latest_stamp(provider: Provider, requested_model: &str) -> Option<FamilyLatestStamp> {
    let hit = resolve_alias(provider, requested_model)?;
    let (stale, age_days) = match hit.staleness {
        Staleness::Fresh => (false, None),
        Staleness::Stale { age_days } => (true, Some(age_days)),
    };
    Some(FamilyLatestStamp {
        alias: requested_model.to_string(),
        identity_key: hit.identity_key.to_string(),
        family_key: hit.family.key.to_string(),
        artifact_generated_at: ARTIFACT_GENERATED_AT.to_string(),
        stale,
        age_days,
    })
}

#[cfg(test)]
mod tests {
    use chrono::Days;

    use super::*;

    fn generated_at() -> DateTime<Utc> {
        *GENERATED_AT
    }

    /// The resolver's answer for a known family must be the committed
    /// generated row's own `latest` — read from the module, not
    /// hardcoded, so an artifact refresh cannot silently invalidate the
    /// test.
    #[test]
    fn family_latest_returns_the_vendored_latest_release() {
        let row = FAMILY_INDEX
            .iter()
            .find(|row| row.key == "anthropic/claude-opus")
            .expect("claude joins pin anthropic/claude-opus into the slice");
        let expected = row.latest.expect("the opus family names a latest release");
        let hit = family_latest_at("anthropic/claude-opus", generated_at())
            .expect("vendored family resolves");
        assert_eq!(hit.identity_key, expected);
        assert_eq!(family_key(hit.identity_key), "anthropic/claude-opus");
        assert!(!hit.family.members.is_empty());
        assert_eq!(hit.staleness, Staleness::Fresh);
    }

    #[test]
    fn unknown_family_is_none() {
        assert!(family_latest_at("nobody/nothing", generated_at()).is_none());
    }

    /// The generated index must uphold the binary-search precondition.
    #[test]
    fn family_index_is_sorted_by_key() {
        assert!(FAMILY_INDEX.windows(2).all(|pair| pair[0].key < pair[1].key));
    }

    #[test]
    fn staleness_flips_past_the_max_age_and_travels_with_the_answer() {
        let fresh = family_latest_at(
            "anthropic/claude-opus",
            generated_at() + Days::new(FAMILY_LATEST_MAX_AGE_DAYS),
        )
        .unwrap();
        assert_eq!(fresh.staleness, Staleness::Fresh);

        let stale =
            family_latest_at("anthropic/claude-opus", generated_at() + Days::new(40)).unwrap();
        assert_eq!(stale.staleness, Staleness::Stale { age_days: 40 });
    }

    /// Claude's `opus`/`sonnet` rolling aliases resolve through their
    /// marked records to the vendored family latest.
    #[test]
    fn claude_rolling_aliases_resolve() {
        let now = generated_at();
        for (alias, family) in [
            ("opus", "anthropic/claude-opus"),
            ("sonnet", "anthropic/claude-sonnet"),
        ] {
            let hit = resolve_alias_at(Provider::Claude, alias, now)
                .unwrap_or_else(|| panic!("alias `{alias}` resolves"));
            assert_eq!(hit.family.key, family);
            assert_eq!(
                hit.identity_key,
                hit.family.latest.expect("resolved families carry latest")
            );
        }
    }

    /// Gemini's `flash` alias sits on TWO records (2.5 and 3-preview);
    /// both derive `google/gemini-flash`, so the duplicate collapses to
    /// one answer.
    #[test]
    fn gemini_duplicate_flash_alias_resolves_to_one_family() {
        let hit = resolve_alias_at(Provider::Gemini, "flash", generated_at())
            .expect("duplicate same-family alias resolves");
        assert_eq!(hit.family.key, "google/gemini-flash");
    }

    /// Gemini `auto` is an unjoined router id: alias without a
    /// `catalog_id` join is never marked, so it does not resolve.
    #[test]
    fn gemini_auto_router_alias_does_not_resolve() {
        assert!(resolve_alias_at(Provider::Gemini, "auto", generated_at()).is_none());
    }

    #[test]
    fn unknown_alias_is_none() {
        assert!(resolve_alias_at(Provider::Claude, "turbo", generated_at()).is_none());
    }

    /// The stamp echoes the resolver verbatim: alias, family key, and the
    /// vendored latest identity key, with the artifact's own timestamp.
    #[test]
    fn family_latest_stamp_records_resolution_verbatim() {
        let stamp = family_latest_stamp(Provider::Claude, "opus")
            .expect("claude `opus` is a marked rolling alias");
        let hit = resolve_alias(Provider::Claude, "opus").unwrap();
        assert_eq!(stamp.alias, "opus");
        assert_eq!(stamp.identity_key, hit.identity_key);
        assert_eq!(stamp.family_key, hit.family.key);
        assert_eq!(stamp.artifact_generated_at, ARTIFACT_GENERATED_AT);
        assert_eq!(stamp.stale, matches!(hit.staleness, Staleness::Stale { .. }));
    }

    #[test]
    fn family_latest_stamp_is_none_for_non_alias_models() {
        assert!(family_latest_stamp(Provider::Claude, "claude-sonnet-4-20250514").is_none());
        assert!(family_latest_stamp(Provider::Claude, "turbo").is_none());
    }

    #[test]
    fn family_latest_stamp_serde_round_trips() {
        let stamp = FamilyLatestStamp {
            alias: "flash".to_string(),
            identity_key: "google/gemini-flash@3+preview".to_string(),
            family_key: "google/gemini-flash".to_string(),
            artifact_generated_at: "2026-07-06T22:44:37.286219+00:00".to_string(),
            stale: true,
            age_days: Some(42),
        };
        let json = serde_json::to_string(&stamp).unwrap();
        let back: FamilyLatestStamp = serde_json::from_str(&json).unwrap();
        assert_eq!(back, stamp);
    }
}
