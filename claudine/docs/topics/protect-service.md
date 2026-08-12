# Protect Service

Protect is Claudine's standalone deny-catalog runtime safety layer. It blocks
common dangerous actions during live agent sessions using a binary decision
model: every evaluation returns either `Allow` or `Block`.

Protect is deliberately **best-effort defense-in-depth**, not a security
boundary. Provider permission systems and `claudine-contract` filesystem
sandboxing remain the load-bearing controls. The catalog catches obvious
destructive patterns but does not perform shell-aware parsing, variable
expansion, or exhaustive command-syntax analysis.

The refactor implemented on 2026-04-06 removed the old posture/severity
pipeline entirely. Protect no longer depends on `PolicyEngine`, no longer
downgrades outcomes based on provider capabilities, and no longer has advisory,
warn, ask, or redaction modes.

## Decision Model

- `Allow` means execution continues unchanged.
- `Block` means the action is denied and Claudine returns the matched rule data.

If a rule matches, there is no per-invocation bypass. To proceed, the user must
disable the relevant rule group in config.

## Scan Surfaces

Protect evaluates three runtime surfaces:

| Surface | What is scanned | How it is extracted |
|---------|-----------------|---------------------|
| `bash_command` | Shell command strings | Tool names containing `bash`, `shell`, or `exec`, plus `run_command` and `terminal`; command keys `command`, `cmd`, `script`, `input`, or a string array |
| `write_path` | Target file paths | Tool names containing `write`, `edit`, `create`, or `delete`; path keys `path`, `file_path`, `file`, `target`, `filename`, `dest`, or `paths[]` |
| `mcp_response` | MCP response string payloads | Only MCP-backed tool responses; JSON responses are scanned by walking individual string leaves |

If a bash- or write-shaped tool is recognized but the relevant payload cannot
be extracted, Protect reports an `Unparsed` observation. At the dispatch
boundary this is treated defensively: the tool is blocked with a warning and a
synthetic `unparsed_*` match, while unrelated tools continue normally.

Two details matter in practice:

- Protect pre-checks run even when no event binding exists for the current provider/event.
- MCP payloads are scanned field-by-field, not by concatenating the full JSON response, which avoids cross-field false positives.

MCP responses come from untrusted servers, so the scan is bounded: at most
10,000 string leaves totalling 1 MiB, with any single leaf truncated to 64 KiB.
The built-in patterns run in linear time (Rust's `regex` has no catastrophic
backtracking), so these caps simply bound the per-response cost against a hostile
multi-megabyte body. Truncation only shortens what is scanned — a match in the
surviving prefix still blocks — and a `warn!` is emitted whenever a cap clips the
input.

## Built-in Rule Groups

The shipped catalog exposes 12 built-in groups plus optional user-defined
custom command patterns:

| Group | What it covers |
|-------|----------------|
| `filesystem_destruction` | `rm -rf`, `find -delete`, `shred`, recursive permission/ownership wipes |
| `disk_manipulation` | `mkfs`, `dd` to device nodes, `fdisk`, `parted`, LVM/ZFS destruction, macOS `diskutil` erase |
| `remote_execution` | `curl \| bash`, `wget \| sh`, process-substitution shells, reverse shells |
| `git_destructive` | `push --force`, `reset --hard`, `clean -fdx`, `branch -D`, reflog expiration, `.git` deletion |
| `system_sabotage` | fork bombs, shutdown/reboot, service mass termination, bootloader damage, platform-specific kernel sabotage |
| `network_sabotage` | firewall flushes, interface shutdown, route deletion, SSH credential removal |
| `container_cloud` | destructive Docker, Kubernetes, and cloud project/bucket/database operations |
| `database_nukes` | `DROP DATABASE`, `TRUNCATE`, Redis flushes, destructive package-manager cleanup |
| `obfuscated_execution` | base64/hex decode to shell, `eval $(echo ...)`, hidden shell payload execution |
| `prompt_injection` | indirect prompt injection and tool-poisoning patterns in MCP responses |
| `credential_exfiltration` | credential harvesting, token scraping, outbound exfiltration, history/log destruction |
| `sensitive_paths` | writes or edits to protected filesystem prefixes |

User-defined `custom_patterns` are compiled as an additional rule group. Each
pattern may optionally declare a `surface` (`bash_command` or `mcp_response`);
when omitted it defaults to `bash_command`. `write_path` custom patterns are
not supported.

## Platform Filtering

Protect compiles an OS-aware catalog at startup:

- `ProtectPlatform::MacOs` loads macOS-specific rules such as `diskutil` and excludes Linux-only rules.
- `ProtectPlatform::Linux` loads Linux-specific rules such as `fdisk`, `rmmod`, and Linux device patterns.
- Cross-platform rules such as `rm -rf`, `git push --force`, and `curl | bash` load everywhere.

## Sensitive Path Handling

Path checks are implemented lexically in `path.rs`. Relative write paths are
resolved against the event `cwd`, `~` is expanded, and `.` / `..` segments are
collapsed before matching.

