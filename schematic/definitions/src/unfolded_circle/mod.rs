//! Unfolded Circle API definitions.
//!
//! This module contains definitions for the Unfolded Circle Core REST API and
//! WebSocket APIs (Core, Dock, Integration).

pub mod core_rest;
pub mod core_ws;
pub mod dock_ws;
pub mod integration_ws;

pub use core_rest::define_unfolded_circle_core_rest_api;
pub use core_ws::define_unfolded_circle_core_ws_api;
pub use dock_ws::define_unfolded_circle_dock_ws_api;
pub use integration_ws::define_unfolded_circle_integration_ws_api;
