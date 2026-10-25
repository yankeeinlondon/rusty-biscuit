//! DMLS types a document's lifecycle stacks under its host: the lifecycle
//! artifact beside a host catalog in the document's schema roots
//! (`claudine/schemas/globals.lifecycle.json` beside `globals.yaml`) gives the
//! host's events, renames, per-event globals, and verbs, and the library's one
//! scoped walk (`darkmatter::lifecycle::lifecycle_call_checks`) narrows each
//! call by the `when` items around it, ending a fact at a write. Without a
//! usable artifact, the `md` host's lifecycle is analyzed and the document
//! says the analysis is incomplete.
//!
//! Every row runs a real stdio session with `[expressions] strict = true`,
//! where a call on a possibly-null `count` is a `dm.expression.uncertain_call`
//! warning: a guarded call is silent and a call after a write warns. Each
//! shipped-artifact row is also compared with the library walk under the
//! profile the committed artifact rebuilds, which Claudine's
//! `host_catalog::lifecycle_artifact` tests prove equal to composition's.

use std::path::{Path, PathBuf};

use darkmatter::lifecycle::host_profile::HostLifecycleProfile;
use darkmatter::lifecycle::md_host;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::host::FactsArtifact;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::catalog::{GlobalCatalog, GlobalRequirement};
use serde_json::{Value, json};

use crate::claudine_schema_discovery::{Fixture, initialize_params, open, uri_of, write};
use crate::common::{LspFixture, LspWorkspace};

const GLOBALS: &str = include_str!("../../../../claudine/schemas/globals.yaml");
const ARTIFACT: &str = include_str!("../../../../claudine/schemas/globals.lifecycle.json");
const CALL_CODES: [&str; 2] = ["dm.expression.uncertain_call", "dm.expression.incompatible_call"];
const INCOMPLETE: &str = "dm.lifecycle.incomplete_analysis";
const STRICT: &str = "[expressions]\nstrict = true\n";

/// What sits beside the host catalog.
#[derive(Clone, Copy, Debug)]
enum Artifact {
    /// The committed artifact.
    Shipped,
    /// No artifact.
    Absent,
    /// The committed artifact beside an edited catalog.
    Stale,
    /// The committed artifact without its `success` event.
    LackingEvent,
}

/// The host a row's documents are opened under.
#[derive(Clone, Copy, Debug)]
enum Host {
    /// Claudine's schema bundle and host catalog in the package area, with
    /// `artifact` beside the catalog.
    Claudine(Artifact),
    /// A repository with no host catalog.
    Md,
}

/// One document and what the editor must show for it.
struct Case {
    text: String,
    /// A needle on each line whose call warns.
    warns: Vec<&'static str>,
    /// Whether the library walk under the host must agree (false when the
    /// host's parser refuses the document).
    parity: bool,
    /// Whether no `dm.expression.*` diagnostic at all may appear.
    quiet: bool,
}

