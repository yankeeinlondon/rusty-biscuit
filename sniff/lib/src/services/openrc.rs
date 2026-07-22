use tracing::warn;

use super::Service;
use crate::process::{self, timeouts};

pub(crate) fn list_openrc_services() -> Vec<Service> {
    let output = match process::run_with_timeout("rc-status", &["--all"], timeouts::SERVICE_COMMAND)
    {
        Ok(o) if o.status.success() => o,
        Ok(_) => return Vec::new(),
        Err(e) => {
            warn!(error = %e, cmd = "rc-status", "service detection subprocess failed");
            return Vec::new();
        }
    };

    let stdout = output.stdout_lossy();
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
