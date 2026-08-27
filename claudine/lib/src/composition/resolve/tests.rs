use super::*;
use std::collections::HashMap;
use std::error::Error as _;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn prompt_magic_roots_are_closest_first() {
    // Only Claudine conventions are registered. The package, area,
    // repository, and home roots are supplied by biscuit-file's intrinsic
    // `@` chain and must not be duplicated here.
    let area = Path::new("/repo/claudine");
    let package = Path::new("/repo/claudine/lib");
    let repo = Path::new("/repo");
    let home = Path::new("/home/u");
    let got = prompt_magic_roots(Some(repo), Some(area), Some(package), Some(home));
    assert_eq!(
        got,
        vec![
            PathBuf::from("/repo/claudine/lib/prompts"),
            PathBuf::from("/repo/claudine/prompts"),
            PathBuf::from("/repo/prompts"),
            PathBuf::from("/repo/.claudine/prompts"),
            PathBuf::from("/repo/docs"),
            PathBuf::from("/repo/.claude/skills"),
            PathBuf::from("/repo/.codex/skills"),
            PathBuf::from("/repo/.gemini/skills"),
            PathBuf::from("/repo/.opencode/skills"),
            PathBuf::from("/repo/.goose/skills"),
            PathBuf::from("/repo/.qwen/skills"),
            PathBuf::from("/repo/.kimi/skills"),
            PathBuf::from("/home/u/.claudine/prompts"),
        ],
    );
}

#[test]
fn prompt_magic_roots_skip_absent_anchors() {
    // No package area, no HOME: only the repo roots are registered.
    let got = prompt_magic_roots(Some(Path::new("/repo")), None, None, None);
    assert_eq!(
        got,
        vec![
            PathBuf::from("/repo/prompts"),
            PathBuf::from("/repo/.claudine/prompts"),
            PathBuf::from("/repo/docs"),
            PathBuf::from("/repo/.claude/skills"),
            PathBuf::from("/repo/.codex/skills"),
            PathBuf::from("/repo/.gemini/skills"),
            PathBuf::from("/repo/.opencode/skills"),
            PathBuf::from("/repo/.goose/skills"),
            PathBuf::from("/repo/.qwen/skills"),
            PathBuf::from("/repo/.kimi/skills"),
        ],
    );
    assert!(prompt_magic_roots(None, None, None, None).is_empty());
}

#[test]
fn prompt_magic_candidates_interleave_conventions_and_intrinsic_scopes_once() {
    let repo = Path::new("/repo");
    let area = Path::new("/repo/claudine");
    let package = Path::new("/repo/claudine/lib");
    let home = Path::new("/home/u");
    let catalog = biscuit_file::RepositoryScopeCatalog::new(
        repo,
        vec![area.to_path_buf()],
        vec![package.to_path_buf()],
        biscuit_file::PackageAreaFallback::FirstComponent,
    )
    .unwrap();
    let mut context = FileResolutionContext::from_snapshot(
        package.join("src"),
        Some(home.to_path_buf()),
        HashMap::new(),
    )
    .with_repository_scope_catalog(catalog);
    for root in prompt_magic_roots(Some(repo), Some(area), Some(package), Some(home)) {
        context = context.add_magic_path(root, PathPosition::Start);
    }

    let candidates = FileReference::new("@shared.md")
        .unwrap()
        .candidate_plan(&context)
        .unwrap()
        .iter()
        .map(|candidate| candidate.path().to_path_buf())
        .collect::<Vec<_>>();

    assert_eq!(candidates[0], package.join("prompts/shared.md"));
    assert_eq!(candidates[1], area.join("prompts/shared.md"));
    let package_intrinsic = package.join("shared.md");
    let area_intrinsic = area.join("shared.md");
    let repo_intrinsic = repo.join("shared.md");
    let home_intrinsic = home.join("shared.md");
    for intrinsic in [
        package_intrinsic,
        area_intrinsic,
        repo_intrinsic,
        home_intrinsic,
    ] {
        assert_eq!(
            candidates.iter().filter(|candidate| **candidate == intrinsic).count(),
            1,
            "intrinsic root must occur exactly once: {intrinsic:?}"
        );
    }
}

