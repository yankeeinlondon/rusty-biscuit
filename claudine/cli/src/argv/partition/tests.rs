//! The pre-clap partition, and the partition followed by type-aware
//! ownership: what reaches clap, what reaches ownership, and the exact
//! forwarded tokens.

use claudine::composition::{OwnershipCandidate, OwnershipError, SchemaParameters, own_arguments};
use claudine::provider::Provider;

use super::*;

fn argv(tokens: &[&str]) -> Vec<OsString> {
    tokens.iter().map(OsString::from).collect()
}

fn strs(tokens: &[&str]) -> Vec<String> {
    tokens.iter().map(|s| s.to_string()).collect()
}

/// The Claudine argv, and the arguments after the file with `|` standing for
/// a removed Claudine option.
fn partition(tokens: &[&str]) -> (Vec<String>, Vec<String>, Option<Vec<String>>) {
    let (claudine, after) = partition_composition_tail(argv(tokens)).expect("no partition error");
    let claudine = claudine
        .into_iter()
        .map(|t| t.to_string_lossy().into_owned())
        .collect();
    let arguments = after
        .arguments()
        .iter()
        .map(|argument| match argument {
            CallerArgument::Token(token) => token.clone(),
            CallerArgument::ClaudineOption => "|".to_string(),
        })
        .collect();
    (claudine, arguments, after.opaque().map(<[String]>::to_vec))
}

fn codex() -> Vec<OwnershipCandidate> {
    vec![OwnershipCandidate {
        provider: Provider::Codex,
        command_path: strs(&["exec"]),
    }]
}

/// Partition, then own with `schema` for Codex: (clap argv + owned Claudine
/// tokens, forwarded tokens).
fn owned(tokens: &[&str], schema: &SchemaParameters) -> Result<(Vec<String>, Vec<String>), OwnershipError> {
    let (claudine, after) = partition_composition_tail(argv(tokens)).expect("no partition error");
    let owned = own_arguments(&after, schema, &codex())?;
    let mut claudine: Vec<String> = claudine
        .into_iter()
        .map(|t| t.to_string_lossy().into_owned())
        .collect();
    claudine.extend(owned.claudine);
    Ok((claudine, owned.tail.launch_args().to_vec()))
}

#[test]
fn non_composition_argv_passes_through() {
    let (claudine, after, opaque) = partition(&["claudine", "codex", "-c", "x"]);
    assert_eq!(claudine, strs(&["claudine", "codex", "-c", "x"]));
    assert!(after.is_empty() && opaque.is_none());
}

#[test]
fn everything_after_the_file_waits_for_ownership() {
    let (claudine, after, opaque) = partition(&[
        "claudine", "sequence", "fleet.md", "-y", "--provider", "codex", "-c", "model_reasoning_effort=low", "phase=2",
    ]);
    assert_eq!(claudine, strs(&["claudine", "sequence", "fleet.md", "-y", "--provider", "codex"]));
    assert_eq!(after, strs(&["|", "|", "-c", "model_reasoning_effort=low", "phase=2"]));
    assert_eq!(opaque, None);
}

#[test]
fn setters_before_the_file_stay_with_clap() {
    let (claudine, after, _) = partition(&["claudine", "compose", "name=Ken", "file.md", "x=y"]);
    assert_eq!(claudine, strs(&["claudine", "compose", "name=Ken", "file.md"]));
    assert_eq!(after, strs(&["x=y"]));
}

#[test]
fn a_claudine_option_after_a_provider_switch_is_reclaimed_and_marked() {
    let (claudine, after, _) = partition(&["claudine", "compose", "file.md", "--config", "-m", "gpt5", "foo"]);
    assert_eq!(claudine, strs(&["claudine", "compose", "file.md", "-m", "gpt5"]));
    assert_eq!(after, strs(&["--config", "|", "foo"]));
}

