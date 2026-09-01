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

fn structured_no_match(
    dir: &TempDir,
    kind: CompositionKind,
    reference: &str,
) -> CompositionError {
    let context = biscuit_file::FileResolutionContext::new(dir.path())
        .with_repository_root(dir.path());
    resolve_composition_source(reference, kind, &shared_args(), &context)
        .unwrap_err()
        .downcast::<CompositionError>()
        .expect("report should carry CompositionError")
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
fn compose_and_inline_explicit_misses_keep_probe_evidence_and_suggestions() {
    use claudine::diagnostics::Diagnostic;

    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join(".git")).unwrap();
    let suggestion = dir.path().join("homelab/docs/unifi/access.md");
    fs::create_dir_all(suggestion.parent().unwrap()).unwrap();
    fs::write(&suggestion, "---\nprompt: test\n---\nbody\n").unwrap();

    for kind in [CompositionKind::Direct, CompositionKind::Inline] {
        let err = structured_no_match(&dir, kind, "./docs/unifi/access.md");
        assert!(
            matches!(err, CompositionError::FileReferenceNoMatch { .. }),
            "explicit miss should retain the structured resolver result: {err:?}"
        );
        let detail = err.detail();
        assert_eq!(detail["reference"], "./docs/unifi/access.md");
        assert_eq!(
            detail["base_dir"],
            biscuit_file::to_portable_string(dir.path())
        );
        assert_eq!(detail["failure"], "no_match");
        assert_eq!(
            detail["suggestions"],
            serde_json::json!(["homelab/docs/unifi/access.md"])
        );
        let candidates = detail["candidates"].as_array().unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0]["provenance"], "source");
        assert_eq!(candidates[0]["disposition"], "missing");
        assert!(
            !err.to_string().contains("autocomplete"),
            "explicit miss should not surface picker errors: {err}"
        );
    }
}

#[test]
fn compose_and_inline_bare_misses_still_attempt_autocomplete() {
    let dir = TempDir::new().unwrap();
    for kind in [CompositionKind::Direct, CompositionKind::Inline] {
        for reference in ["access", "access.md"] {
            let err = structured_no_match(&dir, kind, reference);
            assert!(
                matches!(err, CompositionError::AutocompleteNotInteractive),
                "bare miss should reach the existing picker gate: {err:?}"
            );
        }
    }
}
