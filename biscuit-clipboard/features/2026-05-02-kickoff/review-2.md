---
agent: codex
model: ""
ready: false
---

# Biscuit Clipboard Kickoff - Review #2

## Verdict

Not ready for production.

Most Review #1 implementation gaps are materially improved: the daemon now wires a real `clipboard-rs` watcher supervisor, concealed events are dropped before history, the REST error envelope exists, query params on `/history/:id/content` work, autostart manifests exist, and the package test suite is green.

Two blockers remain. First, `POST /set` records a history entry but does not write to the OS clipboard, so `clip set "foo" && clip get` does not satisfy the spec. Second, the user-observable clipboard-observation behavior is still only verified at Level 1 with mocks/synthetic triggers, not with a real terminal/session and real OS clipboard events.

Verification run:

```text
cargo test -p biscuit-clipboard -p biscuit-clipboard-service -p biscuit-clipboard-cli --no-fail-fast
```

Result: passed.

## Findings

### Critical: `POST /set` does not set the host clipboard

Spec requirement: `POST /set` sets clipboard content, and `clip set` targets that REST endpoint so only `clipper` touches the host clipboard. The CLI also has an explicit validation checkpoint: `clip set "foo" && clip get` should return `foo`.

Implementation currently builds a `ClipboardFormat` and inserts it into `History`, then broadcasts an event. It never calls `state.clipboard.set_text(...)` or any equivalent host clipboard write path: [api.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-clipboard/biscuit-clipboard/service/src/api.rs:381). The client does post the tagged text request from `clip set`: [client.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-clipboard/biscuit-clipboard/lib/src/client.rs:217), but the backend trait only exposes `set_text`: [backend.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-clipboard/biscuit-clipboard/lib/src/backend.rs:30). Image, HTML, RTF, and file-list `SetRequest` variants can therefore be stored in history but cannot be written to the OS clipboard at all.

Impact: `clip set` can report success while leaving the actual clipboard unchanged. `/current` is a live OS read, so it will not necessarily reflect the value just set.

Verification level present: Level 1 only. Tests assert the HTTP body sent to `/set` and history insertion, but no test drives `clip set` followed by a live `/current`/`clip get` against a backend that proves `set_text` was invoked. Required level for this user-visible behavior: at least Level 1 with backend invocation assertions for text, and platform-gated real OS clipboard coverage for the end-to-end `clip set && clip get` path.

Fix: call the live backend from `set_content` before or atomically with history insertion. Extend `ClipboardBackend` with write methods for the `SetRequest` formats that are in V1 scope, or reject unsupported variants with `501`/document them as not included. Add tests that fail if `/set` only mutates history.

### High: Real clipboard watching is not verified beyond Level 1

Spec requirement: the daemon watches host clipboard changes via `clipboard-rs`, captures all available formats, skips concealed macOS pasteboard entries, and maintains history from actual clipboard changes.

Implementation now uses `ClipboardWatcherContext` in `spawn_system_watcher`, and the daemon supervisor consumes `WatcherEvent`s. The tests, however, exercise `capture_event`, `spawn_watcher_with_trigger`, and direct `apply_watcher_event` calls with `MockClipboard`. That is useful Level 1 coverage, but it does not prove that a real `clipboard-rs` watcher emits events when the OS clipboard changes, that copied text/images/files enter `/history`, or that platform concealed types are skipped in an actual pasteboard session.

Verification level present: Level 1. Required level: Level 3-equivalent for this product domain, because the requirement depends on real OS clipboard event emission rather than manufactured bytes/events. A Level 2-style spawned terminal/session is enough for CLI rendering and service text capture, but the core watcher path needs platform-gated real OS clipboard injection/interaction.

Fix: add env-gated integration tests such as `RUN_CLIPBOARD_E2E=1` that start `clipper` with an isolated runtime dir, write to the real OS clipboard using a platform tool or clipboard-rs client, poll `/history`, and verify text plus at least one non-text format where feasible. On macOS, add a gated concealed-type test or explicitly document why it remains manual.

### High: `clip get --format` is forwarded by the client but ignored by `/current`

Spec requirement: `clip get --format html` prints the current clipboard in the requested format.

