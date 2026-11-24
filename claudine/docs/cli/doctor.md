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
# → exit status 1; the `config` row says "is missing" with a remedy that
#   starts "run `claudine setup`"

# Fix what can be fixed automatically, then show the result.
claudine doctor --fix
# → exit status 0 when every repair worked and no other error is left

# Use the report from a script.
claudine doctor --json | jq -r '.[] | select(.severity == "error") | .id'
# → the ids of the rows that are errors, one per line
```

## What an inspection never does

A run without `--fix`:

- writes, creates, converts, or deletes none of your files: not
  Claudine's or Darkmatter's configuration, not the published
  registration, and nothing in your home or user profile. It does not
  create a default configuration, convert a legacy `config.json`, or
  refresh the published registration. The one exception is on native
  Windows: each PowerShell probe of the `path` row gets a private folder
  in the system temporary directory, deleted when the probe ends (see
  below);
- starts no setup interview, even when `~/.claudine/config.yaml` is missing;
- makes no network request, synthesizes no speech, and plays no sound;
- never runs `claudine`: the `path` row only looks the name up.

On native Windows the `path` row starts each installed shell once (see
[`path`](#path-executable-path)). Those shells leave your profile alone
too. PowerShell writes a startup cache every time it starts, even without
a profile, so doctor gives each PowerShell probe a private folder in the
system temporary directory as its profile and cache location. That folder
is the only place an inspection writes, and it is deleted when the probe
ends.

The report never shows a configuration value, the contents of a
`permissions` block, a credential, or file contents. A problem is named by
its file, its location, and what kind of problem it is. Each remedy names a
command that prints the full message (see
[where the full message is printed](#where-the-full-message-is-printed)).

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
    - this process finds `claudine` at /home/me/.cargo/bin/claudine

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
| `~/.claudine/config.yaml` | Yes: missing is an `error`, remedy `claudine setup`. Setup writes the file only after it publishes the host registration, so if publication stops, correct what setup names and run it again. |
| `~/.darkmatter.yaml` | Yes: missing is an `error`, remedy `claudine setup`. |
| `{repo-root}/.claudine/config.yaml` | No; checked when it exists. A legacy `config.json` in its place is a `warn`. |
| `{repo-root}/.darkmatter.yaml` | No; checked when it exists. |

The repository files are checked only when you run doctor inside a
repository, as working-copy files. If a repository file is the same file as
your user configuration (for example, when your home is the repository), it
is checked once, as a user file. If no home directory can be found, the
`config` row says so, and the checks that do not need a home still run.

A legacy `config.json` with no `config.yaml` beside it is reported and left
in place; doctor never converts it, with or without `--fix`. Which command
converts it depends on where it is:

| Legacy file | Severity | Command that converts it |
| --- | --- | --- |
| `~/.claudine/config.json` | `error` (the user file is required) | `claudine setup` |
| `{repo-root}/.claudine/config.json` | `warn` (the repository file is optional) | `claudine claude` with a prompt, run inside the repository; it converts the file before Claude Code starts. `claudine setup`, listing commands, an interactive `claudine claude` with no prompt, and a `--dry-run` leave it as it is. |

```sh
cd my-repo   # holds .claudine/config.json and no .claudine/config.yaml
claudine doctor
# • [warn] Configuration files (config): is not present; a legacy `config.json` beside it has not been converted
#     - Remedy: `claudine claude` with a prompt, run inside the repository, converts it before the provider starts
claudine claude "hi"
# → .claudine/config.yaml is written and config.json is kept as config.json.bak
```

When the repository is your home, its legacy file is your user legacy file
and is reported once, as the user's.

#### A `permissions` reference whose file is broken

When a `.darkmatter.yaml` sets `permissions` to a file reference, `config`
follows it the way `md` does and checks the file it names. A reference
that cannot be used is an `error` in `config`, whatever state Claudine's
file for that scope is in: missing, invalid, or the same file as your user
configuration. A repository that has a `.darkmatter.yaml` and no
`.claudine/config.yaml` is checked too, even though the
`darkmatter-permissions` row skips that scope.

```yaml
# {repo-root}/.darkmatter.yaml, with no {repo-root}/policy.yaml
permissions: ./policy.yaml
# config: {repo-root}/.darkmatter.yaml: has a `permissions` reference to a file that does not exist (line 1, column 14)
```

The reference is broken when the file it names:

- does not exist;
- is not valid YAML, or is not a mapping;
- has no `permissions`, or a `permissions` that is `null`, a scalar, or
  another reference (references do not chain);
- holds a `permissions` mapping that Darkmatter's rules for the scope
  refuse;
- or, for a repository reference, lies outside the repository, including
  through a link. Darkmatter's user policy follows a link out of your home,
  so a user reference through one passes.

When the file it names is that scope's Claudine configuration, the finding
says so, beside that file's own finding:

```text
/home/me/.claudine/config.yaml: is missing
/home/me/.darkmatter.yaml: has a `permissions` reference to Claudine's configuration, which does not exist (line 1, column 14)
```

The finding names the `.darkmatter.yaml` and the line and column where it
writes `permissions`, never the reference or the file it names, since both
spell what you wrote. A reference that is itself invalid (an empty string,
a number, a boolean) is reported once, as an invalid value; nothing is
followed. Doctor repairs neither: correct the reference or the file it
names. For your user file the remedy adds that `md init --inline` replaces
the reference with a block; that command rewrites only your user file, so
a repository reference is corrected by hand. Doctor checks the working
copy only; the committed (`HEAD`) policy is not read.

#### Where the full message is printed

Doctor names a problem without quoting your values. To see the full
message, run the command its remedy names. A remedy names a command only
for a problem that command reports in that file; otherwise it says what to
correct. Not every command reads every file: `claudine --help`, shell
completion, and `claudine registrations` read no configuration, and
`claudine doctor` reads the files only to diagnose them, so it never stops
on one. `claudine setup` and listing commands such as `claudine actions`
read only your user file, and only a provider launch stops on an error in
the repository file (not every provider: `claudine goose` launches without
reading it). The remedies therefore name these commands:

| Problem in | Command that prints it |
| --- | --- |
| `~/.claudine/config.yaml` | `claudine actions` (or `claudine setup`) stops with the located error and exits 1. A mapper's regular expression is the exception: no load compiles it, so `claudine actions` lists the action and exits 0, and the remedy names the setting to correct, such as `actions.before_tool[0].call.mapper.pattern`. |
| `{repo-root}/.claudine/config.yaml` | `claudine claude` run inside the repository stops with the located error before Claude Code starts. `claudine setup` and `claudine actions` do not read this file, and neither does a `--dry-run`. With a prompt, the same launch converts a legacy `config.json` there, or stops with the reason it cannot. |
| a `.darkmatter.yaml` | `md compose` of a document that runs a shell command. Any `md compose` stops on a file that does not load; a `permissions` problem appears only when a shell command needs the policy. `md render` reports neither. An invalid `register` setting stops no compose, so its remedy says to correct `register` at the reported position. |

For example, with an unclosed list in your user file:

```sh
claudine doctor
# • [error] Configuration files (config): is not one valid YAML mapping (line 2, column 1)
#     - /home/me/.claudine/config.yaml: is not one valid YAML mapping (line 2, column 1)
#     - Remedy: correct it; `claudine actions` prints the full message
claudine actions
# invalid configuration: /home/me/.claudine/config.yaml:2:1: not valid YAML: …
```

#### Which file a rule names

Besides the schema, doctor applies the rules a normal load applies to
the settings themselves (a selected messenger route must exist, a sound
name must be known, a regular expression must compile). Inside a
repository it judges your user file with the repository's overrides
merged on top, as a launch there does, and names the file that sets the
value that broke the rule:

| Setting | The file named |
| --- | --- |
| An event's actions (a mapper's regular expression) | The repository file when it lists that event, because it replaces the event's whole action list; otherwise your user file. |
| `exit_expressions` | The repository file when it declares them, because its declaration replaces yours; otherwise your user file. |
| The selected messenger route | When the repository's `active_messenger` selects a route your user file does not define, both: the repository's `active_messenger`, and the `messenger.configurations` of your user file it selects from. Otherwise your user file. |
| `protect`, `default_sounds`, and the route definitions | Always your user file; a repository cannot set them. |

```yaml
# ~/.claudine/config.yaml: before_tool's mapper does not compile
# {repo-root}/.claudine/config.yaml: replaces before_tool, adds a broken after_tool
actions:
  before_tool: [ … valid … ]
  after_tool:  [ … broken mapper … ]