fn case(text: String, warns: &[&'static str]) -> Case {
    Case { text, warns: warns.to_vec(), parity: true, quiet: false }
}

const HEAD: &str = "---\n$schema:\n  count: number\n  flag: boolean\ncount: 1\nflag: true\n";

/// Guard → call → set → call in `event`, with `verb` writing the calls.
fn guard_write(event: &str, verb: &str) -> String {
    format!(
        "{HEAD}{event}:\n  - when: count\n    then:\n      - {verb}: \"{{{{ min(count, 5) }}}}\"\n      - set:\n          count: 7\n      - {verb}: \"{{{{ min(count, 6) }}}}\"\n---\nBody\n"
    )
}

fn guard_write_cases(events: &[&str], verbs: &[&str]) -> Vec<Case> {
    events
        .iter()
        .flat_map(|event| verbs.iter().map(move |verb| case(guard_write(event, verb), &["min(count, 6)"])))
        .collect()
}

/// A host-only verb whose operand holds a call: silent under the guard, and
/// warning in the unguarded control item after it.
fn operand(event: &str, verb: &str, guarded: &str, control: &str) -> Case {
    case(
        format!("{HEAD}{event}:\n  - when: count\n    then:\n      - {verb}: {guarded}\n  - {verb}: {control}\n---\nBody\n"),
        &["min(count, 6)"],
    )
}

fn operand_cases(rows_in: std::ops::Range<usize>) -> Vec<Case> {
    let rows: [(&str, &str, &str); 9] = [
        ("start", "speak", "\"{{ min(count, N) }}\""),
        ("start", "effect", "\"{{ min(count, N) }}\""),
        ("start", "notify", "\"{{ min(count, N) }}\""),
        ("failure", "retry", "{ with: { x: \"{{ min(count, N) }}\" } }"),
        ("failure", "resume", "\"{{ min(count, N) }}\""),
        ("failure", "proxy", "{ target: \"@next.md\", with: { x: \"{{ min(count, N) }}\" } }"),
        ("success", "prep", "{ target: \"@next.md\", with: { x: \"{{ min(count, N) }}\" } }"),
        ("success", "break", "{ reason: \"{{ min(count, N) }}\" }"),
        ("success", "defer", "{ delay: 1s, reason: \"{{ min(count, N) }}\" }"),
    ];
    rows[rows_in]
        .iter()
        .map(|(event, verb, template)| operand(event, verb, &template.replace('N', "5"), &template.replace('N', "6")))
        .collect()
}

macro_rules! rows {
    ($($(#[$doc:meta])* $name:ident: $host:expr => $cases:expr;)+) => {
        $(
            $(#[$doc])*
            #[test]
            fn $name() {
                check($host, $cases);
            }
        )+
    };
}

rows! {
    /// R1-F1.1: Claudine's own `start`, `success`, `blocked`, `failure`,
    /// written with `stdout`.
    claudine_events_narrow_until_a_write_with_stdout: Host::Claudine(Artifact::Shipped) =>
        guard_write_cases(&["start", "success", "blocked", "failure"], &["stdout"]);
    /// R1-F1.1, written with Claudine's `say`.
    claudine_events_narrow_until_a_write_with_say: Host::Claudine(Artifact::Shipped) =>
        guard_write_cases(&["start", "success", "blocked", "failure"], &["say"]);

    /// R1-F1.2: the shared events with Claudine's `say`.
    shared_events_with_say_narrow_until_a_write: Host::Claudine(Artifact::Shipped) =>
        guard_write_cases(&["initialize", "cancelled", "finalize"], &["say"]);
    /// R1-F1.2: Darkmatter's `composed`/`failed` spellings are not Claudine
    /// stacks. Composition refuses the document (`LifecycleRenamedEvent`)
    /// before anything runs, so no narrowing applies: the values are analyzed
    /// as plain frontmatter and both calls warn.
    renamed_events_are_not_claudine_stacks: Host::Claudine(Artifact::Shipped) => ["composed", "failed"]
        .into_iter()
        .map(|renamed| {
            Case { parity: false, ..case(guard_write(renamed, "say"), &["min(count, 5)", "min(count, 6)"]) }
        })
        .collect();

    /// Guards Claudine's schema types as expressions are typed at their own
    /// site too: a nested guard under `when: timing.document_ms` is silent,
    /// and the same guard outside it warns (`timing` is typed by
    /// Darkmatter's global catalog; a document with its own `$schema` is not
    /// typed by Claudine's trigger, so this one has none).
    schema_typed_guards_take_the_guarded_scope: Host::Claudine(Artifact::Shipped) => vec![case(
        "---\nsuccess:\n  - when: timing.document_ms\n    then:\n      - when: min(timing.document_ms, 5) > 1\n        then:\n          - stdout: \"{{ min(timing.document_ms, 6) }}\"\n  - when: min(timing.document_ms, 7) > 1\n    then:\n      - stdout: done\n---\nBody\n".to_string(),
        &["min(timing.document_ms, 7)"],
    )];

    /// R1-F1.3: Claudine's `loop` gate, beside a valid `until`.
    loop_gate_narrows_until_a_write: Host::Claudine(Artifact::Shipped) => vec![case(
        format!("{HEAD}loop:\n  max: 3\n  until: loop.count > 2\n  gate:\n    - when: count\n      then:\n        - stdout: \"{{{{ min(count, 5) }}}}\"\n        - set:\n            count: 7\n        - stdout: \"{{{{ min(count, 6) }}}}\"\n---\nBody\n"),
        &["min(count, 6)"],
    )];

    /// R1-F1.4: the false branch of `is_null(count)` proves `count` present.
    false_branch_narrows_until_a_write: Host::Claudine(Artifact::Shipped) => vec![case(
        format!("{HEAD}success:\n  - when: is_null(count)\n    then:\n      - stdout: none\n    else:\n      - stdout: \"{{{{ min(count, 5) }}}}\"\n      - set:\n          count: 7\n      - stdout: \"{{{{ min(count, 6) }}}}\"\n---\nBody\n"),
        &["min(count, 6)"],
    )];

    /// R1-F1.5: a write inside a nested conditional ends the fact for the
    /// later call.
    nested_write_ends_the_fact: Host::Claudine(Artifact::Shipped) => vec![case(
        format!("{HEAD}success:\n  - when: count\n    then:\n      - stdout: \"{{{{ min(count, 5) }}}}\"\n      - when: flag\n        then:\n          - set:\n              count: 7\n      - stdout: \"{{{{ min(count, 6) }}}}\"\n---\nBody\n"),
        &["min(count, 6)"],
    )];

    /// R1-F1.6: without a host catalog, the `md` host's five events.
    md_events_narrow_until_a_write: Host::Md =>
        guard_write_cases(&["initialize", "composed", "failed", "cancelled", "finalize"], &["stdout"]);

    /// R1-F1.7: `err` is available in Claudine's `blocked` and `failure` and
    /// in `md`'s `failed`; reading it is no expression diagnostic.
    claudine_error_events_read_err: Host::Claudine(Artifact::Shipped) => ["blocked", "failure"]
        .into_iter()
        .map(|event| Case { quiet: true, ..case(format!("---\n{event}:\n  - stdout: \"{{{{ err.msg }}}}\"\n---\nBody\n"), &[]) })
        .collect();
    md_failed_reads_err: Host::Md =>
        vec![Case { quiet: true, ..case("---\nfailed:\n  - stdout: \"{{ err.msg }}\"\n---\nBody\n".to_string(), &[]) }];

    /// R1-F1.8: every other host-only verb's expression operand takes the
    /// guarded scope: the communication verbs.
    host_only_communication_operands_take_the_guarded_scope: Host::Claudine(Artifact::Shipped) => operand_cases(0..3);
    /// R1-F1.8: the recovery directives.
    host_only_recovery_operands_take_the_guarded_scope: Host::Claudine(Artifact::Shipped) => operand_cases(3..6);
    /// R1-F1.8: `prep`, `break`, and `defer`.
    host_only_flow_operands_take_the_guarded_scope: Host::Claudine(Artifact::Shipped) => operand_cases(6..9);

    /// Without the artifact, a Claudine event is plain frontmatter to the
    /// `md` host: nothing is narrowed, and the document says so.
    an_absent_artifact_falls_back_and_is_reported: Host::Claudine(Artifact::Absent) =>
        vec![Case { parity: false, ..case(guard_write("success", "stdout"), &["min(count, 5)", "min(count, 6)"]) }];
    a_stale_artifact_falls_back_and_is_reported: Host::Claudine(Artifact::Stale) =>
        vec![Case { parity: false, ..case(guard_write("success", "stdout"), &["min(count, 5)", "min(count, 6)"]) }];
    an_artifact_lacking_an_event_falls_back_and_is_reported: Host::Claudine(Artifact::LackingEvent) =>
        vec![Case { parity: false, ..case(guard_write("success", "stdout"), &["min(count, 5)", "min(count, 6)"]) }];
}

/// The reason text each unusable artifact is reported with.
fn incomplete_reason(artifact: Artifact) -> Option<&'static str> {
    match artifact {
        Artifact::Shipped => None,
        Artifact::Absent => Some("is absent"),
        Artifact::Stale => Some("regenerate the artifact"),
        Artifact::LackingEvent => Some("refuses"),
    }
}

/// The committed artifact with its `success` event removed.
fn lacking_success() -> String {
    let mut root: Value = serde_json::from_str(ARTIFACT).unwrap();
    let events = root["events"].as_array_mut().unwrap();
    let before = events.len();
    events.retain(|event| event["name"] != json!("success"));
    assert_eq!(events.len(), before - 1);
    serde_json::to_string_pretty(&root).unwrap()
}

fn check(host: Host, cases: Vec<Case>) {
    let workspace = LspWorkspace::new();
    let (mut session, documents) = match host {
        Host::Claudine(artifact) => {
            let fixture = Fixture::new(workspace.path());
            let schemas = fixture.repo().join("claudine/schemas");
            let catalog = match artifact {
                Artifact::Stale => GLOBALS.replace("The one-based iteration.", "Edited."),
                _ => GLOBALS.to_string(),
            };
            write(&schemas.join("globals.yaml"), &catalog);
            match artifact {
                Artifact::Absent => {}
                Artifact::LackingEvent => write(&schemas.join("globals.lifecycle.json"), &lacking_success()),
                Artifact::Shipped | Artifact::Stale => write(&schemas.join("globals.lifecycle.json"), ARTIFACT),
            }
            write(&fixture.repo().join(".dmls.toml"), STRICT);
            let mut session = LspFixture::start_with_snapshot(&workspace, fixture.snapshot());
            session.initialize(initialize_params(&fixture.repo()));
            let documents: Vec<PathBuf> = (0..cases.len())
                .map(|index| fixture.document(&format!("doc{index}.md"), &cases[index].text))
                .collect();
            (session, documents)
        }
        Host::Md => {
            let root = workspace.path();
            for dir in [".git/objects", ".git/refs/heads"] {
                std::fs::create_dir_all(root.join(dir)).unwrap();
            }
            write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
            write(&root.join(".dmls.toml"), STRICT);
            let mut session = LspFixture::start(&workspace);
            session.initialize(initialize_params(root));
            let documents: Vec<PathBuf> = (0..cases.len())
                .map(|index| {
                    let path = root.join(format!("doc{index}.md"));
                    write(&path, &cases[index].text);
                    path
                })
                .collect();
            (session, documents)
        }
    };
    for (case, path) in cases.iter().zip(&documents) {
        let uri = uri_of(path);
        open(&session, &uri, &case.text);
        let diagnostics = latest_diagnostics(&mut session, &uri);
        let label = format!("{host:?}\n{}", case.text);

        let expected: Vec<u64> = case.warns.iter().map(|needle| line_of(&case.text, needle)).collect();
        assert_eq!(call_lines(&diagnostics), expected, "{label}\n{diagnostics:#?}");
        if case.parity {
            assert_eq!(library_lines(host, &case.text), expected, "the library walk disagrees: {label}");
        } else if matches!(host, Host::Claudine(Artifact::Shipped)) {
            let error = darkmatter::lifecycle::parse_lifecycle_config_with(
                &frontmatter_of(&case.text),
                Path::new("doc.md"),
                &shipped_profile().parse_options(None),
            )
            .expect_err("Claudine refuses a Darkmatter spelling");
            assert!(error.to_string().contains("blocked") || error.to_string().contains("start"), "{error}");
        }
        if case.quiet {
            let expression: Vec<&Value> = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic["code"].as_str().is_some_and(|code| code.starts_with("dm.expression.")))
                .collect();
            assert!(expression.is_empty(), "{label}\n{expression:#?}");
        }
        let incomplete: Vec<&str> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic["code"] == json!(INCOMPLETE))
            .map(|diagnostic| diagnostic["message"].as_str().unwrap())
            .collect();
        match host {
            Host::Claudine(artifact) => match incomplete_reason(artifact) {
                None => assert!(incomplete.is_empty(), "{label}\n{incomplete:#?}"),
                Some(reason) => {
                    assert_eq!(incomplete.len(), 1, "{label}\n{diagnostics:#?}");
                    assert!(incomplete[0].contains("globals.lifecycle.json") && incomplete[0].contains(reason), "{}", incomplete[0]);
                }
            },
            Host::Md => assert!(incomplete.is_empty(), "{label}\n{incomplete:#?}"),
        }
    }
    session.shutdown();
}

/// The latest diagnostics published for `uri` once the server has handled
/// everything sent so far.
fn latest_diagnostics(session: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    session.flush_server();
    let mut latest = session.wait_for_diagnostics_params(uri);
    while let Some(params) = session.take_buffered_diagnostics(uri) {
        latest = params;
    }
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

fn line_of(text: &str, needle: &str) -> u64 {
    text.lines().position(|line| line.contains(needle)).unwrap_or_else(|| panic!("{needle} in {text}")) as u64
}

/// The sorted lines of every call diagnostic.
fn call_lines(diagnostics: &[Value]) -> Vec<u64> {
    let mut lines: Vec<u64> = diagnostics
        .iter()
        .filter(|diagnostic| CALL_CODES.iter().any(|code| diagnostic["code"] == json!(code)))
        .map(|diagnostic| diagnostic["range"]["start"]["line"].as_u64().unwrap())
        .collect();
    lines.sort_unstable();
    lines
}

/// The profile the committed artifact rebuilds for the shipped catalog.
fn shipped_profile() -> HostLifecycleProfile {
    let path = "claudine/schemas/globals.yaml";
    let catalog = GlobalCatalog::from_documents(
        path,
        &[(path, GLOBALS)],
        &[GlobalRequirement::shape("seq"), GlobalRequirement::shape("loop"), GlobalRequirement::shape("group")],
    )
    .expect("the shipped catalog loads");
    HostLifecycleProfile::from_artifact(Box::leak(Box::new(catalog)), Path::new(path), GLOBALS, FactsArtifact::Text(ARTIFACT))
        .expect("the committed artifact reads")
}

fn frontmatter_of(text: &str) -> Value {
    let markdown = Markdown::from(text);
    Value::Object(markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect())
}

/// The lines of every surface the library walk finds a strict call
/// diagnostic in, under `host`'s registry and parse options.
fn library_lines(host: Host, text: &str) -> Vec<u64> {
    let markdown = Markdown::from(text);
    let frontmatter = frontmatter_of(text);
    let dir = tempfile::tempdir().unwrap();
    let schema = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path()))
        .effective_for(&markdown)
        .expect("the schema resolves");
    let profile = matches!(host, Host::Claudine(_)).then(shipped_profile);
    let (config, registry) = match &profile {
        Some(profile) => (
            darkmatter::lifecycle::parse_lifecycle_config_with(&frontmatter, Path::new("doc.md"), &profile.parse_options(None)),
            profile.registry(),
        ),
        None => (md_host::parse_lifecycle(&frontmatter, Path::new("doc.md"), None), md_host::registry()),
    };
    let config = config.expect("the host parses the document");
    let mut lines: Vec<u64> = darkmatter::lifecycle::lifecycle_call_checks(&frontmatter, &config, registry, schema.as_ref())
        .iter()
        .filter(|surface| surface.checks.iter().any(|found| !found.check.diagnostics(true).is_empty()))
        .map(|surface| line_of(text, &surface.text))
        .collect();
    lines.sort_unstable();
    lines
}
