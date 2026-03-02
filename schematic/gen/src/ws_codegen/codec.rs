//! Encode/decode helper generation for WS message types.

use proc_macro2::TokenStream;

/// Generate codec helpers for a WS API (placeholder for future per-type codecs).
pub fn generate_codec_helpers() -> TokenStream {
    // Codec generation is handled inline by the shared module's blanket impls.
    // Per-type codec specialization will be added when AsyncAPI import
    // provides enough schema fidelity.
    TokenStream::new()
}
