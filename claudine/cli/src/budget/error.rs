use std::path::PathBuf;

use super::model::LedgerState;

/// Why a budget operation was refused.
#[derive(Debug, thiserror::Error)]
pub(crate) enum BudgetError {
    #[error("invalid budget ledger: {0}")]
    Invalid(String),

    #[error("{0}")]
    Exhausted(String),

    #[error("the budget ledger is {} and cannot start a run{}", state.label(), reason.as_deref().map(|r| format!(" ({r})")).unwrap_or_default())]
    NotRunnable {
        state: LedgerState,
        reason: Option<String>,
    },

    #[error("cannot {action} a budget ledger that is {}", state.label())]
    Transition {
        action: &'static str,
        state: LedgerState,
    },

    #[error("{} is held by another run", path.display())]
    Locked { path: PathBuf },

    #[error("budget ledger {} already exists", path.display())]
    AlreadyExists { path: PathBuf },

    #[error("budget ledger {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("budget ledger {}: {source}", path.display())]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}
