use super::*;
use std::collections::HashMap;
use std::error::Error as _;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use crate::diagnostics::Diagnostic;
use tempfile::TempDir;

#[test]
fn prompt_magic_roots_are_closest_first() {
    // Only Claudine conventions are registered. The package, area,
    // local-root, and home roots are supplied by biscuit-file's intrinsic
    // `@` chain and must not be duplicated here.
    let area = Path::new("/repo/claudine");
    let package = Path::new("/repo/claudine/lib");
    let local_root = Path::new("/repo");
    let home = Path::new("/home/u");
    let got = prompt_magic_roots(local_root, Some(area), Some(package), Some(home));
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
    // No package area, no HOME: only the local-tree roots are registered,
    // and none of them is a user row.
    let got = prompt_magic_roots(Path::new("/repo"), None, None, None);
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
}

#[test]
fn prompt_magic_roots_without_repository_register_launch_conventions() {
    // R5: outside a repository the launch directory is the local root, so it
    // registers the same convention rows a repository would.
    let home = Path::new("/home/u");
    let got = prompt_magic_roots(Path::new("/home/u/scratch"), None, None, Some(home));
    assert_eq!(
        got,
        vec![
            PathBuf::from("/home/u/scratch/prompts"),
            PathBuf::from("/home/u/scratch/.claudine/prompts"),
            PathBuf::from("/home/u/scratch/docs"),
            PathBuf::from("/home/u/scratch/.claude/skills"),
            PathBuf::from("/home/u/scratch/.codex/skills"),
            PathBuf::from("/home/u/scratch/.gemini/skills"),
            PathBuf::from("/home/u/scratch/.opencode/skills"),
            PathBuf::from("/home/u/scratch/.goose/skills"),
            PathBuf::from("/home/u/scratch/.qwen/skills"),
            PathBuf::from("/home/u/scratch/.kimi/skills"),
            PathBuf::from("/home/u/.claudine/prompts"),
        ],
    );
}

#[test]
fn prompt_magic_fallback_roots_are_local_then_home_claudine() {
    let home = Path::new("/home/u");
    assert_eq!(
        prompt_magic_fallback_roots(Path::new("/repo"), Some(home)),
        vec![PathBuf::from("/repo/.claudine"), PathBuf::from("/home/u/.claudine")],
    );
    // Without a repository the launch directory registers the local row.
    assert_eq!(
        prompt_magic_fallback_roots(Path::new("/home/u/scratch"), Some(home)),
        vec![
            PathBuf::from("/home/u/scratch/.claudine"),
            PathBuf::from("/home/u/.claudine"),
        ],
    );
    assert_eq!(
        prompt_magic_fallback_roots(Path::new("/repo"), None),
        vec![PathBuf::from("/repo/.claudine")],
    );
}

#[test]
fn with_prompt_magic_roots_marks_the_claudine_home_rows_user_tier() {
    // Ruling 1: when the local root is `$HOME` itself, the `~/.claudine` rows
    // must be registered user-tier exactly once — never re-registered as an
    // inferred local twin, which the chain's local-first dedup would keep.
    let home = Path::new("/home/u");
    let context = with_prompt_magic_roots(
        FileResolutionContext::from_snapshot(home, Some(home.to_path_buf()), HashMap::new()),
        home,
        None,
        None,
        Some(home),
    );
    let registrations = context.magic_path_registrations();
    let claudine_rows: Vec<_> = registrations
        .iter()
        .filter(|registration| {
            registration.path() == home.join(".claudine")
                || registration.path() == home.join(".claudine").join("prompts")
        })
        .collect();
    assert_eq!(
        claudine_rows.len(),
        2,
        "each `.claudine` row must be registered exactly once: {registrations:?}",
    );
    assert!(
        claudine_rows
            .iter()
            .all(|registration| registration.tier() == biscuit_file::MagicPathTier::User),
        "every `.claudine` row under a launch-equals-home local root must be user-tier: {registrations:?}",
    );
}

