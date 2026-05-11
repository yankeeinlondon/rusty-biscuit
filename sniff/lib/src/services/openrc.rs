use std::process::Command;
use tracing::warn;

use super::Service;

pub(crate) fn list_openrc_services() -> Vec<Service> {
    let output = match Command::new("rc-status")
        .arg("--all")
        .output()
        .map_err(|e| {
            warn!(error = %e, cmd = "rc-status", "service detection subprocess failed");
            e
        }) {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Runlevel:") || !line.contains('[') {
            continue;
        }

        if let Some(bracket_pos) = line.find('[') {
            let name = line[..bracket_pos].trim().to_string();
            let status_part = &line[bracket_pos..];

            let running = status_part.contains("started");

            if !name.is_empty() {
                services.push(Service {
                    name,
                    pid: None,
                    running,
                    status: None,
                });
            }
        }
    }

    services
}
