---
agent: claude
model: opus
phases: 7
start_phase: 7
status: ready
---

# Biscuit Clipboard — Review #1 Remediation Plan

This plan addresses every item raised in
`features/2026-05-02-kickoff/review-1.md` (17 Critical/Significant gaps,
9 Test Coverage gaps, 17 Code Quality items). It is sequenced so that
each phase ends in a green workspace state.

## Conventions

- After every phase the following must pass for all three crates
  (`biscuit-clipboard-lib`, `biscuit-clipboard-cli`,
  `biscuit-clipboard-service`):
    - `cargo test -p <pkg>` — all tests green
    - `cargo clippy -p <pkg> --all-targets -- -D warnings` — clean
    - `cargo fmt --all -- --check`
- Use the area `justfile` for batch runs:
  `just -f biscuit-clipboard/justfile test` and
  `just -f biscuit-clipboard/justfile lint` (verify these recipes exist;
  if not, run direct cargo commands per crate).
- Tests follow the monorepo convention:
  - lib unit tests live next to the code (`#[cfg(test)] mod tests`)
  - CLI integration tests in `biscuit-clipboard/cli/tests/`
    using `assert_cmd` + `predicates`
  - Service integration tests in `biscuit-clipboard/service/tests/`
    using `wiremock`/`reqwest` + an in-process Axum app where helpful
- Doc comments follow the repo Rustdoc convention (no H1 inside `///`,
  H2 sections: `Examples`, `Returns`, `Errors`, `Panics`, `Safety`,
  `Notes`).

## Decisions on Ambiguous Review Items

These are the engineering decisions I am making up front so the
implementing subagents do not have to re-derive them.

- **#8 `clip clear` semantics — Decision: clear the clipboard, NOT the
  history.** Spec table is explicit: "`clip clear` — Clear clipboard".
  The CLI will be changed to call a new endpoint that empties the OS
  clipboard. History clearing remains available via `DELETE /history`
  (already implemented) and is exposed in the CLI as `clip history
  --clear` (a small additive flag) so the existing capability is not
  lost. The new server-side endpoint added is `POST /clear` (clears the
  OS clipboard via `ClipboardBackend::set_text("")`).
- **#10 `clip watch` architecture — Decision: stream from the service
  via `GET /events` (Server-Sent Events).** This preserves the "only
  `clipper` touches the clipboard" invariant. SSE was chosen over
  long-poll-on-`?since` because it's a natural fit for a push stream,
  Axum has first-class SSE support, and it does not require clients to
  reason about clock skew. If the service is not running, `clip watch`
  auto-starts it (consistent with all other CLI commands). A
  `--foreground` flag is documented as the legacy debug path (refuses
  to run when the service is up); this satisfies the "for debugging"
  language in the spec without leaving two watchers running.
- **#14 Autostart — Decision: defer to v1.1, but ship a documented
  shim now.** Implementing per-platform autostart (launchd plist,
  systemd unit, Windows Startup) is a meaningful body of work that
  touches OS configuration and is poorly suited to automated testing
  in CI. It is scoped into a Phase 7 that adds:
    1. A `clip service install` / `clip service uninstall` subcommand
       that emits the platform-appropriate manifest to a stable path
       and prints next-step instructions, but does not require root.
       On macOS we additionally invoke `launchctl bootstrap`/`bootout`
       when running as the current user; on Linux we write a user
       systemd unit and call `systemctl --user enable --now`; on
       Windows we drop a Startup folder shortcut.
    2. A clearly documented `## Notes` section in the README marking
       autostart as v1 best-effort.
   The phase is small, isolated, and can be reduced to "documentation
   only + `service install` stub" if time pressure forces it. This
   covers the spec intent without blocking the rest of the fixes.

---

## Phase 1: Wire watcher + supervisor + concealed gate into the daemon

**Status:** complete

**Addresses:** Critical #1, #2, #5, #13.

**Goal:** Make the daemon actually observe the clipboard. The watcher
must use `clipboard-rs`'s change listener (not polling), must capture
all available formats, must skip concealed entries, and must be
restarted by a supervisor whose status surfaces in `/health`.

### Files touched

