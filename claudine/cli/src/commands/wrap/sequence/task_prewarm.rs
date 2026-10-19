//! TTS pre-warm of a sequence task's own `setup:`/`teardown:` stacks.

use claudine::composition::sequence::preflight::PreflightTask;
use claudine::invocation_context::InvocationContext;
use serde_json::Value;

use super::iterate::SequenceRunContext;

/// Submits a task's own `setup:`/`teardown:` for TTS pre-warm wherever a
/// task's context is established (a step's task, a serial member, a parallel
/// member): only then are the `ctx`/`env` capture and the interpolated
/// frontmatter that preflight had to skip known.
pub(super) struct TaskStackPrewarm<'a> {
    /// The step's effective frontmatter: what a top-level task's stacks and
    /// every group member's stacks are evaluated against.
    frontmatter: &'a serde_json::Map<String, Value>,
    /// The invocation's bridged user settings, which decide the gates; `None`
    /// when pre-warm is off, so the configuration is never read for nothing.
    user: Option<claudine::events::GlobalSettings>,
}

impl<'a> TaskStackPrewarm<'a> {
    pub(super) fn new(run: &SequenceRunContext<'_>, frontmatter: &'a serde_json::Map<String, Value>) -> Self {
        // As lifecycle playback does, an unreadable configuration plays with
        // the defaults.
        let user = run.prep_context.invocation.prewarm().is_enabled().then(|| {
            claudine::dispatch::loader::load_claudine_config(
                None,
                run.prep_context.source_repo_root.as_deref(),
            )
            .map(|config| claudine::dispatch::loader::bridge_tts_settings(&config))
            .unwrap_or_default()
        });
        Self { frontmatter, user }
    }

    /// Submit `task`'s stacks with the context it captured, through
    /// `document`'s handle, which keeps the step's injected keys late.
    pub(super) fn submit(
        &self,
        document: &InvocationContext,
        task: &PreflightTask,
        context: &darkmatter::markdown::compose::ComposeContext,
    ) {
        if let Some(user) = &self.user {
            claudine::composition::lifecycle::prewarm::submit_task_stacks(
                document.prewarm(),
                user,
                task,
                self.frontmatter,
                context,
            );
        }
    }
}
