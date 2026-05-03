# Biscuit Clipboard Kickoff

## Overview

**Biscuit Clipboard** is a background service that watches the host's clipboard for changes, maintains a rolling history of entries, and exposes a REST API for other tools to query clipboard state and history.

The first consumer is **Claudine**, which will use the API to understand what's on the clipboard — enabling context-aware interactions like offering to paste an image into a prompt, or surfacing what changed since a session started.

## Binaries

| Binary | Crate | Role |
|--------|-------|------|
| `clip` | `biscuit-clipboard-cli` | CLI client — talks to the service via REST; auto-starts the service if not running |
| `clipper` | `biscuit-clipboard-service` | Background daemon — watches clipboard via `clipboard-rs`, serves REST API |

The `clip` CLI is implemented with `clap` following the `cli` skill conventions. Some commands (e.g. `clip history`) open a TUI built with `ratatui` and `biscuit-tui` components.

Cross-platform: macOS, Windows, Linux.

## Architecture

### Service Model

`clipper` is a long-running background service that:

1. Starts at login (launchd on macOS, Startup on Windows, systemd/autostart on Linux)
2. Watches clipboard changes via `clipboard-rs` change listener (read-only observation — does not interfere with other clipboard managers running on the host)
3. Maintains a rolling history with a **1-hour TTL, minimum 2 entries**
4. Serves a REST API on a fixed local port (default `17530`, overridable via `CLIP_PORT` environment variable)

`clipper` and `clip` are designed to coexist peacefully with other clipboard managers (Paste, Maccy, Ditto, CopyQ, etc.). They only observe clipboard state — they never claim exclusive access or interfere with another manager's operation.

#### Process coordination

On startup, `clipper` writes two files to a platform-appropriate runtime directory:

- **PID file** (`clipper.pid`) — locked with `flock` (or equivalent) to prevent concurrent instances. If the lock is already held, the new process exits immediately.
- **Port file** (`clipper.port`) — contains the TCP port the service is listening on. This is the source of truth for locating a running service.

The PID file is held for the lifetime of the process. If `clipper` crashes, the lock is released automatically by the OS, allowing a fresh start.

**Known limitation:** API authentication is deferred to a follow-up iteration. In this version, any local process can read/write clipboard state via the REST API. The `/health` fingerprint (see below) prevents false-positive service detection but does not provide access control.

### Startup Behavior (clip CLI)

When the user runs `clip <command>`, the CLI performs a four-step handshake to locate or start the service:

1. **Read port file** — look up `clipper.port` for the recorded PID and port number.
2. **PID liveness check** — verify that the recorded PID is still a running process. If not, the service is stale.
3. **Health fingerprint** — send `GET /health` with an `X-Clipper: 1` request header. A genuine `clipper` instance responds with an `X-Clipper: 1` response header. This prevents false positives when another service occupies the same port.
4. **Auto-start** — if the port file is missing, the PID is dead, or the health check fails the fingerprint test, start a new `clipper` instance. Poll `GET /health` with exponential backoff (starting at 50 ms, cap at 500 ms) for up to 5 seconds. Once healthy, proxy the original request.

The `CLIP_PORT` environment variable is used when **starting** the service, but the port file is the source of truth for **finding** a running service. This means a user can change `CLIP_PORT` between starts without confusing the CLI.

This means the service auto-starts on first use and stays running in the background.

### History Retention

Entries are retained with a **1-hour TTL**, but the **last 2 entries are always kept** regardless of age. This ensures there's always meaningful clipboard context available even after a period of inactivity.

History is maintained as a ring buffer. Deduplication uses content hashing (xxHash via `biscuit-hash`) to skip identical re-copies.

#### Memory strategy: disk spill for large content