- `biscuit-clipboard/lib/src/watcher.rs` — rewrite around
  `ClipboardWatcherContext` + `ClipboardHandler`. Wrap
  `on_clipboard_change` in `std::panic::catch_unwind`. Capture text,
  HTML, RTF, image, files in a single event. Skip when
  `backend.is_concealed()? == true`. Surface watcher death through
  the existing mpsc channel (a sentinel `WatcherEvent::Died { reason }`
  variant or a dedicated control channel — pick one, document it).
- `biscuit-clipboard/lib/src/lib.rs` — re-export supervisor types.
- `biscuit-clipboard/lib/src/config.rs` — add a small struct exposing
  watcher restart policy (max retries, backoff base) so the service
  can configure it from env if needed.
- `biscuit-clipboard/service/src/main.rs` — after `Storage::new`:
    1. instantiate `SystemClipboard` (wrapped in `Arc`).
    2. call `spawn_watcher(...)`, hand the receiver to a Tokio task
       that calls `history.write().await.insert(...)` per event,
       skipping concealed events as a defense-in-depth check.
    3. start a `Supervisor` task that re-spawns the watcher on
       failure with exponential backoff (max 3 retries, per spec).
    4. plumb `Arc<ArcSwap<SupervisorStatus>>` (or
       `Arc<RwLock<SupervisorStatus>>` — pick `ArcSwap` for lock-free
       reads on the hot path) into `AppState`.
    5. switch `eprintln!`/`println!` calls to `tracing::info!` /
       `tracing::error!` (overlaps with Code Quality #15 — done here
       since we are already touching `main.rs`).
- `biscuit-clipboard/service/src/api.rs` — extend `HealthResponse`:
  add `watcher: "running" | "degraded"`. Return `503` with
  `service_unavailable` when degraded (uses the new `ErrorResponse`
  shape from Phase 2 — for this phase, leave the `503` behind a
  TODO comment with a feature flag, OR land the `ErrorResponse` shim
  in this phase as a private helper that Phase 2 will re-home).
  Recommendation: introduce a minimal `ErrorResponse` here (since the
  daemon needs to emit `service_unavailable`) and let Phase 2 expand
  it; this avoids a breakage window.

### Tests to add

- `lib/src/watcher.rs`: a `MockClipboard`-backed test that drives
  `on_clipboard_change` by calling the handler directly, asserts the
  channel receives a multi-format event, and asserts that concealed
  payloads are dropped.
- `lib/src/watcher.rs`: a panic-recovery test — install a handler
  whose first call panics, assert the supervisor receives the death
  signal and the next call succeeds.
- `service/tests/health.rs`: end-to-end test that brings up the
  service with a `MockClipboard`, asserts `/health` reports
  `watcher: "running"`, then forces the watcher into degraded state
  (e.g. by sending `WatcherEvent::Died` directly to the supervisor)
  and asserts `/health` returns `503` with body
  `{ error: { code: "service_unavailable", ... } }`.

### Acceptance criteria

- A real `clipper` started against a `MockClipboard` populates history
  via the watcher path, with no `POST /set` calls.
- Concealed entries do not appear in history.
- `/health` reports `watcher: "running"` in the happy path and `503` +
  `service_unavailable` after exhausted restarts.
- All tests pass; clippy clean for all three crates.

---

## Phase 2: REST API contract fixes

**Status:** complete

**Addresses:** Critical #3, #4, #6; Significant #7. Also Code Quality
#3 (shared types), #5 (FORMAT_PRIORITY), #6
(`FormatNotAvailable`), #14 (`set_text` body in client).

**Goal:** Bring the public REST surface back into spec compliance:
tagged `SetRequest`, error envelope, `/current` semantics, query
parameters on `/history` and `/history/:id/content`.

### Files touched

