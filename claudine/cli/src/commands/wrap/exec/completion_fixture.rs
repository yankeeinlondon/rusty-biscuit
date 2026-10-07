//! Compiled only for hermetic caller tests. The directory's final component
//! selects a fixed scenario; production builds expose no timeout override.

use std::path::PathBuf;
use std::sync::{OnceLock, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use color_eyre::eyre::{Result, eyre};
use claudine::stream::semantic::SemanticEvent;
use super::reader_join::ReaderBudget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Point { Completion, Answer, Output }

struct Fixture {
    directory: PathBuf,
    point: Point,
    entered: AtomicBool,
}
static FIXTURE: OnceLock<Option<Fixture>> = OnceLock::new();

#[cfg(feature = "test-fixtures")]
pub(crate) fn initialize() -> Result<()> {
    let fixture = std::env::var_os("CLAUDINE_TEST_COMPLETION_FIXTURE").map(|path| {
        let directory = PathBuf::from(path);
        let point = match directory.file_name().and_then(|name| name.to_str()) {
            Some("completion-callback") => Point::Completion,
            Some("answer-callback") => Point::Answer,
            Some("output-delivery") => Point::Output,
            _ => return Err(eyre!("invalid completion fixture scenario")),
        };
        if !directory.is_dir() { return Err(eyre!("completion fixture directory is missing")); }
        Ok(Fixture { directory, point, entered: AtomicBool::new(false) })
    }).transpose()?;
    let _ = FIXTURE.set(fixture);
    Ok(())
}

pub(crate) fn budget() -> ReaderBudget {
    if FIXTURE.get().is_some_and(Option::is_some) {
        ReaderBudget { pipe_cap: Duration::from_millis(100), pipe_settle: Duration::from_millis(25), drain_limit: Duration::from_millis(200) }
    } else { ReaderBudget::default() }
}

pub(crate) fn event(event: &SemanticEvent) {
    match event {
        SemanticEvent::TurnComplete { .. } => stall(Point::Completion),
        SemanticEvent::OutputText { .. } => stall(Point::Answer),
        SemanticEvent::Reasoning { extra, .. } if extra["origin"] == "agent_message" => stall(Point::Answer),
        _ => {}
    }
}

pub(crate) fn output() { stall(Point::Output); }

fn stall(point: Point) {
    let Some(Some(fixture)) = FIXTURE.get() else { return; };
    if fixture.point != point || fixture.entered.swap(true, Ordering::AcqRel) { return; }
    std::fs::write(fixture.directory.join("ready"), b"ready").expect("fixture readiness");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !fixture.directory.join("release").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub(crate) fn publish(provider: Option<claudine::provider::Provider>, exit_code: i32,
    observation: &super::super::run_scope::CompletionObservation) -> Result<()> {
    if let Some(Some(fixture)) = FIXTURE.get() {
        let value = serde_json::json!({ "provider": provider.map(|p| p.as_slug()), "exit_code": exit_code, "observation": observation });
        std::fs::write(fixture.directory.join("observation.json"), serde_json::to_vec(&value)?)?;
    }
    Ok(())
}
