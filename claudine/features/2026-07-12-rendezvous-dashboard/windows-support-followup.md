# Follow-up — Windows support for the rendezvous daemon (review Finding 9)

**Status: SUPERSEDED (2026-07-16).** The gap this note recorded is closed.

- **What closed it:** [`claudine/fixes/2026-07-13-rendezvous-local-ipc/spec.md`](../../fixes/2026-07-13-rendezvous-local-ipc/spec.md)
- **Authoritative documentation:** [`claudine/docs/rendezvous/local-ipc.md`](../../docs/rendezvous/local-ipc.md)

The fix went wider than this note scoped. This note framed the problem as a
missing Windows listener; the fix treated it as one contract problem spanning
endpoint naming, endpoint authorization, data ownership, daemon identity, client
retry, and server startup, which all had to agree on the same owner and
transport semantics.

Two of the recommendations below were **not** followed, deliberately:

- **"mirroring `spawn_uds_server`"** (step 1) was rejected. Mirroring the Unix
  startup function would have duplicated the storage, projection, identity,
  register, QUIC, discovery, and worker stack into a second file that could then
  drift. Instead a transport-neutral `prepare_daemon` was extracted, and the two
  platform modules own only listener, accept, permission, and cleanup logic. The
  note's own conclusion — one portable `spawn_local_server` — did survive, and is
  the production entry point.
- **Re-gating the `claudine-cli` daemon tests per-OS** (step 3) was replaced by
  removing the gates: the `rendezvous-daemon` dev-dependency left its
  `cfg(unix)` target section, and the call-site tests now run on all three OSes.

Step 4's insistence on runtime verification was kept and hardened: the Windows
leg is gating, not soft-fail, in `.github/workflows/rendezvous-tests.yml`.

Everything below is the note as written on 2026-07-13, retained as the historical
record of what was true at review time.

---

## Historical record (2026-07-13)

**Status:** deferred (ratified 2026-07-13). This note records the exact gap and the
work needed so a later pass — on a machine with a Windows toolchain/runner — can close it.

### What is already portable
- `rendezvous-core` — `socket::default_socket_path` resolves a UDS path on Unix and a
  named-pipe path on Windows.
- `rendezvous-client` — `connect` dispatches to `connect_uds` (Unix) or
  `connect_named_pipe` (Windows). The dashboard call site uses this portable client.

### The gap
- `rendezvous-daemon` `server.rs` binds a `tokio::net::UnixListener` unconditionally and
  only exposes `spawn_uds_server`. There is **no Windows named-pipe server**, so the daemon
  — and `claudine dashboard` end-to-end — cannot run on Windows, regardless of the portable
  client.

### Made honest now (this pass)
- `claudine-cli` already declares `rendezvous-daemon` under
  `[target.'cfg(unix)'.dev-dependencies]`. The test functions that spawn the daemon are now
  `#[cfg(unix)]`-gated (`dashboard/tests.rs::fetch_snapshot_reflects_a_live_session`,
  `session_report.rs::round_trip_against_live_daemon` +
  `status_reporter_flips_and_clears_waiting` + the `await_status` helper), so `claudine-cli`
  compiles cleanly on Windows. The L2 `level2_lifecycle_control.rs` is already
  `#![cfg(unix)]`.
- `server.rs` module docs now state the Unix-only constraint and point here.

### Work to close it (a focused pass with a Windows runner)
1. Add `spawn_named_pipe_server` to `rendezvous-daemon` `server.rs`, mirroring
   `spawn_uds_server` but binding a `tokio::net::windows::named_pipe` server and feeding a
   named-pipe incoming stream into the same tonic `Server` + persistence stack. Gate the two
   server fns per-OS; expose one portable `spawn_local_server` that picks the right one.
2. Audit the rest of `rendezvous-daemon` for other Unix-only assumptions surfaced once the
   `UnixListener` blocker is removed (socket-path handling, cleanup-on-drop semantics).
3. Un-gate (or dual-gate) the `claudine-cli` daemon-spawning tests so they run on Windows via
   the named-pipe path.
4. Add Windows CI coverage: a real named-pipe daemon ⇄ client ⇄ `claudine dashboard` round
   trip. Do not consider Finding 9 closed without this runtime verification — a compile-only
   Windows build can hide pipe-semantics bugs.
