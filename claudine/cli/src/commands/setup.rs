//! `claudine setup`: Claudine's setup with Darkmatter
//! ([`claudine::config::setup::setup_claudine`]) as a command, for a home
//! that is already configured. It publishes the host registration, completes
//! the `permissions` handshake, registers the manifest in `~/.darkmatter.yaml`
//! in place of the retired `~/.claudine/darkmatter-host.yaml`, and removes the
//! retired files. Darkmatter's format 1 warning names this command.
//!
//! It never asks anything, so a script or an upgrade can run it, and a rerun
//! changes nothing. An existing Claudine configuration keeps every setting
//! (only the handshake may add its `permissions` block); with none, it writes
//! the same defaults as a first run without a terminal.

use clap::Args;
use color_eyre::eyre::Result;

use claudine::config::persist::SaveOutcome;
use claudine::config::schema::ConfigScope;

use crate::commands::init_wizard;
use crate::log;

/// Arguments for `claudine setup` (none).
#[derive(Debug, Args)]
pub struct SetupArgs {}

/// Runs setup for the user's configuration.
pub async fn run(_args: SetupArgs) -> Result<()> {
    let path = if crate::load_existing_config()? {
        // Claudine's own file is left as the user wrote it.
        init_wizard::write_with_darkmatter_setup(ConfigScope::User, |_| Ok(SaveOutcome::default()))?
    } else {
        init_wizard::run_headless_initialization().await?
    };
    log::message(&format!(
        "Claudine is set up with Darkmatter; its configuration is `{}`",
        biscuit_file::to_forward_slash_string(&path)
    ));
    Ok(())
}
