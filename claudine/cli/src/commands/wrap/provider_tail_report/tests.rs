use super::*;

fn strings(tokens: &[&str]) -> Vec<String> {
    tokens.iter().map(|token| token.to_string()).collect()
}

fn implicit(tokens: &[&str]) -> ProviderTail {
    ProviderTail::new(strings(tokens), None)
}

/// The non-interactive launch context the wrappers use.
fn context(provider: Provider) -> SwitchContext {
    let profile = crate::commands::wrap::profile::profile_for_provider(provider).unwrap();
    SwitchContext::for_launch(profile, true)
}

#[test]
fn implicit_notice_names_switches_only() {
    let message = forwarding_message(&context(Provider::Codex),
        &implicit(&["-c", "model_reasoning_effort=low"]),
    )
    .unwrap();
    assert_eq!(message, "Forwarding provider arguments to Codex: -c");
}

#[test]
fn explicit_notice_is_opaque() {
    let tail = ProviderTail::new(Vec::new(), Some(strings(&["-c", "value", "--native"])));
    let message = forwarding_message(&context(Provider::Codex), &tail).unwrap();
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
    let message = forwarding_message(&context(Provider::Codex), &tail).unwrap();
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
    let message = forwarding_message(&context(Provider::Codex), &tail).unwrap();
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
        let message = forwarding_message(&context(Provider::Codex), &tail).unwrap();
        assert!(!message.contains("recogni"), "{message}");
    }
}

#[test]
fn empty_tail_has_no_notice() {
    assert_eq!(forwarding_message(&context(Provider::Codex), &ProviderTail::default()), None);
    let empty_suffix = ProviderTail::new(Vec::new(), Some(Vec::new()));
    assert_eq!(forwarding_message(&context(Provider::Codex), &empty_suffix), None);
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
        switch_names_for_display(&tail, None),
        strings(&["-c", "--api-key", "--token", "--flag", "-k"])
    );
}

#[test]
fn attached_short_tokens_are_described_not_echoed() {
    let tail = implicit(&["-csecret", "-yq", "-m"]);
    let names = switch_names_for_display(&tail, None);
    assert_eq!(names, strings(&[ATTACHED_SHORT_TOKEN, ATTACHED_SHORT_TOKEN, "-m"]));
    let message = forwarding_message(&context(Provider::Codex), &tail).unwrap();
    assert!(!message.contains("secret"), "{message}");
    assert!(!message.contains("-cs"), "{message}");
    assert!(!message.contains("-yq"), "{message}");
}

#[test]
fn explicit_tokens_are_never_listed_as_names() {
    let tail = ProviderTail::new(strings(&["--model-x"]), Some(strings(&["--api-key", "sk-1"])));
    assert_eq!(switch_names_for_display(&tail, None), strings(&["--model-x"]));
}

#[test]
fn bare_operands_only_produce_the_generic_notice() {
    let message = forwarding_message(&context(Provider::Claude), &implicit(&["run", "x=y"])).unwrap();
    assert_eq!(message, "Forwarding provider arguments to Claude.");
}

#[test]
fn switch_names_are_escaped_for_prose_markup() {
    let message = forwarding_message(&context(Provider::Codex), &implicit(&["--<b>x</b>"])).unwrap();
    assert!(!message.contains("<b>"), "{message}");
}

#[test]
fn control_characters_never_reach_the_terminal() {
    let message = forwarding_message(&context(Provider::Codex), &implicit(&["--x\u{1b}[31m"])).unwrap();
    assert!(!message.contains('\u{1b}'), "{message:?}");
}

/// The context is the entrypoint the profile really launches: Codex runs
/// `exec` without a terminal and its root command with one, and Claude's
/// non-interactive mode is a flag, not a command path.
#[test]
fn the_switch_context_is_the_launch_entrypoint() {
    let codex = crate::commands::wrap::profile::profile_for_provider(Provider::Codex).unwrap();
    assert_eq!(SwitchContext::for_launch(codex, true).command_path, strings(&["exec"]));
    assert!(SwitchContext::for_launch(codex, false).command_path.is_empty());
    assert!(context(Provider::Claude).command_path.is_empty());
    assert_eq!(context(Provider::Claude).provider, Provider::Claude);
}

