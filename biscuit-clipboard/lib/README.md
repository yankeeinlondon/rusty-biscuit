# Biscuit Clipboard Library

Core library for clipboard observation, history management, and content models used by the `clipper` service and `clip` CLI.

## Modules

- `content` — `ContentType`, `ClipboardFormat`, `ImageSnapshot`
- `entry` — `ClipboardEntry`, the `EntryId` newtype, and the `FORMAT_PRIORITY` constant
- `backend` — `ClipboardBackend` trait and `SystemClipboard` implementation
- `history` — `VecDeque`-backed ring buffer with TTL + min/max bounds
- `storage` — Disk-spill logic for large entries
- `watcher` — Change-listener watcher and `Supervisor`
- `api_types` — Shared REST request/response types
- `client` — `ClipperClient` (REST client used by `clip`)
- `config` — Port resolution, runtime-dir, cross-platform PID liveness
- `error` — Error types

The Axum router that exposes the REST surface lives in the
`biscuit-clipboard-service` crate (`service/src/api.rs`), not in this lib.
