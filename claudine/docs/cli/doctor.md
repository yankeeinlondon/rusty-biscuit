---
kind: canonical-documentation
blast_radius:
  - claudine/cli/src/commands/doctor.rs
  - claudine/lib/src/doctor/
  - claudine/lib/src/composition/registration/repair.rs
---
# `claudine doctor`

`claudine doctor` checks that Claudine is set up correctly and tells you how
to fix each problem it finds. One run checks your configuration files, the
schema files Claudine registers with Darkmatter, how Darkmatter gets its
permissions, speech, and whether `claudine` can be found on your `PATH`.

```text
claudine doctor [--fix] [--json]
```

| Option | What it does |
| --- | --- |
| *(none)* | Inspect and print the report. Nothing on disk changes. |
| `--fix` | Inspect, run the safe automatic repairs, then check the repaired rows again. |
| `--json` | Print the same report as one JSON array. |
| `--plain` | (global) Print the human report without color. |

Doctor works even when your configuration is missing or broken, which is
when you need it most. It never asks a question, so a script or a CI step
can run it.

## Quick examples

```sh
# A healthy setup: every row is ok or info.
claudine doctor
# → exit status 0, last line "No problems found."

# A brand-new home that was never set up.
claudine doctor
# → exit status 1; the `config` row says "is missing" with the remedy
#   "run `claudine setup`"

# Fix what can be fixed automatically, then show the result.
claudine doctor --fix
# → exit status 0 when every repair worked and no other error is left

# Use the report from a script.
claudine doctor --json | jq -r '.[] | select(.severity == "error") | .id'
# → the ids of the rows that are errors, one per line
```

## What an inspection never does

A run without `--fix`:

- writes, creates, converts, or deletes no file. It does not create a
  default configuration, convert a legacy `config.json`, or refresh the
  published registration;
- starts no setup interview, even when `~/.claudine/config.yaml` is missing;
- makes no network request, synthesizes no speech, and plays no sound;
- never runs `claudine`: the `path` row only looks the name up.

