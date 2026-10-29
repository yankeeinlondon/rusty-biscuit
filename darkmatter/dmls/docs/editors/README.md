---
kind: canonical-documentation
---
# DMLS Editor Setup

The Darkmatter Language Server (`dmls`) speaks standard LSP 3.17 over stdio, so
any conforming client can drive it. These guides cover the four primary
targets, in support order:

| Editor | Guide | Integration path |
|--------|-------|------------------|
| VS Code | [vscode.md](./vscode.md) | Shipped `vscode-dmls` extension (`just install-vscode-package`) or generic LSP bridge |
| Zed | [zed.md](./zed.md) | Shipped `zed-dmls` dev extension launching native `dmls` |
| Neovim | [neovim.md](./neovim.md) | Built-in LSP client (0.10+) |
| Helix | [helix.md](./helix.md) | `languages.toml` registration |

## Prerequisites

1. Build and install the binary so it is on your `PATH`:

   ```bash
   just install-dmls                          # from darkmatter/ (canonical)
   cargo install --path darkmatter/dmls      # equivalent, from the repo root
   # or, from a release archive, drop `dmls` (dmls.exe on Windows) on PATH
   ```

2. Confirm it launches:

   ```bash
   dmls --version
   ```

`dmls` writes logs to stderr or a log file only — **stdout is reserved for LSP
framing**, so never redirect stdout into your terminal while an editor is
attached.

## CLI flags

| Flag | Effect |
|------|--------|
| `--version` | Print the version and exit. |
| `--stdio` | No-op; accepted for clients that pass it by convention. |
| `--log-level <level>` | `error`/`warn`/`info`/`debug`/`trace` (default `info`). |
| `--log-file <path>` | Append logs to a file instead of stderr. |
| `--config <path>` | Explicit `.dmls.toml` path; otherwise discovered at the workspace root. |

## Root marker and configuration

`dmls` treats a `.dmls.toml` file at the workspace root as both an editor root
marker and its configuration source (overridden by the editor's
`workspace/didChangeConfiguration` settings). A minimal file:

```toml
# .dmls.toml
[wiki]
enable = true
# wiki_root = "notes"        # narrow/redirect the wiki resolution universe
# path_style = "shortest"    # shortest | relative | root-relative

[schema]
strict = false

# Warn on call arguments that may be null or whose type is unknown.
[expressions]
strict = false

# Activate an extension baseline schema by name → path + globs. The path
# names a document schema; trigger schemas need no entry here.
# [schema.extensions.team]
# path = "schemas/team-prompt.yaml"
# globs = ["prompts/**"]

# Workspace defaults for textDocument/formatting; the same rule keys as a
# document's `style.markdown`, which overrides them per document.
[formatting.markdown]
# list-item-padding = "none" # preserve | none | one
# fixed-width = 80           # 1..=1000, true (120), or false
```

