//! Canonical, cross-crate message text for each [`AgentResolutionState`].
//!
//! The spec requires the `--dry-run` metadata table to be a *faithful
//! prediction* of the live (non-dry-run) path: every state's dry-run cell,
//! its live TTY pre-prompt, and its no-TTY abort body must agree. Those three
//! surfaces live in two crates (`claudine` for the no-TTY abort body in
//! `super::error`; `claudine-cli` for the dry-run renderer and the TTY
//! pre-prompt), so the only way to keep them from drifting is a single source
//! of truth. That source is here.
//!
//! These helpers return Prose **markup** (e.g. `<red>…</red>`), not
//! terminal-rendered strings: each caller renders with its own
//! [`biscuit_terminal::terminal::Terminal`] so terminal-capability downgrades
//! (no-color, no-OSC8) are honored at the call site.

use super::types::AgentResolutionState;
use crate::provider::Provider;

/// Render the multi-line breakdown for an agent-resolution state as Prose
/// markup.
///
/// This is the canonical text for the dry-run `Agent` table cell **and** the
/// body of the live no-TTY abort for every non–auto-selecting state except
/// [`AgentResolutionState::SingleInvalid`], whose live message is imperative
/// and needs the document link (see [`invalid_agent_message`]).
///
/// [`AgentResolutionState::Selected`] returns the bare provider name; the
/// remaining states return their full breakdown including any
/// `NOT valid` suggestion list.
pub fn agent_state_breakdown(state: &AgentResolutionState) -> String {
    match state {
        AgentResolutionState::NoAgent => no_agent_breakdown(),
        AgentResolutionState::Selected { provider } => provider.to_string(),
        AgentResolutionState::SingleInvalid { hint } => single_invalid_breakdown(hint),
        AgentResolutionState::SingleNotInstalled { provider } => {
            single_not_installed_breakdown(*provider)
        }
        AgentResolutionState::ListMultipleInstalled {
            installed,
            not_installed,
            invalid,
        } => list_multiple_installed_breakdown(installed, not_installed, invalid),
        AgentResolutionState::ListOneInstalled {
            selected, invalid, ..
        } => list_one_installed_breakdown(*selected, invalid),
        AgentResolutionState::ZeroInstalledList {
            not_installed,
            invalid,
        } => zero_installed_list_breakdown(not_installed, invalid),
    }
}

/// The imperative `Invalid Agent:` message shown before the re-prompt (TTY)
/// and as the abort body (no-TTY) for [`AgentResolutionState::SingleInvalid`].
///
/// `file_link` is pre-built Prose `<a href=…>…</a>` markup for the source
/// document; the Prose layer downgrades it to plain text when the terminal
/// does not support OSC8.
pub fn invalid_agent_message(hint: &str, file_link: &str) -> String {
    format!(
        "<red><b>Invalid Agent:</b></red> the {file_link} references an invalid Agent provider \
         '{hint}'. Choose from the installed agents on this host:"
    )
}

/// The shared `NOT valid` suggestion-list trailer appended whenever a
/// frontmatter `agent` list carried entries that match no provider.
fn invalid_suggestions_trailer(invalid: &[String]) -> String {
    if invalid.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "\nThe following agents were suggested but are <b><red>NOT</red></b> valid Agents:\n",
    );
    for hint in invalid {
        out.push_str(&format!("- {hint}\n"));
    }
    out
}

fn no_agent_breakdown() -> String {
    String::from(
        "- CLI caller didn't specify the Agent\n\
         - the Markdown document didn't suggest any Agents in <inverse>agent</inverse> Frontmatter property\n\
         - the caller will be interactively asked to choose an agent when <i>composing</i> called without the <green>--dry-run</green> flag; otherwise the run aborts with the same message",
    )
}

fn single_invalid_breakdown(hint: &str) -> String {
    format!(
        "<red><b>Invalid Agent</b></red>(<dim>{hint}</dim>) <i>defined in Markdown's \
         <inverse>agent</inverse> Frontmatter! Caller will be prompted to choose a valid Agent \
         when run interactively; otherwise the run aborts with the same message.</i>"
    )
}

