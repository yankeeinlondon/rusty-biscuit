//! A host's lifecycle globals stay protected from `::map` / `::filter`
//! callback names, through Darkmatter's public collection analysis and the
//! binding view Claudine declares for each lifecycle scope.

use claudine::composition::lifecycle::bindings::{GroupMembership, LifecycleScope, binding_view};
use darkmatter::markdown::compose::collection_blocks::{CollectionAnalysisOptions, analyze_collection_blocks};
use darkmatter::markdown::compose::expression::{BindingView, DeclaredTypes, TypeScope};
use darkmatter::markdown::compose::ComposeOperationSet;

fn analyze(source: &str, view: Option<&BindingView>) -> Result<(), String> {
    let declared = DeclaredTypes::new(None);
    let site = TypeScope::unguarded(&declared);
    let options = CollectionAnalysisOptions {
        operations: ComposeOperationSet::all(),
        binding_view: view,
        // The source is undeclared; the recoveries waive only that requirement.
        allow_untyped_map_variables: true,
        allow_untyped_filter_variables: true,
        source_path: None,
    };
    analyze_collection_blocks(source, &site, &options).map(|_| ()).map_err(|error| error.code().to_string())
}

fn region(kind: &str, callback: &str) -> String {
    format!("::{kind} data \"{callback}\"\nbody\n::end-{kind}\n")
}

#[test]
fn every_declared_lifecycle_global_is_a_protected_callback_name_in_every_scope() {
    for scope in LifecycleScope::ALL {
        let view = binding_view(scope, GroupMembership::Unknown);
        let globals: Vec<String> = view.globals().map(|global| global.root().to_string()).collect();
        assert!(!globals.is_empty(), "{scope:?} declares no globals");
        for global in &globals {
            for kind in ["map", "filter"] {
                let source = region(kind, &format!("{global} => 1"));
                assert_eq!(
                    analyze(&source, Some(&view)),
                    Err("dm.expression.invalid_callback".to_string()),
                    "{scope:?} {kind} {global}"
                );
                // Protection follows the view in force: a plain document view allows the name.
                assert_eq!(analyze(&source, None), Ok(()), "{scope:?} {kind} {global} without the host view");
            }
        }
    }
}

#[test]
fn a_callback_may_read_a_declared_global_it_does_not_shadow() {
    let view = binding_view(LifecycleScope::Event(claudine::composition::lifecycle::LifecycleSignal::Loop), GroupMembership::Unknown);
    for kind in ["map", "filter"] {
        assert_eq!(analyze(&region(kind, "i => loop.count"), Some(&view)), Ok(()), "{kind}");
    }
}

mod expression_lifecycle {
    use std::path::Path;

    use claudine::composition::{
        DefaultLifecycleEmitter, PrepareOptions, PreparedComposition, ResolvedCompositionSource,
        prepare_direct,
    };
    use claudine::composition::lifecycle::executor::{
        DisabledShellRunner, LifecycleEventOutcome, StackControl, StackExecutionContext,
    };
    use claudine::composition::LifecycleSignal;
    use claudine::events::GlobalSettings;
    use claudine::messaging::RuntimeMessagingSettings;
    use darkmatter::markdown::compose::SharedFrontmatter;
    use serde_json::{Value, json};

    const UNMET: &str = "{{ filter(depends-on, dep => !frontmatter(first(find_files('**/' + dep + '/spec.md')), 'implemented')) }}";

    fn prepare(root: &Path, document: &str) -> PreparedComposition {
        let path = root.join("gate.md");
        std::fs::write(&path, document).unwrap();
        let source = ResolvedCompositionSource {
            original_ref: path.display().to_string(),
            resolved_path: path,
            original_text: document.to_string(),
            markdown: document.into(),
        };
        let context = biscuit_file::FileResolutionContext::from_snapshot(root, None, Default::default());
        prepare_direct(&source, PrepareOptions::new(context)).expect("fixture prepares")
    }

