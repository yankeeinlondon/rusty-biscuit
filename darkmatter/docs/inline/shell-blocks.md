# Shell Blocks

Shell blocks allow multiple shell commands to be executed sequentially and their
combined output rendered into the document.

## Syntax

```md
::shell-block [params]
command1
command2
::end-block
```

Unlike `::shell` directives which execute a single command, shell blocks group
multiple commands together. Each logical line in the body is treated as a
separate command. Commands are prepared (approval, policy checks) before any
execute, then run sequentially.

## Parameters

Shell blocks use **key-value** parameter syntax (not flag syntax like `::shell`):

| Parameter | Example | Description |
|-----------|---------|-------------|
| `when_error` | `when_error="fallback text"` | Replace any failing command with this text |
| `when_exit_code` | `when_exit_code="1,fallback"` | Replace exit code 1 with fallback text |
| `except_exit_code` | `except_exit_code="1,fallback"` | Replace all errors *except* code 1 |
| `stderr_contains` | `stderr_contains="not found,ok"` | Replace if stderr contains "not found" |
| `stderr_lacks` | `stderr_lacks="error,fallback"` | Replace if stderr lacks "error" |
| `enrich_error` | `enrich_error="extra context"` | Add context to unhandled error messages |
| `enrich_error_on` | `enrich_error_on="1,context"` | Add context only for exit code 1 |
| `timeout` | `timeout=5` | Override default timeout (seconds) for all commands in block |

> **Note:** Do not use `--param` or `::param:value` syntax. Shell blocks require
> `param="value"` syntax. Using the wrong style will produce a targeted error
> hint.

## Command Body Rules

- Each non-empty line is a logical command
- Blank lines are ignored
- A trailing backslash `\` joins the current line with the next non-blank line
  (one space separator)
- Escaped backslash `\\` produces a literal backslash, not a continuation

## Output Rendering

Each command's combined output (stdout + stderr) is concatenated **verbatim**.
The only transformation a shell block applies to captured output is the
container indentation re-applied at the splice boundary, so output stays nested
under a surrounding list item or block quote. Nothing is trimmed, dropped, or
otherwise normalized:

- Leading and trailing whitespace, line endings, embedded code fences, and
  Unicode content are preserved byte-for-byte.
- Separation between commands comes from each command's own trailing newline,
  not an inserted blank line. A command that emits no trailing newline has the
  next command's output appended directly.
- If every command produces empty output, the block renders as an empty string.

## Example

```md
::shell-block
echo "First command"
echo "Second command"
::end-block
```

Renders as (each `echo` supplies its own trailing newline):

```
First command
Second command
```

## Error Handling

### Per-command fallback

The `when_error` parameter applies to each command individually. If a command
fails, it is replaced with the fallback text and subsequent commands continue:

```md
::shell-block when_error="(failed)"
echo "before"
false
echo "after"
::end-block
```

Renders as:

```
before

(failed)

after
```

### Unhandled failures

If a command fails with no matching error handler:

- Output from already-succeeded commands is preserved in the error
- The error includes the failing command's source excerpt
- The block is not partially rendered into the document

In terminal error output, preserved partial output is visually demoted with
Prose `<dim>` styling. If shell-block error output gains a dedicated HTML
renderer, use a matching CSS treatment for that demoted partial output.

## Security

Shell blocks share the same security infrastructure as `::shell` directives:

- [Pre-Flight Shell Approval](../topics/pre-flight-checks.md) — blacklist, whitelist, and approval flow
- [Shell Expansion](./shell-expansion.md) — `::shell` directive details

Commands inside a shell block are discovered during pre-flight and presented for
approval along with `::shell` directives.

## Nesting

Shell blocks can appear inside page blocks (`::block` / `::end-block`). If the
parent page block evaluates to false, the shell block is removed without
executing any commands.

```md
::block when="show_details"
::shell-block
echo "detailed info"
::end-block
::end-block
```

## Comparison with `::shell`

| Feature | `::shell` | `::shell-block` |
|---------|-----------|----------------|
| Commands | Single | Multiple |
| Parameters | Flag syntax (`--param`) | Key-value (`param="value"`) |
| Output | Combined stdout + stderr | Per-command combined output, concatenated verbatim |
| Error scope | Per-command | Per-command within block |
| Empty output | Removed entirely | Contributes nothing (no placeholder) |

---

> Return to [Darkmatter Pipeline](../darkmatter-compose-pipeline.md)
