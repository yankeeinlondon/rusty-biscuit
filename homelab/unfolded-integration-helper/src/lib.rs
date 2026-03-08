//! Shared runtime helpers for Unfolded Circle integrations.
//!
//! The helper crate centralizes the protocol rules that the integrations share:
//! requests arrive with top-level `id`, responses echo top-level `req_id` plus
//! top-level `code`, and `entity_change` / `device_state` are asynchronous
//! events delivered through the shared WebSocket host after `subscribe_events`.

pub mod connectivity;
pub mod envelope;
#[cfg(feature = "mdns")]
pub mod mdns;
pub mod state_cache;
pub mod subscriptions;
pub mod test_fixtures;

pub use std::collections::HashMap as StdHashMap;

pub use connectivity::{ConnectivityState, ConnectivityTracker};
pub use envelope::{
    EnvelopeError, IntegrationRequest, RequestEnvelope, available_entities_response,
    device_state_event, driver_version_response, entity_change_event, entity_states_response,
    result_response,
};
pub use state_cache::{CachedEntityState, EntityState, StateCache};
pub use subscriptions::SubscriptionRegistry;
pub type SubscriptionManager = SubscriptionRegistry;
pub type Attributes = std::collections::HashMap<String, serde_json::Value>;
