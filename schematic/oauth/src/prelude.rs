//! Prelude module for convenient glob imports.
//!
//! This module re-exports the most commonly used types from `schematic-oauth`
//! for ergonomic glob imports.
//!
//! ## Examples
//!
//! ```
//! use schematic_oauth::prelude::*;
//! ```

pub use crate::error::OAuthError;
pub use crate::manager::OAuth2Manager;
pub use crate::store::{FileTokenStore, MemoryTokenStore, TokenStore};
pub use crate::types::{AuthorizationSession, OAuth2RuntimeConfig, StoredTokens};
