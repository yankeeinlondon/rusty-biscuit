//! WebSocket code generation pipeline.
//!
//! This module contains the WS-specific code generation infrastructure:
//!
//! - [`plan`] - Lowering `WebSocketApi` definitions into `WsRuntimePlan`
//! - [`shared`] - Shared runtime types generator (`ws_shared.rs`)
//! - [`supervisor`] - The connection supervisor and setup session in `ws_shared.rs`
//! - [`client`] - Per-API typed client generation
//! - [`host`] - Server-role host generation
//! - [`docs`] - Module documentation generation

pub mod client;
pub mod docs;
pub mod host;
pub mod plan;
pub mod shared;
pub mod supervisor;
