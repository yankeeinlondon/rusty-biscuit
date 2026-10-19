# How to use the Darkmatter LSP

The Darkmatter Language Server shipped as `dmls` (`darkmatter/dmls`). Current
usage documentation lives with the crate and in this directory:

- [`darkmatter/dmls/README.md`](../../dmls/README.md) — installation,
  architecture, CLI, configuration, packaging.
- [`darkmatter/dmls/docs/features.md`](../../dmls/docs/features.md) — the full
  capability reference and per-editor support matrix.
- [`darkmatter/dmls/docs/editors/`](../../dmls/docs/editors/) — per-editor
  setup guides (VS Code, Zed, Neovim, Helix).
- [DMLS Configuration](./configuration.md) — every `.dmls.toml` key, its
  default, and how it combines with editor settings.
- [DMLS Indexing](./indexing.md) — what the server reads at startup and on
  demand, the schema registry, and the log lines that report them.
- [LSP Architecture](./architecture.md) — the server's state, file
  resolution contexts, and invalidation.

The other documents in this directory are the pre-implementation research and
design material that informed the original DMLS design; they are historical,
not current truth.