On native Windows the `path` row starts each installed shell once (see
[`path`](#path-executable-path)). Doctor writes nothing itself, but
PowerShell refreshes its own startup cache under `%LOCALAPPDATA%` whenever
it starts, as it does for any other program that runs it.

The report never shows a configuration value, the contents of a
`permissions` block, a credential, or file contents. A problem is named by
its file, its location, and what kind of problem it is. Any other
`claudine` command prints the full message for a configuration error.

## Exit status

| Status | Meaning |
| --- | --- |
| `0` | The final report has no `error` row. Warnings and info rows do not fail the run. |
| `1` | At least one row is an `error`, including a repair that failed. A failure to write the report to stdout is also `1`. |
| `2` | A usage error, such as an unknown option. Nothing is checked. |

With `--fix`, the status reflects the state **after** the repairs: a home
whose errors were all repaired exits `0`.

## The report

Doctor always prints one row per check, in this order, even when a check
could not run:

| Id | Title | What it reports | Automatic repair |
| --- | --- | --- | --- |
| `config` | Configuration files | Each configuration file is missing, unreadable, invalid YAML, or breaks its schema or Claudine's rules. | None |
| `registration` | Darkmatter host registration | Whether Claudine's schema files are published and named in `~/.darkmatter.yaml`. | Yes |
| `darkmatter-permissions` | Darkmatter permissions handoff | Where Darkmatter takes its permissions from, at each scope. | Yes |
| `repo-config` | Repository configuration | Whether the repository has its own `.claudine/config.yaml`. | None |
| `stray-config-keys` | Unknown configuration keys | Every key Claudine does not know, with its location. | None |
| `tts` | Text-to-speech | Whether a speech provider is available, and its quality. | None |
| `path` | Executable path | Whether `claudine` is found on the executable path. | None |

Each row has a **severity**, least to most serious:

| Severity | Meaning |
| --- | --- |
| `ok` | The check ran and found nothing wrong. |
| `info` | An observation, or a check that could not run; the row says what blocked it. |
| `warn` | A problem that does not stop Claudine. |
| `error` | A problem that does. |

A row's severity is the worst of its findings. When a check is blocked by
another problem (for example, the permissions check cannot read a file the
`config` row reports as invalid), the row is `info` and says so, and the
other checks still run.

### The human report

The default output is a bulleted list. Each row shows its severity, title,
id, and a one-line summary; the lines below it are the row's findings and
remedies. The last line counts errors and warnings.

```text
• [ok] Configuration files (config): every configuration file is valid
    - /home/me/.claudine/config.yaml: valid
    - /home/me/.darkmatter.yaml: valid
• [warn] Darkmatter host registration (registration): is left over from the retired format 1 registration
    - /home/me/.claudine/darkmatter-host.yaml: is left over from the retired format 1 registration
    - the repair removes it once the `register` entry names the published manifest
    - Remedy: run `claudine doctor --fix`
…
• [ok] Executable path (path): this process finds `claudine` at /home/me/.cargo/bin/claudine

0 errors, 1 warning.
```

The report goes to stdout; anything about running the command itself goes
to stderr. Color is used only when stdout is a terminal: redirected output
and `--plain` are plain text.

### The JSON report

`--json` prints one array with one object per row, in the order above.
Every object has exactly these fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | string | The check's stable id. |
| `severity` | string | `ok`, `info`, `warn`, or `error`. |
| `summary` | string | One line. |
| `detail` | string | The findings and remedies, one per line; empty when there is nothing to add. |
| `fixable` | boolean | `true` when `--fix` has a repair for a finding in this row. |

```json
[
  {
    "id": "config",
    "severity": "ok",
    "summary": "every configuration file is valid",
    "detail": "/home/me/.claudine/config.yaml: valid\n/home/me/.darkmatter.yaml: valid",
    "fixable": false
  },
  {
    "id": "registration",
    "severity": "warn",
    "summary": "is left over from the retired format 1 registration",
    "detail": "/home/me/.claudine/darkmatter-host.yaml: is left over from the retired format 1 registration\nthe repair removes it once the `register` entry names the published manifest\nRemedy: run `claudine doctor --fix`",
    "fixable": true
  }
]
```

(The example shows two of the seven rows.) Rely on `id`, `severity`, and
`fixable`; the wording of `summary` and `detail` is for people and can
change.

## The checks

### `config`: configuration files

Doctor checks these files, each against the schema of its scope, with the
same defaults and rules a normal command uses:

| File | Required |
| --- | --- |
| `~/.claudine/config.yaml` | Yes: missing is an `error`, remedy `claudine setup`. |
| `~/.darkmatter.yaml` | Yes: missing is an `error`, remedy `claudine setup`. |
| `{repo-root}/.claudine/config.yaml` | No; checked when it exists. |
| `{repo-root}/.darkmatter.yaml` | No; checked when it exists. |

The repository files are checked only when you run doctor inside a
repository, as working-copy files. If a repository file is the same file as
your user configuration (for example, when your home is the repository), it
is checked once, as a user file. If no home directory can be found, the
`config` row says so, and the checks that do not need a home still run.

A legacy `~/.claudine/config.json` with no `config.yaml` beside it is
reported with its remedy (any ordinary `claudine` command converts it, or
run `claudine setup`). Doctor leaves it in place.

See [Configuration](../topics/configuration.md) for the files and their
settings.

### `stray-config-keys`: unknown keys

Every key Claudine's schema does not know is an `error` here, with its file,
line, and column, listed in the order they appear in the file. That
includes a user-only setting written in a repository
file, such as `tts` in `{repo-root}/.claudine/config.yaml`. The `config` row
also reports the file as invalid but leaves the keys to this row. When a
file cannot be parsed at all, this row says which file could not be
inspected.

```text
• [error] Unknown configuration keys (stray-config-keys): unknown key `tts` at the top level (line 1, column 1)
    - /home/me/.claudine/config.yaml: no unknown keys
    - /work/repo/.claudine/config.yaml: unknown key `tts` at the top level (line 1, column 1)
    - Remedy: it is a user setting; move it to `~/.claudine/config.yaml` or remove it
```

### `repo-config`: repository configuration

Inside a repository, reports whether `.claudine/config.yaml` exists there.
An absent file, and a run outside a repository, are `info`. A repository
file that exists is `ok` here; its problems are reported under `config`.

### `registration`: Darkmatter host registration

Darkmatter learns about Claudine's document types from schema files that
Claudine publishes under `~/.claudine/registrations/darkmatter/` and names
in the `register` list of `~/.darkmatter.yaml`. See
[Schemas → the published host registration](../topics/schemas.md#the-published-host-registration)
and Darkmatter's [host registration](../../../darkmatter/docs/topics/host-registration.md).

Doctor compares the manifest and every published file, byte for byte,
with what this `claudine` binary would publish:

| Finding | Severity |
| --- | --- |
| The registration folder, the manifest, or a published file is missing | `error` |
| A published file differs from what this binary publishes (Darkmatter refuses to load the registration until it is republished) | `warn` |
| The manifest differs, but Darkmatter still accepts it as Claudine's registration | `warn` |
| The manifest differs and Darkmatter refuses it as Claudine's registration | `error` |
| An extra file in the registration folder | `warn` |
| No `register` entry in `~/.darkmatter.yaml` names Claudine's manifest | `error` |
| `~/.claudine/darkmatter-host.yaml` or `~/.claudine/darkmatter-host/` (the retired format 1 registration) still exists | `warn` |
| A path doctor would repair is a symbolic link or junction | `error`, repaired by hand |

Any spelling of the entry that names the same file passes. Entries for
other hosts are never changed.

### `darkmatter-permissions`: the permissions handoff

Claudine keeps Darkmatter's `permissions` in its own configuration, and
Darkmatter's file points at it:

```yaml
# ~/.darkmatter.yaml
permissions: ~/.claudine/config.yaml
```

Doctor checks the user scope, and inside a repository also the repository
scope when both of its files exist. A scope without one of its files is
skipped, as `info`.

| What Darkmatter's file has | Severity | Repair |
| --- | --- | --- |
| A reference to Claudine's file (any spelling that resolves to it) | `ok` | — |
| A reference to another valid file | `info` | — |
| A broken or invalid reference | `error` | By hand |
| No `permissions`, or an inline block Claudine can take over | `error` | `--fix` |
| An inline block that differs from Claudine's own `permissions` | `error` | By hand: choose which to keep |

A repository's scope describes the files on disk. The grants a command
actually gets in a repository still combine Darkmatter's user, committed,
and working-copy policy; see Darkmatter's
[permissions](../../../darkmatter/docs/topics/permissions.md).

### `tts`: text-to-speech

Reads the `tts` setting of your user configuration. When the configuration
cannot be read or is invalid, the row is `info`: "speech settings could not
be determined".

- With automatic selection, the row names the best-rated available
  provider and, when it differs, the provider Claudine tries first.
- With `tts.provider` set to a provider Claudine knows, only that provider
  is checked, because speech never falls back to another one.
- An unknown `tts.provider` is at least a `warn`, and the automatic
  candidates are checked, as speech does at run time.

| Hook speech | No provider | Best quality below Good | Good or better |
| --- | --- | --- | --- |
| On (`tts` is `true` or an object) | `error` | `warn` | `ok` |
| Off (`tts` absent, `null`, or `false`) | `info` | `info` | `ok` |

With hook speech off, lifecycle `say` can still speak with automatic
selection. Detection checks local prerequisites only, such as an
executable or a credential being present. A passing row does not prove a
working voice, network access, or audio playback.

### `path`: executable path

Looks `claudine` up on the `PATH` of the process doctor runs in, without
running it. A miss is an `error` on every operating system. On WSL2, the
Linux environment is checked.

On native Windows, doctor also asks each of these shells, **when it is
installed**, whether it finds `claudine`, because each can build a
different `PATH` when it starts:

- PowerShell (`pwsh`, otherwise Windows PowerShell),
- Git Bash or MSYS2,
- Cygwin.

Each probe runs without the shell's startup files
(`-NoProfile`, `--norc --noprofile`), with no window, and is stopped after
2 seconds. A shell that is not installed is not mentioned. For each shell
that is, the row says which of these happened:

| Outcome | Severity |
| --- | --- |
| Found `claudine` | `ok` (listed in the detail) |
| Ran and did not find it | `warn` |
| Timed out, could not start, or failed | `warn` |

A shell's result is never an `error`, so a shell you never open cannot fail
doctor or a CI step. A miss reads, for example: "a fresh Git Bash without
your startup files cannot find `claudine`". No shell is probed on macOS,
Linux, or WSL2. How Sniff finds and probes these shells is described in
[Windows shells](../../../sniff/docs/topics/windows-shells.md).

## Repairs (`--fix`)

`--fix` changes nothing a plain inspection would not report as fixable.
It runs in this order:

1. inspect every row;
2. for each row with a repair whose inspection, read again from disk, is
   still fixable, run the repair: first `registration`, then
   `darkmatter-permissions`;
3. check each repaired row again, together with the `config` row it
   depends on, from disk.

The repaired rows' `detail` lists every change (`Repaired: …`). A repair
that fails makes its row an `error`, with the reason and every change made
before it stopped (`Already changed: …`). A failed repair does not stop the
other repair, and findings in other rows do not block a repair.

Running `--fix` again after it succeeded changes nothing.

### Registration repair

The repair needs a valid existing `~/.darkmatter.yaml`. When the file is
missing or invalid, the row says so and suggests `claudine setup` or fixing
the file first.

```mermaid
flowchart LR
    A[Check for links] --> B[Publish the registration]
    B --> C[Remove extra files]
    C --> D[Verify the published files]
    D --> E[Add the register entry]
    E --> F[Verify the entry]
    F --> G[Remove the format 1 files]
```

- Publishing uses the same publisher as `claudine setup`. It may overwrite
  changed files and delete extra files inside
  `~/.claudine/registrations/darkmatter/`, and nowhere else.
- The `register` entry is written with Darkmatter's locked writer, keeping
  your comments, other settings, and other hosts' entries.
- The retired `~/.claudine/darkmatter-host.yaml` and
  `~/.claudine/darkmatter-host/` are removed only after the entry is in
  place. If anything before that fails, they stay, so a retry or an older
  Claudine still works.
- No step follows a symbolic link or junction. A link in the way is
  reported, and you remove it yourself.
- Another `claudine setup` or `doctor --fix` running at the same time
  waits for this one. If another writer holds `~/.darkmatter.yaml` for more
  than 2 seconds, the repair fails, keeps the published files and the old
  entry, and can be retried.

### Permissions repair

When Darkmatter's file has no `permissions`, or an inline block Claudine
can take over, the repair does what `claudine setup` does:

1. writes the block into Claudine's file (or keeps the equal block already
   there, for example after an earlier attempt stopped halfway);
2. then replaces Darkmatter's block with a reference to Claudine's file.

Both edited files are validated before either is replaced, and comments
and other settings are kept. If the second write fails, Darkmatter's
original block stays in force. The repair edits only existing, valid files:
it never runs setup, converts a legacy configuration, or writes through a
symbolic link. A conflicting block is never merged; the row tells you to
choose one. See
[Configuration → Darkmatter's permissions](../topics/configuration.md#darkmatters-permissions).

## See also

- [`claudine setup`](./setup.md): sets Claudine up with Darkmatter; doctor's
  remedy for a missing configuration.
- [Configuration](../topics/configuration.md)
- [Schemas](../topics/schemas.md)
