use clap::Args;
use color_eyre::eyre::Result;

use claudine::config::detect_agents;

use crate::log;

/// Arguments for the uninstall subcommand.
#[derive(Args)]
pub struct UninstallArgs {
    /// Keep config files, only remove hook registrations.
    #[arg(long)]
    pub keep_config: bool,
}

/// Remove Claudine hooks from all agents.
pub fn run(args: UninstallArgs) -> Result<()> {
    let agents = detect_agents();

    log::message("Removing Claudine hooks...");

    for (provider, configurator) in &agents {
        match configurator.deregister(None) {
            Ok(()) => {
                log::message(&format!("  {provider}: hooks removed"));
            }
            Err(e) => {
                log::error(&format!("  {provider}: {e}"));
            }
        }
    }

    if !args.keep_config {
        // Remove config files
        if let Some(home) = dirs::home_dir() {
            let config_path = home.join(".claudine").join("config.json");
            if config_path.exists() {
                std::fs::remove_file(&config_path)?;
                log::message(&format!(
                    "  Removed {}",
                    biscuit_file::to_portable_string(&config_path)
                ));
            }
        }
    }

    log::message("");
    log::message("Claudine has been uninstalled.");
    if args.keep_config {
        log::message("Config files preserved. Run `claudine sync` to re-register hooks.");
    }
    Ok(())
}
