//! Setter ownership after a provider switch: every row of the setter
//! ownership table, over a controlled switch catalog so a row does not move
//! when the research does, and the authored-schema sources that protect a
//! declared parameter directly after a switch.
//!
//! Every owned row asserts both Claudine's tokens and the exact forwarded
//! tokens.

use std::fs;
use std::path::Path;

use biscuit_file::FileResolutionContext;
use tempfile::TempDir;

use super::super::*;
use crate::composition::resolve::resolve_composition_source_in_context;
use crate::composition::schema::authored_schema_parameters;
use crate::provider::{CliSwitch, CliSwitchCatalog, SwitchScope, match_token_in};

// ── A controlled catalog ──
//
// One catalog, scoped by command path the way the research is: at the root
// (where a Claude launch runs) `-c` takes no value and `--add-dir` a list; at
// `exec` (where a Codex launch runs) `-c` takes one string and `--quiet` is a
// provider-only switch that takes no value. A candidate's command path picks
// its reading, so a union of root and `exec` candidates disagrees exactly
// where Claude and Codex do today.

const ROOT: &[&str] = &[];
const EXEC: &[&str] = &["exec"];

static CONTROLLED: &[CliSwitch] = &[
    CliSwitch {
        flag: "--continue",
        aliases: &["-c"],
        value: SwitchValue::None,
        attachments: &[],
        scopes: &[SwitchScope::Command(ROOT)],
        description: "Takes no value.",
        gap: None,
    },
    CliSwitch {
        flag: "--add-dir",
        aliases: &[],
        value: SwitchValue::Variadic { min: VariadicMin::AtLeast(1) },
        attachments: &[SwitchAttachment::Space],
        scopes: &[SwitchScope::Command(ROOT)],
        description: "A list of values.",
        gap: None,
    },
    CliSwitch {
        flag: "--config",
        aliases: &["-c"],
        value: SwitchValue::String { optional: false },
        attachments: &[SwitchAttachment::Space, SwitchAttachment::Equals],
        scopes: &[SwitchScope::Command(EXEC)],
        description: "One string value.",
        gap: None,
    },
    CliSwitch {
        flag: "--quiet",
        aliases: &[],
        value: SwitchValue::None,
        attachments: &[],
        scopes: &[SwitchScope::Command(EXEC)],
        description: "Takes no value.",
        gap: None,
    },
];

struct Controlled;

impl SwitchSource for Controlled {
    fn matched<'t>(&self, _: Provider, path: &[&str], token: &'t str) -> Option<SwitchToken<'t>> {
        match_token_in(CliSwitchCatalog::Researched(CONTROLLED), path, token)
    }
}

fn s(tokens: &[&str]) -> Vec<String> {
    tokens.iter().map(|token| token.to_string()).collect()
}

/// Tokens after the file, with `|` for a removed Claudine option (`--yolo`,
/// `-m gpt5`), and the tokens after an authored `--`.
fn after_file(tokens: &[&str], opaque: Option<&[&str]>) -> ArgumentsAfterFile {
    let arguments = tokens
        .iter()
        .map(|token| match *token {
            "|" => CallerArgument::ClaudineOption,
            other => CallerArgument::Token(other.to_string()),
        })
        .collect();
    ArgumentsAfterFile::new(arguments, opaque.map(s))
}

#[derive(Clone, Copy, Debug)]
enum Candidates {
    /// `--codex`.
    Codex,
    /// `--claude`.
    Claude,
    /// No provider named and no authored `agent`: every provider.
    Every,
}

impl Candidates {
    /// Each provider reads the controlled catalog at its command path; with
    /// no hint every provider is a candidate at both readings.
    fn list(self) -> Vec<OwnershipCandidate> {
        let at = |provider, path: &[&str]| OwnershipCandidate { provider, command_path: s(path) };
        match self {
            Self::Codex => vec![at(Provider::Codex, EXEC)],
            Self::Claude => vec![at(Provider::Claude, ROOT)],
            Self::Every => crate::provider::PROVIDERS_DISPLAY_ORDER
                .iter()
                .flat_map(|provider| [at(*provider, ROOT), at(*provider, EXEC)])
                .collect(),
        }
    }
}

fn phase_declared() -> SchemaParameters {
    SchemaParameters::from_names(["phase"])
}

