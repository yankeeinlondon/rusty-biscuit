mod attempt;
mod launch;
mod loop_control;
mod prompt;
mod session_key;
mod shell_options;
mod types;

pub(crate) use attempt::execute_harness_attempt;
pub(crate) use launch::build_harness_launch;
pub(crate) use session_key::session_compat_key;
pub(crate) use loop_control::{LaunchRebuildIntent, run_harness_loop};
pub(crate) use prompt::{
    find_wrapper_harness_source, materialize_harness_prompt, materialize_passthrough_harness_seed,
    materialized_harness_prompt_from_prepared, preflight_proxy_target,
};
pub(crate) use shell_options::{
    CachedHarnessLoopContext, apply_composition_shell_overrides, build_harness_shell_options,
    build_harness_shell_options_with_cache, harness_policy_root,
};
pub(crate) use types::{
    AttemptLaunch, HarnessPromptMode, HarnessPromptState, MaterializedHarnessPrompt,
    harness_prompt_mode_label,
};
