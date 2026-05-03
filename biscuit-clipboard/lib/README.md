# Biscuit Clipboard Library

Core library for clipboard observation, history management, and content models used by the `clipper` service and `clip` CLI.

## Modules

- `content` — `ContentType`, `ClipboardFormat`, `ImageSnapshot`
- `entry` — `ClipboardEntry` with xxHash-based `EntryId`
- `backend` — `ClipboardBackend` trait and `SystemClipboard` implementation
- `history` — Ring buffer with 1-hour TTL and 2-entry floor
- `storage` — Disk-spill logic for large entries
- `error` — Error types
