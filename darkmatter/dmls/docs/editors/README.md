# DMLS Editor Setup

The Darkmatter Language Server (`dmls`) speaks standard LSP 3.17 over stdio, so
any conforming client can drive it. These guides cover the four primary
targets, in support order:

| Editor | Guide | Integration path |
|--------|-------|------------------|
| VS Code | [vscode.md](./vscode.md) | Generic LSP config or thin extension |
| Zed | [zed.md](./zed.md) | Thin WASM extension launching native `dmls` |
| Neovim | [neovim.md](./neovim.md) | Built-in LSP client (0.10+) |
| Helix | [helix.md](./helix.md) | `languages.toml` registration |

## Prerequisites

1. Build and install the binary so it is on your `PATH`:

   ```bash
   cargo install --path darkmatter/dmls
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
marker and its configuration source (layered under LSP
`workspace/configuration`). A minimal file:

```toml
# .dmls.toml
[wiki]
enable = true
# wiki_root = "notes"        # narrow/redirect the wiki resolution universe
# path_style = "shortest"    # shortest | relative | root-relative

[schema]
strict = false

# Activate an extension baseline schema by name → path + globs.
# [schema.extensions.claudine]
# path = "docs/schemas/claudine.yaml"
# globs = [".claude/**", "prompts/**"]

[formatting]
# cleanup = "compact"        # cleanup variant used by textDocument/formatting
# fixed_width = 80
```

See [spec.md](../../../features/2026-07-04-dmls/spec.md) § Configuration for the
full key list.

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

- **Position encoding.** VS Code and Zed offer UTF-16 only; Neovim and Helix
  advertise UTF-8. `dmls` negotiates automatically and defaults to UTF-16 when
  negotiation is absent.
- **File watching on Linux + Neovim.** Client-side watching is limited on Linux;
  `dmls` keeps a server-side rescan fallback so unopened-file changes still
  update the graph.
- **File rename participation.** VS Code, Zed, and Helix support
  `workspace/willRenameFiles`; Neovim does not — link rewriting on file rename
  is unavailable there in v1.
- **Folding.** Neovim requests line-only folding; `dmls` emits line-safe ranges
  everywhere so this needs no per-editor tuning.