#[test]
fn skill_reference_prefers_repository_then_falls_back_to_home() {
    let fixture = TempDir::new().unwrap();
    let repo = fixture.path().join("repo");
    let home = fixture.path().join("home");
    let repo_skill = repo.join(".claude/skills/name/SKILL.md");
    let home_skill = home.join(".claude/skills/name/SKILL.md");
    fs::create_dir_all(repo_skill.parent().unwrap()).unwrap();
    fs::create_dir_all(home_skill.parent().unwrap()).unwrap();
    fs::write(&repo_skill, "repo skill").unwrap();
    fs::write(&home_skill, "home skill").unwrap();
    let catalog = biscuit_file::RepositoryScopeCatalog::new(
        &repo,
        Vec::new(),
        Vec::new(),
        biscuit_file::PackageAreaFallback::None,
    )
    .unwrap();
    let context = FileResolutionContext::from_snapshot(
        &repo,
        Some(home.clone()),
        HashMap::new(),
    )
    .with_repository_scope_catalog(catalog);
    let reference = FileReference::new("@.claude/skills/name/SKILL.md").unwrap();

    assert_eq!(reference.resolve_in_context(&context).unwrap(), Some(repo_skill.clone()));
    fs::remove_file(repo_skill).unwrap();
    assert_eq!(reference.resolve_in_context(&context).unwrap(), Some(home_skill));
}

#[test]
fn markdown_load_read_cause_is_recoverable() {
    let err = CompositionError::MarkdownLoad {
        path: PathBuf::from("/tmp/whatever.md"),
        source: MarkdownLoadCause::Read(io::Error::other("boom")),
    };

    // The typed source walks to the sub-enum; the transparent arm carries
    // the concrete io::Error, recoverable by matching the variant.
    let cause = err.source().expect("MarkdownLoad must carry a source");
    let load_cause = cause
        .downcast_ref::<MarkdownLoadCause>()
        .expect("source must be a MarkdownLoadCause");
    let io_err = match load_cause {
        MarkdownLoadCause::Read(io_err) => io_err,
        other => panic!("expected Read cause, got: {other:?}"),
    };
    assert_eq!(io_err.kind(), ErrorKind::Other);
    assert_eq!(io_err.to_string(), "boom");
}

#[test]
fn markdown_load_parse_cause_round_trips() {
    // A non-frontmatter MarkdownError routed through map_load_error lands in
    // the MarkdownLoad::Parse arm with the typed MarkdownError reachable.
    let file = PathBuf::from("/tmp/whatever.md");
    let other = MarkdownError::AstParse("synthetic ast failure".to_string());
    let err = map_load_error(&file, other);
    match &err {
        CompositionError::MarkdownLoad {
            source: MarkdownLoadCause::Parse(_),
            ..
        } => {}
        other => panic!("expected MarkdownLoad::Parse, got: {other:?}"),
    }

    let load_cause = err
        .source()
        .and_then(|s| s.downcast_ref::<MarkdownLoadCause>())
        .expect("source must be a MarkdownLoadCause");
    let parsed = match load_cause {
        MarkdownLoadCause::Parse(md_err) => md_err,
        other => panic!("expected Parse cause, got: {other:?}"),
    };
    assert!(matches!(**parsed, MarkdownError::AstParse(_)));
}

#[test]
fn resolve_absolute_markdown_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    fs::write(&file, "---\ntitle: Test\n---\n# Hello").unwrap();

    let result = resolve_composition_source(file.to_str().unwrap()).unwrap();
    assert_eq!(result.resolved_path, file);
    assert_eq!(result.original_ref, file.to_str().unwrap());
    assert_eq!(result.original_text, "---\ntitle: Test\n---\n# Hello");

    let title: Option<String> = result.markdown.fm_get("title").unwrap();
    assert_eq!(title, Some("Test".to_string()));
}

#[test]
fn resolve_rejects_non_markdown() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.txt");
    fs::write(&file, "hello").unwrap();

    let err = resolve_composition_source(file.to_str().unwrap()).unwrap_err();
    assert!(matches!(err, CompositionError::NotMarkdown(_)));
}

#[test]
fn resolve_missing_file() {
    let err = resolve_composition_source("/nonexistent/path/test.md").unwrap_err();
    assert!(matches!(err, CompositionError::FileNotFound(_)));
}

#[test]
fn resolve_malformed_frontmatter_reports_parse_error_not_missing_prompt() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("metadata.md");
    // Block-scalar body indented 4 spaces on the first line, then 3 on a
    // later line — YAML closes the scalar early and chokes. Previously this
    // surfaced as a misleading `PromptPropertyMissing`.
    fs::write(
        &file,
        "---\nprompt: |-\n    First line sets indent to four.\n   Three spaces breaks it.\n---\n",
    )
    .unwrap();

    let err = resolve_composition_source(file.to_str().unwrap()).unwrap_err();
    assert!(
        matches!(err, CompositionError::FrontmatterParse(_)),
        "expected FrontmatterParse, got: {err:?}"
    );
}

