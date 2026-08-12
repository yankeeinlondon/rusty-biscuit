use std::collections::HashMap;
use std::{env, fs, path::PathBuf};

use tracing::{debug, warn};

use super::{Service, ENRICHMENT_CHUNK};
use crate::process::{self, timeouts};

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

    let mut names = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            names.push(name.to_string());
        }
    }

    let statuses = collect_runit_statuses(&names);

    names
        .into_iter()
        .map(|name| {
            let (running, pid) = statuses.get(&name).copied().unwrap_or((false, None));
            Service {
                name,
                pid,
                running,
                status: None,
            }
        })
        .collect()
}

/// Queries many runit services in a bounded number of subprocesses.
///
/// `sv status` accepts many service names and emits one line per service, so this
/// costs `ceil(services / ENRICHMENT_CHUNK)` spawns rather than one per service.
///
/// ## Returns
///
/// Service name to `(running, pid)`. Services absent from the map — because their
/// chunk failed or timed out — degrade to `(false, None)` at the call site, which
/// is exactly what a failed per-service `sv` produced before.
fn collect_runit_statuses(names: &[String]) -> HashMap<String, (bool, Option<u32>)> {
    let mut statuses = HashMap::new();

    for chunk in names.chunks(ENRICHMENT_CHUNK) {
        let mut args: Vec<&str> = vec!["status"];
        args.extend(chunk.iter().map(String::as_str));

        // `sv` exits non-zero when any named service is down, so exit status is not
        // a usable signal here — the per-line output is parsed regardless.
        let output = match process::run_with_timeout("sv", &args, timeouts::SERVICE_COMMAND) {
            Ok(o) => o,
            Err(e) => {
                warn!(error = %e, cmd = "sv", "service detection subprocess failed");
                continue;
            }
        };

        parse_sv_status_lines(&output.stdout_lossy(), &mut statuses);
    }

    statuses
}

/// Parses `sv status` output, one line per service.
///
/// Lines look like `run: nginx: (pid 1234) 56s` or `down: redis: 3s`.
fn parse_sv_status_lines(stdout: &str, statuses: &mut HashMap<String, (bool, Option<u32>)>) {
    for line in stdout.lines() {
        let line = line.trim();
        let Some((state, rest)) = line.split_once(": ") else {
            continue;
        };
        let Some((name, detail)) = rest.split_once(':') else {
            continue;
        };

        // `sv` echoes the name as given; a configured SVDIR can make it a path.
        let name = name
            .trim()
            .rsplit('/')
            .next()
            .unwrap_or(name)
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }

        let running = state == "run";
        let pid = if running {
            detail.find("(pid ").and_then(|start| {
                let rest = &detail[start + 5..];
                rest.find(')')
                    .and_then(|end| rest[..end].trim().parse::<u32>().ok())
            })
        } else {
            None
        };

        statuses.insert(name, (running, pid));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_batched_status_response() {
        let stdout = "run: nginx: (pid 1234) 56s\ndown: redis: 3s\nrun: sshd: (pid 90) 1s\n";
        let mut statuses = HashMap::new();
        parse_sv_status_lines(stdout, &mut statuses);

        assert_eq!(statuses.get("nginx"), Some(&(true, Some(1234))));
        assert_eq!(statuses.get("redis"), Some(&(false, None)));
        assert_eq!(statuses.get("sshd"), Some(&(true, Some(90))));
    }

    /// A configured `SVDIR` makes `sv` echo the name as a path.
    #[test]
    fn takes_the_basename_of_a_path_style_name() {
        let stdout = "run: /var/service/nginx: (pid 7) 2s\n";
        let mut statuses = HashMap::new();
        parse_sv_status_lines(stdout, &mut statuses);

        assert_eq!(statuses.get("nginx"), Some(&(true, Some(7))));
    }

    /// A running service without a reported PID is running, not down.
    #[test]
    fn a_run_line_without_a_pid_is_still_running() {
        let stdout = "run: odd: 5s\n";
        let mut statuses = HashMap::new();
        parse_sv_status_lines(stdout, &mut statuses);

        assert_eq!(statuses.get("odd"), Some(&(true, None)));
    }

    /// `sv` writes diagnostics for unknown services; they must not become entries.
    #[test]
    fn ignores_unparseable_lines() {
        let stdout = "warning: nosuch: unable to open supervise/ok\ngarbage\nrun: real: (pid 1) 1s\n";
        let mut statuses = HashMap::new();
        parse_sv_status_lines(stdout, &mut statuses);

        assert_eq!(statuses.get("real"), Some(&(true, Some(1))));
        assert_eq!(statuses.get("nosuch"), Some(&(false, None)));
    }
}
