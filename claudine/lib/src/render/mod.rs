//! Functional render components for wrapped-session output.
//!
//! Components implement `biscuit_terminal`'s `TerminalRenderable` and consume
//! normalized data plus policy — never `Provider`. Provider variance enters
//! the render layer only as data, so two providers' equivalent events render
//! identically by construction. Design authority:
//! `features/2026-07-02-provider-metadata/design/render-components.md`.

mod assistant_stream;
mod event_renderer;
mod final_message;
mod metrics_report;
mod prompt;
mod stream;
mod task_stream;
mod thinking_stream;

pub use assistant_stream::AssistantStream;
pub use event_renderer::{
    DISPATCH, Disposition, EventRenderer, RenderUnit, error_kind_presentation,
    pending_matches_tool_call, strip_progress_verb,
};
pub use final_message::FinalMessage;
pub use metrics_report::MetricsReport;
pub use stream::StreamRenderable;
pub use task_stream::{
    TASK_PALETTE, TaskBar, TaskFrameWriter, TaskLiveOutput, TaskStream, TaskStreamFrame,
    TaskStreamOutcome, TaskStreamSink,
};
pub use thinking_stream::ThinkingStream;
pub use prompt::{
    AgentPrompt, ReportMode, SystemPrompt, TruncationMode, parse_frontmatter_verbosity,
    resolve_agent_prompt_report_mode, resolve_system_prompt_report_mode,
};
