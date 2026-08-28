# biscuit-icon Dependencies

Notable crates used by the `biscuit-icon` library and the `icon` CLI, and why.

## Library (`biscuit-icon`)

| Crate | Why |
|-------|-----|
| `renderable` (path) | Multi-target render tree; `Icon` implements `TreeRenderable` to emit inline SVG for the browser/markdown targets, and `TerminalRenderable` for the terminal ladder. |
| `biscuit-terminal` (path) | Terminal rendering (glyph → image → text ladder). The `image` feature gates both the direct `resvg` dependency and the `biscuit-visualized` dependency (which brings in `resvg` for Mermaid/graph rendering). When `image` is off, the default build contains no `resvg`. |
| `rusqlite` (bundled) | Local on-disk cache (`~/.cache/biscuit-icon/icons.db`) of network-fetched Iconify bodies **and** set metadata. The `bundled` feature compiles SQLite in, avoiding a system dependency. |
| `reqwest` (0.12) | Iconify JSON API client (`GET /{prefix}.json?icons={name}`, `/collections`, `/search?query=…`), used by both runtime lookups and the dev-only `populate_assets` binary. |
| `strum` / `strum_macros` | Enum ↔ string conversion (`Display`/`EnumString`/`EnumIter`) for every domain set, powering the string-convenience constructors. |
| `serde` / `serde_json` | Deserializing the Iconify JSON API responses and serializing `IconBody`. |
| `thiserror` | `IconError` definition. |
| `tokio` | Async runtime for the network client; also used to offload cache I/O from the async runtime thread with `spawn_blocking`. |
| `dirs` | Resolving the default cache directory when `HOME` is unset. |
| `tracing` | Structured diagnostics. |

## CLI (`biscuit-icon-cli`, binary `icon`)

| Crate | Why |
|-------|-----|
| `clap` (derive/env/unstable-ext/wrap_help) | Argument parsing and subcommands. |
| `clap_complete` (unstable-dynamic) | `completions <shell>` script generation plus dynamic, cache-querying value completion via `ArgValueCompleter`. |
| `color-eyre` | Error reporting in `main`. |
| `darkmatter` (path) | Markdown-to-terminal rendering for the `--code-block` format flag. |
| `tokio` | Async runtime. |
| `tracing` / `tracing-subscriber` | `--debug` / `RUST_LOG` diagnostics on stderr; `--verbose` drives user-facing output only. |
| `biscuit-tui` (path) | Interactive picker (`choose_many`) for the `--pick` flag and inexact-match TTY flows. |

## Cargo features

- `image` (default **off**) on both `biscuit-icon` and `biscuit-icon-cli` enables image-protocol terminal rendering of glyph-less icons. When disabled, the terminal ladder stops at the text identifier fallback.
- `terminal-tests` on the CLI enables the L2 test gate. Its integration target
  declares `required-features`, so local L1 does not compile that target; CI and
  `just test-l2` enable it alongside `image`.
- The `iconify` crate is intentionally **not** used; bodies are vendored offline and assembled into `<svg>` locally.
