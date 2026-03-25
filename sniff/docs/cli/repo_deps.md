---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/lib/src/filesystem/repo.rs
---

# The `sniff repo deps` Subcommand

Renders the internal dependency graph of a monorepo workspace — showing which packages depend on which other packages within the same workspace. Only applies to monorepos (Cargo workspaces, pnpm workspaces, Nx, etc.); exits with an error message if no workspace packages are found.

## Arguments and Flags

```
sniff repo deps [--ui] [filter]
```

| Argument | Description |
|----------|-------------|
| `filter` | Optional positional filter — see [Package Filtering](#package-filtering) below |
| `--ui`   | Render the dependency graph as a Mermaid flowchart diagram instead of text |

The filter can also be supplied at the `repo` level and is inherited by `deps` when no subcommand-level filter is given. The subcommand-level filter takes precedence when both are set.

## Default Text Output

Without `--ui`, the command prints a styled dependency list. The title line shows the number of packages participating in dependency relationships:

```
Dependencies (N packages with dependencies)
```

When a filter is active, the title instead shows how many packages are being shown out of the total:

```
Dependencies (showing M of N packages)
```

Each participating package is shown as a top-level bullet with its name in bold blue. Indented beneath it are the dependency relationships:

- **`depends-on:`** — a comma-separated list of workspace-internal packages that this package imports
- **`used-by:`** — a comma-separated list of workspace-internal packages that import this package

At the end of the output a dim hint line is printed:

```
use the --ui flag to show this in a visual format
```

### Isolated Packages

Packages that have neither `depends_on` entries nor `used_by` entries are **omitted** from text output by default. They are silently excluded unless an explicit filter is provided — when a filter is active, all packages matching the filter are shown regardless of whether they participate in any dependency relationship.

## Visual Mode (`--ui`)

```
sniff repo deps --ui
```

Builds a Mermaid `flowchart TD` diagram from the workspace dependency data and renders it inline in the terminal using `MermaidDiagram` (via biscuit-terminal). Falls back to a fenced code block if the terminal cannot display images or `mmdc` is not available.

The diagram structure:

- Packages are grouped into **subgraphs** by their `package_area` (e.g., `subgraph sniff`)
- A single root-level package (`package_area == "root"`) is emitted as a plain node without a subgraph
- Each package is a labeled node: `n0["package-name"]`
- Edges are drawn from each package to each of its `depends_on` entries: `n0 --> n1`
- Only edges where both the source and target are present in the (possibly filtered) package list are drawn

If no internal dependency edges exist among the (filtered) packages, the message `No internal dependencies found between workspace packages` is printed instead.

## Package Filtering

The optional positional `filter` argument controls which packages are included. The filter syntax is shared across all `repo` subcommands:

| Pattern | Effect |
|---------|--------|
| `biscuit` | Include packages whose name contains `biscuit` (case-insensitive substring match) |
| `@sniff` | Include packages whose `package_area` equals `sniff` |
| `!biscuit` | Exclude packages whose name contains `biscuit` |
| `!@sniff` | Exclude packages whose `package_area` equals `sniff` |

When a filter is active, text mode shows all matched packages (including isolated ones). Visual mode only draws edges for packages present in the filtered set — cross-area edges to packages outside the filter are silently dropped.

## JSON Output

```
sniff --json repo deps [filter]
```

Returns the full repo data at the top level, equivalent to `sniff --json repo`. Each package object in the `packages` array includes:

```json
{
  "name": "sniff-cli",
  "package_area": "sniff",
  "relative": "sniff/cli",
  "depends_on": ["sniff-lib"],
  "used_by": [],
  ...
}
```

The `depends_on` and `used_by` fields are omitted from JSON when they are empty (controlled by `#[serde(default, skip_serializing_if = "Vec::is_empty")]`). The `--ui` flag has no effect on JSON output.

## Plain Output

```
sniff --plain repo deps [filter]
```

Adding `--plain` strips all ANSI escape codes (colors, bold, italic, hyperlinks) from the text output. The `--ui` flag still renders the Mermaid diagram; `--plain` only affects the styled text around it.
