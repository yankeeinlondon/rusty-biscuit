use std::collections::HashMap;

use tracing::warn;

use super::{Service, ENRICHMENT_CHUNK};
use crate::process::{self, timeouts};

pub(crate) fn list_systemd_services() -> Vec<Service> {
    let output = match process::run_with_timeout(
        "systemctl",
        &[
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--plain",
        ],
        timeouts::SERVICE_COMMAND,
    ) {
        Ok(o) if o.status.success() => o,
        Ok(_) => return Vec::new(),
        Err(e) => {
            warn!(error = %e, cmd = "systemctl", "service detection subprocess failed");
            return Vec::new();
        }
    };

    let stdout = output.stdout_lossy();
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

            services.push(Service {
                name,
                pid: None,
                running: active == "active" && sub == "running",
                status: None,
            });
        }
    }

    let pids = collect_systemd_pids(
        services
            .iter()
            .filter(|s| s.running)
            .map(|s| s.name.as_str()),
    );
    for service in &mut services {
        service.pid = pids.get(&service.name).copied();
    }

    services
}

/// Collects `MainPID` for many units in a bounded number of subprocesses.
///
/// `systemctl show` accepts many units at once and emits one property block per
/// unit, so this costs `ceil(units / ENRICHMENT_CHUNK)` spawns rather than one per
/// unit. `Id` is requested alongside `MainPID` because the blocks carry no other
/// marker tying a `MainPID=` back to the unit that owns it.
///
/// ## Returns
///
/// Service name (without the `.service` suffix) to PID, for units reporting a
/// nonzero `MainPID`. A chunk that fails or times out contributes nothing, which
/// leaves its services at `pid: None` — the same degradation a failed probe
/// produced before, and the reason a timeout cannot discard a healthy chunk.
fn collect_systemd_pids<'a>(names: impl Iterator<Item = &'a str>) -> HashMap<String, u32> {
    let units: Vec<String> = names.map(|n| format!("{n}.service")).collect();
    let mut pids = HashMap::new();

    for chunk in units.chunks(ENRICHMENT_CHUNK) {
        let mut args: Vec<&str> = vec!["show", "--property=Id", "--property=MainPID"];
        args.extend(chunk.iter().map(String::as_str));

        let output = match process::run_with_timeout("systemctl", &args, timeouts::SERVICE_COMMAND) {
            Ok(o) if o.status.success() => o,
            Ok(_) => continue,
            Err(e) => {
                warn!(error = %e, cmd = "systemctl show", "systemd PID enrichment failed for a chunk");
                continue;
            }
        };

        parse_systemd_show_blocks(&output.stdout_lossy(), &mut pids);
    }

    pids
}

