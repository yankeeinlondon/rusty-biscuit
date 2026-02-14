# Service Detection

System service detection across multiple init systems.

## Supported Init Systems

| System | OS | Detection Method |
|--------|:--:|------------------|
| systemd | Linux | `systemctl` |
| launchd | macOS | `launchctl` |
| OpenRC | Linux | `/sbin/openrc` |
| runit | Linux | `/etc/runit/` |
| S6 | Linux | `s6-rc` |
| Dinit | Linux | `dinitctl` |
| Upstart | Linux | `initctl` |
| BusyboxInit | Linux | BusyBox init |
| ContainerMinimalInit | Linux | Container init (tini, dumb-init) |
| WindowsScm | Windows | Service Control Manager |

## Key Types

```rust
use sniff_lib::services::{detect_services, ServicesInfo, Service, ServiceState};

// Detect everything
let info: ServicesInfo = detect_services();

if let Some(init) = &info.init_system {
    println!("Init system: {:?}", init);
}

// Filter by state
let running: Vec<_> = info.services
    .iter()
    .filter(|s| s.running)
    .collect();

println!("Running services: {}", running.len());
```

## Service States

| State | Description |
|-------|-------------|
| `Running` | Currently active |
| `Stopped` | Not running |
| `Initializing` | Starting up |
| `All` | Filter: show all services |

## CLI Subcommand

```bash
sniff services                       # Running services (default, text output)
sniff services --state all           # All services
sniff services --state running       # Only running
sniff services --state stopped       # Only stopped
sniff services --json                # JSON output
```

## Evidence Tracking

Detection uses multiple fallback methods with evidence tracking:

```rust
use sniff_lib::services::ServiceManager;

let mgr = ServiceManager::detect();
println!("Init: {:?}", mgr.init_system);
println!("Evidence: {:?}", mgr.evidence);
```
