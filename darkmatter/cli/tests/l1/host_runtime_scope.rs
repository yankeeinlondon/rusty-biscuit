//! Host functions with a real handler, in process, through the `md` host's
//! public lifecycle evaluation entry (`md_host::evaluate`) against a
//! temporary mutation root. A set built with `HostFunctions::builder` and
//! entered on this thread is what a host runtime supplies; `md compose` never
//! has one (see `compose_host_scope`).
//!
//! - an expression in an action operand calls the host function through the
//!   expression dispatcher, and the same spelling as an action key is an unknown
//!   verb that `md` skips;
//! - the handler runs only after its arguments bind (a pre-conversion source
//!   exclusion refuses before entry), and its result is checked against the
//!   declared return type;
//! - a built-in expression function that is also an action verb works in
//!   both positions;
//! - an effect-only mutation is not an ordinary expression function.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use darkmatter::effects::EffectEngine;
use darkmatter::lifecycle::executor::LifecycleEventOutcome;
use darkmatter::lifecycle::md_host::{self, CommandOutput, MdEvent, MdRuntimeState, MdStackContext};
use darkmatter::lifecycle::run::ShellRefusal;
use darkmatter::lifecycle::{LifecycleEffects, LifecycleSignal, LifecycleTiming};
use darkmatter::markdown::compose::expression::{HostFunctionCatalog, HostFunctions};
use darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, SharedFrontmatter};
use serde_json::{Value, json};
use simplified_schema::coercion::ValueKind;
use tempfile::TempDir;

const CATALOG: &str = "\
kind: expression-schema
categories:
    - Acme -> Ask the acme host.
types:
    acme_double: |-
        function(
            parameters(value: number);
            returns(number);
            category(Acme);
        ) -> Twice the value.
    acme_broken: |-
        function(
            parameters(value: number);
            returns(number);
            category(Acme);
        ) -> Declares a number and answers text.
";

/// The acme host's runtime: `acme_double` refuses a string source before
/// conversion and counts its entries; `acme_broken` breaks its return type.
fn acme(entries: &Arc<AtomicUsize>) -> HostFunctions {
    let catalog = HostFunctionCatalog::load(CATALOG, "functions.yaml").expect("the catalog loads");
    let counted = Arc::clone(entries);
    HostFunctions::builder(catalog)
        .function(
            "acme_double",
            &[],
            |registration| registration.reject_sources("value", &[ValueKind::Text]),
            Arc::new(move |call| {
                counted.fetch_add(1, Ordering::SeqCst);
                Ok(json!(call.args().number(0).expect("a bound number") * 2.0))
            }),
        )
        .function("acme_broken", &[], |registration| registration, Arc::new(|_| Ok(json!("text"))))
        .build()
        .expect("the host registers")
}

/// Collects `info` and `warn` lines; every other channel is silent.
#[derive(Default)]
struct Recorder {
    lines: Mutex<Vec<String>>,
}

impl LifecycleEffects for Recorder {
    fn stdout(&self, _: &str) {}
    fn stderr(&self, _: LifecycleSignal, _: &str) {}
    fn info(&self, text: &str) {
        self.lines.lock().unwrap().push(text.to_string());
    }
    fn warn(&self, text: &str) {
        self.lines.lock().unwrap().push(text.to_string());
    }
    fn success(&self, _: &str) {}
    fn message(&self, _: &str) {}
    fn speech(&self, _: &str) {}
    fn effect(&self, _: &str) {}
    fn notification(&self, _: &str) {}
}

impl CommandOutput for Recorder {
    fn stdout(&self, _: &str) {}
    fn stderr(&self, _: &str) {}
}

/// A mutation root holding `doc.md`, the document every stack runs for.
struct Root {
    dir: TempDir,
}

