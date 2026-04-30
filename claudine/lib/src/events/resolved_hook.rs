use serde::{Deserialize, Serialize};

use super::{AgenticEvent, EventMeta};
use crate::actions::HookAction;
use crate::provider::Provider;

/// A resolved hook binding ready for execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedHook {
    /// The normalized event that fired.
    pub event: AgenticEvent,

    /// Normalized metadata extracted from the native payload.
    pub meta: EventMeta,

    /// The provider that originated this event.
    pub provider: Provider,

    /// Actions to execute in declaration order.
    pub actions: Vec<HookAction>,

    /// Whether this hook's event supports blocking on the originating CLI.
    pub can_block: bool,
}
