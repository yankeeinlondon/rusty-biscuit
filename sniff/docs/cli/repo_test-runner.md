---
blast_radius:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/output/test_runner_report.rs
  - sniff/lib/src/filesystem/repo/test_runner_usage.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/programs/test_runner_spec.rs
---

# The `sniff repo test-runner` Subcommand

Reports the test runner(s) a repository **declares**, collapsed across the
packages in scope, with the evidence that triggered each detection.

This is the repo-usage surface. It answers "which runner does this repo/package
*use*?" by inspecting manifest dependency keys, config files, ecosystem
defaults, and naming conventions. It is distinct from
[`sniff software test-runners`](../../cli/README.md), which answers "is this
runner *installed and runnable* on the host?" The two surfaces share one
`TestRunner` enum but use different evidence; do not conflate them.

No network requests or deep git analysis are performed — detection uses the same
repository-structure scan as `sniff repo structure`.

## Default Behavior

The command aims for a **single answer**: the test runner you actually use. The
default text output is comma-separated (CSV); when the scope resolves to one
runner it is a single bare name:

```
cargo-nextest
```

A package that configures or declares a runner reports **that runner alone** —
the implicit `cargo test` / `go test` / `unittest` ecosystem default is
superseded (see [Prioritization](#prioritization)). Only when several packages
in scope use *different* runners is the answer a list:

```
cargo-nextest (sniff package), vitest (sniff-cli package)
```

When no runner is declared for the context, nothing is written to STDOUT (a dim
hint goes to STDERR unless `--plain`) and the process exits `1`.

### Evidence under `--verbose`

The evidence source is provenance, shown only under `-v/--verbose`, appended in
parentheses. For a `config` source the located file is a clickable link to its
repo-root-relative path:

```
cargo-nextest (configuration located at: .config/nextest.toml)
```

In a multi-runner list, `-v` also names the attributing package:

```
cargo-nextest (configuration located at: .config/nextest.toml, sniff package),
vitest (configuration located at: sniff-cli/vitest.config.ts, sniff-cli package)
```

A runner shared by many packages through one workspace-root config names no
single package (it is repo-wide). Use `--json` for the source in structured form.

## Prioritization

Each package collapses to the **strongest evidence tier present**, so a single
answer emerges wherever one exists. Sources rank strongest-first:

| Source | Meaning |
|--------|---------|
| `config` | A config file owned by the runner was found (e.g. `vitest.config.ts`, `.config/nextest.toml`). Disambiguates runners that share a manifest key. |
| `manifest` | The runner appears as an exact dependency key in a package manifest (e.g. `vitest`, `phpunit/phpunit`). Exact match only — a package merely *named* `jest-helper` does not count. |
| `ecosystem default` | The implicit built-in for the ecosystem (`cargo test`, `go test`, `node --test`, `unittest`, `mix test`, …). Reported **only when it is the sole signal** — a configured or declared runner supersedes it. |
| `convention` | Weakest. Inferred from test-file naming only, for stdlib runners with no dedicated config or manifest marker. |

So a Cargo crate with a nextest config reports `cargo-nextest`, not `cargo-nextest`
*and* `cargo test`. A plain crate with no explicit runner reports `cargo test`
(the default is then the only signal). A package can still yield more than one
runner when two markers of the same top tier are present (e.g. pytest + tox, both
config files).

### Workspace-root config (nextest)

Most config files are detected in each package's own directory. A few runners
keep a **single config at the workspace/repo root** that governs every member
rather than one config per package. nextest is the canonical case: a Cargo
workspace has one `.config/nextest.toml` (or `nextest.toml`) at the root, never
duplicated into each crate.

For these runners the config search extends from the package directory up to the
repo root, so scanning an individual member crate still surfaces nextest. Without
this, a workspace using nextest would report only the `cargo test` ecosystem
default, because no member crate carries a nextest marker of its own.

## Aggregation Scope

Per-package effective runners are collapsed across the scope, which depends on
where the command runs (mirrors the `package-manager` collapse rule):

```text
package        -> the runner(s) that package uses
package-area   -> union across packages in the area; uniform -> single answer,
                  else attributed list
repo root      -> union across all packages; uniform -> single answer, else list
```

Entries are deduplicated by `(runner, source)`: packages sharing one
workspace-root config collapse to a single entry naming all of them, while
per-package configs of the same runner stay distinct. In a non-monorepo, or a
monorepo with no discovered packages, detection runs at the resolved root
directory directly (no package attribution to carry).

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current one. Also fixes the scope to that directory. |
| `--csv` | Comma-separated on a single line. |
| `--list` | Newline-delimited, one per line. |
| `--md` | Markdown unordered list (`- item`). |
| `--json` | Emit the structured report (see below). |
| `-v/--verbose` | Add each runner's evidence source (and, in a list, its package). |

`--csv`/`--list`/`--md` select the **delimiter** (comma, newline, `- ` list);
styling is governed by the terminal and `--plain`, and detail by `-v`. They are
mutually exclusive. The default (no format flag) is a CSV.

- without `-v` — distinct runner names only.
- with `-v` — each item keeps the same styled provenance the default CSV shows
  (`configuration located at: …` with a clickable link, `declared as
  dependency: …`, plus the package in a multi-runner list), in the chosen
  delimiter. For example `--list -v`:

  ```
  cargo-nextest (configuration located at: .config/nextest.toml)
  vitest (declared as dependency: vitest, sniff-cli package)
  ```

Add `--plain` to strip styling for scripting; the config link then degrades to a
Markdown link.

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more runners were detected for the context. |
| `1` | No runner is declared for the context (text mode only; nothing on STDOUT). |

Under `--json` the command always exits `0` and emits a document even when no
runner is declared — the empty result is `{ "test_runners": [] }`.

## Examples

```bash
# The runner for the current context (single answer where one exists)
sniff repo test-runner
# → cargo-nextest

# Add evidence (and package, in a list) under --verbose
sniff repo test-runner -v
# → cargo-nextest (configuration located at: .config/nextest.toml)

# Names only, for piping
sniff repo test-runner --list
# → cargo-nextest

# Analyze a specific package directory
sniff repo test-runner -b sniff/lib

# Structured metadata for a script
sniff repo test-runner --json | jq -r '.test_runners[].binary'
# → cargo nextest run
```

## JSON Output (`--json`)

Always a `test_runners` array (length 0, 1, or N — matching the `TestRunner[]`
shape). Each entry carries the runner identity, the literal **run command**
(`binary`), the documentation `website`, the tagged `source`, and the in-scope
`packages` that attribute it:

```json
{
  "test_runners": [
    {
      "runner": "Nextest",
      "name": "cargo-nextest",
      "binary": "cargo nextest run",
      "website": "https://nexte.st/",
      "source": {
        "kind": "config",
        "filename": ".config/nextest.toml",
        "path": ".config/nextest.toml",
        "href": "file:///abs/path/.config/nextest.toml"
      },
      "packages": ["sniff-lib", "sniff-cli"]
    }
  ]
}
```

Fields:

- `binary` — the command you'd type to run the tests, ignoring task runners like
  `just` (e.g. `cargo nextest run`, `cargo test`, `go test ./...`, `vitest run`).
- `source` is tagged by `kind`: `config` (with `filename`, repo-relative `path`,
  and an absolute `file://` `href`), `manifest` (with `key`), `ecosystem_default`,
  or `convention`.
- `packages` — every in-scope package that resolves to this exact `(runner, source)`.

STDOUT carries the full JSON document; nothing is written to STDERR under `--json`.

## Bare `sniff repo --json`

The consolidated `sniff repo --json` aggregate collapses the repo-wide
`test_runner` fact across all packages to `string | string[] | null`, using the
same collapse rule. The focused `sniff repo test-runner --json` leaf documented
above keeps the richer per-runner evidence shape.

## Related Subcommands

| Subcommand | Output |
|------------|--------|
| [`structure`](./repo_structure.md) | Full hierarchical package overview |
| [`is-monorepo`](./repo_is-monorepo.md) | Monorepo label and predicate |
| [`packages`](./repo_packages.md) | Discovered package catalog |
