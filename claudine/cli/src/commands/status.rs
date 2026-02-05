use color_eyre::eyre::Result;

use claudine_lib::config::detect_agents;

use crate::log;

/// Show registration status for all detected agents.
pub fn run() -> Result<()> {
    let agents = detect_agents();

    log::data("Claudine Agent Status");
    log::data(&"\u{2500}".repeat(50));
    log::data(&format!("{:<15} {:<15}", "Provider", "Hooks Registered"));
    log::data(&"\u{2500}".repeat(50));

    for (provider, configurator) in &agents {
        let registered = match configurator.is_registered(None) {
            Ok(true) => "yes",
            Ok(false) => "no",
            Err(_) => "error",
        };
        log::data(&format!("{provider:<15} {registered:<15}"));
    }

    log::data(&"\u{2500}".repeat(50));
    Ok(())
}
