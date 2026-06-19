use claudine::provider::Provider;
use color_eyre::eyre::Result;
use std::collections::HashMap;
use std::ffi::OsString;

use super::{AttemptLaunch, HarnessPromptState, MaterializedHarnessPrompt};

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_harness_launch(
    provider: Provider,
    profile: &dyn super::super::profile::WrapperProfile,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    state: &mut HarnessPromptState,
    materialized: &MaterializedHarnessPrompt,
    effective_non_interactive: bool,
    cli_timeout: Option<String>,
    plan_timeout: Option<std::time::Duration>,
    cli_step_timeout: Option<String>,
    plan_step_timeout: Option<std::time::Duration>,
) -> Result<AttemptLaunch> {
    let mut args = if let Some(session_id) = state.next_resume_session_id.take() {
        let mut args = super::super::resume::normalize_resume_args(
            profile,
            profile.build_resume_args(&session_id)?,
        );
        super::super::resume::append_resume_passthrough_args(&mut args, base_args);
        args
    } else {
        base_args.to_vec()
    };
    state.next_prompt_override = None;

    let prompt = super::super::inline::strip_prompt_tags_for_provider(provider, &materialized.prompt);
    let prompt_source = super::super::profile::PromptSource::Inline(prompt.clone());
    let delivery = profile.prompt_delivery(&args, &prompt, effective_non_interactive)?;
    let wire_prompt = delivery.as_wire_rpc().map(str::to_string);
    let stdin_seed = delivery.apply_to(&mut args);
    super::super::profile::require_prompt_present(
        profile.binary(),
        effective_non_interactive,
        &prompt_source,
    )?;

    let mut env = base_env.clone();
    for (key, value) in &materialized.env_overrides {
        env.insert(key.clone().into(), value.clone().into());
    }

    let timeout_config = super::super::composition::resolve_timeouts(
        cli_timeout,
        plan_timeout,
        cli_step_timeout,
        plan_step_timeout,
    )
    .with_provider(provider);

    Ok(AttemptLaunch {
        args,
        env,
        stdin_seed,
        wire_prompt,
        timeout_config,
    })
}
