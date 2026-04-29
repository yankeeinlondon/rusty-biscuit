//! Typed YOLO / auto-approve permission metadata.
//!
//! Phase 5 of the centralized providers refactor replaces the stringly
//! typed `yolo_equivalent: Option<&'static str>` field with a structured
//! [`YoloSupport`] enum that captures the full shape of each provider's
//! YOLO surface (direct flag, env-var, mode-conditional, etc.).

use serde::Serialize;

/// How a provider exposes a "skip permission prompts" / YOLO mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum YoloSupport {
    /// The provider does not expose any YOLO equivalent.
    None,
    /// A single CLI flag enables YOLO mode in any context.
    DirectFlag {
        /// The native CLI flag (e.g. `--dangerously-skip-permissions`).
        native_flag: &'static str,
    },
    /// A single CLI flag plus accepted alias spellings.
    DirectFlagWithAlias {
        /// The canonical native flag.
        native_flag: &'static str,
        /// Additional accepted alias forms.
        aliases: &'static [&'static str],
    },
    /// YOLO is only reachable in non-interactive mode (e.g. OpenCode's
    /// `--dangerously-skip-permissions` is honored only with `run`).
    NonInteractiveOnly {
        /// The native flag forwarded in non-interactive mode.
        non_interactive_flag: &'static str,
    },
    /// YOLO is selected by setting an environment variable to a fixed
    /// value (e.g. Goose's `GOOSE_MODE=auto`).
    EnvVar {
        /// Environment variable name.
        env_var: &'static str,
        /// Required value to enable YOLO mode.
        value: &'static str,
    },
}
