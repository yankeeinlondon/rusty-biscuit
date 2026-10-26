//! OAuth2 runtime for schematic-generated REST clients.
//!
//! This crate provides token lifecycle management (acquisition, caching,
//! refresh, revocation) for APIs that use OAuth2 authentication. It wraps
//! the [`oauth2`] crate with Schematic-oriented types and integrates with
//! the declarative [`schematic_define::OAuth2Config`] model.
//!
//! ## Supported Flows
//!
//! - **Authorization Code with PKCE** via [`OAuth2Manager::begin_authorization`]
//!   and [`OAuth2Manager::exchange_code`]
//! - **Client Credentials** via [`OAuth2Manager::acquire_client_credentials_token`]
//! - **Token Refresh** (automatic when calling [`OAuth2Manager::get_valid_token`])
//! - **Token Revocation** via [`OAuth2Manager::revoke_token`]
//!
//! Each acquisition flow runs only for the grant the provider declares, and
//! the provider's `client_auth` decides how the client authenticates. The
//! device-code grant has no flow here.
//!
//! ## Token Storage
//!
//! Tokens are persisted through the [`TokenStore`] trait. Two built-in
//! implementations are provided:
//!
//! - [`MemoryTokenStore`] for tests and short-lived processes
//! - [`FileTokenStore`] for persisting across restarts

pub mod error;
pub mod manager;
pub mod prelude;
pub mod store;
pub mod types;

pub use error::OAuthError;
pub use manager::OAuth2Manager;

pub use store::{FileTokenStore, MemoryTokenStore, TokenStore};
pub use types::{AuthorizationSession, OAuth2RuntimeConfig, StoredTokens};
