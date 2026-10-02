use super::*;
use crate::provider::{CliSwitch, CliSwitchCatalog, SwitchScope, match_token_in};

fn s(tokens: &[&str]) -> Vec<String> {
    tokens.iter().map(|token| token.to_string()).collect()
}

/// Tokens after the file; `|` stands for a removed Claudine option.
fn after_file(tokens: &[&str]) -> ArgumentsAfterFile {
    after_file_with(tokens, None)
}

fn after_file_with(tokens: &[&str], opaque: Option<&[&str]>) -> ArgumentsAfterFile {
    let arguments = tokens
        .iter()
        .map(|token| match *token {
            "|" => CallerArgument::ClaudineOption,
            other => CallerArgument::Token(other.to_string()),
        })
        .collect();
    ArgumentsAfterFile::new(arguments, opaque.map(s))
}

/// A candidate at `path`, the command path its non-interactive launch uses
/// (`exec` for Codex, `run` for OpenCode, the root for Claude).
fn candidate_at(provider: Provider, path: &[&str]) -> OwnershipCandidate {
    OwnershipCandidate { provider, command_path: s(path) }
}

fn candidate(provider: Provider) -> OwnershipCandidate {
    candidate_at(provider, &[])
}

fn codex() -> Vec<OwnershipCandidate> {
    vec![candidate_at(Provider::Codex, &["exec"])]
}

fn claude() -> Vec<OwnershipCandidate> {
    vec![candidate(Provider::Claude)]
}

fn claude_and_codex() -> Vec<OwnershipCandidate> {
    vec![candidate(Provider::Claude), candidate_at(Provider::Codex, &["exec"])]
}

fn every_provider() -> Vec<OwnershipCandidate> {
    crate::provider::PROVIDERS_DISPLAY_ORDER
        .iter()
        .map(|provider| candidate(*provider))
        .collect()
}

fn own(tokens: &[&str], schema: &SchemaParameters, candidates: &[OwnershipCandidate]) -> OwnedArguments {
    own_arguments(&after_file(tokens), schema, candidates)
        .unwrap_or_else(|err| panic!("{tokens:?} should be owned: {err}"))
}

fn own_err(tokens: &[&str], schema: &SchemaParameters, candidates: &[OwnershipCandidate]) -> OwnershipError {
    match own_arguments(&after_file(tokens), schema, candidates) {
        Ok(owned) => panic!(
            "{tokens:?} should fail; claudine {:?}, tail {:?}",
            owned.claudine,
            owned.tail.launch_args()
        ),
        Err(err) => err,
    }
}

fn phase_schema() -> SchemaParameters {
    SchemaParameters::from_names(["phase"])
}

fn assigned(owned: &OwnedArguments) -> Vec<(usize, std::ops::Range<usize>)> {
    owned
        .tail
        .assignments()
        .iter()
        .map(|assignment| (assignment.switch, assignment.values.clone()))
        .collect()
}

// ── The spec's three example commands ──

#[test]
fn a_string_switch_takes_one_setter_and_the_next_setter_is_claudines() {
    for candidates in [codex(), every_provider()] {
        let owned = own(&["-c", "model_reasoning_effort=low", "phase=2"], &phase_schema(), &candidates);
        assert_eq!(owned.tail.launch_args(), s(&["-c", "model_reasoning_effort=low"]));
        assert_eq!(owned.claudine, s(&["phase=2"]));
        assert_eq!(assigned(&owned), [(0, 1..2)]);
    }
    // Without a schema the second setter is still Claudine's: it follows a
    // value, not a switch.
    let owned = own(&["-c", "model_reasoning_effort=low", "phase=2"], &SchemaParameters::NoSchema, &every_provider());
    assert_eq!(owned.claudine, s(&["phase=2"]));
}

