//! HTTP header management for REST APIs.
//!
//! This module provides types for configuring dynamic HTTP headers based on
//! environment variables, including secure credential handling via [`SensitiveString`].
//!
//! ## Examples
//!
//! Create an environment variable list with fallback:
//!
//! ```
//! use schematic_define::EnvList;
//!
//! let env = EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY"]);
//! assert_eq!(env.names().len(), 2);
//! ```
//!
//! Define API key header from environment:
//!
//! ```
//! use schematic_define::{ApiKeyEnv, EnvList};
//!
//! let api_key = ApiKeyEnv {
//!     names: EnvList::single("X_API_KEY"),
//!     header: "X-API-Key".to_string(),
//! };
//! ```

mod builder;
mod env;
mod error;
mod sensitive;

pub use builder::Headers;
pub use env::{ApiKeyEnv, EnvList, EnvMapping};
pub use error::HeaderError;
pub use sensitive::SensitiveString;
