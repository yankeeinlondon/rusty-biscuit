//! Initialize-bootstrap read tests.
//!
//! The claim under test is ordering, not tolerance: the bootstrap read of a
//! document whose body includes a file `initialize` has not created yet
//! succeeds, while full preparation of the same document still fails until the
//! file exists. Fixture: `fixes/2026-09-15-initialize-after-proxy`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use darkmatter::markdown::compose::expression::Expr;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use tempfile::TempDir;

use crate::composition::lifecycle_actions::LifecycleActionKind;
use crate::composition::types::CallerInputLayers;
use crate::composition::{
    BootstrapPreparation, BootstrapRequest, CompositionError, CompositionMode, DocumentEntryReason,
    DocumentPreparation, LifecycleConfig, LifecycleSignal, PrepareOptions, PromptSource,
    ResolvedCompositionSource, SchemaStage, preflight_bootstrap_shell, prepare_bootstrap,
    prepare_document,
};
use crate::harness::ShellApprovalOptions;

/// The reported caller argument spelling: a launch-relative spec path.
const SPEC: &str = "fixes/2026-09-14-demo/spec.md";
const LOG: &str = "fixes/2026-09-14-demo/implementation-log.md";

fn source_at(dir: &Path, text: &str) -> ResolvedCompositionSource {
    let file = dir.join("implement-plan.md");
    fs::write(&file, text).unwrap();
    let original_text = fs::read_to_string(&file).unwrap();
    ResolvedCompositionSource {
        original_ref: "./_implement/implement-plan.md".to_string(),
        resolved_path: file,
        original_text: original_text.clone(),
        markdown: original_text.into(),
    }
}

/// Caller `spec` and `phase`, anchored on the launch directory.
fn options_in(dir: &Path, phase: u32) -> PrepareOptions {
    CallerInputLayers {
        set_overrides: Some(serde_json::json!({ "spec": SPEC, "phase": phase })),
        file_ref_fallback_dir: Some(dir.to_path_buf()),
        ..CallerInputLayers::default()
    }
    .apply_to(PrepareOptions::default())
}

fn bootstrap(
    mode: CompositionMode,
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<BootstrapPreparation, CompositionError> {
    prepare_bootstrap(BootstrapRequest {
        entry: DocumentEntryReason::ProxyTarget,
        mode,
        source,
        options,
    })
}

fn full(
    mode: CompositionMode,
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<crate::composition::PreparedComposition, CompositionError> {
    prepare_document(DocumentPreparation {
        entry: DocumentEntryReason::ProxyTarget,
        mode,
        source,
        prompt_source: PromptSource::ComposedBody,
        schema: SchemaStage::Validate,
        options,
    })
}

fn shell_commands(lifecycle: &LifecycleConfig, signal: LifecycleSignal) -> Vec<Expr> {
    lifecycle
        .stack(signal)
        .into_iter()
        .flatten()
        .flat_map(|item| &item.actions)
        .filter_map(|action| match &action.kind {
            LifecycleActionKind::Shell(shell) => Some(shell.command.clone()),
            _ => None,
        })
        .collect()
}

/// The reproduction's target: `log` derives from the caller's `spec` with
/// `dirname`, `initialize` creates it, and the body includes it both inside the
/// reported nested guards and unconditionally.
const GENERATED_INCLUDE_DOC: &str = "\
---
spec: \"\"
phase: 1
log: \"{{ dirname(spec) + '/implementation-log.md' }}\"
initialize:
  stack:
    - action:
        - ensure_file: \"{{log}}\"
---
Implement phase {{phase}}.

::block when=\"file_exists(log)\"
::block when=\"phase > 1\"
::file {{log}}
::end-block
::end-block

::file {{log}}
";

#[test]
fn bootstrap_reads_the_lifecycle_surface_of_a_body_that_includes_a_missing_file() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("fixes/2026-09-14-demo")).unwrap();
    let source = source_at(dir.path(), GENERATED_INCLUDE_DOC);

    let staged = bootstrap(CompositionMode::ChainedDocument, &source, options_in(dir.path(), 1))
        .expect("the bootstrap read never dereferences the body's includes");

    assert_eq!(staged.entry, DocumentEntryReason::ProxyTarget);
    assert_eq!(staged.mode, CompositionMode::ChainedDocument);
    assert_eq!(staged.resolved_path, source.resolved_path);
    assert_eq!(staged.effective_frontmatter["log"], LOG);
    assert_eq!(staged.effective_frontmatter["phase"], 1);
    assert_eq!(
        staged.input_layers.set_overrides,
        Some(serde_json::json!({ "spec": SPEC, "phase": 1 })),
        "the caller layers are retained for the stabilized reread"
    );
    assert!(shell_commands(&staged.lifecycle, LifecycleSignal::Initialize).is_empty());
    assert!(
        staged.effective_frontmatter["initialize"].to_string().contains("{{log}}"),
        "the lifecycle subtree keeps its authored spans for event time"
    );
    assert!(staged.deferred_lifecycle_keys.contains(&"initialize".to_string()));
    assert!(!dir.path().join(LOG).exists(), "bootstrap runs no lifecycle effect");

    let err = full(CompositionMode::ChainedDocument, &source, options_in(dir.path(), 1))
        .expect_err("full preparation still dereferences the missing include");
    assert!(
        matches!(err, CompositionError::ComposeFailed(_)),
        "a missing include stays a compose failure, not suppressed; got {err:?}"
    );

    // Stand in for `initialize`: once the file exists, the full read the
    // stabilized reread performs composes it at both include sites.
    fs::write(dir.path().join(LOG), "LOG-CONTENT\n").unwrap();
    let prepared = full(CompositionMode::ChainedDocument, &source, options_in(dir.path(), 2))
        .expect("after initialize the same document prepares");
    assert_eq!(prepared.prompt.matches("LOG-CONTENT").count(), 2, "{}", prepared.prompt);
    assert!(!prepared.schema_verdict_deferred);
}