fn own_controlled(
    tokens: &[&str],
    opaque: Option<&[&str]>,
    schema: &SchemaParameters,
    candidates: Candidates,
) -> Result<OwnedArguments, OwnershipError> {
    own_in(&Controlled, &after_file(tokens, opaque), schema, &candidates.list()).map(|(owned, _)| owned)
}

struct OwnedRow {
    /// The command line after `compose plan.md`, for failure messages.
    input: &'static str,
    tokens: &'static [&'static str],
    opaque: Option<&'static [&'static str]>,
    declares_phase: bool,
    candidates: Candidates,
    claudine: &'static [&'static str],
    forwarded: &'static [&'static str],
}

/// The setter ownership table's rows that launch, plus the acceptance
/// extras. No row's schema claims `x`.
const OWNED_ROWS: &[OwnedRow] = &[
    // Control: a switch's own setter-shaped value is forwarded unchanged.
    OwnedRow {
        input: "--codex -c model_reasoning_effort=low",
        tokens: &["-c", "model_reasoning_effort=low"],
        opaque: None,
        declares_phase: true,
        candidates: Candidates::Codex,
        claudine: &[],
        forwarded: &["-c", "model_reasoning_effort=low"],
    },
    OwnedRow {
        input: "--codex -c x=y phase=2",
        tokens: &["-c", "x=y", "phase=2"],
        opaque: None,
        declares_phase: false,
        candidates: Candidates::Codex,
        claudine: &["phase=2"],
        forwarded: &["-c", "x=y"],
    },
    OwnedRow {
        input: "-c x=y phase=2 (no provider hint)",
        tokens: &["-c", "x=y", "phase=2"],
        opaque: None,
        declares_phase: false,
        candidates: Candidates::Every,
        claudine: &["phase=2"],
        forwarded: &["-c", "x=y"],
    },
    OwnedRow {
        input: "--codex --yolo phase=2",
        tokens: &["|", "phase=2"],
        opaque: None,
        declares_phase: false,
        candidates: Candidates::Codex,
        claudine: &["phase=2"],
        forwarded: &[],
    },
    OwnedRow {
        input: "--codex --quiet phase=2 (a provider switch that takes no value)",
        tokens: &["--quiet", "phase=2"],
        opaque: None,
        declares_phase: false,
        candidates: Candidates::Codex,
        claudine: &["phase=2"],
        forwarded: &["--quiet"],
    },
    OwnedRow {
        input: "--claude --add-dir a b phase=2",
        tokens: &["--add-dir", "a", "b", "phase=2"],
        opaque: None,
        declares_phase: false,
        candidates: Candidates::Claude,
        claudine: &["phase=2"],
        forwarded: &["--add-dir", "a", "b"],
    },
    OwnedRow {
        input: "--claude --add-dir x=y phase=2",
        tokens: &["--add-dir", "x=y", "phase=2"],
        opaque: None,
        declares_phase: false,
        candidates: Candidates::Claude,
        claudine: &["phase=2"],
        forwarded: &["--add-dir", "x=y"],
    },
    OwnedRow {
        input: "--codex --frobnicate phase=2 (an unrecognized switch)",
        tokens: &["--frobnicate", "phase=2"],
        opaque: None,
        declares_phase: false,
        candidates: Candidates::Codex,
        claudine: &["phase=2"],
        forwarded: &["--frobnicate"],
    },
    OwnedRow {
        input: "--codex --config=x=y phase=2",
        tokens: &["--config=x=y", "phase=2"],
        opaque: None,
        declares_phase: false,
        candidates: Candidates::Codex,
        claudine: &["phase=2"],
        forwarded: &["--config=x=y"],
    },
    OwnedRow {
        input: "--codex -c x=y -m gpt5 phase=2",
        tokens: &["-c", "x=y", "|", "phase=2"],
        opaque: None,
        declares_phase: false,
        candidates: Candidates::Codex,
        claudine: &["phase=2"],
        forwarded: &["-c", "x=y"],
    },
    OwnedRow {
        input: "--claude --add-dir a -m gpt5 b",
        tokens: &["--add-dir", "a", "|", "b"],
        opaque: None,
        declares_phase: false,
        candidates: Candidates::Claude,
        claudine: &["b"],
        forwarded: &["--add-dir", "a"],
    },
    OwnedRow {
        input: "--codex -- -c x=y phase=2",
        tokens: &[],
        opaque: Some(&["-c", "x=y", "phase=2"]),
        declares_phase: false,
        candidates: Candidates::Codex,
        claudine: &[],
        forwarded: &["-c", "x=y", "phase=2"],
    },
    OwnedRow {
        input: "--codex -- -c x=y phase=2 (phase declared)",
        tokens: &[],
        opaque: Some(&["-c", "x=y", "phase=2"]),
        declares_phase: true,
        candidates: Candidates::Codex,
        claudine: &[],
        forwarded: &["-c", "x=y", "phase=2"],
    },
    // The opaque part is never checked, even when it ends with a switch
    // that takes a value.
    OwnedRow {
        input: "--codex -- -c",
        tokens: &[],
        opaque: Some(&["-c"]),
        declares_phase: true,
        candidates: Candidates::Codex,
        claudine: &[],
        forwarded: &["-c"],
    },
];

