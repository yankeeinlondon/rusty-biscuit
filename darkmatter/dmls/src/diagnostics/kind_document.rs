//! Diagnostics for a YAML document validated by its top-level `kind`.
//!
//! Each problem of the overlay's kind report becomes one `darkmatter.schema`
//! diagnostic, coded, graded, and ranged as a Markdown frontmatter problem is
//! ([`generic_problem_diagnostics`]): a missing key ranges its parent mapping
//! (the root mapping for a top-level key), an unknown key ranges the key, and
//! a type, enum, or constraint problem ranges the value. The envelope
//! diagnostics (standalone schema and trigger) still run beside these, and
//! own a definition the payload grammar rejects: once the envelope has
//! reported its error, which locates the offending token, the kind report's
//! coarser meta-type problem for the same text is not repeated.

use lsp_types::Diagnostic;

use crate::diagnostics::frontmatter::{generic_problem_diagnostics, is_meta_type_problem};
use crate::overlay::SuggestionState;
use crate::providers::DocumentContext;

/// Appends the kind diagnostics of the document in `ctx` to `out`; none for a
/// document no shipped kind claims, or whose tree is a stale last-good tree.
pub(crate) fn diagnostics(ctx: &DocumentContext, out: &mut Vec<Diagnostic>) {
    let Some(overlay) = ctx.overlay.filter(|overlay| !overlay.stale) else {
        return;
    };
    let Some(kind) = &overlay.kind_document else {
        return;
    };
    let envelope_reported = matches!(
        &overlay.suggestions,
        SuggestionState::Standalone { error: Some(_), .. }
            | SuggestionState::TriggerError(_)
            | SuggestionState::TriggerWarning(_)
    );
    let problems = kind
        .report
        .problems
        .iter()
        .filter(|problem| !(envelope_reported && is_meta_type_problem(problem)));
    generic_problem_diagnostics(ctx, &kind.ast, &kind.bundle, problems, out);
}
