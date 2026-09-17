//! Checked `ctx.*` classification.
//!
//! A `ctx.<key>` read has four distinct answers that the unchecked
//! `Option<Value>` lookups collapse into one: a captured value (typed
//! `null`/`""`/`[]`/`{}` included), a known key whose owning group was never
//! captured, a captured group whose projection omitted one of its own keys, and
//! a key no group owns. Ownership comes only from [`ContextGroup::for_key`] and
//! capture state only from the snapshot's captured groups; there is no second
//! key list. Contract: `fixes/2026-08-02-silent-empty-ctx-values/error-taxonomy.md`.

use serde_json::Value;

use super::capture::ContextGroup;
use crate::markdown::compose::expression::ExpressionError;

/// Classification of one `ctx.<key>` read against an authoritative snapshot.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CtxLookupOutcome {
    /// The owning group was captured and the projection holds the key.
    Present(Value),
    /// The key is cataloged, but the snapshot never captured its group.
    NotCaptured { key: String, group: ContextGroup },
    /// The group was captured, but its projection omitted a cataloged key.
    ProjectionMissing { key: String, group: ContextGroup },
    /// No group owns the key; the unknown-context-variable path owns this case.
    Unknown,
}

impl CtxLookupOutcome {
    /// Classifies `key` given the snapshot's capture state and projection.
    ///
    /// `projection` must read the captured values only, never user-authored
    /// `ctx` data, so authored values cannot mask a missing projection.
    pub(crate) fn classify(
        key: &str,
        is_captured: impl FnOnce(ContextGroup) -> bool,
        projection: impl FnOnce(&str) -> Option<Value>,
    ) -> Self {
        let Some(group) = ContextGroup::for_key(key) else {
            return Self::Unknown;
        };
        if !is_captured(group) {
            return Self::NotCaptured { key: key.to_string(), group };
        }
        match projection(key) {
            Some(value) => Self::Present(value),
            None => Self::ProjectionMissing { key: key.to_string(), group },
        }
    }

    /// Answers an explicit `ctx.<key>` read on the evaluator channel.
    ///
    /// `unknown` supplies the unchecked answer for a key no group owns, so
    /// authored custom `ctx` keys still resolve.
    pub(crate) fn into_checked(
        self,
        unknown: impl FnOnce() -> Option<Value>,
    ) -> Result<Option<Value>, ExpressionError> {
        match self {
            Self::Present(value) => Ok(Some(value)),
            Self::Unknown => Ok(unknown()),
            Self::NotCaptured { key, group } => Err(ExpressionError::ContextNotCaptured { key, group }),
            Self::ProjectionMissing { key, group } => {
                Err(ExpressionError::ContextProjectionInvariant { key, group })
            }
        }
    }

