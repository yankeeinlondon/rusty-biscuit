//! Level 1 integration tests for `schematic-runtime`.
//!
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod builder;
mod call;
mod call_build;
mod client_settings;
mod compile_fail;
mod credential_resolution;
mod decode_boundary;
mod envelope;
mod hooks;
mod origin_policy;
mod pager_collect;
mod pager_mechanisms;
mod pager_outside;
mod pager_partition;
mod pager_rules;
mod pager_session;
mod pager_surface;
mod pager_support;
mod pager_transport;
mod raw;
mod redaction;
mod redirects;
mod send;
mod support;
mod test_layout;
mod timing;
mod whole_send_elapsed;
