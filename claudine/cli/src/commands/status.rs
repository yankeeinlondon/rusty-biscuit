use color_eyre::eyre::Result;

use claudine_lib::config::detect_agents;

/// Show registration status for all detected agents.
pub fn run() -> Result<()> {
    let agents = detect_agents();

    println!("Claudine Agent Status");
    println!("{}", "\u{2500}".repeat(50));
    println!("{:<15} {:<15}", "Provider", "Hooks Registered");
    println!("{}", "\u{2500}".repeat(50));

    for (provider, configurator) in &agents {
        let registered = match configurator.is_registered(None) {
            Ok(true) => "yes",
            Ok(false) => "no",
            Err(_) => "error",
        };
        println!("{provider:<15} {registered:<15}");
    }

    println!("{}", "\u{2500}".repeat(50));
    Ok(())
}
