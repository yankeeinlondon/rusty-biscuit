//! Functional render components for wrapped-session output.
//!
//! Components implement `biscuit_terminal`'s `TerminalRenderable` and consume
//! normalized data plus policy — never `Provider`. Provider variance enters
//! the render layer only as data, so two providers' equivalent events render
//! identically by construction. Design authority:
//! `features/2026-07-02-provider-metadata/design/render-components.md`.

mod final_message;

pub use final_message::FinalMessage;