# config: {repo-root}/.claudine/config.yaml: the mapper of action 1 of `after_tool` is not a valid regular expression
```

When the repository replaces a setting your user file gets wrong, the
merged configuration is valid and `config` reports nothing for it. The
rows that read your user file alone, `tts` and the user scope of
`darkmatter-permissions`, then explain the problem themselves.

A rule broken in the merged configuration and named under the repository
file is reported by no other command (a launch falls back to the
defaults), so its remedy names none.

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

An action or messenger route still written in the earlier flat form
(`- type: bash` beside `command:`) is an invalid value in `config`, not a
stray key: its `type:` or `provider:` tag is never listed here. A misspelled
setting inside such an entry is listed here all the same, at the line and
column where the file writes it, so one run shows both edits:

```yaml
actions:
  session_start:
    - type: bash          # config: has an invalid value at `/actions/session_start/0` (line 3, column 7)
      command: ./notify.sh
      timout: 5           # stray-config-keys: unknown key `timout` in `/actions/session_start/0` (line 5, column 7)
```

The rules that judge the whole file, such as writing both
`preferred_agent` and `favorite_agent` or an exit expression with both or
neither of `pattern` and `patterns`, are reported in `config` beside a flat
entry's invalid value as well, each at its own line and column.

```text
• [error] Unknown configuration keys (stray-config-keys): unknown key `tts` at the top level (line 1, column 1)
    - /home/me/.claudine/config.yaml: no unknown keys
    - /work/repo/.claudine/config.yaml: unknown key `tts` at the top level (line 1, column 1)
    - Remedy: it is a user setting; move it to `~/.claudine/config.yaml` or remove it