#[test]
fn inline_bootstrap_reads_the_lifecycle_surface_of_a_prompt_that_includes_a_missing_file() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("fixes/2026-09-14-demo")).unwrap();
    let source = source_at(
        dir.path(),
        "\
---
spec: \"\"
log: \"{{ dirname(spec) + '/implementation-log.md' }}\"
prompt: |-
  Summarize the log.

  ::file {{log}}
initialize:
  stack:
    - action:
        - ensure_file: \"{{log}}\"
---
Body the agent rewrites, including ::file ./not-yet-written.md

::file ./not-yet-written.md
",
    );

    let staged = bootstrap(
        CompositionMode::InlineFrontmatterPrompt,
        &source,
        options_in(dir.path(), 1),
    )
    .expect("inline bootstrap never composes the prompt's includes");
    assert_eq!(staged.mode, CompositionMode::InlineFrontmatterPrompt);
    assert_eq!(staged.effective_frontmatter["log"], LOG);
    assert!(staged.lifecycle.stack(LifecycleSignal::Initialize).is_some());

    let err = full(
        CompositionMode::InlineFrontmatterPrompt,
        &source,
        options_in(dir.path(), 1),
    )
    .expect_err("full inline preparation composes the prompt");
    assert!(matches!(err, CompositionError::ComposeFailed(_)), "got {err:?}");

    fs::write(dir.path().join(LOG), "LOG-CONTENT\n").unwrap();
    let prepared = full(
        CompositionMode::InlineFrontmatterPrompt,
        &source,
        options_in(dir.path(), 1),
    )
    .expect("after initialize the inline prompt composes");
    assert!(prepared.prompt.contains("LOG-CONTENT"), "{}", prepared.prompt);
}

/// A counting handler that allows every command it is asked about.
struct CountingHandler(Mutex<Vec<String>>);