#[test]
fn explicit_separator_forwards_an_opaque_tail() {
    let (claudine, after, opaque) = partition(&["claudine", "compose", "file.md", "--", "-c", "--silent", "value"]);
    assert_eq!(claudine, strs(&["claudine", "compose", "file.md"]));
    assert!(after.is_empty());
    // Even `--silent` (a Claudine flag) is opaque after `--`.
    assert_eq!(opaque, Some(strs(&["-c", "--silent", "value"])));
}

#[test]
fn only_the_first_separator_is_consumed() {
    let (_, _, opaque) = partition(&["claudine", "compose", "file.md", "--", "a", "--", "b"]);
    assert_eq!(opaque, Some(strs(&["a", "--", "b"])));
    let (_, _, empty) = partition(&["claudine", "compose", "file.md", "-c", "x", "--"]);
    assert_eq!(empty, Some(Vec::new()), "an authored empty suffix is kept");
}

#[test]
fn switch_before_file_errors() {
    let err = partition_composition_tail(argv(&["claudine", "compose", "--unknown", "file.md"]))
        .expect_err("switch before file must error");
    assert!(matches!(err, PartitionError::SwitchBeforeFile { .. }));
}

#[test]
fn separator_before_file_errors() {
    let err = partition_composition_tail(argv(&["claudine", "compose", "--", "file.md"]))
        .expect_err("separator before file must error");
    assert!(matches!(err, PartitionError::SeparatorBeforeFile { .. }));
}

#[test]
fn value_flag_value_is_not_mistaken_for_file() {
    let (claudine, after, _) = partition(&["claudine", "compose", "-m", "gpt5", "file.md", "--x"]);
    assert_eq!(claudine, strs(&["claudine", "compose", "-m", "gpt5", "file.md"]));
    assert_eq!(after, strs(&["--x"]));
}

#[test]
fn a_second_bare_word_is_no_longer_a_second_file() {
    let (claudine, after, _) = partition(&["claudine", "compose", "a.md", "b.md"]);
    assert_eq!(claudine, strs(&["claudine", "compose", "a.md"]));
    assert_eq!(after, strs(&["b.md"]));
}

#[test]
fn bundled_bool_shorts_are_owned() {
    let (claudine, after, _) = partition(&["claudine", "compose", "file.md", "-yq", "--x"]);
    assert_eq!(claudine, strs(&["claudine", "compose", "file.md", "-yq"]));
    assert_eq!(after, strs(&["|", "--x"]));
}

// ── Partition followed by ownership: the exact forwarded tokens ──

#[test]
fn the_headline_command_forwards_one_setter_and_applies_the_next() {
    let (claudine, forwarded) = owned(
        &["claudine", "compose", "plan.md", "--provider", "codex", "-c", "model_reasoning_effort=low", "phase=2"],
        &SchemaParameters::from_names(["phase"]),
    )
    .unwrap();
    assert_eq!(claudine, strs(&["claudine", "compose", "plan.md", "--provider", "codex", "phase=2"]));
    assert_eq!(forwarded, strs(&["-c", "model_reasoning_effort=low"]));
}

#[test]
fn a_claudine_token_between_a_switch_and_a_word_never_joins_them() {
    // `-c -y foo`: `-y` is Claudine's, so `foo` is not `-c`'s value, and
    // Codex's `-c` is left empty.
    let err = owned(&["claudine", "compose", "plan.md", "-c", "-y", "foo"], &SchemaParameters::NoSchema)
        .expect_err("an interrupted value run must fail");
    assert!(matches!(err, OwnershipError::Mismatch(_)), "{err:?}");
    // `--codex -c phase=2 x=y` with `phase` declared fails rather than
    // attaching `x=y`.
    let err = owned(
        &["claudine", "compose", "plan.md", "-c", "phase=2", "x=y"],
        &SchemaParameters::from_names(["phase"]),
    )
    .expect_err("a schema parameter leaves -c empty");
    assert!(matches!(err, OwnershipError::Mismatch(_)), "{err:?}");
}

