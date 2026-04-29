# Shell Blocks

In Darkmatter we have a well established feature called [shell expansion](darkmatter/docs/inline/shell-expansion.md) which allows a markdown document to inject the STDOUT output of a shell command using the `::shell` directive.

In this feature we're going to offer a counterparty to the shell expansion feature called **Shell Blocks**:

- Shell Blocks are a Markdown _block_ construct and are defined structurally as:

    ```md
    ::shell-block <params>
    _command or commands_
    ::end-block
    ```

- The **Shell Block** is used over the single line _shell expansion_ for two primary reasons:
    1. Single Command with Breathing Room

        Sometimes the freedom to "spread out" the command over multiple lines (with `\` continuance character)
        can make the command MUCH easier to understand and being able to _understand_ is important for both the
        long term maintenance of the prompt but also so that the security approval (which all shell commands require)
        is presented in a way that the approver can approve with good understanding of what they're looking at.

    2. Multiple Commands

        The _shell expansion_ command allows for a single chained execution. This often is just a single command
        but it _can_ be a chain of commands piped into one another. What a _shell expansion_ can NOT do is run
        multiple commands one after another. In contrast **Shell Blocks** can execute 1 or more shell commands within
        the block.

- The whitelisting of shell commands in a **Shell Block** behaves exactly the same as it would in _shell expansion_.
    - the only exception is that parser responsible for identifying commands needs to be able parse out the "multiple
      commands" requirement described above.
    - each command within a Shell Block triggers individual approval, exactly like `::shell`
    - if any command is denied, the entire block is skipped (no commands execute)
    - the discovery phase enumerates all block commands into the flat discovery list; this requires zero changes to the
      existing approval infrastructure
- Parameter's for **Shell Block** are the same but the syntax is slightly nicer as we have our own line for the parameters
  to sit on instead of sharing space with the shell commands. This is covered in more detail in the next section.

## Parameters

The [shell expansion parameters](darkmatter/docs/inline/shell-expansion.md#handling-error-exit-codes) are largely designed
for the author to handle error exit codes in a more flexible way:

- `--when-error <string>` in **Shell Expansion** is `when_error="{string}"` for **Shell Blocks**
- `--when-exit-code <#> <string>` in **Shell Expansion** is `when_exit_code="{#},{string}"`
- etc.

In essence, because the `::shell-block` line is dedicated to parameters rather than sharing with shell command we are
using a more typical/idiomatic syntax here. That said, it still will be easy for authors to get mixed up. At some point
we'll have an LSP to help people use the right syntax but for now we just need to understand that there is a high
likelihood that authors will get mixed up (in either direction) and we need to make sure we have a very good error message
to both let them know what happened, where it happened, but also _hint_ at what they likely meant.

## Command Boundaries

The block body is split into individual commands using the following rules:

- Each non-empty line is treated as a separate command
- A trailing `\` on a line joins it with the next line (continuation), producing a single logical command spanning
  multiple physical lines
- Blank lines are ignored
- Each logical line is tokenized using the same tokenizer as `::shell` — no pipes (`|`), no command chaining (`&&`),
  no statement separators (`;`)

This means the following block body executes two independent commands:

```md
::shell-block
echo "hello"
echo "world"
::end-block
```

While this block body executes a single command spread across two physical lines:

```md
::shell-block
echo \
  "hello"
::end-block
```

## Handling Multiple Commands

With the possibility of multiple commands with multiple utterances of STDOUT/STDERR we need to understand what extra
rules and precautions we need to put in place:

- first off, when a **Shell Block** only emits a single STDOUT/STDERR we will behave identically to **Shell Expansion**
- a "successful" outcome is when all commands in the block execute without returning an error code:
    - when we have a successful outcome we will emit a _trimmed_ version of the combined output (STDOUT and STDERR) of each command
    - if the trimmed combined output is an empty string then it is not rendered at all into the prompt document
    - all commands whose trimmed combined output is _not_ empty will be rendered with a terminating "\n"
    - in between commands with non-empty combined output, we should render an empty row to separate them
        - remember: a command whose combined output is an empty string after trimming is fully removed from rendering
          and should not add extra empty rows
        - a command with empty STDOUT but non-empty STDERR is considered non-empty (it has output)
- error handling parameters apply **per-command**, not per-block:
    - when a command fails and a matching handler exists (e.g., `when_error`), the fallback string replaces only
      that failing command's output — execution continues with subsequent commands
    - this is a deliberate departure from single `::shell` behavior, where the fallback replaces the entire
      directive output
    - when a command fails and **no** matching handler exists, execution stops immediately; output from already-succeeded
      commands is preserved in the rendered document but visually demoted, followed by the error presentation
        - the exact visual treatment of the demoted partial output (dimmed, commented, etc.) will be determined
          during implementation
- there are no atomicity guarantees: if command 6 of 10 fails, commands 1–5 have already executed and there is no
  mechanism to roll them back — this is an accepted constraint with no intention of ever providing a transaction-based
  solution

- like anywhere in else in Darkmatter it is critical that error presentation be well thought out
    - you can't just point to a file and line number because _interpolation_ of the composed document means that
      the line number you're referencing probably doesn't exist in the actual file!
    - always _render_ a code block that shows the line where the error occurred as well as the lines before and after

## Interoperating with Page Blocks

The [Page Blocks](darkmatter/docs/inline/page-blocks.md) functionality delimits its blocks with `::block` / `::end-block`. Shell Blocks share this same `::end-block` terminator.

A unified stack-based parser at pipeline step #4 resolves this sharing without ambiguity:

- Both `::block` and `::shell-block` are recognized as opening tokens and pushed onto the same stack
- `::end-block` always pops the most recent entry, regardless of block type
- After step #4 completes, all `::block` / `::end-block` pairs are resolved and removed; only `::shell-block` / `::end-block` pairs remain for step #7

Because nesting a `::shell-block` inside a `::block` (or vice versa) follows normal stack discipline, interleaving is handled naturally by the parser rather than being prohibited by convention.

## Execution Order

The **Shell Block** is executed as the last step of the **Inline Pre** stage of the [Darkmatter Pipeline](darkmatter/docs/darkmatter-compose-pipeline.md), immediately following the **Shell Expansion** operation.
