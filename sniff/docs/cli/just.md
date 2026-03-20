# The `sniff just` Subcommand

Detects all justfiles within the current scope and catalogs their recipes. Each recipe body is hashed with xxHash (via `biscuit-hash`) for content fingerprinting.

## Scope Resolution

- If the current directory is inside a **git repository**, all justfiles under the repo root are discovered
- If **not** in a git repo, all justfiles under the current directory (or `--base`) are discovered
- Hidden directories (`.git`, `.cache`, etc.), `node_modules`, `target`, and `vendor` are always skipped
- Recognized filenames: `justfile`, `Justfile`, `.justfile`

Justfile reads and recipe parsing are parallelized with rayon.

## Default Behavior (Non-verbose)

When invoked without `-v`, the output is a compact `UnorderedList` (from biscuit-terminal). Each item is:

- Prose::new(`<a href="file://{absolute_path}">{relative_path}</a> <dim>({N} recipes, <i>{M} private</i>)</dim>`)
- The relative path is an **OSC8 hyperlink** pointing to the file's absolute `file://` URL
- The private count portion (`, <i>{M} private</i>`) is only shown when there are private recipes
- **No recipe list** is shown underneath

## Verbose Mode (`-v`)

Adding `--verbose` or `-v` expands each justfile entry to show its recipes underneath as an indented `UnorderedList` with bullet `"  "`.

### Recipe Rendering

Each recipe line renders as:

- Prose::new(`<purple-500>{name}</purple-500> {params} <dim>{description}</dim>`)
- Private recipes use `<dim>{name}</dim>` instead of purple

### Parameter Display

Parameters are parsed into structured form with awareness of required vs optional:

| Form | Example | Display | Style |
|------|---------|---------|-------|
| Required | `name` | `name` | Normal weight |
| Optional (empty default) | `*args=""` | `*args` | `<dim>` |
| Optional (with default) | `env="staging"` | `env="staging"` | `<dim>` |
| Variadic required (`+`) | `+targets` | `+targets` | Normal weight |
| Variadic optional (`*`) | `*args` | `*args` | `<dim>` |

The `=""` trailing (empty string default) is stripped from display to reduce noise.

### Description Alignment

- Descriptions are right-padded using `PadRight` so that all descriptions for a given justfile start at the same column
- The column width is calculated as the maximum `name + params` visible width across all visible recipes in that justfile, plus 2 characters of breathing room
- Descriptions come from `#` comment lines immediately above a recipe declaration (per just convention)
- Multiple consecutive comment lines are joined with spaces

### Double-verbose (`-vv`)

At verbosity level 2+, each recipe also shows its xxHash:

- Prose::new(`  <dim>#{hash:016x}</dim>`)

## The `default` Recipe

The `default` recipe is treated specially:

- It is **never** listed in the recipe list
- It is tracked via the `has_default` boolean on `JustfileInfo`
- When a justfile does **not** have a `default` recipe, a warning is emitted on **stderr**:
  - Prose::new(`<i><dim>no </dim><blue>default</blue><dim> recipe was defined in </dim>{relative_path}</i>`)
- This warning appears after all stdout output

## Path Filtering (Positional Args)

```
sniff just [filter...]
```

- Zero or more positional arguments act as path substring filters
- **OR logic**: a justfile is included if its path contains ANY of the filter strings
- Matching is case-insensitive
- Example: `sniff just sniff homelab` shows justfiles whose path contains "sniff" OR "homelab"

## Recipe Filtering (`--with <recipe>`)

```
sniff just --with build
sniff just --with build sniff homelab
```

Filters justfiles by those containing a specific recipe name.

### Non-verbose (default)

- Only justfiles **with** the recipe are displayed
- Header: Prose::new(`<b>Justfiles</b> with <purple-500>{recipe}</purple-500> <dim>({N})</dim>`)
- If no justfiles match: Prose::new(`<dim>No justfiles contain the </dim><purple-500>{recipe}</purple-500><dim> recipe.</dim>`)

### Verbose (`-v`)

- Both groups are shown with headings:
  - Prose::new(`<b>With</b> <purple-500>{recipe}</purple-500> <dim>({N})</dim>`)
  - Prose::new(`<b>Without</b> <purple-500>{recipe}</purple-500> <dim>({N})</dim>`)
- Each group renders the full recipe-expanded list

Path filters and `--with` can be combined: `sniff just --with build sniff` shows only justfiles whose path contains "sniff" AND which have a `build` recipe.

## JSON Output (`--json`)

```
sniff --json just [filter...] [--with <recipe>]
```

Returns a JSON array of `JustfileInfo` objects:

```json
[
  {
    "path": "/absolute/path/to/justfile",
    "relative": "package/justfile",
    "has_default": true,
    "recipes": [
      {
        "name": "build",
        "params": [
          {
            "name": "args",
            "variadic": true,
            "optional": true,
            "default": ""
          }
        ],
        "private": false,
        "description": "Build the project",
        "body": "    cargo build {{args}}",
        "hash": 2743948372849283
      }
    ]
  }
]
```

- When `--with` is used, only matching justfiles are included in JSON output
- The `default` recipe is excluded from the `recipes` array; its presence is tracked via `has_default`
- `description` is `null` when no comment precedes the recipe
- `params` is a structured array (never raw text) with `name`, `variadic`, `optional`, and `default` fields

## Plain Output (`--plain`)

Adding `--plain` strips all ANSI escape codes (colors, bold, links) from the text output.