Claudine's schemas need no extension entry. They are trigger schemas, which
`dmls` discovers in the document's schema roots: a document under `claudine/`
reaches `claudine/schemas/` automatically, and a document elsewhere needs the
complete seven-file trigger bundle in the repository root's `schemas/`, an absolute
`SCHEMAS_DIR`, or `~/schemas`. Copy `globals.yaml` and its companion
`globals.lifecycle.json` too for Claudine-specific globals and lifecycle
analysis. `dmls` reads these files from disk only and uses the trigger bundle
for diagnostics, completion, and hover. See
[DMLS Schema Support](../../../docs/topics/schemas/dmls-schema-support.md#claudine-documents).

The older `formatting.cleanup` (`compact` → `list-item-padding = "none"`,
`loose` → `"one"`, `default` → no override) and `formatting.fixed_width` keys
are still read and translated, with one migration warning per key each time
the configuration loads. A `[formatting]` table that mixes them with
`formatting.markdown`, or that has an unknown key or invalid value, is ignored
and the remaining settings apply. Each finding is written to the server log
and shown in the editor as one `window/showMessage` notification, prefixed
`dmls:` and naming its source and key: a migration is a Warning, a rejected
setting an Error. Findings are shown when the server starts and again after
every `workspace/didChangeConfiguration`, never per formatting request, so a
legacy key keeps warning once per configuration change until it is migrated.

See [DMLS Configuration](../../../docs/lsp/configuration.md) for the full key
list.

## Formatting

"Format Document" (`textDocument/formatting`) runs the same clean transaction
as `md clean`, on the editor's current buffer rather than the file on disk:

- frontmatter YAML repair, the same repairs `md clean` applies, unless the
  document sets `style.markdown.frontmatter: false`;
- body formatting under the
  [`style.markdown` rules](../../../docs/topics/markdown-formatting.md). For each
  rule, the document's own `style.markdown` wins over
  `[formatting.markdown]`, which wins over the built-in default.

The result is one whole-document edit, or no edit when the buffer is already
clean. Formatting never composes the document, runs shell commands, or reads
the network. Only whole-document formatting is offered; there is no range or
on-type formatting.

Formatting can decline. When cleanup cannot format the buffer without changing
its meaning (usually a malformed directive such as a `::block` with no
`::end-block`), or the frontmatter cannot be read even after repair, `dmls`
returns no edits and shows one Warning through `window/showMessage`. The
warning names the line and column in the editor's position encoding, so a
format-on-save that changed nothing is visible rather than silent:

```text
cleanup left the document unchanged because of a malformed directive region
at line 4, column 1: `::block when="x"` is never closed by an `::end-block`
```

## Filetype / document selector

Register `dmls` for the `markdown` language. Recommended file globs for
watchers where a client asks for them:

```
**/*.{md,markdown,mdown,mkdn}
```

## Client capability notes

`dmls` gates optional behavior on a per-client `ClientProfile` (built at
`initialize`), so each editor gets the richest surface it actually supports and
degrades safely otherwise. The full capability matrix and its sources are in
[r7-editor-capability-matrix.md](../../design/research/r7-editor-capability-matrix.md).
Highlights that affect setup:

- **Position encoding.** VS Code and Zed offer UTF-16 only. Helix advertises
  UTF-8 first; Neovim 0.10 may offer only UTF-16 while newer releases offer
  UTF-8 first. `dmls` negotiates automatically and defaults to UTF-16 when
  negotiation is absent.
- **File watching on Linux + Neovim.** Client-side watching is limited on Linux;
  `dmls` keeps a server-side rescan fallback so unopened-file changes still
  update the graph.
- **File rename participation.** VS Code, Zed, and Helix support
  `workspace/willRenameFiles`; Neovim does not — link rewriting on file rename
  is unavailable there in v1.
- **Folding.** Neovim requests line-only folding; `dmls` emits line-safe ranges
  everywhere so this needs no per-editor tuning.

## Troubleshooting

**`dmls` runs in a terminal but the editor can't find it.** GUI-launched
editors (Dock, Spotlight, Finder) do not inherit your shell's `PATH`. Configure
the absolute binary path instead — usually `~/.cargo/bin/dmls`. Both shipped
extensions expose a setting for this (`dmls.server.path` in VS Code;
`binary.path` in Zed's LSP settings).

**Server list is empty, or servers sit in a "waiting" state after editor
restart (Zed).** Zed captures each project's environment by running your login
shell headlessly (`$SHELL -l -i -c … --printenv` with pipes, no TTY). If your
shell init blocks in that context — e.g. completion loaders that spawn CLIs, or
anything that probes the terminal — Zed never finishes project setup and starts
**no** language servers for the project (not just `dmls`). Check for wedged
capture shells with `pgrep -fl printenv`; long-lived entries mean your shell
init is the problem. Guard interactive-only init (completions, prompt tooling)
behind a TTY test such as `[ -t 0 ] || [ -t 1 ]`.

**Where the logs are.** `dmls` logs to stderr (or `--log-file <path>`); stdout
is LSP framing only.

- VS Code: **Output → "Darkmatter Language Server"**.
- Zed: command palette → `dev: open language server logs`; editor-side errors
  land in `~/Library/Logs/Zed/Zed.log` (macOS).
- Any editor: add `--log-level debug --log-file /tmp/dmls.log` to the server
  arguments for a standalone trace.

**Probe the server by hand.** `dmls --version` proves the binary runs;
`dmls --bench-index <dir>` proves it can index a workspace (an index store
fill, then a warm restart answering references, rename, and workspace
symbols) without any editor involved.
