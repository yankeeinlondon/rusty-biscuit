//! Session tracking and subagent lifecycle dispatch.
//!
//! The bridge maintains [`ChildSessionInfo`] for OpenCode child sessions
//! discovered from `service=session ... parentID=...` records so it can
//! convert subsequent `service=session.prompt ... exiting loop` records
//! into [`SemanticEvent::SubagentStop`] events with a 1:1
//! [`SubagentStart`]/[`SubagentStop`] pairing.

use std::time::Instant;

use serde_json::{Map, Value, json};
use tracing::debug;

use crate::stream::logs::opencode::events::OpenCodeLogRecord;
use crate::stream::semantic::{SemanticEvent, SemanticEventSink};

use super::format::base_extra;
use super::{OpenCodeLogBridge, StderrIngestOutcome};

/// Bookkeeping for an OpenCode child session observed on the stderr stream.
///
/// Created when a `service=session ... parentID=...` record is classified
/// as [`LogClassification::SessionCreated`] with a non-empty `parent_id`.
/// `stopped` flips to `true` the first time the child's `exiting loop`
/// fires so subsequent `StepExit` records do not emit duplicate
/// [`SemanticEvent::SubagentStop`] events for the same child.
#[derive(Debug, Clone, Default)]
pub(super) struct ChildSessionInfo {
    pub(super) parent_id: String,
    pub(super) name: Option<String>,
    pub(super) stopped: bool,
}

impl<S: SemanticEventSink> OpenCodeLogBridge<S> {
    pub(super) fn on_session_created(
        &mut self,
        record: &OpenCodeLogRecord,
        id: String,
        parent_id: Option<String>,
    ) -> StderrIngestOutcome {
        // The bare-value body parser absorbs the trailing ` created`
        // keyword into the last tag's value; in practice that tag is
        // `title=`, so strip the suffix before exposing the title to
        // consumers.
        let title = record
            .tags
            .get("title")
            .map(|s| s.strip_suffix(" created").unwrap_or(s).to_string())
            .filter(|s| !s.is_empty());
        let mut extra_map = base_extra(record, "session_created");
        extra_map.insert("session_id".into(), Value::String(id.clone()));
        if let Some(ref parent) = parent_id {
            extra_map.insert("parent_id".into(), Value::String(parent.clone()));
        }
        if let Some(ref title) = title {
            extra_map.insert("title".into(), Value::String(title.clone()));
        }

        match parent_id {
            Some(parent) => {
                // A child session starting is forward progress: the run is
                // dispatching real work, not spinning on dropped generations.
                // Reset the stalled-generation backstop so the subagent gets a
                // fresh silence baseline.
                self.reset_stalled_generation_progress(Instant::now());
                self.child_sessions.insert(
                    id.clone(),
                    ChildSessionInfo {
                        parent_id: parent,
                        name: title.clone(),
                        stopped: false,
                    },
                );
                self.sink.on_semantic_event(SemanticEvent::SubagentStart {
                    name: title,
                    id: Some(id),
                    extra: Value::Object(extra_map),
                });
            }
            None => {
                if self.primary_session_emitted {
                    debug!(
                        session_id = %id,
                        "opencode primary SessionStart already emitted on stderr; skipping duplicate",
                    );
                    return StderrIngestOutcome::Consumed;
                }
                // Cross-stream dedup: if the stdout NDJSON parser has
                // already emitted any semantic event (its `init` /
                // `session_start` payload most likely), the stderr-derived
                // primary SessionStart would be a redundant duplicate.
                // First arrival wins, the stderr session_created is
                // recorded but not re-emitted.
                if self.stdout_event_seen.load(std::sync::atomic::Ordering::SeqCst) {
                    debug!(
                        session_id = %id,
                        "opencode stdout already emitted a semantic event; skipping stderr primary SessionStart",
                    );
                    self.primary_session_emitted = true;
                    return StderrIngestOutcome::Consumed;
                }
                self.primary_session_emitted = true;
                self.sink.on_semantic_event(SemanticEvent::SessionStart {
                    session_id: Some(id),
                    model: None,
                    extra: Value::Object(extra_map),
                });
            }
        }

        StderrIngestOutcome::Consumed
    }

