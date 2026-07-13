//! Microbenchmark for the `system prompt` substage observed at ~437ms
//! in `claudine compose --perf` on the rusty-biscuit worktree.
//!
//! This test isolates each call inside `resolve_and_prepare_for_session`
//! and prints wall-clock timings so a regression hunt can attribute the
//! cost to a specific step (resolve / first compose pass / appendix
//! compose pass / Claude `apply_system_prompt` temp file write).
//!
//! The test is `#[ignore]`d by default — it's diagnostic, not a gate.

use claudine::system_prompt::{
    ResolvedSystemPrompt, LaunchContext, SystemPromptArgs, resolve_and_prepare_for_session,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use std::path::PathBuf;
use std::time::Instant;

fn worktree_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn launch_context_for_root(root: &std::path::Path) -> LaunchContext {
    LaunchContext {
        agent: None,
        cwd: root.to_path_buf(),
        repo_root: Some(root.to_path_buf()),
        package_area_root: Some(root.to_path_buf()),
        package_root: None,
    }
}

#[test]
#[ignore = "diagnostic perf bench; run with --ignored --nocapture"]
fn bench_system_prompt_resolution_cold_and_warm() {
    let root = worktree_root();
    if !root.join("system-prompt.md").is_file() {
        eprintln!("skip: no system-prompt.md at worktree root {:?}", root);
        return;
    }
    let ctx = launch_context_for_root(&root);
    let args = SystemPromptArgs::default();

    // First call: cold — pays for any one-time lazy initialisations
    // (Darkmatter parsers, syntect grammar tables, regex compiles, …).
    let t0 = Instant::now();
    let cold = resolve_and_prepare_for_session(&args, &ctx, true).unwrap();
    let cold_elapsed = t0.elapsed();
    eprintln!("cold resolve_and_prepare_for_session: {cold_elapsed:?}");

    // Second call in the same process: warm — pays only the actual
    // composition cost. The delta from cold isolates lazy init overhead.
    let t1 = Instant::now();
    let warm = resolve_and_prepare_for_session(&args, &ctx, true).unwrap();
    let warm_elapsed = t1.elapsed();
    eprintln!("warm resolve_and_prepare_for_session: {warm_elapsed:?}");

    // Third call to confirm warm path is stable.
    let t2 = Instant::now();
    let _ = resolve_and_prepare_for_session(&args, &ctx, true).unwrap();
    let warm2 = t2.elapsed();
    eprintln!("warm #2 resolve_and_prepare_for_session: {warm2:?}");

    match cold {
        ResolvedSystemPrompt::Ready(p) => {
            eprintln!(
                "composed length (cold): {} chars",
                p.composed_markdown.len()
            )
        }
        other => panic!("expected Ready, got {:?}", other),
    }
    match warm {
        ResolvedSystemPrompt::Ready(_) => {}
        other => panic!("expected Ready, got {:?}", other),
    }
}

#[test]
#[ignore = "diagnostic perf bench; run with --ignored --nocapture"]
fn bench_resolve_and_prepare_step_by_step() {
    use claudine::system_prompt::resolve;

    let root = worktree_root();
    let ctx = launch_context_for_root(&root);
    let args = SystemPromptArgs::default();

    for run in 1..=3 {
        let t0 = Instant::now();
        let resolved = resolve::resolve_system_prompt_source(&args, &ctx).unwrap();
        let t_resolve = t0.elapsed();
        let (source, raw_text) = resolved.expect("system-prompt.md should be discovered");
        eprintln!(
            "run #{run} resolve_system_prompt_source: {:?} (raw {} bytes)",
            t_resolve,
            raw_text.len()
        );

        // Replicate prepare_system_prompt: build Markdown + compose.
        let t1 = Instant::now();
        let md: Markdown = raw_text.as_str().into();
        let mut opts = ComposeOptions::new();
        if let Some(p) = match &source {
            claudine::system_prompt::SystemPromptSource::StandardDiscovered { path, .. }
            | claudine::system_prompt::SystemPromptSource::ExplicitFile { path, .. }
            | claudine::system_prompt::SystemPromptSource::NonInteractiveFile { path, .. } => {
                Some(path.as_path())
            }
            _ => None,
        } {
            opts = opts.with_source_file(p);
        }
        let (composed, _report) = md.compose_with(opts).unwrap();
        let _ = composed.content();
        let t_compose_sp = t1.elapsed();
        eprintln!("run #{run} compose system-prompt.md: {:?}", t_compose_sp);

        // Non-interactive candidates resolution (file probing).
        let t2 = Instant::now();
        let candidates = resolve::resolve_non_interactive_candidates(&ctx).unwrap();
        let t_candidates = t2.elapsed();
        eprintln!(
            "run #{run} resolve_non_interactive_candidates: {:?} ({} candidates)",
            t_candidates,
            candidates.len()
        );

        // Compose first candidate (mirrors prepare_non_interactive_appendix
        // returning early on first non-empty).
        let t3 = Instant::now();
        let (cand_source, cand_text) = candidates.into_iter().next().unwrap();
        let md: Markdown = cand_text.as_str().into();
        let mut opts = ComposeOptions::new();
        if let claudine::system_prompt::SystemPromptSource::NonInteractiveFile { path, .. } =
            &cand_source
        {
            opts = opts.with_source_file(path);
        }
        let (composed, _report) = md.compose_with(opts).unwrap();
        let _ = composed.content();
        let t_compose_ni = t3.elapsed();
        eprintln!("run #{run} compose non-interactive.md: {:?}", t_compose_ni);

        let total = t_resolve + t_compose_sp + t_candidates + t_compose_ni;
        eprintln!("run #{run} step-sum: {:?}", total);
    }

    // Now call the real function for comparison.
    for run in 1..=3 {
        let t = Instant::now();
        let _ = resolve_and_prepare_for_session(&args, &ctx, true).unwrap();
        eprintln!(
            "run #{run} resolve_and_prepare_for_session: {:?}",
            t.elapsed()
        );
    }
}

#[test]
#[ignore = "diagnostic perf bench; run with --ignored --nocapture"]
fn bench_raw_darkmatter_compose_passes() {
    let root = worktree_root();
    let sp_path = root.join("system-prompt.md");
    let ni_path = root.join(".claudine").join("non-interactive.md");

    let sp_text = match std::fs::read_to_string(&sp_path) {
        Ok(t) => t,
        Err(_) => {
            eprintln!("skip: missing {sp_path:?}");
            return;
        }
    };
    let ni_text = match std::fs::read_to_string(&ni_path) {
        Ok(t) => t,
        Err(_) => {
            eprintln!("skip: missing {ni_path:?}");
            return;
        }
    };

    eprintln!("system-prompt.md: {} bytes", sp_text.len());
    eprintln!("non-interactive.md: {} bytes", ni_text.len());

    // 1) System prompt compose pass.
    for run in 1..=3 {
        let md: Markdown = sp_text.as_str().into();
        let opts = ComposeOptions::new().with_source_file(&sp_path);
        let t = Instant::now();
        let (_composed, _report) = md.compose_with(opts).unwrap();
        eprintln!("system-prompt.md compose run #{run}: {:?}", t.elapsed());
    }

    // 2) Non-interactive appendix compose pass.
    for run in 1..=3 {
        let md: Markdown = ni_text.as_str().into();
        let opts = ComposeOptions::new().with_source_file(&ni_path);
        let t = Instant::now();
        let (_composed, _report) = md.compose_with(opts).unwrap();
        eprintln!("non-interactive.md compose run #{run}: {:?}", t.elapsed());
    }

    // 3) Pure parse cost (no source file, no transclusion / link resolve).
    for run in 1..=3 {
        let t = Instant::now();
        let md: Markdown = sp_text.as_str().into();
        std::hint::black_box(&md);
        eprintln!("system-prompt.md parse run #{run}: {:?}", t.elapsed());
    }

    // 4) Compose without source file (skips canonicalize + link resolve).
    for run in 1..=3 {
        let md: Markdown = sp_text.as_str().into();
        let opts = ComposeOptions::new();
        let t = Instant::now();
        let (_composed, _report) = md.compose_with(opts).unwrap();
        eprintln!(
            "system-prompt.md compose-without-source run #{run}: {:?}",
            t.elapsed()
        );
    }
}