The CLI forwards `format` and `encoding` to `get_current_with_format`: [main.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-clipboard/biscuit-clipboard/cli/src/main.rs:136). The client appends those query params to `/current`: [client.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-clipboard/biscuit-clipboard/lib/src/client.rs:118). The service handler does not accept `Query<ContentQuery>` and always returns an `EntrySummary` JSON preview for any non-empty clipboard: [api.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-clipboard/biscuit-clipboard/service/src/api.rs:338).

Impact: `clip get --format html` prints the primary preview, not necessarily the requested HTML. `--encoding base64` is also ineffective on `/current`.

Verification level present: Level 1, but only on the client URL construction. There is no service or CLI test proving requested current formats are selected. Required level: Level 1 is sufficient for the API contract here; add service tests with a multi-format `MockClipboard` and CLI wiremock tests that prove non-default formats are printed.

Fix: make `/current` accept `ContentQuery` and reuse `select_format`/`render_format` against the live snapshot, while preserving the no-query entry-shaped response required by the spec.

### Medium: Spilled files are not cleaned up when history entries expire

Spec requirement: spilled files are cleaned up when their corresponding history entries expire from the ring buffer.

`Storage` has cleanup helpers: [storage.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-clipboard/biscuit-clipboard/lib/src/storage.rs:109). `History::evict` removes expired entries and cap overflow entries internally without returning removed entries or notifying `Storage`: [history.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-clipboard/biscuit-clipboard/lib/src/history.rs:154). `History::clear` also drops all entries without cache cleanup: [history.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-clipboard/biscuit-clipboard/lib/src/history.rs:150).

Impact: large image files can accumulate indefinitely in the cache directory even though history no longer references them.

Verification level present: Level 1 verifies spill creation and read-back, but not expiry cleanup. Required level: Level 1 is sufficient. Add tests with a short TTL/max cap and assert the `.dat` file is removed when the entry is evicted or history is cleared.

Fix: have history mutation APIs return evicted/removed entries, and let the service/storage owner clean their spilled payloads. Alternatively move storage ownership into a higher-level history store that can perform eviction and cleanup together.

### Medium: `clipper` direct-start default port is random, not `17530`

Spec requirement: the service listens on fixed local port `17530` by default, overridable by `CLIP_PORT`.

The daemon argument default is `0`, which binds an ephemeral port when a user runs `clipper` directly: [main.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-clipboard/biscuit-clipboard/service/src/main.rs:21). The CLI auto-start path passes the configured port, so this mainly affects direct service launches and autostart manifests that run `clipper` without arguments.

Verification level present: Level 1 tests cover port-file behavior, but not the direct default. Required level: Level 1 is sufficient.

Fix: default `Args::port` to `biscuit_clipboard::config::configured_port()` via clap default logic or parse an `Option<u16>` and resolve after parsing.

## Test-Level Matrix

| Requirement | Strongest observed verification | Required | Status |
|---|---:|---:|---|
| `clip set` changes the actual clipboard | Level 1 partial HTTP-body/history tests | Level 1 backend assertion plus OS clipboard E2E | Gap |
| Daemon observes real clipboard changes into history | Level 1 mock/synthetic watcher tests | Level 3-equivalent OS clipboard event test | Gap |
| Multi-format watcher capture | Level 1 mock capture tests | Level 1 plus real OS smoke for at least representative formats | Partial |
| Concealed content skipped | Level 1 mock/apply-event tests | Level 3-equivalent macOS pasteboard test or documented manual gate | Gap |
| `/history` and `/history/:id/content` query behavior | Level 1 service tests | Level 1 | OK |
| `clip get --format` current format selection | Level 1 client URL test only | Level 1 service + CLI behavior | Gap |
| Disk spill read-back | Level 1 API round-trip tests | Level 1 | OK |
| Disk spill expiry cleanup | No effective coverage | Level 1 | Gap |
| Autostart manifest writing | Level 1 filesystem tests | Level 1 | OK |

## Notes

The current code is much closer than Review #1, and most remaining issues are narrowly fixable. I would not ship until the `/set` path writes the OS clipboard and the release gate includes at least one env-gated real clipboard watcher test, because those are the product's primary user-observable behaviors.
