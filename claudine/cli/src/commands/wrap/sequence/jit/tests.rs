//! Just-in-time composition unit coverage.
//!
//! The layering helpers are pure; the template-preflight options carry the
//! document-relative resolution contract a step's approved shell bytes depend on.

use super::{StepComposeContext, build_template_preflight_options, compose_step};
use std::collections::BTreeMap;

use darkmatter::markdown::Markdown;

use claudine::composition::resolve_shell_approvals;
use claudine::harness::ShellApprovalOptions;

#[test]
fn sequence_step_preparation_is_one_exact_document_epoch() {
    use clap::Parser;

    #[derive(Debug, clap::Parser)]
    struct Probe {
        #[command(flatten)]
        shared: crate::commands::compose::SharedComposeArgs,
    }

    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("step.md");
    std::fs::write(
        &source_path,
        "---\nprepared: '{{ ctx.os }}'\n---\nbody={{ ctx.os }}\n",
    )
    .unwrap();
    let invocation =
        claudine::invocation_context::InvocationContext::capture_at(directory.path());
    let source_context = invocation.derive_source(&source_path).unwrap();
    let source = claudine::composition::resolve_composition_source(
        source_path.to_string_lossy().as_ref(),
    )
    .unwrap();
    let shared = Probe::try_parse_from(["probe", "--dry-run"])
        .unwrap()
        .shared;
    let approval_cache =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let caller_input_records = BTreeMap::from([(
        "spec".to_string(),
        darkmatter::markdown::compose::CallerInputRecord::new(
            serde_json::json!("fixes/case/spec.md"),
            invocation.launch_file_resolution_context().clone(),
        ),
    )]);
    let context = StepComposeContext {
        source_repo_root: source_context.repository_root(),
        child_cwd: directory.path(),
        launch_area: Some(directory.path()),
        shared: &shared,
        approval_cache,
        inline_mode: false,
        file_resolution_context: source_context.file_resolution_context(),
        caller_input_records: &caller_input_records,
        invocation: &invocation,
    };
    let step = compose_step(
        &source,
        &context,
        &serde_json::json!({}),
        &BTreeMap::new(),
        std::collections::HashSet::new(),
        false,
    )
    .unwrap();

    assert!(step.prepared.prompt.contains("body="));
    assert_eq!(
        step.prepared.input_layers.caller_input_records,
        caller_input_records,
        "sequence preparation must retain only the immutable caller records"
    );
    assert_eq!(
        step.prepared.document_epoch.unwrap().work_snapshot(),
        claudine::invocation_context::DocumentEpochWork {
            launch_context_constructions: 1,
            launch_context_extensions: 0,
            ambient_fallbacks: 0,
            prepared_context_consumers: BTreeMap::from([
                ("body".to_string(), 1),
                ("effective-frontmatter".to_string(), 1),
                ("preflight".to_string(), 1),
            ]),
        },
        "one sequence step must prepare through one complete epoch"
    );
}

/// RAII guard that switches the process CWD and restores it on drop
/// (including on panic). Tests using it are serialized to avoid racing on
/// process-global CWD with other CWD-mutating tests.
struct CwdGuard {
    prior: std::path::PathBuf,
}

impl CwdGuard {
    fn enter(dir: &std::path::Path) -> Self {
        let prior = std::env::current_dir().expect("read CWD");
        std::env::set_current_dir(dir).expect("set CWD");
        Self { prior }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior);
    }
}

