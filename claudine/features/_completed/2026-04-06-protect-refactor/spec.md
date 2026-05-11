# Protect Refactor

We have already refactored Protect once and we're going to do it again. This is a ground-up redesign that replaces the current 8-step evaluation pipeline, posture system, and severity matrix with a simple deny catalog.

## Goals

- Protect users, especially those running YOLO mode, from:
    - running dangerous commands in tool calls
    - writing/editing sensitive file paths
    - prompt injection attacks in MCP server responses
- Encourage users to feel more comfortable using YOLO mode
- DO NOT get in the way of execution — long-running and non-interactive sessions must complete without human intervention
- Protect is a safety net that catches genuinely dangerous actions, not a permission system

## Core Design Principles

1. **Default allow with curated deny catalog** — Protect only intervenes for genuinely destructive actions
2. **Block on match** — when an enabled rule matches, the action is blocked. No warn tier, no ask tier.
3. **No bypass** — if a rule blocks, the user must disable the rule in config to proceed. No per-invocation override.
4. **Per-rule opt-out** — each rule group can be individually toggled on/off
5. **No PolicyEngine dependency** — Protect is a standalone regex-based service. PolicyEngine continues to exist for other purposes but is not used by Protect.
6. **No postures, no severity, no capability downgrade** — the current complexity is removed entirely

## Architecture

### Scan Surfaces

Protect scans two surfaces:

1. **Bash tool arguments** — shell commands are matched against the command pattern catalog
2. **Write/Edit tool paths** — file paths are matched against a sensitive-path prefix list (~15 prefixes: `/etc/`, `/var/`, `/usr/`, `/boot/`, `~/.ssh/`, `~/.gnupg/`, `/dev/`, `/proc/`, `/sys/`, `/System/`, etc.)

MCP prompt injection detection is included as one of the rule groups, scanning MCP server responses.

### Rule Groups (~10 consolidated groups)

The ~80 patterns from [regexp.md](./regexp.md) are consolidated into approximately 10 groups. Each group is a single toggle:

| Group | What it covers |
|---|---|
| `filesystem_destruction` | `rm -rf`, `find -delete`, `shred`, recursive permission wipes |
| `disk_manipulation` | `mkfs`, `dd`, `fdisk`, `parted`, volume/ZFS destruction |
| `remote_execution` | `curl | bash`, `wget | sh`, reverse shells, fileless execution |
| `git_destructive` | `push --force`, `reset --hard`, `clean -fdx`, `branch -D`, reflog purge, `.git` deletion |
| `system_sabotage` | kernel module removal, bootloader destruction, service mass-termination, fork bombs |
| `network_sabotage` | firewall flush, interface shutdown, SSH key wipe |
| `container_cloud` | `docker system prune -a`, `kubectl delete namespaces`, cloud project/bucket deletion |
| `database_nukes` | `DROP DATABASE`, `redis-cli flushall`, package manager nukes |
| `obfuscated_execution` | base64 decode to shell, hex-to-bash, `eval $(echo ...)` |
| `prompt_injection` | Indirect injection, tool poisoning, semantic escape patterns, exfiltration via tool abuse |
| `credential_exfiltration` | Credential harvesting, data streaming over network, log/audit trail destruction |
| `sensitive_paths` | Write/Edit to `/etc/`, `~/.ssh/`, `/var/`, `/usr/`, `/boot/`, `/dev/`, `/proc/`, `/sys/`, `/System/` |

### OS-Aware Catalog

The pattern catalog is platform-filtered at load time:

- **macOS**: excludes Linux-only patterns (sysrq, swapoff, rmmod, modprobe, fdisk). Includes macOS-specific patterns (`diskutil`, `csrutil disable`, etc.)
- **Linux**: excludes macOS-only patterns. Includes Linux-specific patterns (sysrq, kernel modules, swap, etc.)
- **Cross-platform**: patterns like `rm -rf`, `git push --force`, `curl | bash`, Docker/K8s commands load on all platforms

### Path Allowlist (False Positive Handling)

Each rule group can have an `allow_paths` list. If a matched command targets an allowed path, the rule does not fire. This handles the common case of `rm -rf node_modules` or `rm -rf target`:

```json
{
  "protect": {
    "rules": {
      "filesystem_destruction": {
        "enabled": true,
        "allow_paths": ["node_modules", "target", "dist", "build", ".cache"]
      }
    }
  }
}
```

### Custom Patterns

Users can add their own regex patterns that behave identically to built-in rules (block on match):

```json
{
  "protect": {
    "custom_patterns": [
      { "name": "no_prod_deploy", "pattern": "deploy.*production" },
      { "name": "no_terraform_destroy", "pattern": "terraform\\s+destroy" }
    ]
  }
}
```

### Configuration

Minimal config — `"protect": true` enables all defaults:

```json
{
  "protect": true
}
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

### Reporting

When Protect blocks, it reports the group name, the specific pattern that matched, and the matched text:

```
[protect] BLOCKED
  Group: filesystem_destruction
  Rule:  rm\s+-rf\s+/\*?
  Match: "rm -rf /var/*"

  Disable group:
    protect.rules.filesystem_destruction = false
```

### Implementation Notes

- Use `RegexSet` for performance — compiles all patterns in a group into a single automaton, matches in one pass
- Patterns should use `\s+` instead of literal spaces to handle whitespace variations
- Patterns should be prepended with optional `(sudo\s+)?` where relevant
- The catalog is a compile-time asset, not loaded from disk at runtime

## What Gets Removed

The following concepts from the current implementation are removed entirely:

- `ProtectPosture` (Advisory/Balanced/Strict)
- `ProtectSeverity` (Info/Medium/High/Critical)
- The 8-step evaluation pipeline
- The decision matrix (effect x certainty x posture)
- Capability-aware downgrade
- `ProtectIntent` / `PolicyQuery` mapping
- YOLO-mode softening logic
- `ProtectState` / rolling decision records
- All PolicyEngine integration within Protect

## Regular Expressions

- Command patterns: [Regexp for Tool Calls](./regexp.md#dangerous-regexp-patterns-for-tool-calling)
- MCP injection patterns: [Regexp for MCP](./regexp.md#mcp-prompt-injection)

