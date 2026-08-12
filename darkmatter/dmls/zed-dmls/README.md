# zed-dmls

Zed extension that launches the native [Darkmatter Language Server](https://github.com/)
(`dmls`) for Markdown files. The extension contains **no language logic** — it is
a thin WASM shim (`zed_extension_api`) that resolves and starts the native
binary. All intelligence lives in `dmls`.

> **This directory is a scaffold.** It lives inside the rusty-biscuit monorepo
> for versioning convenience but is **excluded from the Cargo workspace**
> (per AD-7 in the DMLS design). To publish, copy this directory into its own
> public `zed-dmls` repository, add a top-level `LICENSE`, and submit to
> `zed-industries/extensions`. It targets `wasm32-wasip2` and depends on
> `zed_extension_api`, so it is intentionally not built by the monorepo's
> `just` recipes.

## Layout

```
zed-dmls/
├── extension.toml     # id, name, version, [language_servers.dmls]
├── Cargo.toml         # cdylib crate on zed_extension_api
└── src/lib.rs         # binary resolution + language_server_command
```

## Binary resolution order

`language_server_command` resolves the `dmls` binary in this order (the proven
IWE pattern):

1. **PATH** — `worktree.which("dmls")`.
2. **Settings override** — `binary.path` (and optional `binary.arguments`) from
   the extension's LSP settings.
3. **GitHub release download** — the platform-matched asset from the latest
   `dmls` release, cached by version so it downloads once per upgrade.

Per-platform release asset names (see the release recipe `just dist` in
`darkmatter/justfile`):

| Platform | Asset |
|----------|-------|
| macOS (universal) | `dmls-<version>-macos-universal.tar.gz` |
| Linux x86_64 | `dmls-<version>-linux-x86_64.tar.gz` |
| Linux aarch64 | `dmls-<version>-linux-aarch64.tar.gz` |
| Windows x86_64 | `dmls-<version>-windows-x86_64.zip` |

## Semantic token styling

`dmls` emits LSP semantic tokens that classify Darkmatter machinery so a theme
can de-emphasize it: interpolations (`macro.interpolation`), directive keywords
and closers (`macro.directive`, `macro.directive.closer`), and wiki-link frames
(`macro.wiki`) read as muted machinery, while wiki inner text (`string.wiki`)
reads link-like. Directive targets and option values are `string.directive`;
option keys are `property.directive`.

Zed keeps semantic tokens **disabled by default**; a user must opt in per
language in `settings.json`:

```json
{
  "languages": {
    "Markdown": { "semantic_tokens": "combined" }
  }
}
```

Zed resolves semantic-token colors through the **active theme**, not through a
language-server extension — this Markdown-launcher extension has no API to inject
token colors (that would require registering a distinct `Darkmatter` language,
deferred by design). The recommended defaults therefore ship here as a copyable
theme override. Add to `settings.json` and adjust the palette to your theme
(muted machinery vs. link-like wiki text):

```json
{
  "experimental.theme_overrides": {
    "syntax": {
      "comment": { "color": "#7d8590" },
      "link_uri": { "color": "#539bf5" }
    }
  }
}
```

Zed maps the custom modifiers onto its highlight names; where a theme does not
distinguish a Darkmatter modifier, the token falls back to its standard base
type (`macro`, `string`, `property`) and still renders sensibly. The
`interpolation` / `directive` / `closer` / `wiki` distinctions are the targeting
surface a richer theme can address individually.

## Develop

```
# install as a dev extension
# Zed: command palette → "zed: install dev extension" → select this directory
```

Update `REPO` in `src/lib.rs` and the asset-name mapping to match the repository
that publishes `dmls` release archives before shipping.
