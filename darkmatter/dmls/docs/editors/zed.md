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

1. Run `just install-dmls` from `darkmatter/`. It installs the native binary
   and, when Zed's data directory exists on the host, stages the extension
   (including the bundled `extension.wasm`) to a stable per-user directory
   outside the checkout and points Zed's `extensions/installed/dmls` link at
   it. A dangling link left behind by a removed worktree is repaired the same
   way. The doctor then runs and reports the result.
2. Restart Zed if it was running, then open a Markdown file. Zed launches
   `dmls` for it. Check the language-server status menu (bottom bar) — `dmls`
   should be listed and green.

The recipe warns, without failing, when the registration could not be made
automatically (no Zed data directory, or an `installed/dmls` entry that is a
real directory rather than a link); the output then names the stable directory
to select via **command palette → `zed: install dev extension`**. Never select
the sibling `vscode-dmls` directory or a checkout/worktree directory.

Re-running the recipe atomically refreshes the stable copy, so removing a
source worktree cannot break the installed extension. After extension-source
changes, rebuild the bundled `extension.wasm` and re-run the recipe; restart
the language server or Zed after a native `dmls` update.

## Troubleshooting

Run `just zed-doctor` for a bounded check of the binary, registration target,
manifest, and recent Zed log evidence. In particular:

- `No extension manifest found for extension dmls` during startup means the
  existing dev-extension registration points to a removed path.
- `Failed to install dev extension: No extension manifest found for extension
  vscode-dmls` during installation means the sibling VS Code extension was
  selected. The extension id in this message comes from the selected folder;
  select the stable directory printed by `just install-dmls`.

The Zed log is consulted only when the registration is broken, and only for
lines about `dmls` or `vscode-dmls`.

Zed Preview and custom installations can supply `--staging-dir`,
`--zed-data-dir`, and `--zed-log` directly to `zed-dmls stage` or
`zed-dmls doctor`. Add `--plain` for deterministic automation output.

If Zed starts with **no** language servers for the project at all (dmls *and*
codebook/YAML missing or stuck "waiting"), that is usually not the extension —
see the [shell env-capture note in Troubleshooting](README.md#troubleshooting).

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

## Semantic tokens (de-emphasize Darkmatter machinery)

`dmls` emits semantic tokens so a theme can dim interpolations (`{{ … }}`),
directive lines (`::file`, `::block`, …), and wiki-link brackets while letting
wiki inner text read link-like. Zed ships semantic tokens **disabled by
default**, so this is opt-in per language in `settings.json`:

```json
{
  "languages": {
    "Markdown": { "semantic_tokens": "combined" }
  }
}
```

`"combined"` layers DMLS tokens over Zed's Markdown grammar highlighting;
`"full"` uses the server tokens alone. Colors come from the **active theme**, not
the extension — see [zed-dmls/README.md](../../zed-dmls/README.md#semantic-token-styling)
for the recommended `experimental.theme_overrides` recipe (muted machinery vs.
link-like wiki text). Server-side, semantic tokens are on by default and can be
switched off with `[semantic_tokens] enable = false` in `.dmls.toml`.

### Smoke example

In a Markdown file, with the opt-in above:

```md
Deploy {{ project.name }} to {{ env.STAGE }}.

::file ./intro.md

See [[architecture#overview]] for the design.
```

Each `{{ … }}` span, the `::file` keyword and its `./intro.md` target, and the
`[[ ]]` brackets/`#` separator should render muted; `architecture` and `overview`
should read link-like. A `{{{ literal }}}` carries the extra `inert` modifier
for themes that fade it further.

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
heading + file rename, the v1 code-action set, whole-document formatting, and
semantic tokens (opt-in, see above).
