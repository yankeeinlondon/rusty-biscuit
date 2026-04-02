//! Messaging support for Claudine hook system.
//!
//! This module provides outbound messaging capabilities for hook actions,
//! supporting Discord, Slack, Signal, and WhatsApp providers.

mod config;
// mod resolve;
// mod send;

pub use config::{MessagingRouteConfig, ScopedMessagingSettings};
// pub use resolve::{
//     MessagingScope, ResolvedMessagingRoute, RuntimeMessagingSettings, resolve_effective_route,
// };
// pub use send::execute_message;
