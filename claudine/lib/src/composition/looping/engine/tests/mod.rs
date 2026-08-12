//! Tests for the composition loop engine.

use std::cell::RefCell;
use std::path::Path;

use darkmatter::markdown::{Frontmatter, Markdown};
use serde_json::json;
use tempfile::TempDir;

use super::*;
use crate::composition::types::{LoopAction, LoopCondition};

/// The loop-engine wiring captures non-empty `timing`/`current` globals so
/// loop lifecycle events (`initialize`, `loop`) expose `timing.document_ms`
/// and a populated `current.env`, rather than the pre-fix `None`/`None`.
#[test]
fn capture_loop_lifecycle_globals_populates_timing_and_env() {
    let loop_start = std::time::Instant::now();
    let (timing, current) =
        capture_loop_lifecycle_globals(Some(Path::new(".")), None, loop_start);

    assert!(
        timing.document_ms.is_some(),
        "document_ms is populated from the run-level instant"
    );
    assert!(
        timing.total_ms.is_some(),
        "total_ms is populated because a run_start instant is supplied"
    );
    assert!(
        current.env.is_object() && !current.env.as_object().unwrap().is_empty(),
        "current.env is a non-empty process-environment snapshot"
    );
    // base_dir = "." → ctx is captured (at minimum ctx.today).
    assert!(
        current.ctx.get("today").is_some(),
        "current.ctx snapshot carries today"
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


mod iteration_actions;
mod lifecycle_control;
mod rate_limits;
mod seed_state;
