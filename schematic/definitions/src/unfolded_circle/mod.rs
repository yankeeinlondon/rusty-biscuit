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
//! | Dock WS | WebSocket | `type` (req/resp/event) | Post-connect token |
//! | Integration WS | WebSocket | `kind` (req/resp/event) | API key in header |
//!
//! All WebSocket envelope types use `Box<RawValue>` for deferred payload parsing.
//! Use `parse_payload::<T>()` on any envelope to deserialize `msg_data` on demand.

pub mod core_rest;
pub mod core_ws;
pub mod dock_ws;
pub mod integration_ws;

pub use core_rest::define_unfolded_circle_core_rest_api;
pub use core_ws::define_unfolded_circle_core_ws_api;
pub use dock_ws::define_unfolded_circle_dock_ws_api;
pub use integration_ws::define_unfolded_circle_integration_ws_api;
