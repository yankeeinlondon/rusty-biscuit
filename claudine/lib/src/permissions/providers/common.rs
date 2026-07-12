// Format-agnostic helpers shared across provider policy backends.
//
// Provider backends that differ only in a literal or struct name delegate
// here instead of carrying their own copy. Genuinely format-specific
// helpers (TOML serialization, JSON array manipulation) stay local to
// the provider that owns them.

use std::collections::BTreeMap;

use crate::permissions::canonical::MappingFidelity;
use crate::permissions::mutation::OneShotMutationPlan;
use crate::permissions::native::NativeEffectivePolicy;

/// Returns the source-id of the first contributing source, falling back
/// to a provider-specific derived label when no sources are present.
///
/// Each provider passes its own fallback literal (e.g. `"codex-derived"`,
/// `"gemini-derived"`) so the derived-source label stays provider-identifiable.
pub(crate) fn first_source_id(native: &NativeEffectivePolicy, fallback: &str) -> String {
    native
        .sources
        .first()
        .map(|source| source.id.clone())
        .unwrap_or_else(|| fallback.to_owned())
}

/// Builds a one-shot mutation plan with no environment overrides.
///
/// Most providers produce one-shot plans that only carry argv — the env
/// map is empty. This constructor removes the repeated
/// `env: BTreeMap::new()` boilerplate across those providers.
pub(crate) fn one_shot_plan(argv: Vec<String>, fidelity: MappingFidelity) -> OneShotMutationPlan {
    OneShotMutationPlan {
        argv,
        env: BTreeMap::new(),
        fidelity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::native::{NativeEffectivePolicy, PolicySource, PolicySourceKind};
    use crate::provider::Provider;

    fn make_policy(sources: Vec<PolicySource>) -> NativeEffectivePolicy {
        NativeEffectivePolicy::new(Provider::Codex, sources, ())
    }

    fn make_source(id: &str) -> PolicySource {
        PolicySource {
            id: id.to_owned(),
            kind: PolicySourceKind::UserConfig,
            path: None,
            precedence: 0,
            writable: false,
        }
    }

    #[test]
    fn first_source_id_returns_first_source_id() {
        let policy = make_policy(vec![make_source("user-config"), make_source("repo-config")]);
        assert_eq!(first_source_id(&policy, "codex-derived"), "user-config");
    }

    #[test]
    fn first_source_id_falls_back_when_no_sources() {
        let policy = make_policy(vec![]);
        assert_eq!(first_source_id(&policy, "codex-derived"), "codex-derived");
    }

    #[test]
    fn first_source_id_fallback_is_provider_specific() {
        let policy = make_policy(vec![]);
        assert_eq!(first_source_id(&policy, "gemini-derived"), "gemini-derived");
    }

    #[test]
    fn one_shot_plan_has_empty_env() {
        let plan = one_shot_plan(
            vec!["--yolo".to_owned()],
            MappingFidelity::Approximate,
        );
        assert_eq!(plan.argv, vec!["--yolo".to_owned()]);
        assert!(plan.env.is_empty());
        assert_eq!(plan.fidelity, MappingFidelity::Approximate);
    }

    #[test]
    fn one_shot_plan_preserves_exact_fidelity() {
        let plan = one_shot_plan(vec![], MappingFidelity::Exact);
        assert_eq!(plan.fidelity, MappingFidelity::Exact);
    }
}
