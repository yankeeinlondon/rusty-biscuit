# `md init`

## Overview

`md init` creates your Darkmatter configuration file, `~/.darkmatter.yaml`,
with every setting written at its default, so you can see what is in force
and edit it. Run it again at any time: it only adds settings that are
missing and never changes what you wrote.

```sh
md init                 # create or complete ~/.darkmatter.yaml
md init --repo          # ... and the repository's .darkmatter.yaml
md init --inline        # write a referenced permissions block back inline
```

What the settings mean is described in
[Configuration Files](../topics/configuration.md).

## Usage

```sh
md init [--repo] [--inline] [--output <TARGET>]
```

### Options

- `--repo`: also create or complete `{repo-root}/.darkmatter.yaml`, with the
  repository defaults. Outside a repository the command fails before writing
  anything, the user file included.
- `--inline`: when a file's `permissions` is a reference to another file
  (`permissions: ~/.claudine/config.yaml`), replace it with the block that
  file holds. If that file cannot be read, the reference is replaced with
  the defaults and the output says so, because the defaults may change your
  permissions. Without `--inline`, a reference is always left in place.
- `--output <terminal|plain|markdown|markdown-plus|html|browser|json|json5|yaml|toml>`:
  What to print (default: `terminal`).
- `--width <N|N%|auto|source|unlimited>`: see
  [Output and Width](./output-and-width.md#choosing-a-width).

## What it writes

For a missing file, every setting at its default:

```yaml
permissions:
  file-read:
  - ./*.txt
  - ./*.text
  - ./*.md
  - ./*.csv
  - ./*.tsv
  - ./*.json
  - ./*.json5
  - ./*.yaml
  - ./*.yml
  - ./*.toml
  shell-commands: []
  network: deny
register: []
```

The repository file is the same with `&**/*.<ext>` entries (files anywhere in
the repository) instead of `./*.<ext>`.

For an existing file, only a missing top-level setting is added. Given

```yaml
# mine
permissions:
  network: allow   # I fetch remote schemas
```

`md init` appends `register: []` and changes nothing else. A file that
already has both settings is not written at all, and its modification time
does not change.

An invalid file is not repaired: the command fails with the file's located
problems (`dm.config.invalid`, or `dm.config.reference` for a malformed
reference) and writes nothing. Fix it by hand, or use `--inline` when the
problem is a reference.

When Claudine is set up, `~/.darkmatter.yaml` holds a reference to
`~/.claudine/config.yaml`; `md init` keeps it. See
[When Claudine is set up](../topics/configuration.md#when-claudine-is-set-up).

## Output

The formatted outputs list each file and what happened to it:

```text
- Created `/home/me/.darkmatter.yaml` (user configuration)
- Unchanged `/home/me/project/.darkmatter.yaml` (repository configuration)
```

A data output is one object:

```json
{
  "files": [
    { "path": "/home/me/.darkmatter.yaml", "scope": "user", "action": "created" }
  ],
  "notices": [],
  "outcome": "success",
  "diagnostics": []
}
```

| Field | Meaning |
|---|---|
| `files` | one row per file, the user file first; `action` is `created`, `updated`, or `unchanged` |
| `notices` | `{ file, message }` rows, such as defaults written in place of an unreadable reference |
| `outcome`, `diagnostics` | the run's status and its records; stderr stays empty |

`md init` reports no `dm.config.missing` warning: the file it would name is
the one being created. Other configuration notices, such as the conversion
of a legacy policy file, are reported as for every command.

The schema is `cli/schemas/output/init.yaml`.

## Exit status

| Status | When |
|---|---|
| 0 | every file was created, completed, or already complete |
| 1 | nothing was written: `--repo` outside a repository, no home directory (`dm.config.not_saved`), an invalid existing file, or a failed write |
| 2 | a usage error |
