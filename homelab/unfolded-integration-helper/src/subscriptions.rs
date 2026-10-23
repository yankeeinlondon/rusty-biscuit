use std::collections::HashMap;

use schematic_schema::unfolded_circle_integration_ws::{
    UnfoldedCircleIntegrationEventHub, WsConnectionContext,
};
use serde_json::Value;

use crate::connectivity::ConnectivityState;
use crate::envelope::{device_state_event, entity_change_event};

/// Thin bridge between integration code and the shared host event hub.
#[derive(Clone, Debug)]
pub struct SubscriptionRegistry {
    hub: UnfoldedCircleIntegrationEventHub,
}

impl SubscriptionRegistry {
    /// Create a registry bound to the shared host event hub.
    #[must_use]
    pub fn new(hub: UnfoldedCircleIntegrationEventHub) -> Self {
        Self { hub }
    }

    /// Returns the underlying shared event hub.
    #[must_use]
    pub fn hub(&self) -> UnfoldedCircleIntegrationEventHub {
        self.hub.clone()
    }

    /// Mark the current request connection as subscribed.
    pub async fn subscribe(&self, context: &WsConnectionContext) -> bool {
        context.mark_subscribed().await
    }

    /// Broadcast a raw JSON event, as the event builders produce it, to
    /// subscribed clients.
    pub async fn broadcast_raw(&self, message: Value) -> usize {
        self.hub.broadcast_value_to_subscribers(message).await
    }

    /// Broadcast an `entity_change` event to subscribed clients.
    pub async fn broadcast_entity_change(
        &self,
        entity_id: &str,
        entity_type: &str,
        attributes: &HashMap<String, Value>,
    ) -> usize {
        self.broadcast_raw(entity_change_event(entity_id, entity_type, attributes))
            .await
    }

    /// Broadcast a `device_state` event to subscribed clients.
    pub async fn broadcast_device_state(&self, state: ConnectivityState) -> usize {
        self.broadcast_raw(device_state_event(state.as_uc_label()))
            .await
    }
}