/// The headline switch, read from the compiled Codex catalog: `-c` is
/// Codex's `--config`, a string that may be attached to its short spelling.
#[test]
fn codex_dash_c_is_explained_from_the_compiled_catalog() {
    let codex = context(Provider::Codex);
    let SwitchLookup::Known(config) = lookup_switch(Provider::Codex, &["exec"], "-c") else {
        panic!("the compiled Codex catalog does not establish -c at exec");
    };
    assert_eq!(config.flag, "--config");
    assert_eq!(config.value, claudine::provider::SwitchValue::String { optional: false });

    let tail = implicit(&["-c", "model_reasoning_effort=low", "-c", "x=y", "--config=z=1"]);
    let explanations = switch_explanations(&codex, &tail);
    assert_eq!(explanations.len(), 2, "one per distinct spelling: {explanations:?}");
    assert!(
        explanations[0].starts_with("-c is Codex's --config switch (")
            && explanations[0].ends_with("); forwarding to Codex."),
        "{}",
        explanations[0]
    );
    assert!(
        explanations[1].starts_with("--config is one of Codex's switches (")
            && explanations[1].ends_with("); forwarding to Codex."),
        "{}",
        explanations[1]
    );
    for explanation in &explanations {
        assert!(!explanation.contains("model_reasoning_effort") && !explanation.contains("z=1"));
    }
}

/// A researched short-attached form splits off the value (`-csecret` names
/// `-c`); without a context nothing is split.
#[test]
fn a_researched_attached_value_is_split_off_its_switch() {
    let codex = context(Provider::Codex);
    let tail = implicit(&["-csk-secret-value", "-cplain=1"]);
    assert_eq!(switch_names_for_display(&tail, Some(&codex)), strings(&["-c", "-c"]));
    assert_eq!(
        switch_names_for_display(&tail, None),
        strings(&[ATTACHED_SHORT_TOKEN, ATTACHED_SHORT_TOKEN])
    );
    let message = forwarding_message(&codex, &tail).unwrap();
    assert_eq!(message, "Forwarding provider arguments to Codex: -c, -c");
    assert_eq!(switch_explanations(&codex, &tail).len(), 1);
}

/// A switch the catalog does not establish at this command path is
/// forwarded anyway, and the message never says the provider rejects it.
#[test]
fn an_unrecognized_switch_is_forwarded_without_a_rejection_claim() {
    let codex = context(Provider::Codex);
    let explanations = switch_explanations(&codex, &implicit(&["--frobnicate", "x"]));
    assert_eq!(
        explanations,
        strings(&["--frobnicate: Claudine's compiled Codex switch catalog has no established \
                   type for it at its `exec` command; Claudine forwards it anyway."])
    );
    assert!(!explanations[0].contains("reject") && !explanations[0].contains("recogni"));
    let root = SwitchContext {
        provider: Provider::Codex,
        command_path: Vec::new(),
    };
    assert!(switch_explanations(&root, &implicit(&["--frobnicate"]))[0].contains("at its root command"));
}

/// The explicit tail stays opaque: no switch inside it is explained.
#[test]
fn an_explicit_tail_is_never_explained() {
    let codex = context(Provider::Codex);
    let tail = ProviderTail::new(Vec::new(), Some(strings(&["-c", "x=y"])));
    assert!(switch_explanations(&codex, &tail).is_empty());
    let mixed = ProviderTail::new(strings(&["--frobnicate"]), Some(strings(&["-c", "x=y"])));
    let explanations = switch_explanations(&codex, &mixed);
    assert_eq!(explanations.len(), 1);
    assert!(explanations[0].starts_with("--frobnicate:"));
}