#[test]
fn a_variadic_switch_takes_a_contiguous_run_and_never_a_later_setter() {
    let owned = own(&["--add-dir", "a", "b", "x=y"], &SchemaParameters::NoSchema, &claude());
    assert_eq!(owned.tail.launch_args(), s(&["--add-dir", "a", "b"]));
    assert_eq!(owned.claudine, s(&["x=y"]));
    assert_eq!(assigned(&owned), [(0, 1..3)]);

    // A setter as the first value is the switch's (rule 3); later bare words
    // still join the run.
    let owned = own(&["--add-dir", "x=y", "a"], &SchemaParameters::NoSchema, &claude());
    assert_eq!(owned.tail.launch_args(), s(&["--add-dir", "x=y", "a"]));
    assert!(owned.claudine.is_empty());

    // A setter ends the run; a later bare word does not reconnect to it.
    let owned = own(&["--add-dir", "a", "k=v", "b"], &SchemaParameters::NoSchema, &claude());
    assert_eq!(owned.tail.launch_args(), s(&["--add-dir", "a"]));
    assert_eq!(owned.claudine, s(&["k=v", "b"]));
}

// ── Rule 1: a Claudine option ends a value run ──

#[test]
fn a_claudine_option_interrupts_a_value_run() {
    // `--codex -c phase=2 x=y`: `phase=2` is the schema's, which leaves `-c`
    // empty; `x=y` must not become its value.
    let err = own_err(&["-c", "phase=2", "x=y"], &phase_schema(), &codex());
    let OwnershipError::Mismatch(mismatch) = &err else { panic!("{err:?}") };
    assert_eq!(mismatch.kind, MismatchKind::MissingValue);
    assert_eq!((mismatch.provider, mismatch.switch.as_str()), (Provider::Codex, "-c"));

    // A removed Claudine option between a switch and a bare word.
    let err = own_err(&["-c", "|", "foo"], &SchemaParameters::NoSchema, &codex());
    assert!(matches!(err, OwnershipError::Mismatch(TailMismatch { kind: MismatchKind::MissingValue, .. })), "{err:?}");

    // Claude's `-c` takes nothing, so the bare word is a positional either way.
    let owned = own(&["-c", "|", "foo"], &SchemaParameters::NoSchema, &claude());
    assert_eq!(owned.claudine, s(&["foo"]));
}

// ── Rule 2: schema parameters and the reserved `argv` ──

#[test]
fn a_declared_parameter_is_claudines_even_directly_after_a_switch() {
    // Codex is left with an empty `-c`, but Claude's `-c` takes nothing, so
    // ownership (which fails only when every candidate is wrong) lets it
    // through for the resolved-provider check to judge.
    let owned = own(&["-c", "phase=2", "foo"], &phase_schema(), &claude_and_codex());
    assert_eq!(owned.claudine, s(&["phase=2", "foo"]));
    assert_eq!(owned.tail.launch_args(), s(&["-c"]));
    assert_eq!(assigned(&owned), [(0, 1..1)]);
    let mismatch = check_launch_tail(&owned.tail, Provider::Codex, &["exec"]).unwrap_err();
    assert_eq!(mismatch.kind, MismatchKind::MissingValue);
}

#[test]
fn argv_is_reserved_before_the_separator_wherever_it_appears() {
    for tokens in [&["argv=a"][..], &["-c", "argv=a"], &["--add-dir", "a", "argv=[1]"], &["argv=null"]] {
        assert_eq!(own_err(tokens, &SchemaParameters::NoSchema, &codex()), OwnershipError::ReservedArgv, "{tokens:?}");
    }
    // After `--` it is opaque provider data.
    let owned = own_arguments(&after_file_with(&[], Some(&["argv=a"])), &SchemaParameters::NoSchema, &codex()).unwrap();
    assert_eq!(owned.tail.launch_args(), s(&["argv=a"]));
    assert!(owned.claudine.is_empty());
}

#[test]
fn no_schema_declared_names_and_unestablished_names_are_three_outcomes() {
    let tokens = ["-c", "phase=2"];
    let no_schema = own(&tokens, &SchemaParameters::NoSchema, &codex());
    assert_eq!(no_schema.tail.launch_args(), s(&tokens));

    let no_names = own(&tokens, &SchemaParameters::from_names(Vec::<String>::new()), &codex());
    assert_eq!(no_names.tail.launch_args(), s(&tokens), "an empty schema declares nothing");

    assert!(matches!(own_err(&tokens, &phase_schema(), &codex()), OwnershipError::Mismatch(_)));

    let err = own_err(&tokens, &SchemaParameters::Unestablished, &codex());
    assert_eq!(
        err,
        OwnershipError::ContestedSetter { switch: "-c".to_string(), key: "phase".to_string() }
    );
    let message = err.to_string();
    assert!(message.contains("`--`") && message.contains("--set"), "{message}");

    // Unestablished names matter only where a switch would take the setter.
    let owned = own(&["phase=2", "--json", "k=v"], &SchemaParameters::Unestablished, &codex());
    assert_eq!(owned.claudine, s(&["phase=2", "k=v"]));
}

