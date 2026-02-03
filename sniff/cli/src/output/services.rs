//! Services section output formatting.

use sniff_lib::services::{Service, ServiceState, ServicesInfo};

/// Print services information as text.
pub fn print_services_text(info: &ServicesInfo, verbose: u8, state_filter: ServiceState) {
    println!("=== Services ===");
    println!("Init System: {}", info.init_system);
    println!("Host OS: {}", info.host_os);

    // Show evidence at verbose level 1+
    if verbose > 0 && !info.evidence.hints.is_empty() {
        println!("\nDetection hints:");
        for hint in &info.evidence.hints {
            println!("  - {}", hint);
        }
    }
    if verbose > 0 && !info.evidence.notes.is_empty() {
        println!("\nNotes:");
        for note in &info.evidence.notes {
            println!("  - {}", note);
        }
    }

    // Count services by state from full list
    let total_running = info.services.iter().filter(|s| s.running).count();
    let total_stopped = info.services.len() - total_running;

    // Filter services based on state
    let filtered: Vec<&Service> = info
        .services
        .iter()
        .filter(|s| match state_filter {
            ServiceState::All => true,
            ServiceState::Running => s.running,
            ServiceState::Stopped => !s.running,
            ServiceState::Initializing => false, // Not applicable to listed services
        })
        .collect();

    println!();
    match state_filter {
        ServiceState::All => {
            println!(
                "Services: {} total ({} running, {} stopped)",
                info.services.len(),
                total_running,
                total_stopped
            );
        }
        ServiceState::Running => {
            println!("Running Services: {}", filtered.len());
        }
        ServiceState::Stopped => {
            println!("Stopped Services: {}", filtered.len());
        }
        ServiceState::Initializing => {
            println!("Services: {}", filtered.len());
        }
    }

    if filtered.is_empty() {
        println!("  (none)");
    } else {
        // Show services (limit to 20 at verbose 0, all at verbose 1+)
        let show_count = if verbose > 0 {
            filtered.len()
        } else {
            20.min(filtered.len())
        };

        for service in filtered.iter().take(show_count) {
            let status = if service.running { "running" } else { "stopped" };
            let pid_str = service
                .pid
                .map(|p| format!(" (PID {})", p))
                .unwrap_or_default();
            println!("  {} [{}]{}", service.name, status, pid_str);
        }

        if filtered.len() > show_count {
            println!("  ... and {} more", filtered.len() - show_count);
        }
    }

    // When showing running services, also show stopped count
    if state_filter == ServiceState::Running && total_stopped > 0 {
        println!();
        println!("Stopped Services: {}", total_stopped);
    }

    println!();
}

/// Print services information as JSON.
pub fn print_services_json(
    info: &ServicesInfo,
    state_filter: ServiceState,
) -> serde_json::Result<()> {
    // Filter services based on state
    let filtered: Vec<&Service> = info
        .services
        .iter()
        .filter(|s| match state_filter {
            ServiceState::All => true,
            ServiceState::Running => s.running,
            ServiceState::Stopped => !s.running,
            ServiceState::Initializing => false,
        })
        .collect();

    // Build output structure
    let output = serde_json::json!({
        "init_system": info.init_system.to_string(),
        "host_os": info.host_os.to_string(),
        "evidence": {
            "hints": info.evidence.hints,
            "notes": info.evidence.notes,
        },
        "services": filtered,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