#[test]
fn positionals_are_left_for_argv_in_order() {
    let (claudine, forwarded) = owned(
        &["claudine", "compose", "plan.md", "alpha", "-c", "x=y", "beta", "phase=2"],
        &SchemaParameters::NoSchema,
    )
    .unwrap();
    assert_eq!(claudine, strs(&["claudine", "compose", "plan.md", "alpha", "beta", "phase=2"]));
    assert_eq!(forwarded, strs(&["-c", "x=y"]));
}

#[test]
fn a_mixed_tail_keeps_its_boundary_and_only_the_prefix_is_owned() {
    let (claudine, after) =
        partition_composition_tail(argv(&["claudine", "compose", "plan.md", "-c", "x=y", "--", "--native", "phase=2"]))
            .unwrap();
    let owned = own_arguments(&after, &SchemaParameters::from_names(["phase"]), &codex()).unwrap();
    assert_eq!(claudine.len(), 3);
    assert!(owned.claudine.is_empty());
    assert_eq!(owned.tail.launch_args(), strs(&["-c", "x=y", "--native", "phase=2"]));
    assert_eq!(owned.tail.boundary(), Some(2));
    assert_eq!(owned.tail.assignments().len(), 1);
}

// ── Drift detection: the owned surface is derived from clap definitions ──

#[test]
fn owned_surface_is_derived_from_clap_and_non_empty() {
    let owned = OwnedFlags::for_composition();
    assert!(
        !owned.value_flags.is_empty() && !owned.bool_flags.is_empty(),
        "owned surface must be populated from the clap command definitions"
    );
    // Representative value-bearing options (space form consumes next token).
    for flag in ["--provider", "--model", "-m", "--set", "--output", "-o"] {
        assert!(
            owned.is_value_flag(flag),
            "{flag} must be a value-bearing owned flag; if the clap surface \
             changed, this drift test is the intended failure point"
        );
    }
    // Representative boolean options.
    for flag in ["--yolo", "-y", "--silent", "--dry-run", "--sandbox"] {
        assert!(owned.is_bool_flag(flag), "{flag} must be a boolean owned flag");
    }
    // The `sequence`-only flag is in the union.
    assert!(owned.is_value_flag("--fail-fast"));
}

// ── Non-UTF-8 refusal: never rewrite the bytes a caller forwarded ──

#[cfg(unix)]
fn invalid_token() -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(vec![b'b', 0xFF, b'd'])
}

#[cfg(windows)]
fn invalid_token() -> OsString {
    use std::os::windows::ffi::OsStringExt;
    // An unpaired surrogate: valid WTF-16, not valid UTF-8.
    OsString::from_wide(&[0x0062, 0xD800, 0x0064])
}

fn with_invalid(tokens: &[&str], at: usize) -> Vec<OsString> {
    let mut argv = argv(tokens);
    argv.insert(at, invalid_token());
    argv
}

#[test]
fn a_non_utf8_argument_after_the_file_is_refused_by_position() {
    let argv = with_invalid(&["claudine", "compose", "file.md", "--x", "-c"], 5);
    let err = partition_composition_tail(argv).expect_err("lossy conversion must be refused");
    assert_eq!(err, PartitionError::NonUtf8Argument { position: 3 });
    let message = err.to_string();
    assert!(message.contains("argument 3 after the composition file"), "{message}");
    assert!(message.contains("not valid UTF-8"), "{message}");
    assert!(!message.contains('\u{FFFD}'), "{message}");
}

#[test]
fn a_non_utf8_opaque_token_is_refused_by_position() {
    let argv = with_invalid(&["claudine", "compose", "file.md", "-c", "x", "--", "a"], 7);
    let err = partition_composition_tail(argv).expect_err("lossy conversion must be refused");
    assert_eq!(err, PartitionError::NonUtf8Argument { position: 5 });
}

#[test]
fn a_non_utf8_token_before_the_file_is_left_for_clap() {
    // As the file it is a Claudine positional; clap reports it, so the
    // partition passes it through untouched.
    let argv = with_invalid(&["claudine", "compose"], 2);
    let (claudine, after) = partition_composition_tail(argv).expect("not an argument after the file");
    assert_eq!(claudine[2], invalid_token());
    assert!(after.arguments().is_empty());
}