Small entries stay in memory. Large entries (especially images) are written to a platform-appropriate cache directory (e.g. `~/.cache/biscuit-clipboard/` on Linux, `~/Library/Caches/biscuit-clipboard/` on macOS, `%LOCALAPPDATA%\biscuit-clipboard\` on Windows) and loaded on demand. The ring buffer holds only references and metadata in memory; full content is recoverable from history via the filesystem.

A configurable size threshold (default: **64 KiB**) determines whether an entry's content stays inline or spills to disk. Entries below the threshold are held directly in the ring buffer. Entries above it are written to a content-addressed file (`{cache_dir}/{xxhash}.dat`) and the ring buffer stores only the file path and metadata.

The `/content` endpoint transparently loads from disk when the requested entry has been spilled. Spilled files are cleaned up when their corresponding history entries expire from the ring buffer.

### Backend Abstraction

Clipboard operations are isolated behind a `ClipboardBackend` trait, making the code testable with mocks and leaving the door open to swap `clipboard-rs` for `arboard` if needed:

```rust
trait ClipboardBackend {
    fn get_text(&self) -> Result<Option<String>>;
    fn get_html(&self) -> Result<Option<String>>;
    fn get_image(&self) -> Result<Option<ImageData>>;
    fn get_files(&self) -> Result<Option<Vec<PathBuf>>>;
    fn set_text(&self, text: &str) -> Result<()>;
    fn has(&self, format: ContentFormat) -> bool;
    fn is_concealed(&self) -> Result<bool>;
}
```

The `ClipboardBackend` trait is intentionally synchronous, matching the underlying `clipboard-rs` API. All async bridging is handled externally (see Threading Model below).

### Threading Model

`clipboard-rs` provides a blocking change listener (`start_watch()` blocks the calling thread). The service bridges this synchronous world to the async Tokio/Axum runtime as follows:

1. **Dedicated OS thread** — the `clipboard-rs` watcher runs on a `std::thread` spawned at service start. The `on_clipboard_change()` handler captures the new `ClipboardEntry` and sends it through a `tokio::sync::mpsc` channel.
2. **Async receiver** — a Tokio task drains the channel and inserts entries into the history ring buffer, which is wrapped in `Arc<RwLock<History>>` for shared access.
3. **Axum handlers** — REST handlers access history via the `Arc<RwLock<History>>`. Direct clipboard reads (e.g. `GET /current`) use `tokio::task::spawn_blocking` to call `ClipboardBackend` methods without blocking the Tokio runtime.
4. **Supervisor task** — a Tokio supervisor task monitors the watcher OS thread. The `on_clipboard_change` handler is wrapped in `std::panic::catch_unwind`; on panic, an error is sent through the mpsc channel. If the watcher thread dies, the supervisor spawns a new one with exponential backoff (max 3 retries). After exhausting retries, the service degrades gracefully — it continues serving stale history and the `/health` endpoint reports `watcher: "degraded"`.

This separation keeps the synchronous `ClipboardBackend` trait honest (no hidden async assumptions) while ensuring the Axum runtime never blocks on clipboard I/O.

### Content Model

A single clipboard copy can contain multiple formats simultaneously (plain text + HTML + RTF). Each entry captures all available formats:

```rust
struct ClipboardEntry {
    id: EntryId,
    timestamp: SystemTime,
    content_hash: u64,
    formats: Vec<ClipboardFormat>,
}

/// Unique identifier for a history entry.
///
/// Computed as the xxHash content hash encoded as a lowercase hex string
/// (e.g. `"a1b2c3d4e5f67890"`). This provides uniqueness and ties the ID
/// to the entry's content — identical clipboard payloads produce the same ID.
type EntryId = String;

enum ClipboardFormat {
    Text(String),
    Html(String),
    Rtf(String),
    Image(ImageSnapshot),
    Files(Vec<PathBuf>),
}

/// Holds image data either inline (small) or on disk (large).
///
/// The size threshold is configurable (default 64 KiB). Below the threshold,
/// `Inline` holds raw PNG/JPEG bytes directly. Above it, `Spilled` stores a
/// content-addressed path in the cache directory — the bytes are loaded on
/// demand by the `/content` endpoint.
enum ImageSnapshot {
    Inline { data: Vec<u8>, width: u32, height: u32 },
    Spilled { path: PathBuf, width: u32, height: u32, size_bytes: u64 },
}

enum ContentType {
    Text,
    Html,
    Rtf,
    Image,
    Files,
}
```

The `ContentType` enum identifies the primary type for display/filtering. `ClipboardFormat` captures the actual data in each format.

#### Preview field

The `preview` field shown in REST responses is **computed on-the-fly during serialization** — it is not stored in the `ClipboardEntry` struct. The computation rules are:

- **Text/HTML/RTF**: first ~80 characters of the primary text format, truncated with `...` if longer.
- **Image**: `"Image {width}x{height}"` (e.g. `"Image 1920x1080"`).
- **Files**: `"{n} files"` (e.g. `"3 files"`), or the single filename when only one file.

## REST API

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Is the service alive? Responds with `X-Clipper: 1` header; body includes `watcher` field (`"running"` or `"degraded"`) |
| `GET` | `/history` | List entries (query: `?since=<timestamp>`, `?limit=<n>`) |
| `GET` | `/history/latest` | Most recent clipboard entry |
| `GET` | `/history/:id` | Single entry detail |
| `GET` | `/history/:id/content` | Get entry content — raw bytes by default; `?encoding=base64` for base64; `?format=text\|html\|rtf\|image\|files` to select format |
| `GET` | `/history/:id/thumbnail` | Image thumbnail if applicable |
| `GET` | `/current` | Live read from OS clipboard at request time; returns same shape as history entries with `id: "current"`, `204 No Content` when empty; does not insert into history |
| `POST` | `/set` | Set clipboard content (JSON body with `content_type` discriminator; see Request Shape) |
| `DELETE` | `/history` | Clear history |

### Request Shape

`POST /set` accepts a JSON body with a `content_type` discriminator that maps directly to the `ContentType` enum.

**Text:**

```json
{
  "content_type": "text",
  "data": "hello"
}
```

**Image:**

```json
{
  "content_type": "image",
  "data": "<base64-encoded image bytes>",
  "width": 100,
  "height": 100
}
```

The `content_type` field determines which variant is created. The `data` field contains the payload (plain string for text-based types, base64 for binary). Image requests additionally require `width` and `height`.

The CLI's `clip set` command targets this REST endpoint rather than calling the `ClipboardBackend` directly, maintaining the invariant that only the `clipper` service touches the host clipboard.

Setting multiple formats in a single request (e.g. text + HTML simultaneously) is deferred to a future iteration.

### Response Shape

```json
{
  "id": "a1b2c3",
  "timestamp": "2026-05-02T14:32:01Z",
  "content_type": "text",
  "formats": ["text", "html"],
  "preview": "Hello world...",
  "size_bytes": 42
}
```

### Error Responses

All REST API error responses use a flat JSON shape:

```json
{ "error": { "code": "entry_not_found", "message": "No history entry with id 'abc123'" } }
```

The `code` field is a machine-readable snake_case string. The `message` field is human-readable. Implemented as a single `ErrorResponse` struct in `error.rs` with an `IntoResponse` impl shared by all handlers.

Common error codes:

| Code | HTTP Status | Meaning |
|------|-------------|---------|
| `entry_not_found` | 404 | No history entry with the given ID |
| `bad_request` | 400 | Malformed request body or invalid parameters |
| `format_not_available` | 406 | Requested format does not exist for the entry |
| `internal` | 500 | Unexpected server error |
| `service_unavailable` | 503 | Service is degraded (e.g. watcher exhausted retries) |

### Content Format Selection

The `/history/:id/content` endpoint accepts a `?format=` query parameter with values: `text`, `html`, `rtf`, `image`, `files`. When omitted, the endpoint returns the "primary" format using this priority order: text > html > rtf > image > files. Returns `406` with error code `format_not_available` if the requested format does not exist for that entry.

The `?encoding=base64` parameter remains orthogonal to format selection and can be combined with any `?format=` value.

### Sensitive Content Filtering

On macOS, the service checks for the `org.nspasteboard.ConcealedType` pasteboard type (set by password managers such as 1Password and Bitwarden). If present, the clipboard entry is skipped entirely — it is not stored in history.

The `ClipboardBackend` trait exposes `fn is_concealed(&self) -> Result<bool>` for this purpose. The macOS implementation checks the pasteboard type directly. Windows and Linux implementations return `false` in v1. Platform coverage for concealed-type detection will be expanded in a follow-up iteration as platform-specific conventions are identified.

### Claudine Integration Points

Claudine's primary use cases:

- `GET /history?since=<session_start>` — "what changed since I started"
- `GET /history/latest` — "what's on the clipboard right now"
- Content-type awareness — detect that the user has an image on the clipboard and offer context-aware actions

## CLI Commands

| Command | Mode | Description |
|---------|------|-------------|
| `clip get` | Stdout | Print current clipboard (text, or metadata for other types) |
| `clip get --format html` | Stdout | Print current clipboard in specific format |
| `clip set` | Stdin | Set clipboard from stdin |
| `clip set "text"` | Arg | Set clipboard from argument |
| `clip info` | Stdout | Metadata about current clipboard (type, size, timestamp) |
| `clip history` | **TUI** | Browse entries, select to paste |
| `clip history --json` | Stdout | Machine-readable history dump |
| `clip watch` | Foreground | Run the watcher in foreground (for debugging) |
| `clip clear` | Stdout | Clear clipboard |
| `clip service start` | Stdout | Start the background daemon |
| `clip service stop` | Stdout | Stop the background daemon |
| `clip service status` | Stdout | Is it running, uptime, entry count |

## History TUI

`clip history` opens a filmstrip-style TUI that animates up from the bottom of the terminal. The core component is a reusable **bottom-drawer filmstrip** widget that lives in `biscuit-tui` for reuse across the monorepo.

- **Height**: `min(50% of terminal rows, 40 rows)`
- **Width**: `100%` of terminal width
- **Layout**: horizontal filmstrip — each "frame" is a clipboard entry showing a preview (truncated text, image thumbnail, file count, etc.)
- **Navigation**: `←` / `→` arrows move selection between entries, `Enter` selects the active entry (copies it back to the clipboard)
- **Animation**: the panel slides up from the bottom edge on open; pressing `Escape` slides it back down (reverse animation), `q` also dismisses

Each frame displays:
- Content type icon/badge (Text, HTML, RTF, Image, Files)
- Preview snippet (first ~80 chars of text, or image dimensions, or file count)
- Relative timestamp (e.g. "2m ago", "1h ago")

## Module Layout

### Library (`biscuit-clipboard/lib/src/`)

```
src/
├── lib.rs              # Re-exports
├── backend.rs          # ClipboardBackend trait + clipboard-rs impl
├── content.rs          # ContentType, ClipboardFormat, ImageSnapshot
├── entry.rs            # ClipboardEntry (id, timestamp, hash, formats, preview)
├── history.rs          # Ring buffer with 1hr TTL + 2-entry floor + dedup
├── storage.rs          # Disk-spill logic (cache dir, content-addressed files, cleanup)
├── watcher.rs          # Wraps clipboard-rs watcher, feeds history via mpsc channel; includes catch_unwind panic recovery and supervisor restart logic
├── server.rs           # REST API (axum), health fingerprint via X-Clipper header
├── client.rs           # REST client for CLI to talk to service
├── config.rs           # Port (default 17530, override via CLIP_PORT env), TTL, max entries, spill threshold
└── error.rs            # Error types
```

### Service (`biscuit-clipboard/service/src/`)

```
src/
├── main.rs             # Daemon entry point, signal handling, PID/port file creation
└── daemon.rs           # Service lifecycle (start, stop, status), flock PID file, port file write, startup handshake
```

### CLI (`biscuit-clipboard/cli/src/`)

```
src/
├── main.rs             # clap app entry point
├── commands/
│   ├── mod.rs
│   ├── get.rs          # clip get
│   ├── set.rs          # clip set
│   ├── info.rs         # clip info
│   ├── history.rs      # clip history (TUI + --json)
│   ├── clear.rs        # clip clear
│   ├── watch.rs        # clip watch (foreground)
│   └── service.rs      # clip service {start,stop,status}
└── tui/
    ├── mod.rs
    └── history_view.rs # Ratatui history browser using biscuit-tui components
```

## V1 Scope

V1 delivers the full specification with one exception: the `clip history` TUI filmstrip is deferred to v1.1.

### Included in V1

- All REST endpoints (`/health`, `/history`, `/history/latest`, `/history/:id`, `/history/:id/content`, `/history/:id/thumbnail`, `/current`, `/set`, `DELETE /history`).
- All CLI commands (`get`, `set`, `info`, `clear`, `watch`, `service start/stop/status`).
- `clip history --json` — machine-readable JSON output to stdout.
- Background service with PID/port file coordination and auto-start.
- Disk-spill storage for large content.
- Content hashing (xxHash) for deduplication and `EntryId` generation.
- Sensitive content filtering — macOS concealed-type detection to skip password manager entries.
- Cross-platform support (macOS, Windows, Linux).

### Excluded from V1 (deferred to v1.1)

- **`clip history` TUI filmstrip** — the interactive ratatui-based filmstrip widget described in the "History TUI" section. In v1, `clip history` outputs JSON to stdout (equivalent to `clip history --json`). The TUI filmstrip, including the reusable `biscuit-tui` bottom-drawer component, is planned for v1.1.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clipboard-rs` (0.3) | Clipboard backend — change listener + multi-format read/write |
| `axum` | REST API server |
| `tokio` | Async runtime |
| `biscuit-hash` | xxHash content hashing for deduplication |
| `biscuit-tui` | TUI components for `clip history` |
| `clap` | CLI argument parsing |
| `ratatui` | TUI rendering |
| `serde` / `serde_json` | Serialization for REST API |
| `chrono` | Timestamp handling |
| `dirs` | Platform-specific cache directory resolution for disk-spill storage |

## clipboard-rs Reference

The underlying clipboard backend crate.

| Attribute        | Value                  |
|------------------|------------------------|
| Latest version   | **0.3.4** (2026-04-02) |
| Recent downloads | ~10K/month             |
| Repo stars       | **160**                |
| License          | MIT                    |
| MSRV             | 1.67                   |
| Edition          | 2021                   |
| Latest commit    | 2026-04-02             |

### URLs

- Repository: https://github.com/ChurchTao/clipboard-rs
- Docs: https://docs.rs/clipboard-rs
- Crates.io: https://crates.io/crates/clipboard-rs

### Functional Footprint

- **Multi-format read/write**: plain text, HTML, RTF (rich text), PNG/JPEG/BMP/TIFF images, file lists (URI), arbitrary custom formats
- **Clipboard change listener**: handler-based real-time monitoring -- standout feature, enables history without polling
- **Thumbnail generation** for images
- **Custom format access** via format identifiers (e.g. macOS UTI types, Windows custom CF formats)
- **Configurable X11 read timeout** via `ClipboardContext::new_with_options` (default 500ms)
- **Platform coverage**: Windows, macOS, Linux X11, Linux Wayland (opt-in), iOS (beta), Android (in progress)

### Features (v0.3.4)

| Feature   | Default | Description                                                      |
|-----------|---------|------------------------------------------------------------------|
| `default` | Yes     | Enables `image`                                                  |
| `image`   | Yes     | PNG/JPEG/BMP/TIFF image clipboard support; pulls in `image` 0.25 |
| `wayland` | No      | Wayland support via `wl-clipboard-rs` 0.9 (Unix only)            |

### Platform Backends

- **Windows**: `windows` 0.59 + `clipboard-win` 5.4.1 (with `monitor` feature for change detection)
- **macOS**: `objc2` 0.6.3 (NSPasteboard with `changeCount` polling)
- **Linux/X11**: `x11rb` 0.13.2 (impl borrows from `x11-clipboard`)
- **Linux/Wayland**: optional via `wayland` feature, uses `wl-clipboard-rs`

### Code Examples

#### Multi-format read

```rust
use clipboard_rs::{Clipboard, ClipboardContext, ContentFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ClipboardContext::new()?;

    if ctx.has(ContentFormat::Text) {
        println!("Text: {}", ctx.get_text()?);
    }
    if ctx.has(ContentFormat::Html) {
        println!("HTML: {}", ctx.get_html()?);
    }
    if ctx.has(ContentFormat::Rtf) {
        println!("RTF: {}", ctx.get_rich_text()?);
    }
    if ctx.has(ContentFormat::Image) {
        let img = ctx.get_image()?;
        println!("Image: {}x{}", img.get_width(), img.get_height());
    }
    if ctx.has(ContentFormat::Files) {
        for f in ctx.get_files()? {
            println!("File: {}", f);
        }
    }
    Ok(())
}
```

#### Clipboard change listener (the killer feature for history)

```rust
use clipboard_rs::{
    Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher,
    ClipboardWatcherContext,
};

struct Manager {
    ctx: ClipboardContext,
}

impl ClipboardHandler for Manager {
    fn on_clipboard_change(&mut self) {
        // record timestamp + content type + payload here
        if let Ok(text) = self.ctx.get_text() {
            println!("clipboard changed: {}", text);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager { ctx: ClipboardContext::new()? };
    let mut watcher = ClipboardWatcherContext::new()?;
    watcher.add_handler(manager);
    watcher.start_watch();
    Ok(())
}
```
