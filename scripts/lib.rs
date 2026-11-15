//! In-process API for the `repo-deps` binaries whose decisions are tested at a
//! public boundary rather than by spawning the binary for every case.
//!
//! `[lib]` cannot declare `required-features`, so every module gates itself:
//! without features this library is empty and `ci-rollup` links nothing from
//! it.

#[cfg(feature = "local-tools")]
#[path = "ci-push/mod.rs"]
pub mod ci_push;

#[cfg(feature = "build-tools")]
#[path = "ci-plan-render.rs"]
pub mod ci_plan_render;
