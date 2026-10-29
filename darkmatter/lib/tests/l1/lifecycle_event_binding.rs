//! Event-time binding of lifecycle action operands, through the `md` host's
//! public evaluation entry against a temporary mutation root:
//!
//! - a side effect's evaluated operands bind to its typed overloads before
//!   the effect engine runs, and the engine receives the bound values;
//! - an expression-function action renders a template operand once before
//!   the call, and a lazy function still stops at its short-circuit.

use std::path::Path;
use std::sync::Mutex;

use darkmatter::effects::EffectEngine;
use darkmatter::lifecycle::executor::LifecycleEventOutcome;
use darkmatter::lifecycle::md_host::{self, CommandOutput, MdEvent, MdRuntimeState, MdStackContext};
use darkmatter::lifecycle::run::ShellRefusal;
use darkmatter::lifecycle::{LifecycleEffects, LifecycleSignal, LifecycleTiming};
use darkmatter::markdown::compose::shell_expansion::ShellRuleSet;
use darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, SharedFrontmatter};
use serde_json::{Value, json};
use tempfile::TempDir;

struct Silent;

impl LifecycleEffects for Silent {
    fn stdout(&self, _: &str) {}
    fn stderr(&self, _: LifecycleSignal, _: &str) {}
    fn info(&self, _: &str) {}
    fn warn(&self, _: &str) {}
    fn success(&self, _: &str) {}
    fn message(&self, _: &str) {}
    fn speech(&self, _: &str) {}
    fn effect(&self, _: &str) {}
    fn notification(&self, _: &str) {}
}

impl CommandOutput for Silent {
    fn stdout(&self, _: &str) {}
    fn stderr(&self, _: &str) {}
}

const DOCUMENT: &str = "---\ntitle: kept\n---\nbody\n";

/// A mutation root holding `doc.md`, the document every stack runs for.
struct Root {
    dir: TempDir,
}