```

### `repo-config`: repository configuration

Inside a repository, reports whether `.claudine/config.yaml` exists there.
An absent file, and a run outside a repository, are `info`. A legacy
`.claudine/config.json` with no `config.yaml` is also `info` here, reported
as awaiting conversion rather than absent; `config` carries its warning and
remedy. A repository file that exists is `ok` here; its problems are
reported under `config`.

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
| A published file differs from what this binary publishes | `warn` |
| The manifest differs, but Darkmatter still loads the bundle it lists as Claudine's registration | `warn` |
| The manifest differs and Darkmatter refuses the bundle it lists as Claudine's registration | `error` |
| An extra file in the registration folder | `warn` |
| No `register` entry in `~/.darkmatter.yaml` names Claudine's manifest | `error` |
| `~/.claudine/darkmatter-host.yaml` or `~/.claudine/darkmatter-host/` (the retired format 1 registration) still exists | `warn` |
| A `register` entry in `~/.darkmatter.yaml` still names the retired `~/.claudine/darkmatter-host.yaml`, in any spelling, whether or not that file still exists | `warn` |
| A path doctor would repair is a symbolic link or junction | `error`, repaired by hand |
| The `register` edit the repair needs reaches an inline list or root mapping, or an entry or list with an anchor, alias, or tag | the finding's own severity, repaired by hand (see [Files the repairs cannot edit in place](#files-the-repairs-cannot-edit-in-place)) |

A leftover entry for the retired manifest is easy to miss: deleting the old
files by hand after upgrading leaves this list behind, and Darkmatter warns
about the entry each time it loads its configuration:

```yaml
# ~/.darkmatter.yaml
register:
  - ~/.claudine/registrations/darkmatter/manifest.yaml
  - ~/.claudine/darkmatter-host.yaml   # the old manifest, already deleted