fn single_not_installed_breakdown(provider: Provider) -> String {
    format!(
        "<yellow><b>Agent Not Installed:</b></yellow>(<dim>{provider}</dim>) <i>the Markdown \
         document's <inverse>agent</inverse> specifies an Agent platform which is not installed \
         on this host. When run interactively, caller will be asked to choose an Agent; otherwise \
         the run aborts with the same message.</i>"
    )
}

fn list_multiple_installed_breakdown(
    installed: &[Provider],
    not_installed: &[Provider],
    invalid: &[String],
) -> String {
    let mut body = String::from(
        "<green>✓</green> caller will be asked to choose interactively between suggested Agents:\n",
    );
    for provider in installed {
        body.push_str(&format!("- {provider}\n"));
    }
    for provider in not_installed {
        body.push_str(&format!("- <dim>{provider}</dim>\n"));
    }
    body.push_str(&invalid_suggestions_trailer(invalid));
    body
}

fn list_one_installed_breakdown(selected: Provider, invalid: &[String]) -> String {
    let mut body = format!(
        "<green>✓</green> the <b>{selected}</b> will be used without the need for interactive prompting\n\
         the Markdown document suggested multiple agent's but only <b>{selected}</b> is installed on this host"
    );
    if !invalid.is_empty() {
        body.push('\n');
        body.push_str(&invalid_suggestions_trailer(invalid));
    }
    body
}

fn zero_installed_list_breakdown(not_installed: &[Provider], invalid: &[String]) -> String {
    let mut body = String::from(
        "None of the suggested agents are installed/valid; caller will choose from all installed \
         agents when run interactively; otherwise the run aborts with the same message:\n",
    );
    for provider in not_installed {
        body.push_str(&format!("- <dim>{provider}</dim>\n"));
    }
    body.push_str(&invalid_suggestions_trailer(invalid));
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_is_bare_provider_name() {
        let state = AgentResolutionState::Selected {
            provider: Provider::Claude,
        };
        assert_eq!(agent_state_breakdown(&state), Provider::Claude.to_string());
    }

    #[test]
    fn no_agent_lists_the_three_bullets() {
        let body = agent_state_breakdown(&AgentResolutionState::NoAgent);
        assert!(body.contains("didn't specify the Agent"));
        assert!(body.contains("didn't suggest any Agents"));
        assert!(body.contains("aborts with the same message"));
    }

    #[test]
    fn list_multiple_dims_not_installed_and_lists_invalid() {
        let body = agent_state_breakdown(&AgentResolutionState::ListMultipleInstalled {
            installed: vec![Provider::Claude],
            not_installed: vec![Provider::Gemini],
            invalid: vec!["bad".into()],
        });
        assert!(body.contains("choose interactively between suggested Agents"));
        assert!(body.contains(&format!("- {}", Provider::Claude)));
        assert!(body.contains(&format!("<dim>{}</dim>", Provider::Gemini)));
        assert!(body.contains("<b><red>NOT</red></b>"));
        assert!(body.contains("- bad"));
    }

    #[test]
    fn zero_installed_list_lists_dim_and_invalid() {
        let body = agent_state_breakdown(&AgentResolutionState::ZeroInstalledList {
            not_installed: vec![Provider::Gemini],
            invalid: vec!["bad".into()],
        });
        assert!(body.contains("None of the suggested agents are installed/valid"));
        assert!(body.contains(&format!("<dim>{}</dim>", Provider::Gemini)));
        assert!(body.contains("<b><red>NOT</red></b>"));
        assert!(body.contains("- bad"));
    }

    #[test]
    fn invalid_agent_message_is_imperative_with_link() {
        let msg = invalid_agent_message("nope", "<a href=\"file:///x.md\">x.md</a>");
        assert!(msg.contains("<red><b>Invalid Agent:</b></red>"));
        assert!(msg.contains("references an invalid Agent provider 'nope'"));
        assert!(msg.contains("Choose from the installed agents on this host:"));
        assert!(msg.contains("file:///x.md"));
    }
}
