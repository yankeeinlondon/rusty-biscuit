//! Services section output formatting.

use std::fmt::Write;

use sniff::services::{Service, ServiceState, ServicesInfo};

/// Render services information as text.
pub fn render_services_text(info: &ServicesInfo, verbose: u8, state_filter: ServiceState) -> String {
    let mut out = String::new();
    writeln!(out, "=== Services ===").unwrap();
    writeln!(out, "Init System: {}", info.init_system).unwrap();
    writeln!(out, "Host OS: {}", info.host_os).unwrap();

    // Show evidence at verbose level 1+
    if verbose > 0 && !info.evidence.hints.is_empty() {
        writeln!(out, "\nDetection hints:").unwrap();
        for hint in &info.evidence.hints {
            writeln!(out, "  - {}", hint).unwrap();
        }
    }
    if verbose > 0 && !info.evidence.notes.is_empty() {
        writeln!(out, "\nNotes:").unwrap();
        for note in &info.evidence.notes {
            writeln!(out, "  - {}", note).unwrap();
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

    writeln!(out).unwrap();
    match state_filter {
        ServiceState::All => {
            writeln!(
                out,
                "Services: {} total ({} running, {} stopped)",
                info.services.len(),
                total_running,
                total_stopped
            ).unwrap();
        }
        ServiceState::Running => {
            writeln!(out, "Running Services: {}", filtered.len()).unwrap();
        }
        ServiceState::Stopped => {
            writeln!(out, "Stopped Services: {}", filtered.len()).unwrap();
        }
        ServiceState::Initializing => {
            writeln!(out, "Services: {}", filtered.len()).unwrap();
        }
    }

    if filtered.is_empty() {
        writeln!(out, "  (none)").unwrap();
    } else {
        // Show services (limit to 20 at verbose 0, all at verbose 1+)
        let show_count = if verbose > 0 {
            filtered.len()
        } else {
            20.min(filtered.len())
        };

        for service in filtered.iter().take(show_count) {
            let status = if service.running {
                "running"
            } else {
                "stopped"
            };
            let pid_str = service
                .pid
                .map(|p| format!(" (PID {})", p))
                .unwrap_or_default();
            writeln!(out, "  {} [{}]{}", service.name, status, pid_str).unwrap();
        }

        if filtered.len() > show_count {
            writeln!(out, "  ... and {} more", filtered.len() - show_count).unwrap();
        }
    }

    // When showing running services, also show stopped count
    if state_filter == ServiceState::Running && total_stopped > 0 {
        writeln!(out).unwrap();
        writeln!(out, "Stopped Services: {}", total_stopped).unwrap();
    }

    writeln!(out).unwrap();
    out
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