/// Parses `systemctl show --property=Id --property=MainPID` output.
///
/// Blocks are separated by blank lines and property order within a block is not
/// guaranteed, so `Id` and `MainPID` are paired only at a block boundary.
fn parse_systemd_show_blocks(stdout: &str, pids: &mut HashMap<String, u32>) {
    let mut id: Option<String> = None;
    let mut pid: Option<u32> = None;

    let mut flush = |id: &mut Option<String>, pid: &mut Option<u32>| {
        // `MainPID=0` means "no main process", not PID zero.
        if let (Some(unit), Some(p)) = (id.take(), pid.take())
            && p > 0
        {
            pids.insert(unit.trim_end_matches(".service").to_string(), p);
        }
    };

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            flush(&mut id, &mut pid);
        } else if let Some(value) = line.strip_prefix("Id=") {
            id = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("MainPID=") {
            pid = value.parse().ok();
        }
    }
    flush(&mut id, &mut pid);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives the real backend against a `systemctl` shim on `PATH`, which logs
    /// one line per invocation.
    ///
    /// ## Returns
    ///
    /// The detected services and every argv the shim was called with.
    ///
    /// ## Notes
    ///
    /// Unix-only: the shim is a shell script, and systemd is a Linux backend that
    /// no Windows host would exercise anyway.
    #[cfg(unix)]
    fn run_against_shim(running_units: usize) -> (Vec<Service>, Vec<String>) {
        use std::os::unix::fs::PermissionsExt;

        use crate::test_helpers::ENV_MUTEX;

        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().expect("tempdir");
        let log = dir.path().join("calls.log");

        // `list-units` prints N running units; `show` echoes an Id/MainPID block
        // per unit argument. Both record their argv so the test can count spawns.
        let script = format!(
            r#"#!/bin/sh
echo "$@" >> "{log}"
if [ "$1" = "list-units" ]; then
  i=1
  while [ $i -le {running_units} ]; do
    echo "svc$i.service loaded active running Service $i"
    i=$((i+1))
  done
  exit 0
fi
if [ "$1" = "show" ]; then
  shift 3
  for unit in "$@"; do
    echo "Id=$unit"
    echo "MainPID=100"
    echo ""
  done
  exit 0
fi
exit 1
"#,
            log = log.display(),
            running_units = running_units,
        );
        // Written directly rather than via `test_helpers::make_executable_fixture`,
        // which supplies its own no-op body and would clobber this script.
        let shim = dir.path().join("systemctl");
        std::fs::write(&shim, script).expect("write shim");
        let mut perms = std::fs::metadata(&shim).expect("shim metadata").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&shim, perms).expect("set shim mode");

        let original = std::env::var_os("PATH");
        let mut paths = vec![dir.path().to_path_buf()];
        if let Some(p) = &original {
            paths.extend(std::env::split_paths(p));
        }
        let joined = std::env::join_paths(paths).expect("join PATH");
        // SAFETY: ENV_MUTEX serializes every env-mutating test in the crate.
        unsafe { std::env::set_var("PATH", &joined) };

        let services = list_systemd_services();

        unsafe {
            match original {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }

        let calls = std::fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect();
        (services, calls)
    }

    /// PID enrichment was one `systemctl show` per running service. It must now be
    /// one per chunk — the whole point of R12.4.
    #[cfg(unix)]
    #[test]
    fn pid_enrichment_costs_one_subprocess_per_chunk_not_per_service() {
        let units = 300;
        let (services, calls) = run_against_shim(units);

        assert_eq!(services.len(), units);
        assert!(services.iter().all(|s| s.pid == Some(100)));

        let show_calls = calls.iter().filter(|c| c.starts_with("show ")).count();
        let expected = units.div_ceil(ENRICHMENT_CHUNK);
        assert_eq!(
            show_calls, expected,
            "300 running services must cost {expected} show calls, not 300"
        );
        // One listing plus the enrichment chunks; nothing else.
        assert_eq!(calls.len(), 1 + expected);
    }

    /// Chunking exists for command-line length only, so a service count under the
    /// chunk size must collapse to exactly one enrichment call.
    #[cfg(unix)]
    #[test]
    fn a_small_service_count_needs_a_single_enrichment_call() {
        let (services, calls) = run_against_shim(3);

        assert_eq!(services.len(), 3);
        assert_eq!(calls.len(), 2, "one list-units + one show");
    }

    /// R12.11: a failed primary listing returns the backend's existing empty
    /// result, and must not go on to enrich anything.
    #[cfg(unix)]
    #[test]
    fn a_failed_primary_listing_returns_empty_and_enriches_nothing() {
        // Zero running units makes the shim's `list-units` print nothing; a real
        // failure is covered by the `Ok(_) => return Vec::new()` arm above. Here we
        // assert the no-running-services path performs no enrichment spawn at all.
        let (services, calls) = run_against_shim(0);

        assert!(services.is_empty());
        assert_eq!(calls.len(), 1, "no running services means no `show` call");
    }

    #[test]
    fn parses_a_multi_unit_show_block() {
        let stdout = "Id=nginx.service\nMainPID=1234\n\nId=redis.service\nMainPID=5678\n";
        let mut pids = HashMap::new();
        parse_systemd_show_blocks(stdout, &mut pids);

        assert_eq!(pids.get("nginx"), Some(&1234));
        assert_eq!(pids.get("redis"), Some(&5678));
    }

    /// `MainPID=0` means "no main process", not PID zero.
    #[test]
    fn drops_units_reporting_pid_zero() {
        let stdout = "Id=stopped.service\nMainPID=0\n\nId=live.service\nMainPID=9\n";
        let mut pids = HashMap::new();
        parse_systemd_show_blocks(stdout, &mut pids);

        assert!(!pids.contains_key("stopped"));
        assert_eq!(pids.get("live"), Some(&9));
    }

    /// Property order within a block is not guaranteed by systemd.
    #[test]
    fn pairs_properties_regardless_of_order() {
        let stdout = "MainPID=42\nId=reversed.service\n";
        let mut pids = HashMap::new();
        parse_systemd_show_blocks(stdout, &mut pids);

        assert_eq!(pids.get("reversed"), Some(&42));
    }

    /// A block missing its partner property must not pair across the boundary and
    /// steal the next block's value.
    #[test]
    fn does_not_pair_across_a_block_boundary() {
        let stdout = "Id=orphan.service\n\nMainPID=99\n\nId=other.service\nMainPID=7\n";
        let mut pids = HashMap::new();
        parse_systemd_show_blocks(stdout, &mut pids);

        assert!(!pids.contains_key("orphan"));
        assert_eq!(pids.get("other"), Some(&7));
        assert_eq!(pids.len(), 1);
    }
}
