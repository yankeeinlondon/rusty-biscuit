//! Terminal rendering for the provider / execution / file-reference error
//! family, plus the catch-all generic block.
//!
//! Covers the empty-composed-body abort, shell-expansion delegation, and every
//! variant without a dedicated block (which render their `Display` message).
//! The dispatcher in [`super`] routes this family here, including its `_` arm.

use super::super::*;
use biscuit_file::{FileReference, FileReferenceKind, RootProvenance};
use biscuit_terminal::components::list::UnorderedList;
use crate::composition::types::CompositionMode;
use crate::harness::ResolutionDetail;

/// Render the [`StatusBlock`] for a provider/execution/file-reference-family
/// [`CompositionError`], or the generic block for any remaining variant.
pub(super) fn status_block(err: &CompositionError, term: &Terminal) -> StatusBlock {
    match err {
        CompositionError::ComposedBodyEmpty {
            source_path,
            mode,
            provided_overrides,
        } => {
            let file_link = super::render_file_link(source_path);
            let mode_label = match mode {
                CompositionMode::ChainedDocument => "chained (compose)",
                CompositionMode::InlineFrontmatterPrompt => "inline (inline-compose)",
            };
            let mut body = format!(
                "Composition produced an <b>empty prompt body</b> for {file_link}.\n\n\
                 Mode: <i>{mode_label}</i>"
            );
            if provided_overrides.is_empty() {
                body.push_str("\n\nNo `key=value` overrides were provided.");
            } else {
                body.push_str("\n\n<b>Provided overrides:</b>");
                for key in provided_overrides {
                    body.push_str(&format!("\n- <cyan>`{key}`</cyan>"));
                }
            }
            body.push_str(
                "\n\nThe document composed without error, but every block in the body \
                 was stripped by its `when` condition (or the body was empty to begin with). \
                 The provider CLI would otherwise reject this as \"Input must be provided …\" \
                 without naming the real cause.",
            );
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "composed prompt is empty",
                ))
                .body(body)
                .hint(
                    "Check that the variables you passed match the `::block when=…` \
                     conditions in the prompt, or verify there is body content outside any \
                     conditional block.",
                )
        }
        CompositionError::ShellExpansionFailed { error, .. } => {
            // Delegate to the structured shell-expansion block so the
            // linked source path, source excerpt, composed frontmatter
            // block, and captured stderr/stdout all survive the claudine
            // boundary instead of being flattened by the catch-all arm.
            error.status_block(term)
        }
        CompositionError::FileReferenceNoMatch {
            reference,
            resolution,
            suggestions,
        } => {
            // A direct magic (`@`) miss reports the directories the `@`
            // chain searched, not the joined candidate paths: a nonmatching
            // join such as `<root>/prompts/prompts/<x>` is just the rules
            // applied and reads as a resolver bug (R4 of
            // 2026-09-23-local-before-home). Every other reference kind
            // keeps the "Tried:" candidate list, because a bare or explicit
            // relative reference has no search chain to name.
            let body = if resolution.kind() == FileReferenceKind::Magic
                && !resolution.magic_search_roots().is_empty()
            {
                render_magic_no_match_body(reference, resolution, suggestions, term)
            } else {
                render_candidate_no_match_body(reference, resolution, suggestions, term)
            };

            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "Unresolvable file reference",
                ))
                .body(body)
                .hint("Correct the reference and try again.")
        }
        _ => {
            let msg = err.to_string();
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "composition failed"))
                .body(msg)
        }
    }
}

/// The human-readable body for a direct magic (`@`) reference miss.
///
/// Names the reference payload once, then lists the `@` search roots in
/// priority order — every local root before the first home root, exactly as
/// resolution searched them. Only configured roots carry the `(*)` marker; the
/// intrinsic roots (package, package area, local root, home) do not. No joined
/// candidate paths and no provenance labels appear in this branch (R4).
fn render_magic_no_match_body(
    reference: &str,
    resolution: &ResolutionDetail,
    suggestions: &[String],
    term: &Terminal,
) -> String {
    // The reference already parsed to reach a no-match, so the fallback to the
    // authored text is unreachable in practice.
    let parsed = FileReference::new(reference).ok();
    let payload = parsed.as_ref().map_or(reference, FileReference::payload);
    let mut body = Prose::new(format!(
        "<cyan>`{}`</cyan> was not found under any directory an <b>`@` reference</b> searches:",
        Prose::escape_text(payload),
    ))
    .render(term);
    body.push('\n');
    let mut roots = UnorderedList::empty();
    for root in resolution.magic_search_roots() {
        let path = biscuit_file::to_portable_string(root.path());
        let marker = (root.provenance() == RootProvenance::Magic).then_some(" (*)");
        roots.add(Prose::new(format!(
            "<cyan>`{}`</cyan>{}",
            Prose::escape_text(&path),
            marker.unwrap_or_default(),
        )));
    }
    body.push_str(&roots.render(term));
    body.push_str("\n\n");
    body.push_str(
        &Prose::new(
            "<i>(*) searched in addition to the standard `@` roots, for this context</i>",
        )
        .render(term),
    );
    append_suggestions(&mut body, suggestions, term);
    body
}

/// The human-readable body for a non-magic reference miss: the anchor the
/// reference resolved from plus the ordered candidate list with provenance
/// labels. Bare, explicit-relative, absolute, and vault references have no
/// search chain to name, so their joined candidates remain the report.
fn render_candidate_no_match_body(
    reference: &str,
    resolution: &ResolutionDetail,
    suggestions: &[String],
    term: &Terminal,
) -> String {
    let mut body = Prose::new(format!(
        "Cannot resolve <cyan>`{}`</cyan> from launch directory <cyan>`{}`</cyan>.",
        Prose::escape_text(reference),
        Prose::escape_text(&biscuit_file::to_portable_string(resolution.base_dir())),
    ))
    .render(term);

    if !resolution.candidates().is_empty() {
        body.push_str("\n\n");
        body.push_str(&Prose::new("<b>Tried:</b>").render(term));
        body.push('\n');
        let mut candidates = UnorderedList::empty();
        for probed in resolution.candidates() {
            let provenance = match probed.candidate().provenance() {
                RootProvenance::Repository => "repository",
                RootProvenance::Source => "launch directory",
                RootProvenance::PackageRoot => "package",
                RootProvenance::PackageArea => "package area",
                RootProvenance::Home => "home",
                RootProvenance::Magic => "magic",
                RootProvenance::Vault => "vault",
                RootProvenance::Absolute => "absolute",
                RootProvenance::LocalRoot => "local root",
            };
            let path = biscuit_file::to_portable_string(probed.candidate().path());
            candidates.add(Prose::new(format!(
                "<b>{provenance}</b>: <cyan>`{}`</cyan>",
                Prose::escape_text(&path),
            )));
        }
        body.push_str(&candidates.render(term));
    }

    append_suggestions(&mut body, suggestions, term);
    body
}

fn append_suggestions(body: &mut String, suggestions: &[String], term: &Terminal) {
    if suggestions.is_empty() {
        return;
    }
    body.push_str("\n\n");
    body.push_str(&Prose::new("<b>Did you mean:</b>").render(term));
    body.push('\n');
    let mut paths = UnorderedList::empty();
    for path in suggestions {
        paths.add(Prose::new(format!(
            "<cyan>`{}`</cyan>",
            Prose::escape_text(path),
        )));
    }
    body.push_str(&paths.render(term));
}
