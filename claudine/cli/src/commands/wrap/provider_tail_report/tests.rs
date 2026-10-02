use super::*;

fn strings(tokens: &[&str]) -> Vec<String> {
    tokens.iter().map(|token| token.to_string()).collect()
}

fn implicit(tokens: &[&str]) -> ProviderTail {
    ProviderTail::new(strings(tokens), None)
}

#[test]
fn implicit_notice_names_switches_only() {
    let message = forwarding_message(
        Provider::Codex,
        &implicit(&["-c", "model_reasoning_effort=low"]),
    )
    .unwrap();
    assert_eq!(message, "Forwarding provider arguments to Codex: -c");
}

#[test]
fn explicit_notice_is_opaque() {
    let tail = ProviderTail::new(Vec::new(), Some(strings(&["-c", "value", "--native"])));
    let message = forwarding_message(Provider::Codex, &tail).unwrap();
    assert_eq!(
        message,
        "Forwarding an opaque argument tail to Codex (passed after --)."
    );
}

#[test]
fn mixed_tail_is_one_notice_with_prefix_names_and_opaque_summary() {
    let tail = ProviderTail::new(
        strings(&["-c", "x=y"]),
        Some(strings(&["--native", "z"])),
    );
    let message = forwarding_message(Provider::Codex, &tail).unwrap();
    assert_eq!(
        message,
        "Forwarding provider arguments to Codex: -c, followed by an opaque argument tail \
         (passed after --)."
    );
    assert!(!message.contains("--native"));
}

#[test]
fn authored_empty_suffix_reads_as_implicit() {
    let tail = ProviderTail::new(strings(&["-c", "x"]), Some(Vec::new()));
    let message = forwarding_message(Provider::Codex, &tail).unwrap();
    assert_eq!(message, "Forwarding provider arguments to Codex: -c");
}

#[test]
fn notice_never_claims_recognition() {
    for tail in [
        implicit(&["-c", "x"]),
        implicit(&["operand"]),
        ProviderTail::new(Vec::new(), Some(strings(&["x"]))),
        ProviderTail::new(strings(&["-c"]), Some(strings(&["x"]))),
    ] {
        let message = forwarding_message(Provider::Codex, &tail).unwrap();
        assert!(!message.contains("recogni"), "{message}");
    }
}

#[test]
fn empty_tail_has_no_notice() {
    assert_eq!(forwarding_message(Provider::Codex, &ProviderTail::default()), None);
    let empty_suffix = ProviderTail::new(Vec::new(), Some(Vec::new()));
    assert_eq!(forwarding_message(Provider::Codex, &empty_suffix), None);
}

#[test]
fn display_strips_values_and_secrets() {
    let tail = implicit(&[
        "-c",
        "model_reasoning_effort=low",
        "--api-key",
        "sk-secret",
        "--token=sk-other",
        "--flag=plainvalue",
        "-k=hidden",
    ]);
    assert_eq!(
        switch_names_for_display(&tail),
        strings(&["-c", "--api-key", "--token", "--flag", "-k"])
    );
}

#[test]
fn attached_short_tokens_are_described_not_echoed() {
    let tail = implicit(&["-csecret", "-yq", "-m"]);
    let names = switch_names_for_display(&tail);
    assert_eq!(names, strings(&[ATTACHED_SHORT_TOKEN, ATTACHED_SHORT_TOKEN, "-m"]));
    let message = forwarding_message(Provider::Codex, &tail).unwrap();
    assert!(!message.contains("secret"), "{message}");
    assert!(!message.contains("-cs"), "{message}");
    assert!(!message.contains("-yq"), "{message}");
}

#[test]
fn explicit_tokens_are_never_listed_as_names() {
    let tail = ProviderTail::new(strings(&["--model-x"]), Some(strings(&["--api-key", "sk-1"])));
    assert_eq!(switch_names_for_display(&tail), strings(&["--model-x"]));
}

#[test]
fn bare_operands_only_produce_the_generic_notice() {
    let message = forwarding_message(Provider::Claude, &implicit(&["run", "x=y"])).unwrap();
    assert_eq!(message, "Forwarding provider arguments to Claude.");
}

#[test]
fn switch_names_are_escaped_for_prose_markup() {
    let message = forwarding_message(Provider::Codex, &implicit(&["--<b>x</b>"])).unwrap();
    assert!(!message.contains("<b>"), "{message}");
}

#[test]
fn control_characters_never_reach_the_terminal() {
    let message = forwarding_message(Provider::Codex, &implicit(&["--x\u{1b}[31m"])).unwrap();
    assert!(!message.contains('\u{1b}'), "{message:?}");
}
