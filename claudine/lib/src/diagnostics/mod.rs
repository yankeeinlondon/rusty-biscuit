//! Diagnostic facets — the handleable classification contract.
//!
//! This module is the **handling** counterpart to biscuit-terminal's
//! [`BlockError`] **rendering** contract. Where `BlockError` structures an
//! error so a *human reads* it well, [`Diagnostic`] structures the same typed
//! error so a *program reacts* to it well — a Rust caller or a prompt-document
//! author writing a `when:` clause. The two are one chain, not two: `Diagnostic`
//! is a **supertrait** of `BlockError`, so render and classify resolve through
//! the same deepest-meaningful-cause walk (see
//! `claudine/features/2026-06-28-real-errors/error-structure.md` §2 and §6).
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
//!
//! ## Status
//!
//! This is the ratified, additive contract substrate. The `Diagnostic`
//! *implementations* for the concrete typed errors (composition, provider,
//! cap, lifecycle `err.*` projection) are wired in a later step — they depend
//! on the typed-error cascade that converts the still-`String`-typed error
//! boundaries, which is tracked in the Phase 2/3/6 implementation notes of the
//! feature plan.

mod facets;
mod registry;

pub use facets::{Category, Disposition, Origin, Severity};
pub use registry::{CODES, CodeSpec, code_spec};

use serde_json::Value;

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
