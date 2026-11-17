//! The editor service behind the lifecycle `edit` action in Claudine's event
//! stacks.
//!
//! The action itself (parsing, target resolution, cancellation, failure
//! routing) is Darkmatter's; Claudine supplies only what its host knows about
//! its streams. Darkmatter's [`ProcessEditorService`] decides from those facts
//! whether an editor may borrow the terminal, needs a display, or must be
//! refused.

use darkmatter::editor::{
    EditOutcome, EditRequest, EditorHostFacts, EditorOutput, EditorService, EnvironmentProbe, ProcessEditorService,
};

/// The [`EditorService`] every Claudine stack lends its `edit` actions.
///
/// It observes the process (terminals, display session) at each edit rather
/// than once, so a stack that runs long after startup sees the streams as
/// they are when the editor would open.
pub(crate) struct ClaudineEditorService;

pub(crate) static EDITOR_SERVICE: ClaudineEditorService = ClaudineEditorService;

impl EditorService for ClaudineEditorService {
    fn edit(&self, request: &EditRequest<'_>) -> EditOutcome {
        service().edit(request)
    }
}

/// The service for this process as it is now.
pub(super) fn service() -> ProcessEditorService {
    service_with(EnvironmentProbe::process())
}

/// The service with an explicit view of the process; tests use it to stand
/// in for a host they are not running on (no display, no terminal).
pub(super) fn service_with(probe: EnvironmentProbe) -> ProcessEditorService {
    // Claudine has no structured result of its own on stdout, so its streams
    // are never reserved and eligibility follows only the probe's facts. A
    // piped prompt leaves stdin without a terminal, which the probe already
    // refuses for a terminal editor, so stdin is never separately reserved.
    ProcessEditorService { facts: EditorHostFacts { output: EditorOutput::Terminal, stdin_reserved: false }, probe }
}

#[cfg(test)]
mod tests;