// ── Rule 4: union readings and ambiguity ──

#[test]
fn candidates_that_disagree_over_a_bare_word_are_ambiguous() {
    let err = own_err(&["-c", "foo"], &SchemaParameters::NoSchema, &claude_and_codex());
    let OwnershipError::Ambiguous(ambiguous) = &err else { panic!("{err:?}") };
    assert_eq!(ambiguous.switch, "-c");
    assert_eq!(
        ambiguous.readings,
        [
            SwitchReading { provider: Provider::Claude, value: SwitchValue::None, takes_word: false },
            SwitchReading {
                provider: Provider::Codex,
                value: SwitchValue::String { optional: false },
                takes_word: true
            },
        ]
    );
    let message = err.to_string();
    assert!(message.starts_with("ambiguous provider argument"), "{message}");
    assert!(message.contains("Claude: takes no value") && message.contains("Codex: takes one value"), "{message}");
    assert!(message.contains("`--`"), "{message}");
    assert!(!message.contains("foo"), "the word itself is never echoed: {message}");

    // One candidate: no disagreement.
    assert_eq!(own(&["-c", "foo"], &SchemaParameters::NoSchema, &codex()).tail.launch_args(), s(&["-c", "foo"]));
    assert_eq!(own(&["-c", "foo"], &SchemaParameters::NoSchema, &claude()).claudine, s(&["foo"]));
}

#[test]
fn scalar_against_variadic_is_ambiguous_at_the_second_word() {
    // Claude's `--add-dir` is variadic at its root; Codex's is one string.
    let owned = own(&["--add-dir", "a"], &SchemaParameters::NoSchema, &claude_and_codex());
    assert_eq!(owned.tail.launch_args(), s(&["--add-dir", "a"]));
    let err = own_err(&["--add-dir", "a", "b"], &SchemaParameters::NoSchema, &claude_and_codex());
    assert!(matches!(err, OwnershipError::Ambiguous(_)), "{err:?}");
}

#[test]
fn unknown_is_never_none() {
    // Not in Codex's catalog: unknown, which takes the next bare word.
    let owned = own(&["--frobnicate", "foo"], &SchemaParameters::NoSchema, &codex());
    assert_eq!(owned.tail.launch_args(), s(&["--frobnicate", "foo"]));
    // Known as none for Claude, unknown for Codex: they disagree.
    let err = own_err(&["--continue", "foo"], &SchemaParameters::NoSchema, &claude_and_codex());
    assert!(matches!(err, OwnershipError::Ambiguous(_)), "{err:?}");
    // A researched record with an unknown type reads the same way.
    let opencode = vec![candidate_at(Provider::OpenCode, &["run"])];
    let owned = own(&["--get-yargs-completions", "foo"], &SchemaParameters::NoSchema, &opencode);
    assert_eq!(owned.tail.launch_args(), s(&["--get-yargs-completions", "foo"]));
}

