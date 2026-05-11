use color_eyre::eyre::{Result, eyre};
use indexmap::IndexMap;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tracing::info_span;

use super::profile::WrapperProfile;

#[derive(Debug, Clone)]
pub(crate) struct NextAttemptPlan {
    pub(crate) next_attempt: u32,
    pub(crate) prompt_append: Option<String>,
    pub(crate) prompt_override: Option<String>,
    pub(crate) set_overlay: Option<IndexMap<String, Value>>,
    pub(crate) redirect_source: Option<PathBuf>,
    pub(crate) resume_session_id: Option<String>,
    pub(crate) clear_prompt_tail: bool,
}

pub(crate) fn normalize_resume_args(
    profile: &dyn WrapperProfile,
    mut args: Vec<String>,
) -> Vec<String> {
    if args.first().is_some_and(|arg| arg == profile.binary()) {
        args.remove(0);
    }
    args
}

pub(crate) fn append_resume_passthrough_args(resume_args: &mut Vec<String>, base_args: &[String]) {
    let mut index = 0;
    while index < base_args.len() {
        match base_args[index].as_str() {
            "--json" | "--verbose" if !resume_args.iter().any(|arg| arg == &base_args[index]) => {
                resume_args.push(base_args[index].clone());
            }
            "--output-format" | "--format" | "--output-last-message" => {
                if index + 1 < base_args.len()
                    && !resume_args.iter().any(|arg| arg == &base_args[index])
                {
                    resume_args.push(base_args[index].clone());
                    resume_args.push(base_args[index + 1].clone());
                }
                index += 1;
            }
            _ => {}
        }
        index += 1;
    }
}

pub(crate) fn apply_next_attempt_plan(
    state: &mut super::harness_orch::HarnessPromptState,
    plan: &NextAttemptPlan,
) {
    if plan.clear_prompt_tail {
        state.prompt_tail.clear();
    }
    if let Some(ref path) = plan.redirect_source {
        state.source_path = path.clone();
        // Update the reporting reference so status output reflects the
        // redirected file rather than the original.
        state.original_ref = path.display().to_string();
    }
    if let Some(ref overlay) = plan.set_overlay {
        super::overlay::merge_frontmatter_overlay(&mut state.overlay, overlay);
    }
    if let Some(ref append) = plan.prompt_append {
        state.prompt_tail.push(append.clone());
    }
    if let Some(ref prompt) = plan.prompt_override {
        state.next_prompt_override = Some(prompt.clone());
    }
    state.next_resume_session_id = plan.resume_session_id.clone();
}

