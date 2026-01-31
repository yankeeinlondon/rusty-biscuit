# Service Detection

System service detection across multiple init systems.

## Supported Init Systems

| System | OS | Detection Method |
|--------|:--:|------------------|
| systemd | Linux | `systemctl` |
| launchd | macOS | `launchctl` |
| OpenRC | Linux | `/sbin/openrc` |
| SysVinit | Linux | `/etc/init.d/` |
| runit | Linux | `/etc/runit/` |
| s6 | Linux | `/etc/s6/` |

## Usage

```rust
use sniff_lib::services::{detect_init_system, list_services, ServiceState};

// Detect init system
let init = detect_init_system();
println!("Init system: {:?}", init);

// List services
let services = list_services()?;
for svc in services {
    println!("{}: {:?}", svc.name, svc.state);
}
```

## Service States

| State | Description |
|-------|-------------|
| `Running` | Currently active |
| `Stopped` | Not running |
| `Failed` | Exited with error |
| `Unknown` | State cannot be determined |

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
pub struct InitSystemEvidence {
    pub system: InitSystem,
    pub detection_method: &'static str,
    pub confidence: Confidence,
}
```

Confidence levels: `High`, `Medium`, `Low`
