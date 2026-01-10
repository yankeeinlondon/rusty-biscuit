//! OpenAPI specification generation.
//!
//! This module provides utilities for generating OpenAPI 3.1.0 specifications
//! from REST API endpoint definitions.
//!
//! ## Examples
//!
//! ```rust
//! use api::openapi::{OpenApiGenerator, OpenApiInfo, Server, EndpointSpec, OutputFormat};
//! use api::RestMethod;
//!
//! let spec = OpenApiGenerator::new(OpenApiInfo::new("My API", "1.0.0"))
//!     .add_server(Server::new("https://api.example.com"))
//!     .add_endpoint(
//!         EndpointSpec::new("get_users", RestMethod::Get, "/users")
//!             .with_summary("Get all users")
//!     )
//!     .add_endpoint(
//!         EndpointSpec::new("get_user", RestMethod::Get, "/users/{id}")
//!             .with_summary("Get user by ID")
//!     )
//!     .generate(OutputFormat::Json)
//!     .unwrap();
//!
//! println!("{}", spec);
//! ```

pub mod generator;

pub use generator::{
    Contact, EndpointSpec, GenerateError, License, OpenApiGenerator, OpenApiInfo, OutputFormat,
    SecurityScheme, Server,
};
