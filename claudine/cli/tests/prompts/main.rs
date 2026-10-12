//! Opt-in tests of the repository's internal `prompts/`, built only with the
//! `prompt-tests` feature and run by `just test-prompts`, never by CI.
//!
//! A prompt may be a draft or deliberately broken, and editing one must never
//! fail the normal suite. Tests whose subject is Claudine's behavior and that
//! merely need a realistic prompt stay in their tier and read a frozen copy
//! from `tests/fixtures/frozen_prompts/`.

#[path = "../common/mod.rs"]
mod common;

// Installs the `claudine-fake-relay` fixture binary on Windows, which only
// `test-fixtures` builds.
#[cfg(feature = "test-fixtures")]
mod pr_flow_rehearsal;
mod prompt_guide_defects;
mod shipped_implement_router;
mod shipped_prompt_contract;
mod shipped_prompt_route_drift;
mod shipped_prompts;
