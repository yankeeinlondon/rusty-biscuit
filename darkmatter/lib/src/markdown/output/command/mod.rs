//! The values Darkmatter's commands hand to the shared output dispatch
//! ([`biscuit_terminal::output`]).
//!
//! A value implements [`CommandOutput`](renderable::output::CommandOutput), so
//! one result can be printed as any `--output` target without the command
//! choosing a renderer. Fallible rendering happens before the value exists: the
//! trait's overrides cannot report an error.

mod document;

#[cfg(test)]
mod tests;

pub use document::{
    DocumentEnvelope, DocumentOutput, LifecycleOutputEntry, Outcome, PerfData, PerfStage,
    ReferenceSummary, document_markdown,
};