impl Root {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::write(dir.path().join("doc.md"), DOCUMENT).expect("document");
        Self { dir }
    }

    fn path(&self, name: &str) -> std::path::PathBuf {
        self.dir.path().join(name)
    }

    /// Runs the `composed` stack (the start slot) of `lifecycle` with `frontmatter` as `fm`.
    fn start(&self, frontmatter: Value, lifecycle: Value) -> LifecycleEventOutcome {
        let source = self.path("doc.md");
        let config = md_host::parse_lifecycle(&lifecycle, &source, None)
            .unwrap_or_else(|error| panic!("{lifecycle} parses: {error}"));
        let options = ComposeOptions::new_with_context(ComposeContext::capture_minimal()).with_source_file(&source);
        let request = ComposeRequest::with_context(options, biscuit_file::FileResolutionContext::new(self.dir.path()))
            .expect("request");
        let frontmatter: SharedFrontmatter = frontmatter.as_object().cloned().unwrap_or_default().into();
        let live = Mutex::new(frontmatter.clone());
        let state = MdRuntimeState::default();
        let approved = ShellPreApprovals::new();
        let blacklist = ShellRuleSet::default();
        let engine = EffectEngine::builder().mutation_root(self.dir.path()).auto_rehash(false).build();
        let timing = LifecycleTiming::default();
        let never = || false;
        let context = MdStackContext {
            request: &request,
            source_path: &source,
            frontmatter: &frontmatter,
            live_frontmatter: &live,
            state: &state,
            approved: &approved,
            user_blacklist: &blacklist,
            effect_engine: &engine,
            effects: &Silent,
            output: &Silent,
            timing: &timing,
            interrupted: &never,
        };
        let event = MdEvent {
            signal: LifecycleSignal::Start,
            name: "composed",
            error: None,
            shells: Err(ShellRefusal::Locked),
        };
        md_host::evaluate(&context, &config, &event)
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn evaluation_message(outcome: &LifecycleEventOutcome) -> &str {
    let error = outcome
        .evaluation_error
        .as_ref()
        .unwrap_or_else(|| panic!("expected an evaluation error, got {outcome:?}"));
    &error.msg
}

// ── Side effects bind before dispatch ────────────────────────────────────

#[test]
fn a_side_effect_operand_that_binds_no_overload_fails_before_the_engine() {
    let root = Root::new();

    let outcome = root.start(json!({}), json!({ "composed": [{ "merge_frontmatter": ["doc.md", "{{ 5 }}"] }] }));

    let message = evaluation_message(&outcome);
    assert!(message.contains("merge_frontmatter"), "{message}");
    assert!(message.contains("obj (argument 2)"), "the failing argument is named: {message}");
    assert!(outcome.action_error.is_none(), "never a dispatch failure: {outcome:?}");
    assert_eq!(read(&root.path("doc.md")), DOCUMENT, "the document is unchanged");
}

#[test]
fn no_error_does_not_suppress_a_binding_failure() {
    // A binding failure is an evaluation failure, which `no_error` never
    // suppresses; only a dispatch failure is suppressible.
    let root = Root::new();

    let outcome = root.start(
        json!({}),
        json!({ "composed": [{ "merge_frontmatter": { "file": "doc.md", "obj": "{{ 5 }}", "no_error": true } }] }),
    );

    assert!(evaluation_message(&outcome).contains("obj (argument 2)"));
    assert_eq!(read(&root.path("doc.md")), DOCUMENT);
}

#[test]
fn a_side_effect_receives_its_bound_values() {
    let root = Root::new();

    let outcome = root.start(json!({}), json!({ "composed": [{ "ensure_file": ["out.txt", 3] }] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(read(&root.path("out.txt")), "3", "a number for a text parameter arrives as its text");
}

#[test]
fn the_bound_overload_selects_the_ensure_file_form() {
    let root = Root::new();

    let outcome = root.start(
        json!({}),
        json!({ "composed": [{ "ensure_file": "empty.txt" }, { "ensure_file": { "file": "full.txt", "content": "body" } }] }),
    );

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(read(&root.path("empty.txt")), "");
    assert_eq!(read(&root.path("full.txt")), "body");
}

#[test]
fn a_template_operand_that_evaluates_to_a_valid_value_dispatches() {
    let root = Root::new();

    let outcome = root.start(
        json!({ "status": "ready" }),
        json!({ "composed": [{ "merge_frontmatter": ["doc.md", "{{ { state: fm.status } }}"] }] }),
    );

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert!(read(&root.path("doc.md")).contains("state: ready"), "{}", read(&root.path("doc.md")));
}

// ── Expression-function operands ─────────────────────────────────────────

#[test]
fn a_mixed_text_operand_renders_before_the_function_binds() {
    // `"1{{ fm.digit }}"` arrived as raw template text, which `min` cannot
    // bind as a number; rendered, it is `15`.
    let root = Root::new();

    let outcome = root.start(json!({ "digit": 5 }), json!({ "composed": [{ "min": ["1{{ fm.digit }}", 50] }] }));

    assert_eq!(outcome, LifecycleEventOutcome::default());
}

#[test]
fn lazy_functions_short_circuit_before_a_failing_template_operand() {
    // `number(fm.word)` fails for `abc`; reaching the operand would raise.
    let root = Root::new();
    let failing = "x{{ number(fm.word) }}";
    let frontmatter = json!({ "word": "abc" });

    for (verb, stop) in [("and", false), ("or", true)] {
        let outcome = root.start(frontmatter.clone(), json!({ "composed": [{ verb: [stop, failing] }] }));
        assert_eq!(outcome, LifecycleEventOutcome::default(), "{verb} stops at {stop}");

        let outcome = root.start(frontmatter.clone(), json!({ "composed": [{ verb: [!stop, failing] }] }));
        assert!(evaluation_message(&outcome).contains("number"), "{verb} evaluates the operand after {}", !stop);
    }
}
