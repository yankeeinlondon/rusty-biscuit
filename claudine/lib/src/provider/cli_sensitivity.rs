//! Typed CLI override sensitivity descriptor for [`ProviderInfo`].
//!
//! [`CliSensitiveAxes`] enumerates which permission-policy axes a
//! provider's CLI flags can override at runtime. The `permissions::query`
//! module consults this descriptor to decide whether to mark a configured
//! result as `MayChangeWithCli`, replacing the previous
//! `match Provider { ... }` dispatch.

use serde::Serialize;

/// Per-axis booleans capturing which permission-policy axes are subject to
/// CLI override sensitivity for a given provider.
///
/// Each flag corresponds to a [`crate::permissions::query::PolicyQuery`]
/// variant. When `true`, queries against that axis must be reported as
/// `MayChangeWithCli` for this provider (the configured policy may be
/// flipped at runtime by a CLI flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CliSensitiveAxes {
    /// CLI flags can override read-path policy.
    pub read_path: bool,
    /// CLI flags can override write-path policy.
    pub write_path: bool,
    /// CLI flags can override directory-traverse policy.
    pub traverse_path: bool,
    /// CLI flags can override command-execution policy.
    pub execute_command: bool,
    /// CLI flags can override domain-access policy.
    pub access_domain: bool,
    /// CLI flags can override MCP-server-use policy.
    pub use_mcp_server: bool,
    /// CLI flags can override MCP-tool-use policy.
    pub use_mcp_tool: bool,
    /// CLI flags can override subagent-spawn policy.
    pub spawn_subagent: bool,
    /// CLI flags can override mode-switch policy.
    pub switch_mode: bool,
    /// CLI flags can override the modify-provider-config policy.
    pub modify_provider_config: bool,
}

impl CliSensitiveAxes {
    /// All axes are insensitive to CLI overrides.
    pub const NONE: Self = Self {
        read_path: false,
        write_path: false,
        traverse_path: false,
        execute_command: false,
        access_domain: false,
        use_mcp_server: false,
        use_mcp_tool: false,
        spawn_subagent: false,
        switch_mode: false,
        modify_provider_config: false,
    };

    /// All axes are sensitive to CLI overrides.
    pub const ALL: Self = Self {
        read_path: true,
        write_path: true,
        traverse_path: true,
        execute_command: true,
        access_domain: true,
        use_mcp_server: true,
        use_mcp_tool: true,
        spawn_subagent: true,
        switch_mode: true,
        modify_provider_config: true,
    };
}
