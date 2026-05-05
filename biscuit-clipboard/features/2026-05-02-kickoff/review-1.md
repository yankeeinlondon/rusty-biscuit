---
agent: claude
model: ""
ready: false
---

# Biscuit Clipboard Kickoff — Review #1

## Verdict

**Not ready for production.** The build is green and 97 tests pass, but the
implementation has a critical functional gap: the `clipper` daemon never starts
the clipboard watcher, so history is only ever populated by `POST /set`
requests, not by observing real clipboard activity. That's the core feature of
the product. Several REST API contracts also diverge from the spec in ways that
will break the documented client integration (Claudine).

The skeleton, threading model abstractions, storage/spill design, and test
posture for unit-level concerns are solid; the gaps are concentrated in the
service entry point, the request/response shapes on the API, and the watcher
implementation.

---

## Critical Gaps (Block Release)

### 1. The daemon never observes the clipboard

`biscuit-clipboard/service/src/main.rs` builds `AppState` with an empty
`History`, binds the Axum listener, and starts signal handlers — but it never
calls `spawn_watcher`, never wires an mpsc receiver to the history insert loop,
and never starts the supervisor task described in the "Threading Model" section
of the spec.

Result: `GET /history`, `GET /history/latest`, and `GET /current` only ever
return entries that were written via `POST /set`. The product's core value
proposition — automatic clipboard observation with rolling history — is missing
from the running daemon.

