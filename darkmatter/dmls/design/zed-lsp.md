---
prompt: |-
  We are creating a LSP for Darkmatter (and Claudine) using Rust and we targeting a few editors of which [Zed](https://zed.dev/docs/) is one of the editors we very much want to use this LSP in.

  Zed, unlike many editors, has a some special requirements for LSP authors. The short version is it must be compiled to a WASM target but that is too simplified to be helpful.

  Your task is to research what the requirements are for developing a LSP to run in Zed.

  - what are the technical requirements?
  - what is the process to getting the LSP onto the approved list so that it is visible as an available option in the editors config
  - how can you run the LSP prior to it being approved into Zed?

  We are planning on using Rust for our LSP and more specifically we're looking at basing our implementation on IWES/IWE. You can read more in the document @darkmatter/docs/dmls/design/design-strategy.md .

  Based on this type of architecture, how easily can we target Zed's LSP requirements? Does IWES/IWE already have an LSP designed for Zed?
last_updated: 2026-07-06
hash: c7ee023e8621782f-7078c657ddcc86e4
---
# Zed LSP Requirements for DMLS

## Executive Summary

Zed does **not** require the language server itself to be compiled to WebAssembly. The WebAssembly requirement applies to the **Zed extension code**: the small Rust extension that tells Zed how to install, locate, configure, and launch the language server. The language server process is normally a native executable that speaks LSP over stdio.

That distinction matters for DMLS. A Rust `dmls` binary based on IWES/IWE can remain a normal cross-platform native LSP server. Zed support should be a separate, small Zed extension crate compiled as `cdylib` to WebAssembly using `zed_extension_api`. The extension registers DMLS as a language server for Markdown, optionally registers Darkmatter-specific language metadata later, and returns a `zed::Command` that launches the native `dmls` executable.

This makes Zed support very achievable for the IWES-based DMLS architecture described in `darkmatter/dmls/design/design-strategy.md`. IWES already has an LSP server, `iwes`, and IWE already has an approved Zed extension. That extension is the best reference implementation for DMLS.

## Sources Checked

Primary sources:

- [Zed: Developing Extensions](https://zed.dev/docs/extensions/developing-extensions)
- [Zed: Language Extensions](https://zed.dev/docs/extensions/languages)
- [Zed: Configuring Languages](https://zed.dev/docs/configuring-languages)
- [Zed Rust Extension API docs](https://docs.rs/zed_extension_api/latest/zed_extension_api/)
- [Zed extensions registry repository](https://github.com/zed-industries/extensions)
- [IWE repository](https://github.com/iwe-org/iwe)
- [IWE Zed extension](https://github.com/iwe-org/zed-iwe)
- [IWE Zed editor docs](https://iwe.md/docs/editors/zed/)
- [IWE extension registry page](https://zed.dev/extensions/iwe)

Local design inputs:

- `darkmatter/dmls/design/design-strategy.md`
- `darkmatter/dmls/design/extending-iwes-lsp.md`
- `darkmatter/dmls/README.md`

## Technical Requirements

### Extension Shape

A Zed extension is a Git repository with an `extension.toml` manifest. For an extension that launches a language server, it also needs Rust code compiled to WebAssembly.

Minimum structure:

```text
zed-dmls/
  extension.toml
  Cargo.toml
  src/
    lib.rs
```

If DMLS defines a new Zed language such as `Darkmatter`, the extension also needs language metadata:

```text
zed-dmls/
  languages/
    darkmatter/
      config.toml
      highlights.scm
```

If DMLS only targets normal Markdown files initially, the extension can register the language server against Zed's existing `Markdown` language, as the IWE extension does.

### `extension.toml`

Zed's current docs show language servers declared like this:

```toml
id = "darkmatter"
name = "Darkmatter"
version = "0.0.1"
schema_version = 1
authors = ["..."]
description = "Darkmatter and Claudine language server support"
repository = "https://github.com/..."

[language_servers.dmls]
name = "DMLS"
languages = ["Markdown"]
```

The IWE extension currently uses a similar older-looking form:

```toml
[language_servers.iwe]
name = "IWE"
language = "Markdown"
```

For new work, prefer the current documented `languages = [...]` form unless testing shows Zed still requires the older singular key for registry compatibility.

### Rust Extension Crate

The extension crate must be a `cdylib` and depend on `zed_extension_api`:

```toml
[package]
name = "zed-dmls"
version = "0.0.1"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
zed_extension_api = "0.7.0"
```

The extension implements `zed::Extension` and registers itself:

```rust
use zed_extension_api as zed;

struct DmlsExtension {
    cached_binary_path: Option<String>,
}

impl zed::Extension for DmlsExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        Ok(zed::Command {
            command: self.resolve_dmls(language_server_id, worktree)?,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(DmlsExtension);
```

The extension is WebAssembly. The `dmls` server it launches is native.

### WebAssembly Constraints

The Zed extension code runs in a Wasm environment. Zed's docs call out a few practical constraints:

- `cfg` directives may not behave the way ordinary native Rust code expects.
- `std::env::var` should not be used as the normal way to read user environment.
- Use `zed_extension_api::current_platform()` for OS/architecture detection.
- Use `Worktree` APIs such as `worktree.which(...)`, `worktree.root_path()`, `worktree.shell_env()`, and `worktree.read_text_file(...)`.
- Debug output from `println!` and `dbg!` goes to the Zed process; run Zed with `zed --foreground` for more useful logs.

This means the Zed extension should stay small. It should not contain DMLS indexing, parsing, schema, or IWES graph logic.

### Language Server Binary Requirements

For registry publication, Zed's current extension rules say language/debugger/MCP extensions must **not ship the language server binary inside the extension**. The extension should either:

- find a user-installed binary in the environment, or
- download the language server from a release source using the Zed extension API.

The IWE extension does both:

1. It checks `worktree.which("iwes")`.
2. If not found, it calls `zed::latest_github_release("iwe-org/iwe", ...)`.
3. It picks an asset for the current platform.
4. It downloads and unpacks the asset.
5. It returns the downloaded `iwes` path as the LSP command.

DMLS should copy that model.

Expected DMLS release assets:

```text
dmls-vX.Y.Z-universal-apple-darwin.tar.gz
dmls-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
dmls-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz
dmls-vX.Y.Z-x86_64-pc-windows-msvc.zip
```

Because this monorepo targets macOS, Windows, and Linux, DMLS should plan for all three from the start. IWE's Zed docs currently list macOS and Linux as supported, with Windows planned, but the current `zed-iwe` source already contains a Windows x86_64 case. DMLS should close that gap rather than inherit it.

### LSP Protocol Requirements

Zed is a normal LSP client once the server is launched. DMLS should speak standard LSP over stdio.

For the IWES-based plan, the current local decision to use `lsp-server` and `lsp-types` remains compatible with Zed. Zed does not require `tower-lsp`.

Important protocol details for Zed:

- Return normal `ServerCapabilities` from `initialize`.
- Use `textDocument/publishDiagnostics` for diagnostics.
- Use standard LSP methods for completion, hover, definition, references, rename, formatting, folding ranges, document symbols, workspace symbols, inlay hints, and code actions.
- Accept `initializationOptions` and workspace configuration where useful.
- Use UTF-16 LSP positions unless client capability negotiation explicitly supports another position encoding.
- Keep stdio reserved for LSP framing. Logs should go to stderr or a log file, not stdout.

### Zed Language Integration

Zed language support has two layers:

1. Tree-sitter for syntax highlighting and structural editor features.
2. LSP for semantic editor features.

If DMLS targets `Markdown`, Zed already has Markdown language support, so the extension can initially be language-server-only.

If we want a distinct `Darkmatter` language mode, the extension must provide:

```toml
name = "Darkmatter"
grammar = "markdown"
path_suffixes = ["md", "markdown"]
line_comments = []
```

However, a distinct language mode for `.md` files may compete with Zed's built-in Markdown handling. The safer first path is:

- register DMLS for `Markdown`;
- let users enable/disable/prioritize DMLS through Zed's normal language-server settings;
- add a distinct `Darkmatter` language only if we need separate file associations, syntax queries, semantic token defaults, or behavior that should not apply to all Markdown.

Zed lets users prioritize registered language servers:

```json
{
  "languages": {
    "Markdown": {
      "language_servers": ["dmls", "..."]
    }
  }
}
```

It also lets users disable a server with a `!` prefix:

```json
{
  "languages": {
    "Markdown": {
      "language_servers": ["dmls", "!markdown-oxide", "..."]
    }
  }
}
```

### User Configuration

Zed supports LSP configuration through `settings.json`.

Initialization options:

```json
{
  "lsp": {
    "dmls": {
      "initialization_options": {
        "darkmatter": {
          "strictStyle": true
        },
        "claudine": {
          "enabled": true
        }
      }
    }
  }
}
```

Workspace configuration:

```json
{
  "lsp": {
    "dmls": {
      "settings": {
        "darkmatter": {
          "schemaMode": "workspace"
        }
      }
    }
  }
}
```

Binary override for local development or manual installation:

```json
{
  "lsp": {
    "dmls": {
      "binary": {
        "path": "/absolute/path/to/dmls",
        "arguments": [],
        "env": {
          "RUST_LOG": "dmls=debug"
        }
      }
    }
  }
}
```

Zed's docs show this binary override pattern for registered language servers. The key point is that the server id must already be registered by a built-in language integration or installed extension. Zed does not currently work like Neovim or Helix where a user can freely define an arbitrary new LSP command from editor config alone.

## Publishing and Approval Process

To make DMLS visible in Zed's extension UI:

1. Create a public Zed extension repository with `extension.toml`, Rust extension code, and a valid extension license.
2. Test it locally as a dev extension.
3. Fork `zed-industries/extensions`.
4. Add the extension repository as an HTTPS Git submodule under `extensions/{extension-id}`.
5. Add an entry to the top-level `extensions.toml`:

   ```toml
   [darkmatter]
   submodule = "extensions/darkmatter"
   version = "0.0.1"
   ```

   If the Zed extension lives in a subdirectory of the repository, include `path`:

   ```toml
   [darkmatter]
   submodule = "extensions/darkmatter"
   path = "packages/zed"
   version = "0.0.1"
   ```

6. Run `pnpm sort-extensions` in the `zed-industries/extensions` checkout.
7. Open a pull request.
8. After merge, Zed packages and publishes the extension to the extension registry.

Current registry review constraints that matter for DMLS:

- The extension id must be unique and effectively permanent.
- Extension ids and names must not include `zed`, `Zed`, or `extension`.
- The extension repository must include an accepted license for the extension code.
- The extension should include only resources it needs.
- The extension must not attempt to read or modify environment outside the Zed-provided extension environment.
- A language-server extension must find or download the server, not bundle the server binary inside the extension.
- Untested or non-functioning extension submissions may be closed without detailed review.

Accepted extension-code licenses currently include Apache 2.0, BSD 2-Clause, BSD 3-Clause, CC BY 4.0, GPLv3, LGPLv3, MIT, Unlicense, and zlib.

The license rule applies to the Zed extension code. It does not force the language server binary or the whole monorepo to use the same license.

## Running Before Approval

Use a dev extension.

Process:

1. Build or install a local `dmls` binary.
2. Create a local `zed-dmls` extension repository.
3. Implement `language_server_command` so it first checks either:
    - a local absolute path during development,
    - `worktree.which("dmls")`,
    - or an environment/configured path.

4. In Zed, open the Extensions page.
5. Run `Install Dev Extension` or the `zed: install dev extension` action.
6. Select the local extension directory.
7. Open a Markdown file.
8. Check `zed: open log` and LSP logs if it does not start.
9. For more verbose extension logs, launch Zed from a terminal with:

```sh
zed --foreground
```

A development-only `language_server_command` can simply return a local binary path:

```rust
fn language_server_command(
    &mut self,
    _language_server_id: &zed::LanguageServerId,
    worktree: &zed::Worktree,
) -> zed::Result<zed::Command> {
    let command = worktree
        .which("dmls")
        .ok_or_else(|| "dmls was not found on PATH".to_string())?;

    Ok(zed::Command {
        command,
        args: vec![],
        env: Default::default(),
    })
}
```

Once the extension is registered, a user can also override the binary through Zed settings:

```json
{
  "lsp": {
    "dmls": {
      "binary": {
        "path": "/Users/ken/.cargo/bin/dmls",
        "arguments": []
      }
    }
  }
}
```

For very early testing, another pragmatic option is to install the existing IWE Zed extension and override its `iwe` binary path to point at a DMLS-compatible binary. That is useful only as a quick smoke test. It is not the right long-term path because logs, settings keys, server id, extension metadata, and user-facing naming would all say `iwe`.

## IWES/IWE and Zed

IWES/IWE already has Zed support.

Current facts:

- IWE ships a Rust LSP server named `iwes`.
- IWE documents support for VS Code, Neovim, Zed, Helix, and other LSP clients.
- The Zed extension is published as `IWE`.
- The extension registry page describes it as "Markdown files graph navigation and transformation."
- The extension provides language-server support.
- IWE's Zed docs say the extension uses a system `iwes` binary when available and otherwise downloads from GitHub releases.
- The `zed-iwe` source implements the expected Zed pattern: a Wasm extension that locates or downloads a native `iwes` binary and returns it as the language-server command.

That means DMLS is not breaking new ground for Zed. The path is proven by the upstream project we already plan to study.

## Fit With the IWES-Based DMLS Architecture

The IWES-based DMLS strategy fits Zed well.

The design strategy recommends:

- `lsp-server` plus `lsp-types`;
- full document sync initially;
- an IWES-derived router and graph substrate;
- a Darkmatter semantic overlay;
- a single native `dmls` binary.

None of that conflicts with Zed. Zed only needs a registered extension that can start the server.

The main DMLS work remains editor-independent:

- stable LSP capability negotiation;
- source maps from Darkmatter/IWES spans to UTF-16 LSP ranges;
- diagnostics for frontmatter, schemas, style, compose directives, and links;
- safe passive analysis that does not execute shell expansion or fetch remote content during ordinary editor requests;
- graph indexing and invalidation;
- cross-platform binary releases.

The Zed-specific work should stay thin:

- extension manifest;
- `zed_extension_api` wrapper;
- platform-specific release asset selection;
- optional `initialization_options` and workspace configuration forwarding;
- semantic token styling defaults if DMLS emits custom token types;
- local dev-extension workflow.

## Architecture Recommendation

Use a two-repository or two-package split:

```text
darkmatter/dmls/
  Native Rust LSP server.
  Built and tested like the rest of rusty-biscuit.
  Published as cross-platform release artifacts.

zed-dmls/
  Small Zed extension.
  Compiled to Wasm.
  Finds or downloads dmls.
  Registers dmls for Markdown.
```

If we keep the Zed extension inside this monorepo, it should still be packaged as a small extension crate with its own accepted license file at the extension path if submitted to `zed-industries/extensions` using a `path` entry.

Recommended DMLS launch strategy:

1. Check for a user/system `dmls` on `PATH`.
2. Check whether Zed settings provide a binary override.
3. Otherwise download the latest stable release asset for the current platform.
4. Cache by version.
5. Return the binary path as `zed::Command`.

Recommended first Zed target:

```toml
[language_servers.dmls]
name = "DMLS"
languages = ["Markdown"]
```

Add a distinct `Darkmatter` Zed language only after the Markdown integration works and we know the benefit outweighs file-association friction.

## Risks and Gotchas

- The common shorthand "Zed requires the LSP to be Wasm" is misleading. The extension is Wasm; the LSP is native.
- Zed users cannot cleanly configure an arbitrary new LSP from settings alone. A server must be registered by core Zed or an extension before binary overrides and priority settings are useful.
- Zed extensions that launch language servers require Rust via `rustup` for local development.
- The Zed extension code is sandboxed enough that normal native Rust assumptions around environment and platform detection should be avoided.
- Published extensions must not bundle native language-server binaries directly.
- If DMLS applies to `Markdown`, users may have multiple Markdown language servers. We need clear docs showing how to prioritize DMLS.
- If DMLS emits semantic tokens, Zed has semantic tokens disabled by default unless users enable `"semantic_tokens": "combined"` or `"full"`.
- If DMLS does any shell expansion, remote fetch, or compose execution in passive LSP requests, that will be a security and trust problem. Passive analysis must be non-executing by design.

## Answer to the Core Feasibility Question

Targeting Zed should be straightforward for DMLS if we keep the boundary clean:

- `dmls` remains a normal Rust native LSP server.
- `zed-dmls` is a small Wasm extension that launches `dmls`.
- Release automation produces native binaries for macOS, Linux, and Windows.
- The Zed extension follows the existing IWE pattern.

IWES/IWE already has an LSP designed to work in Zed, and it already has an approved Zed extension. DMLS can use that as the reference implementation, but should not try to reuse the IWE extension directly except for early smoke testing. The durable solution is a DMLS-specific Zed extension that registers its own server id, settings keys, release assets, and user-facing metadata.
