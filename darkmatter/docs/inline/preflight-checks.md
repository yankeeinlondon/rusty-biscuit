# Pre-Flight Shell Approval

Before the compose pipeline executes any shell command, every command must pass through a security approval flow. This pre-flight check ensures that no malicious or accidentally harmful command runs without explicit authorization.

## Why This Exists

Shell commands can appear in three places during Darkmatter composition:

1. **`::shell` directives** — inline shell expansion in the document body
2. **`::shell-block` / `::end-block` directives** — multi-command sequential execution
3. **Frontmatter `$(cmd)` expressions** — shell expansion in top-level frontmatter string values

Without pre-flight approval, an unguarded shell directive could execute destructive or unintended commands during what should be a safe document composition.

## Approval vs Execution

Pre-flight separates two concerns that used to be conflated:

- **The approval set is condition-blind.** Collection walks frontmatter, body
  `::shell`/`::shell-block`, and the transclusion graph **without** evaluating
  any `when=` or page-block condition. Both sides of every `$(...)` ternary,
  every `::block` (even `when=`-false ones), and every conditionally-transcluded
  document contribute their commands. The set is deduped by normalized command
  string and approved **once, up front**, in a single batched prompt.
- **The execution set is condition-aware.** Each shell stage runs only the
  commands whose branch is actually reached given the document's final state
  (after page blocks have pruned dead regions and frontmatter is resolved).

The governing invariant is:

```
execution_set ⊆ approval_set      (always)
```

Because approval is a superset of anything reachable under any state, the
execution-time gate degrades to a pure membership check: the command is already
approved, so it runs with no prompt — this run or any later loop iteration with
flipped conditions. A dead-branch command is **approved but never executed**.

A miss surfaces as `NotPreApproved`, which after collection is purely a bug
sentinel. A user-authored command whose shape depends on a frontmatter value
still pending frontmatter-shell expansion (the chicken-and-egg case) is rejected
up front as `DynamicCommandShape` rather than surfacing as a late
`NotPreApproved`.

## Security Policy

Darkmatter uses a two-stage security design:

1. **Global blacklist** — a built-in set of commands that are never allowed, regardless of user configuration
2. **User-controlled whitelist/blacklist files** — per-repo or per-user allow/deny lists

Every shell command discovered during composition is checked against these layers in order. The first matching layer determines the outcome:

- **Blacklisted** → immediate compose error
- **Not on host** → immediate compose error (command executable not found)
- **Whitelisted** → approved, execute normally
- **Unknown** → interactive approval prompt (or deny in non-interactive contexts)

### Blacklisted Commands and Syntax

The following commands will never be allowed:

- `rm`, `rimraf`, `find*-delete`, `unlink`, `shred`, `wipe`
- `echo* >>*`, `echo* >*`, `* >*`
- `install`
- `brew`, `apt`, `nala`, `pacman`, `dnf`, `yum`
- `npm uninstall`, `pnpm uninstall`, `bun uninstall`, `yarn uninstall`
- `npm install`, `pnpm install`, `bun install`, `yarn install`
- `npm add`, `pnpm add`, `bun add`, `yarn add`
- `mv`, `dd`
- `zfs`, `zpool`, `wipefs`, `mkfs*`, `parted`, `mparted`, `sgdisk`
- `pvcreate`, `lvremove`, `vgremove`, `mdadm`, `cryptsetup`
- `chmod`, `chgrp`, `chown`, `setfacl`
- `tar`, `unzip`, `rsync`, `cp`
- `kill`, `pkill`, `killall`
- `systemctl`
- `shutdown`, `reboot`, `poweroff`, `halt`, `init`
- `git reset`, `git clean`, `git checkout`, `git restore`, `git rebase`, `git branch`, `git push`, `git reflog`, `git gc`
- `psql -c`, `mysql -e`, `redis-cli FLUSH*`, `mongosh --eval`
- `ssh`, `scp`, `rsync`, `ansible`
- `curl`, `wget`, `http`
- `source`, `eval`, `sudo`, `doas`, `su`
- `docker rm*`, `docker system prune*`, `docker volume rm*`, `docker volume prune*`
- `kubectl delete*`, `helm uninstall*`, `terraform destroy*`

### Whitelist and Blacklist Files

Approved commands are stored in a namespaced whitelist file:

- `[repo root]/.darkmatter-shell-whitelist` if CWD is a git repo
- `${HOME}/.darkmatter-shell-whitelist` otherwise

A companion blacklist file tracks user-denied commands:

- `[repo root]/.darkmatter-shell-blacklist` if CWD is a git repo
- `${HOME}/.darkmatter-shell-blacklist` otherwise

## Interactive Approval

When a command does not match the blacklist and is not in the whitelist, the user is prompted with these options:

| Option | Behavior |
|--------|----------|
| Allow exact command | Persist the command with all parameters to `.darkmatter-shell-whitelist` |
| Allow with any parameters | Persist a wildcard signature for the executable to `.darkmatter-shell-whitelist` |
| Allow once | Execute this session only; cached but not persisted |
| Deny | Abort the pipeline with an error; nothing is persisted |
| Blacklist | Abort the pipeline and add the command to `.darkmatter-shell-blacklist` |

## Timeouts

- Default timeout: 10 seconds
- Override globally: `--timeout <seconds>` (CLI) or `ComposeOptions::with_shell_timeout()` (library)
- Override per-command: `$(cmd)::timeout:<seconds>` (frontmatter only)
- Default timeout behavior: compose error
- With `--allow-shell-timeout`: timed-out commands are replaced with an empty string and compose emits a warning instead of failing

## Integration with External Tools

When Darkmatter is used through an external orchestrator (such as Claudine), the pre-flight flow can be bypassed in favor of the orchestrator's own approval system. In this mode, Darkmatter receives a set of pre-approved commands and skips its own whitelist, blacklist, and interactive approval checks. See the [Claudine/Darkmatter boundary](#claudinedarkmatter-boundary) below for details.

### Claudine/Darkmatter Boundary

**Darkmatter's role is discovery.** It walks the document graph condition-blind — following transclusions, resolving interpolation, parsing `::shell`/`::shell-block` directives, and scanning frontmatter for `$(...)` expressions. `Markdown::compose_preflight(options)` returns a `ComposePreflightReport` whose `approval_set()` is every command that could run under any state, without checking policy files or making approval decisions. (`collect_shell_commands` remains as the lower-level entry point that returns the raw entries.)

**Claudine's role is authorization.** It takes the approval set from Darkmatter, merges in commands from its harness, checks everything against the whitelist, and prompts the user once for anything missing. The merged, authorized set is handed back to the pipeline as the execution membership source.

**During composition**, Darkmatter receives the pre-approved set via `pre_approved_commands` on `ComposeOptions` and trusts it completely, bypassing its own approval flow.

This separation ensures:

- The orchestrator never needs to understand transclusion, interpolation, or the document graph
- Darkmatter never needs to understand the orchestrator's harness, interactive prompting, or session lifecycle
- There is exactly one place where shell approval decisions are made

## See Also

- [Shell Expansion](../inline/shell-expansion.md) — `::shell` directive syntax, output handling, and error exit codes
- [Frontmatter Shell Expansion](../inline/fm-shell-expansion.md) — `$(cmd)` expressions in frontmatter
- [Shell Blocks](../inline/shell-blocks.md) — `::shell-block` / `::end-block` multi-command directives
- [Darkmatter Compose Pipeline](../darkmatter-compose-pipeline.md)
