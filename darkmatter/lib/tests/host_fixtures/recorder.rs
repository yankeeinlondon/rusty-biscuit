//! An in-process `md` lifecycle run that records instead of acting: which
//! events [`md_host::drive`] evaluated, every communication and effect a
//! stack requested (`info`, `message`, speech, sound effects,
//! notifications, command output), and whether the run prepared and wrote.
//!
//! Stacks run through the real executor ([`md_host::evaluate`]) against the
//! caller's [`ComposeRequest`], so bindings, host globals, and failures are
//! production's. Nothing reaches a terminal, a speaker, or the desktop;
//! preparation and the write only record that they were asked for, and no
//! lifecycle shell is approved, so none runs.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use darkmatter::effects::EffectEngine;
use darkmatter::lifecycle::executor::LifecycleEventOutcome;
use darkmatter::lifecycle::md_host::{self, CommandOutput, MdEvent, MdRunEnd, MdRunHost, MdRuntimeState, MdStackContext};
use darkmatter::lifecycle::run::ApprovalEpoch;
use darkmatter::lifecycle::{LifecycleConfig, LifecycleEffects, LifecycleErrorInfo, LifecycleSignal, LifecycleTiming};
use darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals;
use darkmatter::markdown::compose::{ComposeRequest, SharedFrontmatter};
use serde_json::{Map, Value};

/// Every request a run made, in order: `event <name>`, `prepare`, `write`,
/// and one line per effect (`info <text>`, `speech <text>`, …).
#[derive(Debug, Default)]
pub struct EffectLog(Mutex<Vec<String>>);

impl EffectLog {
    fn push(&self, line: String) {
        self.0.lock().unwrap().push(line);
    }

    /// The recorded lines.
    pub fn lines(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl LifecycleEffects for EffectLog {
    fn stdout(&self, text: &str) {
        self.push(format!("stdout {text}"));
    }
    fn stderr(&self, _: LifecycleSignal, text: &str) {
        self.push(format!("stderr {text}"));
    }
    fn info(&self, text: &str) {
        self.push(format!("info {text}"));
    }
    fn warn(&self, text: &str) {
        self.push(format!("warn {text}"));
    }
    fn success(&self, text: &str) {
        self.push(format!("success {text}"));
    }
    fn message(&self, text: &str) {
        self.push(format!("message {text}"));
    }
    fn speech(&self, text: &str) {
        self.push(format!("speech {text}"));
    }
    fn effect(&self, name: &str) {
        self.push(format!("effect {name}"));
    }
    fn notification(&self, title: &str) {
        self.push(format!("notify {title}"));
    }
}

impl CommandOutput for EffectLog {
    fn stdout(&self, text: &str) {
        self.push(format!("command stdout {text}"));
    }
    fn stderr(&self, text: &str) {
        self.push(format!("command stderr {text}"));
    }
}

/// A recorded run: how it ended and everything it requested.
#[derive(Debug)]
pub struct Recorded {
    pub end: MdRunEnd,
    pub log: Vec<String>,
}

impl Recorded {
    /// The `event <name>` lines, as names.
    pub fn events(&self) -> Vec<&str> {
        self.log.iter().filter_map(|line| line.strip_prefix("event ")).collect()
    }
}

struct RecordingHost<'a> {
    request: &'a ComposeRequest,
    config: &'a LifecycleConfig,
    source_path: &'a Path,
    frontmatter: SharedFrontmatter,
    live: Mutex<SharedFrontmatter>,
    state: MdRuntimeState,
    approved: ShellPreApprovals,
    engine: EffectEngine,
    log: &'a EffectLog,
    timing: LifecycleTiming,
}

impl MdRunHost for RecordingHost<'_> {
    fn event(&mut self, event: MdEvent) -> LifecycleEventOutcome {
        self.log.push(format!("event {}", event.name));
        let probe = || false;
        let context = MdStackContext {
            request: self.request,
            source_path: self.source_path,
            frontmatter: &self.frontmatter,
            live_frontmatter: &self.live,
            state: &self.state,
            approved: &self.approved,
            effect_engine: &self.engine,
            effects: self.log,
            output: self.log,
            timing: &self.timing,
            interrupted: &probe,
            editor: None,
        };
        md_host::evaluate(&context, self.config, &event)
    }

    fn prepare(&mut self) -> Result<ApprovalEpoch, LifecycleErrorInfo> {
        self.log.push("prepare".to_string());
        Ok(ApprovalEpoch(1))
    }

    fn write(&mut self) -> Result<(), LifecycleErrorInfo> {
        self.log.push("write".to_string());
        Ok(())
    }

    fn interrupted(&self) -> bool {
        false
    }
}

/// Drives `config` in-process against `request`, with `frontmatter` as the
/// document's effective frontmatter, and records the run.
pub fn record_run(
    request: &ComposeRequest,
    config: &LifecycleConfig,
    source_path: &Path,
    frontmatter: Map<String, Value>,
) -> Recorded {
    let mutation_root: PathBuf = source_path.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let log = EffectLog::default();
    let shared = SharedFrontmatter::from(frontmatter);
    let mut host = RecordingHost {
        request,
        config,
        source_path,
        live: Mutex::new(shared.clone()),
        frontmatter: shared,
        state: MdRuntimeState::default(),
        approved: ShellPreApprovals::new(),
        engine: EffectEngine::builder().mutation_root(mutation_root).build(),
        log: &log,
        timing: LifecycleTiming::default(),
    };
    let end = md_host::drive(&mut host, config);
    Recorded { end, log: log.lines() }
}
