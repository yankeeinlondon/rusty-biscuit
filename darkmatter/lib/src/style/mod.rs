//! Frontmatter `style:` parser for darkmatter documents.
//!
//! Lowers a user-authored YAML `style:` block into a typed sparse tree
//! ([`StyleFrontmatter`]) whose length, alignment, and color values are
//! [`renderable`] runtime types. No rendering is performed by this module;
//! sub-specs #2..#7 (see
//! `renderable/features/_unscheduled/style-property/spec.md`) consume the
//! parsed value to drive `DarkmatterPage`.
//!
//! ## Examples
//!
//! ```no_run
//! use darkmatter::markdown::Markdown;
//! use darkmatter::style::{from_frontmatter, into_strict};
//!
//! let md = Markdown::try_from_content(
//!     "---\nstyle:\n  page:\n    left-margin: 2ch\n---\n",
//! )
//! .unwrap();
//! let (style, warnings) = from_frontmatter(md.frontmatter()).unwrap();
//!
//! // Schema-strict validation: succeeds on documents whose only warnings
//! // are `KnownButInactive`.
//! let _ = into_strict((style, warnings));
//! ```
//!
//! ## Notes
//!
//! Frontmatter values are stored as `serde_json::Value` (the canonical
//! `Frontmatter` map representation in darkmatter). YAML text parsing
//! happens upstream via `biscuit_file::serde_yaml_ng` and is not the
//! concern of this module.
//!
//! Diagnostics carry an optional [`StyleSpan`] for source positions; v1
//! always sets it to `None`, but the field exists so later sub-specs can
//! populate it without changing the public surface.

pub mod alignment;
pub mod apply;
pub mod bespoke;
pub mod cli_claims;
pub mod color;
pub mod descriptor;
pub mod error;
pub mod length;
pub mod parse;
pub mod schema;
pub mod walker;
pub mod warning;

#[cfg(test)]
mod coverage_tests;

pub use apply::{
    apply_color_style, apply_component_style, apply_disclosure_style, apply_hr_style,
    apply_list_style, apply_page_style, ComponentStyleOverrides, DisclosureStyleOverrides,
    HrStyleOverrides, ListStyleOverrides, PageStyleOverrides, StyleApplyError,
};
pub use bespoke::{
    apply_bespoke_style, BespokeStyleOverrides, MetaTag, PageMeta, PageStylesheet,
};
pub use apply::{map_hr_alignment, map_hr_kind, map_hr_weight};
pub use cli_claims::{
    apply_cli_claims, bespoke_style_overrides_from_claims,
    component_style_overrides_from_claims, disclosure_style_overrides_from_claims,
    hr_style_overrides_from_claims, list_style_overrides_from_claims, CliStyleClaims,
    FillClaim, page_style_overrides_from_claims,
};
pub use color::{lower_to_css, lower_to_sgr, wrap_with_color, StyleColor};
pub use error::StyleParseError;
pub use parse::{build_yaml_position_map, from_frontmatter, from_json_value, into_strict};
pub use schema::StyleFrontmatter;
pub use warning::{StyleSpan, StyleWarning, StyleWarningKind};

#[cfg(test)]
mod tests {
    /// Smoke test: the module is reachable and compiles.
    #[test]
    fn module_compiles() {
        // Intentionally empty. Existence is the assertion.
    }
}
