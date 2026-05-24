//! CLI surface for the `claudine logs week` subcommand.
//!
//! The week window delegates to [`super::trends`] for the default report and to
//! [`super::errors`] when invoked with `week errors`. See [`super::run_range_window`]
//! for the shared dispatch.