impl ShellApprovalHandler for CountingHandler {
    fn approve(
        &self,
        request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.0.lock().unwrap().push(request.normalized_exact.clone());
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

fn approval_options(
    policy_root: &Path,
    handler: Option<Arc<dyn ShellApprovalHandler>>,
    cache: &Arc<Mutex<HashMap<String, crate::harness::shell::CachedApprovalDecision>>>,
) -> ShellApprovalOptions {
    ShellApprovalOptions {
        policy_root: Some(policy_root.to_path_buf()),
        approval_handler: handler,
        approval_cache: Arc::clone(cache),
        ..Default::default()
    }
}

#[test]
fn bootstrap_rejects_shell_actions_even_with_cached_approval() {
    let dir = TempDir::new().unwrap();
    for action in [
        serde_json::json!({"shell": "echo ready"}),
        serde_json::json!({"action": "shell", "command": "echo ready", "no_error": true}),
    ] {
        let source = source_at(dir.path(), &format!(
            "---\ninitialize:\n  stack:\n    - when: 'false'\n      action: {}\n---\n::file ./missing.md\n",
            action,
        ));
        let error = bootstrap(CompositionMode::ChainedDocument, &source, PrepareOptions {
            pre_approved_commands: Some(HashSet::from(["echo ready".to_string()])),
            ..PrepareOptions::default()
        }).unwrap_err();
        assert!(matches!(error, CompositionError::LifecycleActionPlacement { ref action, ref event, .. }
            if action == "shell" && event == "initialize"), "{error:?}");
    }
}

#[test]
fn bootstrap_frontmatter_shells_ignore_approval_handlers_and_cached_commands() {
    let dir = TempDir::new().unwrap();
    let policy = TempDir::new().unwrap();
    let source = source_at(dir.path(), "---\nstamp: \"$(echo ready)\"\n---\n::file ./missing.md\n");
    let handler = Arc::new(CountingHandler(Mutex::new(Vec::new())));
    let cache = Arc::new(Mutex::new(HashMap::new()));
    fs::write(policy.path().join(".darkmatter-shell-whitelist"), "prefix echo\n").unwrap();
    cache.lock().unwrap().insert("echo ready".to_string(), crate::harness::shell::CachedApprovalDecision::Allowed);
    let gate = approval_options(policy.path(), Some(handler.clone()), &cache);
    let options = PrepareOptions {
        pre_approved_commands: Some(HashSet::from(["echo ready".to_string()])),
        ..PrepareOptions::default()
    };
    let error = preflight_bootstrap_shell(
        &source, CompositionMode::ChainedDocument, &options, &gate,
    ).unwrap_err();
    assert!(matches!(error, CompositionError::LifecycleInvalid { .. }), "{error:?}");
    assert!(handler.0.lock().unwrap().is_empty(), "prohibition must not request approval");
    let error = bootstrap(CompositionMode::ChainedDocument, &source, options).unwrap_err();
    assert!(matches!(error, CompositionError::ShellExpansionFailed { .. }), "{error:?}");
}

/// A document the bootstrap read cannot judge fails with the same typed error
/// full preparation raises — the bootstrap is not a laxer parser.
#[test]
fn a_malformed_lifecycle_fails_bootstrap_and_full_preparation_identically() {
    let dir = TempDir::new().unwrap();
    let say_conflict = source_at(
        dir.path(),
        "---\nstart:\n  say: Hello\n  say_first: Also hello\n---\nbody\n",
    );
    let late_binding = source_at(
        dir.path(),
        "---\nstart:\n  stack:\n    - action:\n        - shell: \"rm {{err.msg}}\"\n---\nbody\n",
    );

    for mode in [CompositionMode::ChainedDocument, CompositionMode::InlineFrontmatterPrompt] {
        let options = || PrepareOptions {
            set_overrides: Some(serde_json::json!({ "prompt": "do it" })),
            ..PrepareOptions::default()
        };

        let staged = bootstrap(mode, &say_conflict, options()).unwrap_err();
        let prepared = full(mode, &say_conflict, options()).unwrap_err();
        assert!(matches!(staged, CompositionError::LifecycleSayConflict(_)), "{staged:?}");
        assert!(matches!(prepared, CompositionError::LifecycleSayConflict(_)), "{prepared:?}");

        let staged = bootstrap(mode, &late_binding, options()).unwrap_err();
        let prepared = full(mode, &late_binding, options()).unwrap_err();
        for err in [staged, prepared] {
            match err {
                CompositionError::LifecycleShellResolution { property, raw, .. } => {
                    assert_eq!(property, "start.stack[0].action[0].command");
                    assert_eq!(raw, "rm {{err.msg}}");
                }
                other => panic!("expected LifecycleShellResolution, got {other:?}"),
            }
        }
    }
}

/// For a body-independent document, bootstrap and full preparation agree on
/// everything the bootstrap carries.
#[test]
fn bootstrap_and_full_preparation_agree_on_a_body_independent_document() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("fixes/2026-09-14-demo")).unwrap();
    let source = source_at(
        dir.path(),
        "\
---
agent: codex
model: gpt-5
interactive: false
spec: \"\"
phase: 1
enabled: \"true\"
$schema:
  enabled: boolean
log: \"{{ dirname(spec) + '/implementation-log.md' }}\"
prompt: \"Implement phase {{phase}}\"
initialize:
  stack:
    - when: \"phase > 1\"
      action:
        - ensure_file: \"{{log}}\"
success:
  message: \"done {{phase}}\"
---
Implement phase {{phase}}.
",
    );

