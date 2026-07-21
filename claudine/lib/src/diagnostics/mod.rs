//! Diagnostic facets — the handleable classification contract.
//!
//! This module is the **handling** counterpart to biscuit-terminal's
//! [`BlockError`] **rendering** contract. Where `BlockError` structures an
//! error so a *human reads* it well, [`Diagnostic`] structures the same typed
//! error so a *program reacts* to it well — a Rust caller or a prompt-document
//! author writing a `when:` clause. The two are one chain, not two: `Diagnostic`
//! is a **supertrait** of `BlockError`, so render and classify resolve through
//! one walk — [`select_effective_diagnostic`], whose rule is role-based (the
//! first [`Semantic`](DiagnosticRole::Semantic) diagnostic wins; a chain of only
//! [`Transparent`](DiagnosticRole::Transparent) wrappers falls through to its
//! deepest candidate). See
//! `claudine/features/_completed/2026-06-28-real-errors/error-structure.md` §2
//! and §6 for the model, and `docs/topics/error-architecture.md` for the seam.
//!
//! Every handleable error exposes five facets plus a default-derived severity:
//!
//! - [`category`](Diagnostic::category) — coarse domain (closed enum, 12 values)
//! - [`code`](Diagnostic::code) — stable dotted id, the public API contract
//! - [`disposition`](Diagnostic::disposition) — generic strategy (the reuse enabler)
//! - [`origin`](Diagnostic::origin) — who must remediate
//! - [`detail`](Diagnostic::detail) — typed instance payload, projected to `err.detail.*`
//! - [`severity`](Diagnostic::severity) — operator-facing, defaulted from disposition
//!
//! The closed facet enums live in [`facets`]; the locked code catalog (the
//! single source of truth `claudine errors` introspects) lives in [`registry`].
//! [`discovery`] decides *which* error in a chain answers those questions, and
//! [`snapshot`] projects the answer once for anything that leaves the process.
//!
//! ## Wired implementations
//!
//! The concrete `Diagnostic` projections are live:
//!
//! - [`CompositionError`] projects every composition code. For
//!   `composition.invalid_file_reference` it supplies the wrapper context
//!   (`source_path`, `property`, `event`) and merges whichever resolver path
//!   reached its `#[source]`. The shared `biscuit-file`/harness resolver
//!   populates `failure` on both arms; when its probe ran it additionally
//!   projects `kind`, `repository_root`, and the ordered `candidates` from
//!   the retained plan (`HarnessError::PathResolutionFailed`), and when no
//!   probe ran (`HarnessError::FileReferenceUnresolvable`, e.g. a
//!   syntactically-invalid reference) those three stay `null`. The legacy
//!   lower-layer path through `FileReferenceDiagnostic` (the
//!   markdown-interpolation arm) continues to supply the original five
//!   fields (`reference`, `kind`, `base_dir`, `suggestions`, `fallback_dir`)
//!   and reserves the six additions as `null`. Values come from typed data,
//!   never back-derived from `Display` (spec §D3).
//! - [`ClaudineError`] and [`HarnessError`] project the top-level Claudine,
//!   provider, io, config, and usage surface, so lifecycle `err.*` exposes
//!   their facets via [`LifecycleErrorInfo::from_claudine_error`] /
//!   [`from_harness_error`].
//! - Provider / cap / timeout / runaway failures arrive at the lifecycle `err`
//!   global as a synthesized `error_kind` string rather than a typed error;
//!   [`code_for_error_kind`] maps that label to its locked code so
//!   [`LifecycleErrorInfo::from_action_failure`] still projects the full facet
//!   surface. The per-instance values are unknowable from a label, so every
//!   declared `detail` key projects `null` — but the object stays
//!   catalog-shaped, because a registered code must never carry a *top-level*
//!   `null` detail.
//!
//! Every one of those constructors resolves through [`select_effective_diagnostic`],
//! the same walk `claudine-cli`'s error walker renders through and
//! [`DiagnosticSnapshot`] serializes through. That shared seam is what stops a
//! route from classifying one cause while rendering another.
//!
//! [`CompositionError`]: crate::composition::CompositionError
//! [`ClaudineError`]: crate::error::ClaudineError
//! [`HarnessError`]: crate::harness::error::HarnessError
//! [`LifecycleErrorInfo::from_claudine_error`]: crate::composition::lifecycle_context::LifecycleErrorInfo::from_claudine_error
//! [`from_harness_error`]: crate::composition::lifecycle_context::LifecycleErrorInfo::from_harness_error
//! [`LifecycleErrorInfo::from_action_failure`]: crate::composition::lifecycle_context::LifecycleErrorInfo::from_action_failure

mod discovery;
mod error_kind;
mod facets;
mod registry;
mod restored;
mod snapshot;

pub use discovery::{
    DiagnosticRole, EffectiveDiagnostic, MAX_SELECTION_DEPTH, as_diagnostic, next_registered_cause,
    select_effective_diagnostic,
};
pub use error_kind::code_for_error_kind;
pub use facets::{Category, Disposition, Origin, Severity};
pub use registry::{CODES, CodeSpec, code_spec};
pub use restored::RestoredDiagnostic;
pub use snapshot::{DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION, DiagnosticCause, DiagnosticSnapshot};

use std::error::Error as StdError;

use serde_json::{Map, Value};

use biscuit_terminal::errors::BlockError;

use crate::stream::badges::BadgeCategory;

