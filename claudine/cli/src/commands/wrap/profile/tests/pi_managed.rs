//! Pi's structured launch: the research-selected RPC interface, its argv, and
//! its pre-submission JSON fallback.

use super::super::*;

fn pi() -> &'static dyn WrapperProfile {
    profile_for_provider(Provider::Pi).unwrap()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// The non-interactive pipeline's argv for `passthrough`: entrypoint, then
/// the structured-stream flags.
fn structured(passthrough: &[&str]) -> Vec<String> {
    let mut args = strings(passthrough);
    pi().apply_entrypoint(&mut args, true);
    pi().apply_structured_stream(&mut args);
    args
}

#[test]
fn the_research_selects_rpc_with_a_json_fallback() {
    let selection = claudine::steering::execution_selection(Provider::Pi).expect("Pi research selects an interface");
    assert_eq!((selection.preferred, selection.fallback), ("rpc", Some("json")));
}

#[test]
fn a_structured_run_launches_rpc_and_keeps_resources_and_trust() {
    let args = structured(&["--extension", "./probe.ts", "--skill", "./s"]);
    assert_eq!(args, strings(&["--extension", "./probe.ts", "--skill", "./s", "--no-approve", "--mode", "rpc"]));
    for disabled in ["--no-extensions", "--no-skills", "--no-prompt-templates", "--no-context-files", "-p", "--print"] {
        assert!(!args.iter().any(|arg| arg == disabled), "{disabled} must not be injected: {args:?}");
    }
}

#[test]
fn an_rpc_launch_gets_a_control_session_with_the_equivalent_json_fallback() {
    let args = structured(&["--model", "fixture"]);
    let control = pi().stdio_control(&args).expect("RPC argv is driven over retained stdin");
    assert_eq!(control.steering_profile(), Some("retained-rpc"));
    assert!(control.steering_executor().is_some());
    let fallback = control.fallback().expect("the research names a JSON fallback");
    assert_eq!(fallback.args, strings(&["--model", "fixture", "--no-approve", "-p", "--mode", "json"]));
    assert!(fallback.warning.contains("cannot be steered"), "{}", fallback.warning);

    // The JSON launch itself, and interactive argv, take no control session.
    assert!(pi().stdio_control(&fallback.args).is_none());
    assert!(pi().stdio_control(&strings(&["--", "hello"])).is_none());
}

#[test]
fn the_prompt_is_the_seed_the_control_session_submits() {
    let mut args = structured(&[]);
    let seed = pi().prompt_delivery(&args, "- review the plan", true).unwrap().apply_to(&mut args);
    assert_eq!(seed.as_deref(), Some("- review the plan"));
    assert_eq!(args, strings(&["--no-approve", "--mode", "rpc"]), "nothing is added to argv");
}