#[test]
fn user_tier_prompt_row_stays_behind_local_files_when_launch_is_home() {
    // The observable side of ruling 1: launching in `$HOME` without a
    // repository keeps `~/.claudine/prompts/<x>` behind the launch tree's
    // own files, where containment inference alone would have promoted it.
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let local_file = home.join("x.md");
    let user_file = home.join(".claudine/prompts/x.md");
    fs::create_dir_all(user_file.parent().unwrap()).unwrap();
    fs::write(&local_file, "local").unwrap();
    fs::write(&user_file, "user").unwrap();
    let context = with_prompt_magic_roots(
        FileResolutionContext::from_snapshot(&home, Some(home.clone()), HashMap::new()),
        &home,
        None,
        None,
        Some(&home),
    );

    assert_eq!(
        FileReference::new("@x.md")
            .unwrap()
            .resolve_in_context(&context)
            .unwrap(),
        Some(local_file),
    );
}

#[test]
fn path_shaped_prompt_reference_keeps_closest_tier_first() {
    // `@prompts/x.md`: the repository's own `prompts/` precedes both
    // `.claudine` tiers, and the repo tier precedes the user tier.
    let fixture = TempDir::new().unwrap();
    let repo = fixture.path().join("home/config/sh");
    let home = fixture.path().join("home");
    let context = with_prompt_magic_roots(
        FileResolutionContext::from_snapshot(&repo, Some(home.clone()), HashMap::new())
            .with_repository_root(&repo),
        &repo,
        None,
        None,
        Some(&home),
    );

    let candidates = FileReference::new("@prompts/x.md")
        .unwrap()
        .candidate_plan(&context)
        .unwrap()
        .iter()
        .map(|candidate| candidate.path().to_path_buf())
        .collect::<Vec<_>>();
    let position = |path: PathBuf| {
        candidates
            .iter()
            .position(|candidate| *candidate == path)
            .unwrap_or_else(|| panic!("missing candidate {path:?} in {candidates:?}"))
    };

    let repo_prompts = position(repo.join("prompts/x.md"));
    let repo_claudine = position(repo.join(".claudine/prompts/x.md"));
    let user_claudine = position(home.join(".claudine/prompts/x.md"));
    assert!(repo_prompts < repo_claudine, "{candidates:?}");
    assert!(repo_claudine < user_claudine, "{candidates:?}");
}