- `biscuit-clipboard/lib/src/error.rs` (or new
  `biscuit-clipboard/service/src/error.rs` — pick one; placing in lib
  lets the client validate against the same shape — see Code Quality
  #1). Define:

  ```rust
  #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
  pub struct ErrorBody { pub code: ErrorCode, pub message: String }

  #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
  pub struct ErrorResponse { pub error: ErrorBody }

  #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
  #[serde(rename_all = "snake_case")]
  pub enum ErrorCode {
      EntryNotFound,
      BadRequest,
      FormatNotAvailable,
      Internal,
      ServiceUnavailable,
  }
  ```

  Provide an `IntoResponse` impl gated behind an `axum` feature OR
  define it in the service crate as a thin wrapper that converts
  `ErrorResponse` to `(StatusCode, Json<ErrorResponse>)`.
- `biscuit-clipboard/service/src/api.rs`:
    - Replace `SetRequest` with a tagged enum:
      ```rust
      #[derive(Deserialize)]
      #[serde(tag = "content_type", rename_all = "snake_case")]
      pub enum SetRequest {
          Text { data: String },
          Html { data: String },
          Rtf { data: String },
          Image { data: String, width: u32, height: u32 },
          Files { data: Vec<PathBuf> },
      }
      ```
      Decode `data` as base64 for `Image`. Route image bytes through
      `Storage::spill_if_needed` so large images take the disk path
      on creation.
    - Replace every `error_response(...)` call with the new
      `ErrorResponse::*` constructors. All five documented error codes
      must be reachable from at least one handler.
    - Rewrite `/current`:
        - holds `Arc<dyn ClipboardBackend + Send + Sync>` in `AppState`
          (already present from Phase 1)
        - calls `tokio::task::spawn_blocking` to read each format
        - returns `204 No Content` when no formats are available
        - returns the entry-shaped body with `id: "current"` otherwise
        - **never** inserts into history
    - Add `Query<HistoryParams>` (with `since: Option<DateTime<Utc>>`,
      `limit: Option<usize>`) and `Query<ContentParams>` (with
      `format: Option<ContentType>`, `encoding: Option<Encoding>`)
      extractors. Apply format selection priority `text > html > rtf >
      image > files`. Return `406` `format_not_available` on miss.
      Apply `?encoding=base64` orthogonally.
    - Extract `const FORMAT_PRIORITY: [ContentType; 5]` once, used by
      `primary_content_type`, `preview`, and the new `/content`
      handler (Code Quality #5).
- `biscuit-clipboard/lib/src/client.rs`:
    - Move shared response types (`EntrySummary`, `HistoryResponse`,
      `SetResponse`, `HealthResponse`) into the lib (or a shared
      module) and have the service `use` them (Code Quality #3).
    - Update `set_text` body to `{"content_type":"text","data":"..."}`
      (Code Quality #14). Add `set_image(data, width, height)`,
      `set_html(data)` etc. for completeness.
    - Add typed query-parameter helpers for `/history` and `/content`.
    - Replace `expect(...)` on `reqwest::Client::build()` with a
      `Result`-returning `ClipperClient::new() -> Result<Self,
      ClientError>` (Code Quality #13).
- `biscuit-clipboard/lib/src/error.rs`: wire
  `ClipboardError::FormatNotAvailable` into the new `/content`
  handler, deleting the dead-code annotation if present (Code
  Quality #6).
- `biscuit-clipboard/lib/src/history.rs`: extend `History::all` to
  accept `since` and `limit` filters (or expose `History::query(...)`
  for the new params). `History::since` already exists; verify it
  matches the new contract.
- `biscuit-clipboard/lib/src/lib.rs`: update the `## Module Layout`
  doc comment to match reality (Code Quality #1).

### Tests to add

- `service/tests/api_set.rs`: each `content_type` discriminant — text,
  html, rtf, image (small inline), image (large spilled), files. The
  image-spilled case asserts the file lands at
  `{cache_dir}/{xxhash}.dat`. The malformed-base64 case asserts a
  `400 bad_request` with the correct error envelope.
- `service/tests/api_errors.rs`: one fixture per `ErrorCode` value.
  Each asserts both the status code AND the JSON body shape via
  `serde_json::from_slice::<ErrorResponse>`.
- `service/tests/api_current.rs`: `/current` returns `204` when the
  backend is empty; returns `id: "current"` and the live-read body
  when populated; does NOT insert into history (assert
  `history.len()` unchanged after the call).
- `service/tests/api_query.rs`: `?since` filters timestamps;
  `?limit` truncates; `?format=html` returns the HTML payload;
  `?format=html` with an entry that has no HTML returns `406`
  `format_not_available`; `?encoding=base64` round-trips through
  `STANDARD.decode`.

### Acceptance criteria

- A spec-compliant client (e.g. one written purely from
  `spec.md`) can drive every endpoint successfully.
- All five error codes are emitted by at least one handler and have
  at least one test asserting the envelope shape.
- `/current` is a live read with `204` semantics.
- `clippy` clean, all tests green.

---

## Phase 3: Cross-platform + dedup correctness

**Status:** complete

**Addresses:** Critical #11, #12. Also Code Quality #2 (duplicate hash
logic), #7 (`ImageSnapshot::data` returning empty for `Spilled`), #9
(double hash compute).

**Goal:** Make the daemon correctly identify live PIDs on Windows and
fix the spilled-image hash collision.

### Files touched

- `biscuit-clipboard/lib/src/config.rs`:
    - Replace the no-op Windows branch in `is_pid_alive`. Use
      `sysinfo` (already in the workspace per memory) for portability
      across all platforms — this also lets us simplify the Unix path.
      Implementation: `let mut sys = System::new(); sys.refresh_process(Pid::from_u32(pid));
      sys.process(...).is_some()`. Keep the `#[cfg(unix)] kill(pid,
      0)` path if it's measurably faster, behind a `#[cfg]`.
    - Add a `tempfile`-based test that exercises both alive (current
      process pid) and dead (a recently-reaped child) cases.
    - Refactor `runtime_dir`/file readers to accept an injectable base
      directory (e.g. `runtime_dir_in(base: &Path)` plus a thin
      `runtime_dir()` wrapper). This makes the `read_*_file_missing`
      tests deterministic against a `TempDir` (Test Coverage #6).
- `biscuit-clipboard/lib/src/entry.rs`:
    - Centralize hashing: `pub fn content_hash_of(formats:
      &[ClipboardFormat]) -> u64`. For `ImageSnapshot::Spilled`, hash
      the file's xxhash (which is what the filename already encodes —
      decode the hex stem of the path) plus dimensions, NOT the empty
      `data()` slice. For `Inline`, hash the bytes directly. For text,
      hash the string. Document the layout and add a `#[test]` that
      asserts a `Spilled` and an `Inline` of the *same* image content
      hash to the same value.
    - Make `ClipboardEntry::new` take an optional precomputed hash
      via a `with_hash` private constructor (Code Quality #9).
- `biscuit-clipboard/lib/src/content.rs`:
    - Change `ImageSnapshot::data() -> &[u8]` to
      `ImageSnapshot::data() -> Option<&[u8]>` returning `None` for
      `Spilled`, OR remove the method outright and require callers to
      go through `Storage::load_spilled` (preferred — it's a tighter
      API). Audit call sites and update accordingly (Code Quality #7).
- `biscuit-clipboard/lib/src/history.rs`:
    - Delete `compute_content_hash`. Call
      `entry::content_hash_of(...)` from the dedup probe (Code
      Quality #2). The compute-twice issue (Code Quality #9) is
      resolved by passing the hash into `ClipboardEntry::with_hash`.

### Tests to add

- `lib/src/config.rs`: `is_pid_alive(std::process::id())` is `true`;
  `is_pid_alive(spawn_then_wait_for_exit())` is `false`. Use a sleeper
  child whose pid we know.
- `lib/src/config.rs`: `read_pid_file_missing` /
  `read_port_file_missing` use `runtime_dir_in(tempdir)` and assert
  `None`. (Test Coverage #6.)
- `lib/src/entry.rs`: same content as `Inline` and as `Spilled`
  produces the same hash. Two `Spilled` images of different
  size_bytes/dimensions produce different hashes.
- `lib/src/history.rs`: dedup probe now agrees with
  `ClipboardEntry::id` for spilled images (regression test for the
  bug described in #12).

### Acceptance criteria

- `clipper` correctly detects stale PIDs on Windows in manual smoke
  test (CI cannot easily test this — call out in the PR).
- Spilled-image dedup test passes; no hash divergence between the
  history probe and the entry id.
- `read_*_file_missing` tests are deterministic under a populated
  runtime dir.
- All tests pass; clippy clean.

---

## Phase 4: CLI fixes

**Status:** complete

**Addresses:** Significant #8, #9, #10.

**Goal:** Bring the CLI into spec — `clip clear` clears the clipboard,
`clip get --format` works, `clip watch` streams from the service.

### Files touched

- `biscuit-clipboard/service/src/api.rs`: add `POST /clear` that
  clears the OS clipboard via
  `tokio::task::spawn_blocking(|| backend.set_text(""))`. Returns
  `204 No Content`. Add `GET /events` (SSE stream) that fans out
  watcher events to subscribers — the existing mpsc receiver in the
  daemon needs to be refactored into a `tokio::sync::broadcast`
  channel (or a dedicated `tokio::sync::watch` per event type) so
  multiple subscribers (history loop + SSE clients) can both consume.
  Recommended: keep the watcher → history mpsc, AND fan out a clone
  of each successful insert into a `broadcast::Sender<EntrySummary>`
  in `AppState`.
- `biscuit-clipboard/lib/src/client.rs`: add `clear_clipboard()`
  hitting `POST /clear`; add `events_stream()` returning
  `impl Stream<Item = Result<EntrySummary, ClientError>>` driven by
  `eventsource-stream` (or `reqwest_eventsource`). Pick one and add
  it to `Cargo.toml`.
- `biscuit-clipboard/cli/src/main.rs`:
    - Restructure into the spec'd module layout
      (`commands/{get,set,info,history,clear,watch,service}.rs`).
      This is also Code Quality #1 (module name drift) for the CLI.
    - `cmd_clear`: call `client.clear_clipboard()`. Add `clip history
      --clear` flag (or new `clip history clear` subcommand) for
      history clearing so the existing capability is not lost. Update
      the help text accordingly.
    - `cmd_get`: forward the `--format` argument to
      `client.get_current(format)` / `client.get_content(id, format)`.
    - `cmd_watch`: consume `client.events_stream()` and print each
      event in the same format as today. Add a `--foreground` flag
      that runs the legacy in-process watcher and refuses (with a
      clear error) if the service is up. Document it as
      "for debugging only".

### Tests to add

- `service/tests/api_clear.rs`: `POST /clear` returns 204 and the
  backend's `set_text("")` is observed (use `MockClipboard`).
- `service/tests/api_events.rs`: an SSE consumer receives events
  generated by `MockClipboard` change notifications. Cover ordering
  (events arrive in insertion order) and reconnection (consumer
  drops and reconnects, no panic on the server).
- `cli/tests/clip_clear.rs` (`assert_cmd`): `clip clear` succeeds,
  `clip get` afterwards shows empty.
- `cli/tests/clip_get_format.rs`: `clip set --html ...` then
  `clip get --format html` returns the HTML; `clip get --format rtf`
  returns `406` mapped to a non-zero exit + a clear error message.
- `cli/tests/clip_watch.rs`: spawn `clip watch`, write to clipboard
  via `clip set`, assert the watch process emits a line. Time-bounded
  to avoid flake.

### Acceptance criteria

- `clip clear` empties the clipboard, NOT history.
- `clip get --format <fmt>` returns the requested format or fails
  loudly.
- `clip watch` streams from the service; `--foreground` falls back
  with a refusal when the service is up.
- All tests pass; clippy clean.

---

## Phase 5: Code quality + ergonomics sweep

**Status:** complete

**Addresses:** Code Quality #1 (module-name drift, finalize), #4
(`EntryId` newtype), #5 (already done in Phase 2 — verify), #8
(bounded mpsc), #10 + #16 (`History` storage → `VecDeque`), #11
(clippy warnings in tests), #12 (`find_clipper_binary` debug log),
#15 (already done in Phase 1 — verify), #17 (rustdoc on public types).

**Goal:** Apply the non-functional polish in a single pass while the
hot work is fresh.

### Files touched

- `biscuit-clipboard/lib/src/entry.rs`:
    - Introduce `pub struct EntryId(String)` with `Display`,
      `FromStr`, `Serialize`, `Deserialize` (transparent), `AsRef<str>`,
      `From<u64>` (constructs hex). Update every public signature
      that took `String` for an entry id to take `EntryId` (Code
      Quality #4). The `"current"` sentinel from Phase 2 is a valid
      `EntryId` value — document it.
- `biscuit-clipboard/lib/src/history.rs`:
    - Switch `entries: Vec<ClipboardEntry>` to `VecDeque`. Use
      `push_front` for insertion, `pop_back` for TTL/cap eviction.
      Update iteration order tests if any (Code Quality #10, #16).
- `biscuit-clipboard/lib/src/watcher.rs`:
    - Replace `mpsc::unbounded_channel` with `mpsc::channel(64)`. On
      `try_send` failure, drop the *oldest* event by dequeueing one
      receive and retrying (or simply log and drop the new event with
      `tracing::warn!`). Document the policy (Code Quality #8).
- `biscuit-clipboard/cli/src/main.rs` (or
  `biscuit-clipboard/cli/src/commands/service.rs`):
    - `find_clipper_binary` emits a `tracing::debug!`
      (or `eprintln!` gated on `--verbose`) indicating which binary
      path was selected (Code Quality #12).
- All public types: add rustdoc summaries with `## Examples` and
  `## Errors` (where applicable) per the repo Rustdoc convention
  (Code Quality #17). Targets: `ClipboardBackend`,
  `ClipboardFormat`, `ContentType`, `ImageSnapshot`,
  `ClipboardEntry`, `EntryId`, `History`, `Storage`, `Supervisor`,
  `WatcherEvent`, `ClipperClient`, `ErrorResponse`, `ErrorCode`.
- All test code: fix `needless_borrows_for_generic_args` and
  `unnecessary_map_or` warnings (Code Quality #11).
- `biscuit-clipboard/lib/src/lib.rs`: re-export `EntryId`,
  `ErrorResponse`, `ErrorCode`. Verify `## Module Layout` doc
  matches reality (Code Quality #1, finalization).

### Tests to add / update

- `lib/src/entry.rs`: round-trip `EntryId` through serde JSON;
  `EntryId::from_str("not-hex")` is rejected.
- `lib/src/history.rs`: `VecDeque` ordering preserved in `all()` —
  newest entry first, oldest last. Eviction order unchanged.
- `lib/src/watcher.rs`: bounded-channel backpressure test — fill the
  channel without a receiver, assert the policy holds (oldest dropped
  OR newest dropped, per the chosen implementation).
- `cargo doc --no-deps -p biscuit-clipboard-lib` runs without
  warnings (run as a CI check or local sanity).

### Acceptance criteria

- All public lib types carry rustdoc per the repo convention.
- `History` is backed by `VecDeque`; insert and eviction are O(1).
- Watcher mpsc is bounded; a stalled receiver does not grow memory
  unbounded.
- `EntryId` is a newtype across the public API (CLI, lib, service).
- `cargo clippy --all-targets -- -D warnings` clean for all three
  crates.

---

## Phase 6: Test coverage closure

**Status:** complete

**Addresses:** Test Coverage #1–#9. Verifies that fixes from earlier
phases are exercised by tests, and adds the integration suites called
out in the review (CLI, watcher, error shape, image spill, wiremock).

**Goal:** Bring coverage up to the monorepo bar — `assert_cmd` for the
CLI, `wiremock` for the client, real end-to-end flows for the
watcher and the spill path.

Note: many of these tests are already added in Phases 1–4 as the
fixes land. This phase audits and fills the remaining gaps.

### Files touched (test files only — no behavior changes)

- `biscuit-clipboard/cli/tests/`:
    - `clip_get.rs` — round-trip a text payload via
      `clip set "hello"` then `clip get`.
    - `clip_set_stdin.rs` — `echo "hi" | clip set` succeeds.
    - `clip_history_json.rs` — `clip history --json` parses to a
      valid `HistoryResponse`.
    - `clip_service_status.rs` — start, status, stop sequence with a
      tempdir-rooted runtime dir.
    - `clip_info.rs` — `clip info` after `clip set` shows the right
      type and size.
   These are the broad CLI integration tests called out in Test
   Coverage #5.
- `biscuit-clipboard/lib/src/client.rs` (or
  `biscuit-clipboard/lib/tests/client_wiremock.rs`):
    - `try_connect` against a `wiremock` server returning the
      `X-Clipper: 1` response header succeeds.
    - `try_connect` against a server WITHOUT the fingerprint fails
      with the expected `ClientError::FingerprintMismatch`.
    - Auto-start handshake: simulate a server that answers `404`
      first, then `200` after 100ms, assert exponential backoff
      polling succeeds within the 5s budget.
    - Error envelope deserialization: `wiremock` returns `404` with
      `{ "error": { "code": "entry_not_found", ... } }`; the client
      surfaces a typed error matching the code (Test Coverage #2,
      #7).
- `biscuit-clipboard/service/tests/spill_e2e.rs`:
    - `POST /set` with a >64 KiB image, assert `cache_dir/{hash}.dat`
      exists on disk, `GET /history/:id/content` reads it back, and
      after the entry is evicted from history the file is removed
      (Test Coverage #9).
- `biscuit-clipboard/service/tests/concealed.rs`:
    - `MockClipboard { concealed: true }` triggers an
      `on_clipboard_change`, assert history remains empty (Test
      Coverage #8).
- Verify and update (do NOT keep verifying the old behavior):
    - `test_current_reflects_latest`,
      `test_current_empty_returns_not_found`,
      `test_disk_spill_for_large_image_content` — rewrite to assert
      the correct spec'd behavior (Test Coverage #4, #9).
- `biscuit-clipboard/cli/Cargo.toml` and
  `biscuit-clipboard/lib/Cargo.toml`: add `assert_cmd`,
  `predicates`, `wiremock`, `tempfile` to `[dev-dependencies]` if
  not already present.

### Acceptance criteria

- Every Test Coverage #1–#9 item maps to at least one passing test
  by name in the PR description / phase summary.
- `cargo test -p biscuit-clipboard-lib -p biscuit-clipboard-cli -p
  biscuit-clipboard-service` exits 0.
- No previously-correct behavior is regressed (`test_current_*` and
  `test_disk_spill_*` rewrites are explicit and reviewed).
- `cargo clippy --all-targets -- -D warnings` clean.

---

## Phase 7: Autostart — install/uninstall shim + docs

**Status:** pending

**Addresses:** Significant #14.

**Goal:** Provide first-run autostart support without committing to
the long tail of OS-specific edge cases. Per the decision documented
above, this phase ships:

- `clip service install` / `clip service uninstall` subcommands
- documentation in the README clearly marking autostart scope
- an explicit out-of-scope note for system-level (root-owned) install
  paths

If schedule pressure forces it, this phase can be reduced to docs
only — see "Reduced scope fallback" below.

### Files touched

- `biscuit-clipboard/cli/src/commands/service.rs`:
    - `install` subcommand:
        - macOS: write
          `~/Library/LaunchAgents/com.bensimmonds.biscuit-clipboard.plist`
          pointing at the absolute path of the current `clipper`
          binary (resolved via `find_clipper_binary`). Run
          `launchctl bootstrap gui/$UID <plist>` if not already
          loaded.
        - Linux: write
          `~/.config/systemd/user/biscuit-clipboard.service`. Run
          `systemctl --user daemon-reload && systemctl --user enable
          --now biscuit-clipboard.service`.
        - Windows: drop a `.lnk` (or
          `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\biscuit-clipboard.lnk`)
          pointing at the binary. Use `mslnk` or shell out to
          PowerShell — pick one and document.
        - On any path: print a clear "Installed; clipper will start
          on next login. Run `clip service start` to start now."
    - `uninstall` subcommand: reverse of the above. Idempotent.
    - Both commands accept `--dry-run` to print the manifest content
      to stdout without writing.
- `biscuit-clipboard/README.md`:
    - Add an `## Autostart` section explaining install/uninstall
      semantics and listing the manifest paths used per platform.
    - Add a `## Notes` section calling out that system-level
      (multi-user) install is out of scope for v1; manifests are
      always user-scoped.
- Update the spec ↔ implementation drift note in
  `biscuit-clipboard/CLAUDE.md` (or the package's `docs/`) to record
  the autostart decision.

### Tests to add

- `cli/tests/clip_service_install.rs`: `clip service install
  --dry-run` prints a plausible manifest containing the binary path
  for each platform (gated with `#[cfg(target_os = "...")]`).
  Avoid invoking real `launchctl`/`systemctl` in CI — those paths
  are smoke-tested manually.
- `cli/tests/clip_service_uninstall.rs`: `--dry-run` reports what
  would be removed; idempotent on a clean system.

### Acceptance criteria

- A user running `clip service install` on macOS, Linux, or Windows
  ends up with a working autostart entry on next login (manual
  verification — note in PR description).
- `--dry-run` is fully tested in CI for all three target families.
- `cargo clippy --all-targets -- -D warnings` clean.

### Reduced scope fallback

If implementation effort exceeds the phase budget, drop
install/uninstall and:

1. Add an `## Autostart` README section that walks the user through
   the manifest manually (with copy-pasteable plists/units).
2. Open a v1.1 tracking issue capturing the install/uninstall scope.
3. Update the spec to reflect the deferral.

This still satisfies the review's "flag it as a gap" requirement.

---

## Review-Item-to-Phase Map

| Review Item | Category | Phase |
|---|---|---|
| #1 daemon never observes clipboard | Critical | 1 |
| #2 watcher doesn't use change listener | Critical | 1 |
| #3 `POST /set` body diverges from spec | Critical | 2 |
| #4 error response shape wrong | Critical | 2 |
| #5 `/health` missing `watcher` field | Critical | 1 |
| #6 `/current` does not match spec | Critical | 2 |
| #7 query params ignored | Significant | 2 |
| #8 `clip clear` clears history not clipboard | Significant | 4 |
| #9 `clip get --format` ignored | Significant | 4 |
| #10 `clip watch` runs own watcher | Significant | 4 |
| #11 `is_pid_alive` no-op on Windows | Significant | 3 |
| #12 hash mismatch dedup vs id | Significant | 3 |
| #13 concealed content never enforced | Significant | 1 |
| #14 no autostart wiring | Significant | 7 |
| Test Coverage #1 watcher integration | — | 1 |
| Test Coverage #2 spec error shape | — | 2 |
| Test Coverage #3 query params untested | — | 2 |
| Test Coverage #4 `/current` wrong contract | — | 2 |
| Test Coverage #5 no CLI binary tests | — | 6 |
| Test Coverage #6 flaky port/pid file tests | — | 3 |
| Test Coverage #7 `ClipperClient` HTTP untested | — | 6 |
| Test Coverage #8 no concealed-content path | — | 6 |
| Test Coverage #9 image spill e2e missing | — | 6 |
| Code Quality #1 module name drift | — | 2 (lib), 4 (CLI), 5 (final verify) |
| Code Quality #2 duplicate hash logic | — | 3 |
| Code Quality #3 two `EntrySummary` types | — | 2 |
| Code Quality #4 `EntryId = String` newtype | — | 5 |
| Code Quality #5 FORMAT_PRIORITY constant | — | 2 |
| Code Quality #6 `FormatNotAvailable` unused | — | 2 |
| Code Quality #7 `ImageSnapshot::data` returns `&[]` | — | 3 |
| Code Quality #8 unbounded mpsc | — | 5 |
| Code Quality #9 `History::insert` double hash | — | 3 |
| Code Quality #10 ring buffer is `Vec` | — | 5 |
| Code Quality #11 clippy warnings in tests | — | 5 |
| Code Quality #12 `find_clipper_binary` debug log | — | 5 |
| Code Quality #13 `ClipperClient::new` panics | — | 2 |
| Code Quality #14 client `set_text` body | — | 2 |
| Code Quality #15 no structured logging | — | 1 |
| Code Quality #16 `History::entries` should be `VecDeque` | — | 5 |
| Code Quality #17 missing rustdoc | — | 5 |

No review item is dropped. Each appears in at least one phase row.

## Final cross-phase acceptance

After all 7 phases land, the following must be true:

- `cargo test --workspace` exits 0 on macOS and Linux runners (manual
  smoke on Windows).
- `cargo clippy -p biscuit-clipboard-lib -p biscuit-clipboard-cli -p
  biscuit-clipboard-service --all-targets -- -D warnings` exits 0.
- `cargo doc --no-deps -p biscuit-clipboard-lib` produces no warnings.
- A spec-only client (with no knowledge of the implementation) can
  drive every documented endpoint.
- `clip get`, `clip set`, `clip info`, `clip clear`, `clip history
  --json`, `clip watch`, `clip service {start,stop,status,install,
  uninstall}` all behave per the spec table.
- The watcher captures multi-format clipboard changes and the
  supervisor restarts on panic.
- `is_pid_alive` works on Windows.
- The README declares autostart scope clearly (full or fallback).