/// Every reader of a compiled record typed unknown (OpenCode's global
/// `--get-yargs-completions`, OpenCode's and Kilo's `--models` at `stats`)
/// treats it as having no record: ownership applies rule 5 (one bare word,
/// never a setter), the resolved-provider check leaves it to the provider,
/// and completion reads the next word as the provider's.
#[test]
fn a_record_typed_unknown_is_unrecognized_for_every_reader() {
    let records: &[(Provider, &[&str], &str)] = &[
        (Provider::OpenCode, &["run"], "--get-yargs-completions"),
        (Provider::OpenCode, &["stats"], "--models"),
        (Provider::Kilo, &["stats"], "--models"),
    ];
    for (provider, path, switch) in records {
        let matched = match_switch_token(*provider, path, switch).expect("compiled record");
        assert_eq!(matched.switch.value, SwitchValue::Unknown, "{provider} {switch}");
        let candidates = vec![candidate_at(*provider, path)];

        let owned = own(&[switch, "k=v", "foo"], &SchemaParameters::NoSchema, &candidates);
        assert_eq!(owned.tail.launch_args(), s(&[switch]), "{provider} {switch}");
        assert_eq!(owned.claudine, s(&["k=v", "foo"]), "{provider} {switch}");

        let owned = own(&[switch, "a", "b"], &SchemaParameters::NoSchema, &candidates);
        assert_eq!(owned.tail.launch_args(), s(&[switch, "a"]), "{provider} {switch}");
        for tail in [&owned.tail, &own(&[switch], &SchemaParameters::NoSchema, &candidates).tail] {
            assert_eq!(check_launch_tail(tail, *provider, path), Ok(()), "{provider} {switch}");
        }

        assert_eq!(
            last_owner(&[switch, "fo"], &SchemaParameters::NoSchema, &candidates).unwrap(),
            Some(ArgumentOwner::Provider),
            "{provider} {switch}"
        );
        assert_eq!(
            last_owner(&[switch, "k=v"], &SchemaParameters::NoSchema, &candidates).unwrap(),
            Some(ArgumentOwner::Claudine),
            "{provider} {switch}"
        );
    }
}

// ── Rule 5: unrecognized switches ──

#[test]
fn an_unrecognized_switch_takes_a_bare_word_but_never_a_setter() {
    let owned = own(&["--frobnicate", "k=v", "foo"], &SchemaParameters::NoSchema, &codex());
    assert_eq!(owned.tail.launch_args(), s(&["--frobnicate"]));
    assert_eq!(owned.claudine, s(&["k=v", "foo"]));
    // Only one word.
    let owned = own(&["--frobnicate", "a", "b"], &SchemaParameters::NoSchema, &codex());
    assert_eq!(owned.claudine, s(&["b"]));
}

// ── Rule 6: exact spellings and researched attached forms ──

#[test]
fn attached_values_stay_in_one_token_and_take_nothing_more() {
    for attached in ["-cfoo", "--config=foo", "-c=foo"] {
        let owned = own(&[attached, "bar"], &SchemaParameters::NoSchema, &codex());
        assert_eq!(owned.tail.launch_args(), s(&[attached]), "{attached}");
        assert_eq!(owned.claudine, s(&["bar"]), "{attached}");
    }
    // An unresearched long `--name=value` holds its own value too.
    let owned = own(&["--frobnicate=1", "bar"], &SchemaParameters::NoSchema, &codex());
    assert_eq!(owned.claudine, s(&["bar"]));
    // Claude's `-c` takes no value and no attached form, so `-cfoo` is not
    // split: it is unrecognized and takes the next bare word.
    let owned = own(&["-cfoo", "bar"], &SchemaParameters::NoSchema, &claude());
    assert_eq!(owned.tail.launch_args(), s(&["-cfoo", "bar"]));
}

#[test]
fn an_alias_and_its_canonical_spelling_read_the_same() {
    for spelling in ["-c", "--config"] {
        let owned = own(&[spelling, "x=y", "z=w"], &SchemaParameters::NoSchema, &codex());
        assert_eq!(owned.tail.launch_args(), s(&[spelling, "x=y"]), "{spelling}");
        assert_eq!(owned.claudine, s(&["z=w"]));
    }
}

#[test]
fn an_attached_only_optional_value_never_takes_a_separate_word() {
    // Claude's `--debug` takes an optional value, attached only.
    let owned = own(&["--debug", "api"], &SchemaParameters::NoSchema, &claude());
    assert_eq!(owned.tail.launch_args(), s(&["--debug"]));
    assert_eq!(owned.claudine, s(&["api"]));
    let owned = own(&["--debug=api", "x"], &SchemaParameters::NoSchema, &claude());
    assert_eq!(owned.tail.launch_args(), s(&["--debug=api"]));
}

// ── Rule 7 and value edge cases ──

