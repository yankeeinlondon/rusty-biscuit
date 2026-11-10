//! The values Darkmatter's commands hand to the shared output dispatch
//! ([`biscuit_terminal::output`]).
//!
//! A value implements [`CommandOutput`](renderable::output::CommandOutput), so
//! one result can be printed as any `--output` target without the command
//! choosing a renderer. Fallible rendering happens before the value exists: the
//! trait's overrides cannot report an error.

mod clean;
mod data;
mod document;
mod schema_about;
mod value;

#[cfg(test)]
mod tests;

pub use clean::{
    CleanJsonDiagnostic, CleanJsonFrontmatter, CleanJsonPositionedSpan, CleanJsonRepair, CleanJsonReport, CleanJsonSource,
    CleanJsonSourceKind, CleanJsonSpan, FindingRow, json_report as clean_report, schema_failure_report as clean_schema_failure_report,
};
pub use data::{
    CodeBlockData, CodeRange, ComparisonData, DetectData, DetectedFile, EditData, GraphNotation, GraphNotationData, HashData, HashMode, HashValueData,
    IgnorePolicyData, RemovedData, SetData, TriggerConditionData, TriggerFindingData, TriggerRootData, TriggerShadowData,
    TriggerTraceData, TriggersData, ValidateData,
};
pub use schema_about::{AboutData, AboutDetail, AboutEntry, AboutSection};
pub use value::{CommandValue, code_document, empty_document, prose_document, terminal_fold};
pub use document::{
    DocumentEnvelope, DocumentOutput, LifecycleOutputEntry, Outcome, PerfData, PerfStage,
    ReferenceSummary, document_markdown,
};