/// The faceted classification an error exposes for *handling*.
///
/// A supertrait of [`BlockError`]: an implementor is simultaneously renderable
/// (human-facing) and classifiable (program-facing) from one typed cause. A
/// transparent wrapper delegates these facets to its meaningful cause; a layer
/// that deliberately classifies (e.g. "this stream error *is* a plan cap")
/// owns its facets and does not delegate (error-structure §6).
///
/// The facet strings ([`Category::as_str`], [`code`](Self::code), etc.) are a
/// versioned public contract — see [`registry`] for the locked catalog and its
/// additive-only evolution rule.
pub trait Diagnostic: BlockError {
    /// Coarse domain. Must equal the prefix of [`code`](Self::code).
    fn category(&self) -> Category;

    /// Stable dotted code (`cap.plan_limit`). The public API contract; should
    /// be a value present in [`CODES`].
    fn code(&self) -> &'static str;

    /// Generic-strategy facet — what class of response could resolve this.
    fn disposition(&self) -> Disposition;

    /// Who must remediate the error.
    fn origin(&self) -> Origin;

    /// Typed instance payload, projected via serde into the `err.detail.*`
    /// namespace (the same structured data the renderer captures — never a
    /// second copy). Defaults to [`Value::Null`] for errors that carry no
    /// per-instance specifics.
    fn detail(&self) -> Value {
        Value::Null
    }

    /// Operator-facing severity. Defaults to the disposition's default
    /// ([`Disposition::default_severity`]); override per code where the
    /// catalog specifies a non-default (e.g. `provider.context_pressure`).
    fn severity(&self) -> Severity {
        self.disposition().default_severity()
    }

    /// Whether this value speaks for the failure or defers to its cause.
    ///
    /// Defaults to [`DiagnosticRole::Semantic`]: owning the classification is
    /// the norm, and delegating it is the deliberate act. Override only for a
    /// wrapper whose `code`/`detail`/`status_block` all forward to an inner
    /// diagnostic.
    fn role(&self) -> DiagnosticRole {
        DiagnosticRole::Semantic
    }

    /// The next cause [`select_effective_diagnostic`] should visit.
    ///
    /// Defaults to [`Error::source`](StdError::source). Override where a
    /// wrapper holds its meaningful cause in a field `thiserror` does not
    /// expose as a source — `CompositionError`'s boxed wrappers keep `inner`
    /// off `#[source]` so `color_eyre`'s cause-chain fallback does not print
    /// the same `Display` text twice.
    fn diagnostic_source(&self) -> Option<&(dyn StdError + 'static)> {
        StdError::source(self)
    }
}

/// A `detail` object whose keys are exactly the field set the locked catalog
/// declares for `code`, every value pre-seeded to JSON `null`.
///
/// `detail()` projections call this to satisfy the registry contract
/// (error-catalog §2.5): every field a code lists must be a *present* key in
/// `err.detail`, so an absent optional projects as `null` rather than vanishing.
/// Each variant then overwrites the keys it can populate, leaving the rest
/// `null`. Returns an empty object for an unknown code (none of our mapped
/// codes are unknown, but the fallback keeps the projection total).
///
/// ## Examples
///
/// ```
/// use claudine::diagnostics::null_detail_for;
/// use serde_json::json;
///
/// let base = null_detail_for("io.read_failed");
/// assert_eq!(base, json!({ "path": null }));
/// ```
pub fn null_detail_for(code: &str) -> Value {
    let mut object = Map::new();
    if let Some(spec) = code_spec(code) {
        for &field in spec.detail {
            object.insert(field.to_string(), Value::Null);
        }
    }
    Value::Object(object)
}

/// Fold an operator [`BadgeCategory`] onto the unified [`Category`].
///
/// The ratified mapping from error-catalog §5: the badge taxonomy collapses
/// into the unified facet domain (`Billing`/`Quota`/`RateLimit` all become
/// [`Category::Cap`]; `Permission` joins [`Category::Auth`];
/// `ContextPressure` is a [`Category::Provider`] condition). This is the
/// data-level half of the "fold existing taxonomies" work; replacing the
/// in-place `BadgeCategory` *usage* with `Diagnostic` is part of the wiring
/// cascade.
pub fn category_from_badge(badge: BadgeCategory) -> Category {
    match badge {
        BadgeCategory::Auth | BadgeCategory::Permission => Category::Auth,
        BadgeCategory::Billing | BadgeCategory::Quota | BadgeCategory::RateLimit => Category::Cap,
        BadgeCategory::ContextPressure => Category::Provider,
        BadgeCategory::Config => Category::Config,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_categories_fold_per_catalog() {
        assert_eq!(category_from_badge(BadgeCategory::Auth), Category::Auth);
        assert_eq!(
            category_from_badge(BadgeCategory::Permission),
            Category::Auth
        );
        assert_eq!(category_from_badge(BadgeCategory::Billing), Category::Cap);
        assert_eq!(category_from_badge(BadgeCategory::Quota), Category::Cap);
        assert_eq!(category_from_badge(BadgeCategory::RateLimit), Category::Cap);
        assert_eq!(
            category_from_badge(BadgeCategory::ContextPressure),
            Category::Provider
        );
        assert_eq!(category_from_badge(BadgeCategory::Config), Category::Config);
    }

    #[test]
    fn folded_badge_category_is_a_registered_category() {
        // Every folded category must own at least one code in the registry,
        // so a handler keyed off the folded badge has somewhere to land.
        for badge in [
            BadgeCategory::Auth,
            BadgeCategory::Billing,
            BadgeCategory::Quota,
            BadgeCategory::RateLimit,
            BadgeCategory::ContextPressure,
            BadgeCategory::Permission,
            BadgeCategory::Config,
        ] {
            let cat = category_from_badge(badge);
            assert!(
                CODES.iter().any(|spec| spec.category == cat),
                "folded category `{}` has no codes",
                cat.as_str()
            );
        }
    }
}