/// Try to resolve a handler for each failure context and build a recovery
/// plan. Returns `Some(plan)` on the first successful resolution, or `None`
/// if no handler produced an actionable plan.
///
/// The handler-engagement banner is emitted exactly once, only after a
/// concrete `NextAttemptPlan` has been produced.
#[allow(clippy::too_many_arguments)]
pub(crate) fn try_resolve_handler(
    contexts: &[claudine::harness::FailureContext],
    plan: &claudine::harness::HarnessPlan,
    attempt: u32,
    default_max_retries: u32,
    profile: &dyn WrapperProfile,
    session_id: Option<&str>,
    source_path: &Path,
    repo_root: Option<&Path>,
    show_checks: bool,
    term: &biscuit_terminal::terminal::Terminal,
) -> Result<Option<NextAttemptPlan>> {
    let _handler_span = info_span!(
        "harness_handler_resolution",
        attempt,
        failure_count = contexts.len(),
        source_path = %source_path.display(),
    )
    .entered();
    for failure_ctx in contexts {
        match claudine::harness::resolve_handler(
            failure_ctx,
            &plan.handlers,
            plan.programmatic_handler.as_ref(),
        ) {
            Ok(Some(action)) => {
                if let Some(next_plan) = build_next_attempt_plan(
                    &action,
                    attempt,
                    default_max_retries,
                    &failure_ctx.message,
                    profile,
                    session_id,
                    source_path,
                    repo_root,
                    failure_ctx,
                    term,
                )? {
                    // Emit the engagement banner exactly once, only when
                    // a concrete recovery plan has been produced.
                    if show_checks {
                        claudine::harness::report::report_handler_engagement(
                            &source_path.display().to_string(),
                            term,
                        );
                    }
                    return Ok(Some(next_plan));
                }
            }
            Ok(None) => {}
            Err(e) => return Err(eyre!("{e}")),
        }
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_next_attempt_plan(
    action: &claudine::harness::HandlerAction,
    current_attempt: u32,
    default_max_retries: u32,
    failure_message: &str,
    profile: &dyn WrapperProfile,
    session_id: Option<&str>,
    source_path: &Path,
    repo_root: Option<&Path>,
    failure_ctx: &claudine::harness::FailureContext,
    term: &biscuit_terminal::terminal::Terminal,
) -> Result<Option<NextAttemptPlan>> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::prelude::Renderable;
    let _decision_span = info_span!(
        "harness_handler_decision",
        current_attempt,
        action = %handler_action_name(action),
        source_path = %source_path.display(),
    )
    .entered();

    let dispatch_handler_feedback = |msg: &Option<String>, say: &Option<String>| {
        if let Some(text) = say.as_deref() {
            claudine::harness::speak_when_able(text);
        }
        if let Some(msg_text) = msg {
            let rendered = Prose::new(msg_text).render(term);
            eprintln!("{rendered}");
        }
    };

    match action {
        claudine::harness::HandlerAction::Retry {
            prompt_suffix,
            set,
            msg,
            say,
            retries,
        } => {
            let max = retries.unwrap_or(default_max_retries);
            if current_attempt >= max {
                crate::log::warn(&format!(
                    "retry ceiling reached ({current_attempt}/{max}): {failure_message}"
                ));
                return Ok(None);
            }
            dispatch_handler_feedback(msg, say);
            tracing::debug!(
                next_attempt = current_attempt + 1,
                "Handler requested retry"
            );
            let prompt_append = prompt_suffix.clone().or_else(|| {
                Some(format!(
                    "The previous attempt failed: {failure_message}. Please correct the issue and try again."
                ))
            });
            Ok(Some(NextAttemptPlan {
                next_attempt: current_attempt + 1,
                prompt_append,
                prompt_override: None,
                set_overlay: set.clone(),
                redirect_source: None,
                resume_session_id: None,
                clear_prompt_tail: false,
            }))
        }
        claudine::harness::HandlerAction::Resume {
            prompt,
            set,
            msg,
            say,
            retries,
        } => {
            let max = retries.unwrap_or(default_max_retries);
            if current_attempt >= max {
                crate::log::warn(&format!(
                    "resume retry ceiling reached ({current_attempt}/{max}): {failure_message}"
                ));
                return Ok(None);
            }
            claudine::harness::validate_resume(
                &profile.provider().to_string(),
                profile.supports_resume(),
                session_id,
            )?;
            dispatch_handler_feedback(msg, say);
            tracing::debug!(
                next_attempt = current_attempt + 1,
                "Handler requested resume"
            );
            Ok(Some(NextAttemptPlan {
                next_attempt: current_attempt + 1,
                prompt_append: None,
                prompt_override: Some(prompt.clone()),
                set_overlay: set.clone(),
                redirect_source: None,
                resume_session_id: session_id.map(|id| id.to_string()),
                clear_prompt_tail: false,
            }))
        }
        claudine::harness::HandlerAction::Redirect {
            file,
            set,
            msg,
            say,
            resume,
        } => {
            if *resume {
                claudine::harness::validate_resume(
                    &profile.provider().to_string(),
                    profile.supports_resume(),
                    session_id,
                )?;
            }
            dispatch_handler_feedback(msg, say);
            let resolve_ctx = claudine::harness::HarnessResolutionContext {
                source_path,
                repo_root,
            };
            let redirect_source = claudine::harness::resolve_harness_path(file, &resolve_ctx)?;
            tracing::debug!(
                next_attempt = current_attempt + 1,
                redirect_source = %redirect_source.display(),
                "Handler requested redirect"
            );
            let resume_session_id = if *resume {
                session_id.map(|id| id.to_string())
            } else {
                None
            };
            Ok(Some(NextAttemptPlan {
                next_attempt: current_attempt + 1,
                prompt_append: None,
                prompt_override: None,
                set_overlay: set.clone(),
                redirect_source: Some(redirect_source),
                resume_session_id,
                clear_prompt_tail: true,
            }))
        }
        claudine::harness::HandlerAction::Deviate {
            command,
            set,
            msg,
            say,
        } => {
            dispatch_handler_feedback(msg, say);
            match claudine::harness::execute_deviate_command(
                command,
                failure_ctx,
                Some(source_path),
            ) {
                Ok(deviate_exit) => {
                    tracing::debug!(
                        next_attempt = current_attempt + 1,
                        exit_code = deviate_exit,
                        "Handler requested deviate command"
                    );
                    if deviate_exit != 0 {
                        crate::log::warn(&format!(
                            "deviate command '{}' exited with code {deviate_exit}",
                            command.raw
                        ));
                    }
                    Ok(Some(NextAttemptPlan {
                        next_attempt: current_attempt + 1,
                        prompt_append: None,
                        prompt_override: None,
                        set_overlay: set.clone(),
                        redirect_source: None,
                        resume_session_id: None,
                        clear_prompt_tail: false,
                    }))
                }
                Err(e) => Err(eyre!("deviate failed: {e}")),
            }
        }
    }
}

fn handler_action_name(action: &claudine::harness::HandlerAction) -> &'static str {
    match action {
        claudine::harness::HandlerAction::Retry { .. } => "retry",
        claudine::harness::HandlerAction::Resume { .. } => "resume",
        claudine::harness::HandlerAction::Redirect { .. } => "redirect",
        claudine::harness::HandlerAction::Deviate { .. } => "deviate",
    }
}
