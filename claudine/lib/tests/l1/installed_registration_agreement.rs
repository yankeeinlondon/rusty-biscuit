//! The installed registration and Claudine's in-process validation agree.
//!
//! Each case publishes Claudine's inventory into a temporary home, registers
//! its manifest in that home's `~/.darkmatter.yaml`, and judges one document
//! twice: passively, as `md schema validate` and DMLS judge it
//! (`DarkmatterSchemas::validate` / `validate_yaml_document` with the loaded
//! registrations), and in process, through the Claudine launch entry point
//! the document would take (inline, direct, or sequence), which
//! reads only the inventory compiled into this build. Both must accept, or
//! both refuse at the same property paths.
//!
//! Only the schema verdict is compared. The one in-process refusal that is
//! not a schema verdict and still counts as accepted is the inline launch
//! finding no prompt in a `sections` document: sections are validated by
//! `inline-composition.yaml`, but running them is not built yet. Any other
//! non-schema refusal fails the test.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use claudine::composition::registration::{MANIFEST_REFERENCE, RegistrationInventory, publish};
use claudine::composition::{
    CompositionError, PrepareOptions, ResolvedCompositionSource, TriggerFindings, build_preflight_graph,
    load_yaml_document, pre_validate_schema, prepare_direct_with_schema, prepare_inline_with_schema,
    resolve_composition_source_in_context, resolve_sequence_plan,
};
use darkmatter::config::load::{ConfigLoadMode, ConfigSnapshot};
use darkmatter::config::registration::RegisteredHosts;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::{DarkmatterSchemas, YamlOutcome};

/// The Claudine entry point a document takes in process.
#[derive(Clone, Copy, Debug)]
enum Launch {
    Inline,
    Direct,
    /// A Markdown sequence document. Its launch parses `sequence` before any
    /// schema runs, so a value of the wrong shape is refused by the parser
    /// with no property path; the verdict compared is Claudine's own schema
    /// verdict (`pre_validate_schema`), and the launch must refuse whatever
    /// that verdict refuses.
    SequenceDocument,
    /// A formal `kind: sequence` YAML file, judged by sequence preflight.
    FormalSequence,
}

/// A schema verdict: accepted, or refused at these JSON pointer paths.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    Accepted,
    Refused(BTreeSet<String>),
}

/// `{dir}/home`, holding the published registration and the `register`
/// entry, and `{dir}/work`, the launch directory; the folder is no
/// repository.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn installed() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("home")).unwrap();
        std::fs::create_dir_all(root.join("work")).unwrap();
        let fixture = Self { _dir: dir, root };
        publish(&fixture.home(), RegistrationInventory::claudine()).expect("the inventory publishes");
        std::fs::write(fixture.home().join(".darkmatter.yaml"), format!("register:\n  - {MANIFEST_REFERENCE}\n")).unwrap();
        fixture
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn context(&self) -> FileResolutionContext {
        let snapshot = RequestSnapshot::new(self.root.join("work")).with_home(Some(self.home())).with_env(HashMap::new());
        build_resolution_context(&snapshot).unwrap()
    }

    fn hosts(&self) -> Arc<RegisteredHosts> {
        let context = self.context();
        let snapshot = ConfigSnapshot::load(&context, None, ConfigLoadMode::Passive).unwrap();
        let hosts = RegisteredHosts::load(&snapshot, &context);
        assert!(hosts.diagnostics().is_empty(), "{:#?}", hosts.diagnostics());
        assert_eq!(hosts.loaded().profiles.len(), 1, "the installed Claudine profile");
        Arc::new(hosts)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join("work").join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    /// The installed registration's verdict, as the passive reader reaches it.
    fn passive(&self, path: &Path, text: &str) -> Verdict {
        let schemas = DarkmatterSchemas::new(self.context().for_source(path)).with_registered_hosts(self.hosts());
        let problems = if path.extension().is_some_and(|extension| extension == "yaml") {
            match schemas.validate_yaml_document(text, path) {
                Ok(YamlOutcome::Validated(validation)) => validation.report.problems,
                Ok(YamlOutcome::NoSchema) => panic!("{}: the installed kind selects no schema", path.display()),
                Err(error) => panic!("{}: the installed kind refuses to prepare: {error}", path.display()),
            }
        } else {
            let markdown = Markdown::try_from(path).unwrap();
            schemas.validate(&markdown).expect("the schema prepares").problems
        };
        verdict(problems.into_iter().map(|problem| problem.path))
    }

    /// Claudine's in-process verdict through `launch`.
    #[allow(clippy::result_large_err, reason = "the entry points' own results, compared unchanged")]
    fn in_process(&self, path: &Path, launch: Launch) -> Verdict {
        let context = self.context();
        let outcome = match launch {
            Launch::Inline | Launch::Direct => {
                let source = resolve_composition_source_in_context(path.to_str().unwrap(), &context).unwrap();
                let options = PrepareOptions::new(context);
                match launch {
                    Launch::Inline => prepare_inline_with_schema(&source, options).map(drop),
                    _ => prepare_direct_with_schema(&source, options).map(drop),
                }
            }
            Launch::SequenceDocument => {
                let source = resolve_composition_source_in_context(path.to_str().unwrap(), &context).unwrap();
                let schema = pre_validate_schema(&source, None, None, &context, &TriggerFindings::default());
                let launch = resolve_sequence_plan(&source, &context)
                    .and_then(|plan| build_preflight_graph(&plan.expect("a sequence"), &source, &context).map(drop));
                match schema {
                    Ok(validated) => {
                        let dropped: Vec<&str> =
                            validated.dropped_optionals.iter().map(|dropped| dropped.property.as_str()).collect();
                        assert!(dropped.is_empty(), "{}: optional values dropped: {dropped:?}", path.display());
                        launch
                    }
                    Err(refusal) => {
                        assert!(launch.is_err(), "{}: the schema refuses but the launch accepts", path.display());
                        Err(refusal)
                    }
                }
            }
            Launch::FormalSequence => {
                let source = ResolvedCompositionSource {
                    original_ref: path.display().to_string(),
                    resolved_path: path.to_path_buf(),
                    original_text: std::fs::read_to_string(path).unwrap(),
                    markdown: load_yaml_document(path).unwrap(),
                };
                resolve_sequence_plan(&source, &context)
                    .and_then(|plan| build_preflight_graph(&plan.expect("a sequence"), &source, &context).map(drop))
            }
        };
        match outcome {
            Ok(()) => Verdict::Accepted,
            Err(CompositionError::SchemaValidation { problems, .. }) => verdict(problems),
            Err(CompositionError::MissingProperties { pointer_paths, .. }) => verdict(pointer_paths),
            // The schema passed; the inline launch then finds no prompt.
            Err(CompositionError::PromptPropertyMissing) if matches!(launch, Launch::Inline) => Verdict::Accepted,
            Err(other) => panic!("{}: an in-process refusal that is not a schema verdict: {other:?}", path.display()),
        }
    }
}

