# Darkmatter Dependencies

## Remote URL Referencing

- `biscuit-file` with the `fetch` feature supplies the shared HTTP fetch
  primitive and host allowlist policy used by compose remote reads and
  side-effect network writes.
- `reqwest` provides the shared HTTP client for compose remote fetches and the
  side-effect `http_post` verb.
- `tokio` runs remote fetch tasks and the blocking wrapper used by synchronous
  callers.
- `url` parses and normalizes HTTP(S) references before policy checks.

These dependencies are required so every network egress path goes through the
same scheme validation and deny-all-by-default host policy.
