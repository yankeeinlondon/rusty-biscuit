use biscuit_terminal::terminal::Terminal;
use claudine::provider::Provider;
use color_eyre::eyre::Result;
use std::path::Path;

use crate::log;

pub(crate) fn strip_prompt_tags_for_provider(provider: Provider, prompt: &str) -> String {
    if matches!(
        provider,
        Provider::Codex | Provider::Gemini | Provider::OpenCode
    ) {
        claudine::mcp::session::lex_tags(prompt).0
    } else {
        prompt.to_string()
    }
}

pub(crate) fn extract_tags_from_prompt(
    prompt: Option<&str>,
    extract_tags: fn(&str) -> (String, Vec<String>),
) -> (Option<String>, Vec<String>) {
    let Some(prompt) = prompt else {
        return (None, Vec::new());
    };
    let (cleaned, tags) = extract_tags(prompt);
    if tags.is_empty() {
        (None, Vec::new())
    } else {
        (Some(cleaned), tags)
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn report_inline_agent_status(
    provider: Provider,
    source_path: &Path,
    final_response: &str,
    agent_exit: i32,
    termination: claudine::harness::ProcessTermination,
    child_cwd: &Path,
    show_checks: bool,
    term: &Terminal,
) {
    let provider_name = crate::output::capitalize_provider(provider);
    let display_path = source_path
        .strip_prefix(child_cwd)
        .unwrap_or(source_path)
        .display();
    let was_interrupted = matches!(
        termination,
        claudine::harness::ProcessTermination::Interrupted
    ) || agent_exit == 130
        || agent_exit == 143;

    if show_checks {
        if was_interrupted {
            log::message(&crate::output::fm_check_fail(
                &format!("{provider_name} agent was interrupted by the user (code {agent_exit})"),
                term,
            ));
        } else if agent_exit == 0 {
            log::message(&crate::output::fm_check_ok(
                &format!("{provider_name} agent completed successfully"),
                term,
            ));
        } else {
            log::message(&crate::output::fm_check_fail(
                &format!("{provider_name} agent exited with error (code {agent_exit})"),
                term,
            ));
        }
    }

    if was_interrupted && show_checks {
        if final_response.trim().is_empty() {
            log::message(&crate::output::fm_check_fail(
                &format!(
                    "<b>User interrupted the agent with CTRL+C; the body of \
                     <blue-500>{display_path}</blue-500> is empty so it appears no work was accomplished.</b>"
                ),
                term,
            ));
        } else {
            log::message(&crate::output::fm_check_fail(
                &format!(
                    "<b>User interrupted the agent with CTRL+C; the body of \
                     <blue-500>{display_path}</blue-500> has been at least partially filled:</b>"
                ),
                term,
            ));
            eprintln!();
            for line in final_response.lines() {
                eprintln!("  {line}");
            }
        }
    }
}

/// Attempt inline closure validation and application.
///
/// Returns `Ok(())` on success (file rewritten), or a list of
/// `ValidationFailure`s that should be routed through the harness handler
/// system for potential retry/resume/redirect recovery.
pub(crate) fn try_inline_closure(
    closure_plan: &claudine::composition::InlineClosurePlan,
    final_response: &str,
    source_path: &Path,
    child_cwd: &Path,
    show_checks: bool,
    term: &Terminal,
) -> Result<(), Vec<claudine::harness::ValidationFailure>> {
    use claudine::harness::{FailurePhase, ValidationEvent, ValidationFailure, ValidationRuleId};

    let display_path = source_path
        .strip_prefix(child_cwd)
        .unwrap_or(source_path)
        .display();

    let replacement_body = match claudine::composition::closure::extract_replacement_body(
        final_response,
    ) {
        Ok(body) => body,
        Err(error) => {
            let message = format!(
                "the referenced file -- {display_path} -- did not receive a valid replacement body: {error}"
            );
            if show_checks {
                log::message(&crate::output::fm_check_fail(&message, term));
            }
            return Err(vec![ValidationFailure {
                rule_id: ValidationRuleId(9000),
                event: ValidationEvent::InlineResponseEmpty,
                phase: FailurePhase::PostCheck,
                subject_key: Some(source_path.display().to_string()),
                message,
            }]);
        }
    };

    // Read post-run frontmatter for comparison (best-effort)
    let post_run_fm = std::fs::read_to_string(source_path).ok().map(|text| {
        let md: darkmatter::markdown::Markdown = text.into();
        md.frontmatter().as_map().clone()
    });

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match claudine::composition::closure::apply_inline_closure(
        closure_plan,
        &replacement_body,
        source_path,
        &today,
        post_run_fm.as_ref(),
    ) {
        Ok(result) => {
            if show_checks {
                use biscuit_terminal::components::status::{Status, StatusState};
                use biscuit_terminal::prelude::Renderable;

                log::message(&crate::output::fm_check_ok(
                    "Applied the captured replacement body to the target document",
                    term,
                ));
                log::message(&crate::output::fm_check_ok(
                    "Preserved original frontmatter and updated <bold>last_updated</bold>",
                    term,
                ));

                for key in &result.new_properties {
                    log::message(&crate::output::fm_check_ok(
                        &format!("Merged new frontmatter property <bold>\"{key}\"</bold>"),
                        term,
                    ));
                }

                for key in &result.reverted_properties {
                    let status = Status::from_prose(format!(
                        "Agent modified frontmatter property <b>\"{key}\"</b> — reverted to original value"
                    ))
                    .state(StatusState::Warning);
                    log::message(&status.render(term));
                }
            }
            Ok(())
        }
        Err(error) => {
            let is_unchanged = error.to_string().contains("unchanged");
            let event = if is_unchanged {
                ValidationEvent::InlineBodyUnchanged
            } else {
                ValidationEvent::InlineResponseEmpty
            };
            let message = format!("failed to rewrite {display_path}: {error}");
            if show_checks {
                log::message(&crate::output::fm_check_fail(&message, term));
            }
            Err(vec![ValidationFailure {
                rule_id: ValidationRuleId(9001),
                event,
                phase: FailurePhase::PostCheck,
                subject_key: Some(source_path.display().to_string()),
                message,
            }])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::mcp::session::lex_tags;

    #[test]
    fn extracts_tags_from_codex_prompt_position() {
        let prompt = "fix #calendar bugs";

        let (cleaned, tags) = extract_tags_from_prompt(Some(prompt), lex_tags);

        assert_eq!(tags, vec!["calendar"]);
        assert_eq!(cleaned.as_deref(), Some("fix bugs"));
    }

    #[test]
    fn extracts_tags_from_gemini_prompt_flag() {
        let prompt = "debug #slack auth";

        let (cleaned, tags) = extract_tags_from_prompt(Some(prompt), lex_tags);

        assert_eq!(tags, vec!["slack"]);
        assert_eq!(cleaned.as_deref(), Some("debug auth"));
    }
}