/// Research text is shown through the same guard as tokens.
#[test]
fn a_description_reads_inside_parentheses() {
    assert_eq!(first_sentence("Override a configuration value. Repeatable."), "override a configuration value");
    assert_eq!(first_sentence("JSON output only."), "JSON output only");
    assert_eq!(
        first_sentence("Override one value for this run; parsed as TOML."),
        "override one value for this run"
    );
    assert_eq!(first_sentence("x"), "x");
    assert_eq!(display_safe("<b>x</b>\u{1b}[31m"), Prose::escape_text("<b>x</b>[31m"));
}

fn unrecognized(switch: &str, provider: Provider, place: &str) -> String {
    format!(
        "{switch}: Claudine's compiled {provider} switch catalog has no established type for it \
         at {place}; Claudine forwards it anyway."
    )
}

/// OpenCode's global completion-protocol switch has a compiled record that
/// declares its type unknown. A record establishes a type only through its
/// value, so the switch is explained like one with no record at all.
#[test]
fn a_record_typed_unknown_is_explained_as_unrecognized() {
    let opencode = context(Provider::OpenCode);
    assert_eq!(opencode.command_path, strings(&["run"]));
    let SwitchLookup::Known(record) = lookup_switch(Provider::OpenCode, &["run"], "--get-yargs-completions") else {
        panic!("the compiled OpenCode catalog no longer records --get-yargs-completions globally");
    };
    assert_eq!(record.value, SwitchValue::Unknown);

    let explanations = switch_explanations(&opencode, &implicit(&["--get-yargs-completions", "foo"]));
    assert_eq!(
        explanations,
        vec![unrecognized("--get-yargs-completions", Provider::OpenCode, "its `run` command")]
    );
    assert!(!explanations[0].contains("reject") && !explanations[0].contains("is one of"));
}

/// The absent-record control reads exactly as the unknown-type record does.
#[test]
fn an_absent_record_reads_like_an_unknown_type() {
    for provider in [Provider::OpenCode, Provider::Kilo] {
        let explanations = switch_explanations(&context(provider), &implicit(&["--new-unresearched-switch", "foo"]));
        assert_eq!(
            explanations,
            vec![unrecognized("--new-unresearched-switch", provider, "its `run` command")]
        );
    }
}

/// `--models` is recorded (typed unknown) only at `stats`, so at `run` it has
/// no record; at `stats` its record still takes the unknown-type branch.
#[test]
fn models_is_unrecognized_at_run_and_at_stats() {
    for provider in [Provider::OpenCode, Provider::Kilo] {
        assert_eq!(lookup_switch(provider, &["run"], "--models"), SwitchLookup::NotInCatalog);
        let run = switch_explanations(&context(provider), &implicit(&["--models", "foo"]));
        assert_eq!(run, vec![unrecognized("--models", provider, "its `run` command")]);

        let SwitchLookup::Known(record) = lookup_switch(provider, &["stats"], "--models") else {
            panic!("the compiled {provider} catalog no longer records --models at stats");
        };
        assert_eq!(record.value, SwitchValue::Unknown);
        let stats = SwitchContext {
            provider,
            command_path: strings(&["stats"]),
        };
        let explanations = switch_explanations(&stats, &implicit(&["--models", "foo"]));
        assert_eq!(explanations, vec![unrecognized("--models", provider, "its `stats` command")]);
    }
}

/// A researched switch named by its canonical spelling reads without an
/// article, so a provider name starting with a vowel stays grammatical.
#[test]
fn a_known_canonical_switch_reads_without_an_article() {
    let opencode = context(Provider::OpenCode);
    let SwitchLookup::Known(model) = lookup_switch(Provider::OpenCode, &["run"], "--model") else {
        panic!("the compiled OpenCode catalog does not establish --model at run");
    };
    assert_ne!(model.value, SwitchValue::Unknown);
    let explanations = switch_explanations(&opencode, &implicit(&["--model", "x/y"]));
    assert_eq!(explanations.len(), 1);
    assert!(
        explanations[0].starts_with("--model is one of OpenCode's switches (")
            && explanations[0].ends_with("); forwarding to OpenCode."),
        "{}",
        explanations[0]
    );
    assert!(!explanations[0].contains("a OpenCode"));
}
