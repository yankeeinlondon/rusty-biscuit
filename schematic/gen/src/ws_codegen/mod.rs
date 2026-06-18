//! WebSocket code generation pipeline.
//!
//! This module contains the WS-specific code generation infrastructure:
//!
//! - [`plan`] - Lowering `WebSocketApi` definitions into `WsRuntimePlan`
//! - [`shared`] - Shared runtime types generator (`ws_shared.rs`)
//! - [`client`] - Per-API typed client generation
//! - [`host`] - Server-role host generation
//! - [`codec`] - Encode/decode trait generation
//! - [`routing`] - Message routing and correlation helpers
//! - [`docs`] - Module documentation generation

pub mod client;
pub mod docs;
pub mod host;
pub mod plan;
pub mod shared;
