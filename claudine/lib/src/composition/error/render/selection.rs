//! Terminal rendering for the selection / target error family.
//!
//! Covers agent (provider) resolution failures and the interactive
//! file-autocomplete outcomes. The dispatcher in [`super`] routes this family
//! here.

use super::super::*;
use super::escape_prose_path;
use crate::composition::types::AgentResolutionState;

/// Render the [`StatusBlock`] for a selection/target-family
/// [`CompositionError`].
pub(super) fn status_block(err: &CompositionError) -> StatusBlock {
    match err {
        CompositionError::AgentResolutionFailed {
            source_path,
            state,
            installed,
        } => {
            let file_link = super::render_file_link(source_path);
            let body = render_agent_resolution_failed_body(state, installed, &file_link);
            StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "agent resolution failed"))
                .body(body)
                .hint(
                    "Specify an installed provider with --claude, --codex, etc., run in an \
                     interactive terminal, or correct the `agent` frontmatter property."
                )
        }
        CompositionError::AutocompleteNoMatches { query } => StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("CompositionError", "no autocomplete matches"))
            .body(format!(
                "No files matched autocomplete query <cyan>`{}`</cyan>.",
                escape_prose_path(query)
            ))
            .hint("Check the query token or run without a query to see all candidates."),
        CompositionError::AutocompleteOverCap { query, cap } => StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("CompositionError", "too many matches"))
            .body(format!(
                "More than <cyan>{cap}</cyan> files matched autocomplete query \
                 <cyan>`{}`</cyan>.",
                escape_prose_path(query)
            ))
            .hint("Type more characters to narrow the query."),
        CompositionError::AutocompleteNotInteractive => StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new(
                "CompositionError",
                "autocomplete not available",
            ))
            .body("Autocomplete requires an interactive terminal.".to_string())
            .hint("Run in a terminal, or supply an explicit file path or reference."),
        CompositionError::AutocompleteCancelled { query } => StatusBlock::new(StatusState::Warning)
            .error_header(ErrorHeader::new(
                "CompositionError",
                "autocomplete cancelled",
            ))
            .body(format!(
                "Autocomplete for query <cyan>`{}`</cyan> was cancelled.",
                escape_prose_path(query)
            ))
            .hint("Supply an explicit file path or reference, or run the command again."),
        // The dispatcher only routes selection-family variants here.
        _ => unreachable!("non-selection CompositionError routed to selection renderer"),
    }
}

/// Render the human-facing body for [`CompositionError::AgentResolutionFailed`].
///
/// The no-TTY abort body must be the **same** styled message the TTY path
/// would show for the state, so it shares one source of truth with the
/// dry-run table cell and the live TTY pre-prompt — see
/// [`agent_message`](crate::composition::agent_message). The only state with a
/// distinct (imperative) live message is
/// [`AgentResolutionState::SingleInvalid`], which is built here from
/// [`invalid_agent_message`](crate::composition::agent_message::invalid_agent_message)
/// plus the installed-agent list the TTY picker would offer.
pub(crate) fn render_agent_resolution_failed_body(
    state: &AgentResolutionState,
    installed: &[Provider],
    file_link: &str,
) -> String {
    use crate::composition::agent_message::{agent_state_breakdown, invalid_agent_message};

    match state {
        AgentResolutionState::SingleInvalid { hint } => {
            let mut body = invalid_agent_message(hint, file_link);
            if installed.is_empty() {
                body.push_str("\n\n<i><dim>(no agents are installed)</dim></i>");
            } else {
                for provider in installed {
                    body.push_str(&format!("\n- {provider}"));
                }
            }
            body
        }
        // Auto-selecting states never abort; keep a diagnostic if they
        // somehow reach this path.
        AgentResolutionState::ListOneInstalled { .. } => format!(
            "Agent resolution unexpectedly aborted for {file_link} despite an auto-selectable suggestion."
        ),
        AgentResolutionState::Selected { provider } => format!(
            "Agent resolution unexpectedly aborted for {file_link} when <b>{provider}</b> was already selected."
        ),
        // Every other prompting state aborts with the same breakdown the
        // dry-run table predicts and the TTY path shows.
        other => agent_state_breakdown(other),
    }
}