#[test]
fn prompt_magic_candidates_interleave_conventions_and_intrinsic_scopes_once() {
    // Roots must be host-absolute for the scope catalog; none need to exist.
    let fixture = TempDir::new().unwrap();
    let repo = fixture.path().join("repo");
    let area = repo.join("claudine");
    let package = area.join("lib");
    let home = fixture.path().join("home");
    let catalog = biscuit_file::RepositoryScopeCatalog::new(
        repo.clone(),
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
    for root in prompt_magic_roots(&repo, Some(&area), Some(&package), Some(&home)) {
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
    assert!(matches!(
        err,
        CompositionError::FileReferenceNoMatch { .. }
    ));
}

#[test]
fn detailed_no_match_preserves_probe_order_and_diagnostic_shape() {
    let repo = TempDir::new().unwrap();
    let launch = repo.path().join("launch");
    fs::create_dir_all(&launch).unwrap();
    let context = FileResolutionContext::new(&launch).with_repository_root(repo.path());

    let err = resolve_composition_source_in_context("missing.md", &context).unwrap_err();
    let (reference, resolution, suggestions) = err.file_reference_no_match().unwrap();
    assert_eq!(reference, "missing.md");
    assert_eq!(resolution.reference(), "missing.md");
    assert_eq!(resolution.base_dir(), launch);
    assert_eq!(resolution.repository_root(), Some(repo.path()));
    assert!(suggestions.is_empty());

    let detail = err.detail();
    assert_eq!(detail["reference"], serde_json::json!("missing.md"));
    assert_eq!(detail["kind"], serde_json::json!("implicit_relative"));
    assert_eq!(detail["effective_kind"], serde_json::json!("implicit_relative"));
    assert_eq!(
        detail["base_dir"],
        serde_json::json!(biscuit_file::to_portable_string(&launch))
    );
    assert_eq!(
        detail["repository_root"],
        serde_json::json!(biscuit_file::to_portable_string(repo.path()))
    );
    assert_eq!(detail["failure"], serde_json::json!("no_match"));
    assert_eq!(detail["suggestions"], serde_json::json!([]));
    assert_eq!(detail["fallback_dir"], serde_json::Value::Null);
    assert_eq!(detail["source_path"], serde_json::Value::Null);
    assert_eq!(detail["property"], serde_json::Value::Null);
    assert_eq!(detail["event"], serde_json::Value::Null);

    let candidates = detail["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0]["provenance"], serde_json::json!("source"));
    assert_eq!(candidates[1]["provenance"], serde_json::json!("repository"));
    assert_eq!(candidates[0]["disposition"], serde_json::json!("missing"));
    assert_eq!(candidates[1]["disposition"], serde_json::json!("missing"));
}

#[test]
fn detailed_no_match_rendering_matches_suggestion_order_and_uses_portable_paths() {
    let repo = TempDir::new().unwrap();
    let launch = repo.path().join("windows\\style");
    fs::create_dir_all(&launch).unwrap();
    let context = FileResolutionContext::new(&launch).with_repository_root(repo.path());
    let err = resolve_composition_source_in_context("./missing.md", &context)
        .unwrap_err()
        .with_file_reference_suggestions(vec![
            "zeta\\missing.md".to_string(),
            "alpha/missing.md".to_string(),
        ]);

    let detail = err.detail();
    assert_eq!(
        detail["suggestions"],
        serde_json::json!(["zeta/missing.md", "alpha/missing.md"])
    );

    let term = Terminal::default();
    let rendered = err.status_block(&term).render(&term);
    assert!(rendered.contains("launch directory"), "{rendered}");
    assert!(!rendered.contains('\\'), "{rendered}");
    let first = rendered.find("zeta/missing.md").unwrap();
    let second = rendered.find("alpha/missing.md").unwrap();
    assert!(first < second, "{rendered}");
}

/// A plain, very wide terminal so rendered assertions match visible text,
/// not SGR bytes or word-wrapped lines.
fn plain_terminal() -> Terminal {
    Terminal::builder()
        .width(500)
        .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
        .build()
}

/// The reported failure layout (2026-09-23-local-before-home): a plain
/// repository nested under `$HOME`, referencing the missing `@prompts/<x>`.
fn staged_repo_under_home() -> (TempDir, PathBuf, PathBuf) {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let repo = home.join("config").join("sh");
    fs::create_dir_all(&repo).unwrap();
    (fixture, home, repo)
}

#[test]
fn magic_no_match_report_lists_search_roots_in_priority_order() {
    let (_fixture, home, repo) = staged_repo_under_home();
    let context = with_prompt_magic_roots(
        FileResolutionContext::from_snapshot(&repo, Some(home.clone()), HashMap::new())
            .with_repository_root(&repo),
        &repo,
        None,
        None,
        Some(&home),
    );
    let err = resolve_composition_source_in_context("@prompts/missing.md", &context).unwrap_err();

    let term = plain_terminal();
    let rendered = err.report_block_error(&term);
    assert!(
        rendered.contains("was not found under any directory an `@` reference searches:"),
        "got:\n{rendered}"
    );

    // The payload is named exactly once; every joined candidate path would
    // repeat it as a suffix, so one occurrence also proves none are listed.
    assert_eq!(
        rendered.matches("prompts/missing.md").count(),
        1,
        "got:\n{rendered}"
    );
    assert!(!rendered.contains("@prompts/missing.md"), "got:\n{rendered}");

    // No provenance labels in the `@` branch.
    for label in ["magic:", "repository:", "home:", "launch directory:"] {
        assert!(!rendered.contains(label), "label `{label}` leaked: got:\n{rendered}");
    }

    // Local roots precede every home root, and only configured roots carry
    // the `(*)` marker with its footnote.
    let positions: Vec<(usize, &str)> = [
        repo.join(".claude").join("skills"),
        repo.clone(),
        repo.join(".claudine"),
        home.join(".claudine").join("prompts"),
        home.clone(),
        home.join(".claudine"),
    ]
    .iter()
    .map(|path| {
        let line = format!("- `{}`", biscuit_file::to_portable_string(path));
        let position = rendered.find(&line).unwrap_or_else(|| {
            panic!("root line `{line}` missing from:\n{rendered}");
        });
        (position, "")
    })
    .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "roots out of priority order:\n{rendered}"
    );

    let (_, resolution, _) = err.file_reference_no_match().unwrap();
    let configured = resolution
        .magic_search_roots()
        .iter()
        .filter(|root| root.provenance() == biscuit_file::RootProvenance::Magic)
        .count();
    assert!(configured > 0);
    assert_eq!(
        rendered.matches("(*)").count(),
        configured + 1, // + the footnote line
        "got:\n{rendered}"
    );
    assert!(
        rendered.contains("(*) searched in addition to the standard `@` roots, for this context"),
        "got:\n{rendered}"
    );

    // The structured record keeps concrete candidates with dispositions and
    // provenance for tools that inspect it.
    let detail = err.detail();
    let candidates = detail["candidates"].as_array().unwrap();
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|candidate| candidate["path"].is_string()));
    assert!(candidates.iter().all(|candidate| candidate["disposition"] == "missing"));
    assert!(candidates.iter().any(|candidate| candidate["provenance"] == "magic"));
}

