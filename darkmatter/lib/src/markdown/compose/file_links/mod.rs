//! `::file-links` directive — discovers a bounded set of document files and
//! prepares them for [`FileSystem`](biscuit_terminal::components::filesystem::FileSystem)
//! tree rendering.
//!
//! This module owns the directive parser, boundary-safe file discovery,
//! and compose pipeline integration (concurrent resolution and replacement).
//!
//! ## Syntax
//!
//! Two source forms are accepted:
//!
//! - `::file-links <glob>` — a glob pattern resolved relative to the document.
//! - `::file-links --dir <path> [--depth <u32>]` — a directory scan with
//!   optional recursion depth (default `0`).
//!
//! ## Filtering
//!
//! Only files with the allowed document extensions (`.md`, `.txt`, `.doc`,
//! `.docx`, `.xls`, `.xlsx`, `.pdf`) are included, compared case-insensitively.
//! The containing document is excluded. Candidates resolving outside the
//! repository root (or current working directory when no repo is found) are
//! ignored for security.

mod discovery;
mod parser;
mod types;

pub use types::{FileLinksDirective, FileLinksError, FileLinksMode, FileLinksRender, FileLinksResult};

pub(crate) use parser::parse_file_links_directives;
pub(crate) use discovery::discover;
