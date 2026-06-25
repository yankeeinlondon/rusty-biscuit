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
$(<command and args>)::no-cache
```

Examples:

```yaml
---
files: "$(sniff repo dirty-files)"
cwd: "$(pwd)::timeout:1"
build_id: "$(uuidgen)::no-cache"
---
```

## Rules

- The **entire** frontmatter value must be the shell expression -- embedded expressions like `"prefix $(cmd) suffix"` are not supported.
- Only top-level string-valued frontmatter properties are scanned. Nested objects and array elements are ignored.
- The optional `::timeout:<N>` suffix overrides the global shell timeout for that specific command. `N` must be a positive integer of seconds.
- The optional `::no-cache` suffix bypasses the per-compose command cache so the command executes fresh at each occurrence. It combines with `::timeout:<N>` in either order (e.g. `$(uuidgen)::no-cache::timeout:5`).
- Once a value matches the `$(` shape, malformed syntax is a hard compose error. Invalid timeout suffixes, an unrecognized suffix, tokenizer failures, and rejected executable interpolation are not silently ignored.
- Closing `)` characters inside quoted arguments are supported, so values like `$(printf ')')` parse correctly.

## Token Resolution

Inside a `$( … )`, the engine and the shell coexist. A token in **executed
position** (a non-ternary directive body, or a ternary branch) resolves by a
precedence ladder — quoted/numeric/boolean literal → `name(...)` safe expression
function → path-bearing executable → bare name on `PATH` (executable) → bare name
frontmatter property → `null`. `true`/`false` are always booleans, path-bearing
tokens are always executables, and `doc.<name>` forces a frontmatter-property
reading even when a same-named executable exists.

A `$()` must resolve to at least one real shell command in executed position
(for a ternary, at least one branch; the condition never counts). A `$()` that
is entirely expression content — e.g. `"$( file_exists('x') ? 'a' : 'b' )"` —
is rejected with a diagnostic suggesting `{{ … }}` instead. Mixed forms such as
`"$( file_exists('Cargo.toml') ? cargo build : make )"` are fully supported.

See [Token Resolution in `$()` Shell Expressions](../topics/darkmatter-expressions.md#token-resolution-in--shell-expressions)
for the full ladder, the validity rule, and preflight behavior.

## Remote URLs

The `$()` shell ternary condition/branch shares the same local-filesystem-only
resolution context as frontmatter interpolation. A remote URL argument passed
to a read-side function there fails loudly rather than being fetched. Use body
interpolation for remote reads.

## Pipeline Placement

Frontmatter Shell Expansion runs in the **Inline Pre** phase, bracketed by the
two frontmatter interpolation passes and before EffectiveState construction:

1. Merge external/inherited state
2. Apply `--set` overrides
3. **Frontmatter Interpolation (pass 1)** -- resolve `{{ }}` expressions; defer keys that reference a whole-value `$(...)`
4. **Schema Validation** -- validate/coerce frontmatter against `$schema` (values still holding `$(...)` are deferred)
5. **Frontmatter Shell Expansion** -- execute `$(cmd)` expressions
6. **Frontmatter Interpolation (pass 2)** -- resolve the keys deferred in pass 1 against the now-concrete shell-expanded values
7. Build EffectiveState
8. Body operations continue...

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

Frontmatter shell commands participate in the same approval flow as body `::shell` directives. They are included in preflight discovery and subject to whitelist, blacklist, and interactive approval. See [Pre-Flight Shell Approval](../topics/pre-flight-checks.md) for the full policy details.

Discovery and runtime execution use the same pre-compose frontmatter preparation path:

- external state is merged first
- `--set` overrides are applied next
- frontmatter interpolation runs before scanning/execution

This keeps approval preflight aligned with the commands that real compose will execute.

## Error Handling

Frontmatter shell expansion has **no error-recovery options**. Any non-zero exit code, missing executable, blacklisted command, denied approval, or malformed shell expression results in an immediate compose error. This is intentionally simpler than body `::shell` directives.

Timeout failures follow the timeout behavior configured via `--allow-shell-timeout` (CLI) or `ComposeOptions::with_allow_shell_timeout()` (library).

### Post-Expansion Leak Guard

When frontmatter shell expansion is enabled, a final pass over every top-level
string value rejects any value that *still* trims to a whole-value `$(...)`
candidate after expansion has run. This closes the residual leaks the
strict-start scan cannot catch: command output that reproduces `$( … )` (e.g.
`$(echo '$(date)')`), and a whole-value `$(...)` hidden behind leading
whitespace that the start-of-value scan skips. A surviving whole-value
candidate is a hard compose error tagged with the offending frontmatter key and
its source line.

The guard runs only when shell expansion is enabled. When frontmatter shell
expansion is **explicitly disabled**, `$(...)` values are deferred unchanged and
the guard never runs. Mixed and trailing forms (`literal $(echo ok)`,
`$(echo ok) trailing`) are outside the whole-value rule and pass the guard
untouched.

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
