//! Per-platform run limits. There are no defaults: a live run needs both an
//! elapsed-time and an agent-invocation limit from the operator.

use serde::{Deserialize, Serialize};

/// The operator-supplied allowance for one platform run, shared by every
/// pass, the evidence reviewer, and recovery attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunLimits {
    pub max_seconds: u64,
    pub max_invocations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RunConfigError {
    #[error("the {0} limit is required; there is no default")]
    Missing(&'static str),
    #[error("the {0} limit must be positive")]
    Zero(&'static str),
}

impl RunLimits {
    /// Both limits, each present and positive.
    ///
    /// ## Errors
    ///
    /// [`RunConfigError::Missing`] or [`RunConfigError::Zero`], naming the
    /// elapsed-time limit first when both are wrong.
    pub fn new(max_seconds: Option<u64>, max_invocations: Option<u64>) -> Result<Self, RunConfigError> {
        let check = |value: Option<u64>, name: &'static str| match value {
            None => Err(RunConfigError::Missing(name)),
            Some(0) => Err(RunConfigError::Zero(name)),
            Some(value) => Ok(value),
        };
        Ok(Self {
            max_seconds: check(max_seconds, "elapsed-time (--max-seconds)")?,
            max_invocations: check(max_invocations, "agent-invocation (--max-invocations)")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_limits_are_required_and_positive() {
        assert_eq!(RunLimits::new(Some(600), Some(8)), Ok(RunLimits { max_seconds: 600, max_invocations: 8 }));
        assert!(matches!(RunLimits::new(None, Some(8)), Err(RunConfigError::Missing(name)) if name.contains("elapsed")));
        assert!(matches!(RunLimits::new(Some(600), None), Err(RunConfigError::Missing(name)) if name.contains("invocation")));
        assert!(matches!(RunLimits::new(None, None), Err(RunConfigError::Missing(name)) if name.contains("elapsed")));
        assert!(matches!(RunLimits::new(Some(0), Some(8)), Err(RunConfigError::Zero(_))));
        assert!(matches!(RunLimits::new(Some(600), Some(0)), Err(RunConfigError::Zero(_))));
    }
}