#[test]
fn every_setter_ownership_row_keeps_setters_and_forwards_exact_tokens() {
    for row in OWNED_ROWS {
        let schema = if row.declares_phase { phase_declared() } else { SchemaParameters::NoSchema };
        let owned = own_controlled(row.tokens, row.opaque, &schema, row.candidates)
            .unwrap_or_else(|err| panic!("`{}` should launch: {err}", row.input));
        assert_eq!(owned.claudine, s(row.claudine), "Claudine tokens for `{}`", row.input);
        assert_eq!(owned.tail.launch_args(), s(row.forwarded), "forwarded tokens for `{}`", row.input);
    }
}

/// The no-hint row forwards `-c x=y` because one candidate takes a string;
/// a launch resolved to Claude, whose `-c` takes nothing, fails before its
/// spawn and never reroutes `x=y` into frontmatter.
#[test]
fn a_union_forwarded_value_fails_for_a_resolved_provider_that_takes_none() {
    let owned = own_controlled(&["-c", "x=y", "phase=2"], None, &SchemaParameters::NoSchema, Candidates::Every).unwrap();
    assert_eq!(owned.claudine, s(&["phase=2"]), "`x=y` stays out of the setters");
    assert_eq!(owned.tail.launch_args(), s(&["-c", "x=y"]));

    assert_eq!(check_in(&Controlled, &owned.tail, Provider::Codex, EXEC), Ok(()));
    let mismatch = check_in(&Controlled, &owned.tail, Provider::Claude, ROOT).unwrap_err();
    assert_eq!(
        (mismatch.provider, mismatch.switch.as_str(), mismatch.kind, mismatch.value.as_deref()),
        (Provider::Claude, "-c", MismatchKind::ExtraValue, Some("x=y"))
    );
    let message = mismatch.to_string();
    assert!(message.contains("`-c`") && message.contains("Claude") && message.contains("`x=y`"), "{message}");
}

/// A declared parameter directly after a string switch stays Claudine's and
/// leaves the switch without a value; a later setter never becomes that
/// value (adjacency survives classification).
#[test]
fn a_declared_setter_after_a_string_switch_leaves_it_without_a_value() {
    for (input, tokens) in [
        ("--codex -c phase=2", &["-c", "phase=2"][..]),
        ("--codex -c phase=2 x=y", &["-c", "phase=2", "x=y"]),
    ] {
        let err = own_controlled(tokens, None, &phase_declared(), Candidates::Codex)
            .err()
            .unwrap_or_else(|| panic!("`{input}` must not launch"));
        let OwnershipError::Mismatch(mismatch) = &err else { panic!("`{input}`: {err:?}") };
        assert_eq!(
            (mismatch.provider, mismatch.switch.as_str(), mismatch.kind, mismatch.value.as_deref()),
            (Provider::Codex, "-c", MismatchKind::MissingValue, None),
            "`{input}`"
        );
    }
}

/// The missing-value error for a declared setter directly after a string
/// switch names the switch, the conflicting setter key, and the provider,
/// and guides toward a separate provider value or `--`, without echoing the
/// setter's value.
#[test]
fn the_missing_value_error_names_the_conflicting_setter() {
    for (input, tokens) in [
        ("--codex -c phase=2", &["-c", "phase=2"][..]),
        ("--codex -c phase=2 x=y", &["-c", "phase=2", "x=y"]),
    ] {
        let err = own_controlled(tokens, None, &phase_declared(), Candidates::Codex)
            .err()
            .unwrap_or_else(|| panic!("`{input}` must not launch"));
        assert_names_the_conflicting_setter(input, &err.to_string());
    }
    for (source, err) in schema_source_failures() {
        assert_names_the_conflicting_setter(source, &err.to_string());
    }
}

