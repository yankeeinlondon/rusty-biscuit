---
kind: canonical-documentation
---

# `md edit`

`md edit` opens a Markdown file in your editor and waits until you close it, so a
script can pause for a person to write something and then carry on with the file.
It accepts any [file reference](./index.md#file-references), and creates the file
if the reference matches nothing.

```sh
md edit notes.md                     # edit, then print the file's full path
md edit '&docs/plan.md'              # edit a file at the repository root
FILE=$(md edit new.md --output plain) # use the path in a script
```

## How the editor is chosen

`md edit` uses `$EDITOR`, then `$VISUAL`, then the first installed editor from a
default list. A blank variable counts as unset, and a set variable naming a program
that cannot be found gives way to the other variable. The default list is only for
when neither is set: if every set variable names a missing program, `md edit` fails
with exit status 1 and names the variable, rather than opening an editor you did not
choose.

```sh
EDITOR=/usr/local/bin/typo md edit notes.md   # fails; notes.md is not created
```

## Creating a file

When the reference matches nothing, the file is created at the first place the
reference would look: beside you for `./new.md` or `new.md`, at the repository root
for `&new.md`. A malformed reference or a path leaving the repository is refused
before any file is created or an editor is opened. `-` is an ordinary file name here,
because an editor needs a file: `md edit` does not read stdin.

A file that is still empty when the editor closes fails the command with status 1,
so a script never carries on with nothing written. A file `md edit` created for that
run is removed when the edit fails and it is still empty. A file that already existed
is never removed, and neither is one the editor wrote to before failing.

## Output

`md edit` accepts both output families, and the path is what it delivers.

| `--output` | stdout on success |
|---|---|
| `terminal` (default), `plain`, `markdown`, `markdown-plus` | the file's full path |
| `html` | the path, as an HTML page |
| `json`, `json5`, `yaml`, `toml` | one value |

A formatted output writes a failure to stderr. A data output writes one value to
stdout and nothing to stderr, with `file` the resolved path, `editor` the program
launched, and `created` whether `md edit` created the file:

```json
{
  "file": "/work/notes.md",
  "editor": "/usr/bin/vim",
  "created": false,
  "outcome": "success",
  "diagnostics": []
}
```

On a failure, `file` and `editor` are `null` and `diagnostics` says why. The shape is
defined in [`edit.yaml`](../../cli/schemas/output/edit.yaml). See
[Output and Width](./output-and-width.md) and
[Data Contracts](./data-contracts.md#other-commands).

`--width` is accepted, as on every command, and has no effect, because `md edit`
prints no prose.

## Exit status

| Status | When |
|---|---|
| 0 | the editor closed normally and the file has content |
| 1 | the reference is malformed, the editor exited with an error, or the file is empty after editing |
| 2 | a usage error |
