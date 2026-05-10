use std::{env, fs, path::PathBuf, process::Command};
use tracing::{debug, warn};

use super::Service;

pub(crate) fn list_runit_services() -> Vec<Service> {
    let sv_dir = env::var("SVDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/service"));

    let entries = match fs::read_dir(&sv_dir).map_err(|e| {
        debug!(path = %sv_dir.display(), error = %e, "could not read service file");
        e
    }) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut services = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let (running, pid) = check_runit_service_status(&name);

        services.push(Service {
            name,
            pid,
            running,
            status: None,
        });
    }

    services
}

fn check_runit_service_status(service_name: &str) -> (bool, Option<u32>) {
    let output = match Command::new("sv")
        .args(["status", service_name])
        .output()
        .map_err(|e| {
            warn!(error = %e, cmd = "sv", "service detection subprocess failed");
            e
        }) {
        Ok(o) => o,
        Err(_) => return (false, None),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    let running = stdout.starts_with("run:");

    let pid = if running {
        stdout.find("(pid ").and_then(|start| {
            let rest = &stdout[start + 5..];
            rest.find(')')
                .and_then(|end| rest[..end].parse::<u32>().ok())
        })
    } else {
        None
    };

    (running, pid)
}
