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

Prints a styled table of distinct runners and their evidence, one row per
runner. For a Cargo workspace that configures nextest at the workspace root:

```
┌──────────────────┬──────────────────────┐
│ Runner           │ Source               │
├──────────────────┼──────────────────────┤
│ cargo test       │ ecosystem default    │
│ cargo-nextest    │ config               │
│                  │ .config/nextest.toml │
└──────────────────┴──────────────────────┘

distinct runners (across all packages)
```

When exactly one runner is in scope, a focused single-runner report is printed
instead of the table. When no runner is declared for the context, nothing is
written to STDOUT (a dim hint goes to STDERR unless `--plain`) and the process
exits `1`.

## Evidence Sources

Each runner carries the strongest signal that attributed it. Sources are ordered
by signal strength (strongest first):

| Source | Meaning |
|--------|---------|
| `config` | A config file owned by the runner was found (e.g. `vitest.config.ts`, `.config/nextest.toml`). Disambiguates runners that share a manifest key. |
| `manifest` | The runner appears as an exact dependency key in a package manifest (e.g. `vitest`, `phpunit/phpunit`). Exact match only — a package merely *named* `jest-helper` does not count. |
| `ecosystem default` | The runner is the implicit built-in for its ecosystem (`cargo test`, `go test`, `node --test`, `unittest`, `mix test`, …). Reported even when no explicit runner is configured, so consumers can tell "explicitly configured" from "implicitly available". |
| `convention` | Weakest. Inferred from test-file naming only, emitted for stdlib runners (`unittest`, Minitest, `node --test`) with no dedicated config or manifest marker. |

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

Per-package runners are collapsed into a distinct set whose breadth depends on
where the command runs (mirrors the `package-manager` collapse rule):

```text
package        -> the runners that package declares
package-area   -> union across packages in the area; uniform -> singular,
                  else unique list
repo root      -> union across all packages; uniform -> singular, else list
```

In a non-monorepo, or a monorepo with no discovered packages, detection runs at
the resolved root directory directly. The scope label is shown beneath the table
(`across all packages`, `within this package-area`, …).

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current one. Also fixes the scope to that directory. |
| `--csv` | Render runners comma-separated on a single line. |
| `--list` | Render runners newline-delimited, one per line. |
| `--md` | Render runners as a Markdown unordered list (`- name`). |
| `--json` | Emit the structured report (see below). |
| `-v/--verbose` | Add evidence detail to the styled text report. |

`--csv`, `--list`, and `--md` are mutually exclusive and emit names only (no
evidence column). Use `--json` when the evidence source matters to a script.

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | One or more runners were detected for the context. |
| `1` | No runner is declared for the context (text mode only; nothing on STDOUT). |

Under `--json` the command always exits `0` and emits a document even when no
runner is declared — the empty result is `{ "test_runner": null }`.

## Examples

```bash
# Styled table for the current context
sniff repo test-runner

# Just the names, for piping
sniff repo test-runner --list
# → cargo test
#   cargo-nextest

# Single comma-separated line
sniff repo test-runner --csv
# → cargo test, cargo-nextest

# Analyze a specific package directory
sniff repo test-runner -b sniff/lib

# Structured evidence for a script
sniff repo test-runner --json | jq '.test_runners[].runner'
```

## JSON Output (`--json`)

A singular result is emitted as a single object under `test_runner`; multiple
results as an array under `test_runners`. Each entry has the runner variant and
a tagged `source`:

```json
{
  "test_runners": [
    {
      "runner": "CargoTest",
      "source": { "kind": "ecosystem_default" }
    },
    {
      "runner": "Nextest",
      "source": { "kind": "config", "filename": ".config/nextest.toml" }
    }
  ]
}
```

The `source` object is tagged by `kind`: `config` (with `filename`), `manifest`
(with `key`), `ecosystem_default`, or `convention`. STDOUT carries the full JSON
document; nothing is written to STDERR under `--json`.

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
