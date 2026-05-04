---
phases: 5
start_phase: 2
source_files_during_phase_1:
  - biscuit-clipboard/lib/src/backend.rs
  - biscuit-clipboard/lib/src/storage.rs
  - biscuit-clipboard/lib/src/watcher.rs
  - biscuit-clipboard/service/src/api.rs
  - biscuit-clipboard/cli/tests/clip_set.rs
docs_updated_during_phase_1:
  - biscuit-clipboard/features/2026-05-02-kickoff/review-plan-2.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-clipboard/service/src/api.rs
  - biscuit-clipboard/cli/tests/clip_service_install.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-clipboard/lib/src/history.rs
  - biscuit-clipboard/service/src/api.rs
  - biscuit-clipboard/service/src/main.rs
docs_updated_during_phase_3:
  - biscuit-clipboard/features/2026-05-02-kickoff/review-plan-2.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - biscuit-clipboard
  - biscuit-clipboard-service
  - biscuit-clipboard-cli
---
# Biscuit Clipboard Review #2 Implementation Plan

**Generated from:** review-2.md findings against spec.md
**Target state:** All 5 review findings resolved, all tests green, zero lint warnings.

---

## Overview

This plan addresses every finding in `review-2.md` across **5 phases**. Phases are ordered by dependency: Phase 1 (backend writes) unlocks Phase 2 (`/current` format selection) and Phase 5 (real clipboard E2E). Phase 3 (cleanup) is independent. Phase 4 (default port) is a one-line fix.

**Total scope:** ~12 files modified, ~20 new tests added, 1 new integration test module.

---

## Phase 1: Critical — `POST /set` must write to the host clipboard

**Goal:** `clip set "foo" && clip get` returns `foo`. The `/set` endpoint calls the live backend before recording history.

### 1.1 Extend `ClipboardBackend` with write methods

**File:** `biscuit-clipboard/lib/src/backend.rs`

**Changes:**
- Add to `ClipboardBackend` trait:
  ```rust
  fn set_html(&self, html: &str) -> Result<(), ClipboardError>;
  fn set_rtf(&self, rtf: &str) -> Result<(), ClipboardError>;
  fn set_image(&self, data: &[u8], width: u32, height: u32) -> Result<(), ClipboardError>;
  fn set_files(&self, files: &[PathBuf]) -> Result<(), ClipboardError>;
  ```
- Implement all four on `SystemClipboard` using `clipboard_rs::ClipboardContext` APIs (`set_html`, `set_rich_text`, `set_image`, `set_files`).
- Implement all four on `MockClipboard`, with capture logs (e.g. `set_html_calls`, `set_image_calls`) so tests can assert invocation.

**Rationale:** The spec says "only `clipper` touches the host clipboard." Today `/set` bypasses the backend entirely. Extending the trait preserves the abstraction boundary.

### 1.2 Wire backend writes into `set_content`

**File:** `biscuit-clipboard/service/src/api.rs`

**Changes in `set_content` handler (line ~381):**
- After `build_format_from_request` succeeds, spawn a blocking task that writes the built format(s) to `state.clipboard` *before* inserting into history.
- Text → `set_text`, HTML → `set_html`, RTF → `set_rtf`, Image → `set_image`, Files → `set_files`.
- If any backend write fails, return `500 internal` with the error message.
- On success, proceed to history insert + SSE broadcast exactly as today.

**Signature change:**
```rust
async fn set_content(
    State(state): State<SharedState>,
    body: Result<Json<SetRequest>, axum::extract::rejection::JsonRejection>,
) -> Response
```

Add a helper:
```rust
fn write_format_to_backend(
    backend: &(dyn ClipboardBackend + Send + Sync),
    format: &ClipboardFormat,
) -> Result<(), String>
```

### 1.3 Tests

**File:** `biscuit-clipboard/service/src/api.rs` (in existing `#[cfg(test)]` module)