```

Doctor finds such an entry the way Darkmatter's writer does, by the file it
would open, so `~/.claudine/darkmatter-host.yaml`,
`./.claudine/darkmatter-host.yaml`, `.claudine/darkmatter-host.yaml`, and
the absolute path are all the same entry. The finding gives the entry's
position in the list and its line and column, never its text:

```text
• [warn] Darkmatter host registration (registration): `register` entry 2 names the retired format 1 manifest (line 4, column 5)
    - /home/me/.darkmatter.yaml: `register` entry 2 names the retired format 1 manifest (line 4, column 5)
    - Darkmatter warns about it whenever it loads its configuration
    - Remedy: run `claudine doctor --fix`
```

`--fix` changes only the entries that name Claudine's current manifest or
the retired one, in any spelling. The first of them is kept, rewritten to
`~/.claudine/registrations/darkmatter/manifest.yaml` if it names the retired
manifest, with the comment on its line; every later one is removed together
with the comment on its line. In the example above, the retired entry goes
with its `# the old manifest, already deleted` comment. Other hosts'
entries and every other comment, including one on its own line above a
removed entry, are kept. A retired entry in a repository's own
`.darkmatter.yaml` is not this row's concern; only the user file is checked
and edited.

A changed published file is explained by what the installed manifest says
about it:

- **The manifest is unchanged.** The file no longer matches the digest the
  manifest lists for it, and the finding says so: Darkmatter refuses a
  member whose bytes do not match their digest, so the registration is not
  loaded until it is republished.
- **The manifest was changed too**, for example after editing a schema and
  updating its digest by hand. Doctor then does not stop at the digests: it
  prepares the whole bundle the manifest lists exactly as Darkmatter does
  when it loads a `register` entry, so every schema must parse and every
  generated artifact (such as `globals.lifecycle.json`) must still agree
  with the schemas it was built from. If that succeeds, the manifest is a
  `warn` and each edited file only deviates from the shipped bytes. If
  Darkmatter would refuse the bundle, the manifest is an `error` that names
  the file Darkmatter's first refusal is in.

For example, a comment added to `schemas/action.yaml` with its digest
updated still loads (`warn`), while the same comment in
`schemas/group.yaml` does not: `group.yaml` declares one of Claudine's
globals, and the published lifecycle artifact was built from the original
file (`error`).

Doctor finds the files a changed manifest lists the way Darkmatter does:
from the manifest's folder, and never outside it. A manifest that spells
`./schemas/action.yaml` as `../darkmatter/schemas/action.yaml` (leaving the
folder and coming straight back) still names the same file, so with its
digest key spelled the same way it is a `warn`, and an edited file under
either spelling is reported as disagreeing with its digest. A spelling that
ends outside the folder, such as `../schemas/action.yaml`, is refused by
Darkmatter, so the manifest is an `error`.

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
skipped, as `info`; a broken reference there is still an `error` under
`config`.