    pub(super) fn on_step_loop(
        &mut self,
        record: &OpenCodeLogRecord,
        session_id: String,
        step: u32,
    ) -> StderrIngestOutcome {
        // OpenCode emits a `service=session.prompt … loop` record at every
        // HTTP-span boundary inside the same step. Suppress repeats so only
        // genuine step transitions surface.
        if self.last_step_per_session.get(&session_id) == Some(&step) {
            return StderrIngestOutcome::Consumed;
        }
        self.last_step_per_session
            .insert(session_id.clone(), step);

        // A genuine step transition is forward progress: clear the
        // stream-error backstop counter so transient retries that eventually
        // succeed do not accumulate toward a false abort. Drop the fingerprint
        // too, so a post-step error identical to a pre-step one starts a fresh
        // run rather than resuming the old count. The same advance is progress
        // for the stalled-generation backstop — reset its churn count and
        // silence clock. Deduped repeats returned early above, so they do not
        // reach this reset.
        self.consecutive_stream_errors = 0;
        self.last_stream_error_fingerprint = None;
        self.reset_stalled_generation_progress(Instant::now());

        let mut extra_map = base_extra(record, "step_loop");
        extra_map.insert("session_id".into(), Value::String(session_id.clone()));
        extra_map.insert("step".into(), json!(step));

        self.sink.on_semantic_event(SemanticEvent::Info {
            message: format!("step_loop step={step} session={session_id}"),
            extra: Value::Object(extra_map),
        });
        StderrIngestOutcome::Consumed
    }

    pub(super) fn on_step_exit(
        &mut self,
        record: &OpenCodeLogRecord,
        session_id: String,
    ) -> StderrIngestOutcome {
        // Drop the dedup entry so a follow-up prompt on the same session
        // starts fresh. A step exit is forward progress for the stalled-
        // generation backstop: reset its churn count and silence clock so a
        // follow-up prompt is evaluated from a clean baseline.
        self.last_step_per_session.remove(&session_id);
        self.reset_stalled_generation_progress(Instant::now());

        let mut extra_map = base_extra(record, "step_exit");
        extra_map.insert("session_id".into(), Value::String(session_id.clone()));

        self.sink.on_semantic_event(SemanticEvent::Info {
            message: format!("exiting_loop session={session_id}"),
            extra: Value::Object(extra_map),
        });

        // If this session was registered as a subagent child, emit a
        // matching SubagentStop. Only fire once per child session so we
        // maintain a 1:1 SubagentStart/SubagentStop pairing even though
        // OpenCode emits an `exiting loop` record at the end of every
        // step within a session.
        if let Some(child) = self.child_sessions.get_mut(&session_id)
            && !child.stopped
        {
            child.stopped = true;
            let name = child.name.clone();
            let parent_id = child.parent_id.clone();
            let mut subagent_extra = Map::new();
            subagent_extra.insert("provider".into(), Value::String("opencode".into()));
            subagent_extra.insert("source".into(), Value::String("stderr_log".into()));
            subagent_extra.insert(
                "classification".into(),
                Value::String("subagent_stop".into()),
            );
            subagent_extra.insert("session_id".into(), Value::String(session_id.clone()));
            subagent_extra.insert("parent_id".into(), Value::String(parent_id));
            if let Some(ref n) = name {
                subagent_extra.insert("name".into(), Value::String(n.clone()));
            }
            self.sink.on_semantic_event(SemanticEvent::SubagentStop {
                name,
                id: Some(session_id),
                status: None,
                extra: Value::Object(subagent_extra),
            });
        }

        StderrIngestOutcome::Consumed
    }
}
