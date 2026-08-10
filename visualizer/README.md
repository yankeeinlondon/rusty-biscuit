# Visualizer

A cross-platform desktop application built with [Tauri 2](https://tauri.app/).
This is the bare application skeleton — the Excalidraw editor, document model,
and Tauri command/event surface described in
[`features/2026-06-14-excalidraw/spec.md`](features/2026-06-14-excalidraw/spec.md)
are not yet implemented.

## Layout

```
visualizer/
├── dist/               # Static frontend (stub; replaced by React + Vite later)
│   └── index.html
├── src-tauri/          # Rust application crate (package `visualizer`)
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/   # Tauri 2 permission capabilities
│   ├── icons/          # Placeholder app icons (regenerate with `cargo tauri icon`)
│   └── src/
│       ├── main.rs     # Thin binary entry point
│       └── lib.rs      # `run()` — builds the Tauri app
└── justfile
```

The Rust crate lives in `src-tauri/` (Tauri convention) rather than the
monorepo's usual `{area}/src` so the frontend can own the area root once it is
added.

## Commands

```bash
just run      # cargo run -p visualizer (launches the window, embeds dist/)
just build    # cargo build -p visualizer
just test     # workspace test recipe for the visualizer crate
just lint     # cargo clippy -p visualizer

just dev      # cargo tauri dev   (requires `cargo install tauri-cli`)
just bundle   # cargo tauri build (requires the Tauri CLI + platform tooling)
```

`just run` works without the Tauri CLI because the static `dist/` frontend is
embedded into the binary at compile time.

## Notes

- Placeholder PNG, ICO, and ICNS icons are committed so the crate compiles on
  every desktop platform; replace them with real artwork via
  `cargo tauri icon <source.png>` before distribution.
- Capabilities start narrow (`core:default` only); broaden deliberately per the
  spec's security boundaries.
