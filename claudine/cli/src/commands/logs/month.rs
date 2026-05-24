//! CLI surface for the `claudine logs month` subcommand.
//!
//! The month window delegates to [`super::trends`] for the default report and to
//! [`super::errors`] when invoked with `month errors`. See [`super::run_range_window`]
//! for the shared dispatch.
