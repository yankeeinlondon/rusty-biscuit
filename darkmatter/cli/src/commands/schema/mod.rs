//! `md schema` subcommand implementations.

pub mod about;
pub mod assignment;
pub mod detect;
pub mod validate;
pub mod triggers;

pub use about::run_about;
pub use detect::run_detect;
pub use validate::run_validate;
pub use triggers::run_triggers;
