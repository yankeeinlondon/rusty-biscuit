use std::process::Command;
use tracing::warn;

use super::Service;

pub(crate) fn list_systemd_services() -> Vec<Service> {
    let output = match Command::new("systemctl")
        .args([
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--plain",
        ])
        .output()
        .map_err(|e| {
            warn!(error = %e, cmd = "systemctl", "service detection subprocess failed");
            e
        }) {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let unit = parts[0];
            if !unit.ends_with(".service") {
                continue;
            }

            let name = unit.trim_end_matches(".service").to_string();
            let active = parts[2];
            let sub = parts[3];

            let running = active == "active" && sub == "running";
            let pid = if running {
                get_systemd_service_pid(&name)
            } else {
                None
            };

            services.push(Service {
                name,
                pid,
                running,
                status: None,
            });
        }
    }

    services
}

fn get_systemd_service_pid(service_name: &str) -> Option<u32> {
    let output = Command::new("systemctl")
        .args([
            "show",
            &format!("{}.service", service_name),
            "--property=MainPID",
        ])
        .output()
        .map_err(|e| {
            warn!(error = %e, cmd = "systemctl", "service detection subprocess failed");
            e
        })
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .trim()
        .strip_prefix("MainPID=")
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&pid| pid > 0)
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;

    #[test]
    fn test_systemd_services_if_available() {
        use crate::services::{InitSystem, ServiceManager};
        let sm = ServiceManager::detect();
        if sm.init_system == InitSystem::Systemd {
            let services = list_systemd_services();
            assert!(!services.is_empty(), "systemd should return services");
        }
    }
}
