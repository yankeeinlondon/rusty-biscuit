use std::process::Command;
use tracing::warn;

use super::Service;

pub(crate) fn list_launchd_services() -> Vec<Service> {
    let output = match Command::new("launchctl").arg("list").output().map_err(|e| {
        warn!(error = %e, cmd = "launchctl", "service detection subprocess failed");
        e
    }) {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let pid = parts[0].parse::<u32>().ok();
            let status = parts[1].parse::<i32>().ok();
            let name = parts[2..].join(" ");

            services.push(Service {
                name,
                pid,
                running: pid.is_some(),
                status,
            });
        }
    }

    services
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_launchd_services_returns_services() {
        let services = list_launchd_services();
        assert!(
            !services.is_empty(),
            "launchctl list should return services on macOS"
        );

        for service in &services {
            assert!(!service.name.is_empty(), "Service name should not be empty");
        }
    }
}
