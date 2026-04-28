//! Shell Blocks — block-level shell command execution.
//!
//! This module will contain the full shell-block implementation in later
//! phases.  For Phase 1 it exports only the error type so that
//! `MarkdownError::ShellBlock` can be wired up.

/// Errors from shell block parsing or execution.
///
/// This is a placeholder enum expanded in Phase 2.
#[derive(Debug, thiserror::Error)]
pub enum ShellBlockError {
    /// Placeholder variant so the type is non-empty.
    #[error("Shell block error: {0}")]
    Other(String),
}

impl biscuit_terminal::errors::BlockError for ShellBlockError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            ShellBlockError::Other(message) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ShellBlockError", "error"))
                .body(message.clone()),
        }
    }

    fn block_source(&self,
    ) -> Option<&(dyn biscuit_terminal::errors::BlockError + 'static)> {
        None
    }
}
