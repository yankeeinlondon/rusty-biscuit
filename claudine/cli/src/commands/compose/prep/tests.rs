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
        "---\nidentity: '{{ ctx.agent }}/{{ ctx.model }}'\nsuccess:\n  warn: '{{ ctx.agent }}/{{ ctx.model }}'\n---\n{{ ctx.agent }}/{{ ctx.model }}\n",
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

    let (epoch, requirements) =
        capture_document_epoch_context(&invocation, &source, &overrides);
    assert!(!requirements.is_empty());
    assert_eq!(invocation.work_snapshot().launch_context_constructions, 1);

    let preflight = epoch.as_object();
    assert_eq!(preflight.get("agent").and_then(|v| v.as_str()), Some("codex"));
    assert_eq!(preflight.get("model").and_then(|v| v.as_str()), Some("gpt-epoch"));

    let prepared = claudine::composition::prepare_direct_with_schema(
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
    let lifecycle = prepared.compose_context.as_object();
    assert_eq!(lifecycle.get("agent").and_then(|v| v.as_str()), Some("codex"));
    assert_eq!(lifecycle.get("model").and_then(|v| v.as_str()), Some("gpt-epoch"));

    let loop_iteration = claudine::composition::prepare_direct_with_schema(
        &source,
        PrepareOptions {
            invocation_context: Some(invocation.clone()),
            env_overrides: BTreeMap::from([
                ("AGENT".to_string(), "codex".to_string()),
                ("MODEL".to_string(), "gpt-epoch".to_string()),
            ]),
            prepared_context: Some(epoch),
            file_resolution_context: prepared.input_layers.file_resolution_context.clone(),
            ..PrepareOptions::default()
        },
    )
    .unwrap();
    assert_eq!(loop_iteration.prompt.trim(), "codex/gpt-epoch");

    let work = invocation.work_snapshot();
    assert_eq!(work.launch_context_constructions, 1);
    assert_eq!(work.launch_context_extensions, 0);
    assert_eq!(work.ambient_fallbacks, 0);
}