| What Darkmatter's file has | Severity | Repair |
| --- | --- | --- |
| A reference to Claudine's file (any spelling that resolves to it) | `ok` | — |
| A reference to another valid file | `info` | — |
| A broken or invalid reference | `error`, explained under `config` | By hand |
| No `permissions` (or `null`), or an inline block Claudine can take over | `error` | `--fix` |
| The same, in a file whose `register` setting is invalid | `error` | By hand: correct `register` first |
| The same, where Claudine's file is reached through a link that leaves the scope | `error` | By hand: replace the link |
| The same, where the edit reaches an inline root mapping of either file, or a `permissions` value with an anchor (`&name`) or tag (`!tag`) | `error` | By hand: rewrite that part in block form, without the anchor or tag (see [Files the repairs cannot edit in place](#files-the-repairs-cannot-edit-in-place)) |
| An inline block that differs from Claudine's own `permissions` | `error` | By hand: choose which to keep |

`permissions: null` means the same as leaving `permissions` out: Darkmatter
uses its default policy. The repair writes only the reference, and gives
Claudine's file the default block when it has none; `null` is never copied.

Darkmatter's writer checks the whole edited file, so a `register` setting
that is not a list of manifest references (`register: 3`, or a list with a
non-text entry) would make every attempt fail. Doctor therefore offers no
repair for that scope until `register` is corrected; the `config` row names
the position. An omitted, `null`, or empty `register` does not block it. A
scope whose `permissions` already refers to Claudine's file needs no edit,
so a `register` problem does not change its `ok`.

The reference the repair writes must name Claudine's file inside its scope:
your home for the user scope, the repository for the repository scope. When
`config.yaml`, or the `.claudine` folder holding it, is a link into a
dotfiles checkout elsewhere, the repair cannot write that reference, even
when Claudine's file already holds the block and only Darkmatter's file
would change. The row then names where the file resolves to:

```text
/home/me/.claudine/config.yaml resolves to /srv/dotfiles/claudine/config.yaml, outside your home; the repair refers only to a file inside its scope
```

Replace the link with the file it names and run `--fix`. For the user scope
you can instead write the reference yourself, since Darkmatter's user policy
follows it; Darkmatter refuses a repository reference that leaves the
repository. A link whose target stays inside the scope does not block a
repair that writes only the reference.

The remedy is `--fix` in both scopes. `claudine setup` performs the same
user handoff, but only after it publishes Claudine's host registration, so a
link anywhere on the publisher's path (the `~/.claudine` or
`~/.claudine/registrations` folder, the publisher's lock file, the
registration folder, its manifest, or a member it would replace or retire)
stops setup before the handoff, even when the `registration` row is `ok`.
The repair here does not depend on publication: in that home `--fix` still
writes the reference. A link on a path the registration itself uses (either
folder, the registration folder, its manifest, or a member it would replace
or retire) stays an `error` in the `registration` row afterward. A linked
publisher lock beside a healthy registration is the exception: the
`registration` row stays `ok` before and after the handoff, and only setup
stops on it, so check that path yourself when setup refuses and Doctor
shows no registration problem.
Setup never touches a repository's files.
A broken reference is located and explained in the `config` row (see
[A `permissions` reference whose file is broken](#a-permissions-reference-whose-file-is-broken));
this row keeps its `error` and refers to it rather than repeating it.

The handoff edits your user file on its own, so a user file that breaks a
configuration rule blocks the user scope even when a repository override
keeps `config` clear; the row then names the setting that blocks it.

A repository's scope describes the files on disk. The grants a command
actually gets in a repository still combine Darkmatter's user, committed,
and working-copy policy; see Darkmatter's
[permissions](../../../darkmatter/docs/topics/permissions.md).

### `tts`: text-to-speech

Reads the `tts` setting of your user configuration. When the configuration
cannot be read or is invalid, the row is `info`: "speech settings could not
be determined". If `config` is clear because the repository's overrides
replace the setting your user file breaks, this row names that setting
itself (see [Which file a rule names](#which-file-a-rule-names)).

- With automatic selection, the row names the best-rated available
  provider and, when it differs, the provider Claudine tries first. The
  severity follows the best-rated provider, but speech runs the first one
  that works, so the row says "uses" only when they are the same. With
  `TTS_PROVIDER=espeak` and Piper installed, for example, the row reads
  "the best available speech provider is Piper (good quality); hook speech
  tries eSpeak NG first".
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
2 seconds. A PowerShell probe also keeps its startup and module caches in a
temporary folder that is deleted afterwards, so nothing under your profile
changes. Every probe sees the same `PATH` that doctor itself inherited. A shell that is not installed is not mentioned. For each shell
that is, the row says which of these happened:

| Outcome | Severity |
| --- | --- |
| Found `claudine` | `ok` (listed in the detail) |
| Ran and did not find it | `warn` |
| Timed out, could not start, or failed | `warn` |

A shell's result is never an `error`, so a shell you never open cannot fail
doctor or a CI step. The first line of the row's detail is always what doctor's
own process found, including the path it found, even when a shell's warning
becomes the row's summary:

```text
• [warn] Executable path (path): a fresh Git Bash without your startup files cannot find `claudine`
    - this process finds `claudine` at C:\Users\me\.cargo\bin\claudine.exe
    - C:\Program Files\Git\bin\bash.exe: a fresh Git Bash without your startup files cannot find `claudine`
    - asked without its startup files: no profile, rc file, or BASH_ENV
    - Remedy: add the directory that holds `claudine` to the user or system PATH Git Bash starts with, not only to a profile script
```

No shell is probed on macOS,
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

### Files the repairs cannot edit in place

Both repairs edit YAML with Darkmatter's format-preserving editor, which
changes only the text of the values it edits and keeps every comment,
quote, and blank line outside them. A `register` entry the registration
repair removes takes the comment on its line with it (see
[Registration repair](#registration-repair)). It cannot add, replace, or
remove something *inside* an inline (flow) collection, written on one line
in `[…]` or `{…}`, or inside a value that carries an anchor (`&name`),
alias (`*name`), or tag (`!tag`).
YAML means the same thing either way, but only a plain block value, one
`key: value` or `- item` per line with no anchor, alias, or tag, can be
edited in place.

Before it offers a repair, doctor applies the exact edits the repair would
make to the file as it read it, without writing anything. When the editor
would refuse one, the row is not fixable: it names the file and the part
of it to change, without quoting any value, and tells you to rewrite that
part in block form. Where the row also explains why the repair is
unavailable, a refused root is described as written inline; a refused
setting is described as reaching an inline collection or a value with an
anchor, alias, or tag, since doctor does not tell those apart. For
example, Claudine's entry cannot be appended to this list:

```yaml
# ~/.darkmatter.yaml, which doctor cannot edit
register: [~/.acme/manifest.yaml]
```

```text
• [error] Darkmatter host registration (registration): no `register` entry names Claudine's manifest
    - /home/me/.darkmatter.yaml: no `register` entry names Claudine's manifest
    - Remedy: rewrite the `register` list of `~/.darkmatter.yaml` in block form, one `key: value` or `- item` per line, without a `{…}` or `[…]` collection, anchor, alias, or tag, then run `claudine doctor --fix`
```

Rewritten in block form, the same list is repaired:

```yaml
# ~/.darkmatter.yaml, which doctor can edit
register:
  - ~/.acme/manifest.yaml
```

Only an edit that actually reaches the inline part counts, so these are
still repaired:

- an inline list that already names Claudine's manifest and no retired
  one, when the repair only removes the retired files (the list is left
  byte for byte);
- an omitted, `null`, or empty `register: []` in a block root, which the
  repair replaces whole;
- an inline value that the repair moves or replaces whole, such as
  `permissions: {network: deny}` in a block root;
- an empty inline root `{}`, which receives its first key;
- a Claudine file with an inline root whose `permissions` already equals
  Darkmatter's block, when the repair only writes the reference into
  Darkmatter's file.

What a refusal holds back depends on the repair:

- **Registration.** The whole repair becomes manual, because the retired
  files are removed only after the entry is in place. Nothing is published,
  replaced, or removed until the file is rewritten. A root mapping written
  inline (`{register: [], …}`) blocks the repair even with an empty list.
- **Permissions handoff.** Only that scope becomes manual, and neither of
  its files changes. The other scope is still handed off. A transfer into an
  inline Claudine root (`{canonical_provider: claude}`), or a reference
  written into an inline Darkmatter root, is refused this way.

The repairs keep the writer's own checks too: a file rewritten inline
between the inspection and the write is refused by the writer, and nothing
is saved. `claudine setup` stages the same edits and reports the writer's
error, which names the line, column, and setting.

### Registration repair

The repair needs a valid existing `~/.darkmatter.yaml` whose `register`
edit the editor can make in place. When the file is missing or invalid, the
row says so and suggests `claudine setup` or fixing the file first; when the
edit would reach an inline list or root mapping, or an entry or list with an
anchor, alias, or tag, it names that part (see
[Files the repairs cannot edit in place](#files-the-repairs-cannot-edit-in-place)).

```mermaid
flowchart LR
    A[Check for links] --> B[Publish the registration]
    B --> C[Remove extra files]
    C --> D[Verify the published files]
    D --> E[Add the register entry]
    E --> F[Verify the entries]
    F --> G[Remove the format 1 files]
```

- Publishing uses the same publisher as `claudine setup`. It may overwrite
  changed files and delete extra files inside
  `~/.claudine/registrations/darkmatter/`, and nowhere else.
- An entry is extra only when no published path reaches it. On a volume
  that ignores letter case (macOS and Windows by default), a file or folder
  whose name differs from the published one only in case, such as
  `schemas/Action.yaml` after a backup restore, is the published file: it is
  checked byte for byte and never deleted. On a case-sensitive volume
  (Linux) it is a different file, so the repair publishes
  `schemas/action.yaml` and removes `schemas/Action.yaml`.
- The `register` entry is written with Darkmatter's locked writer, keeping
  your other settings and other hosts' entries. Of the entries that name
  Claudine's current or retired manifest, in any spelling and whether or not
  the retired file still exists, the first is kept (rewritten to the current
  manifest if needed, with the comment on its line) and every later one is
  removed in the same edit together with the comment on its line; every
  other comment is kept. The repair then checks both that an entry names
  Claudine's manifest and that no retired entry is left.
- The retired `~/.claudine/darkmatter-host.yaml` and
  `~/.claudine/darkmatter-host/` are removed only after the entry is in
  place. If anything before that fails, they stay, so a retry or an older
  Claudine still works. If removing them fails partway (a read-only
  `~/.claudine` lets the folder's contents go but not the folder), the
  row's `Already changed:` lines name each entry already removed, a removed
  folder standing for everything in it, and the next `--fix` removes and
  reports only what is left:

  ```text
  Repair failed: could not remove /home/me/.claudine/darkmatter-host (permission denied)
  Already changed: updated the `register` entry in /home/me/.darkmatter.yaml
  Already changed: removed /home/me/.claudine/darkmatter-host/0123abcd
  ```
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
original block stays in force. The repair edits only existing, valid files
(a `.darkmatter.yaml` with an invalid `register` setting is not offered for
repair, because the writer would refuse it):
it never runs setup, converts a legacy configuration, or writes through a
symbolic link or junction. That covers each file, the lock file the writer
takes beside it, and every folder between the file and its scope's root
(your home, or the repository root): when the repair would write
Claudine's file through a linked `~/.claudine` or `<repo>/.claudine`, the
row names the link and neither file changes. A
conflicting block is never merged; the row tells you to choose one. See
[Configuration → Darkmatter's permissions](../topics/configuration.md#darkmatters-permissions).

Inside a repository the user and repository handoffs are independent. When
one fails, for example because another writer holds `~/.darkmatter.yaml`'s
lock, that scope keeps its inline block and the other scope is still handed
off in the same run. The row is an `error` whose reason names the failed
scope, and it lists the other scope's writes as `Already changed:`:

```text
Repair failed: user scope: another writer held the lock /home/me/.darkmatter.yaml.darkmatter-lock; nothing was saved
Already changed: /home/me/repo/.claudine/config.yaml: set `permissions`
Already changed: /home/me/repo/.darkmatter.yaml: set `permissions`
```

Running `--fix` again once the lock is free hands off the remaining scope.

## See also

- [`claudine setup`](./setup.md): sets Claudine up with Darkmatter; doctor's
  remedy for a missing configuration.
- [Configuration](../topics/configuration.md)
- [Schemas](../topics/schemas.md)