/// A union passes ownership because Claude's `-c` takes nothing, yet the
/// declared setter is recorded on the assignment, so the check for a launch
/// resolved to Codex names it too. A missing value with no setter directly
/// after the switch (at the end, or cut off by a Claudine option) names none
/// and keeps the general guidance.
#[test]
fn only_a_setter_directly_after_the_switch_is_named_by_the_launch_check() {
    for tokens in [&["-c", "phase=2"][..], &["-c", "phase=2", "x=y"]] {
        let owned = own_controlled(tokens, None, &phase_declared(), Candidates::Every)
            .unwrap_or_else(|err| panic!("{tokens:?}: Claude's `-c` takes nothing, so the union passes: {err}"));
        assert_eq!(owned.tail.launch_args(), s(&["-c"]), "{tokens:?}");
        assert_eq!(owned.tail.assignments()[0].declared_setter.as_deref(), Some("phase"), "{tokens:?}");
        assert_eq!(check_in(&Controlled, &owned.tail, Provider::Claude, ROOT), Ok(()));
        let mismatch = check_in(&Controlled, &owned.tail, Provider::Codex, EXEC).unwrap_err();
        assert_eq!(
            (mismatch.kind, mismatch.declared_setter.as_deref()),
            (MismatchKind::MissingValue, Some("phase")),
            "{tokens:?}"
        );
        assert_names_the_conflicting_setter(&format!("{tokens:?} resolved to Codex"), &mismatch.to_string());
    }

    for (input, tokens) in [
        ("--codex -c", &["-c"][..]),
        ("--codex -c -m gpt5 phase=2", &["-c", "|", "phase=2"]),
    ] {
        let err = own_controlled(tokens, None, &phase_declared(), Candidates::Codex)
            .err()
            .unwrap_or_else(|| panic!("`{input}` must not launch"));
        let OwnershipError::Mismatch(mismatch) = &err else { panic!("`{input}`: {err:?}") };
        assert_eq!((mismatch.kind, mismatch.declared_setter.as_deref()), (MismatchKind::MissingValue, None), "`{input}`");
        let message = err.to_string();
        assert!(message.contains("`-c` takes a value for Codex"), "`{input}`: {message}");
        assert!(!message.contains("`phase`") && !message.contains("declares"), "`{input}`: {message}");
        assert!(message.contains("`--`"), "`{input}`: {message}");
    }
}

fn assert_names_the_conflicting_setter(input: &str, message: &str) {
    assert!(message.contains("`-c`"), "`{input}` names the switch: {message}");
    assert!(message.contains("Codex"), "`{input}` names the provider: {message}");
    assert!(message.contains("`phase`"), "`{input}` names the setter key: {message}");
    assert!(message.contains("separate"), "`{input}` suggests a separate provider value: {message}");
    assert!(message.contains("`--`"), "`{input}` suggests `--`: {message}");
    assert!(!message.contains("phase=2") && !message.contains("x=y"), "`{input}` echoes no token: {message}");
}

// ── Authored-schema sources ──
//
// Each source declares `phase`, read through the same loader ownership uses,
// so a declared `phase` directly after `-c` stays Claudine's and leaves `-c`
// without a value.

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// The authored schema parameters of `docs/plan.md` in `root`, resolved with
/// `root` as the launch directory, so a schema that resolved against the
/// launch directory instead of the document's would read the decoy.
fn schema_of(root: &Path, document: &str) -> Result<SchemaParameters, crate::composition::CompositionError> {
    let file = root.join("docs").join("plan.md");
    write(&file, document);
    let context = FileResolutionContext::new(root);
    let source = resolve_composition_source_in_context(file.to_str().unwrap(), &context).unwrap();
    authored_schema_parameters(&source, Some(root), Some(&context))
}