New tests:
- `test_set_text_invokes_backend_set_text` — POST `/set` with `"content_type":"text"`, then assert `mock.set_text_log()` contains the string.
- `test_set_html_invokes_backend_set_html` — same for HTML variant.
- `test_set_image_invokes_backend_set_image` — same for image variant.
- `test_set_backend_failure_returns_500_internal` — install a `MockClipboard` whose `set_text` returns `Err`, POST text, assert `500` + `ErrorCode::Internal` envelope.
- `test_set_then_current_roundtrip` — set text via `/set`, then `GET /current` on the same app state and assert the body contains the same text.

**File:** `biscuit-clipboard/cli/src/main.rs` or a new CLI integration test

- Add a test that starts a service with `MockClipboard` (or a real one gated by env), calls `clip set "foo"`, then `clip get`, and asserts `"foo"` is printed.

---

## Phase 2: High — `/current` must honour `?format=` and `?encoding=` query params

**Goal:** `clip get --format html` prints the HTML variant of the current clipboard, not the primary preview JSON.

### 2.1 Accept `ContentQuery` in `get_current`

**File:** `biscuit-clipboard/service/src/api.rs`

**Changes in `get_current` handler (line ~338):**
- Change signature to:
  ```rust
  async fn get_current(
      State(state): State<SharedState>,
      Query(params): Query<ContentQuery>,
  ) -> Response
  ```
- After `capture_current` returns `formats`, if `params.format` or `params.encoding` is present:
  1. Build a transient `ClipboardEntry::new(formats)`.
  2. Call `select_format(&entry, params.format)` to pick the requested format.
  3. If found, call `render_format(&state.storage, format, encode_base64)` and return that response.
  4. If not found, return `406 format_not_available`.
- If neither query param is present, preserve existing behaviour: return `EntrySummary` JSON with `id: "current"`.

### 2.2 Tests

**File:** `biscuit-clipboard/service/src/api.rs`

New tests:
- `test_current_with_format_text` — mock with both text and HTML, `GET /current?format=text`, assert body is the text string.
- `test_current_with_format_html` — same, `?format=html`, assert body is the HTML string.
- `test_current_with_format_missing_returns_406` — mock with text only, request `?format=html`, assert `406` + `ErrorCode::FormatNotAvailable`.
- `test_current_with_encoding_base64` — mock with text, `GET /current?encoding=base64`, assert body is base64-encoded text.
- `test_current_without_query_params_returns_entry_summary` — assert existing behaviour still works (regression guard).

**File:** `biscuit-clipboard/lib/src/client.rs` or CLI tests

- Add CLI wiremock test: `test_cli_get_with_format_forwards_and_prints` — mock server returns `"<b>html</b>"` for `/current?format=html`, assert `clip get --format html` prints it.

---

## Phase 3: Medium — Spilled files must be cleaned up when history entries expire

**Goal:** When `History::evict` drops entries, or `History::clear` empties the buffer, any `ImageSnapshot::Spilled` files are deleted from the cache directory.

### 3.1 Return evicted entries from `History`

**File:** `biscuit-clipboard/lib/src/history.rs`

**Changes:**
- Change `insert` signature from `Option<&ClipboardEntry>` to `(Option<&ClipboardEntry>, Vec<ClipboardEntry>)` where the `Vec` holds every entry removed by `evict`.
- Change `clear` signature from `()` to `Vec<ClipboardEntry>` returning all entries that were in the buffer.
- Change `evict` from `fn evict(&mut self)` to `fn evict(&mut self) -> Vec<ClipboardEntry>` collecting removed entries.

**Rationale:** `Storage` owns the cleanup methods but doesn't know when history drops entries. The cleanest fix is to have history report what it removed.

### 3.2 Call cleanup in the service layer

**File:** `biscuit-clipboard/service/src/api.rs`

