//! Diagnostics types for context capture and merge operations.

/// Diagnostic emitted during context capture or merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMergeDiagnostic {
    /// User ctx was an object; merged successfully.
    UserCtxMerged {
        /// Whether any user-defined keys collided with runtime keys.
        had_key_collisions: bool,
    },
    /// User ctx was not an object; replaced with runtime ctx.
    InvalidUserCtxReplaced,
    /// A sniff capture area partially failed.
    PartialRuntimeCapture {
        /// The area that failed (e.g., "git", "os", "hardware").
        area: &'static str,
        /// Human-readable detail about the failure.
        detail: String,
    },
}