#[test]
fn a_dash_value_must_be_attached() {
    let err = own_err(&["-c", "-x"], &SchemaParameters::NoSchema, &codex());
    assert!(matches!(err, OwnershipError::Mismatch(TailMismatch { kind: MismatchKind::MissingValue, .. })), "{err:?}");
    let owned = own(&["--config=-0.5"], &SchemaParameters::NoSchema, &codex());
    assert_eq!(owned.tail.launch_args(), s(&["--config=-0.5"]));
    // A lone `-` is a bare word.
    let owned = own(&["-c", "-"], &SchemaParameters::NoSchema, &codex());
    assert_eq!(owned.tail.launch_args(), s(&["-c", "-"]));
}

#[test]
fn an_empty_value_is_a_value() {
    let owned = own(&["-c", ""], &SchemaParameters::NoSchema, &codex());
    assert_eq!(owned.tail.launch_args(), s(&["-c", ""]));
    let owned = own(&["--config="], &SchemaParameters::NoSchema, &codex());
    assert_eq!(owned.tail.launch_args(), s(&["--config="]));
}

#[test]
fn a_number_switch_takes_only_a_number() {
    let owned = own(&["--max-turns", "5"], &SchemaParameters::NoSchema, &claude());
    assert_eq!(owned.tail.launch_args(), s(&["--max-turns", "5"]));
    let err = own_err(&["--max-turns", "abc"], &SchemaParameters::NoSchema, &claude());
    assert!(matches!(err, OwnershipError::Mismatch(TailMismatch { kind: MismatchKind::MissingValue, .. })), "{err:?}");
}

#[test]
fn the_number_grammar_is_a_finite_decimal() {
    for number in ["5", "+5", "-5", "0.5", ".5", "5.", "1e3", "1E-3", "+2.5e+10"] {
        assert!(is_ownership_number(number), "{number}");
    }
    for word in ["", "+", ".", "e3", "1e", "0x10", "NaN", "inf", "infinity", "1_000", "1.2.3", "5 ", "abc"] {
        assert!(!is_ownership_number(word), "{word}");
    }
}

#[test]
fn a_repeated_scalar_switch_is_two_assignments() {
    let owned = own(&["-c", "a=1", "-c", "a=1"], &SchemaParameters::NoSchema, &codex());
    assert_eq!(owned.tail.launch_args(), s(&["-c", "a=1", "-c", "a=1"]));
    assert_eq!(assigned(&owned), [(0, 1..2), (2, 3..4)]);
}

// ── Rule 8: positionals ──

#[test]
fn bare_words_no_switch_takes_are_positionals_in_order() {
    let owned = own(&["alpha", "-c", "x=y", "beta", "phase=2"], &phase_schema(), &codex());
    assert_eq!(owned.claudine, s(&["alpha", "beta", "phase=2"]));
    assert_eq!(owned.tail.launch_args(), s(&["-c", "x=y"]));
    // Duplicates are preserved.
    let owned = own(&["a", "a"], &SchemaParameters::NoSchema, &codex());
    assert_eq!(owned.claudine, s(&["a", "a"]));
}

// ── Rule 9: the explicit tail is never classified or checked ──

#[test]
fn the_opaque_suffix_is_kept_and_never_checked() {
    let arguments = after_file_with(&["-c", "x=y"], Some(&["-c", "--", "phase=2"]));
    let owned = own_arguments(&arguments, &phase_schema(), &codex()).unwrap();
    assert_eq!(owned.tail.launch_args(), s(&["-c", "x=y", "-c", "--", "phase=2"]));
    assert_eq!(owned.tail.boundary(), Some(2));
    assert_eq!(assigned(&owned), [(0, 1..2)], "only the implicit prefix is assigned");
    assert!(owned.claudine.is_empty());
    assert_eq!(check_launch_tail(&owned.tail, Provider::Codex, &["exec"]), Ok(()));

    let explicit = own_arguments(&after_file_with(&[], Some(&["-c"])), &SchemaParameters::NoSchema, &codex()).unwrap();
    assert_eq!(explicit.tail.boundary(), Some(0));
    assert!(explicit.tail.assignments().is_empty());
}

// ── The resolved-provider check ──

