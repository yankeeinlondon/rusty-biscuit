//! Crate-internal orchestration seam for `darkmatter-cli`. **Not public API.**
//!
//! Nothing here is covered by the crate's compatibility contract. It exists,
//! is `#[doc(hidden)]`, and is gated behind the non-default
//! `internal-hash-orchestration` feature purely so the `md` binary — which is a
//! separate crate — can reach `pub(crate)` items. It may change or disappear in
//! any release. External consumers must use [`Markdown::compare_hash`],
//! [`Markdown::explain_hash_diff`], and [`Markdown::plan_hash_save`].
//!
//! ## Why this exists
//!
//! The 2026-07-15 performance follow-up's Finding 35.5 requires that one
//! `md hash --diff` / `--save` invocation compute each unique
//! `(kind, effective hash options)` artifact at most once and share it between
//! that mode's comparison/planning and its explanation. The same feature's
//! compatibility invariant 2 bars it from adding public API shape.
//!
//! Those two requirements are jointly unsatisfiable through the ordinary public
//! API: the pairing methods must be reachable from `darkmatter-cli`, Rust has no
//! friend-crate visibility, and the products cannot be derived from the existing
//! public API by an outside crate ([`HashExplanation`]'s fields are private, so
//! only this crate can build one). Every alternative is worse:
//!
//! - memoizing artifacts on [`Markdown`] is **unsound** — `content_mut` and
//!   `frontmatter_mut` hand out `&mut` references, so no cache could be
//!   invalidated on mutation;
//! - reverting the CLI to the two-call path restores the double-compute,
//!   forfeiting the finding and leaving its methods dead code.
//!
//! So the seam is confined instead of widened: with default features the
//! crate's public surface is byte-identical to its pre-feature state, and the
//! only opt-in is this workspace's own CLI.
//!
//! [`Markdown`]: crate::markdown::Markdown
//! [`Markdown::compare_hash`]: crate::markdown::Markdown::compare_hash
//! [`Markdown::explain_hash_diff`]: crate::markdown::Markdown::explain_hash_diff
//! [`Markdown::plan_hash_save`]: crate::markdown::Markdown::plan_hash_save
//! [`HashExplanation`]: crate::markdown::hash::HashExplanation

use crate::markdown::hash::{
    HashComparison, HashExplanation, MdHashOptions, SaveDecision, StoredHash,
};
use crate::markdown::{Markdown, MarkdownResult};

/// The comparison and the explanation for one `md hash --diff`, computing the
/// like-for-like artifact once instead of once per product.
///
/// Both products are identical to those of the public
/// [`Markdown::compare_hash`] + [`Markdown::explain_hash_diff`] pair; only the
/// artifact count differs.
///
/// ## Errors
///
/// Propagates [`MarkdownError::MalformedStoredHash`] when the stored value is
/// malformed for its kind.
///
/// [`Markdown::compare_hash`]: crate::markdown::Markdown::compare_hash
/// [`Markdown::explain_hash_diff`]: crate::markdown::Markdown::explain_hash_diff
/// [`MarkdownError::MalformedStoredHash`]: crate::markdown::MarkdownError::MalformedStoredHash
pub fn diff_hash(
    md: &Markdown,
    stored: &StoredHash,
    options: &MdHashOptions,
) -> MarkdownResult<(HashComparison, HashExplanation)> {
    md.diff_hash(stored, options)
}

/// The save decision and the explanation `md hash --save` prints, sharing the
/// stored-policy artifact the planning already computed.
///
/// The explanation is `None` exactly when `stored` is `None`: a first baseline
/// has nothing to compare against. `--save`'s two artifacts — the stored-policy
/// comparison and the selected-policy baseline — remain distinct; they are
/// shared only when their identities provably agree.
///
/// ## Errors
///
/// Propagates [`MarkdownError::MalformedStoredHash`] when the stored value is
/// malformed for its kind.
///
/// [`MarkdownError::MalformedStoredHash`]: crate::markdown::MarkdownError::MalformedStoredHash
pub fn plan_hash_save_explained(
    md: &Markdown,
    stored: Option<&StoredHash>,
    options: &MdHashOptions,
) -> MarkdownResult<(SaveDecision, Option<HashExplanation>)> {
    md.plan_hash_save_explained(stored, options)
}
