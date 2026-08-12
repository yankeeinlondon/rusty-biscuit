//! Connecting a gRPC client to the rendezvous daemon's local control plane.
//!
//! [`connect`] takes a [`LocalEndpoint`], which already knows whether it is a
//! Unix-domain socket or a Windows named pipe. Callers pass one and get a
//! client back; selecting the transport, opening it, waiting out a busy pipe,
//! and classifying the failure all happen behind that one entry point, so a
//! production call site never needs a `cfg(unix)`/`cfg(windows)` branch.
//!
//! ## Errors
//!
//! [`ConnectError`] keeps the cases a caller acts on apart: nothing listening,
//! not permitted, saturated, an endpoint this target cannot speak, and other
//! transport failures. Each preserves the originating OS error as its source.

mod connector;

pub use connector::{ConnectError, connect};
