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

// ── Setter ownership after a provider switch, through the real catalog ──
//
// The rows that need the partition (a Claudine option standing between
// provider tokens) or the compiled research. Each asserts both the Claudine
// tokens and the exact forwarded tokens; the controlled-catalog table is in
// the library's ownership tests.

fn claude() -> Vec<OwnershipCandidate> {
    vec![OwnershipCandidate {
        provider: Provider::Claude,
        command_path: Vec::new(),
    }]
}

/// Partition behind `claudine compose plan.md`, then own for `candidates`:
/// (clap argv after `plan.md`, owned Claudine tokens, forwarded tokens).
fn owned_after_file(
    tokens: &[&str],
    schema: &SchemaParameters,
    candidates: &[OwnershipCandidate],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut line = vec!["claudine", "compose", "plan.md"];
    line.extend_from_slice(tokens);
    let (claudine, after) = partition_composition_tail(argv(&line)).expect("no partition error");
    let owned = own_arguments(&after, schema, candidates).unwrap_or_else(|err| panic!("{tokens:?} should launch: {err}"));
    let clap = claudine
        .into_iter()
        .skip(3)
        .map(|t| t.to_string_lossy().into_owned())
        .collect();
    (clap, owned.claudine, owned.tail.launch_args().to_vec())
}

struct RoutedRow {
    after_file: &'static [&'static str],
    declares_phase: bool,
    candidates: fn() -> Vec<OwnershipCandidate>,
    clap: &'static [&'static str],
    owned: &'static [&'static str],
    forwarded: &'static [&'static str],
}

const ROUTED_ROWS: &[RoutedRow] = &[
    RoutedRow {
        after_file: &["--provider", "codex", "--yolo", "phase=2"],
        declares_phase: false,
        candidates: codex,
        clap: &["--provider", "codex", "--yolo"],
        owned: &["phase=2"],
        forwarded: &[],
    },
    RoutedRow {
        after_file: &["--provider", "codex", "--config=x=y", "phase=2"],
        declares_phase: false,
        candidates: codex,
        clap: &["--provider", "codex"],
        owned: &["phase=2"],
        forwarded: &["--config=x=y"],
    },
    RoutedRow {
        after_file: &["--provider", "codex", "-c", "x=y", "-m", "gpt5", "phase=2"],
        declares_phase: false,
        candidates: codex,
        clap: &["--provider", "codex", "-m", "gpt5"],
        owned: &["phase=2"],
        forwarded: &["-c", "x=y"],
    },
    RoutedRow {
        after_file: &["--provider", "codex", "--frobnicate", "phase=2"],
        declares_phase: false,
        candidates: codex,
        clap: &["--provider", "codex"],
        owned: &["phase=2"],
        forwarded: &["--frobnicate"],
    },
    RoutedRow {
        after_file: &["--provider", "claude", "--add-dir", "a", "b", "phase=2"],
        declares_phase: false,
        candidates: claude,
        clap: &["--provider", "claude"],
        owned: &["phase=2"],
        forwarded: &["--add-dir", "a", "b"],
    },
    RoutedRow {
        after_file: &["--provider", "claude", "--add-dir", "x=y", "phase=2"],
        declares_phase: false,
        candidates: claude,
        clap: &["--provider", "claude"],
        owned: &["phase=2"],
        forwarded: &["--add-dir", "x=y"],
    },
    // A Claudine option ends a variadic run; `b` cannot reconnect to it.
    RoutedRow {
        after_file: &["--provider", "claude", "--add-dir", "a", "-m", "gpt5", "b"],
        declares_phase: false,
        candidates: claude,
        clap: &["--provider", "claude", "-m", "gpt5"],
        owned: &["b"],
        forwarded: &["--add-dir", "a"],
    },
    // The authored `--` is the provider-data escape hatch, even for a
    // declared parameter.
    RoutedRow {
        after_file: &["--provider", "codex", "--", "-c", "x=y", "phase=2"],
        declares_phase: false,
        candidates: codex,
        clap: &["--provider", "codex"],
        owned: &[],
        forwarded: &["-c", "x=y", "phase=2"],
    },
    RoutedRow {
        after_file: &["--provider", "codex", "--", "-c", "x=y", "phase=2"],
        declares_phase: true,
        candidates: codex,
        clap: &["--provider", "codex"],
        owned: &[],
        forwarded: &["-c", "x=y", "phase=2"],
    },
];

#[test]
fn setters_after_provider_switches_are_claudines_for_every_routed_row() {
    for row in ROUTED_ROWS {
        let schema = if row.declares_phase {
            SchemaParameters::from_names(["phase"])
        } else {
            SchemaParameters::NoSchema
        };
        let actual = owned_after_file(row.after_file, &schema, &(row.candidates)());
        assert_eq!(
            actual,
            (strs(row.clap), strs(row.owned), strs(row.forwarded)),
            "{:?}",
            row.after_file
        );
    }
}

/// The "takes no value" row uses a provider-only switch chosen from the
/// generated metadata: Codex `--ephemeral` at `exec`.
#[test]
fn a_researched_no_value_switch_never_takes_the_setter_after_it() {
    let lookup = claudine::provider::lookup_switch(Provider::Codex, &["exec"], "--ephemeral");
    assert_eq!(
        lookup.value(),
        claudine::provider::SwitchValue::None,
        "the fixture needs a Codex `exec` switch researched as taking no value; pick another if the research changed"
    );
    let actual = owned_after_file(&["--provider", "codex", "--ephemeral", "phase=2"], &SchemaParameters::NoSchema, &codex());
    assert_eq!(actual, (strs(&["--provider", "codex"]), strs(&["phase=2"]), strs(&["--ephemeral"])));
}

/// With no provider named and no `agent`, every provider is a candidate:
/// Codex reads `-c` as a string, so `x=y` is forwarded and `phase=2` applied;
/// a launch resolved to Claude, whose `-c` takes nothing, is refused.
#[test]
fn the_no_hint_union_forwards_a_value_the_resolved_provider_may_refuse() {
    let every: Vec<OwnershipCandidate> = claudine::provider::PROVIDERS_DISPLAY_ORDER
        .iter()
        .map(|provider| OwnershipCandidate {
            provider: *provider,
            command_path: Vec::new(),
        })
        .collect();
    let (_, after) = partition_composition_tail(argv(&["claudine", "compose", "plan.md", "-c", "x=y", "phase=2"])).unwrap();
    let owned = own_arguments(&after, &SchemaParameters::NoSchema, &every).unwrap();
    assert_eq!(owned.claudine, strs(&["phase=2"]));
    assert_eq!(owned.tail.launch_args(), strs(&["-c", "x=y"]));
    let mismatch = claudine::composition::check_launch_tail(&owned.tail, Provider::Claude, &[]).unwrap_err();
    assert_eq!((mismatch.switch.as_str(), mismatch.value.as_deref()), ("-c", Some("x=y")));
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

// Shared with the completion matrix, which reads the rest of it.
#[path = "../../../tests/common/owned_value_options.rs"]
#[allow(dead_code)]
mod owned_value_options;

/// The completion matrix enumerates these spellings, so it covers every
/// value slot exactly when they match the clap-derived surface.
#[test]
fn the_completion_matrix_lists_every_owned_value_option() {
    let listed: HashSet<String> = owned_value_options::OWNED_VALUE_OPTIONS
        .iter()
        .flat_map(|option| option.spellings.iter().map(|spelling| spelling.to_string()))
        .collect();
    assert_eq!(listed, OwnedFlags::for_composition().value_flags);
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
