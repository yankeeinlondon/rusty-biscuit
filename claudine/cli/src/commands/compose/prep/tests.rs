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

    let report =
        resolve_composition_source(&file, CompositionKind::Direct, &shared_args()).unwrap_err();

    assert_frontmatter_enriched(report);
}

#[test]
fn inline_compose_source_load_error_is_frontmatter_enriched() {
    let dir = TempDir::new().unwrap();
    let file = malformed_prompt(&dir);

    let report =
        resolve_composition_source(&file, CompositionKind::Inline, &shared_args()).unwrap_err();

    assert_frontmatter_enriched(report);
}
