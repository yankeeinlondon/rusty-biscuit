use super::*;
use std::fs;
use tempfile::TempDir;

fn shared_args() -> SharedComposeArgs {
    SharedComposeArgs {
        provider: None,
        claude: false,
        codex: false,
        gemini: false,
        goose: false,
        kimicode: false,
        opencode: false,
        qwen: false,
        exclude: Vec::new(),
        yolo: false,
        interactive: false,
        no_interactive: false,
        include: Vec::new(),
        model: None,
        output: None,
        append_system_prompt: None,
        replace_system_prompt: None,
        timeout: None,
        step_timeout: None,
        stall_timeout: None,
        operation: None,
        sandbox: false,
        repo: false,
        dry_run: false,
        quiet: false,
        silent: true,
        set: None,
        mcp: false,
        mcp_use: Vec::new(),
        strict: false,
        perf: false,
        max_iterations: None,
        on_rate_limit: None,
        provider_args: Vec::new(),
        provider_args_explicit: false,
    }
}

fn malformed_prompt(dir: &TempDir) -> String {
    let file = dir.path().join("malformed.md");
    fs::write(
        &file,
        "----\nname: malformed\ndescription: near-miss fence\n----\n# Body\n",
    )
    .unwrap();
    file.to_string_lossy().into_owned()
}

fn assert_frontmatter_enriched(report: color_eyre::Report) {
    let err = report
        .downcast::<CompositionError>()
        .expect("report should carry CompositionError");
    match err {
        CompositionError::WithFrontmatter { inner, .. } => {
            assert!(
                matches!(*inner, CompositionError::FrontmatterParse(_)),
                "inner error should remain FrontmatterParse"
            );
        }
        other => panic!("expected WithFrontmatter, got: {other:?}"),
    }
}

#[test]
fn compose_source_load_error_is_frontmatter_enriched() {
    let dir = TempDir::new().unwrap();
    let file = malformed_prompt(&dir);
    let file_resolution_context = biscuit_file::FileResolutionContext::new(dir.path());

    let report = resolve_composition_source(
        &file,
        CompositionKind::Direct,
        &shared_args(),
        &file_resolution_context,
    )
    .unwrap_err();

    assert_frontmatter_enriched(report);
}

#[test]
fn inline_compose_source_load_error_is_frontmatter_enriched() {
    let dir = TempDir::new().unwrap();
    let file = malformed_prompt(&dir);
    let file_resolution_context = biscuit_file::FileResolutionContext::new(dir.path());

    let report = resolve_composition_source(
        &file,
        CompositionKind::Inline,
        &shared_args(),
        &file_resolution_context,
    )
    .unwrap_err();

    assert_frontmatter_enriched(report);
}

#[test]
fn document_epoch_reuses_one_target_adjusted_launch_snapshot() {
    let dir = TempDir::new().unwrap();
    let source_path = dir.path().join("prompt.md");
    fs::write(
        &source_path,
        "---\nidentity: '{{ ctx.agent }}/{{ ctx.model }}'\nloop:\n  until: \"identity == 'codex/gpt-epoch'\"\nsuccess:\n  warn: '{{ ctx.agent }}/{{ ctx.model }}'\n---\n{{ ctx.agent }}/{{ ctx.model }}\n",
    )
    .unwrap();
    let source = claudine::composition::resolve_composition_source(
        source_path.to_str().unwrap(),
    )
    .unwrap();
    let invocation = InvocationContext::capture_at(dir.path());
    let mut overrides = BTreeMap::new();
    overrides.insert("AGENT".to_string(), "codex".to_string());
    overrides.insert("MODEL".to_string(), "gpt-epoch".to_string());

    let (mut epoch, mut requirements) =
        capture_document_epoch_context(&invocation, &source, &overrides);
    assert!(!requirements.is_empty());
    assert_eq!(invocation.work_snapshot().launch_context_constructions, 1);

    let compose_options = darkmatter::markdown::compose::ComposeOptions::new_with_context(
        epoch.clone(),
    )
    .with_source_file(&source_path);
    let preflight = resolve_epoch_shell_approvals(
        &invocation,
        &epoch,
        &source.markdown,
        &compose_options,
        &claudine::harness::ShellApprovalOptions::default(),
    )
    .unwrap();
    assert_eq!(preflight.total_discovered, 0);

    let prepared = prepare_staged_for_epoch(
        CompositionKind::Direct,
        &invocation,
        &epoch,
        &source,
        PrepareOptions {
            invocation_context: Some(invocation.clone()),
            env_overrides: overrides,
            prepared_context: Some(epoch.clone()),
            file_resolution_context: Some(
                invocation
                    .derive_source(&source_path)
                    .unwrap()
                    .file_resolution_context()
                    .clone(),
            ),
            ..PrepareOptions::default()
        },
        claudine::composition::DocumentEntryReason::Direct,
        claudine::composition::SchemaStage::Validate,
    )
    .unwrap();
    assert_eq!(prepared.prompt.trim(), "codex/gpt-epoch");
    assert_eq!(
        prepared
            .effective_frontmatter
            .get("identity")
            .and_then(|v| v.as_str()),
        Some("codex/gpt-epoch")
    );

    let expanded = darkmatter::markdown::compose::ContextRequirements::for_content(
        "{{ ctx.os }}",
    );
    invocation.extend_launch_context(&mut epoch, &mut requirements, &expanded);
    assert!(epoch.as_object().get("os").is_some());

    let loop_config = resolve_epoch_loop_config(&invocation, &epoch, &source).unwrap();
    assert!(loop_config.is_some());

    let lifecycle = crate::commands::wrap::composition::prepared_lifecycle_context(
        Some(&epoch),
        Some(&invocation),
        dir.path(),
        "",
    );
    assert_eq!(
        lifecycle.as_object().get("agent").and_then(|v| v.as_str()),
        Some("codex")
    );
    assert_eq!(
        lifecycle.as_object().get("model").and_then(|v| v.as_str()),
        Some("gpt-epoch")
    );

    let work = invocation.work_snapshot();
    assert_eq!(work.launch_context_constructions, 1);
    assert_eq!(work.launch_context_extensions, 1);
    assert_eq!(work.ambient_fallbacks, 0);
    assert_eq!(work.prepared_context_preflight_observations, 1);
    assert_eq!(work.prepared_context_body_frontmatter_observations, 1);
    assert_eq!(work.prepared_context_loop_observations, 1);
    assert_eq!(work.prepared_context_lifecycle_observations, 1);
}
