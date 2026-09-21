# Shell Expansion

A `::shell` line runs a command while the document is being composed and replaces itself with the command's output.

```md
This repository is on:

::shell git branch --show-current
```

composes to:

```md
This repository is on:

main
```

Use it to put facts into a document that would otherwise go stale: a version, a file list, the state of a working tree. For several commands in a row, use a [shell block](./shell-blocks.md). To store a command's output in a frontmatter property instead of the body, use [frontmatter shell expansion](./fm-shell-expansion.md).

Every command must be approved before it runs. See [Pre-Flight Shell Approval](./preflight-checks.md).

## What gets inserted

- **Both output streams.** Standard output and standard error are captured together and inserted in place of the `::shell` line.
- **Nothing, if the command prints nothing.** A command that succeeds silently removes its line and leaves no gap.
- **Markdown, not a code block.** The output is spliced in as ordinary document text. Lines that follow one another become one paragraph when the document is rendered, and any Markdown in the output is live. Put a blank line above and below the directive, or the output joins the neighboring paragraph. A directive inside a fenced code block is not run; it is shown as written.
- **At the directive's indentation.** A directive inside a list item keeps the output inside that item:

```md
- Untracked files:

    ::shell git ls-files --others --exclude-standard

- Next item
```

## Using document values in the command

The command is interpolated before it runs, so it can use frontmatter and context values:

```md
---
base: main
---

::shell git log --oneline origin/{{ base }}..HEAD
```

The command that is approved is the command after interpolation. A value that changes from run to run therefore produces a command that needs approving again, unless a whitelist entry covers it by prefix.

## When a command fails

By default, a command that exits non-zero stops the composition:

```text
Error: Shell command failed (exit 1) at line 12: 'git log -1 v9.9.9'
```

That is the right default for a fact the document cannot do without. For a fact that may legitimately be absent, tell the directive what to insert instead. The options go between `::shell` and the command:

| Option | Inserts the text when… | Example |
|---|---|---|
| `--when-error <text>` | the command fails for any reason | `::shell --when-error "(not tagged yet)" git describe --tags` |
| `--when-exit-code <n> <text>` | it exits with exactly `n` | `::shell --when-exit-code 1 "(no matches)" git grep -n TODO` |
| `--except-exit-code <n> <text>` | it fails with any code *other than* `n` | `::shell --except-exit-code 2 "(unavailable)" just lint` |
| `--stderr-contains <find> <text>` | it fails and standard error contains `find` | `::shell --stderr-contains "unknown revision" "(no such ref)" git log -1 {{ ref }}` |
| `--stderr-lacks <find> <text>` | it fails and standard error does *not* contain `find` | `::shell --stderr-lacks "fatal" "(failed, but not fatally)" just check` |

Two more options leave the failure in place and improve its message, which helps whoever has to fix it:

| Option | Adds the text to the error… | Example |
|---|---|---|
| `--enrich-error <text>` | always | `::shell --enrich-error "Is the Rust toolchain installed?" cargo --version` |
| `--enrich-error-on <n> <text>` | only for exit code `n` | `::shell --enrich-error-on 2 "Exit 2 from rg means a bad pattern or an unreadable file." rg -c TODO src` |

```text
Error: Shell command failed (exit 1) at line 1: 'cargo --version'
Is the Rust toolchain installed?
```

**Put the options first.** Darkmatter recognizes them anywhere on the line, including after the command. `::shell false --when-error "x"` behaves the same as `::shell --when-error "x" false`, so an option placed last is taken by Darkmatter and never reaches the command.

## Combining commands

`&&` and `||` work as they do in a shell. The output of every command that ran is inserted, in order:

```md
::shell git describe --tags 2>/dev/null || echo "(not tagged yet)"
```

Darkmatter parses the line and starts each program itself; it does not hand the line to `sh` or `cmd`. The line therefore behaves the same on every operating system, and only a deliberate subset of shell syntax is accepted:

| Allowed | Meaning |
|---|---|
| `a && b` | run `b` only if `a` succeeded |
| `a \|\| b` | run `b` only if `a` failed |
| `2>&1` | merge standard error into standard output, in the order the program wrote them |
| `2>/dev/null`, `>/dev/null` | discard a stream |

| Rejected when the document is parsed | Message |
|---|---|
| a pipe, `a \| b` | `Shell pipes are not allowed` |
| a sequence, `a ; b` | `Command chaining (;) is not allowed` |
| a redirect to a file, `a > out.txt` | `Output redirection to arbitrary files is not allowed` |

To filter one command's output with another, do the filtering in the program itself (`git log --grep`, `rg` reading a file) or move the work into a script and run the script.

## Timeouts

A command has 10 seconds. One that runs longer stops the composition:

```text
Error: Shell command timed out after 10s at line 4: 'just ci-local --plan'
```

- `md compose --timeout <seconds>` changes the limit for every command in the run. In the library, use `ComposeOptions::with_shell_timeout()`.
- `md compose --allow-shell-timeout` turns a timeout into an empty insertion and a warning instead of an error. In the library, use `ComposeOptions::with_allow_shell_timeout(true)`.
- A single `::shell` line cannot set its own limit. A [shell block](./shell-blocks.md) can (`timeout=60`), and so can a [frontmatter expression](./fm-shell-expansion.md) (`::timeout:60`).

## Caching

A command that appears more than once runs **once per composition**, and every occurrence gets the same output. That holds across transcluded files. It keeps a document consistent with itself (`git rev-parse HEAD` cannot change halfway down the page) and avoids repeated work.

It is the wrong behavior for a command whose output is supposed to differ each time. Opt out with `--no-cache`, which neither reads nor writes the cache:

```md
::shell uuidgen                 # the same value at every occurrence
::shell --no-cache uuidgen      # a fresh value at each occurrence
```

When a cached command is one that is known to vary (`uuidgen`, `date`, `openssl`), composition prints a one-time warning suggesting `--no-cache`.

The opt-out is spelled to suit each directive: `--no-cache` here, `::no-cache` in a frontmatter expression, and `no_cache=true` on a shell block.

## Approval

A command runs only if it is approved: whitelisted, or approved interactively when the composition starts. A few commands can never be approved. Both checks happen before anything runs, and both cover every `::shell` line in the document and in every file it transcludes, including lines inside a `::block` whose condition is false.

```text
Error: Blocked command at line 3: 'rm -rf build'
Reason: 'rm' is a dangerous command
```

```text
Error: Approval required for 'just ci-local --plan'.
To allow in non-interactive mode, add one of these to <repo>/.darkmatter-shell-whitelist:
  exact just ci-local --plan
```

An approved program that is not installed is reported separately, so the two are easy to tell apart:

```text
Error: Command not found: 'ripgrep' (line 7)
Ensure 'ripgrep' is installed and available on your PATH.
```

A misspelled program name usually shows up as the *approval* message instead, because the misspelling has never been approved. The full policy, the blacklist, and the whitelist file format are in [Pre-Flight Shell Approval](./preflight-checks.md).

---

> Return to [Darkmatter Pipeline](../darkmatter-compose-pipeline.md)
