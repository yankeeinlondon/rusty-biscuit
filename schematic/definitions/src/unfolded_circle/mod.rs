//! Unfolded Circle API definitions.
//!
//! This module contains definitions for the Unfolded Circle Core REST API and
//! WebSocket APIs (Core, Dock, Integration).
//!
//! ## API Overview
//!
//! | API | Transport | Envelope Discriminant | Auth Strategy |
//! |-----|-----------|----------------------|---------------|
//! | Core REST | HTTP | N/A | Bearer token / Basic auth |
//! | Core WS | WebSocket | `kind` (req/resp/event) | API key in header |
//! | Dock WS | WebSocket | `type` (dock/event/auth) | Post-connect token |
//! | Integration WS | WebSocket | `kind` (req/resp/event) | API key in header |
//!
//! The Core and Integration envelopes keep `msg_data` as a `Box<RawValue>`
//! for deferred parsing; their `parse_payload::<T>()` deserializes it on
//! demand. The Dock protocol
//! has no `msg_data`: its envelopes retain every other top-level field in a
//! map, which `parse_data::<T>()` deserializes.

pub mod core_rest;
pub mod core_ws;
pub mod dock_ws;
pub mod integration_ws;

pub use core_rest::define_unfolded_circle_core_rest_api;
pub use core_ws::define_unfolded_circle_core_ws_api;
pub use dock_ws::define_unfolded_circle_dock_ws_api;
pub use integration_ws::define_unfolded_circle_integration_ws_api;
