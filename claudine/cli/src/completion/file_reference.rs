//! File-reference token classification and candidate discovery.
//!
//! Phase 2 of the `2026-04-17-file-completion` feature implements:
//!
//! - token classification (`SetterPartial`, `Bare`, `DotRelative`,
//!   `DotDotRelative`, `Magic`, `Package`, `Unsupported`) using the spec's
//!   strict setter regex `^[A-Za-z_][A-Za-z0-9_]*=`;
//! - repo-context discovery via `sniff::filesystem::repo::detect_repo_structure`
//!   plus `RepoInfo::package_area_for_dir`;
//! - scope-specific candidate discovery (bare landing menu, `./` / `../`,
//!   `@`, `!`);
//! - the bounded walker with depth, candidate, file-size, and skip-list
//!   caps plus deterministic source-rank deduplication.
//!
//! Phase 1 only reserves the module name. The actual types and helpers
//! land in Phase 2; this file intentionally contains no runtime logic yet
//! so Phase 1 can ship a compilable, behaviorally neutral entry path.