fn verdict(paths: impl IntoIterator<Item = String>) -> Verdict {
    let paths: BTreeSet<String> = paths.into_iter().collect();
    if paths.is_empty() { Verdict::Accepted } else { Verdict::Refused(paths) }
}

/// Judges `text` as `work/{name}` both ways and asserts they agree on
/// `expected`, the accept/refuse outcome (a refusal's paths must be equal,
/// and include `pointer`).
fn assert_agreement(name: &str, text: &str, launch: Launch, expected: Option<&str>) {
    let fixture = Fixture::installed();
    let path = fixture.write(name, text);

    let installed = fixture.passive(&path, text);
    let in_process = fixture.in_process(&path, launch);

    assert_eq!(installed, in_process, "{name}: the installed registration and the in-process verdict disagree");
    match (expected, &installed) {
        (None, Verdict::Accepted) => {}
        (Some(pointer), Verdict::Refused(paths)) => assert!(paths.contains(pointer), "{name}: {paths:?}"),
        (expected, actual) => panic!("{name}: expected a refusal at {expected:?}, got {actual:?}"),
    }
}

macro_rules! agreement {
    ($($test:ident: $name:literal, $text:expr, $launch:expr, $expected:expr;)+) => {$(
        #[test]
        fn $test() {
            assert_agreement($name, $text, $launch, $expected);
        }
    )+};
}

agreement! {
    installed_registration_agreement_shared_events_only:
        "shared.md", "---\nfinalize: [stop]\n---\nBody\n", Launch::Direct, None;
    installed_registration_agreement_prompt_only:
        "prompt.md", "---\nprompt: Fix the bug.\n---\nBody\n", Launch::Inline, None;
    installed_registration_agreement_sections_only:
        "sections.md", "---\nsections:\n    Goal: Fix the bug.\n---\nBody\n", Launch::Inline, None;
    installed_registration_agreement_prompt_and_sections:
        "both.md", "---\nprompt: Fix the bug.\nsections:\n    Goal: Fix the bug.\n---\nBody\n", Launch::Inline, Some("/sections");
    installed_registration_agreement_sequence_of_the_wrong_type:
        "sequence.md", "---\nsequence: { steps: 3 }\n---\nBody\n", Launch::SequenceDocument, Some("/sequence");
    installed_registration_agreement_composition_with_agent_and_model:
        "agent.md", "---\nagent: claude\nmodel: sonnet\nstart: [stop]\n---\nBody\n", Launch::Direct, None;
    installed_registration_agreement_formal_sequence_yaml:
        "run.yaml", "kind: sequence\ndescription: steps\nsequence:\n  - one\n", Launch::FormalSequence, None;
    installed_registration_agreement_formal_sequence_yaml_of_the_wrong_type:
        "bad.yaml", "kind: sequence\ndescription: [not, text]\nsequence:\n  - one\n", Launch::FormalSequence, Some("/description");
}
