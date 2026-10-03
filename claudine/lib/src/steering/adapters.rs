//! Implemented steering adapter identifiers.
//!
//! An adapter is the hand-written protocol code that carries one provider's
//! steering mechanism; it lives beside the managed launch that owns the
//! provider's I/O (the CLI wrapper). Its identifier and revision appear here
//! only when the implementation exists; the activation policy
//! (`docs/providers/steering-activation.yaml`) separately records review.
//! Runtime eligibility requires both, and the test below keeps the two sets
//! equal so neither can grant activation alone. Bump an adapter's revision
//! whenever its protocol behavior changes; existing grants then stop applying
//! until re-reviewed.

use super::vocabulary::AdapterRef;

/// Pi's managed RPC adapter: `claudine-cli`
/// `commands/wrap/exec/pi_rpc/executor.rs`.
pub const PI_RPC: AdapterRef = AdapterRef { id: "pi-rpc", revision: 1 };

/// Codex's managed app-server adapter: `claudine-cli`
/// `commands/wrap/exec/codex_app_server/executor.rs`.
pub const CODEX_APP_SERVER: AdapterRef = AdapterRef { id: "codex-app-server", revision: 1 };

/// Adapter revisions implemented in this build.
pub const IMPLEMENTED_ADAPTERS: &[AdapterRef] = &[PI_RPC, CODEX_APP_SERVER];

/// Whether `adapter` is both implemented and reviewed.
pub fn is_usable(adapter: AdapterRef) -> bool {
    IMPLEMENTED_ADAPTERS.contains(&adapter) && super::reviewed_adapters().contains(&adapter)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn implemented_adapters_equal_reviewed_adapters() {
        let implemented: BTreeSet<_> = IMPLEMENTED_ADAPTERS.iter().collect();
        let reviewed: BTreeSet<_> = crate::steering::reviewed_adapters().iter().collect();
        assert_eq!(
            implemented, reviewed,
            "an implemented adapter needs a reviewed policy entry and vice versa \
             (docs/providers/steering-activation.yaml, then `claudine providers generate`)"
        );
        assert_eq!(implemented.len(), IMPLEMENTED_ADAPTERS.len(), "duplicate implemented adapter");
    }
}
