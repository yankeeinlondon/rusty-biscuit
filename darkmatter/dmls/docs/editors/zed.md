# Zed

Zed cannot register an arbitrary stdio LSP from settings alone — it needs a thin
extension. The extension is a small WASM module (`zed_extension_api`) whose only
job is to launch the **native** `dmls` binary; it contains no language logic.
Position encoding is UTF-16 only.

The extension lives in a separate repo, `zed-dmls`. A ready-to-extract scaffold
ships in [`../../zed-dmls/`](../../zed-dmls/) — copy it into its own repository to
publish. See [zed-dmls/README.md](../../zed-dmls/README.md) for the binary
resolution order (PATH → settings `binary.path` → GitHub release download) and
the packaging steps.

## Install as a dev extension

1. Build/install `dmls` so it is on your `PATH` (or configure `binary.path` in
   the extension settings once wired).
2. In Zed: **command palette → `zed: install dev extension`**, and select the
   `zed-dmls` directory.
3. Open a Markdown file; Zed launches `dmls` for it.

## Minimal extension shape

`extension.toml`:

```toml
id = "dmls"
name = "DMLS"
version = "0.0.1"
schema_version = 1

[language_servers.dmls]
name = "Darkmatter Language Server"
languages = ["Markdown"]
```

`src/lib.rs`:

```rust
use zed_extension_api as zed;

struct DmlsExtension;

impl zed::Extension for DmlsExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        Ok(zed::Command {
            command: "dmls".into(),
            args: Vec::new(),
            env: Default::default(),
        })
    }
}

zed::register_extension!(DmlsExtension);
```

## Client quirks that affect `dmls`

- **Never resolve `completionItem.textEdit`.** Zed intentionally does not resolve
  `textEdit` for performance. `dmls` always emits eager `textEdit`s, so
  completion is Zed-safe by construction.
- **File operations.** Zed supports will/did rename, so
  `workspace/willRenameFiles` link rewriting works. Change-annotation support is
  not advertised, so `dmls` relies on explicit code-action titles.
- **Full folding.** Zed supports non-line-only folding and `collapsedText`.

## What you get

Navigation, diagnostics, completion, hover, frontmatter schema intelligence,
directive/transclusion/interpolation intelligence, read-only shell-policy hover,
heading + file rename, the v1 code-action set, and whole-document formatting.
