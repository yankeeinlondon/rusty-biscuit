//! Message routing and correlation helpers.

use proc_macro2::TokenStream;

/// Generate routing helpers for WS message dispatch (placeholder).
pub fn generate_routing_helpers() -> TokenStream {
    // Per-API routing enums (XxxWsInbound, XxxWsOutbound) will be generated
    // when typed envelope structs are available from improved AsyncAPI import.
    TokenStream::new()
}