**Changes:**
- In `set_content`, after `history.insert`, iterate the returned `Vec<ClipboardEntry>` and call `state.storage.cleanup_spilled` on every `ImageSnapshot::Spilled` found in each entry's formats.
- In `delete_history`, after `history.clear()`, iterate the returned entries and perform the same cleanup.
- In `apply_watcher_event` (`main.rs`), after `history.insert`, perform the same cleanup.

Add a helper:
```rust
fn cleanup_evicted_entries(storage: &Storage, evicted: Vec<ClipboardEntry>) {
    for entry in evicted {
        for fmt in &entry.formats {
            if let ClipboardFormat::Image(img) = fmt {
                let _ = storage.cleanup_spilled(img);
            }
        }
    }
}
```

### 3.3 Tests

**File:** `biscuit-clipboard/lib/src/history.rs`

New tests:
- `test_insert_returns_evicted_entries` — set `max_entries=2`, insert 3 items, assert the returned vec contains the oldest entry.
- `test_clear_returns_all_entries` — insert 3, clear, assert returned vec length is 3.

**File:** `biscuit-clipboard/service/src/api.rs`

New tests:
- `test_spilled_file_removed_on_eviction` — app with `spill_threshold=10`, insert a large image (spills to `.dat`), then insert enough text entries to push it out of `max_entries`, assert the `.dat` file no longer exists.
- `test_spilled_file_removed_on_delete_history` — insert large image (spills), `DELETE /history`, assert `.dat` file removed.
- `test_spilled_file_survives_when_entry_still_in_history` — insert large image, do not evict, assert `.dat` still exists (regression guard).

---

## Phase 4: Medium — `clipper` direct-start default port must be `17530`

**Goal:** Running `clipper` without `--port` binds to `17530` (respecting `CLIP_PORT` env var).

### 4.1 Fix CLI default

**File:** `biscuit-clipboard/service/src/main.rs`

**Changes in `Args` (line ~19):**
```rust
#[derive(Parser, Debug)]
#[command(name = "clipper", about = "Biscuit clipboard background service")]
struct Args {
    #[arg(long, default_value_t = biscuit_clipboard::config::configured_port())]
    port: u16,
    // ...
}
```

Or, if clap cannot evaluate a function call in `default_value_t`, use:
```rust
#[arg(long)]
port: Option<u16>,
```
And resolve after parsing with `.unwrap_or_else(configured_port)`.

### 4.2 Tests

**File:** `biscuit-clipboard/service/src/main.rs` (in `#[cfg(test)]`)

New test:
- `test_args_default_port_is_17530` — parse empty args, assert `args.port == 17530`.
- `test_args_port_env_override` — set `CLIP_PORT=12345`, parse empty args, assert `args.port == 12345`.
- `test_args_port_cli_override` — parse `["--port", "9999"]`, assert `args.port == 9999`.

---

## Phase 5: High — Real clipboard watching integration tests

**Goal:** At least one env-gated test proves the watcher captures real OS clipboard changes into `/history`.

### 5.1 New integration test module

**File:** `biscuit-clipboard/service/tests/clipboard_e2e.rs` (new)

**Structure:**
- Gated by `RUN_CLIPBOARD_E2E=1` env var. If not set, the test module compiles but every test returns early with `#[ignore]` or an explicit `return`.
- Each test:
  1. Creates a `tempfile::TempDir` for the runtime directory.
  2. Sets `CLIP_RUNTIME_DIR` to that temp dir.
  3. Spawns `clipper --port 0` (or a fixed ephemeral port).
  4. Polls `/health` until ready.
  5. Writes text to the **real OS clipboard** using a fresh `SystemClipboard` or `clipboard_rs::ClipboardContext`.
  6. Polls `/history` with a short timeout (up to 2s) until the entry appears.
  7. Asserts the text content matches.
  8. Kills the service, cleans up temp dir.

