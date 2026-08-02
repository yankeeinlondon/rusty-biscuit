//! Launch-CWD detection, `system-prompt.md` discovery, and
//! `ResolvedSystemPrompt` resolution shared by all provider wrappers.
//!
//! See [`claudine/docs/topics/system-prompt.md`](../../../docs/topics/system-prompt.md)
//! for the launch-context discovery hierarchy, `--append`/`--replace`
//! precedence, Darkmatter preparation, and per-provider delivery rules.

mod change_state;
pub mod context;
pub mod prepare;
pub mod resolve;
pub mod types;

pub use change_state::check_and_record;
pub use context::LaunchContext;
pub use prepare::{
    prepare_system_prompt, resolve_and_prepare, resolve_and_prepare_for_session,
    resolve_and_prepare_for_session_with_context,
};
pub use resolve::{resolve_non_interactive_candidates, resolve_system_prompt_source};
pub use types::*;
