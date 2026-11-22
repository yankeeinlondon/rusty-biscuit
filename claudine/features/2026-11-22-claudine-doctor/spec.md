---
kind: feature
created: 2026-11-22
status: draft-spec
area: claudine
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-22-host-registration-v2
    - 2026-11-22-expression-and-mutation-schemas
depends-on:
    - 2026-11-22-host-registration-v2
next-spec: 2026-11-22-expression-and-mutation-schemas
---

# `claudine doctor`

## Why this exists

Claudine depends on its environment in ways that fail quietly: a Darkmatter
configuration that lost its `register` entry, an inline `permissions` block
that was never moved into Claudine's configuration, a stray configuration key,
no usable text-to-speech provider. Each was found by hand. `claudine doctor`
checks all of them in one command and says what to do about each.

## Decisions

### Command

```text
claudine doctor [--fix] [--json]
```

- A visible command. It reads the environment and changes nothing unless
  `--fix` is given.
- Each check reports one of `ok`, `info`, `warn`, or `error`, with a one-line
  finding and, when not `ok`, what to do about it.
- **Exit code:** `0` when no check reports `error`; `1` otherwise. Warnings
  never fail the command.
- `--json` prints one object per check (`id`, `severity`, `summary`,
  `detail`, `fixable`) for scripts and CI.
- Terminal output uses `biscuit-terminal` components: a table or list of
  checks, colored by severity, with the remedies as prose.

### Checks are a list

Each check is one entry in a plain list: an id, a title, the function that
runs it, and an optional fix. Adding a check later means adding one entry.
There is no plugin system.

### The checks

| # | Id | Checks | Severity when it fails | `--fix` |
|---|---|---|---|---|
| 1 | `config` | `~/.claudine/config.yaml` and `~/.darkmatter.yaml` exist, parse, and validate against `claudine-config.yaml` and Darkmatter's configuration schema | `error` | no |
| 2 | `registration` | every file `claudine registrations` lists exists under `~/.claudine/registrations/darkmatter/`, is readable, and matches its hash; no unlisted file is there; `~/.darkmatter.yaml` registers the manifest; no old-layout files or `register` entry remain | `error` for missing, unreadable, or unregistered; `warn` for modified or unlisted files and old-layout leftovers | yes: republish, fix the `register` entry, remove the old layout |
| 3 | `darkmatter-permissions` | `~/.darkmatter.yaml`'s `permissions` is a reference to `~/.claudine/config.yaml`; in a repository with both `.darkmatter.yaml` and `.claudine/config.yaml`, the same at repository scope | `error` | yes: the same handshake setup performs |
| 4 | `repo-config` | inside a repository, `{repo-root}/.claudine/config.yaml` exists | `info` (a repository configuration is optional) | no |
| 5 | `stray-config-keys` | Claudine's configuration files hold no key that `claudine-config.yaml` does not declare (`permissions` is declared: it belongs to Darkmatter) | `warn`, naming each key and its line | no: removing a key is the user's decision |
| 6 | `tts` | a text-to-speech provider is available, and the best one is good | see below | no |
| 7 | `path` | the `claudine` executable is on the executable path | `error` | no |

Check 2 uses the hidden `claudine registrations` command's data (in-process,
the same embedded set), defined in `2026-11-22-host-registration-v2`. Until
that spec lands, check 2 is the only check that waits; the others can land
first.

### Text-to-speech (check 6)

Uses `biscuit-speaks`' provider detection (`get_available_providers`) and its
quality tiers (`provider_base_quality`), so the rule follows the library rather
than a list kept here:

| TTS in Claudine's configuration | No provider available | Best provider below `Good` | Best provider `Good` or better |
|---|---|---|---|
| enabled | `error` | `warn` | `ok` |
| disabled | `info` | `info` | `ok` |

Today the library rates Kokoro and Echogarden `Excellent`; `say`, SAPI,
Piper, gTTS, and Sherpa `Good`; Mimic3, Festival, Pico2Wave, and `spd-say`
`Moderate`; eSpeak `Low`.

### Executable path (check 7)

- **macOS and Linux:** the current process's `PATH`.
- **WSL2:** a Linux environment; `doctor` run inside WSL checks WSL's `PATH`.
  It does not check the Windows side.
- **Native Windows:** each installed shell, because their paths differ:
  - PowerShell (`pwsh` when installed, otherwise Windows PowerShell),
  - Git Bash / MSYS2 when installed,
  - Cygwin when installed.

  Each shell is started non-interactively and asked for `claudine`; a shell
  that is not installed is skipped, not reported. Installed shells are found
  with `sniff`. The finding names each shell that cannot find `claudine`.

## Documentation to update

- `claudine/docs/cli/`: a `doctor` page with every check, its severities, and
  what `--fix` does.
- `claudine/README.md`: one line pointing to `claudine doctor` for
  troubleshooting.
- The `claudine` repo skill and the `use-claudine` caller skill, then
  `just publish-skills`.

## Tests

- Each check in its passing and failing states, against a temporary home
  folder (no real `~` touched).
- `--fix` for checks 2 and 3 leaves the environment passing and preserves
  every other key and comment in the files it edits.
- Exit code `0` with only warnings, `1` with any error.
- `--json` output parses and lists every check.
- Check 6 with TTS enabled and disabled, with a fake provider list.
- Check 7 on Windows with each shell present and absent (an L2 test on a
  Windows rig).
