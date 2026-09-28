//! Messaging support for Claudine hook system.
//!
//! This module provides outbound messaging capabilities for hook actions,
//! supporting Discord, Slack, Signal, and WhatsApp providers.
//!
//! Sends return immediately and run as tracked tasks. A program that must not
//! lose them at exit awaits [`drain_deliveries`] first; the `claudine` CLI does
//! so on every ordinary exit.

mod config;
mod delivery;
mod resolve;
mod send;

pub use config::{
    MessagingRouteConfig, ScopedMessagingSettings, validate_discord_webhook_url,
    validate_slack_webhook_url,
};
pub use delivery::{DELIVERY_DRAIN_BUDGET, DeliveryLabel, DrainOutcome, drain_deliveries};
pub use resolve::{
    MessagingScope, ResolvedMessagingRoute, RuntimeMessagingSettings, SignalRecipient,
    parse_signal_recipient, resolve_effective_route, resolve_image_path, resolve_secret,
};
pub use send::{
    MessagingError, execute_message, execute_notification, execute_resolved_message,
    test_webhook_connection,
};