    for mode in [CompositionMode::ChainedDocument, CompositionMode::InlineFrontmatterPrompt] {
        let staged = bootstrap(mode, &source, options_in(dir.path(), 3)).unwrap();
        let prepared = full(mode, &source, options_in(dir.path(), 3)).unwrap();

        assert_eq!(staged.effective_frontmatter, prepared.effective_frontmatter, "{mode:?}");
        assert_eq!(staged.effective_frontmatter["enabled"], true, "declared types coerce");
        assert_eq!(staged.effective_frontmatter["phase"], 3);
        assert_eq!(staged.selection_hints, prepared.selection_hints);
        assert_eq!(staged.lifecycle, prepared.lifecycle);
        assert!(shell_commands(&staged.lifecycle, LifecycleSignal::Initialize).is_empty());
        assert_eq!(staged.deferred_lifecycle_keys, prepared.deferred_lifecycle_keys);
        assert_eq!(staged.resolved_path, prepared.resolved_path);
        assert_eq!(staged.source_repo_root, prepared.source_repo_root);
        assert_eq!(staged.input_layers.set_overrides, prepared.input_layers.set_overrides);
    }
}

/// The bootstrap never judges the schema, even when the caller's options ask
/// for a verdict; the stabilized reread owns it.
#[test]
fn bootstrap_always_withholds_the_schema_verdict() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "---\n$schema:\n  count: 'number(required)'\n---\nbody\n",
    );
    let options = || PrepareOptions {
        defer_schema_verdict: false,
        ..PrepareOptions::default()
    };

    bootstrap(CompositionMode::ChainedDocument, &source, options())
        .expect("`initialize` may still supply `count`");
    let err = full(CompositionMode::ChainedDocument, &source, options()).unwrap_err();
    assert!(matches!(err, CompositionError::MissingProperties { .. }), "{err:?}");
}

/// The bootstrap read is attributed to its epoch as an effective-frontmatter
/// consumer only: it composes no body.
#[test]
fn bootstrap_observes_the_prepared_context_as_frontmatter_only() {
    let dir = TempDir::new().unwrap();
    let source = source_at(
        dir.path(),
        "---\nos: \"{{ ctx.os }}\"\n---\n{{ ctx.os }}\n\n::file ./missing.md\n",
    );
    let invocation = crate::invocation_context::InvocationContext::capture_at(dir.path());
    let requirements =
        darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
    let document_epoch = invocation.begin_document_epoch();
    let context = document_epoch.capture_launch_context(&requirements);

    let staged = bootstrap(
        CompositionMode::ChainedDocument,
        &source,
        PrepareOptions {
            invocation_context: Some(invocation.clone()),
            document_epoch: Some(document_epoch),
            prepared_context: Some(context),
            ..PrepareOptions::default()
        },
    )
    .unwrap();

    assert_eq!(
        staged.document_epoch.unwrap().work_snapshot(),
        crate::invocation_context::DocumentEpochWork {
            launch_context_constructions: 1,
            launch_context_extensions: 0,
            ambient_fallbacks: 0,
            prepared_context_consumers: std::collections::BTreeMap::from([(
                "effective-frontmatter".to_string(),
                1
            )]),
        }
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "only an entry that emits `initialize`")]
fn a_retry_never_takes_a_bootstrap_read() {
    let dir = TempDir::new().unwrap();
    let source = source_at(dir.path(), "---\ntitle: x\n---\nbody\n");
    let _ = prepare_bootstrap(BootstrapRequest {
        entry: DocumentEntryReason::Retry,
        mode: CompositionMode::ChainedDocument,
        source: &source,
        options: PrepareOptions::default(),
    });
}

#[test]
fn bootstrap_overrides_cannot_enable_shell_expansion() {
    let dir = TempDir::new().unwrap();
    let source = source_at(dir.path(), "---\ninitialize: {}\n---\n::file ./missing.md\n");
    let error = bootstrap(CompositionMode::ChainedDocument, &source, PrepareOptions {
        set_overrides: Some(serde_json::json!({"stamp": "$(echo ready)"})),
        pre_approved_commands: Some(HashSet::from(["echo ready".to_string()])),
        ..PrepareOptions::default()
    }).unwrap_err();
    assert!(matches!(error, CompositionError::ShellExpansionFailed { .. }), "{error:?}");
}