/// A sequence step with a `::shell` directive whose `{{ … }}` argument
/// depends on a read-side `file_exists` against a document-relative file
/// must see that file during the template SHELL preflight, resolving against
/// the document directory (`base_dir`) CWD-independently — exactly as the
/// per-step `pre_validate_schema` and the final prepare pass do. The
/// launch-area fallback is diagnostic-only and never a candidate (D2).
#[test]
#[serial_test::serial(preflight_cwd)]
fn template_preflight_resolves_against_document_dir() {
    let doc_dir = tempfile::TempDir::new().unwrap();
    let launch_dir = tempfile::TempDir::new().unwrap();
    let unrelated = tempfile::TempDir::new().unwrap();

    // `spec.md` lives under the prompt (document) directory — the base a
    // reference authored inside the document resolves against (D2).
    std::fs::write(doc_dir.path().join("spec.md"), "# Spec\n").unwrap();

    let source_path = doc_dir.path().join("prompt.md");
    std::fs::write(
        &source_path,
        "::shell echo {{ file_exists(spec) }}\n",
    )
    .unwrap();

    // Whitelist `echo` so preflight approves without an interactive handler.
    std::fs::write(
        doc_dir.path().join(".darkmatter-shell-whitelist"),
        "prefix echo\n",
    )
    .unwrap();

    let md = Markdown::try_from(source_path.as_path()).unwrap();
    let overrides = serde_json::json!({ "spec": "spec.md" });
    let approval_options = ShellApprovalOptions {
        policy_root: Some(doc_dir.path().to_path_buf()),
        approval_handler: None,
        ..Default::default()
    };

    // Switch ambient CWD elsewhere to prove resolution is independent of any
    // post-launch chdir.
    let _cwd = CwdGuard::enter(unrelated.path());

    let env_overrides: BTreeMap<String, String> = BTreeMap::new();
    let (opts, _, _) = build_template_preflight_options(
        &env_overrides,
        &source_path,
        &md,
        (&overrides, &Default::default()),
        Some(launch_dir.path()),
        None,
        None,
    );
    let result =
        resolve_shell_approvals(Some(&md), Some(&opts), &approval_options, None, None).unwrap();

    assert!(
        result.approved_commands.contains("echo true"),
        "template preflight must resolve file_exists(spec) against the document \
         dir, CWD-independently; approved: {:?}",
        result.approved_commands,
    );
}

/// A launch-only file is not a candidate for a reference authored by the
/// template, even when ambient CWD is unrelated.
#[test]
#[serial_test::serial(preflight_cwd)]
fn template_preflight_does_not_resolve_launch_only_file() {
    let doc_dir = tempfile::TempDir::new().unwrap();
    let launch_dir = tempfile::TempDir::new().unwrap();
    let unrelated = tempfile::TempDir::new().unwrap();

    std::fs::write(launch_dir.path().join("spec.md"), "# Spec\n").unwrap();

    let source_path = doc_dir.path().join("prompt.md");
    std::fs::write(
        &source_path,
        "::shell echo {{ file_exists(spec) }}\n",
    )
    .unwrap();
    std::fs::write(
        doc_dir.path().join(".darkmatter-shell-whitelist"),
        "prefix echo\n",
    )
    .unwrap();

    let md = Markdown::try_from(source_path.as_path()).unwrap();
    let overrides = serde_json::json!({ "spec": "spec.md" });
    let approval_options = ShellApprovalOptions {
        policy_root: Some(doc_dir.path().to_path_buf()),
        approval_handler: None,
        ..Default::default()
    };

    let _cwd = CwdGuard::enter(unrelated.path());

    let env_overrides: BTreeMap<String, String> = BTreeMap::new();
    let (opts, _, _) = build_template_preflight_options(
        &env_overrides,
        &source_path,
        &md,
        (&overrides, &Default::default()),
        None,
        None,
        None,
    );
    let result =
        resolve_shell_approvals(Some(&md), Some(&opts), &approval_options, None, None).unwrap();

    assert!(
        result.approved_commands.contains("echo false"),
        "launch-only spec.md is unreachable from the document context, so \
         file_exists(spec) resolves false; approved: {:?}",
        result.approved_commands,
    );
    assert!(
        !result.approved_commands.contains("echo true"),
        "document-backed resolution must not see the launch-area file; approved: {:?}",
        result.approved_commands,
    );
}