    fn initialize(root: &Path, prepared: &PreparedComposition) -> LifecycleEventOutcome {
        let frontmatter = SharedFrontmatter::from(prepared.effective_frontmatter.as_object().unwrap().clone());
        let engine = darkmatter::effects::EffectEngine::builder().mutation_root(root).auto_rehash(false).build();
        let context = biscuit_file::FileResolutionContext::from_snapshot(root, None, Default::default());
        let settings = GlobalSettings::default();
        let messaging = RuntimeMessagingSettings { user: None, repo: None };
        let term = biscuit_terminal::terminal::Terminal::default();
        let runtime = claudine::composition::runtime_state::RuntimeState::new();
        runtime.append_output_entry(json!("earlier"));
        let seq = json!({"outputs": []});
        let stack = StackExecutionContext {
            signal: LifecycleSignal::Initialize, scope: None, frontmatter: &frontmatter,
            live_frontmatter: None, runtime_state: Some(&runtime), err: None, timing: None,
            current: None, group: None, seq: Some(&seq), loop_values: None,
            base_dir: Some(root), ctx_base_dir: Some(root),
            prepared_context: Some(&prepared.compose_context), file_resolution_context: &context,
            effect_engine: &engine, shell_runner: &DisabledShellRunner,
            emitter: &DefaultLifecycleEmitter, term: &term, source_path: &prepared.resolved_path,
            repo_root: None, messaging: &messaging, settings: &settings,
        };
        stack.execute_event(&prepared.lifecycle)
    }

    fn dependency_gate(dependencies: Value, implementation: Option<&str>, expected: &[&str]) {
        let dir = tempfile::tempdir().unwrap();
        for (name, value) in [("ready", Some("true")), ("pending", implementation)] {
            let root = dir.path().join("features").join(name);
            std::fs::create_dir_all(&root).unwrap();
            let field = value.map(|value| format!("implemented: {value}\n")).unwrap_or_default();
            std::fs::write(root.join("spec.md"), format!("---\n{field}---\n")).unwrap();
        }
        let document = format!(
            "---\n$schema:\n  depends-on: string[](required)\n  unmet: string[](required)\ndepends-on: {dependencies}\nunmet: \"{UNMET}\"\ninitialize:\n  - when: unmet\n    then:\n      - error: 'cannot start: {{{{ as_csv(unmet) }}}} not implemented'\n---\nReady to compose.\n"
        );
        let prepared = prepare(dir.path(), &document);
        assert_eq!(prepared.effective_frontmatter["unmet"], json!(expected));
        assert_eq!(prepared.prompt.trim(), "Ready to compose.");
        let outcome = initialize(dir.path(), &prepared);
        assert!(outcome.action_error.is_none(), "{outcome:?}");
        assert!(outcome.evaluation_error.is_none(), "{outcome:?}");
        if expected.is_empty() {
            assert_eq!(outcome.control, None);
        } else {
            assert_eq!(outcome.control, Some(StackControl::Error {
                reason: Some("cannot start: pending not implemented".to_string()),
            }));
        }
    }

    #[test]
    fn dependency_filter_rejects_and_names_an_unimplemented_spec() {
        dependency_gate(json!(["ready", "pending"]), Some("false"), &["pending"]);
    }

    #[test]
    fn dependency_filter_accepts_all_implemented_specs() {
        dependency_gate(json!(["ready", "pending"]), Some("true"), &[]);
    }

    #[test]
    fn dependency_filter_accepts_an_empty_dependency_list() {
        dependency_gate(json!([]), Some("false"), &[]);
    }

    #[test]
    fn dependency_filter_treats_null_implementation_as_unimplemented() {
        dependency_gate(json!(["pending"]), Some("null"), &["pending"]);
    }

    #[test]
    fn dependency_filter_treats_absent_implementation_as_unimplemented() {
        dependency_gate(json!(["pending"]), None, &["pending"]);
    }

    #[test]
    fn map_condition_and_error_interpolation_read_the_host_sequence_global() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare(dir.path(), "---\ninitialize:\n  - when: map([1], item => seq.outputs)\n    then:\n      - error: 'previous: {{ as_csv(map(seq.outputs, item -> upper(item))) }}'\n---\nbody\n");
        let outcome = initialize(dir.path(), &prepared);
        assert_eq!(outcome.control, Some(StackControl::Error { reason: Some("previous: EARLIER".into()) }));
        assert!(outcome.evaluation_error.is_none(), "{outcome:?}");
    }

    #[test]
    fn expression_callbacks_cannot_shadow_a_host_global_even_on_empty_inputs() {
        let dir = tempfile::tempdir().unwrap();
        for function in ["map", "filter"] {
            let text = format!("---\ninitialize:\n  - when: {function}([], seq => true)\n    then:\n      - error: unreachable\n---\nbody\n");
            let prepared = prepare(dir.path(), &text);
            let outcome = initialize(dir.path(), &prepared);
            assert!(outcome.control.is_none(), "{outcome:?}");
            assert!(outcome.action_error.is_none(), "{outcome:?}");
            let error = outcome.evaluation_error.expect("a protected parameter is an evaluation error");
            assert!(error.msg.contains("seq"), "{error:?}");
            assert!(error.msg.contains("callback"), "{error:?}");
        }
    }
}
