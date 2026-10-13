//! Spike: the async pager and the generated type surface for D14.
//!
//! - [`spec`]: the static, generator-emitted description of an endpoint and
//!   its strategy, plus the capability traits that gate knobs (encoding A).
//! - [`engine`]: the one non-generic pagination engine shared by every
//!   endpoint and every strategy kind.
//! - [`pager`]: the thin generic `Pager<E>` / `Pages<E>` pair the caller sees.
//! - [`generated`]: what the generator would emit for seven sample endpoints.
//! - [`encoding_a1`]: the real pager's capability traits with impl-level
//!   bounds instead of method-level bounds, kept to compare compiler messages.
//! - [`encoding_b`]: the rejected type-parameter typestate encoding, kept for
//!   the comparison in `findings.md`.

pub mod client;
pub mod encoding_a1;
pub mod encoding_b;
pub mod engine;
pub mod error;
pub mod generated;
pub mod page;
pub mod pager;
pub mod spec;

pub use client::{Client, SentRequest};
pub use error::{PageSizeError, PagerError};
pub use page::Page;
pub use pager::{Pager, Pages};
