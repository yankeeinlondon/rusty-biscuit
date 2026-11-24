---
kind: canonical-documentation
blast_radius:
  - claudine/cli/src/commands/registrations.rs
  - claudine/lib/src/composition/registration.rs
  - claudine/justfile
---
# Claudine's `registrations` command (maintainers)

`claudine registrations` shows, and installs, the files Claudine registers
with Darkmatter: the schemas of `claudine/registration/`, the generated
lifecycle artifact, and the `darkmatter-host` manifest that lists them. `md`
and DMLS read that installed set to understand Claudine documents (see
[Schemas → The published host registration](../topics/schemas.md#the-published-host-registration)).

The command is hidden from `claudine --help` because it is a maintenance
tool: `just install` calls it, and you call it to check what a binary would
install or to repair an installation. It needs no Claudine or Darkmatter
configuration and never changes either.

## Usage

```sh
# List every file this binary would install.
claudine registrations

# Print one file with its exact contents (the path exactly as listed).
claudine registrations schemas/seq.yaml

# Install the set under ~/.claudine/registrations/darkmatter/.
claudine registrations publish
```

| You type | It does | Reads or writes |
| --- | --- | --- |
| `claudine registrations` | prints a `file[]` list | nothing on disk |
| `claudine registrations <filepath>` | prints that file's `file_details` | nothing on disk |
| `claudine registrations publish` | installs the set, then prints the list | only `~/.claudine/registrations/` |

Listing and printing describe the copy compiled into the running binary,
not what is installed, so they work before anything was ever published.
`publish` is a reserved word, never a file path: every published path ends
in `.yaml` or `.json`.

## Output

Every form prints one pretty-printed JSON document and a newline to stdout,
and nothing else there. Errors and their causes go to stderr with a nonzero
exit status, and stdout stays empty.

A listing is an array sorted by `filepath`. It always includes the manifest
(`kind: manifest`) and the lifecycle artifact (`kind: lifecycle`), followed
by one entry for each file of `claudine/registration/` under `schemas/`
(17 entries in all today). The start of a real listing:

```json
[
  {
    "kind": "lifecycle",
    "filepath": "globals.lifecycle.json",
    "hash": "65b2a01fc37ee498"
  },
  {
    "kind": "manifest",
    "filepath": "manifest.yaml",
    "hash": "c267e7f25cd136dd"
  },
  {
    "kind": "schema",
    "filepath": "schemas/action.yaml",
    "hash": "3aacfe80b81d5c8b"
  },
  {
    "kind": "schema",
    "filepath": "schemas/claudine-config.yaml",
    "hash": "3156d81967419cca"
  },
  {
    "kind": "schema-trigger",
    "filepath": "schemas/composition.yaml",
    "hash": "da76020c2097827c",
    "name": "composition"
  }
]
```

The hashes change whenever a file's bytes change, so compare them between
two binaries, or between a binary and an installation, rather than with
this page.

| Field | Meaning |
| --- | --- |
| `kind` | `manifest`, `schema-trigger`, `schema`, `yaml-schema`, `namespace-schema`, or `lifecycle` |
| `filepath` | the path inside `~/.claudine/registrations/darkmatter/`, written with `/` on every OS |
| `hash` | XXH64 (seed 0) of the file's exact bytes, as 16 lowercase hexadecimal digits |
| `name` | the file's display name; the key is absent when the file declares none |

Printing one file adds `contents`, its complete text. The files are UTF-8,
so encoding `contents` as UTF-8 gives back exactly the bytes `hash`
describes. A path that is not in the listing, in any other spelling
included (`./schemas/seq.yaml`, a backslash), exits 1 with nothing on stdout:

```text
Error: `./schemas/seq.yaml` is not a published registration path; run `claudine registrations` for the list
```

The manifest's `contents` is the format 2 `darkmatter-host` manifest
Darkmatter reads. It lists every other file with its BLAKE3 digest (a
different hash from the listing's XXH64 `hash`):

```yaml
# Written by Claudine from the registration compiled into it, and rewritten when that changes; do not edit.
kind: darkmatter-host
format_version: 2
host: claudine
version: "0.1.0"
files:
    - "./schemas/action.yaml"
    # … every other file under schemas/, in path order
lifecycle: "./globals.lifecycle.json"
functions: []
digests:
    "./globals.lifecycle.json": "73347213b879e94fe6fc409e460f322fd03825d329e82d43b8163b35d367901a"
    "./schemas/action.yaml": "b89504d935dc16eaa8b837245bbd244c33f59a7ac31e904f346d46bef833651f"
    # … one digest for each listed file
```

## Publishing

`publish` writes only files whose bytes differ, and the manifest last, so a
reader sees either the old set or the new one. When the installation is
already current it writes nothing (every file keeps its modification time)
and still prints the list. It refuses to write through a symbolic link or
junction, or over something that is not a directory where one belongs; the
error names the path. Such a link is almost always found before anything is
written. If a publish stops partway (a link appeared during it, or a file
could not be written or removed), the error lists every file it already
wrote or removed, for example
`the registration was only partly published; already changed: removed ~/.claudine/registrations/darkmatter/.claudine-tmp-…`.
The manifest is written last, so the old one still stands, and publishing
again finishes the job.

Publishing never edits `~/.darkmatter.yaml`. The `register` entry that
points `md` and DMLS at the manifest is added by Claudine's first-run setup.

No other `claudine` command reads or repairs the installed files. If a file
is missing or edited, run `claudine registrations publish` (or
`just install`) to restore it; until then `md` and DMLS refuse the whole
registration (see
[Schemas → Hand edits, recovery, and overrides](../topics/schemas.md#hand-edits-recovery-and-overrides)).

## How `just install` publishes

`just install` in `claudine/` builds Claudine with `just build --release`,
installs it with `cargo install`, and then runs `just publish-registrations`.
That recipe runs `registrations publish` with the release binary it just
built, found in Cargo's target directory (`cargo metadata`, `.exe` on
Windows), never a `claudine` found on `PATH` or in the install root, which
could be an older build. Install flags such as `--root` or `--force` change
where the binary goes, not what is published. A failed publish fails
`just install`.

```sh
# Publish from the current worktree's release build without reinstalling.
just publish-registrations
```
