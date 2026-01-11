//! HTTP client module.
//!
//! This module provides the async HTTP client for executing API requests
//! with automatic authentication handling and tracing instrumentation.
//!
//! ## Examples
//!
//! ```rust,ignore
//! use api::client::{ApiClient, ApiClientBuilder};
//! use api::{Endpoint, RestMethod, ApiAuthMethod};
//! use api::response::JsonFormat;
//! use url::Url;
//!
//! #[derive(serde::Deserialize)]
//! struct User { id: u64, name: String }
//!
//! // Create a client with bearer token auth
//! let base_url = Url::parse("https://api.example.com")?;
//! let client = ApiClient::builder(base_url)
//!     .auth(ApiAuthMethod::BearerToken, "sk-xxx")
//!     .build()?;
//!
//! // Execute a request
//! let endpoint: Endpoint<JsonFormat<User>> = Endpoint::builder()
//!     .id("get_user")
//!     .method(RestMethod::Get)
//!     .path("/users/1")
//!     .build();
//!
//! let user = client.execute(&endpoint).await?;
//! ```

mod executor;

pub use executor::{ApiClient, ApiClientBuilder};