Path comparison accepts `/` and `\` rule separators on every host. Exact,
directory-prefix, sensitive-prefix, and allow-path matches require a path-segment
boundary, so a rule for `C:\proj` includes `C:\proj\src` but not `C:\proj2`.

Current built-in sensitive prefixes are:

- Absolute prefixes: `/bin`, `/boot`, `/dev`, `/etc`, `/opt`, `/proc`, `/root`,
  `/sbin`, `/sys`, `/System/Library`, `/System/Applications`,
  `/Library/LaunchDaemons`, `/usr`, `/var`
- Home-relative prefixes: `~/.aws`, `~/.claude`, `~/.codex`, `~/.config/gh`,
  `~/.docker/config.json`, `~/.gemini`, `~/.git-credentials`, `~/.gnupg`,
  `~/.goose`, `~/.kube`, `~/.netrc`, `~/.npmrc`, `~/.opencode`, `~/.qwen`,
  `~/.roo`, `~/.ssh`

## Configuration

`settings.protect` accepts either shorthand boolean form or an object.

Shorthand:

```json
{ "protect": true }
```

Expanded form:

```json
{
  "protect": {
    "enabled": true,
    "rules": {
      "git_destructive": false,
      "filesystem_destruction": {
        "enabled": true,
        "allow_paths": ["node_modules", "target"]
      }
    },
    "custom_patterns": [
      { "name": "no_prod_deploy", "pattern": "deploy.*production" }
    ]
  }
}
```

Important config semantics:

- Unknown top-level keys under `protect` are rejected.
- Removed fields such as `posture` are rejected rather than ignored.
- Any built-in group can be toggled with `false` or `{ "enabled": false }`.
- `allow_paths` is only supported for `filesystem_destruction` and `sensitive_paths`.
- `custom_patterns` must be valid regexes at config load time.

## `allow_paths`

`allow_paths` exists to suppress common false positives such as
`rm -rf node_modules` or writes into a known-safe subtree.

Command-path matching behavior:

- Relative allow entries such as `node_modules` or `target` match only as an
  anchored component-sequence prefix of the target. `allow_paths = ["build"]`
  allows `build/output.o` but does **not** allow `/etc/build/passwd`.
- Absolute allow entries match the exact path or a descendant path with a
  component boundary, so `/var/tmp` does not permit `/var/tmpevil`. POSIX,
  Windows drive-rooted, and UNC spellings are recognized portably; drive-relative
  spellings such as `C:tmp` remain relative.
- For destructive bash commands, all extracted target operands must be allowed
  or the rule still blocks.

Some rules whose target grammar is not parsed correctly by the `rm`-operand
heuristic (for example `find ... -delete`, `chmod`, `chown`) declare
`supports_allow_paths = false`. For those rules, `allow_paths` is ignored and
Protect blocks regardless of the allow list.

Example:

```json
{
  "protect": {
    "rules": {
      "filesystem_destruction": {
        "enabled": true,
        "allow_paths": ["node_modules", "target", "dist", "build", ".cache"]
      },
      "sensitive_paths": {
        "enabled": true,
        "allow_paths": ["/etc/my-safe-generated-dir"]
      }
    }
  }
}
```

## Config Merge Semantics

When both user config and repo config define Protect settings, Claudine merges
them with Protect-specific rules:

- `enabled` becomes `user.enabled || repo.enabled`
- per-group toggles are merged slot-by-slot, with repo values overriding user values
- `custom_patterns` are combined as `repo + user`

This differs from the retired Protect implementation and is important when
documenting repo-scoped safety defaults.

## Runtime Integration

Protect is wired into dispatch in two places:

1. Pre-evaluation runs before binding lookup and before actions execute.
2. Post-evaluation runs on MCP-backed tool responses before the final hook response is returned.

If Protect blocks in either phase, Claudine maps the result into a provider
deny response with structured raw metadata:

```json
{
  "protect": {
    "outcome": "block",
    "group": "filesystem_destruction",
    "rule_id": "rm_root"
  }
}
```

## Public API

The public surface of `claudine::protect` is:

- `ProtectService`
- `ProtectRequest`
- `ProtectObservation`
- `ProtectConfig`
- `ProtectRuleToggles`
- `RuleGroupConfig`
- `CustomPattern`
- `ProtectDecision`
- `ProtectMatch`
- `ProtectOutcome`
- `RuleGroup`
- `ScanSurface`
- `ProtectPlatform`
- `extract_protect_request`
- `format_blocked_message`

`ProtectRequest` currently has three variants:

```rust
pub enum ProtectRequest<'a> {
    BashCommand { command: &'a str },
    WritePath { path: &'a str, cwd: Option<&'a str> },
    McpResponse { payloads: Vec<Cow<'a, str>> },
}
```

## Module Layout

| Module | Responsibility |
|--------|----------------|
| `catalog.rs` | Rule group enum, platform enum, built-in rule definitions |
| `config.rs` | `ProtectConfig`, per-group toggles, custom pattern validation |
| `decision.rs` | `ProtectOutcome`, `ProtectMatch`, `ProtectDecision` |
| `matcher.rs` | `RegexSet` compilation and first-match lookup |
| `observe.rs` | Event-to-`ProtectRequest` extraction from runtime metadata |
| `path.rs` | Sensitive-path normalization, prefix checks, target extraction, `allow_paths` helpers |
| `report.rs` | User-facing blocked-message formatting |
| `service.rs` | `ProtectService::new()` and `ProtectService::evaluate()` |

## Blocked Output

When a rule matches, the formatted message includes the group, rule id,
pattern, matched text, and the config key that disables the group:

```text
[protect] BLOCKED
  Group: filesystem_destruction
  Rule: rm_root_glob
  Pattern: (sudo\s+)?rm\s+-rf\s+/\*
  Match: "rm -rf /var/*"

  Disable group:
    protect.rules.filesystem_destruction = false
```

## What Was Removed

- `ProtectPosture` (`Advisory`, `Balanced`, `Strict`)
- `ProtectSeverity` (`Info`, `Medium`, `High`, `Critical`)
- the 8-step evaluation pipeline
- decision-matrix scoring
- capability-aware downgrade
- `ProtectIntent` / `PolicyQuery` mapping
- YOLO softening logic
- rolling state / memory of prior protect decisions
- PolicyEngine dependency inside Protect
- MCP redaction mode
