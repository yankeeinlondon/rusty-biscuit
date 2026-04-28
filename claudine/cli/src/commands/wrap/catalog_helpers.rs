//! Catalog-derived helpers for wrapper-profile dispatch.
//!
//! Phase 5 of the centralized providers refactor introduces these
//! standalone helpers as the seed surface for Phase 6 wrapper profile
//! thinning. Each helper reads typed data off
//! [`claudine::provider::provider_info`] and emits the argv / warnings
//! that a [`super::profile::WrapperProfile`] override would otherwise
//! recompute.
//!
//! These helpers are intentionally unused by the existing per-provider
//! `apply_*` implementations during Phase 5 to keep the wrap pipeline
//! snapshot-stable. Phase 6 will swap callers over to them and delete
//! the override implementations whose behavior matches the catalog
//! defaults.

use claudine::provider::{Provider, YoloSupport, provider_info};

/// Returns the argv tokens that toggle YOLO / auto-approve mode for
/// `provider`, derived from the typed [`YoloSupport`] catalog data.
///
/// Returns an empty vector for providers that have no YOLO surface
/// ([`YoloSupport::None`]) or providers that gate YOLO on a non-CLI
/// mechanism such as an environment variable.
#[allow(dead_code)] // Phase 6 will swap callers over to this helper.
pub(crate) fn yolo_args(provider: Provider) -> Vec<String> {
    match provider_info(provider).yolo {
        YoloSupport::None => Vec::new(),
        YoloSupport::DirectFlag { native_flag } => vec![native_flag.to_string()],
        YoloSupport::DirectFlagWithAlias { native_flag, .. } => vec![native_flag.to_string()],
        YoloSupport::NonInteractiveOnly { .. } => Vec::new(),
        YoloSupport::EnvVar { .. } => Vec::new(),
    }
}

/// Returns the argv tokens that toggle YOLO / auto-approve mode for
/// `provider` when running in the requested mode. Mirrors
/// [`yolo_args`] but additionally honors
/// [`YoloSupport::NonInteractiveOnly`] and surfaces the env-var pair for
/// [`YoloSupport::EnvVar`] callers to act on.
#[allow(dead_code)] // Phase 6 will swap callers over to this helper.
pub(crate) fn yolo_args_for_mode(provider: Provider, non_interactive: bool) -> Vec<String> {
    match provider_info(provider).yolo {
        YoloSupport::NonInteractiveOnly {
            non_interactive_flag,
        } if non_interactive => vec![non_interactive_flag.to_string()],
        _ => yolo_args(provider),
    }
}

/// Returns the env-var pair that enables YOLO mode for `provider`, if
/// the provider's YOLO surface is environment-variable based.
#[allow(dead_code)] // Phase 6 will swap callers over to this helper.
pub(crate) fn yolo_env_var(provider: Provider) -> Option<(&'static str, &'static str)> {
    match provider_info(provider).yolo {
        YoloSupport::EnvVar { env_var, value } => Some((env_var, value)),
        _ => None,
    }
}

/// Returns whether the provider exposes any YOLO surface at all.
#[allow(dead_code)] // Phase 6 will swap callers over to this helper.
pub(crate) fn supports_yolo(provider: Provider) -> bool {
    !matches!(provider_info(provider).yolo, YoloSupport::None)
}
