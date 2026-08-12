# vscode-dmls

VS Code extension that launches the native Darkmatter Language Server (`dmls`)
for Markdown files. It contains **no language logic** — it is a thin client
(`vscode-languageclient`) that starts the native binary over stdio. All
intelligence lives in `dmls`. Mirrors the Zed shim in the sibling `zed-dmls`
directory.

> **This directory is a scaffold**, versioned inside the monorepo for
> convenience. It is a Node package, not a Cargo crate, so it is not part of the
> Cargo workspace and is not built by the `just` recipes.

## Prerequisites

Install `dmls` so it is on your `PATH`:

```bash
cargo install --path .. --force   # from this directory; installs ../ (the dmls crate)
dmls --version
```

## Install the extension

Get the client dependency once:

```bash
npm install
```

### Quick test (Extension Development Host)

1. Open this folder in VS Code (`code .`).
2. Press **F5** (Run → "Run DMLS Extension"). A second VS Code window opens with
   the extension loaded.
3. Open any Markdown file there; `dmls` attaches.

### Permanent install (VSIX)

```bash
npx @vscode/vsce package --allow-missing-repository
code --install-extension dmls-0.0.1.vsix
```

Reload VS Code (`Developer: Reload Window`) and open a Markdown file.

## Configuration

- `dmls.server.path` — path to the `dmls` binary (default `dmls` on `PATH`). Set
  an absolute path (e.g. `~/.cargo/bin/dmls`) if VS Code cannot find it.
- `dmls.server.args` — extra arguments passed to `dmls`.

Server logs appear in **Output → "Darkmatter Language Server"**. Standard LSP
`.dmls.toml` workspace configuration applies; see the DMLS VS Code editor guide
(`darkmatter/dmls/docs/editors/vscode.md`) in the monorepo.