#[test]
fn a_union_value_fails_for_the_provider_that_takes_none() {
    let owned = own(&["-c", "model_reasoning_effort=low"], &SchemaParameters::NoSchema, &claude_and_codex());
    assert_eq!(check_launch_tail(&owned.tail, Provider::Codex, &["exec"]), Ok(()));
    let mismatch = check_launch_tail(&owned.tail, Provider::Claude, &[]).unwrap_err();
    assert_eq!(mismatch.kind, MismatchKind::ExtraValue);
    assert_eq!(mismatch.value.as_deref(), Some("model_reasoning_effort=low"));
    let message = mismatch.to_string();
    assert!(message.contains("`-c`") && message.contains("Claude") && message.contains("root command"), "{message}");
}

#[test]
fn the_check_uses_the_resume_entrypoint_answers() {
    // Codex `-i` is variadic at `exec` and one string at `exec resume`.
    let owned = own(&["-i", "a.png", "b.png"], &SchemaParameters::NoSchema, &codex());
    assert_eq!(check_launch_tail(&owned.tail, Provider::Codex, &["exec"]), Ok(()));
    let mismatch = check_launch_tail(&owned.tail, Provider::Codex, &["exec", "resume"]).unwrap_err();
    assert_eq!((mismatch.kind, mismatch.value.as_deref()), (MismatchKind::ExtraValue, Some("b.png")));
    assert!(mismatch.to_string().contains("`exec resume` command"), "{mismatch}");
}

#[test]
fn a_tail_without_assignments_always_passes() {
    let direct = ProviderTail::new(s(&["-c", "x=y", "--max-turns", "abc"]), None);
    assert_eq!(check_launch_tail(&direct, Provider::Claude, &[]), Ok(()));
}

#[test]
fn every_candidate_must_be_wrong_for_ownership_to_fail() {
    // A trailing `-c` is empty for Codex but correct for Claude.
    let owned = own(&["-c"], &SchemaParameters::NoSchema, &claude_and_codex());
    assert_eq!(owned.tail.launch_args(), s(&["-c"]));
    assert!(check_launch_tail(&owned.tail, Provider::Codex, &["exec"]).is_err());
    assert!(matches!(own_err(&["-c"], &SchemaParameters::NoSchema, &codex()), OwnershipError::Mismatch(_)));
}

#[test]
fn a_mismatch_never_shows_a_recognized_secret() {
    let secret = "sk-proj-ownershipsecret0123456789";
    let token = format!("token={secret}");
    let owned = own(&["-c", &token], &SchemaParameters::NoSchema, &claude_and_codex());
    let mismatch = check_launch_tail(&owned.tail, Provider::Claude, &[]).unwrap_err();
    let rendered = format!("{mismatch} {mismatch:?}");
    assert!(!rendered.contains(secret), "{rendered}");
    assert!(display_value("--api-key", "plain") == crate::secrets::MASK);
    assert_eq!(display_value("-c", "a\u{1b}[31mb"), "a\\u{1b}[31mb");
    // A short credential the catalog shapes would miss is still an argument secret.
    assert_eq!(display_value("-c", "endpoint=sk-short"), "endpoint=****");
}

#[test]
fn debug_output_never_shows_a_token() {
    let arguments = after_file_with(&["-c", "sk-secret"], Some(&["--token=sk-other"]));
    let rendered = format!("{arguments:?}");
    assert!(!rendered.contains("sk-") && !rendered.contains("token"), "{rendered}");
    let owned = own_arguments(&arguments, &SchemaParameters::NoSchema, &codex()).unwrap();
    let rendered = format!("{owned:?} {:?}", SchemaParameters::from_names(["secret-name"]));
    assert!(!rendered.contains("sk-") && !rendered.contains("secret-name"), "{rendered}");
}

// ── A fixed catalog: shapes the research does not contain today ──

static PAIR: &[CliSwitch] = &[CliSwitch {
    flag: "--pair",
    aliases: &[],
    value: SwitchValue::Variadic { min: VariadicMin::AtLeast(2) },
    attachments: &[SwitchAttachment::Space, SwitchAttachment::Equals],
    scopes: &[SwitchScope::Global],
    description: "Takes at least two values.",
    gap: None,
}, CliSwitch {
    flag: "--maybe",
    aliases: &[],
    value: SwitchValue::Variadic { min: VariadicMin::Unknown },
    attachments: &[SwitchAttachment::Space],
    scopes: &[SwitchScope::Global],
    description: "Minimum not established.",
    gap: Some("minimum not established"),
}];

