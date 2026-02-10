/// Errors that can occur when working with agent status detection and queries.
#[derive(Debug, thiserror::Error)]
pub enum AgentStatusError {
    /// Failed to spawn a PTY process for agent detection.
    ///
    /// ## Examples
    ///
    /// ```text
    /// AgentStatusError::PtySpawnError("claude --version".to_string())
    /// ```
    #[error("Failed to spawn PTY process: {0}")]
    PtySpawnError(String),

    /// Failed to read output from a spawned PTY process.
    ///
    /// ## Examples
    ///
    /// ```text
    /// AgentStatusError::PtyReadError("timeout after 5s".to_string())
    /// ```
    #[error("Failed to read PTY output: {0}")]
    PtyReadError(String),

    /// Failed to write to a PTY process stdin during interactive session.
    #[error("Failed to write to PTY: {0}")]
    PtyWriteError(String),

    /// Failed to parse agent status response or version string.
    ///
    /// ## Examples
    ///
    /// ```text
    /// AgentStatusError::ParseError("invalid JSON response".to_string())
    /// ```
    #[error("Failed to parse agent status: {0}")]
    ParseError(String),

    /// Operation timed out while querying agent status.
    ///
    /// ## Examples
    ///
    /// ```text
    /// AgentStatusError::TimeoutError("status query exceeded 10s".to_string())
    /// ```
    #[error("Operation timed out: {0}")]
    TimeoutError(String),

    /// The requested platform is installed but has no valid API key.
    ///
    /// ## Examples
    ///
    /// ```text
    /// AgentStatusError::PlatformNotInstalled("ClaudeCode binary not found".to_string())
    /// ```
    #[error("Platform not installed: {0}")]
    PlatformNotInstalled(String),

    /// The requested platform is not supported on this operating system.
    ///
    /// ## Examples
    ///
    /// ```text
    /// AgentStatusError::UnsupportedPlatform("Codex not available on Linux".to_string())
    /// ```
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),
}
