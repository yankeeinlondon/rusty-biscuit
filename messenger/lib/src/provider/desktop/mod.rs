//! Desktop notifications provider.
//!
//! Provides a single [`Provider`](crate::provider::Provider) that delivers a
//! [`Message`](crate::message::Message) to the host operating system's
//! notification center. Runtime backend selection (Linux, macOS, Windows)
//! and platform-specific mapping land in later phases; this module exists
//! in phase 2 only to register the feature-gated public surface.