struct Fixed;

impl SwitchSource for Fixed {
    fn matched<'t>(&self, _: Provider, path: &[&str], token: &'t str) -> Option<SwitchToken<'t>> {
        match_token_in(CliSwitchCatalog::Researched(PAIR), path, token)
    }
}

#[test]
fn a_variadic_minimum_is_checked_and_an_unknown_minimum_never_fails() {
    let fixed = |tokens: &[&str]| {
        own_in(&Fixed, &after_file(tokens), &SchemaParameters::NoSchema, &codex()).map(|(owned, _)| owned)
    };
    let one = fixed(&["--pair", "a"]);
    let Err(OwnershipError::Mismatch(mismatch)) = one else { panic!("{one:?}") };
    assert_eq!(mismatch.kind, MismatchKind::TooFewValues { min: 2, given: 1 });
    assert!(mismatch.to_string().contains("at least 2 values"), "{mismatch}");

    let two = fixed(&["--pair", "a", "b"]).unwrap();
    assert_eq!(two.tail.launch_args(), s(&["--pair", "a", "b"]));
    // An attached first value counts and is not extended.
    let attached = fixed(&["--pair=a", "b"]);
    let Err(OwnershipError::Mismatch(mismatch)) = attached else { panic!("{attached:?}") };
    assert_eq!(mismatch.kind, MismatchKind::TooFewValues { min: 2, given: 1 });

    let none = fixed(&["--maybe"]).unwrap();
    assert_eq!(none.tail.launch_args(), s(&["--maybe"]));
}

fn last_owner(
    tokens: &[&str],
    schema: &SchemaParameters,
    candidates: &[OwnershipCandidate],
) -> Result<Option<ArgumentOwner>, OwnershipError> {
    owner_of_last_argument(&after_file(tokens), schema, candidates)
}

#[test]
fn the_last_argument_owner_follows_the_same_rules_as_ownership() {
    let phase = SchemaParameters::from_names(["phase"]);
    let cases: &[(&[&str], &[OwnershipCandidate], Option<ArgumentOwner>)] = &[
        // A word an open string switch takes is the provider's, even when
        // it is empty (the cursor sits on an unfinished value).
        (&["-c", ""], &codex(), Some(ArgumentOwner::Provider)),
        (&["-c", "mod"], &codex(), Some(ArgumentOwner::Provider)),
        // A declared parameter directly after the switch is Claudine's.
        (&["-c", "low", "ph"], &codex(), Some(ArgumentOwner::Claudine)),
        (&["-c", "x=y", "phase="], &codex(), Some(ArgumentOwner::Claudine)),
        // A removed Claudine option is nobody's word, and it ends the run:
        // the word after it is Claudine's.
        (&["-c", "x=y", "|"], &codex(), None),
        (&["-c", "x=y", "|", "ph"], &codex(), Some(ArgumentOwner::Claudine)),
        // No switch: candidates are never consulted.
        (&["alpha", "ph"], &[], Some(ArgumentOwner::Claudine)),
        (&[], &[], None),
    ];
    for (tokens, candidates, expected) in cases {
        assert_eq!(last_owner(tokens, &phase, candidates).unwrap(), *expected, "{tokens:?}");
    }
}

#[test]
fn the_last_argument_owner_reports_every_ownership_error() {
    let phase = SchemaParameters::from_names(["phase"]);
    // A declared parameter leaves `-c` without a value: terminal, not an
    // unfinished slot.
    assert!(matches!(
        last_owner(&["-c", "phase=2", "ph"], &phase, &codex()),
        Err(OwnershipError::Mismatch(TailMismatch { kind: MismatchKind::MissingValue, .. }))
    ));
    assert!(matches!(
        last_owner(&["-c", ""], &SchemaParameters::NoSchema, &claude_and_codex()),
        Err(OwnershipError::Ambiguous(_))
    ));
    assert!(matches!(
        last_owner(&["-c", "x=y"], &SchemaParameters::Unestablished, &codex()),
        Err(OwnershipError::ContestedSetter { .. })
    ));
    assert!(matches!(
        last_owner(&["argv="], &SchemaParameters::NoSchema, &[]),
        Err(OwnershipError::ReservedArgv)
    ));
}
