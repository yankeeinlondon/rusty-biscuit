//! `md schema` subcommand implementations.

pub mod about;
pub mod assignment;
pub mod detect;
pub mod validate;

pub use about::run_about;
pub use detect::run_detect;
pub use validate::run_validate;
