use claudine::provider::Provider;
use color_eyre::eyre::Result;
use std::collections::HashMap;
use std::ffi::OsString;
use std::time::Duration;

use super::{AttemptLaunch, MaterializedHarnessPrompt};

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_harness_launch(
    provider: Provider,
    profile: &dyn super::super::profile::WrapperProfile,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    // The live session to resume, or `None` for a fresh session. Read from the
    // active-document state's provider-attempt slice by the caller — there is no
    // separate resume directive on the prompt state to consume.
    resume_session: Option<&str>,
    materialized: &MaterializedHarnessPrompt,
    effective_non_interactive: bool,
    cli_timeout: Option<String>,
    plan_timeout: Option<std::time::Duration>,
    cli_step_timeout: Option<String>,
    plan_step_timeout: Option<std::time::Duration>,
    cli_stall_timeout: Option<String>,
) -> Result<AttemptLaunch> {
    let mut args = if let Some(session_id) = resume_session {
        let mut args = super::super::resume::normalize_resume_args(
            profile,
            profile.build_resume_args(session_id)?,
        );
        super::super::resume::append_resume_passthrough_args(&mut args, base_args);
        args
    } else {
        base_args.to_vec()
    };

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
        cli_step_timeout.clone(),
        plan_step_timeout,
    )
    .with_provider(provider);
    let step_timeout_user_configured =
        step_timeout_user_configured(cli_step_timeout.as_deref(), plan_step_timeout);

    let stall_timeout =
        super::super::composition::resolve_stall_timeout(cli_stall_timeout.clone());
    let stall_timeout_user_configured =
        stall_timeout_user_configured(cli_stall_timeout.as_deref());

    Ok(AttemptLaunch {
        args,
        env,
        stdin_seed,
        wire_prompt,
        timeout_config,
        step_timeout_user_configured,
        stall_timeout,
        stall_timeout_user_configured,
    })
}

/// Mirror of [`step_timeout_user_configured`] for the stalled-generation
/// backstop: `true` when the budget came from the CLI flag or a valid non-zero
/// `CLAUDINE_OPENCODE_STALL_TIMEOUT` env value rather than the built-in `10m`
/// default.
fn stall_timeout_user_configured(cli: Option<&str>) -> bool {
    if cli.is_some() {
        return true;
    }
    let Ok(raw) = std::env::var("CLAUDINE_OPENCODE_STALL_TIMEOUT") else {
        return false;
    };
    let trimmed = raw.trim();
    !trimmed.is_empty()
        && !is_zero_duration_literal(trimmed)
        && claudine::harness::parse_timeout(trimmed, std::path::Path::new("<env>")).is_ok()
}

fn step_timeout_user_configured(cli: Option<&str>, frontmatter: Option<Duration>) -> bool {
    if cli.is_some() || frontmatter.is_some() {
        return true;
    }
    let Ok(raw) = std::env::var("CLAUDINE_STEP_TIMEOUT") else {
        return false;
    };
    let trimmed = raw.trim();
    !trimmed.is_empty()
        && !is_zero_duration_literal(trimmed)
        && claudine::harness::parse_timeout(trimmed, std::path::Path::new("<env>")).is_ok()
}

/// True only when the numeric component is exactly `0.0` (`0`, `0s`, `0.0s`,
/// `0m`). A fractional value like `0.5s` is non-zero, so the `.` is included in
/// the numeric prefix and parsed as `f64`.
fn is_zero_duration_literal(value: &str) -> bool {
    let trimmed = value.trim();
    let num_end = trimmed
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(trimmed.len());
    if num_end == 0 {
        return false;
    }
    let (num, _rest) = trimmed.split_at(num_end);
    num.parse::<f64>().is_ok_and(|n| n == 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }

        fn clear(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(value) = &self.original {
                    std::env::set_var(self.key, value);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn built_in_step_timeout_is_not_user_configured() {
        let _guard = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

        assert!(!step_timeout_user_configured(None, None));
    }

    #[test]
    #[serial_test::serial]
    fn cli_frontmatter_and_valid_env_step_timeout_are_user_configured() {
        let _guard = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "5m");

        assert!(step_timeout_user_configured(None, None));
        assert!(step_timeout_user_configured(Some("30s"), None));
        assert!(step_timeout_user_configured(
            None,
            Some(Duration::from_secs(45))
        ));
    }

    #[test]
    #[serial_test::serial]
    fn zero_or_invalid_env_step_timeout_is_not_user_configured() {
        {
            let _guard = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "0s");
            assert!(!step_timeout_user_configured(None, None));
        }
        {
            let _guard = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "not a duration");
            assert!(!step_timeout_user_configured(None, None));
        }
    }

    #[test]
    #[serial_test::serial]
    fn fractional_env_stall_timeout_is_user_configured() {
        let _guard = EnvGuard::set("CLAUDINE_OPENCODE_STALL_TIMEOUT", "0.5s");
        assert!(stall_timeout_user_configured(None));
    }

    #[test]
    #[serial_test::serial]
    fn zero_env_stall_timeout_is_not_user_configured() {
        let _guard = EnvGuard::set("CLAUDINE_OPENCODE_STALL_TIMEOUT", "0s");
        assert!(!stall_timeout_user_configured(None));
    }
}
