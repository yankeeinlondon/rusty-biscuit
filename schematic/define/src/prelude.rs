//! Convenient re-exports for working with schematic definitions.
//!
//! This prelude provides all the core types needed to define REST APIs.
//!
//! ## Examples
//!
//! ```
//! use schematic_define::prelude::*;
//!
//! let api = RestApi {
//!     name: "MyAPI".to_string(),
//!     description: "My API".to_string(),
//!     base_url: "https://api.example.com".to_string(),
//!     docs_url: None,
//!     auth: AuthStrategy::BearerToken { header: None },
//!     env_auth: vec!["API_KEY".to_string()],
//!     env_username: None,
//!     headers: vec![],
//!     endpoints: vec![],
//! };
//! ```

pub use crate::auth::{AuthStrategy, UpdateStrategy};
pub use crate::response::ApiResponse;
pub use crate::schema::{Schema, SchemaObject};
pub use crate::types::{Endpoint, RestApi, RestMethod};