    /// Answers a bare name (`when="repo"`) that fell back to the ctx namespace.
    ///
    /// A bare name is primarily a document variable, so an uncaptured group
    /// resolves as an undefined name rather than a missing capture; a captured
    /// group still enforces the projection invariant.
    pub(crate) fn into_checked_bare_name(
        self,
        unknown: impl FnOnce() -> Option<Value>,
    ) -> Result<Option<Value>, ExpressionError> {
        match self {
            Self::NotCaptured { .. } => Ok(None),
            outcome => outcome.into_checked(unknown),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::{EvaluationLookup, evaluate, parse};
    use crate::markdown::compose::frontmatter_interpolation::FrontmatterSeedState;
    use crate::markdown::compose::interpolation::{EvalResult, Evaluator};
    use crate::markdown::compose::{ComposeContext, EffectiveState};
    use serde_json::json;
    use std::collections::HashMap;

    fn state(frontmatter: Value, context: ComposeContext) -> EffectiveState {
        let frontmatter: HashMap<String, Value> = match frontmatter {
            Value::Object(map) => map.into_iter().collect(),
            _ => HashMap::new(),
        };
        EffectiveState::new(&frontmatter, None, context)
    }

    fn eval(expression: &str, lookup: &impl EvaluationLookup) -> Result<Value, ExpressionError> {
        evaluate(&parse(expression).expect("expression parses"), lookup)
    }

    #[test]
    fn captured_typed_null_and_empty_values_are_present() {
        let context = ComposeContext::fixed_for_testing_with([
            ("branch", Value::Null),
            ("repo_root", json!("")),
            ("dirty_files", json!([])),
            ("gpu", json!({})),
        ]);

        for (key, want) in [
            ("branch", Value::Null),
            ("repo_root", json!("")),
            ("dirty_files", json!([])),
            ("gpu", json!({})),
            ("today", json!("2024-06-15")),
        ] {
            assert_eq!(context.classify_ctx_key(key), CtxLookupOutcome::Present(want.clone()));
            let state = state(json!({}), context.clone());
            assert_eq!(
                eval(&format!("ctx.{key}"), &state).unwrap(),
                want,
                "ctx.{key} must evaluate to its captured value without a diagnostic",
            );
        }
    }

    #[test]
    fn a_known_key_of_an_uncaptured_group_is_not_captured() {
        let context = ComposeContext::fixed_for_testing();

        assert_eq!(
            context.classify_ctx_key("repo_root"),
            CtxLookupOutcome::NotCaptured { key: "repo_root".into(), group: ContextGroup::Repo },
        );
        let state = state(json!({}), context);
        assert!(matches!(
            eval("ctx.repo_root", &state),
            Err(ExpressionError::ContextNotCaptured { ref key, group: ContextGroup::Repo })
                if key == "repo_root"
        ));
        // The unchecked public accessor keeps its compatibility answer.
        assert_eq!(state.get("ctx.repo_root"), None);
    }

    #[test]
    fn a_captured_group_missing_a_cataloged_key_is_an_internal_invariant() {
        let context = ComposeContext::fixed_for_testing().with_projection_key_removed("timezone");

        assert_eq!(context.classify_ctx_key("timezone"),
            CtxLookupOutcome::ProjectionMissing {
                key: "timezone".into(),
                group: ContextGroup::DateTime,
            },
        );
        let state = state(json!({}), context);
        assert!(matches!(
            eval("ctx.timezone", &state),
            Err(ExpressionError::ContextProjectionInvariant {
                ref key,
                group: ContextGroup::DateTime,
            }) if key == "timezone"
        ));
    }

    #[test]
    fn an_unknown_key_keeps_the_unchecked_lookup() {
        let context = ComposeContext::fixed_for_testing();
        assert_eq!(context.classify_ctx_key("oss"), CtxLookupOutcome::Unknown);

        let state = state(json!({ "ctx": { "custom": "authored" } }), context);
        assert_eq!(eval("ctx.oss", &state).unwrap(), Value::Null);
        assert_eq!(eval("ctx.custom", &state).unwrap(), json!("authored"));
    }

    #[test]
    fn date_time_aliases_classify_under_their_group() {
        let context = ComposeContext::fixed_for_testing();
        for alias in ["utc", "dow", "dow_abbr"] {
            assert!(
                matches!(context.classify_ctx_key(alias), CtxLookupOutcome::Present(_)),
                "alias {alias} must be projected by a DateTime capture",
            );
        }
    }

    /// Ruling 1: authored `ctx` values never satisfy an uncaptured group, but
    /// still override the projection once the group is captured.
    #[test]
    fn an_authored_ctx_value_cannot_mask_an_uncaptured_group() {
        let authored = json!({ "ctx": { "os": "authored-os" } });

        let uncaptured = state(authored.clone(), ComposeContext::fixed_for_testing());
        assert!(matches!(
            eval("ctx.os", &uncaptured),
            Err(ExpressionError::ContextNotCaptured { group: ContextGroup::Os, .. })
        ));
        // The unchecked public accessor keeps its compatibility answer.
        assert_eq!(uncaptured.get("ctx.os"), Some(json!("authored-os")));

        let captured = state(
            authored,
            ComposeContext::fixed_for_testing_with([("os", json!("macos"))]),
        );
        assert_eq!(eval("ctx.os", &captured).unwrap(), json!("authored-os"));
    }

    /// Ruling 8: a bare name that falls back to the ctx namespace is a document
    /// variable first.
    #[test]
    fn a_bare_name_fallback_is_undefined_when_its_group_is_uncaptured() {
        let uncaptured = state(json!({}), ComposeContext::fixed_for_testing());
        assert_eq!(uncaptured.get_checked("repo_root").unwrap(), None);

        let captured = state(
            json!({}),
            ComposeContext::fixed_for_testing_with([("repo_root", json!("/repo"))]),
        );
        assert_eq!(captured.get_checked("repo_root").unwrap(), Some(json!("/repo")));

        let shadowed = state(json!({ "repo_root": "frontmatter" }), ComposeContext::fixed_for_testing());
        assert_eq!(shadowed.get_checked("repo_root").unwrap(), Some(json!("frontmatter")));

        let malformed = state(
            json!({}),
            ComposeContext::fixed_for_testing().with_projection_key_removed("today"),
        );
        assert!(matches!(
            malformed.get_checked("today"),
            Err(ExpressionError::ContextProjectionInvariant { .. })
        ));
    }

    /// Only evaluated references are checked: an unchosen ternary branch and a
    /// short-circuited fallback never read their `ctx.*` operand.
    #[test]
    fn unevaluated_operands_raise_nothing() {
        let state = state(json!({}), ComposeContext::fixed_for_testing());
        assert_eq!(eval("false ? ctx.repo_root : 'x'", &state).unwrap(), json!("x"));
        assert_eq!(eval("'set' || ctx.repo_root", &state).unwrap(), json!("set"));
        assert!(eval("true ? ctx.repo_root : 'x'", &state).is_err());
    }

    /// The interpolation evaluator's variable fast path uses the same channel.
    #[test]
    fn the_interpolation_fast_path_surfaces_the_typed_error() {
        let state = state(json!({}), ComposeContext::fixed_for_testing());
        let evaluator = Evaluator::new(&state);

        match evaluator.eval(&parse("ctx.repo_root").unwrap()) {
            EvalResult::Error { error, .. } => {
                assert!(matches!(error, ExpressionError::ContextNotCaptured { .. }));
                assert!(error.is_authoring_fatal());
            }
            other => panic!("expected a typed error, got {other:?}"),
        }
    }

    /// Both compose lookups raise the missing-capture error; neither falls back
    /// to an empty answer.
    #[test]
    fn compose_lookups_raise_not_captured() {
        let state = state(json!({}), ComposeContext::fixed_for_testing());
        assert!(matches!(
            eval("ctx.repo_root", &state),
            Err(ExpressionError::ContextNotCaptured { group: ContextGroup::Repo, .. })
        ));

        let seed = FrontmatterSeedState::new(HashMap::new(), ComposeContext::fixed_for_testing());
        assert!(matches!(
            seed.get_checked("ctx.repo_root"),
            Err(ExpressionError::ContextNotCaptured { group: ContextGroup::Repo, .. })
        ));
    }

    /// Verification #6 end to end: a malformed projection fails composition as
    /// an internal invariant on the body and condition surfaces, in lenient mode.
    #[test]
    fn composition_fails_on_a_malformed_projection() {
        use crate::markdown::Markdown;
        use crate::markdown::compose::ComposeOptions;

        for content in [
            "today={{ ctx.today }}\n",
            "::block when=\"ctx.timezone == 'UTC'\"\ninside\n::end-block\n",
        ] {
            let context = ComposeContext::fixed_for_testing()
                .with_projection_key_removed("today")
                .with_projection_key_removed("timezone");
            let error = Markdown::from(content)
                .compose_with(ComposeOptions::new_with_context(context).with_fail_fast(false))
                .expect_err("a malformed projection must fail composition");
            assert!(
                matches!(
                    error.missing_runtime_context(),
                    Some(ExpressionError::ContextProjectionInvariant { group: ContextGroup::DateTime, .. })
                ),
                "{content}: {error:?}"
            );
            assert!(error.to_string().contains("Darkmatter bug"), "{error}");
        }
    }

    #[test]
    fn the_frontmatter_seed_lookup_classifies_through_the_same_path() {
        let seed = FrontmatterSeedState::new(
            HashMap::new(),
            ComposeContext::fixed_for_testing().with_projection_key_removed("today"),
        );
        assert!(matches!(
            seed.get_checked("ctx.today"),
            Err(ExpressionError::ContextProjectionInvariant { group: ContextGroup::DateTime, .. })
        ));
        assert_eq!(seed.get_checked("ctx.oss").unwrap(), None);
        assert_eq!(seed.get_checked("ctx.year").unwrap(), Some(json!("2024")));
    }
}