#[test]
fn distributed_step_keeps_launch_identity_and_source_schema_and_files() {
    let temp = tempfile::tempdir().unwrap();
    let launch_repo = temp.path().join("launch");
    let launch_dir = launch_repo.join("alpha/lib");
    let source_repo = temp.path().join("source");
    let source_dir = source_repo.join("nested");
    std::fs::create_dir_all(&launch_dir).unwrap();
    std::fs::create_dir_all(&source_dir).unwrap();
    for repo in [&launch_repo, &source_repo] {
        let status = std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(repo)
            .status()
            .unwrap();
        assert!(status.success());
    }
    std::fs::write(
        launch_repo.join("Cargo.toml"),
        "[workspace]\nmembers = [\"alpha/lib\", \"sibling\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    std::fs::write(
        launch_dir.join("Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(launch_repo.join("sibling")).unwrap();
    std::fs::write(
        launch_repo.join("sibling/Cargo.toml"),
        "[package]\nname = \"sibling\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(
        launch_dir.join("schema.yaml"),
        "launch_only: string(required)\n",
    )
    .unwrap();
    std::fs::write(launch_dir.join("fragment.md"), "LAUNCH-FRAGMENT\n").unwrap();
    std::fs::write(
        source_dir.join("schema.yaml"),
        "source_marker: string(required)\nspec: 'file(eager; required)'\n",
    )
    .unwrap();
    std::fs::write(source_dir.join("spec.md"), "SOURCE-SPEC\n").unwrap();
    std::fs::write(source_dir.join("fragment.md"), "SOURCE-FRAGMENT\n").unwrap();
    let source_path = source_dir.join("step.md");
    std::fs::write(
        &source_path,
        concat!(
            "---\n",
            "$schema: ./schema.yaml\n",
            "source_marker: source-owned\n",
            "spec: spec.md\n",
            "---\n",
            "REPO={{ ctx.repo_root }} AREA={{ ctx.area }} AGENT={{ ctx.agent }} ",
            "MODEL={{ ctx.model }} ENV={{ env.AGENT }}/{{ env.MODEL }} ",
            "FILE={{ file_exists(spec) }}\n",
            "SOURCE-BODY\n",
        ),
    )
    .unwrap();

    let invocation = claudine::invocation_context::InvocationContext::capture_at(&launch_dir);
    let source_context = invocation.derive_source(&source_path).unwrap();
    let source = claudine::composition::resolve_composition_source(
        source_path.to_string_lossy().as_ref(),
    )
    .unwrap();
    let pre = claudine::composition::pre_validate_schema(&source, None, Some(&launch_dir))
        .expect("the source-side schema must be selected");
    let env = BTreeMap::from([
        ("AGENT".to_string(), "codex".to_string()),
        ("MODEL".to_string(), "gpt-5".to_string()),
    ]);
    let (options, context, _) = build_template_preflight_options(
        &env,
        &pre.source.resolved_path,
        &pre.source.markdown,
        (&serde_json::json!({}), &Default::default()),
        Some(&launch_dir),
        Some(source_context.file_resolution_context()),
        Some(&invocation),
    );
    let (composed, _) = pre.source.markdown.compose_with(options).unwrap();
    let body = composed.content();

    assert!(body.contains("AREA=alpha AGENT=codex MODEL=gpt-5 ENV=codex/gpt-5 FILE=true"));
    assert!(body.contains(&biscuit_file::to_portable_string(&launch_repo)));
    assert!(body.contains("SOURCE-BODY"));
    assert!(!body.contains("LAUNCH-FRAGMENT"));
    assert_eq!(context.get("area").and_then(serde_json::Value::as_str), Some("alpha"));
    let effective = context.as_object();
    assert_eq!(effective.get("agent").and_then(serde_json::Value::as_str), Some("codex"));
    assert_eq!(effective.get("model").and_then(serde_json::Value::as_str), Some("gpt-5"));
    assert_eq!(source_context.repository_root(), Some(source_repo.as_path()));
}
// ---------------------------------------------------------------------------
// Layer precedence
// ---------------------------------------------------------------------------

mod layering {
    use claudine::composition::{RuntimeSnapshot, resolve_composition_source};
    use claudine::composition::sequence::resolve_sequence_plan;
    use serde_json::{Value, json};

    use super::super::{reserved_overlay, step_set_overrides};

    /// A two-step plan built from a real document, so the overlay under test is
    /// the one normalization actually produces.
    fn two_step_plan(dir: &std::path::Path) -> claudine::composition::SequencePlan {
        let path = dir.join("seq.md");
        std::fs::write(
            &path,
            "---\nphase: from-document\nsequence:\n  - alpha\n  - beta\n---\nBody.\n",
        )
        .unwrap();
        let source = resolve_composition_source(path.to_str().unwrap()).unwrap();
        resolve_sequence_plan(&source).unwrap().expect("declares a sequence")
    }

    fn snapshot(mutations: &[(&str, Value)], outputs: &[&str]) -> RuntimeSnapshot {
        RuntimeSnapshot {
            mutations: mutations
                .iter()
                .map(|(k, v)| ((*k).to_string(), v.clone()))
                .collect(),
            outputs: outputs.iter().map(|s| json!(*s)).collect(),
        }
    }

    /// The ratified order: user setters lose to accumulated mutations, and both
    /// lose to the reserved overlay.
    #[test]
    fn mutations_outrank_user_setters_and_the_overlay_outranks_both() {
        let dir = tempfile::TempDir::new().unwrap();
        let plan = two_step_plan(dir.path());
        let user = json!({ "phase": "from-user", "state": "hijacked", "only_user": "kept" });
        let runtime = snapshot(&[("phase", json!("from-mutation"))], &[]);

        let merged = step_set_overrides(&plan, 0, Some(&user), Some(&runtime));
        let map = merged.as_object().unwrap();

        assert_eq!(map["phase"], json!("from-mutation"));
        assert_eq!(map["only_user"], json!("kept"));
        assert_eq!(
            map["state"]["name"],
            json!("alpha"),
            "a user setter must not displace the reserved overlay"
        );
    }

    /// The accumulator is the runtime layer's, not an overlay placeholder: a
    /// sequence in progress must not have its `outputs` reset at every step.
    #[test]
    fn the_runtime_accumulator_survives_the_overlay() {
        let dir = tempfile::TempDir::new().unwrap();
        let plan = two_step_plan(dir.path());
        let runtime = snapshot(&[], &["first"]);

        let merged = step_set_overrides(&plan, 1, None, Some(&runtime));
        assert_eq!(merged["outputs"], json!(["first"]));
        assert!(
            !reserved_overlay(&plan, 1)
                .as_object()
                .unwrap()
                .contains_key("outputs"),
            "the overlay must not carry an `outputs` copy to clobber it with"
        );
    }

    /// No runtime cell — the validation pass and `--dry-run` — still yields an
    /// initialized empty accumulator rather than an undefined root.
    #[test]
    fn the_initial_view_has_an_empty_but_present_accumulator() {
        let dir = tempfile::TempDir::new().unwrap();
        let plan = two_step_plan(dir.path());

        let merged = step_set_overrides(&plan, 0, None, None);
        assert_eq!(merged["outputs"], json!([]));
    }

    /// Both neighbors render on an interior step; a boundary neighbor is `null`
    /// rather than absent, so `{{ previous }}` reads empty instead of unknown.
    #[test]
    fn the_overlay_carries_the_neighbor_states() {
        let dir = tempfile::TempDir::new().unwrap();
        let plan = two_step_plan(dir.path());

        let first = reserved_overlay(&plan, 0);
        assert!(first["previous"].is_null());
        assert_eq!(first["next"]["name"], json!("beta"));

        let last = reserved_overlay(&plan, 1);
        assert_eq!(last["previous"]["name"], json!("alpha"));
        assert!(last["next"].is_null());
    }
}
