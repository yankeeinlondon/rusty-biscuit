# Claudine Dependencies

## Executable Lookup

- `which` is pinned to major version `8` across both `claudine` (library) and
  `claudine-cli` so provider and tool discovery behavior stays consistent.

## Lifecycle Requeue

- `claudine-cli` depends on `rendezvous-client` and `rendezvous-core` on every
  target so the lifecycle `requeue(...)` control action can append to the
  rendezvous deferred-execution session log. On Unix the client connects over
  the existing UDS transport; on Windows it connects over a named pipe via
  `tokio::net::windows::named_pipe::ClientOptions`. When the daemon is
  unreachable on either platform, the entry is durably appended to a local
  fallback file (`<config_dir>/claudine/rendezvous/deferred-queue.jsonl`,
  overridable via `CLAUDINE_RENDEZVOUS_FALLBACK_DIR`) so the prompt is never
  lost.
- `tonic` is a direct CLI dependency for the rendezvous RPC status type
  surfaced by that enqueue path.
- `dirs` resolves the per-user config directory for the fallback file.
- `thiserror` is used by the CLI's internal enqueue error type so the typed
  composition error can preserve a clear, source-aware failure message.