#[test]
fn resolve_four_dash_fence_maps_to_frontmatter_parse() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("four-dash.md");
    fs::write(
        &file,
        "----\nname: cross-platform\ndescription: near-miss fence\n----\n# Body\n",
    )
    .unwrap();

    let err = resolve_composition_source(file.to_str().unwrap()).unwrap_err();
    assert!(
        matches!(err, CompositionError::FrontmatterParse(_)),
        "expected FrontmatterParse for ---- fence, got: {err:?}"
    );
    assert!(
        !matches!(err, CompositionError::MarkdownLoad { .. }),
        "must not fall back to MarkdownLoad: {err:?}"
    );
    assert!(
        !matches!(err, CompositionError::FileNotFound(_)),
        "must not report file not found: {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("----"),
        "error message should name the offending fence: {msg}"
    );
}

#[test]
fn load_error_enrichment_wraps_actual_four_dash_source() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("four-dash.md");
    fs::write(
        &file,
        "----\nname: cross-platform\ndescription: near-miss fence\n----\n# Body\n",
    )
    .unwrap();

    let err = resolve_composition_source(file.to_str().unwrap()).unwrap_err();
    let err = enrich_composition_source_load_error(file.to_str().unwrap(), err, true);

    match err {
        CompositionError::WithFrontmatter { inner, excerpt } => {
            assert!(
                matches!(*inner, CompositionError::FrontmatterParse(_)),
                "inner error should remain FrontmatterParse"
            );
            assert_eq!(excerpt.highlight_line(), Some(1));
        }
        other => panic!("expected WithFrontmatter, got: {other:?}"),
    }
}

#[test]
fn resolve_markdown_extension() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.markdown");
    fs::write(&file, "# Hello").unwrap();

    let result = resolve_composition_source(file.to_str().unwrap()).unwrap();
    assert_eq!(result.resolved_path, file);
}

#[test]
fn validate_permissions_writable_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    fs::write(&file, "# Hello").unwrap();
    assert!(validate_file_permissions(&file).is_ok());
}

#[test]
fn validate_permissions_readonly_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("readonly.md");
    fs::write(&file, "# Hello").unwrap();
    let original_permissions = fs::metadata(&file).unwrap().permissions();
    let mut perms = original_permissions.clone();
    perms.set_readonly(true);
    fs::set_permissions(&file, perms).unwrap();

    let err = validate_file_permissions(&file).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::InsufficientFilePermissions { .. }
    ));

    // Cleanup: restore permissions so TempDir can delete
    fs::set_permissions(&file, original_permissions).unwrap();
}

#[test]
fn validate_permissions_nonexistent_file() {
    let err = validate_file_permissions(Path::new("/nonexistent/path.md")).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::InsufficientFilePermissions { .. }
    ));
}

#[test]
fn is_markdown_path_variants() {
    assert!(is_markdown_path(Path::new("test.md")));
    assert!(is_markdown_path(Path::new("test.markdown")));
    assert!(is_markdown_path(Path::new("test.MD")));
    assert!(!is_markdown_path(Path::new("test.txt")));
    assert!(!is_markdown_path(Path::new("test")));
}

/// Acceptance criterion #5: the shipped `prompts/cross-platform.md` prompt
/// (already fixed to `---` fences) loads as a composition source with
/// non-empty frontmatter and a body that begins with the real heading. No
/// YAML keys from the frontmatter may leak into the body.
#[test]
fn cross_platform_prompt_composes_cleanly() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("claudine/lib parent")
        .parent()
        .expect("workspace root");
    let path = workspace_root.join("prompts/cross-platform.md");

    let source = resolve_composition_source(path.to_str().unwrap())
        .expect("cross-platform.md should resolve and parse cleanly");

    assert!(
        !source.markdown.frontmatter().is_empty(),
        "frontmatter should be parsed and non-empty"
    );
    let name: Option<String> = source.markdown.fm_get("name").unwrap();
    assert_eq!(name, Some("cross-platform".to_string()));

    let content = source.markdown.content();
    assert!(
        content.starts_with("# Ensuring Cross Platform Support"),
        "body should start with the real heading; got: {content}"
    );
    assert!(
        !content.contains("name: cross-platform"),
        "frontmatter YAML must not leak into body: {content}"
    );
    assert!(
        !content.contains("description:"),
        "frontmatter YAML must not leak into body: {content}"
    );
}
