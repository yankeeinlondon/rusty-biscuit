//! Tests for the composition loop engine.

use std::cell::RefCell;
use std::path::Path;

use darkmatter::markdown::{Frontmatter, Markdown};
use serde_json::json;
use tempfile::TempDir;

use super::*;
use crate::composition::prepare::PrepareOptions;
use crate::composition::types::{
    CompositionMode, LoopAction, LoopCondition, ResolvedCompositionSource,
};
use super::super::config::resolve_loop_config;
use super::super::seed::build_loop_seed;

/// The loop-engine wiring captures a populated `timing` global so loop
/// lifecycle events (`initialize`, `loop`) expose `timing.document_ms` and
/// `timing.total_ms`, rather than the pre-fix `None`/`None`.
#[test]
fn capture_loop_lifecycle_timing_populates_document_and_total_ms() {
    let loop_start = std::time::Instant::now();
    let timing = capture_loop_lifecycle_timing(loop_start);

    assert!(
        timing.document_ms.is_some(),
        "document_ms is populated from the run-level instant"
    );
    assert!(
        timing.total_ms.is_some(),
        "total_ms is populated because a run_start instant is supplied"
    );
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn counter_loop(max: usize) -> LoopConfig {
    LoopConfig {
        condition: LoopCondition::While(format!("counter < {max}")),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    }
}

fn make_source(frontmatter: &[(&str, serde_json::Value)]) -> ResolvedCompositionSource {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("loop.md");
    let mut fm = darkmatter::markdown::Frontmatter::new();
    for (key, value) in frontmatter {
        fm.insert(key, value.clone()).unwrap();
    }
    let md = darkmatter::markdown::Markdown::with_frontmatter(fm, "Body");
    std::fs::write(&file, md.as_string()).unwrap();
    let original_text = std::fs::read_to_string(&file).unwrap();
    let markdown: darkmatter::markdown::Markdown = original_text.clone().into();
    ResolvedCompositionSource {
        original_ref: file.to_string_lossy().to_string(),
        resolved_path: file,
        original_text,
        markdown,
    }
}

fn make_source_with_body(
    frontmatter: &[(&str, serde_json::Value)],
    body: &str,
) -> ResolvedCompositionSource {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("loop.md");
    let mut fm = Frontmatter::new();
    for (key, value) in frontmatter {
        fm.insert(key, value.clone()).unwrap();
    }
    let md = Markdown::with_frontmatter(fm, body);
    std::fs::write(&file, md.as_string()).unwrap();
    let original_text = std::fs::read_to_string(&file).unwrap();
    let markdown: Markdown = original_text.clone().into();
    ResolvedCompositionSource {
        original_ref: file.to_string_lossy().to_string(),
        resolved_path: file,
        original_text,
        markdown,
    }
}

/// Drive the loop engine with no lifecycle blocks, handing each iteration to
/// `executor`.
///
/// Tests whose subject is iteration timing, actions, rate limits, or seeding
/// use this so the lifecycle wiring stays out of their fixtures.
fn run_loop(
    prompt_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    options: LoopExecutionOptions,
    mut executor: impl FnMut(LoopIterationContext) -> Result<LoopIterationOutput, CompositionError>,
) -> Result<LoopExecutionResult, CompositionError> {
    let settings = crate::events::GlobalSettings::default();
    let messaging = crate::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = biscuit_terminal::terminal::Terminal::default();
    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: prompt_path,
        repo_root: prompt_path.parent(),
        launch_area: None,
        context: None,
    };
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(prompt_path.parent().unwrap_or(Path::new(".")))
        .auto_rehash(false)
        .build();
    let initialize_frontmatter = initial_frontmatter.clone();
    execute_loop_with_lifecycle(
        prompt_path,
        config,
        initial_frontmatter,
        &initialize_frontmatter,
        None,
        options,
        &LifecycleConfig::default(),
        &lifecycle_ctx,
        &effect_engine,
        &crate::composition::lifecycle_executor::SystemShellRunner,
        &crate::composition::DefaultLifecycleEmitter,
        None,
        None,
        |ctx, guard| {
            guard.emit_start_once();
            executor(ctx)
        },
    )
}

mod gate_ambient;
mod iteration_actions;
mod lifecycle_control;
mod post_checked_timing;
mod rate_limits;
mod seed_state;
