---
blast_radius:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/output/version_report.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/python.rs
---

# The `sniff repo version` Subcommand

Reports the declared version(s) for the current package / package-area / repo
context. Scoped like [`sniff repo test-runner`](./repo_test-runner.md) — by
default the CWD picks the scope (package when inside a single crate, the
surrounding package-area, or repo at the monorepo root); `--all`,
`--package <NAME>`, and `--package-area <NAME>` override that resolution.

Mirrors the `repo test-runner` collapse rule: uniform repos collapse to one
entry (e.g. every crate at `0.1.0`); divergent versions stay as separate
entries; a single-package / non-monorepo directory reports its singular
version. Sources are tracked per collapse entry, so JSON never implies a
version came from one manifest when several packages read the same value from
different manifests.

## Default Behavior

Prints the version string(s) and exits `0`. Uniform scope collapses to a
single value:

```
0.1.0
```

Variance renders a comma-separated list:

```
0.1.0, 0.2.0
```

When no version is resolvable anywhere in scope, prints nothing, emits a hint
on stderr, and exits `1` (use `--no-error` to suppress the exit code).

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Use `<DIR>` as the working directory instead of the CWD. The enclosing repo is discovered from it, so `<DIR>` only picks the scope — `--all` from inside a package still spans the whole repo. |
| `--all` | Scope to every package in the repository (overrides CWD) |
| `--package <NAME>` | Scope to a specific package by name (overrides CWD) |
| `--package-area <NAME>` | Scope to a specific package area by name (overrides CWD) |
| `--csv` | Render as a single-line comma-separated list |
| `--list` | Render as a newline-delimited list (one per line) |
| `--md` | Render as a Markdown unordered list (`- version` per line) |
| `--json` | Render as JSON |
| `-v/--verbose` | Append the manifest source (hyperlinked) and the attributing package |
| `--no-error` | Exit `0` with no output when no version is found |
| `--on-error <MESSAGE>` | Message to display when no version is found (text mode only) |
| `--plain` | Strip terminal styling (text mode) |

`--csv` / `--list` / `--md` are mutually exclusive. `--all` /
`--package` / `--package-area` are mutually exclusive (and the CWD scope
kicks in when none of them is set).

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | At least one version was found (or `--no-error` was passed) |
| `1` | No version found in scope (unless `--no-error`) |

## Examples

```bash
# Monorepo root → all distinct versions across the repo
sniff repo version
# → 0.1.0

# Inside a single crate → that crate's version
(cd sniff/lib && sniff repo version)
# → 0.1.0

# Inside a package-area → distinct versions across the area
(cd sniff && sniff repo version)
# → 0.1.0

# Scope overrides (work from any CWD)
sniff repo version --all
sniff repo version --package sniff-lib
sniff repo version --package-area sniff

# Unknown override names error clearly
sniff repo version --package ghost
# → error: unknown package 'ghost' (valid: biscuit, sniff, ...)

# Formats
sniff repo version --csv
sniff repo version --list
sniff repo version --md

# Verbose appends the source manifest and attributing package
sniff repo version -v
# → 0.1.0 (from Cargo.toml in sniff-lib)
```

## Verbose Source Detail

`--verbose` / `-v` appends a parenthetical to each entry:

- **Single source** (most common case): hyperlinked manifest path.
  Workspace-inherited Cargo versions name the source as
  `[workspace.package]` in the root `Cargo.toml` so the user can tell the
  value was inherited, not literal.
- **Multiple sources** for the same version (e.g. a workspace root plus
  a literal override in one member): renders `from N manifests` instead
  of picking a misleading first source.

Verbose markup is rendered through `biscuit-terminal`'s `Prose`, so
hyperlinks, dim/italic/blue styles, and `--plain` all behave like the rest
of the CLI.

## JSON Output (`--json`)

```bash
sniff repo version --json
```

Always returns the `{ "versions": [...] }` shape:

```json
{
  "versions": [
    {
      "version": "0.1.0",
      "packages": ["sniff-lib", "sniff-cli"],
      "sources": [
        {
          "manifest": "Cargo.toml",
          "path": "sniff/lib/Cargo.toml",
          "href": "file:///repo/sniff/lib/Cargo.toml",
          "inherited": false,
          "packages": ["sniff-lib"]
        }
      ]
    }
  ]
}
```

- Each `version` entry carries the in-scope `packages` (first-seen order)
  and the distinct `sources` that contributed the value.
- Each `source` carries its own `packages` (which packages read this
  version from this manifest), the manifest `path` (repo-relative), an
  absolute `href` for terminal hyperlinks, and the `inherited` flag.
- An empty result emits `{ "versions": [] }` on stdout and exits `1`
  unless `--no-error`. The `--on-error` text is text-mode only — JSON
  stdout stays valid JSON.

## Bare `sniff repo --json` Aggregate

The consolidated `SniffRepo` aggregate keeps a single top-level
`version: string | null`. It is the **`AggregateScope::Repo` collapse**:

- exactly one distinct version across all packages → that string;
- zero or more-than-one distinct versions → `null`.

A pure-virtual Cargo workspace with uniform member versions now reports
the version string instead of `null`. The serialized type does not change,
so consumers of `repo --json` keep their existing `string | null`
contract; only the value improves.

## Related Subcommands

| Subcommand | Output |
|------------|--------|
| [`name`](./repo_name.md) | Repository name |
| [`is-monorepo`](./repo_is-monorepo.md) | Monorepo label (e.g. `cargo`; `false` if not) |
| [`package-count`](./repo_package-count.md) | Number of discovered packages |
| [`test-runner`](./repo_test-runner.md) | Declared test runners with evidence |
| [`package-manager`](./repo_package-manager.md) | Declared package managers |
