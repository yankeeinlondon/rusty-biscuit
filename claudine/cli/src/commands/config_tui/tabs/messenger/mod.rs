use super::super::app::App;
use claudine::config::claudine_config::{ClaudineMessengerConfig, MessengerProviderConfig};

pub mod masked_input;
pub mod redaction;
pub mod routes;
pub mod test_connection;

mod input;
mod modal;
mod render;

// Public tab API, re-exported from the focused submodules so external callers
// (`tabs::messenger::render`, `app.rs` key dispatch) keep their paths.
pub use input::{handle_key, handle_messenger_input_modal};
pub use modal::{handle_messenger_add_modal, handle_messenger_select_modal};
pub use render::render;

// Surfaced for the `tests` module, which reaches them via `use super::*`.
#[cfg(test)]
pub(super) use super::super::app::{AppMode, ModalState};
#[cfg(test)]
pub(super) use routes::{PROVIDERS, build_messenger_from_fields};
#[cfg(test)]
pub(super) use test_connection::build_test_route_from_modal;

pub(super) fn sorted_messenger_configs(app: &App) -> Vec<(String, &MessengerProviderConfig)> {
    let mut configs: Vec<_> = app
        .config
        .messenger
        .as_ref()
        .map(|m| {
            m.configurations
                .iter()
                .map(|(name, config)| (name.clone(), config))
                .collect()
        })
        .unwrap_or_default();
    configs.sort_by(|left, right| left.0.cmp(&right.0));
    configs
}

pub(super) fn sorted_messenger_names(app: &App) -> Vec<String> {
    sorted_messenger_configs(app)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

pub(super) fn ensure_messenger_config(app: &mut App) {
    if app.config.messenger.is_none() {
        app.config.messenger = Some(ClaudineMessengerConfig {
            active_config: None,
            configurations: Default::default(),
        });
    }
}

#[cfg(test)]
mod tests;