**Tests:**
- `e2e_text_roundtrip` — copy text, assert history contains it.
- `e2e_html_roundtrip` (optional, platform-dependent) — copy HTML, assert history contains both `text` and `html` formats.
- `e2e_image_roundtrip` (optional) — copy a small PNG, assert history entry type is `image`.
- `e2e_concealed_skipped_macos` (macOS-only, optional) — if feasible, use a platform tool to place `org.nspasteboard.ConcealedType` on the pasteboard, assert the entry does **not** appear in history. If not automatable, leave a comment documenting the manual test steps.

### 5.2 CI / Makefile notes

Add a comment or README note explaining:
```bash
RUN_CLIPBOARD_E2E=1 cargo test -p biscuit-clipboard-service --test clipboard_e2e
```
These tests are **not** run in standard `cargo test` because they mutate the user's real clipboard and require a graphical session (or clipboard daemon on Linux).

---

## Lint & Typecheck Concerns

- **Unused imports:** After adding new methods to `ClipboardBackend`, verify no unused imports in `backend.rs`.
- **Clippy:** Run `cargo clippy -p biscuit-clipboard -p biscuit-clipboard-service -p biscuit-clipboard-cli --all-targets -- -D warnings` after each phase.
- **Tests:** Run `cargo test -p biscuit-clipboard -p biscuit-clipboard-service -p biscuit-clipboard-cli --no-fail-fast` after each phase.
- **`unsafe` blocks:** The existing `unsafe` env-var mutations in tests (e.g. `client.rs` line 486) are already present and gated by `serial_test`. Do not introduce new `unsafe`.

---

## Test Matrix (post-fix)

| Requirement | Verification after fix |
|---|---|
| `clip set` changes the actual clipboard | Level 1: mock assertions in `api.rs` tests; Level 3-equivalent: `clipboard_e2e.rs` real OS roundtrip |
| Daemon observes real clipboard changes | Level 3-equivalent: `clipboard_e2e.rs` |
| Multi-format watcher capture | Level 1: existing mock tests; Level 3-equivalent: `clipboard_e2e.rs` HTML/image |
| Concealed content skipped | Level 1: existing `apply_event_drops_concealed_entries`; Level 3: macOS manual or gated test |
| `/history` and `/history/:id/content` query behaviour | Level 1: existing tests (OK) |
| `clip get --format` current format selection | Level 1: new service + CLI tests in Phase 2 |
| Disk spill read-back | Level 1: existing tests (OK) |
| Disk spill expiry cleanup | Level 1: new tests in Phase 3 |
| Autostart manifest writing | Level 1: existing tests (OK) |

---

## Dependencies & Risks

| Risk | Mitigation |
|---|---|
| `clipboard_rs::ClipboardContext::set_html` / `set_image` APIs differ from read APIs | Check `clipboard-rs` 0.3.4 docs; if a write API is missing, return `501 Not Implemented` for that variant with a documented fallback. |
| `History::insert` signature change breaks call sites | There are 3 call sites (`api.rs` `set_content`, `main.rs` `apply_watcher_event`, and tests). Update all in Phase 3. |
| E2E tests fail on headless CI (Linux without X11/Wayland) | Gate behind `RUN_CLIPBOARD_E2E=1`; document that they require a clipboard-capable session. |
| E2E tests interfere with developer's clipboard | Same gating; tests clean up the clipboard after themselves (clear or restore original). |
| Default port `default_value_t` may not compile with function call | Fallback to `Option<u16>` + manual resolution if clap derive macro rejects it. |

---

## Estimated Effort

- Phase 1: 3–4 hours (trait changes + service wiring + tests)
- Phase 2: 2–3 hours (handler change + render reuse + tests)
- Phase 3: 2–3 hours (history API change + service cleanup wiring + tests)
- Phase 4: 30 minutes (one-line fix + unit tests)
- Phase 5: 2–4 hours (integration test scaffolding + platform debugging)

**Total:** ~10–15 hours of focused implementation time.
