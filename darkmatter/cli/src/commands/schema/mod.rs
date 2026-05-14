//! `md schema` subcommand implementations.

pub mod assignment;
pub mod detect;
pub mod validate;

pub use detect::run_detect;
pub use validate::run_validate;
