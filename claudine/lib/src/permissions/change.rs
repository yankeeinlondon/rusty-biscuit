use std::path::PathBuf;

use super::canonical::{CanonicalApprovalMode, CanonicalSandboxMode};

/// A requested policy change to be planned by a backend.
#[derive(Debug, Clone)]
pub struct PolicyChange {
    /// Operations to perform.
    pub operations: Vec<PolicyChangeOp>,
    /// Where to target the change.
    pub target: PolicyChangeTarget,
    /// Whether the change should be persistent or one-shot.
    pub persistence: PolicyPersistence,
}

/// Individual change operation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PolicyChangeOp {
    /// Grant read access to a path.
    GrantRead(PathBuf),
    /// Grant write access to a path.
    GrantWrite(PathBuf),
    /// Deny read access to a path.
    DenyRead(PathBuf),
    /// Deny write access to a path.
    DenyWrite(PathBuf),
    /// Require user approval for a command pattern.
    RequireApprovalForCommand(CommandPattern),
    /// Allow a command pattern.
    AllowCommand(CommandPattern),
    /// Deny a command pattern.
    DenyCommand(CommandPattern),
    /// Allow access to a domain.
    AllowDomain(String),
    /// Deny access to a domain.
    DenyDomain(String),
    /// Allow an MCP server.
    AllowMcpServer(String),
    /// Deny an MCP server.
    DenyMcpServer(String),
    /// Allow a specific MCP tool.
    AllowMcpTool {
        /// Server identifier.
        server: String,
        /// Tool name.
        tool: String,
    },
    /// Deny a specific MCP tool.
    DenyMcpTool {
        /// Server identifier.
        server: String,
        /// Tool name.
        tool: String,
    },
    /// Allow subagent spawning.
    AllowSubagent(Option<String>),
    /// Deny subagent spawning.
    DenySubagent(Option<String>),
    /// Set the approval mode.
    SetApprovalMode(CanonicalApprovalMode),
    /// Set the sandbox mode.
    SetSandboxMode(CanonicalSandboxMode),
}

/// Target config layer for a persistent change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyChangeTarget {
    /// Let the engine choose the best target based on policy source scope.
    Auto,
    /// Target user-scope config.
    UserConfig,
    /// Target repo-scope config.
    RepoConfig,
    /// Target a local override file.
    LocalOverride,
}

/// Whether a change should be persistent or ephemeral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyPersistence {
    /// Write the change to config files.
    Persistent,
    /// Express the change as one-shot CLI args only.
    OneShot,
}

/// A command matching pattern for change operations.
#[derive(Debug, Clone)]
pub struct CommandPattern {
    /// Raw pattern string (executable name, prefix, or regex).
    pub raw: String,
}

impl CommandPattern {
    /// Creates a command pattern from a raw string.
    pub fn new(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }
}

impl PolicyChange {
    /// Creates a persistent change with auto-targeting.
    pub fn persistent(operations: Vec<PolicyChangeOp>) -> Self {
        Self {
            operations,
            target: PolicyChangeTarget::Auto,
            persistence: PolicyPersistence::Persistent,
        }
    }

    /// Creates a one-shot change.
    pub fn one_shot(operations: Vec<PolicyChangeOp>) -> Self {
        Self {
            operations,
            target: PolicyChangeTarget::Auto,
            persistence: PolicyPersistence::OneShot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistent_change_uses_auto_target_and_persistent_mode() {
        let change =
            PolicyChange::persistent(vec![PolicyChangeOp::AllowDomain("example.com".to_string())]);

        assert!(matches!(change.target, PolicyChangeTarget::Auto));
        assert!(matches!(change.persistence, PolicyPersistence::Persistent));
        assert_eq!(change.operations.len(), 1);
    }

    #[test]
    fn one_shot_change_uses_one_shot_mode() {
        let change =
            PolicyChange::one_shot(vec![PolicyChangeOp::DenyDomain("example.com".to_string())]);

        assert!(matches!(change.persistence, PolicyPersistence::OneShot));
    }

    #[test]
    fn command_pattern_new_preserves_raw_string() {
        let pattern = CommandPattern::new("git push");

        assert_eq!(pattern.raw, "git push");
    }
}
