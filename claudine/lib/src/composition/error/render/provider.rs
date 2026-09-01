//! Terminal rendering for the provider / execution / file-reference error
//! family, plus the catch-all generic block.
//!
//! Covers the empty-composed-body abort, shell-expansion delegation, and every
//! variant without a dedicated block (which render their `Display` message).
//! The dispatcher in [`super`] routes this family here, including its `_` arm.

use super::super::*;
use biscuit_file::RootProvenance;
use biscuit_terminal::components::list::UnorderedList;
use crate::composition::types::CompositionMode;

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
                        RootProvenance::Package => "package",
                        RootProvenance::Home => "home",
                        RootProvenance::Magic => "magic",
                        RootProvenance::Vault => "vault",
                        RootProvenance::Absolute => "absolute",
                    };
                    let path = biscuit_file::to_portable_string(probed.candidate().path());
                    candidates.add(Prose::new(format!(
                        "<b>{provenance}</b>: <cyan>`{}`</cyan>",
                        Prose::escape_text(&path),
                    )));
                }
                body.push_str(&candidates.render(term));
            }

            if !suggestions.is_empty() {
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