**Fix:** in `main.rs`, after `Storage::new`, instantiate a `SystemClipboard`,
call `spawn_watcher`, spawn a Tokio task that drains the receiver and calls
`history.insert(...)` (skipping when `is_concealed()` is true), and wire a
`Supervisor` that respawns the watcher on failure. Plumb the supervisor
status into `/health` (see #5).

### 2. `spawn_watcher` does not use `clipboard-rs`'s change listener

`lib/src/watcher.rs` polls `backend.get_text()` every 250 ms in a loop. The
spec is explicit that the killer feature of `clipboard-rs` — the
`ClipboardHandler` / `ClipboardWatcherContext` change listener — is what the
watcher should use, and that the on-change handler should be wrapped in
`std::panic::catch_unwind` so the supervisor can restart it. None of that is
present.

Beyond missing the architecture, the polling implementation also:

- Only reads text. HTML, RTF, image, and files formats are dropped, contrary
  to the multi-format spec ("A single clipboard copy can contain multiple
  formats simultaneously").
- Has no `is_concealed` guard, so password manager entries will leak into
  history once #1 is fixed.
- Cannot detect content that is set and then immediately re-set (250 ms gap)
  or content that disappears and is re-added with the same payload.

**Fix:** rewrite `spawn_watcher` around `ClipboardWatcherContext` per the spec's
example, capture every available format inside `on_clipboard_change`, wrap the
body in `catch_unwind`, and emit a `WatcherEvent` carrying all formats plus a
concealed flag.

### 3. `POST /set` request body diverges from the spec

The spec defines:

```json
{ "content_type": "text", "data": "hello" }
{ "content_type": "image", "data": "<base64>", "width": 100, "height": 100 }
```

The implementation in `service/src/api.rs::SetRequest` is `{ "text": "..." }`
only. The integration test `test_disk_spill_for_large_image_content` actually
demonstrates the bug: it sends a spec-compliant `content_type: "image"` body,
gets `400 BAD_REQUEST`, and silently falls through to a text fallback — the
test then asserts only "history is non-empty", which is satisfied by the
fallback rather than by the image path.

Image set is therefore *not implemented at all*, despite being explicit in the
"Request Shape" and "V1 Scope" sections.

**Fix:** make `SetRequest` a tagged enum (`#[serde(tag = "content_type")]`)
mirroring `ContentType`, decode base64 for image bodies, route through
`Storage::spill_if_needed` so large images go to disk on creation, and update
the failing assertion in the test.

### 4. Error response shape is wrong

The spec mandates:

```json
{ "error": { "code": "entry_not_found", "message": "..." } }
```

`api.rs::error_response` produces `{ "error": "<message>" }` (flat string,
no code). There is no shared `ErrorResponse` struct, no `IntoResponse` impl,
and none of the documented error codes (`entry_not_found`, `bad_request`,
`format_not_available`, `internal`, `service_unavailable`) are emitted.

This is a public contract break: any client written against the spec will
fail to deserialize errors.

**Fix:** introduce `error.rs` in the service crate with a single
`ErrorResponse { code, message }` struct, an `IntoResponse` impl, and helper
constructors; replace every `error_response(...)` callsite.

### 5. `/health` is missing the `watcher` field and the degraded contract

The spec says `/health` body must include `watcher: "running" | "degraded"`,
and that `503 service_unavailable` is returned once the supervisor exhausts
retries. The current `HealthResponse` only has `{ status, entries }`. The
supervisor exists in the lib but is never instantiated by the daemon, so its
status cannot be observed.

**Fix:** thread the `Supervisor` (or a snapshot of `SupervisorStatus`) into
`AppState` via an `ArcSwap` or `Arc<Mutex<...>>`, project it into the health
body, and return 503 when degraded.

### 6. `/current` does not match the documented behavior

Spec: "Live read from OS clipboard at request time; returns same shape as
history entries with `id: \"current\"`, `204 No Content` when empty; does not
insert into history."

Implementation: returns `history.latest()`, with `404 NOT_FOUND` (and a
non-spec error body) when empty, and the entry's real id rather than
`"current"`. There is no live `ClipboardBackend` read.

**Fix:** `AppState` should hold an `Arc<dyn ClipboardBackend + Send + Sync>`;
the handler calls `tokio::task::spawn_blocking` to read each format, builds
an in-memory `ClipboardEntry` with `id: "current"`, returns `204` when no
formats are available, and never inserts into history.

---

## Significant Gaps (Behavior Off-Spec, Should Block Release)

### 7. Query parameters on `/history` and `/history/:id/content` are ignored

The spec lists `?since=`, `?limit=`, `?format=`, and `?encoding=base64`. None
of them are wired up in the handlers or the client. `History::since` exists in
the lib but is unused by the API. `History` itself has no `limit` argument on
`all()`. `find_format` exists but `get_content` only ever returns text or
image, never HTML/RTF/files.

**Fix:** add `Query<HistoryParams>` and `Query<ContentParams>` extractors,
implement format selection with the documented priority (`text > html > rtf >
image > files`), return `406 format_not_available` when the requested format
is missing, and apply `?encoding=base64` orthogonally for binary payloads.

### 8. `clip clear` clears history, not the clipboard

`cli/src/main.rs::cmd_clear` calls `client.clear_history()`. The spec table
says `clip clear` should "Clear clipboard". This is a user-visible footgun —
running `clip clear` will silently wipe the user's history while leaving the
clipboard untouched.

**Fix:** either change the CLI to call a new `POST /clear` (or `set_text("")`)
endpoint, or keep `clear` for history and rename the spec command. Pick one
and make the doc and code agree.

### 9. `clip get --format <fmt>` argument is parsed but ignored

`cmd_get` matches `format: _` and never forwards it. The CLI advertises the
flag but silently does nothing with it.

### 10. `clip watch` runs its own watcher instead of streaming from the service

`cmd_watch` constructs a fresh `SystemClipboard` and calls `spawn_watcher`,
which means two watchers run if the service is also up — and any concealed-
content handling done by the service is bypassed. The spec describes `clip
watch` as a foreground run "for debugging"; even so, having the CLI bypass
the service contradicts the "only `clipper` touches the clipboard" invariant
that the `set` command was specifically designed to uphold.

**Fix:** add `GET /events` (SSE) or `GET /history?since=<server-time>`
polling on the service, and have `clip watch` consume that. If a true
foreground daemon is the goal, document it that way and have it refuse to
run when the service is up.

### 11. `is_pid_alive` is a no-op on Windows

`config::is_pid_alive` returns `false` on non-Unix targets. The spec is
explicit that V1 is cross-platform (macOS, Windows, Linux). On Windows this
will make `service_status` always report Stopped, will treat every existing
clipper as dead, and will trigger an auto-start storm.

**Fix:** use `windows::Win32::System::Threading::OpenProcess` (or the
`sysinfo` crate already used elsewhere in the monorepo) on Windows.

### 12. Hash mismatch between `history.rs` dedup probe and `entry.rs` id

Both files compute a content hash, but for `ImageSnapshot::Spilled`:

- `entry.rs::compute_hash` writes `size_bytes.to_le_bytes()`
- `history.rs::compute_content_hash` writes `img.data()`, which returns `&[]`
  for `Spilled` (see `ImageSnapshot::data`)

Two spilled images of different sizes can collide in the dedup check yet
produce different `id`s once an entry is created. The two functions are also
straight duplicates for the non-spilled cases. This will become a real bug as
soon as the spill path is exercised (currently it isn't, because the watcher
doesn't generate images).

**Fix:** delete `history::compute_content_hash` and call into a shared
`ClipboardEntry::content_hash_of(&[ClipboardFormat])` helper. Make the spilled
hash use a stable identity (the file's xxhash, which is what the filename
already encodes).

### 13. Concealed content is read from the trait but never enforced

`ClipboardBackend::is_concealed` exists and the `MockClipboard` honors it, but
no caller in the service ever checks it. The spec says "If present, the
clipboard entry is skipped entirely — it is not stored in history." Currently,
since the watcher is missing entirely (#1), this is moot — but the fix for #1
must include this gate.

### 14. No autostart wiring (launchd / systemd / Startup)

Spec: "Starts at login (launchd on macOS, Startup on Windows, systemd/autostart
on Linux)." There is no plist, no systemd unit, no autostart shim, and no
`clip service install` command. The CLI's `service start` simply spawns the
binary in the foreground of the current session. After a reboot the service
is gone.

This may be acceptable to defer if it is explicitly noted as v1.x — but the
spec lists it under V1 scope, so flag it as a gap.

---

## Test Coverage Gaps

The unit/integration coverage of `History`, `Storage`, and the supervisor is
strong. The watcher, REST contract, and end-to-end CLI↔service flow are
under-tested. Specifically:

1. **No watcher integration test.** `test_spawn_watcher_sends_events` only
   verifies the polling loop emits something for a static text mock. There is
   no test that asserts `clipboard-rs` change-listener integration, panic
   recovery via `catch_unwind`, supervisor restart on watcher death, or that
   the watcher captures HTML/RTF/image/files alongside text.

2. **No test asserts spec error shape.** Tests check status codes but not
   the `{ error: { code, message } }` body. Add fixture tests for each
   error code.

3. **No test of `?since`, `?limit`, `?format`, `?encoding=base64`.** All
   query-parameter behavior is untested.

4. **`/current` tests assert the wrong contract.** `test_current_reflects_latest`
   and `test_current_empty_returns_not_found` codify the current (incorrect)
   behavior; they will need to be rewritten when /current is fixed.

5. **No CLI binary test.** No `tests/` directory with `assert_cmd` /
   `predicates` exercising `clip get`, `clip set`, `clip history --json`,
   `clip service status`. The monorepo convention (per memory: "assert_cmd +
   predicates for CLI integration tests") is to have these.

6. **`config::read_port_file_missing` and `read_pid_file_missing` are flaky.**
   They read the user's real `dirs::runtime_dir()` and assert `None`. If the
   developer happens to have a real `clipper` running, both tests fail. Use
   a `tempfile::TempDir` and an env override (or restructure `runtime_dir` to
   accept an injected base).

7. **`ClipperClient` HTTP behavior is untested.** `test_client_new` and
   `test_service_status_stopped` are smoke tests; nothing verifies `try_connect`
   against a `wiremock` server, the auto-start handshake, the X-Clipper
   fingerprint check on the *response*, or the exponential-backoff polling.

8. **No concealed-content test path.** `MockClipboard.concealed = true`
   should round-trip through the watcher and verify the entry is dropped.

9. **Image disk-spill end-to-end is not actually tested.** As noted in #3,
   `test_disk_spill_for_large_image_content` falls back to text on the 400
   and asserts only that history isn't empty. The spilled-file path on disk
   is never inspected.

---

## Code Quality / Ergonomics

1. **Module name drift from spec.** Spec calls for `lib/src/server.rs`; the
   service's REST router lives in `service/src/api.rs` instead. Either bring
   the router back into the lib (which lets the lib-level `client.rs` and
   `server.rs` be tested together) or update the spec — but the lib's module
   doc still advertises a layout that doesn't exist (`lib.rs` doesn't list
   `client` or `watcher` in its `## Module Layout` doc comment, and lists
   nothing about a server). Pick one truth.

2. **Duplicate hash logic.** `entry::compute_hash` and
   `history::compute_content_hash` are 90% identical. Centralize.

3. **Two `EntrySummary` types.** Defined in both `lib/src/client.rs` and
   `service/src/api.rs`, with the same fields but `From<&ClipboardEntry>`
   only on the service one. Move the canonical type to the lib and have the
   service `use` it. Same goes for `HistoryResponse`, `SetResponse`,
   `HealthResponse`.

4. **`EntryId = String` is a footgun.** Untyped strings make it easy to pass
   a path or arbitrary text where an id is expected. A newtype
   `pub struct EntryId(String)` with `Display` / `FromStr` would catch a
   class of bugs, especially as the CLI grows.

5. **`primary_content_type` and `preview` duplicate the priority list.**
   Extract `const FORMAT_PRIORITY: [ContentType; 5]` once and reuse.

6. **`ClipboardError::FormatNotAvailable` is defined but never raised.**
   Either wire it into `get_content` (#7) or remove.

7. **`ImageSnapshot::data` returns `&[]` for `Spilled`.** That's a quiet
   correctness hazard (see #12). Either return `Option<&[u8]>` or remove the
   method and force callers through `Storage::load_spilled`.

8. **Unbounded mpsc on the watcher channel.** `mpsc::unbounded_channel` is
   used; if the receiver task stalls, memory grows without limit. A bounded
   channel (`mpsc::channel(64)`) with `try_send` and a drop-oldest policy on
   backpressure is safer for a long-running daemon.

9. **`History::insert` recomputes the hash twice.** The dedup probe in
   `compute_content_hash` runs, then `ClipboardEntry::new` does it again.
   Compute once, pass the hash into a private `with_hash` constructor.

10. **`History::with_max_entries`'s `.pop()` removes from the back, but
    after `entries.insert(0, …)`** so the newest entry is at index 0 — that
    is correct, but the comment/spec calls this a "ring buffer" and `Vec` +
    `insert(0, …)` is O(n). A `VecDeque::push_front` is both more
    semantically honest and faster.

11. **Clippy warnings.** 8 warnings in test code (`needless_borrows_for_generic_args`,
    `unnecessary_map_or`). They're cosmetic, but the curated areas in the
    monorepo run clippy clean in CI.

12. **`find_clipper_binary` returns `String` and can produce `"clipper"` with
    no parent.** When sibling lookup fails, it shells out via `PATH`, which is
    fine — but worth an `eprintln!`-style debug log so users understand which
    binary was launched.

13. **`ClipperClient::new` panics on `reqwest::Client::build()` failure.**
    Replace `expect(...)` with a `Result`-returning constructor.

14. **`set_text` body in client uses `{"text": "..."}`** to match the
    server's current shape — once #3 is fixed this needs to be updated to
    `{"content_type":"text","data":"..."}`.

15. **No structured logging in the service hot path.** `tracing` is in
    deps but only `eprintln!` / `println!` are used in `main.rs`. Switch
    to `tracing::info!` / `tracing::error!` so users can filter via
    `RUST_LOG`.

16. **`History::entries: Vec` should be `VecDeque`.** Front insert is O(n);
    front-truncate via `pop_front`/`pop_back` is O(1). No correctness impact
    today, but once the watcher is firing it will matter.

17. **Doc comments are missing on most public types.** `ClipboardBackend`,
    `History`, `Storage`, `ClipperClient`, `Supervisor`, `WatcherEvent` —
    none have rustdoc summaries. The CLAUDE.md doc convention (`## Examples`,
    `## Errors`) hasn't been applied.

---

## Summary of Recommended Action

Before merging:

1. Wire the watcher and supervisor into the daemon (fixes #1, #2, #5, #13).
2. Implement the spec'd `POST /set` and `/history/:id/content` request
   shapes, including images and base64 (fixes #3, #7).
3. Make `/current` a live read with `id: "current"` and `204` semantics
   (fixes #6).
4. Switch error responses to `{ error: { code, message } }` and emit the
   documented codes (fix #4).
5. Fix Windows pid-liveness (#11) and the spilled-image hash divergence
   (#12).
6. Add CLI integration tests, watcher integration tests, error-shape
   tests, and a real image-spill end-to-end test (fixes #1–#7 in coverage).
7. Decide on `clip clear` semantics and remove dead `--format` arg (#8, #9).

After those, the package is shippable. Storage, supervisor, dedup, and the
unit-level lib design are good foundations.
