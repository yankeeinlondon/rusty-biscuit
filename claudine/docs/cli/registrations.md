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
(`kind: manifest`) and the lifecycle artifact (`kind: lifecycle`). An
excerpt (the hashes are illustrative):

```json
[
  {
    "kind": "lifecycle",
    "filepath": "globals.lifecycle.json",
    "hash": "5f0c2a9e81d34b77"
  },
  {
    "kind": "manifest",
    "filepath": "manifest.yaml",
    "hash": "0b6e1f2d3c4a5968"
  },
  {
    "kind": "schema-trigger",
    "filepath": "schemas/composition.yaml",
    "hash": "9a8b7c6d5e4f3021",
    "name": "composition"
  }
]
```

| Field | Meaning |
| --- | --- |
| `kind` | `manifest`, `schema-trigger`, `schema`, `yaml-schema`, `namespace-schema`, or `lifecycle` |
| `filepath` | the path inside `~/.claudine/registrations/darkmatter/`, written with `/` on every OS |
| `hash` | XXH64 (seed 0) of the file's exact bytes, as 16 lowercase hexadecimal digits |
| `name` | the file's display name; the key is absent when the file declares none |

Printing one file adds `contents`, its complete text. The files are UTF-8,
so encoding `contents` as UTF-8 gives back exactly the bytes `hash`
describes.

## Publishing

`publish` writes only files whose bytes differ, and the manifest last, so a
reader sees either the old set or the new one. When the installation is
already current it writes nothing (every file keeps its modification time)
and still prints the list. It refuses to write through a symbolic link or
junction, or over something that is not a directory where one belongs; the
error names the path, and nothing is written.

Publishing never edits `~/.darkmatter.yaml`. The `register` entry that
points `md` and DMLS at the manifest is added by Claudine's first-run setup.

No other `claudine` command reads or repairs the installed files. If a file
is missing or edited, run `claudine registrations publish` (or
`just install`) to restore it.

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
