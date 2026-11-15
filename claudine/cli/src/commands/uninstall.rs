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
        // `config.json` is the legacy file a load converts; remove it too.
        if let Some(home) = biscuit_file::home_dir() {
            let config_path = claudine::dispatch::loader::user_config_path();
            let legacy_path = home.join(".claudine").join("config.json");
            for path in [config_path, legacy_path] {
                if path.exists() {
                    std::fs::remove_file(&path)?;
                    log::message(&format!(
                        "  Removed {}",
                        biscuit_file::to_portable_string(&path)
                    ));
                }
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
