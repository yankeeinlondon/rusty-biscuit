---
blast_radius:
  - darkmatter/features/2026-04-08-shell-expansion-in-fm/spec.md
  - darkmatter/features/2026-04-08-shell-expansion-in-fm/tech-design.md
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/types.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/cli/src/commands.rs
---

# Frontmatter Shell Expansion

Frontmatter Shell Expansion allows shell commands to be executed during the compose pipeline and their stdout output stored as frontmatter property values.

## Syntax

A top-level frontmatter property whose entire string value matches one of these patterns is treated as a shell expression:

```text
$(<command and args>)
$(<command and args>)::timeout:<seconds>
```

Examples:

```yaml
---
files: "$(sniff repo dirty-files)"
cwd: "$(pwd)::timeout:1"
---
```

## Rules

- The **entire** frontmatter value must be the shell expression -- embedded expressions like `"prefix $(cmd) suffix"` are not supported.
- Only top-level string-valued frontmatter properties are scanned. Nested objects and array elements are ignored.
- The optional `::timeout:<N>` suffix overrides the global shell timeout for that specific command. `N` must be a positive integer of seconds.
- Once a value matches the `$(` shape, malformed syntax is a hard compose error. Invalid timeout suffixes, tokenizer failures, and rejected executable interpolation are not silently ignored.
- Closing `)` characters inside quoted arguments are supported, so values like `$(printf ')')` parse correctly.

## Pipeline Placement

Frontmatter Shell Expansion runs in the **Inline Pre** phase, after Frontmatter Interpolation and before EffectiveState construction:

1. Merge external/inherited state
2. Apply `--set` overrides
3. **Frontmatter Interpolation** -- resolve `{{ }}` expressions
4. **Frontmatter Shell Expansion** -- execute `$(cmd)` expressions
5. Build EffectiveState
6. Body operations continue...

Because interpolation runs first, shell commands can use interpolated values as arguments:

```yaml
---
file: README.md
dir: "$(dirname {{file}})"
---
```

After interpolation, the shell stage sees `$(dirname README.md)`.

## Security

### Executable Token Rule

The executable (first token) of a frontmatter shell command must **not** come from interpolation. Only arguments may be interpolated.

Rejected:

```yaml
cmd: ls
bad: "$({{cmd}} -la)"        # executable from interpolation
```

Accepted:

```yaml
file: README.md
dir: "$(dirname {{file}})"   # only argument is interpolated
```

### Approval

Frontmatter shell commands participate in the same approval flow as body `::shell` directives. They are included in preflight discovery and subject to whitelist, blacklist, and interactive approval.

Discovery and runtime execution use the same pre-compose frontmatter preparation path:

- external state is merged first
- `--set` overrides are applied next
- frontmatter interpolation runs before scanning/execution

This keeps approval preflight aligned with the commands that real compose will execute.

## Error Handling

Frontmatter shell expansion has **no error-recovery options**. Any non-zero exit code, missing executable, blacklisted command, denied approval, or malformed shell expression results in an immediate compose error. This is intentionally simpler than body `::shell` directives.

Timeout failures follow the timeout behavior configured via `--allow-shell-timeout` (CLI) or `ComposeOptions::with_allow_shell_timeout()` (library).

## Output Normalization

- Only `stdout` is written back into frontmatter. Successful `stderr` output is ignored for value storage.
- The `stdout` from a frontmatter shell command is trimmed of all surrounding whitespace (`.trim()`) before being stored as the frontmatter value.

## Concurrency

When multiple top-level frontmatter properties contain shell expressions, they execute concurrently after approvals and policy checks have been resolved. Results are written back in deterministic top-level frontmatter iteration order.

## Timeouts

- Default global timeout: 10 seconds
- Override globally: `--timeout <seconds>` (CLI) or `ComposeOptions::with_shell_timeout()` (library)
- Override per-command: `$(cmd)::timeout:<seconds>`
- Timeout outcome:
  - Default: compose error
  - With `--allow-shell-timeout`: empty string replacement + warning

## Compose Reporting

The compose report tracks frontmatter shell expansion separately from body shell expansion.

- `ComposeOperation` variant: `FrontmatterShellExpansion`
- Phase: `InlinePre`
- Report field: `frontmatter_shell_expansions_applied`
- Perf metric name: `frontmatter shell expansion`

## Drift Detection

This document may need review when any of these files change:

- `darkmatter/features/2026-04-08-shell-expansion-in-fm/spec.md`
- `darkmatter/features/2026-04-08-shell-expansion-in-fm/tech-design.md`
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
- `darkmatter/lib/src/markdown/compose/types.rs`
- `darkmatter/lib/src/markdown/compose/mod.rs`
- `darkmatter/cli/src/commands.rs`
