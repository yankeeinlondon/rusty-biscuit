//! Stalled-generation backstop — the live-but-dead guard.
//!
//! When OpenCode keeps launching fresh generations (`llm_call_start`) without
//! ever making forward progress, the two-condition trip in
//! [`record_llm_call_and_check_trip`] fires an
//! [`EarlyTermination::StalledGeneration`] so the wrapper can abort the
//! retry-churn loop. This module holds the detection methods, the constant
//! threshold, and the badge renderer.

use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

use crate::stream::logs::opencode::events::OpenCodeLogRecord;
use crate::stream::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};

use super::format::{base_extra, duration_as_millis_u64};
use super::{
    EarlyTermination, OpenCodeLogBridge, StalledGenerationContext, StderrIngestOutcome,
};

/// Generation attempts (`llm_call_start`) tolerated since the last
/// progress-class event before the stalled-generation backstop may trip.
/// Mirrors [`MAX_CONSECUTIVE_STREAM_ERRORS`] in shipping as a constant rather
/// than a knob.
///
/// This count is one half of a two-condition defense against false positives:
/// a trip additionally requires progress silence to exceed `stall_timeout`.
/// Because the count condition is mandatory, a single legitimately slow
/// generation (one `llm_call_start` that takes >10m on a struggling endpoint)
/// is exempt — only genuine retry churn, where OpenCode relaunches the same
/// dropped generation again and again with no forward progress, accumulates
/// toward the threshold.
pub(super) const MAX_GENERATIONS_WITHOUT_PROGRESS: u32 = 4;

impl<S: SemanticEventSink> OpenCodeLogBridge<S> {
    /// Mark forward progress for the stalled-generation backstop: advance the
    /// silence clock to `now` and clear the generation churn counter. The last
    /// captured generation context is deliberately retained so a later trip
    /// message can still name the most recent attempted generation.
    ///
    /// Resets the **shared** progress cell, so a stdout-origin progress event
    /// observing the same cell and a stderr-bridge progress event converge on
    /// one counter.
    pub(super) fn reset_stalled_generation_progress(&mut self, now: Instant) {
        self.progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .mark_progress(now);
    }

    /// Record one streamed `llm_call_start` and evaluate the two-condition
    /// trip. Increments the generation churn counter and stores `ctx` as the
    /// last attempted generation, then returns
    /// [`EarlyTermination::StalledGeneration`] only when **both** conditions
    /// hold: the guard is armed (`stall_timeout` is `Some`), churn has reached
    /// [`MAX_GENERATIONS_WITHOUT_PROGRESS`], and progress silence has reached
    /// the configured budget. The first `LlmCall` after progress never trips on
    /// its own — the count condition protects a legitimately slow generation.
    pub(super) fn record_llm_call_and_check_trip(
        &mut self,
        now: Instant,
        ctx: StalledGenerationContext,
    ) -> Option<EarlyTermination> {
        self.last_generation_context = Some(ctx);

        let mut progress = self
            .progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        progress.generation_count_since_progress =
            progress.generation_count_since_progress.saturating_add(1);

        let stall_timeout = self.stall_timeout?;
        if progress.generation_count_since_progress < MAX_GENERATIONS_WITHOUT_PROGRESS {
            return None;
        }
        let stall_duration = now.duration_since(progress.last_progress_at);
        if stall_duration < stall_timeout {
            return None;
        }
        let generation_count = progress.generation_count_since_progress;
        drop(progress);
        Some(EarlyTermination::StalledGeneration {
            generation_count,
            stall_duration,
            // The retained context names the last attempted generation; it was
            // just overwritten above, so it is the current call's metadata.
            context: self.last_generation_context.clone().unwrap_or_default(),
        })
    }

    /// Emit the terminal live-stderr badge for a stalled-generation trip and
    /// request a fail-fast abort exactly once. The badge carries the
    /// generation-attempt count, the elapsed progress silence, and any safe
    /// context (session id, step, agent, provider id, model id, mode) — never
    /// prompt text or tool payloads. Uses [`SemanticErrorKind::AgentNative`] so
    /// color/style match other agent-native failures.
    pub(super) fn on_stalled_generation(
        &mut self,
        record: &OpenCodeLogRecord,
        termination: EarlyTermination,
    ) -> StderrIngestOutcome {
        let EarlyTermination::StalledGeneration {
            generation_count,
            stall_duration,
            context,
        } = &termination
        else {
            return StderrIngestOutcome::Consumed;
        };

        let message = render_stalled_generation_badge(*generation_count, *stall_duration, context);

        let mut extra_map: Map<String, Value> = base_extra(record, "stalled_generation");
        extra_map.insert("label".into(), Value::String("Stalled Generation".into()));
        extra_map.insert("generation_count".into(), json!(generation_count));
        extra_map.insert(
            "stall_duration_ms".into(),
            json!(duration_as_millis_u64(*stall_duration)),
        );
        if let Some(session_id) = &context.session_id {
            extra_map.insert("session_id".into(), Value::String(session_id.clone()));
        }
        if let Some(step) = context.step {
            extra_map.insert("step".into(), json!(step));
        }
        if let Some(agent) = &context.agent {
            extra_map.insert("agent".into(), Value::String(agent.clone()));
        }
        if let Some(provider_id) = &context.provider_id {
            extra_map.insert("provider_id".into(), Value::String(provider_id.clone()));
        }
        if let Some(model_id) = &context.model_id {
            extra_map.insert("model_id".into(), Value::String(model_id.clone()));
        }
        if let Some(mode) = &context.mode {
            extra_map.insert("mode".into(), Value::String(mode.clone()));
        }

        self.sink.on_semantic_event(SemanticEvent::Error {
            message,
            terminal: true,
            kind: SemanticErrorKind::AgentNative,
            extra: Value::Object(extra_map),
        });
        self.fire_early_termination(termination);
        StderrIngestOutcome::Consumed
    }
}

/// Render the live-stderr badge message for a stalled-generation trip. Mirrors
/// the CLI-side `render_stalled_generation_message` shape: the generation-
/// attempt count, the elapsed progress silence in seconds, and any available
/// safe context. Never includes prompt text or tool payloads.
pub(super) fn render_stalled_generation_badge(
    generation_count: u32,
    stall_duration: Duration,
    context: &StalledGenerationContext,
) -> String {
    let seconds = stall_duration.as_secs();
    let mut message = format!(
        "stalled generation: {generation_count} attempts over {seconds}s with no progress; \
         aborting the live-but-dead retry loop"
    );

    let mut details: Vec<String> = Vec::new();
    if let Some(session_id) = &context.session_id {
        details.push(format!("session={session_id}"));
    }
    if let Some(step) = context.step {
        details.push(format!("step={step}"));
    }
    if let Some(agent) = &context.agent {
        details.push(format!("agent={agent}"));
    }
    if let Some(provider_id) = &context.provider_id {
        details.push(format!("provider={provider_id}"));
    }
    if let Some(model_id) = &context.model_id {
        details.push(format!("model={model_id}"));
    }
    if let Some(mode) = &context.mode {
        details.push(format!("mode={mode}"));
    }
    if !details.is_empty() {
        message.push_str(&format!(" ({})", details.join(", ")));
    }

    message
}
