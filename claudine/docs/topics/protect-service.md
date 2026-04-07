# Protect Service

Protect is a standalone deny-catalog service that blocks dangerous actions
during agentic CLI sessions. It scans three surfaces and returns a binary
Allow or Block decision.

## Scan Surfaces

| Surface | What is scanned | When |
|---------|----------------|------|
| Bash commands | Shell command strings | Before tool execution |
| Write/Edit paths | Target file paths | Before tool execution |
| MCP responses | Response payloads from MCP servers | After tool execution |

## Rule Groups

| Group | What it covers |
|-------|---------------|
| `filesystem_destruction` | `rm -rf`, `find -delete`, `shred`, recursive permission wipes |
| `disk_manipulation` | `mkfs`, `dd`, `fdisk`, `parted`, volume/ZFS destruction |
| `remote_execution` | `curl \| bash`, `wget \| sh`, reverse shells |
| `git_destructive` | `push --force`, `reset --hard`, `clean -fdx`, `branch -D` |
| `system_sabotage` | kernel module removal, fork bombs, bootloader destruction |
| `network_sabotage` | firewall flush, interface shutdown, SSH key wipe |
| `container_cloud` | `docker system prune -a`, `kubectl delete namespaces`, cloud deletions |
| `database_nukes` | `DROP DATABASE`, `redis-cli flushall` |
| `obfuscated_execution` | base64/hex decode to shell, `eval $(echo ...)` |
| `prompt_injection` | Indirect injection, tool poisoning, semantic escapes |
| `credential_exfiltration` | Credential harvesting, data streaming, audit trail destruction |
| `sensitive_paths` | Write/Edit to `/etc/`, `~/.ssh/`, `/var/`, `/usr/`, `/boot/`, etc. |

## Configuration

Shorthand (enables all defaults):

```json
{ "protect": true }
```

Per-group toggles:

```json
{
  "protect": {
    "rules": {
      "git_destructive": false,
      "filesystem_destruction": {
        "enabled": true,
        "allow_paths": ["node_modules", "target"]
      }
    }
  }
}
```

Custom patterns:

```json
{
  "protect": {
    "custom_patterns": [
      { "name": "no_prod_deploy", "pattern": "deploy.*production" }
    ]
  }
}
```

## Decision Model

Every evaluation returns exactly one of:

- **Allow** — action proceeds normally
- **Block** — action is stopped, user sees the matched rule details

There are no advisory, ask, or warn tiers.

## Blocked Output

When blocked, the output includes group, rule, pattern, matched text, and
the config key to disable the group:

```text
[protect] BLOCKED
  Group: filesystem_destruction
  Rule: rm_recursive_force
  Pattern: (sudo\s+)?rm\s+(-\w+\s+)*-\w*[rR]\w*[fF]
  Match: "rm -rf /var/*"

  Disable group:
    protect.rules.filesystem_destruction = false
```

## Module Layout

| Module | Responsibility |
|--------|---------------|
| `catalog.rs` | Rule definitions, group enum, platform filtering |
| `config.rs` | Flat config schema, validation |
| `matcher.rs` | `RegexSet` compilation, evaluation flow |
| `path.rs` | Path normalization, sensitive prefix checks, `allow_paths` |
| `decision.rs` | `ProtectOutcome`, `ProtectMatch`, `ProtectDecision` |
| `service.rs` | Orchestration: `ProtectService::new()`, `evaluate()` |
| `observe.rs` | Event-to-request extraction |
| `report.rs` | Blocked message formatting |

## What Was Removed

- `ProtectPosture` (Advisory/Balanced/Strict)
- `ProtectSeverity` (Info/Medium/High/Critical)
- 8-step evaluation pipeline
- Decision matrix (effect x certainty x posture)
- Capability-aware downgrade
- `ProtectIntent` / `PolicyQuery` mapping
- YOLO-mode softening
- `ProtectState` / rolling decision records
- All PolicyEngine integration within Protect
- MCP redaction (replaced by block)