impl Root {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temporary directory");
        std::fs::write(dir.path().join("doc.md"), "---\ntitle: kept\n---\nbody\n").expect("document");
        Self { dir }
    }

    fn path(&self, name: &str) -> std::path::PathBuf {
        self.dir.path().join(name)
    }

    /// Runs `stack` as the `composed` event with `frontmatter` as `fm`: the
    /// outcome and the `info` and `warn` lines it wrote.
    fn start(&self, frontmatter: Value, stack: Value) -> (LifecycleEventOutcome, Vec<String>) {
        let source = self.path("doc.md");
        let config = md_host::parse_lifecycle(&json!({ "composed": stack }), &source, None)
            .unwrap_or_else(|error| panic!("{stack} parses: {error}"));
        let options = ComposeOptions::new_with_context(ComposeContext::capture_minimal()).with_source_file(&source);
        let request = ComposeRequest::with_context(options, biscuit_file::FileResolutionContext::new(self.dir.path()))
            .expect("request");
        let frontmatter: SharedFrontmatter = frontmatter.as_object().cloned().unwrap_or_default().into();
        let live = Mutex::new(frontmatter.clone());
        let state = MdRuntimeState::default();
        let approved = ShellPreApprovals::new();
        let engine = EffectEngine::builder().mutation_root(self.dir.path()).auto_rehash(false).build();
        let timing = LifecycleTiming::default();
        let recorder = Recorder::default();
        let never = || false;
        let context = MdStackContext {
            request: &request,
            source_path: &source,
            frontmatter: &frontmatter,
            live_frontmatter: &live,
            state: &state,
            approved: &approved,
            effect_engine: &engine,
            effects: &recorder,
            output: &recorder,
            timing: &timing,
            interrupted: &never,
            editor: None,
        };
        let event = MdEvent {
            signal: LifecycleSignal::Start,
            name: "composed",
            error: None,
            shells: Err(ShellRefusal::Locked),
        };
        let outcome = md_host::evaluate(&context, &config, &event);
        (outcome, recorder.lines.into_inner().unwrap())
    }
}

fn evaluation_message(outcome: &LifecycleEventOutcome) -> String {
    outcome
        .evaluation_error
        .as_ref()
        .unwrap_or_else(|| panic!("expected an evaluation error, got {outcome:?}"))
        .msg
        .clone()
}

#[test]
fn a_host_call_in_an_action_operand_runs_its_handler_and_its_name_is_no_action_key() {
    let entries = Arc::new(AtomicUsize::new(0));
    let host = acme(&entries);
    let _scope = host.enter();
    let root = Root::new();

    let (operand, info) = root.start(json!({ "n": 3 }), json!([{ "info": "{{ acme_double(fm.n) }}" }]));
    let (action_key, skipped) = root.start(json!({}), json!([{ "acme_double": [2] }]));

    assert_eq!(operand, LifecycleEventOutcome::default());
    assert_eq!(info, ["6"]);
    // An unknown verb to `md`: kept by name and skipped, never dispatched.
    assert_eq!(action_key, LifecycleEventOutcome::default());
    assert_eq!(skipped, ["`md` does not run `acme_double`; skipped it in `composed`"]);
    assert_eq!(entries.load(Ordering::SeqCst), 1, "only the operand entered the handler");
}

#[test]
fn a_host_handler_runs_only_after_binding_and_its_result_is_checked() {
    let entries = Arc::new(AtomicUsize::new(0));
    let host = acme(&entries);
    let _scope = host.enter();
    let root = Root::new();

    let (excluded, excluded_info) = root.start(json!({}), json!([{ "info": "{{ acme_double('2') }}" }]));
    let (broken, broken_info) = root.start(json!({}), json!([{ "info": "{{ acme_broken(1) }}" }]));

    let message = evaluation_message(&excluded);
    assert!(message.contains("acme_double"), "{message}");
    assert_eq!(entries.load(Ordering::SeqCst), 0, "a refused source never enters the handler: {message}");
    let message = evaluation_message(&broken);
    assert!(message.contains("acme_broken") && message.contains("return"), "the declared return judges the result: {message}");
    assert!(excluded_info.is_empty() && broken_info.is_empty(), "nothing was written");
}

#[test]
fn a_built_in_function_that_is_also_an_action_works_in_both_positions() {
    let entries = Arc::new(AtomicUsize::new(0));
    let host = acme(&entries);
    let _scope = host.enter();
    let root = Root::new();

    let (operand, info) = root.start(json!({}), json!([{ "info": "{{ round(2.6) }}" }]));
    let (action, _) = root.start(json!({}), json!([{ "round": [2.6] }]));

    assert_eq!(operand, LifecycleEventOutcome::default());
    assert_eq!(info, ["3"]);
    assert_eq!(action, LifecycleEventOutcome::default());
}

#[test]
fn an_effect_only_mutation_is_no_ordinary_expression_function() {
    let root = Root::new();

    let (operand, info) = root.start(json!({}), json!([{ "info": "{{ ensure_file('operand.txt') }}" }]));
    let (action, _) = root.start(json!({}), json!([{ "ensure_file": ["action.txt"] }]));

    let message = evaluation_message(&operand);
    assert!(message.contains("ensure_file"), "{message}");
    assert!(info.is_empty());
    assert!(!root.path("operand.txt").exists(), "the expression wrote nothing");
    assert_eq!(action, LifecycleEventOutcome::default());
    assert!(root.path("action.txt").exists(), "the action is the effect");
}