#[test]
fn magic_no_match_without_repository_lists_the_launch_directory_without_marker() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let scratch = home.join("scratch");
    fs::create_dir_all(&scratch).unwrap();
    let context = with_prompt_magic_roots(
        FileResolutionContext::from_snapshot(&scratch, Some(home.clone()), HashMap::new()),
        &scratch,
        None,
        None,
        Some(&home),
    );
    let err = resolve_composition_source_in_context("@missing.md", &context).unwrap_err();

    let term = plain_terminal();
    let rendered = err.report_block_error(&term);

    // The request directory is the intrinsic local root: listed, first, and
    // unmarked, unlike the configured convention rows under it.
    let launch_line = format!("- `{}`", biscuit_file::to_portable_string(&scratch));
    assert!(rendered.contains(&launch_line), "got:\n{rendered}");
    let launch_position = rendered.find(&launch_line).unwrap();
    let home_line = format!("- `{}`", biscuit_file::to_portable_string(&home));
    let home_position = rendered.find(&home_line).unwrap();
    assert!(launch_position < home_position, "got:\n{rendered}");
    let after_launch = &rendered[launch_position + launch_line.len()..];
    assert!(
        !after_launch.starts_with(" (*)"),
        "intrinsic local root must stay unmarked:\n{rendered}"
    );

    // The no-repository local root reports its own machine provenance.
    let detail = err.detail();
    let candidates = detail["candidates"].as_array().unwrap();
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate["provenance"] == "local_root"),
        "got: {candidates:?}"
    );
}

#[test]
fn bare_no_match_keeps_the_candidate_report() {
    let (_fixture, home, repo) = staged_repo_under_home();
    let context = with_prompt_magic_roots(
        FileResolutionContext::from_snapshot(&repo, Some(home.clone()), HashMap::new())
            .with_repository_root(&repo),
        &repo,
        None,
        None,
        Some(&home),
    );
    let err = resolve_composition_source_in_context("missing.md", &context).unwrap_err();

    let term = plain_terminal();
    let rendered = err.report_block_error(&term);
    assert!(
        rendered.contains("Cannot resolve `missing.md` from launch directory"),
        "got:\n{rendered}"
    );
    assert!(rendered.contains("Tried:"), "got:\n{rendered}");
    assert!(
        !rendered.contains("was not found under any directory"),
        "bare references keep their report; got:\n{rendered}"
    );

    let (_, resolution, _) = err.file_reference_no_match().unwrap();
    assert!(resolution.magic_search_roots().is_empty());
}

#[test]
fn detailed_resolution_preserves_non_no_match_typed_errors() {
    let context = FileResolutionContext::new("/tmp");
    let err = resolve_composition_source_in_context("https://example.com/prompt.md", &context)
        .unwrap_err();
    match err {
        CompositionError::InvalidReference { source, .. } => {
            assert!(matches!(
                source,
                biscuit_file::FileReferenceError::RemoteNotLocal(_)
            ));
        }
        other => panic!("expected typed InvalidReference, got: {other:?}"),
    }
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
        !matches!(
            err,
            CompositionError::FileNotFound(_)
                | CompositionError::FileReferenceNoMatch { .. }
        ),
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

/// Acceptance criterion #5: the shipped `prompts/_reviews/cross-platform.md` prompt
/// (already fixed to `---` fences) loads as a composition source with
/// non-empty frontmatter and a body that begins with the real heading. No
/// YAML keys from the frontmatter may leak into the body.
#[test]
fn cross_platform_prompt_composes_cleanly() {
    let manifest_dir = biscuit_test_harness::manifest_dir!();
    let workspace_root = manifest_dir
        .parent()
        .expect("claudine/lib parent")
        .parent()
        .expect("workspace root");
    let path = workspace_root.join("prompts/_reviews/cross-platform.md");

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