/// One temp directory per schema source, each with a decoy schema of the
/// same name at the launch directory that declares `decoy` and not `phase`.
fn schema_sources() -> Vec<(&'static str, TempDir, &'static str)> {
    let fixture = |files: &[(&str, &str)]| {
        let root = tempfile::tempdir().unwrap();
        for (name, content) in files {
            write(&root.path().join(name), content);
        }
        for decoy in ["a.schema.yaml", "b.schema.yaml", "plan.schema.json"] {
            write(&root.path().join(decoy), "$schema:\n  decoy: string\n");
        }
        root
    };
    vec![
        ("inline $schema", fixture(&[]), "---\n$schema:\n  phase: string\n---\nPhase {{ phase }}\n"),
        (
            "a root union declaring `phase` in its second arm",
            fixture(&[
                ("docs/a.schema.yaml", "$schema:\n  title: string\n"),
                ("docs/b.schema.yaml", "$schema:\n  phase: string\n"),
            ]),
            "---\n$schema: [./a.schema.yaml, ./b.schema.yaml]\ntitle: t\n---\nBody\n",
        ),
        (
            "a raw JSON Schema",
            fixture(&[(
                "docs/plan.schema.json",
                r#"{"type":"object","properties":{"phase":{"type":"string"}}}"#,
            )]),
            "---\n$schema: ./plan.schema.json\n---\nBody\n",
        ),
        (
            "a source-relative external schema",
            fixture(&[("docs/a.schema.yaml", "$schema:\n  phase: string\n")]),
            "---\n$schema: ./a.schema.yaml\n---\nBody\n",
        ),
    ]
}

/// `--codex -c phase=2` against each schema source: the ownership error.
fn schema_source_failures() -> Vec<(&'static str, OwnershipError)> {
    schema_sources()
        .into_iter()
        .map(|(source, root, document)| {
            let schema = schema_of(root.path(), document).unwrap_or_else(|err| panic!("{source}: {err}"));
            let declares = |key| schema.declares(key);
            assert_eq!((declares("phase"), declares("decoy")), (Some(true), Some(false)), "{source}");
            let err = own_controlled(&["-c", "phase=2"], None, &schema, Candidates::Codex)
                .err()
                .unwrap_or_else(|| panic!("{source}: `-c phase=2` must not launch"));
            (source, err)
        })
        .collect()
}

#[test]
fn every_schema_source_protects_a_declared_parameter_after_a_switch() {
    for (source, err) in schema_source_failures() {
        let OwnershipError::Mismatch(mismatch) = &err else { panic!("{source}: {err:?}") };
        assert_eq!(
            (mismatch.provider, mismatch.switch.as_str(), mismatch.kind),
            (Provider::Codex, "-c", MismatchKind::MissingValue),
            "{source}"
        );
    }
    // A key no source declares is still the switch's value.
    for (source, root, document) in schema_sources() {
        let schema = schema_of(root.path(), document).unwrap();
        let owned = own_controlled(&["-c", "x=y", "phase=2"], None, &schema, Candidates::Codex)
            .unwrap_or_else(|err| panic!("{source}: {err}"));
        assert_eq!(
            (owned.claudine, owned.tail.launch_args().to_vec()),
            (s(&["phase=2"]), s(&["-c", "x=y"])),
            "{source}"
        );
    }
}

/// A templated `$schema` cannot say whether `phase` is a parameter: the
/// contested setter is an error with `--`/`--set` guidance, never silently
/// provider data. An unreadable `$schema` is an error before ownership.
#[test]
fn an_unestablished_schema_never_routes_a_contested_setter_silently() {
    let root = tempfile::tempdir().unwrap();
    let schema = schema_of(root.path(), "---\n$schema: \"{{ env.SETTER_SCHEMA }}\"\n---\nBody\n").unwrap();
    assert!(matches!(schema, SchemaParameters::Unestablished), "{schema:?}");
    let err = own_controlled(&["-c", "phase=2"], None, &schema, Candidates::Codex).unwrap_err();
    assert_eq!(err, OwnershipError::ContestedSetter { switch: "-c".to_string(), key: "phase".to_string() });
    let message = err.to_string();
    assert!(message.contains("`--`") && message.contains("--set"), "{message}");
    assert!(!message.contains("phase=2"), "{message}");

    // Where no switch would take the setter, the unknown names do not matter.
    let owned = own_controlled(&["-c", "x", "phase=2"], None, &schema, Candidates::Codex).unwrap();
    assert_eq!((owned.claudine, owned.tail.launch_args().to_vec()), (s(&["phase=2"]), s(&["-c", "x"])));

    let root = tempfile::tempdir().unwrap();
    let err = schema_of(root.path(), "---\n$schema: ./missing.schema.yaml\n---\nBody\n").unwrap_err();
    assert!(err.to_string().contains("missing.schema.yaml"), "{err}");
}
