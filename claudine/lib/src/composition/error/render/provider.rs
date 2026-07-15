//! Terminal rendering for the provider / execution / file-reference error
//! family, plus the catch-all generic block.
//!
//! Covers the empty-composed-body abort, shell-expansion delegation, and every
//! variant without a dedicated block (which render their `Display` message).
//! The dispatcher in [`super`] routes this family here, including its `_` arm.

use super::super::*;
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
        _ => {
            let msg = err.to_string();
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "composition failed"))
                .body(msg)
        }
    }
}
