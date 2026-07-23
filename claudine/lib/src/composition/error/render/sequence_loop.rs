//! Terminal rendering for the sequence / loop error family.
//!
//! Covers per-iteration loop failures, loop rate-limit halts, per-step
//! sequence validation failures, the sequence-interactive rejection, and the
//! inline-compose-on-a-sequence mismatch. The dispatcher in [`super`] routes
//! this family here.

use super::super::*;
use super::{escape_prose_path, render_file_link};

/// Render the [`StatusBlock`] for a sequence/loop-family [`CompositionError`].
pub(super) fn status_block(err: &CompositionError) -> StatusBlock {
    match err {
        CompositionError::LoopIterationFailed {
            iteration,
            exit_code,
            reason,
            exit_reason,
            ..
        } => {
            // Surface the actionable cause (`step_timeout`,
            // `wall-clock timeout`, signal, …) in the header instead of
            // the generic `composition failed` line. The cause comes
            // from the iteration's session_end JSONL row's
            // `extra.exit_reason` — not from `LoopInvalid` (which is
            // reserved for frontmatter parse errors).
            let title = exit_reason
                .clone()
                .unwrap_or_else(|| "iteration failed".to_string());
            let body =
                format!("Iteration {iteration} exited with code {exit_code}.\n\n{reason}");
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", &title))
                .body(body)
        }
        CompositionError::LoopRateLimited { .. } => StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("CompositionError", "rate limited"))
            .body(err.to_string())
            .hint(
                "Re-run after the listed reset time, or use \
                     `--on-rate-limit pause` to wait automatically.",
            ),
        CompositionError::SequenceMissingProperties { failures, .. } => {
            render_sequence_missing_properties_block(failures)
        }
        CompositionError::SequenceInteractiveRejected(source_path) => {
            let file_link = render_file_link(source_path);
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "CompositionError",
                    "interactive rejected for sequence",
                ))
                .body(format!(
                    "The document {file_link} sets <cyan>`interactive: true`</cyan> in its \
                     frontmatter, but a <cyan>`sequence`</cyan> is serial automation and does \
                     not support interactive sessions.\n\n\
                     Use <cyan>`claudine compose`</cyan> or <cyan>`claudine inline-compose`</cyan> \
                     for dialog-shaped prompts. To run an individual sequence step \
                     interactively, use the <cyan>`--interactive`</cyan> CLI flag — this remains \
                     the only explicit override."
                ))
        }
        CompositionError::InlineComposeSequenceMismatch { source_path } => {
            render_inline_sequence_mismatch_block(source_path)
        }
        // The dispatcher only routes sequence/loop-family variants here.
        _ => unreachable!("non-sequence/loop CompositionError routed to sequence_loop renderer"),
    }
}

/// Render the diagnostic block for
/// [`CompositionError::InlineComposeSequenceMismatch`].
///
/// Builds the blank-line-separated paragraph sequence: the opening statement;
/// the explanation (document link, both property names, what `sequence` does,
/// and the `claudine sequence` directive); and the upcoming-`sections` note.
/// The authored frontmatter YAML block is appended after this diagnostic by the
/// CLI error walker when the error is enriched with a [`FrontmatterExcerpt`].
///
/// [`FrontmatterExcerpt`]: crate::composition::FrontmatterExcerpt
fn render_inline_sequence_mismatch_block(source_path: &std::path::Path) -> StatusBlock {
    let file_link = render_file_link(source_path);

    let opening =
        Prose::new("You tried to run an inline-compose operation on a document configured as a sequence.");

    let explanation = Prose::new(format!(
        "The document {file_link} defines both <cyan>`prompt`</cyan> and <cyan>`sequence`</cyan>. \
         A <cyan>`sequence`</cyan> makes each state invoke an inline-compose operation using \
         <cyan>`prompt`</cyan>, so run it with <cyan>`claudine sequence`</cyan> instead."
    ));

    let sections_note = Prose::new(
        "Note: the upcoming <cyan>`sections`</cyan> feature may be a better fit when each \
         operation should update a particular section of the document. It may not suit every \
         sequence workflow and is not available yet.",
    );

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new(
            "CompositionError",
            "inline-compose on a sequence",
        ))
        .body(vec![opening, explanation, sections_note])
        .hint("Run the document with `claudine sequence <file>`.")
}

fn render_sequence_missing_properties_block(
    failures: &[SequenceMissingPropertiesStep],
) -> StatusBlock {
    let plural = if failures.len() == 1 { "step" } else { "steps" };
    let mut body = format!(
        "Missing required schema properties in {} {plural} of the sequence.",
        failures.len()
    );

    for failure in failures {
        let file_link = render_file_link(&failure.source_path);
        body.push_str(&format!(
            "\n\n<b>Step {}: <cyan>{}</cyan></b> ({file_link})",
            failure.step,
            escape_prose_path(&failure.step_name),
        ));
        if let Some(desc) = failure
            .frontmatter_description
            .as_deref()
            .filter(|d| !d.trim().is_empty())
        {
            body.push_str(&format!("\n  <i><dim>{}</dim></i>", escape_prose_path(desc)));
        }
        if !failure.missing.is_empty() {
            for prop in &failure.missing {
                let type_label = prop
                    .type_label
                    .as_deref()
                    .filter(|t| !t.is_empty())
                    .unwrap_or("(unknown type)");
                let mut line = format!("\n  - <cyan>`{}`</cyan>: {}", prop.name, type_label);
                if let Some(desc) = prop.description.as_deref().filter(|d| !d.trim().is_empty()) {
                    line.push_str(&format!(" <i><dim>— {}</dim></i>", escape_prose_path(desc)));
                }
                body.push_str(&line);
            }
        } else if !failure.pointer_paths.is_empty() {
            for pointer in &failure.pointer_paths {
                body.push_str(&format!("\n  - <cyan>`{pointer}`</cyan>"));
            }
        }
    }

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new(
            "CompositionError",
            "sequence missing properties",
        ))
        .body(body)
        .hint(
            "Fix the missing values in the sequence document (or pass them via --set) and re-run; \
             every step is validated before the first provider session starts.",
        )
}
